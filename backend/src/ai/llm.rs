use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: Some(content.into()),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            tool_calls: vec![],
            tool_call_id: None,
        }
    }
    pub fn assistant(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content,
            tool_calls,
            tool_call_id: None,
        }
    }
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_calls: vec![],
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<ToolDef>>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            tools: None,
            temperature: 0.2,
            max_tokens: None,
        }
    }
    pub fn with_tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.tools = Some(tools);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: &ChatRequest) -> AppResult<ChatResponse>;
    fn model(&self) -> &str;
}

pub struct OpenAiCompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    /// Resolve the chat-completions endpoint. `LLM_BASE_URL` may be given with
    /// or without the `/v1` suffix (Ollama/vLLM/LM Studio differ).
    /// The prefix is used as-is: set `LLM_BASE_URL` to the OpenAI-compatible
    /// root, e.g. `https://api.openai.com/v1` or
    /// `https://generativelanguage.googleapis.com/v1beta/openai` (Gemini).
    fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else {
            format!("{base}/chat/completions")
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn chat(&self, req: &ChatRequest) -> AppResult<ChatResponse> {
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let mut obj = json!({ "role": m.role });
                if let Some(content) = &m.content {
                    obj["content"] = json!(content);
                }
                if !m.tool_calls.is_empty() {
                    let calls: Vec<Value> = m
                        .tool_calls
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id,
                                "type": "function",
                                "function": {
                                    "name": c.name,
                                    "arguments": c.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                    obj["tool_calls"] = json!(calls);
                }
                if let Some(id) = &m.tool_call_id {
                    obj["tool_call_id"] = json!(id);
                }
                obj
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": req.temperature,
        });
        if let Some(max_tokens) = req.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(tools) = &req.tools {
            let defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(defs);
            body["tool_choice"] = json!("auto");
        }

        let mut request = self.client.post(self.endpoint()).json(&body);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::ai(format!("LLM request failed: {e}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::ai(format!("LLM response read failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::ai(format!(
                "LLM returned {status}: {}",
                truncate(&text, 500)
            )));
        }

        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| AppError::ai(format!("LLM returned invalid JSON: {e}")))?;

        let choice = parsed
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| AppError::ai("LLM response missing choices"))?;

        let message = choice.get("message").cloned().unwrap_or(Value::Null);

        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let tool_calls = message
            .get("tool_calls")
            .and_then(|t| t.as_array())
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|c| {
                        let id = c.get("id")?.as_str()?.to_string();
                        let function = c.get("function")?;
                        let name = function.get("name")?.as_str()?.to_string();
                        let arguments = function
                            .get("arguments")
                            .and_then(|a| a.as_str())
                            .map(|s| serde_json::from_str::<Value>(s).unwrap_or(Value::Null))
                            .unwrap_or(Value::Null);
                        Some(ToolCall {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls,
        })
    }

    fn model(&self) -> &str {
        &self.model
    }
}

pub fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ai::llm::{ChatMessage, ChatRequest};
use crate::auth::CurrentUser;
use crate::domain::chat::{
    ChatResponse, Citation, Conversation, Message, SendMessage, ToolCallTrace,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Hard cap on the tool-call loop (docs/decisions.md D10).
const MAX_TOOL_ITERATIONS: usize = 8;

const SYSTEM_PROMPT: &str = "You are a personal work assistant with access to the user's tasks, \
calendar and document knowledge base. Use the provided tools to look up or change data — never \
invent task ids or document contents. When you use retrieve_knowledge, ground your answer in the \
returned excerpts and cite them as [ref N] matching the excerpt numbers. Be concise. If a tool \
returns an error, explain it plainly to the user.";

#[derive(Debug, Deserialize, Default)]
pub struct NewConversation {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RenameConversation {
    #[serde(default)]
    pub title: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/conversations", get(list).post(create))
        .route(
            "/conversations/:id",
            get(get_one).patch(rename).delete(delete_one),
        )
        .route("/conversations/:id/messages", get(messages).post(send))
}

async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<Conversation>>> {
    Ok(Json(crate::db::chat::conversations(&state.pool).await?))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(body): Json<NewConversation>,
) -> AppResult<(StatusCode, Json<Conversation>)> {
    let conv =
        crate::db::chat::create_conversation(&state.pool, body.title.as_deref().unwrap_or(""))
            .await?;
    Ok((StatusCode::CREATED, Json(conv)))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let conv = crate::db::chat::get_conversation(&state.pool, id).await?;
    let messages = crate::db::chat::messages(&state.pool, id).await?;
    Ok(Json(json!({ "conversation": conv, "messages": messages })))
}

async fn rename(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(body): Json<RenameConversation>,
) -> AppResult<Json<Conversation>> {
    let conv = crate::db::chat::rename_conversation(&state.pool, id, &body.title)
        .await?
        .ok_or_else(|| AppError::not_found("conversation not found"))?;
    Ok(Json(conv))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::chat::delete_conversation(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("conversation not found"));
    }
    Ok(Json(json!({ "deleted": true, "id": id })))
}

async fn messages(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<Message>>> {
    crate::db::chat::get_conversation(&state.pool, id).await?;
    Ok(Json(crate::db::chat::messages(&state.pool, id).await?))
}

/// The chat orchestration: user message → (model requests tool → backend runs
/// tool → result fed back) × N → final answer with citations (§6).
async fn send(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(body): Json<SendMessage>,
) -> AppResult<Json<ChatResponse>> {
    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::validation("content is required"));
    }
    crate::db::chat::get_conversation(&state.pool, id).await?;

    // Rebuild model context from persisted history. Only user/assistant text
    // turns are replayed (tool rows are shown in the UI but not re-sent, which
    // keeps the message sequence valid for the provider).
    let history = crate::db::chat::messages(&state.pool, id).await?;
    let system_prompt = format!(
        "{SYSTEM_PROMPT} The current date/time is {} (UTC). Use list_schedule to check the \
         user's calendar and scheduled tasks before answering questions about their schedule.",
        chrono::Utc::now().to_rfc3339()
    );
    let mut messages: Vec<ChatMessage> = vec![ChatMessage::system(system_prompt)];
    for msg in &history {
        match msg.role.as_str() {
            "user" => {
                if let Some(text) = &msg.content {
                    messages.push(ChatMessage::user(text.clone()));
                }
            }
            "assistant" => {
                if let Some(text) = &msg.content {
                    if !text.is_empty() {
                        messages.push(ChatMessage::assistant(Some(text.clone()), vec![]));
                    }
                }
            }
            _ => {}
        }
    }

    let user_message = crate::db::chat::insert_message(
        &state.pool,
        id,
        "user",
        Some(content.clone()),
        None,
        None,
        None,
    )
    .await?;
    messages.push(ChatMessage::user(content));

    let tools = crate::ai::tools::tool_definitions();
    let mut traces: Vec<ToolCallTrace> = Vec::new();
    let mut citations: Vec<Citation> = Vec::new();
    // Retrieved excerpts are injected into the model context once so the final
    // answer can be grounded without the user asking twice (§5).
    let mut context_injected = false;

    for _ in 0..MAX_TOOL_ITERATIONS {
        let request = ChatRequest::new(messages.clone()).with_tools(tools.clone());
        let response = state.llm.chat(&request).await?;

        if response.tool_calls.is_empty() {
            let text = response
                .content
                .clone()
                .unwrap_or_else(|| "(the assistant returned no text)".to_string());
            let citations_value = if citations.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&citations).unwrap_or(Value::Null))
            };
            let message = crate::db::chat::insert_message(
                &state.pool,
                id,
                "assistant",
                Some(text),
                None,
                None,
                citations_value,
            )
            .await?;
            let _ = user_message;
            return Ok(Json(ChatResponse {
                message,
                tool_calls: traces,
                citations,
            }));
        }

        // Persist the assistant's tool-call turn.
        let calls_value = serde_json::to_value(&response.tool_calls).unwrap_or(Value::Null);
        crate::db::chat::insert_message(
            &state.pool,
            id,
            "assistant",
            response.content.clone(),
            Some(calls_value),
            None,
            None,
        )
        .await?;
        messages.push(ChatMessage::assistant(
            response.content.clone(),
            response.tool_calls.clone(),
        ));

        for call in &response.tool_calls {
            let result = crate::ai::tools::execute(&state, &call.name, &call.arguments).await;
            let result_value = match &result {
                Ok(value) => value.clone(),
                Err(e) => json!({ "error": e.to_string() }),
            };

            let mut pending_context: Option<String> = None;
            if call.name == "retrieve_knowledge" {
                if let Some(list) = result_value.get("citations").and_then(|c| c.as_array()) {
                    if let Ok(parsed) =
                        serde_json::from_value::<Vec<Citation>>(Value::Array(list.clone()))
                    {
                        citations = parsed;
                    }
                }
                if !context_injected {
                    if let Some(results) = result_value.get("results") {
                        if let Ok(chunks) = serde_json::from_value::<
                            Vec<crate::ai::rag::RetrievedChunk>,
                        >(results.clone())
                        {
                            let context = crate::ai::rag::build_context(&chunks);
                            if !context.is_empty() {
                                pending_context = Some(context);
                            }
                        }
                    }
                }
            }

            traces.push(ToolCallTrace {
                name: call.name.clone(),
                args: call.arguments.clone(),
                result: result_value.clone(),
            });

            let result_text = result_value.to_string();
            crate::db::chat::insert_message(
                &state.pool,
                id,
                "tool",
                Some(crate::ai::llm::truncate(&result_text, 8_000)),
                None,
                Some(
                    json!([{ "tool_call_id": call.id, "name": call.name, "result": result_value }]),
                ),
                None,
            )
            .await?;

            messages.push(ChatMessage::tool_result(call.id.clone(), result_text));

            // Ground the model with the retrieved excerpts exactly once.
            if let Some(context) = pending_context.take() {
                messages.push(ChatMessage::user(context));
                context_injected = true;
            }
        }
    }

    Err(AppError::ai(format!(
        "tool loop exceeded {MAX_TOOL_ITERATIONS} iterations without a final answer"
    )))
}

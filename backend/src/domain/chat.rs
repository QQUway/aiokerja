use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
    pub citations: Option<Value>,
    pub created_at: DateTime<Utc>,
}

/// Citation attached to a grounded answer (§5). Serde-serialized into the
/// message `citations` jsonb and returned to the frontend for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub document_id: Uuid,
    pub title: String,
    pub chunk_text: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCallTrace>,
    pub citations: Vec<Citation>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallTrace {
    pub name: String,
    pub args: Value,
    pub result: Value,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SendMessage {
    pub content: String,
}

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::chat::{Conversation, Message};
use crate::error::{AppError, AppResult};

pub async fn conversations(pool: &PgPool) -> AppResult<Vec<Conversation>> {
    let convs =
        sqlx::query_as::<_, Conversation>("SELECT * FROM conversations ORDER BY updated_at DESC")
            .fetch_all(pool)
            .await?;
    Ok(convs)
}

pub async fn create_conversation(pool: &PgPool, title: &str) -> AppResult<Conversation> {
    let title = if title.trim().is_empty() {
        "New chat".to_string()
    } else {
        title.trim().to_string()
    };
    let conv = sqlx::query_as::<_, Conversation>(
        "INSERT INTO conversations (title) VALUES ($1) RETURNING *",
    )
    .bind(title)
    .fetch_one(pool)
    .await?;
    Ok(conv)
}

pub async fn get_conversation(pool: &PgPool, id: Uuid) -> AppResult<Conversation> {
    let conv = sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("conversation not found"))?;
    Ok(conv)
}

pub async fn rename_conversation(
    pool: &PgPool,
    id: Uuid,
    title: &str,
) -> AppResult<Option<Conversation>> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::validation("title cannot be empty"));
    }
    let conv = sqlx::query_as::<_, Conversation>(
        "UPDATE conversations SET title = $2, updated_at = now() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(title)
    .fetch_optional(pool)
    .await?;
    Ok(conv)
}

pub async fn delete_conversation(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM conversations WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn messages(pool: &PgPool, conversation_id: Uuid) -> AppResult<Vec<Message>> {
    let msgs = sqlx::query_as::<_, Message>(
        "SELECT * FROM conversation_messages WHERE conversation_id = $1 ORDER BY created_at",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;
    Ok(msgs)
}

pub async fn insert_message(
    pool: &PgPool,
    conversation_id: Uuid,
    role: &str,
    content: Option<String>,
    tool_calls: Option<Value>,
    tool_results: Option<Value>,
    citations: Option<Value>,
) -> AppResult<Message> {
    let title_hint = content.clone().unwrap_or_default();
    let msg = sqlx::query_as::<_, Message>(
        "INSERT INTO conversation_messages \
         (conversation_id, role, content, tool_calls, tool_results, citations) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
    )
    .bind(conversation_id)
    .bind(role)
    .bind(content)
    .bind(tool_calls)
    .bind(tool_results)
    .bind(citations)
    .fetch_one(pool)
    .await?;

    // Keep the conversation fresh in the list view (only the newest message
    // content becomes the title if it is still the default).
    sqlx::query(
        "UPDATE conversations SET updated_at = now(), title = CASE \
                 WHEN title = 'New chat' AND $2 = 'user' THEN LEFT($3, 60) ELSE title END \
                 WHERE id = $1",
    )
    .bind(conversation_id)
    .bind(role)
    .bind(title_hint)
    .execute(pool)
    .await?;
    Ok(msg)
}

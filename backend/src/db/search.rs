use serde::Serialize;
use sqlx::PgPool;

use crate::domain::document::Document;
use crate::domain::task::Task;
use crate::error::AppResult;

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub tasks: Vec<Task>,
    pub documents: Vec<Document>,
}

/// Global search (§7 Stage 5): ILIKE over task title/description and document
/// title/filename. Semantic retrieval lives in the chat `retrieve_knowledge`
/// tool (docs/decisions.md D12).
pub async fn global(pool: &PgPool, query: &str, limit: i64) -> AppResult<SearchResults> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(SearchResults {
            tasks: vec![],
            documents: vec![],
        });
    }
    let tasks = crate::db::tasks::search(pool, q, limit).await?;
    let documents = crate::db::documents::search(pool, q, limit).await?;
    Ok(SearchResults { tasks, documents })
}

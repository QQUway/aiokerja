use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DOC_STATUSES: [&str; 4] = ["pending", "indexing", "indexed", "failed"];

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Document {
    pub id: Uuid,
    pub filename: String,
    pub title: String,
    pub source: String,
    pub doc_type: String,
    pub upload_date: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub token_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct DocumentQuery {
    pub status: Option<String>,
    pub doc_type: Option<String>,
    pub search: Option<String>,
}

/// Stored candidate before embedding. `embedding` filled by the embedder.
pub struct ChunkWithEmbedding {
    pub index: i32,
    pub text: String,
    pub token_count: i32,
    pub embedding: Vec<f32>,
}

pub fn doc_type_from_filename(filename: &str) -> Option<&'static str> {
    let ext = filename.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => Some("pdf"),
        "txt" => Some("txt"),
        "md" => Some("md"),
        "docx" => Some("docx"),
        _ => None,
    }
}

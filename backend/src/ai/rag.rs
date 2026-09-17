use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub title: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub score: f32,
}

/// Reranking interface (stubbed in v1, real cross-encoder in v2 — §5).
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        _query: &str,
        chunks: Vec<RetrievedChunk>,
        top_k: usize,
    ) -> AppResult<Vec<RetrievedChunk>>;
}

/// Default: keep vector-similarity order.
pub struct NoopReranker;

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(
        &self,
        _query: &str,
        mut chunks: Vec<RetrievedChunk>,
        top_k: usize,
    ) -> AppResult<Vec<RetrievedChunk>> {
        chunks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        chunks.truncate(top_k);
        Ok(chunks)
    }
}

/// RAG retrieval: embed query → vector similarity → rerank (noop in v1).
pub async fn retrieve_knowledge(
    pool: &PgPool,
    embeddings: &dyn crate::ai::embeddings::EmbeddingProvider,
    reranker: &dyn Reranker,
    query: &str,
    top_k: usize,
) -> AppResult<Vec<RetrievedChunk>> {
    let query = query.trim();
    if query.is_empty() {
        return Err(AppError::validation("query is required"));
    }
    let vectors = embeddings.embed(&[query.to_string()]).await?;
    debug_assert_eq!(vectors.len(), 1);

    let hits = crate::db::documents::retrieve_chunks(pool, &vectors[0], top_k as i64 * 4).await?;
    let chunks: Vec<RetrievedChunk> = hits
        .into_iter()
        .map(|h| RetrievedChunk {
            chunk_id: h.chunk_id,
            document_id: h.document_id,
            title: h.title,
            chunk_index: h.chunk_index,
            chunk_text: h.chunk_text,
            score: h.score as f32,
        })
        .collect();

    let okay = reranker.rerank(query, chunks, top_k).await?;

    // Hard threshold: similarity < 0.30 is noise, not a hit. 0.30 is lenient;
    // cosine similarity of unrelated text is typically much lower than this.
    Ok(okay.into_iter().filter(|c| c.score >= 0.30).collect())
}

/// Context construction is kept separate from the system prompt (§5): the
/// retrieved text is assembled here and passed as a distinct user/context
/// message with explicit citations.
pub fn build_context(chunks: &[RetrievedChunk]) -> String {
    if chunks.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "Knowledge base excerpts (treat as authoritative, cite them in your answer \
         with [ref N] where N is the excerpt number):\n",
    );
    for (i, chunk) in chunks.iter().enumerate() {
        out.push_str(&format!(
            "\n[{i}] (document: {}, chunk {})\n{}\n",
            chunk.title, chunk.chunk_index, chunk.chunk_text
        ));
    }
    out
}

pub fn citations_from(chunks: &[RetrievedChunk]) -> Vec<crate::domain::chat::Citation> {
    chunks
        .iter()
        .map(|c| crate::domain::chat::Citation {
            document_id: c.document_id,
            title: c.title.clone(),
            chunk_text: c.chunk_text.clone(),
            score: c.score,
        })
        .collect()
}

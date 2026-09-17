use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::chunk::{chunk_text, HeuristicTokenizer};
use crate::ai::embeddings::EmbeddingProvider;
use crate::config::Config;
use crate::domain::document::{doc_type_from_filename, ChunkWithEmbedding};
use crate::error::{AppError, AppResult};

/// Ingest one document: extract → chunk → embed → store (docs/architecture.md
/// RAG pipeline). Marks the document `indexed` or `failed` and always returns
/// an explicit Result — no panics.
pub async fn ingest_document(
    pool: &PgPool,
    config: &Config,
    embeddings: &dyn EmbeddingProvider,
    document_id: Uuid,
) -> AppResult<()> {
    let doc = crate::db::documents::get(pool, document_id)
        .await?
        .ok_or_else(|| AppError::not_found("document disappeared during ingestion"))?;

    crate::db::documents::set_status(pool, document_id, "indexing", None).await?;

    let raw = crate::db::documents::raw_text(pool, document_id).await?;
    let raw = match raw {
        Some(text) => text,
        None => {
            let err = "document has no raw text (save_raw_text not called before ingestion)";
            crate::db::documents::set_status(pool, document_id, "failed", Some(err)).await?;
            return Err(AppError::bad_request(err));
        }
    };

    let tokenizer = HeuristicTokenizer;
    let chunks = chunk_text(
        &raw,
        config.rag_chunk_size_tokens.max(1),
        config.rag_chunk_overlap_tokens,
        &tokenizer,
    );

    if chunks.is_empty() {
        let err = "no chunks produced (empty extraction)";
        crate::db::documents::set_status(pool, document_id, "failed", Some(err)).await?;
        return Err(AppError::bad_request(err));
    }

    // Embed all chunks in one batch request.
    let texts: Vec<String> = chunks.iter().map(|(text, _)| text.clone()).collect();
    let vectors = embeddings.embed(&texts).await?;
    if vectors.len() != chunks.len() {
        return Err(AppError::ai(format!(
            "embedding count {} != chunk count {}",
            vectors.len(),
            chunks.len()
        )));
    }

    let stored: Vec<ChunkWithEmbedding> = chunks
        .into_iter()
        .zip(vectors)
        .enumerate()
        .map(|(i, ((text, tokens), embedding))| ChunkWithEmbedding {
            index: i as i32,
            text,
            token_count: tokens as i32,
            embedding,
        })
        .collect();

    crate::db::documents::delete_chunks(pool, document_id).await?;
    crate::db::documents::insert_chunks(pool, document_id, &stored).await?;
    crate::db::documents::set_status(pool, document_id, "indexed", None).await?;

    tracing::info!(
        document_id = %document_id,
        filename = %doc.filename,
        chunks = stored.len(),
        "document indexed"
    );
    Ok(())
}

/// Re-ingest a document whose title/type/extraction already exist. Used by
/// POST /api/documents/:id/reindex.
pub async fn reindex(
    pool: &PgPool,
    config: &Config,
    embeddings: &dyn EmbeddingProvider,
    document_id: Uuid,
) -> AppResult<()> {
    ingest_document(pool, config, embeddings, document_id).await?;
    Ok(())
}

/// Process every stuck-`pending` document serially. Called once at startup
/// (docs/decisions.md D5).
pub async fn process_pending(
    pool: PgPool,
    config: std::sync::Arc<Config>,
    embeddings: std::sync::Arc<dyn EmbeddingProvider>,
) {
    let ids = match crate::db::documents::pending_document_ids(&pool).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "failed to load pending documents at startup");
            return;
        }
    };
    for id in ids {
        tracing::info!(document_id = %id, "processing pending document");
        if let Err(e) = ingest_document(&pool, &config, embeddings.as_ref(), id).await {
            tracing::error!(document_id = %id, error = %e, "pending document failed");
        }
    }
}

/// Extract helper used by the upload handler: saves raw text and returns the
/// extracted (possibly truncated) preview. Shared with ingest_document.
pub async fn extract_and_store(
    pool: &PgPool,
    document_id: Uuid,
    filename: &str,
    bytes: &[u8],
) -> AppResult<String> {
    if doc_type_from_filename(filename).is_none() {
        return Err(AppError::bad_request(
            "unsupported file type (supported: pdf, txt, md, docx)",
        ));
    }
    let text = crate::ai::extract::extract_text(filename, bytes)?;
    crate::db::documents::save_raw_text(pool, document_id, &text).await?;
    Ok(text)
}

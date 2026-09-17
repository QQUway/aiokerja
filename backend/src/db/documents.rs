use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::domain::document::{ChunkWithEmbedding, Document, DocumentChunk, DocumentQuery};
use crate::error::AppResult;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChunkHit {
    pub chunk_id: Uuid,
    pub document_id: Uuid,
    pub title: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub score: f64,
}

pub async fn list(pool: &PgPool, q: &DocumentQuery) -> AppResult<Vec<Document>> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM documents WHERE 1=1");
    if let Some(status) = &q.status {
        if !crate::domain::document::DOC_STATUSES.contains(&status.as_str()) {
            return Err(crate::error::AppError::validation(format!(
                "status must be one of {:?}",
                crate::domain::document::DOC_STATUSES
            )));
        }
        qb.push(" AND status = ").push_bind(status.clone());
    }
    if let Some(doc_type) = &q.doc_type {
        qb.push(" AND doc_type = ").push_bind(doc_type.clone());
    }
    if let Some(search) = &q.search {
        let like = format!("%{}%", search.trim());
        qb.push(" AND (title ILIKE ").push_bind(like.clone());
        qb.push(" OR filename ILIKE ").push_bind(like);
        qb.push(")");
    }
    qb.push(" ORDER BY upload_date DESC");
    let docs: Vec<Document> = qb.build_query_as().fetch_all(pool).await?;
    Ok(docs)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<Document>> {
    let doc = sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(doc)
}

pub async fn create(
    pool: &PgPool,
    filename: &str,
    title: &str,
    doc_type: &str,
    content_hash: &str,
) -> AppResult<Document> {
    let doc = sqlx::query_as::<_, Document>(
        "INSERT INTO documents (filename, title, doc_type, content_hash, status) \
         VALUES ($1,$2,$3,$4,'pending') RETURNING *",
    )
    .bind(filename)
    .bind(title)
    .bind(doc_type)
    .bind(content_hash)
    .fetch_one(pool)
    .await?;
    Ok(doc)
}

pub async fn find_by_hash(pool: &PgPool, content_hash: &str) -> AppResult<Option<Document>> {
    let doc = sqlx::query_as::<_, Document>(
        "SELECT * FROM documents WHERE content_hash = $1 ORDER BY upload_date LIMIT 1",
    )
    .bind(content_hash)
    .fetch_optional(pool)
    .await?;
    Ok(doc)
}

pub async fn set_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    error: Option<&str>,
) -> AppResult<()> {
    sqlx::query("UPDATE documents SET status = $2, error = $3, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .bind(error)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn save_raw_text(pool: &PgPool, id: Uuid, text: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO document_extra (document_id, raw_text) VALUES ($1,$2) \
         ON CONFLICT (document_id) DO UPDATE SET raw_text = EXCLUDED.raw_text",
    )
    .bind(id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn raw_text(pool: &PgPool, id: Uuid) -> AppResult<Option<String>> {
    let text: Option<String> =
        sqlx::query_scalar("SELECT raw_text FROM document_extra WHERE document_id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(text)
}

pub async fn chunks(pool: &PgPool, id: Uuid) -> AppResult<Vec<DocumentChunk>> {
    let chunks = sqlx::query_as::<_, DocumentChunk>(
        "SELECT id, document_id, chunk_index, chunk_text, token_count, created_at \
         FROM document_chunks WHERE document_id = $1 ORDER BY chunk_index",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(chunks)
}

pub async fn delete_chunks(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM document_chunks WHERE document_id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_chunks(
    pool: &PgPool,
    document_id: Uuid,
    chunks: &[ChunkWithEmbedding],
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    for chunk in chunks {
        let embedding = pgvector::Vector::from(chunk.embedding.clone());
        sqlx::query(
            "INSERT INTO document_chunks (document_id, chunk_index, chunk_text, token_count, embedding) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(document_id)
        .bind(chunk.index)
        .bind(&chunk.text)
        .bind(chunk.token_count)
        .bind(embedding)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn counts(pool: &PgPool) -> AppResult<(i64, i64)> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(pool)
        .await?;
    let indexed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE status = 'indexed'")
            .fetch_one(pool)
            .await?;
    Ok((total, indexed))
}

pub async fn search(pool: &PgPool, query: &str, limit: i64) -> AppResult<Vec<Document>> {
    let like = format!("%{}%", query.trim());
    let docs = sqlx::query_as::<_, Document>(
        "SELECT * FROM documents WHERE title ILIKE $1 OR filename ILIKE $1 \
         ORDER BY upload_date DESC LIMIT $2",
    )
    .bind(like)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(docs)
}

/// Documents stuck in `indexing` from a previous process — reset to pending on
/// startup (docs/decisions.md D5).
pub async fn reset_stuck_indexing(pool: &PgPool) -> AppResult<u64> {
    let res = sqlx::query(
        "UPDATE documents SET status = 'pending', error = NULL, updated_at = now() \
         WHERE status = 'indexing'",
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn pending_document_ids(pool: &PgPool) -> AppResult<Vec<Uuid>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM documents WHERE status = 'pending' ORDER BY upload_date",
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

/// Vector similarity search (cosine). `embedding` must match the configured
/// dimension (see docs/decisions.md D2).
pub async fn retrieve_chunks(
    pool: &PgPool,
    embedding: &[f32],
    top_k: i64,
) -> AppResult<Vec<ChunkHit>> {
    let vector = pgvector::Vector::from(embedding.to_vec());
    let hits = sqlx::query_as::<_, ChunkHit>(
        "SELECT c.id AS chunk_id, c.document_id, d.title, c.chunk_index, c.chunk_text, \
                 (1 - (c.embedding <=> $1))::float8 AS score \
         FROM document_chunks c JOIN documents d ON d.id = c.document_id \
         WHERE c.embedding IS NOT NULL \
         ORDER BY c.embedding <=> $1 LIMIT $2",
    )
    .bind(vector)
    .bind(top_k)
    .fetch_all(pool)
    .await?;
    Ok(hits)
}

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::config::Config;
use crate::error::AppResult;

/// Create pool, run static migrations, then apply the config-driven vector
/// schema (dimension comes from EMBEDDING_DIMENSIONS, see docs/decisions.md D2).
pub async fn init_db(config: &Config) -> AppResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    ensure_embedding_schema(&pool, config).await?;

    Ok(pool)
}

/// Idempotent: add `embedding vector(dim)` + HNSW index and record the dim in
/// `embedding_index_meta`. Fail loudly on dimension mismatch with what is
/// already indexed (re-index required, see README).
pub async fn ensure_embedding_schema(pool: &PgPool, config: &Config) -> AppResult<()> {
    let dim = i32::try_from(config.embedding_dimensions)
        .map_err(|_| crate::error::AppError::validation("EMBEDDING_DIMENSIONS out of range"))?;

    let existing: Option<(i32, String)> =
        sqlx::query_as("SELECT dimensions, model FROM embedding_index_meta WHERE id = true")
            .fetch_optional(pool)
            .await?;

    if let Some((d, m)) = &existing {
        if *d != dim || *m != config.embedding_model {
            return Err(crate::error::AppError::conflict(format!(
                "embedding index built for dimension {} model '{}', configured {} model '{}'. \
                 Re-index required: delete document_chunks and restart, or align config.",
                d, m, dim, config.embedding_model
            )));
        }
    }

    sqlx::query(&format!(
        "ALTER TABLE document_chunks ADD COLUMN IF NOT EXISTS embedding vector({dim})"
    ))
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS document_chunks_embedding_idx ON document_chunks \
         USING hnsw (embedding vector_cosine_ops)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO embedding_index_meta (id, dimensions, model) VALUES (true, $1, $2) \
         ON CONFLICT (id) DO UPDATE SET dimensions = EXCLUDED.dimensions, \
         model = EXCLUDED.model, updated_at = now()",
    )
    .bind(dim)
    .bind(&config.embedding_model)
    .execute(pool)
    .await?;

    Ok(())
}

pub mod chat;
pub mod dashboard;
pub mod documents;
pub mod events;
pub mod projects;
pub mod search;
pub mod tags;
pub mod tasks;

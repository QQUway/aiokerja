use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::domain::document::{doc_type_from_filename, Document, DocumentChunk, DocumentQuery};
use crate::domain::product::Product;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/documents", get(list).post(upload))
        .route("/documents/:id", get(get_one).delete(delete_one))
        .route("/documents/:id/raw", get(raw))
        .route("/documents/:id/chunks", get(chunks))
        .route("/documents/:id/reindex", post(reindex))
        .route("/documents/:id/extract-datasheet", post(extract_datasheet))
}

async fn list(
    State(state): State<AppState>,
    Query(query): Query<DocumentQuery>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<Document>>> {
    Ok(Json(crate::db::documents::list(&state.pool, &query).await?))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let doc = crate::db::documents::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("document not found"))?;
    let chunk_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_chunks WHERE document_id = $1")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;
    let raw_len: Option<i64> =
        sqlx::query_scalar("SELECT LENGTH(raw_text)::bigint FROM document_extra WHERE document_id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(json!({
        "document": doc,
        "chunk_count": chunk_count,
        "raw_text_length": raw_len.unwrap_or(0),
    })))
}

/// Upload a document (multipart field `file`), extract its text synchronously,
/// then embed + chunk in the background so the upload returns fast.
async fn upload(
    State(state): State<AppState>,
    _user: CurrentUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Document>)> {
    let mut filename: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("multipart parse failed: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            filename = field.file_name().map(String::from).or(filename);
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::bad_request(format!("file read failed: {e}")))?
                    .to_vec(),
            );
        }
    }

    let filename = filename.ok_or_else(|| AppError::bad_request("missing file part"))?;
    let bytes = bytes.ok_or_else(|| AppError::bad_request("missing file content"))?;
    if bytes.is_empty() {
        return Err(AppError::bad_request("file is empty"));
    }
    let doc_type = doc_type_from_filename(&filename).ok_or_else(|| {
        AppError::bad_request("unsupported file type (supported: pdf, txt, md, docx)")
    })?;

    use sha2::{Digest, Sha256};
    let content_hash = hex::encode(Sha256::digest(&bytes));

    if let Some(existing) = crate::db::documents::find_by_hash(&state.pool, &content_hash).await? {
        return Err(AppError::conflict(format!(
            "identical file already uploaded as '{}' (id {})",
            existing.title, existing.id
        )));
    }

    let title = std::path::Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&filename)
        .to_string();

    let doc = crate::db::documents::create(&state.pool, &filename, &title, doc_type, &content_hash)
        .await?;

    match crate::ai::ingest::extract_and_store(&state.pool, doc.id, &filename, &bytes).await {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(document_id = %doc.id, error = %e, "extraction failed");
            crate::db::documents::set_status(&state.pool, doc.id, "failed", Some(&e.to_string()))
                .await?;
            let staged = crate::db::documents::get(&state.pool, doc.id).await?;
            return Ok((
                StatusCode::CREATED,
                Json(staged.ok_or_else(|| AppError::not_found("document vanished after upload"))?),
            ));
        }
    }

    // Full pipeline (chunk + embed + store) runs off the request thread.
    let document_id = doc.id;
    let pool = state.pool.clone();
    let config = state.config.clone();
    let embeddings = state.embeddings.clone();
    tokio::spawn(async move {
        if let Err(e) =
            crate::ai::ingest::ingest_document(&pool, &config, embeddings.as_ref(), document_id)
                .await
        {
            tracing::error!(document_id = %document_id, error = %e, "ingestion failed");
        }
    });

    let doc = crate::db::documents::get(&state.pool, document_id)
        .await?
        .ok_or_else(|| AppError::not_found("document vanished after upload"))?;
    Ok((StatusCode::CREATED, Json(doc)))
}

async fn raw(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<axum::response::Response> {
    crate::db::documents::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("document not found"))?;
    let text = crate::db::documents::raw_text(&state.pool, id)
        .await?
        .unwrap_or_default();
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response())
}

async fn chunks(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<DocumentChunk>>> {
    crate::db::documents::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("document not found"))?;
    Ok(Json(crate::db::documents::chunks(&state.pool, id).await?))
}

/// Re-run the pipeline for a document (e.g. after changing RAG settings
/// affecting only chunking). Embedding model/dimension changes still require a
/// full re-index (see README).
async fn reindex(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    crate::db::documents::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("document not found"))?;

    let pool = state.pool.clone();
    let config = state.config.clone();
    let embeddings = state.embeddings.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::ai::ingest::reindex(&pool, &config, embeddings.as_ref(), id).await {
            tracing::error!(document_id = %id, error = %e, "reindex failed");
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "status": "reindexing", "document_id": id })),
    ))
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ExtractDatasheetRequest {
    device_type: Option<String>,
}

/// Extract structured specs from an AIDC hardware datasheet's extracted text
/// and upsert the resulting product (one product per source document).
async fn extract_datasheet(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(body): Json<ExtractDatasheetRequest>,
) -> AppResult<Json<Product>> {
    let product =
        crate::ai::datasheet::extract_and_save(&state, id, body.device_type.as_deref()).await?;
    Ok(Json(product))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::documents::delete(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("document not found"));
    }
    Ok(Json(json!({ "deleted": true, "id": id })))
}

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let db_ok: bool = sqlx::query_scalar("SELECT true")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(false);

    Ok(Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "database": db_ok,
        "app_url": state.config.app_url,
        "llm_provider": state.config.llm_provider,
        "llm_model": state.llm.model(),
        "embedding_provider": state.config.embedding_provider,
        "embedding_model": state.embeddings.model(),
        "embedding_dimensions": state.embeddings.dimensions(),
    })))
}

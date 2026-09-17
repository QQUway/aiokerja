use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::db::search::SearchResults;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
    _user: CurrentUser,
) -> AppResult<Json<SearchResults>> {
    if params.q.trim().is_empty() {
        return Err(AppError::validation("search query `q` is required"));
    }
    Ok(Json(
        crate::db::search::global(&state.pool, &params.q, 25).await?,
    ))
}

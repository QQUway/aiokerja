use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::auth::CurrentUser;
use crate::db::dashboard::Dashboard;
use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/dashboard", get(dashboard))
}

async fn dashboard(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> AppResult<Json<Dashboard>> {
    Ok(Json(crate::db::dashboard::build(&state.pool).await?))
}

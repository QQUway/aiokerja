use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::domain::tag::{NewTag, Tag, UpdateTag};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tags", get(list).post(create))
        .route("/tags/:id", get(get_one).patch(update).delete(delete_one))
}

async fn list(State(state): State<AppState>, _user: CurrentUser) -> AppResult<Json<Vec<Tag>>> {
    Ok(Json(crate::db::tags::list(&state.pool).await?))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<Tag>> {
    let tag = crate::db::tags::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("tag not found"))?;
    Ok(Json(tag))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(new): Json<NewTag>,
) -> AppResult<(StatusCode, Json<Tag>)> {
    let tag = crate::db::tags::create(&state.pool, &new).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(update): Json<UpdateTag>,
) -> AppResult<Json<Tag>> {
    let tag = crate::db::tags::update(&state.pool, id, &update)
        .await?
        .ok_or_else(|| AppError::not_found("tag not found"))?;
    Ok(Json(tag))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::tags::delete(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("tag not found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::domain::project::{NewProject, Project, UpdateProject};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct DeleteParams {
    #[serde(default)]
    pub cascade: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list).post(create))
        .route(
            "/projects/:id",
            get(get_one).patch(update).delete(delete_one),
        )
}

async fn list(State(state): State<AppState>, _user: CurrentUser) -> AppResult<Json<Vec<Project>>> {
    let projects = crate::db::projects::list(&state.pool).await?;
    Ok(Json(projects))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let project = crate::db::projects::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("project not found"))?;
    let task_count = crate::db::projects::task_count(&state.pool, id).await?;
    Ok(Json(
        json!({ "project": project, "task_count": task_count }),
    ))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(new): Json<NewProject>,
) -> AppResult<(StatusCode, Json<Project>)> {
    let project = crate::db::projects::create(&state.pool, &new).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(update): Json<UpdateProject>,
) -> AppResult<Json<Project>> {
    let project = crate::db::projects::update(&state.pool, id, &update)
        .await?
        .ok_or_else(|| AppError::not_found("project not found"))?;
    Ok(Json(project))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<DeleteParams>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::projects::delete(&state.pool, id, params.cascade).await?;
    if !deleted {
        return Err(AppError::not_found("project not found"));
    }
    Ok(Json(json!({ "deleted": true, "id": id })))
}

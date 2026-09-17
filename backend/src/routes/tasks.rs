use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::db::tasks::TaskFilter;
use crate::domain::task::{NewTask, TaskDetail, TaskListItem, TaskPage, TaskSummary, UpdateTask};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tasks", get(list).post(create))
        .route("/tasks/views/summary", get(summary))
        .route("/tasks/:id", get(get_one).patch(update).delete(delete_one))
        .route(
            "/tasks/:id/tags/:tag_id",
            post(attach_tag).delete(detach_tag),
        )
        .route("/tasks/:id/calendar-event", post(materialize_event))
}

async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(filter): Query<TaskFilter>,
) -> AppResult<Json<TaskPage>> {
    let (tasks, total) = crate::db::tasks::list(&state.pool, &filter).await?;
    let ids: Vec<Uuid> = tasks.iter().map(|t| t.id).collect();
    let mut tag_map = crate::db::tasks::tags_for_many(&state.pool, &ids).await?;
    let items = tasks
        .into_iter()
        .map(|task| {
            let tags = tag_map.remove(&task.id).unwrap_or_default();
            TaskListItem { task, tags }
        })
        .collect();
    Ok(Json(TaskPage { items, total }))
}

async fn detail(state: &AppState, id: Uuid) -> AppResult<TaskDetail> {
    let task = crate::db::tasks::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("task not found"))?;
    let tags = crate::db::tasks::tags_for(&state.pool, id).await?;
    let subtasks = crate::db::tasks::subtasks(&state.pool, id).await?;
    Ok(TaskDetail {
        task,
        tags,
        subtasks,
    })
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<TaskDetail>> {
    Ok(Json(detail(&state, id).await?))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(new): Json<NewTask>,
) -> AppResult<(StatusCode, Json<TaskDetail>)> {
    let task = crate::db::tasks::create(&state.pool, &new).await?;
    let detail = detail(&state, task.id).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(update): Json<UpdateTask>,
) -> AppResult<Json<TaskDetail>> {
    crate::db::tasks::update(&state.pool, id, &update)
        .await?
        .ok_or_else(|| AppError::not_found("task not found"))?;
    Ok(Json(detail(&state, id).await?))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::tasks::delete(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("task not found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

async fn attach_tag(
    State(state): State<AppState>,
    Path((id, tag_id)): Path<(Uuid, Uuid)>,
    _user: CurrentUser,
) -> AppResult<Json<TaskDetail>> {
    if crate::db::tasks::get(&state.pool, id).await?.is_none() {
        return Err(AppError::not_found("task not found"));
    }
    if crate::db::tags::get(&state.pool, tag_id).await?.is_none() {
        return Err(AppError::not_found("tag not found"));
    }
    crate::db::tasks::attach_tag(&state.pool, id, tag_id).await?;
    Ok(Json(detail(&state, id).await?))
}

async fn detach_tag(
    State(state): State<AppState>,
    Path((id, tag_id)): Path<(Uuid, Uuid)>,
    _user: CurrentUser,
) -> AppResult<Json<TaskDetail>> {
    if crate::db::tasks::get(&state.pool, id).await?.is_none() {
        return Err(AppError::not_found("task not found"));
    }
    crate::db::tasks::detach_tag(&state.pool, id, tag_id).await?;
    Ok(Json(detail(&state, id).await?))
}

/// Materialize a calendar event from a task's scheduled window (docs/decisions
/// D3). Read-time merging already shows the task on the calendar; this creates
/// a real, editable `calendar_events` row linked back via `source_task_id`.
async fn materialize_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<(StatusCode, Json<crate::domain::event::CalendarEvent>)> {
    let task = crate::db::tasks::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("task not found"))?;

    let start = task.scheduled_start.or(task.due_date).ok_or_else(|| {
        AppError::validation("task has no scheduled_start or due_date to put on the calendar")
    })?;
    let end = task
        .scheduled_end
        .unwrap_or(start + chrono::Duration::hours(1));

    let new = crate::domain::event::NewEvent {
        title: task.title.clone(),
        description: task.description.clone(),
        start_time: Some(start),
        end_time: Some(end),
        all_day: false,
        recurrence_rule: task.recurrence_rule.clone(),
        reminder_minutes: None,
        source_task_id: Some(task.id),
    };
    let event = crate::db::events::create(&state.pool, &new).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

async fn summary(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> AppResult<Json<TaskSummary>> {
    Ok(Json(crate::db::tasks::summary(&state.pool).await?))
}

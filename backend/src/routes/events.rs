use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::domain::event::{CalendarEvent, CalendarItem, NewEvent, UpdateEvent};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", get(list).post(create))
        .route("/events/:id", get(get_one).patch(update).delete(delete_one))
        .route("/calendar", get(calendar))
}

#[derive(Debug, Deserialize, Default)]
pub struct RangeParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

async fn list(
    State(state): State<AppState>,
    Query(params): Query<RangeParams>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<CalendarEvent>>> {
    let from = params.from.unwrap_or_else(|| {
        Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))
            .unwrap_or_else(Utc::now)
    });
    let to = params.to.unwrap_or(from + Duration::days(30));
    Ok(Json(
        crate::db::events::events_in_range(&state.pool, from, to).await?,
    ))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<CalendarEvent>> {
    let event = crate::db::events::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found("event not found"))?;
    Ok(Json(event))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(new): Json<NewEvent>,
) -> AppResult<(StatusCode, Json<CalendarEvent>)> {
    let event = crate::db::events::create(&state.pool, &new).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(update): Json<UpdateEvent>,
) -> AppResult<Json<CalendarEvent>> {
    let event = crate::db::events::update(&state.pool, id, &update)
        .await?
        .ok_or_else(|| AppError::not_found("event not found"))?;
    Ok(Json(event))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let deleted = crate::db::events::delete(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("event not found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

/// Merged calendar read: real events + task projections (§4, docs/decisions
/// D3). Tasks appear when they have a due_date or scheduled window in range.
async fn calendar(
    State(state): State<AppState>,
    Query(params): Query<RangeParams>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<CalendarItem>>> {
    let from = params.from.unwrap_or_else(|| {
        Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|d| DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))
            .unwrap_or_else(Utc::now)
    });
    let to = params.to.unwrap_or(from + Duration::days(30));
    let items = crate::db::events::calendar_items(&state.pool, from, to).await?;
    Ok(Json(items))
}

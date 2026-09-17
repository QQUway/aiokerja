use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::domain::event::CalendarEvent;
use crate::domain::task::{Task, TaskSummary};
use crate::error::AppResult;

#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub tasks: TaskSummary,
    pub projects_count: i64,
    pub documents_count: i64,
    pub indexed_documents: i64,
    pub upcoming_events: Vec<CalendarEvent>,
    pub recent_tasks: Vec<Task>,
}

pub async fn build(pool: &PgPool) -> AppResult<Dashboard> {
    let tasks = crate::db::tasks::summary(pool).await?;
    let projects_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(pool)
        .await?;
    let (documents_count, indexed_documents) = crate::db::documents::counts(pool).await?;

    let now = Utc::now();
    let horizon: DateTime<Utc> = now + Duration::days(14);
    let upcoming_events = crate::db::events::events_in_range(pool, now, horizon).await?;

    let recent_tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE status NOT IN ('done','cancelled') \
         ORDER BY COALESCE(scheduled_start, due_date) NULLS LAST, created_at DESC LIMIT 10",
    )
    .fetch_all(pool)
    .await?;

    Ok(Dashboard {
        tasks,
        projects_count,
        documents_count,
        indexed_documents,
        upcoming_events,
        recent_tasks,
    })
}

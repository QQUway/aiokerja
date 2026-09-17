use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub reminder_minutes: Option<i32>,
    pub source_task_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct NewEvent {
    pub title: String,
    pub description: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub reminder_minutes: Option<i32>,
    pub source_task_id: Option<Uuid>,
}

impl NewEvent {
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.title.trim().is_empty() {
            return Err(crate::error::AppError::validation("title is required"));
        }
        let start = self.start_time.unwrap_or(chrono::Utc::now());
        let end = self.end_time.unwrap_or(start + chrono::Duration::hours(1));
        if end <= start {
            return Err(crate::error::AppError::validation(
                "end_time must be after start_time",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct UpdateEvent {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub all_day: Option<bool>,
    pub recurrence_rule: Option<Option<String>>,
    pub reminder_minutes: Option<Option<i32>>,
    pub source_task_id: Option<Option<Uuid>>,
}

/// Merged calendar item: a `calendar_event` row or a read-only projection of a
/// task with dates set (see docs/decisions.md D3).
#[derive(Debug, Serialize)]
pub struct CalendarItem {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub all_day: bool,
    pub recurrence_rule: Option<String>,
    pub reminder_minutes: Option<i32>,
    pub source_task_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project_id: Option<Uuid>,
}

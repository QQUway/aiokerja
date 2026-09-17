use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const STATUSES: [&str; 4] = ["todo", "in_progress", "done", "cancelled"];
pub const PRIORITIES: [&str; 4] = ["low", "medium", "high", "urgent"];

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<DateTime<Utc>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub recurrence_rule: Option<String>,
    pub project_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NewTask {
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<DateTime<Utc>>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub recurrence_rule: Option<String>,
    pub project_id: Option<Uuid>,
    pub parent_task_id: Option<Uuid>,
    pub tag_ids: Vec<Uuid>,
}

impl Default for NewTask {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: None,
            status: "todo".into(),
            priority: "medium".into(),
            due_date: None,
            scheduled_start: None,
            scheduled_end: None,
            recurrence_rule: None,
            project_id: None,
            parent_task_id: None,
            tag_ids: Vec::new(),
        }
    }
}

impl NewTask {
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.title.trim().is_empty() {
            return Err(crate::error::AppError::validation("title is required"));
        }
        if !STATUSES.contains(&self.status.as_str()) {
            return Err(crate::error::AppError::validation(format!(
                "status must be one of {STATUSES:?}"
            )));
        }
        if !PRIORITIES.contains(&self.priority.as_str()) {
            return Err(crate::error::AppError::validation(format!(
                "priority must be one of {PRIORITIES:?}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<Option<DateTime<Utc>>>,
    pub scheduled_start: Option<Option<DateTime<Utc>>>,
    pub scheduled_end: Option<Option<DateTime<Utc>>>,
    pub recurrence_rule: Option<Option<String>>,
    pub project_id: Option<Option<Uuid>>,
    pub parent_task_id: Option<Option<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct TaskDetail {
    #[serde(flatten)]
    pub task: Task,
    pub tags: Vec<crate::domain::tag::Tag>,
    pub subtasks: Vec<Task>,
}

#[derive(Debug, Serialize)]
pub struct TaskListItem {
    #[serde(flatten)]
    pub task: Task,
    pub tags: Vec<crate::domain::tag::Tag>,
}

#[derive(Debug, Serialize)]
pub struct TaskPage {
    pub items: Vec<TaskListItem>,
    pub total: u64,
}

#[derive(Debug, Serialize, Default)]
pub struct TaskSummary {
    pub total: u64,
    pub by_status: std::collections::HashMap<String, u64>,
    pub today_count: u64,
    pub upcoming_count: u64,
    pub overdue_count: u64,
}

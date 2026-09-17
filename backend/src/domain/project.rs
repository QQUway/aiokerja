use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
    pub color: String,
}

impl NewProject {
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.name.trim().is_empty() {
            return Err(crate::error::AppError::validation("name is required"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub color: Option<String>,
}

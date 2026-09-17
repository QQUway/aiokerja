use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub attributes: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct NewProduct {
    pub name: String,
    pub category: Option<String>,
    pub attributes: Value,
}

impl NewProduct {
    pub fn validate(&self) -> Result<(), crate::error::AppError> {
        if self.name.trim().is_empty() {
            return Err(crate::error::AppError::validation("name is required"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub category: Option<Option<String>>,
    pub attributes: Option<Value>,
}

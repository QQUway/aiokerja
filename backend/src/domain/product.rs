use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// AIDC hardware categories the datasheet extractor understands (§datasheet).
/// Kept as plain strings (not a DB enum) so new categories don't need a migration.
pub const DEVICE_TYPES: &[&str] = &[
    "rfid_reader",
    "rfid_antenna",
    "handheld_computer",
    "barcode_printer",
    "rfid_printer",
    "scanner",
    "other",
];

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub attributes: Value,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub device_type: Option<String>,
    pub source_document_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct NewProduct {
    pub name: String,
    pub category: Option<String>,
    pub attributes: Value,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub device_type: Option<String>,
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
    pub brand: Option<Option<String>>,
    pub model: Option<Option<String>>,
    pub device_type: Option<Option<String>>,
}

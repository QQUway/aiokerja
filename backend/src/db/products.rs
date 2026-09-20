use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::datasheet::DatasheetExtraction;
use crate::domain::product::Product;
use crate::error::AppResult;

pub async fn get_by_ids(pool: &PgPool, ids: &[Uuid]) -> AppResult<Vec<Product>> {
    let products = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(pool)
        .await?;
    Ok(products)
}

pub async fn find_by_source_document(pool: &PgPool, document_id: Uuid) -> AppResult<Option<Product>> {
    let product =
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE source_document_id = $1")
            .bind(document_id)
            .fetch_optional(pool)
            .await?;
    Ok(product)
}

/// Create or update the product extracted from a datasheet. One product per
/// source document: re-extracting overwrites the previous extraction.
pub async fn upsert_from_datasheet(
    pool: &PgPool,
    document_id: Uuid,
    fallback_name: &str,
    extraction: &DatasheetExtraction,
) -> AppResult<Product> {
    let name = match (&extraction.brand, &extraction.model) {
        (Some(brand), Some(model)) => format!("{brand} {model}"),
        (Some(brand), None) => brand.clone(),
        (None, Some(model)) => model.clone(),
        (None, None) => fallback_name.to_string(),
    };

    let existing = find_by_source_document(pool, document_id).await?;
    let product = if let Some(existing) = existing {
        sqlx::query_as::<_, Product>(
            "UPDATE products SET name = $2, brand = $3, model = $4, device_type = $5, \
             attributes = $6, updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(existing.id)
        .bind(&name)
        .bind(&extraction.brand)
        .bind(&extraction.model)
        .bind(&extraction.device_type)
        .bind(&extraction.attributes)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as::<_, Product>(
            "INSERT INTO products (name, brand, model, device_type, attributes, source_document_id) \
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
        )
        .bind(&name)
        .bind(&extraction.brand)
        .bind(&extraction.model)
        .bind(&extraction.device_type)
        .bind(&extraction.attributes)
        .bind(document_id)
        .fetch_one(pool)
        .await?
    };
    Ok(product)
}

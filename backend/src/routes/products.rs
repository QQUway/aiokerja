use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::domain::product::{NewProduct, Product, UpdateProduct};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products", get(list).post(create))
        .route(
            "/products/:id",
            get(get_one).patch(update).delete(delete_one),
        )
        .route("/products/compare", post(compare))
}

async fn list(State(state): State<AppState>, _user: CurrentUser) -> AppResult<Json<Vec<Product>>> {
    let products = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(products))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<Product>> {
    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("product not found"))?;
    Ok(Json(product))
}

async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(new): Json<NewProduct>,
) -> AppResult<(StatusCode, Json<Product>)> {
    new.validate()?;
    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (name, category, attributes, brand, model, device_type) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
    )
    .bind(new.name.trim())
    .bind(&new.category)
    .bind(&new.attributes)
    .bind(&new.brand)
    .bind(&new.model)
    .bind(&new.device_type)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(product)))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
    Json(update): Json<UpdateProduct>,
) -> AppResult<Json<Product>> {
    let mut qb =
        sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE products SET updated_at = now()");
    if let Some(name) = &update.name {
        if name.trim().is_empty() {
            return Err(AppError::validation("name cannot be empty"));
        }
        qb.push(", name = ").push_bind(name.trim().to_string());
    }
    if let Some(category) = &update.category {
        qb.push(", category = ").push_bind(category.clone());
    }
    if let Some(attributes) = &update.attributes {
        qb.push(", attributes = ").push_bind(attributes.clone());
    }
    if let Some(brand) = &update.brand {
        qb.push(", brand = ").push_bind(brand.clone());
    }
    if let Some(model) = &update.model {
        qb.push(", model = ").push_bind(model.clone());
    }
    if let Some(device_type) = &update.device_type {
        qb.push(", device_type = ").push_bind(device_type.clone());
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");
    let product: Option<Product> = qb.build_query_as().fetch_optional(&state.pool).await?;
    let product = product.ok_or_else(|| AppError::not_found("product not found"))?;
    Ok(Json(product))
}

async fn delete_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _user: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let res = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("product not found"));
    }
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub product_ids: Vec<Uuid>,
    #[serde(default)]
    pub question: Option<String>,
}

async fn compare(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(body): Json<CompareRequest>,
) -> AppResult<Json<crate::ai::compare::ComparisonResult>> {
    let result =
        crate::ai::compare::compare_products(&state, &body.product_ids, body.question.as_deref())
            .await?;
    Ok(Json(result))
}

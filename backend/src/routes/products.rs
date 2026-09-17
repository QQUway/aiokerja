use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
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
        "INSERT INTO products (name, category, attributes) VALUES ($1,$2,$3) RETURNING *",
    )
    .bind(new.name.trim())
    .bind(&new.category)
    .bind(&new.attributes)
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

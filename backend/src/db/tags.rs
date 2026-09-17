use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::domain::tag::{NewTag, Tag, UpdateTag};
use crate::error::{AppError, AppResult};

pub async fn list(pool: &PgPool) -> AppResult<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(tags)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<Tag>> {
    let tag = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(tag)
}

pub async fn create(pool: &PgPool, new: &NewTag) -> AppResult<Tag> {
    new.validate()?;
    let color = if new.color.trim().is_empty() {
        "#6b7280".to_string()
    } else {
        new.color.clone()
    };
    let tag = sqlx::query_as::<_, Tag>("INSERT INTO tags (name, color) VALUES ($1,$2) RETURNING *")
        .bind(new.name.trim())
        .bind(color)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.constraint() == Some("tags_name_key") => {
                AppError::conflict("a tag with this name already exists")
            }
            other => AppError::Database(other),
        })?;
    Ok(tag)
}

pub async fn update(pool: &PgPool, id: Uuid, u: &UpdateTag) -> AppResult<Option<Tag>> {
    if let Some(name) = &u.name {
        if name.trim().is_empty() {
            return Err(AppError::validation("name cannot be empty"));
        }
    }
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE tags SET");
    if let Some(name) = &u.name {
        qb.push(" name = ").push_bind(name.trim().to_string());
    }
    if let Some(color) = &u.color {
        qb.push(" color = ").push_bind(color.clone());
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");
    let tag: Option<Tag> = qb.build_query_as().fetch_optional(pool).await?;
    Ok(tag)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

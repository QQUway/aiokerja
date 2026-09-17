use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::domain::project::{NewProject, Project, UpdateProject};
use crate::error::{AppError, AppResult};

pub async fn list(pool: &PgPool) -> AppResult<Vec<Project>> {
    let projects = sqlx::query_as::<_, Project>("SELECT * FROM projects ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(projects)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<Project>> {
    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(project)
}

pub async fn create(pool: &PgPool, new: &NewProject) -> AppResult<Project> {
    new.validate()?;
    let color = if new.color.trim().is_empty() {
        "#4f46e5".to_string()
    } else {
        new.color.clone()
    };
    let project = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (name, description, color) VALUES ($1,$2,$3) RETURNING *",
    )
    .bind(new.name.trim())
    .bind(&new.description)
    .bind(color)
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(d) if d.constraint() == Some("projects_name_key") => {
            AppError::conflict("a project with this name already exists")
        }
        other => AppError::Database(other),
    })?;
    Ok(project)
}

pub async fn update(pool: &PgPool, id: Uuid, u: &UpdateProject) -> AppResult<Option<Project>> {
    if let Some(name) = &u.name {
        if name.trim().is_empty() {
            return Err(AppError::validation("name cannot be empty"));
        }
    }
    let mut qb: QueryBuilder<Postgres> =
        QueryBuilder::new("UPDATE projects SET updated_at = now()");
    if let Some(name) = &u.name {
        qb.push(", name = ").push_bind(name.trim().to_string());
    }
    if let Some(description) = &u.description {
        qb.push(", description = ").push_bind(description.clone());
    }
    if let Some(color) = &u.color {
        qb.push(", color = ").push_bind(color.clone());
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");
    let project: Option<Project> = qb.build_query_as().fetch_optional(pool).await?;
    Ok(project)
}

pub async fn task_count(pool: &PgPool, project_id: Uuid) -> AppResult<i64> {
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Deleting a project leaves tasks orphaned unless cascade is requested.
/// With default `cascade=false` and existing tasks, this is refused (safety,
/// see docs/api.md).
pub async fn delete(pool: &PgPool, id: Uuid, cascade: bool) -> AppResult<bool> {
    if !cascade {
        let count = task_count(pool, id).await?;
        if count > 0 {
            return Err(AppError::conflict(format!(
                "project has {count} tasks; pass ?cascade=true to delete it and set tasks.project_id to NULL"
            )));
        }
    }
    let res = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

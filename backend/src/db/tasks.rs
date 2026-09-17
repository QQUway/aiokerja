use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::domain::tag::Tag;
use crate::domain::task::{NewTask, Task, TaskSummary, UpdateTask};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct TaskFilter {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub project_id: Option<Uuid>,
    pub tag_id: Option<Uuid>,
    pub search: Option<String>,
    pub parent_id: Option<Uuid>,
    pub include_subtasks: bool,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

fn clamp_limit(limit: Option<u32>) -> i64 {
    limit.unwrap_or(100).min(500) as i64
}

pub async fn list(pool: &PgPool, f: &TaskFilter) -> AppResult<(Vec<Task>, u64)> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT t.* FROM tasks t");
    if let Some(tag_id) = f.tag_id {
        qb.push(" JOIN task_tags tt ON tt.task_id = t.id AND tt.tag_id = ");
        qb.push_bind(tag_id);
    }
    qb.push(" WHERE 1=1");
    if let Some(status) = &f.status {
        qb.push(" AND t.status = ").push_bind(status.clone());
    }
    if let Some(priority) = &f.priority {
        qb.push(" AND t.priority = ").push_bind(priority.clone());
    }
    if let Some(project_id) = f.project_id {
        qb.push(" AND t.project_id = ").push_bind(project_id);
    }
    if let Some(parent_id) = f.parent_id {
        qb.push(" AND t.parent_task_id = ").push_bind(parent_id);
    } else if !f.include_subtasks {
        qb.push(" AND t.parent_task_id IS NULL");
    }
    if let Some(search) = &f.search {
        let like = format!("%{}%", search.trim());
        qb.push(" AND (t.title ILIKE ").push_bind(like.clone());
        qb.push(" OR t.description ILIKE ").push_bind(like);
        qb.push(")");
    }

    let page = f.page.unwrap_or(1).max(1);
    let limit = clamp_limit(f.limit);
    let offset = (page as i64 - 1) * limit;

    qb.push(" ORDER BY t.due_date NULLS LAST, t.created_at DESC");
    qb.push(" LIMIT ").push_bind(limit);
    qb.push(" OFFSET ").push_bind(offset);

    let items: Vec<Task> = qb.build_query_as().fetch_all(pool).await?;
    let total = count(pool, f).await?;
    Ok((items, total as u64))
}

async fn count(pool: &PgPool, f: &TaskFilter) -> AppResult<i64> {
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM tasks t");
    if let Some(tag_id) = f.tag_id {
        qb.push(" JOIN task_tags tt ON tt.task_id = t.id AND tt.tag_id = ");
        qb.push_bind(tag_id);
    }
    qb.push(" WHERE 1=1");
    if let Some(status) = &f.status {
        qb.push(" AND t.status = ").push_bind(status.clone());
    }
    if let Some(priority) = &f.priority {
        qb.push(" AND t.priority = ").push_bind(priority.clone());
    }
    if let Some(project_id) = f.project_id {
        qb.push(" AND t.project_id = ").push_bind(project_id);
    }
    if let Some(parent_id) = f.parent_id {
        qb.push(" AND t.parent_task_id = ").push_bind(parent_id);
    } else if !f.include_subtasks {
        qb.push(" AND t.parent_task_id IS NULL");
    }
    if let Some(search) = &f.search {
        let like = format!("%{}%", search.trim());
        qb.push(" AND (t.title ILIKE ").push_bind(like.clone());
        qb.push(" OR t.description ILIKE ").push_bind(like);
        qb.push(")");
    }
    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<Task>> {
    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(task)
}

pub async fn create(pool: &PgPool, new: &NewTask) -> AppResult<Task> {
    new.validate()?;
    let mut tx = pool.begin().await?;
    let task = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (title, description, status, priority, due_date, scheduled_start, \
         scheduled_end, recurrence_rule, project_id, parent_task_id, completed_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10, CASE WHEN $4 = 'done' THEN now() END) \
         RETURNING *",
    )
    .bind(new.title.trim())
    .bind(&new.description)
    .bind(&new.status)
    .bind(&new.priority)
    .bind(new.due_date)
    .bind(new.scheduled_start)
    .bind(new.scheduled_end)
    .bind(&new.recurrence_rule)
    .bind(new.project_id)
    .bind(new.parent_task_id)
    .fetch_one(&mut *tx)
    .await?;

    if !new.tag_ids.is_empty() {
        set_tags_tx(&mut tx, task.id, &new.tag_ids).await?;
    }
    tx.commit().await?;
    Ok(task)
}

pub async fn update(pool: &PgPool, id: Uuid, u: &UpdateTask) -> AppResult<Option<Task>> {
    if let Some(status) = &u.status {
        if !crate::domain::task::STATUSES.contains(&status.as_str()) {
            return Err(AppError::validation("invalid status"));
        }
    }
    if let Some(priority) = &u.priority {
        if !crate::domain::task::PRIORITIES.contains(&priority.as_str()) {
            return Err(AppError::validation("invalid priority"));
        }
    }
    if let Some(title) = &u.title {
        if title.trim().is_empty() {
            return Err(AppError::validation("title cannot be empty"));
        }
    }

    let mut tx = pool.begin().await?;
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE tasks SET updated_at = now()");
    if let Some(title) = &u.title {
        qb.push(", title = ").push_bind(title.trim().to_string());
    }
    if let Some(description) = &u.description {
        qb.push(", description = ").push_bind(description.clone());
    }
    if let Some(status) = &u.status {
        qb.push(", status = ").push_bind(status.clone());
        if status == "done" {
            qb.push(", completed_at = COALESCE(completed_at, now())");
        } else {
            qb.push(", completed_at = NULL");
        }
    }
    if let Some(priority) = &u.priority {
        qb.push(", priority = ").push_bind(priority.clone());
    }
    if let Some(due) = &u.due_date {
        qb.push(", due_date = ").push_bind(*due);
    }
    if let Some(s) = &u.scheduled_start {
        qb.push(", scheduled_start = ").push_bind(*s);
    }
    if let Some(e) = &u.scheduled_end {
        qb.push(", scheduled_end = ").push_bind(*e);
    }
    if let Some(r) = &u.recurrence_rule {
        qb.push(", recurrence_rule = ").push_bind(r.clone());
    }
    if let Some(p) = &u.project_id {
        qb.push(", project_id = ").push_bind(*p);
    }
    if let Some(p) = &u.parent_task_id {
        qb.push(", parent_task_id = ").push_bind(*p);
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");

    let task: Option<Task> = qb.build_query_as().fetch_optional(&mut *tx).await?;

    if task.is_some() {
        if let Some(tag_ids) = &u.tag_ids {
            set_tags_tx(&mut tx, id, tag_ids).await?;
        }
    }
    tx.commit().await?;
    Ok(task)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn tags_for(pool: &PgPool, task_id: Uuid) -> AppResult<Vec<Tag>> {
    let tags = sqlx::query_as::<_, Tag>(
        "SELECT g.* FROM tags g JOIN task_tags tt ON tt.tag_id = g.id \
         WHERE tt.task_id = $1 ORDER BY g.name",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

pub async fn tags_for_many(
    pool: &PgPool,
    ids: &[Uuid],
) -> AppResult<std::collections::HashMap<Uuid, Vec<Tag>>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    // (Uuid, Tag) tuples need Tag to impl Decode; fetch scalars and map instead.
    let rows: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT tt.task_id, g.id, g.name, g.color FROM tags g \
         JOIN task_tags tt ON tt.tag_id = g.id \
         WHERE tt.task_id = ANY($1) ORDER BY g.name",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    let mut map: std::collections::HashMap<Uuid, Vec<Tag>> = Default::default();
    for (task_id, id, name, color) in rows {
        map.entry(task_id).or_default().push(Tag {
            id,
            name,
            color,
            created_at: chrono::Utc::now(),
        });
    }
    Ok(map)
}

pub async fn subtasks(pool: &PgPool, parent_id: Uuid) -> AppResult<Vec<Task>> {
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE parent_task_id = $1 ORDER BY created_at",
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await?;
    Ok(tasks)
}

async fn set_tags_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    task_id: Uuid,
    tag_ids: &[Uuid],
) -> AppResult<()> {
    sqlx::query("DELETE FROM task_tags WHERE task_id = $1")
        .bind(task_id)
        .execute(&mut **tx)
        .await?;
    for tag_id in tag_ids {
        sqlx::query(
            "INSERT INTO task_tags (task_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(task_id)
        .bind(tag_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn attach_tag(pool: &PgPool, task_id: Uuid, tag_id: Uuid) -> AppResult<()> {
    sqlx::query("INSERT INTO task_tags (task_id, tag_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(task_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn detach_tag(pool: &PgPool, task_id: Uuid, tag_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM task_tags WHERE task_id = $1 AND tag_id = $2")
        .bind(task_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Tasks with a due date or scheduled window overlapping [from, to] — used by
/// the merged calendar read (docs/decisions.md D3).
pub async fn tasks_in_range(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<Task>> {
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks \
         WHERE (scheduled_start IS NOT NULL AND scheduled_start < $2 AND COALESCE(scheduled_end, scheduled_start + interval '1 hour') > $1) \
            OR (due_date IS NOT NULL AND due_date >= $1 AND due_date <= $2) \
         ORDER BY COALESCE(scheduled_start, due_date)",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(tasks)
}

pub async fn search(pool: &PgPool, query: &str, limit: i64) -> AppResult<Vec<Task>> {
    let like = format!("%{}%", query.trim());
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE title ILIKE $1 OR description ILIKE $1 \
         ORDER BY due_date NULLS LAST LIMIT $2",
    )
    .bind(like)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(tasks)
}

pub async fn summary(pool: &PgPool) -> AppResult<TaskSummary> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
        .fetch_one(pool)
        .await?;
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status, COUNT(*) FROM tasks GROUP BY status")
            .fetch_all(pool)
            .await?;
    let by_status: std::collections::HashMap<String, u64> =
        rows.into_iter().map(|(s, c)| (s, c as u64)).collect();

    let now = Utc::now();
    let end_of_today = (now + Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|d| DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))
        .unwrap_or(now);
    let start_of_today = end_of_today - Duration::days(1);

    let today_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE status NOT IN ('done','cancelled') \
         AND ((due_date >= $1 AND due_date < $2) OR (scheduled_start >= $1 AND scheduled_start < $2))",
    )
    .bind(start_of_today)
    .bind(end_of_today)
    .fetch_one(pool)
    .await?;

    let upcoming_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE status NOT IN ('done','cancelled') \
         AND COALESCE(scheduled_start, due_date) >= $1",
    )
    .bind(end_of_today)
    .fetch_one(pool)
    .await?;

    let overdue_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE status NOT IN ('done','cancelled') \
         AND due_date IS NOT NULL AND due_date < $1",
    )
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(TaskSummary {
        total: total as u64,
        by_status,
        today_count: today_count as u64,
        upcoming_count: upcoming_count as u64,
        overdue_count: overdue_count as u64,
    })
}

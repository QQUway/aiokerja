use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::domain::event::{CalendarEvent, CalendarItem, NewEvent, UpdateEvent};
use crate::error::{AppError, AppResult};

pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<CalendarEvent>> {
    let event = sqlx::query_as::<_, CalendarEvent>("SELECT * FROM calendar_events WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(event)
}

pub async fn create(pool: &PgPool, new: &NewEvent) -> AppResult<CalendarEvent> {
    new.validate()?;
    let start = new.start_time.unwrap_or_else(Utc::now);
    let end = new.end_time.unwrap_or(start + chrono::Duration::hours(1));
    let event = sqlx::query_as::<_, CalendarEvent>(
        "INSERT INTO calendar_events (title, description, start_time, end_time, all_day, \
         recurrence_rule, reminder_minutes, source_task_id) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *",
    )
    .bind(new.title.trim())
    .bind(&new.description)
    .bind(start)
    .bind(end)
    .bind(new.all_day)
    .bind(&new.recurrence_rule)
    .bind(new.reminder_minutes)
    .bind(new.source_task_id)
    .fetch_one(pool)
    .await?;
    Ok(event)
}

pub async fn update(pool: &PgPool, id: Uuid, u: &UpdateEvent) -> AppResult<Option<CalendarEvent>> {
    if let Some(title) = &u.title {
        if title.trim().is_empty() {
            return Err(AppError::validation("title cannot be empty"));
        }
    }
    let mut qb: QueryBuilder<Postgres> =
        QueryBuilder::new("UPDATE calendar_events SET updated_at = now()");
    if let Some(title) = &u.title {
        qb.push(", title = ").push_bind(title.trim().to_string());
    }
    if let Some(description) = &u.description {
        qb.push(", description = ").push_bind(description.clone());
    }
    if let Some(start) = u.start_time {
        qb.push(", start_time = ").push_bind(start);
    }
    if let Some(end) = u.end_time {
        qb.push(", end_time = ").push_bind(end);
    }
    if let Some(all_day) = u.all_day {
        qb.push(", all_day = ").push_bind(all_day);
    }
    if let Some(r) = &u.recurrence_rule {
        qb.push(", recurrence_rule = ").push_bind(r.clone());
    }
    if let Some(m) = &u.reminder_minutes {
        qb.push(", reminder_minutes = ").push_bind(*m);
    }
    if let Some(src) = &u.source_task_id {
        qb.push(", source_task_id = ").push_bind(*src);
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");

    let event: Option<CalendarEvent> = qb.build_query_as().fetch_optional(pool).await?;
    Ok(event)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM calendar_events WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn events_in_range(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<CalendarEvent>> {
    let events = sqlx::query_as::<_, CalendarEvent>(
        "SELECT * FROM calendar_events WHERE start_time < $2 AND end_time > $1 \
         ORDER BY start_time",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(events)
}

/// Merged read: real events + task projections (docs/decisions.md D3).
pub async fn calendar_items(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<CalendarItem>> {
    let mut items: Vec<CalendarItem> = Vec::new();

    let events = events_in_range(pool, from, to).await?;
    items.extend(events.into_iter().map(|e| CalendarItem {
        id: e.id,
        kind: "event".to_string(),
        title: e.title,
        description: e.description,
        start_time: e.start_time,
        end_time: e.end_time,
        all_day: e.all_day,
        recurrence_rule: e.recurrence_rule,
        reminder_minutes: e.reminder_minutes,
        source_task_id: e.source_task_id,
        task_id: e.source_task_id,
        status: None,
        priority: None,
        project_id: None,
    }));

    let tasks = crate::db::tasks::tasks_in_range(pool, from, to).await?;
    items.extend(tasks.into_iter().map(|t| {
        CalendarItem {
            id: t.id,
            kind: "task".to_string(),
            title: t.title,
            description: t.description,
            start_time: t.scheduled_start.unwrap_or(t.due_date.unwrap_or(from)),
            end_time: t
                .scheduled_end
                .unwrap_or(t.scheduled_start.unwrap_or(t.due_date.unwrap_or(from))),
            all_day: false,
            recurrence_rule: t.recurrence_rule,
            reminder_minutes: None,
            source_task_id: None,
            task_id: Some(t.id),
            status: Some(t.status),
            priority: Some(t.priority),
            project_id: t.project_id,
        }
    }));

    items.sort_by_key(|a| a.start_time);
    Ok(items)
}

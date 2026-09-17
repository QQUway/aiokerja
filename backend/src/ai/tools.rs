use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ai::embeddings::EmbeddingProvider;
use crate::ai::llm::{ChatMessage, ChatRequest, ToolDef};
use crate::ai::rag::{self, Reranker};
use crate::domain::event::NewEvent;
use crate::domain::task::{NewTask, UpdateTask};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// JSON-schema definitions for the [OI]-style function-calling API (§6).
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "create_task".into(),
            description: "Create a task with an optional due date, priority and project.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "Task title"},
                    "description": {"type": "string"},
                    "priority": {"type": "string", "enum": ["low","medium","high","urgent"]},
                    "due_date": {"type": "string", "description": "RFC3339 timestamp"},
                    "project_id": {"type": "string", "description": "UUID of an existing project"}
                },
                "required": ["title"]
            }),
        },
        ToolDef {
            name: "update_task".into(),
            description: "Update fields of an existing task by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {"type": "string", "description": "UUID of the task"},
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "status": {"type": "string", "enum": ["todo","in_progress","done","cancelled"]},
                    "priority": {"type": "string", "enum": ["low","medium","high","urgent"]},
                    "due_date": {"type": "string"},
                    "project_id": {"type": "string"}
                },
                "required": ["task_id"]
            }),
        },
        ToolDef {
            name: "delete_task".into(),
            description: "Delete a task by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"task_id": {"type": "string"}},
                "required": ["task_id"]
            }),
        },
        ToolDef {
            name: "list_tasks".into(),
            description: "List tasks, optionally filtered by status or project.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string", "enum": ["todo","in_progress","done","cancelled"]},
                    "project_id": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                }
            }),
        },
        ToolDef {
            name: "search_tasks".into(),
            description: "Full-text search over task titles and descriptions.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
        },
        ToolDef {
            name: "create_event".into(),
            description: "Create a calendar event.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "start_time": {"type": "string", "description": "RFC3339 timestamp"},
                    "end_time": {"type": "string", "description": "RFC3339 timestamp"},
                    "reminder_minutes": {"type": "integer"}
                },
                "required": ["title", "start_time", "end_time"]
            }),
        },
        ToolDef {
            name: "update_event".into(),
            description: "Update fields of an existing calendar event by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "event_id": {"type": "string"},
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "start_time": {"type": "string"},
                    "end_time": {"type": "string"},
                    "reminder_minutes": {"type": "integer"}
                },
                "required": ["event_id"]
            }),
        },
        ToolDef {
            name: "list_schedule".into(),
            description: "List the user's schedule (calendar events and scheduled/due tasks) \
                 within a time range. Use this to answer questions about what's coming up."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "RFC3339 start of range; defaults to now"},
                    "to": {"type": "string", "description": "RFC3339 end of range; defaults to 7 days from `from`"}
                }
            }),
        },
        ToolDef {
            name: "delete_event".into(),
            description: "Delete a calendar event by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"event_id": {"type": "string"}},
                "required": ["event_id"]
            }),
        },
        ToolDef {
            name: "search_documents".into(),
            description: "Search uploaded documents by title or filename.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 50}
                },
                "required": ["query"]
            }),
        },
        ToolDef {
            name: "retrieve_knowledge".into(),
            description:
                "Semantic search over the knowledge base. Returns the most relevant document \
                 excerpts with citations. Use this before answering questions about documents."
                    .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "top_k": {"type": "integer", "minimum": 1, "maximum": 20}
                },
                "required": ["query"]
            }),
        },
        ToolDef {
            name: "summarize_document".into(),
            description: "Summarize an uploaded document by id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {"document_id": {"type": "string"}},
                "required": ["document_id"]
            }),
        },
    ]
}

fn parse_uuid(value: &Value, field: &str) -> AppResult<Uuid> {
    let s = value
        .as_str()
        .ok_or_else(|| AppError::validation(format!("{field} must be a UUID string")))?;
    Uuid::parse_str(s).map_err(|_| AppError::validation(format!("{field} is not a valid UUID")))
}

fn opt_uuid(value: &Value, field: &str) -> AppResult<Option<Uuid>> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => Ok(Some(parse_uuid(v, field)?)),
    }
}

fn opt_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(|v| v.as_str()).map(String::from)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct UpdateTaskArgs {
    task_id: Option<Uuid>,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    due_date: Option<chrono::DateTime<chrono::Utc>>,
    project_id: Option<Uuid>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct UpdateEventArgs {
    event_id: Option<Uuid>,
    title: Option<String>,
    description: Option<String>,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    end_time: Option<chrono::DateTime<chrono::Utc>>,
    reminder_minutes: Option<i32>,
}

/// Execute one tool call. Every arm validates its arguments and returns an
/// explicit error rather than panicking (§6).
pub async fn execute(state: &AppState, name: &str, args: &Value) -> AppResult<Value> {
    match name {
        "create_task" => {
            let title = opt_string(args, "title")
                .ok_or_else(|| AppError::validation("create_task: title is required"))?;
            let due_date = match args.get("due_date").and_then(|v| v.as_str()) {
                Some(due) => Some(parse_datetime(due, "due_date")?),
                None => None,
            };
            let new = NewTask {
                title,
                description: opt_string(args, "description"),
                priority: opt_string(args, "priority").unwrap_or_else(|| "medium".to_string()),
                due_date,
                project_id: opt_uuid(args, "project_id")?,
                ..Default::default()
            };
            let task = crate::db::tasks::create(&state.pool, &new).await?;
            Ok(serde_json::to_value(task).unwrap_or(Value::Null))
        }
        "update_task" => {
            let a: UpdateTaskArgs = serde_json::from_value(args.clone())
                .map_err(|e| AppError::validation(format!("update_task: {e}")))?;
            let task_id = a
                .task_id
                .ok_or_else(|| AppError::validation("update_task: task_id is required"))?;
            let update = UpdateTask {
                title: a.title,
                description: a.description.map(Some),
                status: a.status,
                priority: a.priority,
                due_date: a.due_date.map(Some),
                project_id: a.project_id.map(Some),
                ..Default::default()
            };
            let task = crate::db::tasks::update(&state.pool, task_id, &update)
                .await?
                .ok_or_else(|| AppError::not_found("task not found"))?;
            Ok(serde_json::to_value(task).unwrap_or(Value::Null))
        }
        "delete_task" => {
            let task_id = parse_uuid(
                args.get("task_id")
                    .ok_or_else(|| AppError::validation("delete_task: task_id is required"))?,
                "task_id",
            )?;
            let deleted = crate::db::tasks::delete(&state.pool, task_id).await?;
            Ok(json!({ "deleted": deleted, "task_id": task_id }))
        }
        "list_tasks" => {
            let mut filter = crate::db::tasks::TaskFilter {
                status: opt_string(args, "status"),
                project_id: opt_uuid(args, "project_id")?,
                include_subtasks: true,
                ..Default::default()
            };
            if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
                filter.limit = Some(limit as u32);
            }
            let (tasks, total) = crate::db::tasks::list(&state.pool, &filter).await?;
            Ok(json!({ "tasks": tasks, "total": total }))
        }
        "search_tasks" => {
            let query = opt_string(args, "query")
                .ok_or_else(|| AppError::validation("search_tasks: query is required"))?;
            let tasks = crate::db::tasks::search(&state.pool, &query, 20).await?;
            Ok(json!({ "tasks": tasks }))
        }
        "create_event" => {
            let start = args
                .get("start_time")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::validation("create_event: start_time is required"))?;
            let end = args
                .get("end_time")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::validation("create_event: end_time is required"))?;
            let new = NewEvent {
                title: opt_string(args, "title")
                    .ok_or_else(|| AppError::validation("create_event: title is required"))?,
                description: opt_string(args, "description"),
                start_time: Some(parse_datetime(start, "start_time")?),
                end_time: Some(parse_datetime(end, "end_time")?),
                reminder_minutes: args
                    .get("reminder_minutes")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                ..Default::default()
            };
            let event = crate::db::events::create(&state.pool, &new).await?;
            Ok(serde_json::to_value(event).unwrap_or(Value::Null))
        }
        "update_event" => {
            let a: UpdateEventArgs = serde_json::from_value(args.clone())
                .map_err(|e| AppError::validation(format!("update_event: {e}")))?;
            let event_id = a
                .event_id
                .ok_or_else(|| AppError::validation("update_event: event_id is required"))?;
            let update = crate::domain::event::UpdateEvent {
                title: a.title,
                description: a.description.map(Some),
                start_time: a.start_time,
                end_time: a.end_time,
                reminder_minutes: a.reminder_minutes.map(Some),
                ..Default::default()
            };
            let event = crate::db::events::update(&state.pool, event_id, &update)
                .await?
                .ok_or_else(|| AppError::not_found("event not found"))?;
            Ok(serde_json::to_value(event).unwrap_or(Value::Null))
        }
        "list_schedule" => {
            let from = match args.get("from").and_then(|v| v.as_str()) {
                Some(s) => parse_datetime(s, "from")?,
                None => chrono::Utc::now(),
            };
            let to = match args.get("to").and_then(|v| v.as_str()) {
                Some(s) => parse_datetime(s, "to")?,
                None => from + chrono::Duration::days(7),
            };
            let items = crate::db::events::calendar_items(&state.pool, from, to).await?;
            Ok(json!({ "items": items, "from": from, "to": to }))
        }
        "delete_event" => {
            let event_id = parse_uuid(
                args.get("event_id")
                    .ok_or_else(|| AppError::validation("delete_event: event_id is required"))?,
                "event_id",
            )?;
            let deleted = crate::db::events::delete(&state.pool, event_id).await?;
            Ok(json!({ "deleted": deleted, "event_id": event_id }))
        }
        "search_documents" => {
            let query = opt_string(args, "query")
                .ok_or_else(|| AppError::validation("search_documents: query is required"))?;
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            let docs = crate::db::documents::search(&state.pool, &query, limit).await?;
            Ok(json!({ "documents": docs }))
        }
        "retrieve_knowledge" => {
            let query = opt_string(args, "query")
                .ok_or_else(|| AppError::validation("retrieve_knowledge: query is required"))?;
            let top_k = args
                .get("top_k")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(state.config.rag_retrieval_top_k);
            let reranker: &dyn Reranker = state.reranker.as_ref();
            let embeddings: &dyn EmbeddingProvider = state.embeddings.as_ref();
            let chunks =
                rag::retrieve_knowledge(&state.pool, embeddings, reranker, &query, top_k).await?;
            let citations = rag::citations_from(&chunks);
            Ok(json!({ "results": chunks, "citations": citations }))
        }
        "summarize_document" => {
            let document_id = parse_uuid(
                args.get("document_id").ok_or_else(|| {
                    AppError::validation("summarize_document: document_id is required")
                })?,
                "document_id",
            )?;
            let doc = crate::db::documents::get(&state.pool, document_id)
                .await?
                .ok_or_else(|| AppError::not_found("document not found"))?;
            let text = crate::db::documents::raw_text(&state.pool, document_id)
                .await?
                .ok_or_else(|| AppError::not_found("document has no extracted text yet"))?;
            let excerpt = crate::ai::llm::truncate(&text, 12_000);
            let messages = vec![
                ChatMessage::system(
                    "You summarize documents faithfully. Return a concise summary with key points.",
                ),
                ChatMessage::user(format!(
                    "Summarize this document titled '{}':\n\n{excerpt}",
                    doc.title
                )),
            ];
            let response = state.llm.chat(&ChatRequest::new(messages)).await?;
            Ok(json!({
                "document_id": document_id,
                "title": doc.title,
                "summary": response.content.unwrap_or_else(|| "(no summary produced)".into())
            }))
        }
        other => Err(AppError::validation(format!("unknown tool: {other}"))),
    }
}

fn parse_datetime(value: &str, field: &str) -> AppResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| AppError::validation(format!("{field} must be an RFC3339 timestamp")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_required_tools_are_defined() {
        let names: Vec<String> = tool_definitions().into_iter().map(|t| t.name).collect();
        for expected in [
            "create_task",
            "update_task",
            "delete_task",
            "list_tasks",
            "search_tasks",
            "create_event",
            "update_event",
            "list_schedule",
            "delete_event",
            "search_documents",
            "retrieve_knowledge",
            "summarize_document",
        ] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn every_definition_has_object_schema() {
        for tool in tool_definitions() {
            assert_eq!(tool.parameters["type"], "object", "{}", tool.name);
            assert!(
                tool.parameters.get("properties").is_some(),
                "{} missing properties",
                tool.name
            );
        }
    }

    #[test]
    fn parse_uuid_rejects_garbage() {
        assert!(parse_uuid(&json!("not-a-uuid"), "task_id").is_err());
        assert!(parse_uuid(&json!(42), "task_id").is_err());
    }

    #[test]
    fn parse_datetime_requires_rfc3339() {
        assert!(parse_datetime("2026-01-01T10:00:00Z", "start_time").is_ok());
        assert!(parse_datetime("tomorrow", "start_time").is_err());
    }
}

# API

Base path `/api`. Errors: `{ "error": { "code": "...", "message": "..." } }` with
appropriate HTTP status. Timestamps are RFC3339. IDs are UUIDs. No auth in v1
(see README deployment notes).

## Tasks

| Method | Path | Notes |
|---|---|---|
| GET | `/api/tasks` | query: `status`, `priority`, `project_id`, `tag_id`, `search`, `parent_id`, `include_subtasks`(bool), `page`, `limit`. Response `{items[], total}` |
| GET | `/api/tasks/:id` | includes `tags[]`, `subtasks[]` |
| POST | `/api/tasks` | **body**: `title`(req), `description`, `status`, `priority`, `due_date`, `scheduled_start`, `scheduled_end`, `recurrence_rule`, `project_id`, `parent_task_id`, `tag_ids[]` |
| PATCH | `/api/tasks/:id` | partial update of any write field; `completed_at` set when `status=done` |
| DELETE | `/api/tasks/:id` | cascades subtasks, task_tags |
| POST | `/api/tasks/:id/tags` | body `{tag_id}` — attach tag |
| DELETE | `/api/tasks/:id/tags/:tag_id` | detach tag |
| POST | `/api/tasks/:id/calendar-event` | materialize task as editable `calendar_event` (sets `source_task_id`) |
| GET | `/api/tasks/views/summary` | counts by status + for today/upcoming/overdue **must precede `/:id` matching** |

Task shape: `{id, title, description, status, priority, due_date, scheduled_start,
scheduled_end, recurrence_rule, project_id, project, parent_task_id, tags[],
completed_at, created_at, updated_at}`.

## Projects

`GET/POST /api/projects`, `PATCH/DELETE /api/projects/:id`.
Project: `{id, name, description, color, created_at, updated_at}`.
DELETE requires `?cascade=false` unless `task_count=0`.

## Tags

`GET/POST /api/tags`, `PATCH/DELETE /api/tags/:id`. Tag: `{id, name, color}`.

## Calendar

| Method | Path | Notes |
|---|---|---|
| GET | `/api/calendar` | query: `from`, `to` (RFC3339). Merged items, each `{id, kind: event|task, title, start_time, end_time, all_day, source_task_id?, reminder_minutes, ...}`. Task projection `kind:task` is read-only. |
| POST | `/api/events` | body: `title`(req), `description`, `start_time`(req), `end_time`(req), `all_day`, `recurrence_rule`, `reminder_minutes`, `source_task_id` |
| GET | `/api/events/:id` | |
| PATCH | `/api/events/:id` | partial update |
| DELETE | `/api/events/:id` | |

## Documents

| Method | Path | Notes |
|---|---|---|
| GET | `/api/documents` | query: `status`, `doc_type`, `search` |
| POST | `/api/documents` | `multipart/form-data`, field `file`; extensions: pdf, txt, md, docx. Returns doc with `status: pending` |
| GET | `/api/documents/:id` | |
| GET | `/api/documents/:id/raw` | extracted text (from `document_extra`) |
| GET | `/api/documents/:id/chunks` | chunk list with `chunk_index`, `chunk_text`, `token_count` (no embeddings) |
| POST | `/api/documents/:id/reindex` | clear chunks + re-run pipeline |
| DELETE | `/api/documents/:id` | cascades chunks |

Doc: `{id, filename, title, source, doc_type, upload_date, status, error, created_at, updated_at}`.

## Chat

| Method | Path | Notes |
|---|---|---|
| GET | `/api/chat/conversations` | list |
| POST | `/api/chat/conversations` | body `{title?}` |
| GET | `/api/chat/conversations/:id` | messages + metadata |
| PATCH | `/api/chat/conversations/:id` | body `{title}` |
| DELETE | `/api/chat/conversations/:id` | |
| POST | `/api/chat/conversations/:id/messages` | body `{content}`. Orchestrates tool-call loop. Resp: `{message: {id, role, content, citations[]}, tool_calls: [{name, args, result}]}` |

Message: `{id, conversation_id, role, content, tool_calls, tool_results,
citations, created_at}`. Citation: `{document_id, title, chunk_text, score}`.

## Search

`GET /api/search?q=` — returns `{tasks[], documents[]}` via Postgres full-text +
title/ILIKE match. Semantic RAG lives in the chat `retrieve_knowledge` tool.

## Dashboard

`GET /api/dashboard` — `{tasks: {total, by_status, today_count, upcoming_count,
overdue_count}, projects_count, documents_count, indexed_documents, upcoming_events[],
recent_tasks[]}`.

## Products (schema stubbed, v2 UI)

`GET/POST /api/products`, `PATCH/DELETE /api/products/:id`.
Product: `{id, name, category, attributes, created_at, updated_at}`.

---

## AI tool functions (OpenAI-style function calling, §6)

| name | params (JSON schema) | behavior |
|---|---|---|
| `create_task` | `{title: string req, description?: string, priority?: enum, due_date?: string, project_id?: uuid}` | insert, return task |
| `update_task` | `{task_id: uuid req, ...any task field}` | update, return updated task |
| `delete_task` | `{task_id: uuid req}` | delete, return `{deleted: task_id}` |
| `list_tasks` | `{status?: string, project_id?: uuid, limit?: int}` | return task summaries |
| `search_tasks` | `{query: string req}` | ILIKE on title/description |
| `create_event` | `{title: string req, start_time: string req, end_time: string req, description?, reminder_minutes?}` | insert event, return |
| `update_event` | `{event_id: uuid req, ...field}` | update event |
| `search_documents` | `{query: string req, limit?: int}` | metadata/full-text doc search |
| `retrieve_knowledge` | `{query: string req, top_k?: int}` | pgvector retrieval; returns chunks + citations |
| `summarize_document` | `{document_id: uuid req}` | chunks → LLM summary, include in response |

Registry in `ai/tools.rs` exposes each as `serde_json::Value` JSON-schema def and
an async `execute(name, args) -> Result<Value>`. All execute paths validate args
and return explicit `Err(AppError)` (never panic). Every `retrieve_knowledge`
result carries `citations` that the orchestrator forwards to the client and
persists on the assistant message.
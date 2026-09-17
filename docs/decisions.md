# Decisions

Recorded per §1 ("write down what you chose and why and keep moving").

## D1 — Query layer: `sqlx` runtime queries (not sea-orm, not sqlx macros)
`sqlx` with **runtime** `query_as`/`bind` (no `query!` macros) so the backend
image builds without a live `DATABASE_URL`. `FromRow` derive maps rows to domain
structs. Sea-orm rejected: extra abstraction + slower compile, no benefit at this
size. Compile-time-checked macros rejected: they require a database at build
time, which complicates the Docker build and CI. Trade-off: SQL typos are caught
at runtime/test time, mitigated by integration tests against a real Postgres.

## D2 — pgvector embedding column is created at startup, not in the migration
The dimension is config-driven (§1). SQL migrations are static and checked in, so
the vector column + HNSW index are created by an idempotent DDL step
(`ensure_embedding_schema`) using `EMBEDDING_DIMENSIONS`. `embedding_index_meta`
stores the dimension the index was built with; startup errors clearly if the
configured dimension differs from the stored one (prevents silent corruption).

## D3 — Calendar/task integration: approach (a) read-time merge, plus (b) projection
Chose **both, narrowly**: the calendar read endpoint (`GET /api/calendar?from&to`)
returns `calendar_events` merged with a *projection* of tasks that have
`scheduled_start`/`due_date` in range. Tasks are returned as read-only calendar
items with `source: "task"` and `task_id`; they are never written to
`calendar_events`, so there is no drift. `calendar_events.source_task_id` exists
for the explicit "materialize this task on the calendar" action
(`POST /api/tasks/:id/calendar-event`) where the user wants an independent event.
Updating a task's dates is therefore reflected immediately with no sync step.
Rationale: read-time merge alone (§4a) is simplest and drift-free; the projection
table covers the case where a user wants a *separate* event record.

## D4 — Token counting: heuristic tokenizer behind a trait
`ai/chunk.rs` uses a `Tokenizer` trait. Default impl estimates tokens as
`ceil(chars / 4)` with word-boundary snapping, avoiding a runtime download of
tiktoken BPE files (self-hosted/offline friendly). Chunk sizes are therefore
approximate; documented. A real tokenizer can be swapped in without touching the
pipeline.

## D5 — Document ingestion is in-process (`tokio::spawn`), not a queue
§9 forbids Redis/queue infra. Upload returns immediately with status `pending`;
a spawned task extracts → chunks → embeds → stores, updating status. On process
restart, documents stuck in `indexing` are reset to `pending` and re-ingested at
startup (best-effort, no external scheduler).

## D6 — Auth: no-op `CurrentUser` extractor
`auth.rs` implements `FromRequestParts` returning the fixed single user id. Every
handler takes `CurrentUser` even though it is unused for filtering, so adding
real auth later is a change in one file plus adding `user_id` predicates.

## D7 — Reranking: `Reranker` trait with `NoopReranker`
Per §5 the interface exists so a cross-encoder reranker can be dropped in as v2.
Noop preserves vector-similarity order.

## D8 — Frontend state: TanStack Query for server state, no global store
Server data is cached/refetched by TanStack Query; UI-only state (filters, modal
open) is local component state. No Redux/Zustand — unnecessary for this surface.

## D9 — Styling: hand-written CSS with design tokens
Avoids a Tailwind build/config dependency and keeps the Vite setup minimal. One
`styles.css` with CSS custom properties.

## D10 — Tool-calling loop cap
Chat tool loop is capped at 8 iterations to bound runaway tool use; on cap, a
final assistant message is produced with whatever content is available.

## D11 — `products` entity is schema + minimal CRUD only
§3 requires the table; §7 defers the comparison UI. Backend CRUD + a simple list
page exist so the entity is real, but no comparison/matching UI in v1.

## D12 — Full-text search: Postgres `tsvector` generated column
Global search (§7 Stage 5) uses a `tsvector` GIN index on tasks/documents plus
pgvector similarity for the semantic half. No Elasticsearch.

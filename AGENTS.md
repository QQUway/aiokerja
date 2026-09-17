# AGENTS.md — Personal Work Assistant

This file is the operating spec for OpenCode (or any coding agent) building this
project. It captures the product requirement, the decisions already made, and the
process to follow. **Read this entire file before writing any code.**

---

## 0. Mission

Build a self-hostable, all-in-one personal work assistant: task manager + calendar +
RAG-based knowledge base + AI chat assistant with tool-calling, deployable via
`docker compose up -d` on a single VPS.

This is a **real application**, not a mockup: real backend, real database, real
persistence, a usable frontend. No stubbed endpoints that return fake data.

---

## 1. Decisions Already Made (do not re-litigate these)

These were decided with the project owner up front. Follow them exactly. Where this
file says "configurable," it means: has an env var with the stated default, not
"hardcode it and call it a TODO."

| Area | Decision |
|---|---|
| **Backend language** | Rust, Axum web framework |
| **Database** | PostgreSQL, with `pgvector` extension for embeddings (no separate vector DB) |
| **Frontend** | TypeScript + React + Vite |
| **LLM providers (v1)** | OpenAI + any OpenAI-compatible API (Ollama, LM Studio, vLLM, etc.) via a single trait/interface. No Anthropic/Gemini-native code paths in v1 — design the trait so they could be added later without a rewrite. |
| **Embeddings default** | OpenAI `text-embedding-3-small`, 1536 dimensions. The pgvector column dimension is driven by config, defaulting to 1536. Document clearly that changing embedding model/dimension after documents are indexed requires re-indexing (dimension mismatch otherwise). |
| **Tool/function calling** | Native OpenAI-style `tools` param (function calling). This means the assistant's tool-calling features require an OpenAI-compatible endpoint that actually supports function calling (OpenAI itself, or compatible servers like vLLM/Ollama with tool support). Do not build a prompt-based JSON fallback in v1 — document this as a known limitation instead. |
| **Auth (v1)** | **None.** No login, no credentials, no session system. The app assumes it sits behind a trusted network or a reverse proxy that handles access control (e.g. Tailscale, a proxy with basic auth, VPN-only exposure). Document this loudly in the README as a deployment requirement, not an oversight. Structure the backend (e.g. a `current_user`-shaped extractor, even if it's a no-op returning a single fixed user id) so real multi-user auth can be added later without restructuring every handler. |
| **RAG chunking** | Fixed-size chunking by token count with overlap (not structure-aware in v1). Configurable chunk size and overlap via env vars. |
| **Docker Compose services** | Exactly 3: `postgres` (pgvector image), `backend`, `frontend`. No nginx/caddy/reverse-proxy container, no Redis, no message queue. |
| **Process** | Fully autonomous execution through all MVP stages (see §7). Report progress and final results at the end of each stage in commit messages / a running `PROGRESS.md`, but do not stop and wait for approval between stages unless you hit a genuine ambiguity not covered by this file. |

If you hit a decision point **not** covered by this table or the rest of this file,
resolve it yourself using the "simplest maintainable approach" principle, write down
what you chose and why in `docs/decisions.md`, and keep moving. Only stop and ask the
user if the ambiguity materially changes architecture (e.g., something that would be
very expensive to reverse).

---

## 2. Required Process (do this before writing feature code)

1. **Inspect the repository.** Check what already exists before assuming an empty
   repo. If there's partial scaffolding, understand it before extending it.
2. **Write `docs/architecture.md`** covering: module boundaries (api / domain /
   db / ai-rag / integrations), request flow, how the LLM abstraction trait is
   shaped, how the RAG pipeline stages map to code.
3. **Write `docs/schema.md`** with the full Postgres schema (tables, columns,
   types, indexes, the pgvector column + index type) before writing migrations.
4. **Write `docs/api.md`** listing every REST endpoint, method, request/response
   shape, and every AI tool function with its parameter schema.
5. **Write `docs/frontend.md`** with route structure, state management approach,
   and component boundaries.
6. **List major dependencies** (Rust crates, npm packages) with one-line
   justification for each non-obvious choice.
7. Only after 2–6 exist and are internally consistent, start implementing.

Do not skip straight to code generation. A design doc that's wrong is cheap to fix;
a schema that's wrong after 40 tables of application code depend on it is not.

---

## 3. Database Schema — Required Entities

At minimum, design tables for:

- `tasks` (title, description/notes, status, priority, due_date, scheduled_start,
  scheduled_end, recurrence_rule, project_id, parent_task_id for subtasks,
  completed_at, created_at, updated_at)
- `task_tags` / `tags` (many-to-many)
- `projects`
- `calendar_events` (title, description, start_time, end_time, recurrence_rule,
  reminder settings, and a nullable `source_task_id` FK so task-driven calendar
  entries are derivable rather than duplicated data — see §4)
- `documents` (filename, title, source, doc_type, upload_date, tags, status —
  e.g. pending/indexed/failed)
- `document_chunks` (document_id FK, chunk_text, chunk_index, embedding
  `vector(1536)`, token_count)
- `conversations` and `conversation_messages` (role, content, tool_calls,
  tool_results, created_at) — chat history persists
- `products` (as a category-agnostic entity: name, category, and a flexible
  attributes store — e.g. JSONB — since attributes vary wildly between a
  microcontroller and a laptop; do not model fixed columns per product type)

Use proper foreign keys, `NOT NULL` where correct, and indexes on anything
queried by (due_date, status, document_id, embedding via ivfflat/hnsw).

Use `sqlx` migrations (or `sea-orm` migrations if you choose sea-orm as the
query layer — pick one query layer and justify it in `docs/decisions.md`).

---

## 4. Calendar/Task Integration (important design point)

A task with a due date or scheduled time must appear on the calendar **without
creating a duplicate, driftable copy of that data**. Two acceptable approaches:

- **(a)** The calendar view queries `tasks` (where due_date/scheduled_start is
  set) and `calendar_events` together and merges them at read time, or
- **(b)** `calendar_events` can optionally reference a `source_task_id`, and
  such rows are treated as read-only projections, regenerated/kept in sync
  when the task changes.

Pick one, document it in `docs/decisions.md`, and make sure updating a task's
due date is reflected on the calendar with no manual sync step.

---

## 5. RAG Pipeline — Required Shape

Follow this pipeline; do not collapse steps:

```
Document → text extraction → chunking (fixed-size, token-based, w/ overlap)
  → embeddings → pgvector storage → retrieval (vector similarity search)
  → [optional reranking — stub the interface, real reranker can be v2]
  → context construction (kept separate from system prompt, not concatenated in)
  → LLM generation → response with citations back to source document(s)/chunk(s)
```

Required config (env vars, with sensible defaults):

- `RAG_CHUNK_SIZE_TOKENS` (default: 512)
- `RAG_CHUNK_OVERLAP_TOKENS` (default: 64)
- `RAG_RETRIEVAL_TOP_K` (default: 6)
- `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, `EMBEDDING_DIMENSIONS`

Every RAG-grounded answer must return which document(s)/chunk(s) it used, and
the frontend must display these as visible citations — not just log them.

Document ingestion formats required in v1: PDF, TXT, Markdown, DOCX. Structure
the extractor as a trait/interface keyed by file type so more formats can be
added by implementing one function, not touching the pipeline.

---

## 6. AI Tool-Calling Contract

The LLM must never touch SQL or the database directly. All backend operations
the assistant can perform go through explicit tool functions with JSON-schema
parameters, at minimum:

`create_task`, `update_task`, `delete_task`, `list_tasks`, `search_tasks`,
`create_event`, `update_event`, `search_documents`, `retrieve_knowledge`,
`summarize_document`.

Each tool function: validated input, explicit error returns (not panics), and
a corresponding entry in `docs/api.md`. The chat endpoint orchestrates the
tool-call loop (model requests tool → backend executes → result fed back to
model → model responds) using OpenAI-style native function calling as
configured in §1.

---

## 7. MVP Build Order

Do not build everything in parallel. Each stage must leave the app in a
working, `docker compose up -d`-able state before moving to the next.

**Stage 0 — Foundation**
Postgres + pgvector container, Rust/Axum skeleton with health check, React/Vite
skeleton, Docker Compose wiring all three, migrations tooling in place.

**Stage 1 — Tasks & Projects**
Full CRUD for tasks/projects/tags/subtasks/recurrence, Today/Upcoming/Overdue/
Kanban/Project views in the frontend.

**Stage 2 — Calendar**
Events CRUD, month/week/day views, task↔calendar integration per §4.

**Stage 3 — Documents & RAG**
Upload endpoint, extraction for all 4 formats, chunking, embeddings,
pgvector storage, retrieval endpoint with citations. Test retrieval quality
manually before moving on.

**Stage 4 — AI Chat**
Chat UI, conversation persistence, tool-calling loop wired to all tools in §6,
LLM provider abstraction driven by env vars.

**Stage 5 — Dashboard, Global Search, Docker polish**
Dashboard aggregating the above, global search (Postgres full-text +
pgvector), final Dockerfile/compose hardening, README.

**Deferred to post-MVP (do not build in v1 unless explicitly asked):**
product comparison UI, notifications/reminders delivery, external calendar
sync (Google Calendar), reranking model, multi-user auth.

At the end of **every stage**: run `cargo test`, `cargo fmt --check`,
`cargo clippy`, frontend lint/typecheck, build the Docker images, fix any
failures, update the README, and record what was completed.

---

## 8. Configuration — `.env.example` Must Include

```
# Database
DATABASE_URL=postgres://user:password@postgres:5432/workassistant

# LLM
LLM_PROVIDER=openai            # openai | openai_compatible
LLM_API_KEY=
LLM_BASE_URL=                  # required for openai_compatible (e.g. local Ollama/vLLM URL)
LLM_MODEL=gpt-4o-mini

# Embeddings
EMBEDDING_PROVIDER=openai
EMBEDDING_MODEL=text-embedding-3-small
EMBEDDING_DIMENSIONS=1536

# RAG tuning
RAG_CHUNK_SIZE_TOKENS=512
RAG_CHUNK_OVERLAP_TOKENS=64
RAG_RETRIEVAL_TOP_K=6

# App
APP_URL=http://localhost:5173
BACKEND_PORT=8080
LOG_LEVEL=info
```

Never commit a populated `.env`. `.env.example` ships with placeholders only.

---

## 9. Code Quality Bar

- Strong typing throughout; no `serde_json::Value` escape hatches in domain
  logic (fine at the LLM tool-call boundary where shape is genuinely dynamic).
- Clear module boundaries matching `docs/architecture.md`.
- All fallible operations return `Result`, no unexplained `.unwrap()` in
  request-handling paths.
- `tracing` for structured logging.
- Migrations checked into `migrations/`, never hand-edited after being applied.
- Tests for: task CRUD logic, RAG chunking, retrieval ranking, tool-call
  parameter validation. Not aiming for 100% coverage — cover the things that
  are easy to silently break.
- No Kubernetes, no microservices, no Kafka, no Redis, no extra infra beyond
  the 3 containers in §1 unless a concrete need appears and is documented.

---

## 10. Definition of Done for "MVP Complete"

- `docker compose up -d` from a clean clone (with a filled-in `.env`) produces
  a working app: create a task, see it on the calendar, upload a PDF, ask the
  chat assistant a question about it and get a cited answer, ask the assistant
  to create a task and see it appear.
- README documents setup, env vars, the no-auth/reverse-proxy expectation, and
  known limitations (no reranking yet, no multi-user, tool-calling requires a
  function-calling-capable LLM endpoint).
- `docs/architecture.md`, `docs/schema.md`, `docs/api.md`, `docs/frontend.md`,
  `docs/decisions.md` all exist and reflect what was actually built (update
  them if reality diverged from the plan — don't leave stale docs).

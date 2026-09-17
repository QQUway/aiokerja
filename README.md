# Work Assistant

Self-hostable, all-in-one personal work assistant: task manager + calendar + RAG knowledge base
+ AI chat assistant with tool calling. Runs as three containers via `docker compose up -d`.

## Stack

| Layer | Choice |
|---|---|
| Backend | Rust + Axum 0.7, `sqlx` 0.7 (runtime queries, no macros against a live DB) |
| Database | PostgreSQL 16 + `pgvector` (HNSW index on chunk embeddings) |
| Frontend | TypeScript + React 18 + Vite, TanStack Query, react-router-dom, plain CSS |
| LLM | Any OpenAI-compatible API (`openai` or `openai_compatible`: Ollama, LM Studio, vLLM, …) |
| Embeddings | OpenAI-compatible embeddings endpoint (`text-embedding-3-small`, 1536 dims by default) |

## Quick start

```bash
cp .env.example .env      # then fill in LLM_API_KEY / LLM_BASE_URL / LLM_MODEL
docker compose up -d
```

- Frontend: http://localhost:5173
- Backend API: http://localhost:8080/api (health: http://localhost:8080/health)
- Postgres: localhost:5432 (`user` / `password` / `workassistant`)

> **Security — read this.** v1 has **no authentication**. There is no login, no session, no
> credentials. The app assumes it is reachable only from a trusted network. You **must** put it
> behind something that enforces access control: Tailscale/WireGuard, a VPN, or a reverse proxy
> with basic auth. Do **not** expose port 5173/8080 directly to the public internet.
> The backend is structured so real multi-user auth can be added later (handlers already take a
> `CurrentUser` extractor that currently returns a single fixed user id), but it is **not** there yet.

## Environment variables

| Variable | Default | Notes |
|---|---|---|
| `DATABASE_URL` | `postgres://user:password@postgres:5432/workassistant` | Compose service hostname is `postgres` |
| `LLM_PROVIDER` | `openai` | `openai` or `openai_compatible` |
| `LLM_API_KEY` | — | Bearer token; leave empty for local servers without auth |
| `LLM_BASE_URL` | — | Required for `openai_compatible`. Use the full OpenAI-compatible root, including the version segment (`http://host.docker.internal:11434/v1`, `https://generativelanguage.googleapis.com/v1beta/openai`). Used verbatim |
| `LLM_MODEL` | `gpt-4o-mini` | Must be a **function-calling-capable** model (see limitations) |
| `EMBEDDING_PROVIDER` | `openai` | OpenAI-compatible embeddings |
| `EMBEDDING_MODEL` | `text-embedding-3-small` | |
| `EMBEDDING_DIMENSIONS` | `1536` | Drives the pgvector column width |
| `RAG_CHUNK_SIZE_TOKENS` | `512` | Fixed-size chunking target |
| `RAG_CHUNK_OVERLAP_TOKENS` | `64` | Word-level overlap |
| `RAG_RETRIEVAL_TOP_K` | `6` | Chunks passed to the model |
| `APP_URL` | `http://localhost:5173` | |
| `BACKEND_PORT` | `8080` | |
| `LOG_LEVEL` | `info` | `tracing` `EnvFilter` directive |

Never commit a filled-in `.env`. `.env.example` ships with placeholders only.

## Local development (without Docker)

```bash
# Postgres with pgvector
docker run -d --name wa-pg -p 5432:5432 -e POSTGRES_USER=user \
  -e POSTGRES_PASSWORD=password -e POSTGRES_DB=workassistant pgvector/pgvector:pg16

# Backend (needs DATABASE_URL pointing at localhost)
cd backend && cargo run

# Frontend (proxies /api to localhost:8080)
cd frontend && npm install && npm run dev
```

Checks:

```bash
cd backend && cargo test && cargo fmt --check && cargo clippy
cd frontend && npm run lint && npm run build
```

## How it works

- **Calendar/task integration** — `GET /api/calendar?from&to` merges real `calendar_events` with
  read-only projections of tasks that have a `due_date` or `scheduled_start` (items tagged
  `kind: "task"`). Tasks are never duplicated; changing a due date is reflected immediately.
  `POST /api/tasks/:id/calendar-event` materializes a real, editable event linked via
  `source_task_id` when you want one.
- **RAG pipeline** — upload (PDF/TXT/Markdown/DOCX) → text extraction (trait per format) →
  token chunking with overlap → embeddings → pgvector storage → cosine similarity retrieval →
  reranker interface (no-op in v1) → context assembled separately from the system prompt →
  answer with visible citations in the UI.
- **Tool calling** — the chat endpoint runs the OpenAI-style tool loop (cap: 8 iterations) over
  ten tools: `create_task`, `update_task`, `delete_task`, `list_tasks`, `search_tasks`,
  `create_event`, `update_event`, `search_documents`, `retrieve_knowledge`,
  `summarize_document`. The model never touches SQL.
- **Embeddings dimension** — the `embedding vector(N)` column and its HNSW index are created at
  startup from `EMBEDDING_DIMENSIONS`, guarded by the `embedding_index_meta` table.

## Known limitations (v1, by design)

- **No authentication.** See the security note above. Deployment behind a trusted network or
  authenticating proxy is a requirement, not an oversight.
- **Tool calling requires a function-calling-capable model/endpoint.** There is no prompt-based
  JSON fallback. OpenAI models, or local servers with tool support (vLLM, Ollama with tool-capable
  models), are required. Plain text-completion endpoints will not work for the assistant.
- **Changing the embedding model or dimensions after documents are indexed requires re-indexing.**
  The startup guard fails loudly on mismatch; delete `document_chunks` (or the documents and
  re-upload) and restart to rebuild.
- **No reranking yet** — the `Reranker` trait exists with a no-op implementation; a real
  cross-encoder is v2.
- **Single user** — all data belongs to one fixed user id.
- **Recurrence rules are stored, not expanded.** A task/event with `recurrence_rule` shows once;
  no RRULE expansion happens yet.
- **No notifications/reminders delivery**, no external calendar sync, no product comparison UI.
- Retrieval has a fixed cosine-similarity floor (0.30); very short queries can return no hits.

## Repository layout

```
backend/    Rust + Axum API, sqlx migrations, RAG + tool-calling
  src/domain   typed entities (no sqlx::Value in domain logic)
  src/db       SQL access per entity
  src/ai       llm / embeddings / extract / chunk / rag / tools / ingest
  src/routes   HTTP handlers, one module per resource
frontend/   React + Vite SPA
docs/       architecture, schema, api, frontend, decisions
```

See `docs/decisions.md` for the reasoning behind each notable choice.

# PROGRESS — Work Assistant MVP

> **Checkpoint file.** Read this first when resuming. It records what is done, what is
> verified, and exactly what to run next.

Last updated: docker deployment phase. Backend + frontend fully green. **Full 3-container
`docker compose` stack built and verified live.** Dev-mode (no docker build) also verified.

---

## 0. Read this before doing anything

**Verified state:**
- `cargo build` — 0 errors (only a `.future-incompat` note from the `sqlx-postgres v0.7.4`
  dependency, harmless).
- `cargo test` — 13/13 pass (chunking, docx extraction, tool schema, uuid/datetime validation).
- `cargo clippy --all-targets` — 0 lints.
- `cargo fmt --check` — clean.
- Frontend `npx tsc --noEmit` — clean. `npm run build` (vite) — succeeds.
- **Dev-mode stack verified live**: postgres container (5432) + native backend (8080) +
  vite dev server (5173, proxy → 127.0.0.1:8080). `GET /api/health` 200, task creation via
  `POST /api/tasks` verified. Use `./dev.sh`.
- **Docker stack verified live**: `docker compose build` succeeded (both images), then
  `docker compose up -d` → all 3 containers up (`postgres` healthy, `backend` 8080,
  `frontend` 5173). `GET /api/health` 200 direct (8080) and via frontend nginx proxy
  (5173/api/health) 200; `POST /api/tasks` through the proxy 200; `GET /api/projects` 200;
  delete 200.

**Not yet verified (DoD, §5):**
- RAG/chat (needs `LLM_API_KEY` in `.env`); PDF upload; citations; assistant-created tasks.

**Next action for a new session:**

```bash
cd /home/qquway/Projects/AIOKerja
./dev.sh status          # if you want the native dev stack
./dev.sh up              # start postgres + backend + frontend natively
docker compose up -d     # full docker stack (images already built)
```

Then walk the Definition of Done (§5) — task → calendar, PDF → chat with citations.

---

## 1. Phase status

| Phase | Scope | Code | Verified |
|---|---|---|---|
| Design docs | architecture / schema / api / frontend / decisions | ✅ done | ✅ internally consistent |
| Stage 0 — Foundation | compose, Postgres+pgvector, Axum skeleton, React/Vite skeleton, migrations | ✅ done | ✅ build/test/clippy/fmt + native run |
| Stage 1 — Tasks & Projects | CRUD tasks/projects/tags/subtasks + views | ✅ done | ✅ API verified (create task via curl) |
| Stage 2 — Calendar | events CRUD, month view, task↔calendar read-time merge + materialize | ✅ done | ⚠️ code green, not UI-walked |
| Stage 3 — Documents & RAG | upload, 4 extractors, chunking, embeddings, pgvector, retrieval w/ citations | ✅ done | ⚠️ needs LLM key for end-to-end |
| Stage 4 — AI Chat | chat UI, persistence, tool loop, provider abstraction | ✅ done | ⚠️ needs LLM key |
| Stage 5 — Dashboard/Search/Docker/README | dashboard, global search, Dockerfiles, nginx, README | ✅ done | ✅ docs/frontend verified; **docker stack built + up + smoke-tested** |

Everything in AGENTS.md §7 is implemented. Deferred per AGENTS.md: product comparison UI,
notifications delivery, external calendar sync, real reranker, multi-user auth.

---

## 2. Runtime bugs found + fixed (only by actually running)

1. **Startup crash — embedding schema decode.** `ensure_embedding_schema` read
   `embedding_index_meta.dimensions` (SQL `INT4`) into an `i64` → sqlx decode error
   (`INT8 not compatible with INT4`). Fixed in `db/mod.rs`: column now read as `i32`
   (`Option<(i32, String)>`, `i32::try_from(config.embedding_dimensions)`). Backend starts
   clean; migrations + HNSW index idempotent.
2. **Vite dev proxy ECONNREFUSED.** Vite target was `http://localhost:8080`; node resolves
   `localhost` → `::1` (IPv6) but the backend binds IPv4 → proxy 500. Fixed in
   `frontend/vite.config.ts`: target `http://127.0.0.1:8080`. Dev-only; prod nginx.conf
   (proxies `backend:8080`) untouched.

---

## 3. File inventory

### Root
- `AGENTS.md` — the spec (pre-existing)
- `docker-compose.yml` — 3 services: `postgres` (pgvector/pgvector:pg16, pg_isready healthcheck),
  `backend` (build ./backend, 8080, env_file .env, depends_on healthy), `frontend`
  (build ./frontend, 5173:80). Volume `pgdata`.
- `.env.example` — every var from AGENTS.md §8 (placeholders only); `.env` (gitignored)
  present locally, copied from `.env.example`.
- `dev.sh` — **native dev helper**: `up|down|nuke|status|logs`. Postgres in docker only;
  backend = cargo binary, frontend = vite (both hot-reload). Verified working.
- `.gitignore`, `README.md` (setup, env table, loud no-auth/reverse-proxy warning,
  re-index-on-dimension-change, known limitations), `PROGRESS.md` (this file)
- `docs/architecture.md`, `docs/schema.md`, `docs/api.md`, `docs/frontend.md`, `docs/decisions.md`

### backend/
- `Cargo.toml` — axum 0.7 (multipart,json), tokio full, tower-http 0.5 (cors,trace,limit),
  sqlx 0.7 (runtime-tokio-rustls,postgres,uuid,chrono,json,migrate,macros), pgvector 0.3 (sqlx),
  serde/serde_json, uuid v4+serde, chrono serde, tracing + tracing-subscriber, thiserror, anyhow,
  async-trait, reqwest 0.12 (json,rustls-tls), dotenvy, sha2, hex, pdf-extract 0.7, zip 0.6,
  quick-xml 0.31; `[profile.release] lto = "thin"`
- `Dockerfile` — **rust:1.97-bookworm** builder (dep-cache stub trick) → debian:bookworm-slim.
  (Bumped from 1.83: newest crate deps require edition2024.)
- `.dockerignore`, `migrations/0001_init.sql` (full schema; `document_chunks.embedding` is
  intentionally NOT here — added at startup from config, decision D2)
- `src/main.rs` — dotenv → Config → tracing → init_db → providers → reset stuck indexing →
  spawn pending ingestion → CORS(Any) → TraceLayer → routes::api_router() → serve
- `src/config.rs`, `src/error.rs` (AppError + JSON envelope), `src/state.rs`
  (`AppState { pool, config, llm, embeddings, reranker }`), `src/auth.rs` (no-op CurrentUser,
  generic over `S: Send + Sync`, fixed LOCAL_USER_ID)
- `src/domain/` — typed entities + New/Update payloads (`Option<Option<T>>` nullable pattern)
- `src/db/` — tasks (dynamic QueryBuilder list + count), projects (?cascade), tags, events
  (+ merged `calendar_items`), documents (chunks, pgvector insert, `retrieve_chunks` cosine +
  ChunkHit), chat, search, dashboard
- `src/ai/` — llm (OpenAI-compatible, tool-calling), embeddings (batch, dim mismatch error),
  extract (trait: txt/md/pdf/docx via zip+quick-xml), chunk (HeuristicTokenizer
  ceil(chars/4), word-window overlap), rag (NoopReranker, 0.30 score floor, build_context),
  tools (10 defs + execute), ingest (tokio::spawn, reindex, process_pending)
- `src/routes/` — mod.rs (root /health + /api/*), health, tasks (+ views/summary,
  :id/calendar-event), projects, tags, events (+ /calendar merged), documents (multipart `file`,
  raw, chunks, reindex), chat (tool loop cap 8 + context injection from retrieve_knowledge),
  search, dashboard, products

### frontend/
- `package.json`, `vite.config.ts` (dev proxy `/api` → **127.0.0.1:8080**), tsconfig[s],
  index.html, Dockerfile (node:20 → nginx:1.27-alpine), nginx.conf (SPA fallback,
  `/api/` → backend:8080, 50m upload), .dockerignore
- `src/main.tsx`, `src/App.tsx`, `src/styles.css`, `src/api/` (client, types, per-domain Query
  hooks), `src/components/` (StatusBadge, PriorityBadge, TaskForm, EventForm, TaskCard,
  CitationList), `src/pages/` (Dashboard, Tasks, Kanban, Projects, ProjectDetail, Calendar,
  Documents, DocumentDetail, Chat, Search, Products)

---

## 4. Decisions locked (full text in `docs/decisions.md`)

D1 sqlx 0.7 runtime query_as (no compile-time macros; builds without a live DB).
D2 embedding column + HNSW index created at startup from EMBEDDING_DIMENSIONS, guarded by
embedding_index_meta; mismatch fails loudly.
D3 calendar = read-time merge; task projections never written to calendar_events;
source_task_id only for explicit materialize.
D4 heuristic tokenizer ceil(chars/4).
D5 in-process tokio::spawn ingestion; stuck `indexing` reset to `pending` at startup.
D6 no-op CurrentUser, fixed user id.
D7 Reranker trait + NoopReranker.
D8 TanStack Query, no global store. D9 plain CSS tokens. D10 tool loop cap 8.
D11 products minimal CRUD. D12 global search = ILIKE (+ tsvector indexes for later).

---

## 5. Definition of Done — walkthrough checklist

1. Create a task → appears in Tasks list and on Kanban.
2. Give it a due date → appears on Calendar (amber `kind: task`) without manual sync.
3. "To calendar" on that task → real editable event (indigo).
4. Upload a PDF → pending → indexing → indexed; `/documents/:id` shows chunks.
5. Ask chat assistant about that PDF → answer with visible citations.
6. Ask assistant to create a task → appears in Tasks list.
7. `cargo test` / `cargo fmt --check` / `cargo clippy` / `npm run lint` / `npm run build` clean.
8. README states no-auth/reverse-proxy requirement and known limitations.

**Done through #1 + #7 (backend) + frontend compile/build + full docker stack up (health, task create/list/delete via the nginx proxy).** #2–#6 need the .env LLM key (or a
local OpenAI-compatible endpoint) and a running stack.

---

## 6. Environment notes

- Docker: daemon via `sudo systemctl start docker` + `sudo chmod 666 /var/run/docker.sock`
  (repeat if down). Compose plugin v5.5.1 at `~/.docker/cli-plugins/docker-compose`.
- **Docker deploy verified**: `docker compose build` (buildx v0.21.0) builds both images;
  `docker compose up -d` runs `postgres` (healthy) + `backend` (8080) + `frontend` (5173).
  Backend Dockerfile is `rust:1.97-bookworm` (1.83 fails: newest crate deps need edition2024).
  Frontend image = node:20 build → nginx:1.27-alpine (SPA fallback + `/api/` → `backend:8080`).
- To run docker instead of native dev: `./dev.sh down` (or kill by port), then
  `docker compose up -d`. Ports 8080/5173 clash between the two modes — only one at a time.
- **buildx plugin** installed at `~/.docker/cli-plugins/docker-buildx` (v0.21.0 — the v0.25.0
  binary segfaults; GitHub release asset name is `buildx-vX.Y.Z.linux-amd64`, NOT
  `docker-buildx-*`; the `/latest/download` URL 404s — pin an explicit version tag). Without
  it, compose warns "falling back to the classic builder" (cosmetic; classic builder works).
- Toolchain: cargo 1.97.1, node v26.8.2, npm 12.0.2, psql client 18.6. No sqlx-cli (unneeded).
- No local postgres server/pgvector .so — **postgres runs only as the compose container**;
  native backend connects to `localhost:5432`.
- dotenvy walks UP parent dirs: root `.env` (host `postgres`) is picked up from `backend/` —
  always run native backend with explicit `DATABASE_URL=…@localhost:5432/workassistant`.
- Background processes must be started with `setsid nohup … </dev/null & disown` or they die
  when the invoking shell exits. Vite binds IPv6 `[::1]` only — hit it via `localhost`.
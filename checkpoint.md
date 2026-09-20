# AIOKerja — Development Checkpoint

> Living handoff doc. Update at the end of every session so any model/machine can resume.
> Last updated: 2026-09-20 · by Claude Opus 4.8 (planning session)

## 1. Project snapshot
- Stack: Rust/Axum + Postgres16/pgvector (backend, :8080) · React18/Vite/TanStack Query (frontend, :5173) · self-contained RAG · OpenAI-compatible LLM (default Gemini).
- Run: `./dev.sh` or `docker compose up`. Env: copy `.env.example` → `.env` (LLM_*, EMBEDDING_*, DATABASE_URL).
- No auth (single local user, backend/src/auth.rs).
- Full plan: see `plan.md`.

## 2. Conventions (do not fight these)
- Backend: raw sqlx (no ORM), trait registries (extract.rs/tools.rs), errors via AppError/AppResult, vector column added at runtime in db/mod.rs.
- Frontend: one global styles.css (tokens in :root) + 98.css; per-resource TanStack Query hooks in src/api/; no global store; raw HTML elements.

## 3. Decisions log
- Win95 UI via 98.css (native-element styling).
- Comparison uses in-repo pgvector RAG (not external ArdRAG).
- Datasheets = AIDC hardware (RFID readers/antennas, handhelds, barcode/RFID printers, scanners; Zebra/Chainway/TSC/Honeywell); category-aware extraction.

## 4. Workstream progress
### W1 — Seamless flow — DONE
- [x] Chat type-and-go (auto-create conversation on first send, ChatPage.tsx handleSend)
- [x] EventForm end = start+1h default (fills only if end empty)
- [x] Replace confirm() deletes with Win95 ConfirmDialog (Chat/Tasks/Documents/Projects/Products)
- [x] Optimistic delete (onMutate/onError/onSettled) for tasks, documents, projects, products, conversations
### W2 — Win95 UI — DONE
- [x] Vendor + import 98.css (frontend/package.json, main.tsx imports "98.css/dist/98.css")
- [x] Rework :root tokens (teal desktop, silver surface, zero radius, bevel box-shadow vars, Pixelated MS Sans Serif)
- [x] Window chrome in App.tsx (.desktop > .window > .title-bar + .window-body) + sidebar restyle
- [x] Restyle badges/cards/kanban/chat/calendar (all bevels via --bevel-raised/sunken/window)
- [x] Reusable Win95Dialog + ConfirmDialog components (frontend/src/components/)
- Verified via Playwright screenshots against dev server (backend not running in dev sandbox — no docker daemon/postgres available); chrome, bevels, and confirm dialog all render correctly. Full end-to-end (real backend + data) not yet verified — do that when docker is available.
- Gotcha hit & fixed: `.conversation-list button` in styles.css unintentionally collided with `button.primary` (same specificity, later wins) and killed the "New" button's styling — fixed by scoping to `.conversation-list .conv-scroll button`. Watch for similar broad `<container> button` selectors when adding new buttons inside styled containers.
### W3 — Datasheet extract + compare — DONE
- [x] Migration 0002_datasheets.sql (brand/model/device_type/source_document_id on products) + domain/product.rs (DEVICE_TYPES const)
- [x] ai/datasheet.rs — category-aware field hints per device_type, LLM extraction (strict JSON-only prompt, code-fence stripping), extract_and_save() shared by route + tool
- [x] db/products.rs — get_by_ids, find_by_source_document, upsert_from_datasheet (one product per source document, re-extract overwrites)
- [x] db/documents.rs — retrieve_chunks_for_documents (doc-scoped vector search, used only by compare)
- [x] ai/compare.rs — compare_products(): union-of-keys spec matrix + RAG-grounded summary scoped to selected products' source_document_id's, citations via existing rag::citations_from
- [x] POST /documents/:id/extract-datasheet, POST /products/compare + extract_datasheet/compare_products chat tools (ai/tools.rs)
- [x] Frontend: DocumentDetailPage extract action, ProductsPage as datasheet catalog (brand/model/device_type columns, multi-select, Compare button), ProductComparePage (matrix with diff highlighting, grounded summary, citations), api/products.ts hooks, api/types.ts (Product fields, ComparisonResult)
- Backend: `cargo build`, `cargo test` (17 passed, incl. new datasheet.rs unit tests), `cargo clippy` all clean (only pre-existing sqlx-postgres future-incompat warning).
- Frontend: verified via mocked Playwright flow (dev server + route mocks, no live backend) — catalog renders, selection works, compare view shows aligned matrix with amber diff-highlighting, grounded summary text, and citations list. Screenshots confirmed visually correct Win95 styling throughout.
- Gotcha hit & fixed: 98.css hides real `<input type="checkbox">` (opacity:0, position:fixed) and draws the glyph via `label:before` — checkboxes MUST use sibling `id`/`htmlFor` markup (not `<label>` wrapping the input) or they're invisible/unclickable. Fixed in EventForm's "All day" checkbox and ProductsPage's row-select checkboxes. A label with only an `.sr-only` child collapses to 0×0 (pseudo-element content doesn't size the box) — added `.checkbox-only-label` (13x13px) utility class for icon-only checkboxes. Watch for this pattern with any future checkbox.
- Not yet done: full end-to-end test against a real running backend + Postgres + live LLM (no docker daemon available in the dev sandbox this session). Do this next: `docker compose up`, upload a real Zebra/Chainway/TSC/Honeywell PDF datasheet, run extract-datasheet, then compare two real products.

## 5. Where I left off
All three workstreams (flow, Win95 UI, datasheet extract/compare) are implemented and individually verified (typecheck/build/tests clean, UI verified via mocked Playwright screenshots). Nothing has been committed to git yet — review the diff and commit when ready. Next step for a fresh session: run `docker compose up`, do one real end-to-end pass (upload → extract → compare) with live data, then commit.

## 6. Open questions / blockers
None blocking. Real-backend end-to-end verification (see W3 note above) is the main remaining gap before calling this fully done.

# AIOKerja Improvement Plan — Flow, Win95 UI, Datasheet Compare

## Context

AIOKerja ("Work Assistant") is a single-user, self-hostable work assistant: Rust/Axum + Postgres 16/pgvector backend, React 18 + Vite SPA frontend, with a self-contained RAG pipeline and a tool-calling LLM chat loop. This plan covers three improvements:

1. **Usage flow** — make the app feel seamless and fluid, not restrictive.
2. **UI** — give it a retro Windows 95 look.
3. **Datasheet feature** — extract structured specs from AIDC hardware datasheets, compare them side-by-side, and ground the comparison in the existing RAG infra.

See `checkpoint.md` for live progress and handoff state between sessions/models/machines.

### Decisions locked in
- **Win95 UI** → vendor the open-source **98.css** library. The app uses raw HTML elements styled by CSS variables in one `styles.css`, so 98.css (which styles native elements) is the fastest authentic fit.
- **Comparison RAG** → extend the **in-repo pgvector RAG** (no external ArdRAG dependency; stays self-hostable).
- **Datasheet domain** → **AIDC / auto-ID hardware**: RFID readers, RFID antennas, handheld computers, barcode printers, RFID printers, scanners — brands like Zebra, Chainway, TSC, Honeywell. Extraction is **device-category-aware**.

### Key facts about the current code
- Frontend: one global `frontend/src/styles.css` (~909 lines) with all tokens in `:root`; routing + sidebar layout in `frontend/src/App.tsx`; per-resource TanStack Query hooks in `frontend/src/api/`; **no** component library, **no** modals, all raw HTML elements.
- Backend: tool registry + dispatch in `backend/src/ai/tools.rs`; plain-text extraction in `backend/src/ai/extract.rs`; RAG in `backend/src/ai/rag.rs` + `backend/src/db/documents.rs::retrieve_chunks`; single migration `backend/migrations/0001_init.sql` (vector column added at runtime in `backend/src/db/mod.rs`).
- `products` table exists as a stub (`{id, name, category, attributes jsonb, ...}`) with CRUD only (`backend/src/routes/products.rs`, `frontend/src/pages/ProductsPage.tsx`). Comparison was explicitly deferred to v2 — this is the anchor for the new feature.
- No auth (single fixed local user via `backend/src/auth.rs`).

---

## Workstream 1 — Seamless usage flow

Goal: remove the friction points found during exploration. All changes are frontend-only (plus optional smart defaults). Ship as small, independent commits.

1. **Chat: type-and-go.** In `frontend/src/pages/ChatPage.tsx`, when `selected` is null and the user sends a message, auto-create the conversation then send. Remove the `disabled={!selected}` gate on the input/send button and drop the "pick a chat first" empty-state block in favor of an always-usable composer.
2. **Smart form defaults.** In `frontend/src/components/EventForm.tsx`, default `end_time` to `start + 1h` when a start is picked and end is empty; keep it editable.
3. **Replace blocking `confirm()` deletes** (ChatPage, TasksPage, DocumentsPage, ProjectsPage, ProductsPage) with a Win95 dialog (built in Workstream 2).
4. **Optimistic UX** for common mutations (delete, create) via TanStack Query `onMutate`/rollback.
5. **Products JSON rough edge** — the raw `attributes` JSON textarea in `ProductsPage.tsx` gets superseded by the structured datasheet form from Workstream 3.

## Workstream 2 — Windows 95 UI (98.css)

Goal: authentic Win95 chrome while keeping the SPA structure and TanStack data flow intact.

1. **Vendor 98.css.** Add as an npm dep, imported in `frontend/src/main.tsx` before `./styles.css`.
2. **Rework the token layer.** In `frontend/src/styles.css`, replace the warm-paper palette in `:root` with Win95 tokens: teal desktop, silver surfaces, 3D bevel borders, zero radius, MS Sans Serif / Tahoma font stack.
3. **Window-chrome the shell.** In `frontend/src/App.tsx`, wrap layout/content in 98.css `.window` + `.title-bar` markup. Restyle the sidebar as a Win95 list/tree panel.
4. **Restyle bespoke components** 98.css doesn't cover: badges, cards, kanban columns, chat bubbles, calendar grid.
5. **Build a reusable Win95 dialog** component (used by Workstream 1 delete-confirm and the compare view).
6. Keep the responsive breakpoint working.

## Workstream 3 — Datasheet extraction + comparison + RAG

Goal: upload an AIDC hardware datasheet → extract structured, category-aware specs into a product → compare products in a spec matrix, grounded in RAG citations.

### 3a. Data model
Migration `backend/migrations/0002_datasheets.sql` extends `products`: `brand text`, `device_type text`, `model text`, `source_document_id uuid REFERENCES documents(id)`. Reuse existing `attributes jsonb` for specs. Update `backend/src/domain/product.rs`.

### 3b. Structured extraction (LLM, category-aware)
`backend/src/ai/datasheet.rs`: base spec set + per-`device_type` field sets (RFID reader, RFID antenna, handheld computer, barcode/RFID printer, scanner). Reads `document_extra.raw_text`, sends with category schema to the LLM, writes `brand/model/device_type` + `attributes`.
- Endpoint: `POST /api/documents/:id/extract-datasheet`.
- Chat tool: `extract_datasheet` in `backend/src/ai/tools.rs`.

### 3c. Comparison (RAG-grounded)
Endpoint: `POST /api/products/compare` with `{ product_ids, question? }`. Builds aligned spec matrix, grounds a summary via doc-scoped `retrieve_chunks`, returns citations.
- Chat tool: `compare_products` in `ai/tools.rs`.

### 3d. Frontend
- `DocumentDetailPage.tsx`: "Extract datasheet → product" action.
- `ProductsPage.tsx`: datasheet catalog (brand/device_type columns, structured spec view, multi-select).
- New comparison view: spec matrix + highlighted differences + citations (reuse `CitationList.tsx`).
- New hooks in `frontend/src/api/products.ts`.

## Suggested sequencing
1. Deliverable 0 (`plan.md`, `checkpoint.md`) — done.
2. Workstream 2 UI shell (98.css + tokens + window chrome) — gives the dialog primitive.
3. Workstream 1 flow fixes (uses the dialog primitive).
4. Workstream 3 backend then frontend.

## Verification
- Backend: `cd backend && cargo build && cargo test`. Run via `./dev.sh` or `docker compose up`; confirm migration applies and vector schema still initializes.
- Datasheet flow: upload a real datasheet PDF → wait for `indexed` → extract → verify product row → compare two products → confirm matrix + citations. Also drive via Chat tools.
- Flow: Chat auto-creates on first message; EventForm end auto-fills; deletes use Win95 dialog not `confirm()`.
- UI: `cd frontend && npm run lint && npm run build`; visually confirm Win95 chrome and mobile breakpoint.

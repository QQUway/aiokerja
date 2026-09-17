# Frontend

TypeScript + React 18 + Vite. Client-side routing with `react-router-dom`.
Server state via TanStack Query (`@tanstack/react-query`); UI-local state in
components. No global store (D8). Plain CSS with design tokens (D9).

## Routes

| Path | Component | Purpose |
|---|---|---|
| `/` | `DashboardPage` | aggregates: task summary, upcoming events, recent docs, quick actions |
| `/tasks` | `TasksPage` | Today / Upcoming / Overdue / All list views (filter chips) |
| `/tasks/kanban` | `KanbanPage` | drag-free status columns (move via select to keep deps small) |
| `/projects/:projectId?` | `ProjectsPage` | project grid + tasks within a project |
| `/calendar` | `CalendarPage` | month / week / day views, merged task+event items, event CRUD modal |
| `/documents` | `DocumentsPage` | upload (drag-drop), list with indexing status, delete/reindex, view raw text |
| `/documents/:id` | `DocumentDetailPage` | chunks preview + raw text |
| `/chat` | `ChatPage` | message list with persisted conversations, inline citations |
| `/search` | `SearchPage` | global search results grouped by type |
| `/products` | `ProductsPage` | minimal CRUD list (v1, no comparison) |

## Component boundaries

```
src/
  api/client.ts       fetch wrapper (JSON, error envelope, base url)
  api/types.ts        TS types mirroring domain structs (Task, Event, Doc, Message, Citation…)
  api/{tasks,projects,events,documents,chat,search,dashboard,products}.ts  TanStack Query hooks
  components/layout.ts        AppShell: sidebar nav + main outlet
  components/TaskForm.tsx     create/edit task modal (used by Tasks + Calendar quick-add)
  components/EventForm.tsx    event modal
  components/TaskCard.tsx     task row with priority/status/due display
  components/CitationList.tsx citations rendered as clickable links (open doc detail)
  components/StatusBadge.tsx  priority/status pill shared across views
  pages/…                    one file per route above
  styles.css                  design tokens + component classes
  main.tsx  App.tsx  router.tsx
```

## State management
- TanStack Query for all server data: each page calls hooks, `invalidateQueries`
  after mutations. Optimistic updates only for task status changes (keep minimal).
- Polling: `DocumentsPage` refreshes document list every 5s while any doc is
  `pending`/`indexing` (`refetchInterval`).
- Chat page keeps `useMutation` for message send; streaming is out of scope (v1
  returns complete assistant reply).

## Key flows
- **Task appear on calendar**: calendar query hits `/api/calendar` which merges task
  projections server-side (D3) — frontend renders items with `kind` badge and
  treats `kind:task` as read-only except for "materialize event" action.
- **Cited answer**: chat message JSON contains `citations[]`; `CitationList`
  renders `<a href="/documents/:document_id">`. Knowledge-grounded answers always
  carry citations (per §5).

## Networking
Vite dev proxy: `/api` -> `http://localhost:8080`. In production the frontend
container serves static `dist/` (nginx-alpine) and proxies `/api` to the backend
container by service name.
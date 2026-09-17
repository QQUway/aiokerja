# Schema

PostgreSQL 16 + pgvector. Managed with `sqlx::migrate` (`migrations/0001_init.sql`).
Single exception (documented): the `embedding` column of `document_chunks` is
**not** created in the migration because its dimension is config-driven. At
startup `db/documents.rs::ensure_embedding_schema` runs the idempotent DDL:

```sql
ALTER TABLE document_chunks ADD COLUMN IF NOT EXISTS embedding vector($EMBEDDING_DIMENSIONS);
CREATE INDEX IF NOT EXISTS document_chunks_embedding_idx
  ON document_chunks USING hnsw (embedding vector_cosine_ops);
```

Changing `EMBEDDING_MODEL`/`EMBEDDING_DIMENSIONS` after documents are indexed
requires wiping `document_chunks` and re-indexing (documented in README).

`embedding_index_meta` records the dimension used for an index build so startup
can detect a mismatch and fail loudly instead of silently corrupting results.

## Tables

### users
Single row, fixed id `00000000-0000-0000-0000-000000000001` (auth placeholder, §1).

| column | type | notes |
|---|---|---|
| id | uuid PK | |
| name | text NOT NULL | |
| created_at | timestamptz NOT NULL default now() | |

### projects
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| name | text NOT NULL unique | |
| description | text | |
| color | text NOT NULL default '#4f46e5' | |
| created_at / updated_at | timestamptz NOT NULL | |

### tags
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| name | text NOT NULL unique | |
| color | text NOT NULL default '#6b7280' | |

### task_tags (m2m)
PK (task_id, tag_id), both FKs cascade.

### tasks
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| title | text NOT NULL | |
| description | text | |
| status | text NOT NULL default 'todo' | todo / in_progress / done / cancelled |
| priority | text NOT NULL default 'medium' | low / medium / high / urgent |
| due_date | timestamptz | |
| scheduled_start | timestamptz | |
| scheduled_end | timestamptz | |
| recurrence_rule | text | RRULE string |
| project_id | uuid FK projects (set null) | |
| parent_task_id | uuid FK tasks (set null) | subtask |
| completed_at | timestamptz | |
| created_at / updated_at | timestamptz NOT NULL | |

Indexes: `(status)`, `(due_date)`, `(project_id)`, `(parent_task_id)`, `(scheduled_start)`.

### calendar_events
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| title | text NOT NULL | |
| description | text | |
| start_time | timestamptz NOT NULL | |
| end_time | timestamptz NOT NULL | |
| all_day | boolean NOT NULL default false | |
| recurrence_rule | text | RRULE string |
| reminder_minutes | integer | before start |
| source_task_id | uuid FK tasks (set null) | read-only projection, §4 |
| created_at / updated_at | timestamptz NOT NULL | |

Indexes: `(start_time)`, `(source_task_id)`.

### documents
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| filename | text NOT NULL | original |
| title | text NOT NULL | derived |
| source | text NOT NULL default 'upload' | |
| doc_type | text NOT NULL | pdf / txt / md / docx |
| upload_date | timestamptz NOT NULL default now() | |
| content_hash | text | sha256, dedupe |
| status | text NOT NULL default 'pending' | pending / indexing / indexed / failed |
| error | text | last failure message |
| created_at / updated_at | timestamptz NOT NULL | |

extracted text lives in `document_chunks`; a `document_extra` table holds the
full raw text for overview/QA:

### document_extra
| column | type | notes |
|---|---|---|
| document_id | uuid PK FK cascade | |
| raw_text | text NOT NULL | extracted text |

### document_chunks
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| document_id | uuid FK documents cascade | |
| chunk_index | integer NOT NULL | |
| chunk_text | text NOT NULL | |
| embedding | vector(EMBEDDING_DIMENSIONS) | added at startup |
| token_count | integer NOT NULL | |
| created_at | timestamptz NOT NULL | |

Index: HNSW cosine on embedding (created at startup), plus `(document_id)`.

### conversations
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| title | text NOT NULL default 'New chat' | |
| created_at / updated_at | timestamptz NOT NULL | |

### conversation_messages
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| conversation_id | uuid FK conversations cascade | |
| role | text NOT NULL | user / assistant / system / tool |
| content | text | |
| tool_calls | jsonb | assistant-side tool call list |
| tool_results | jsonb | matching tool results (name+json) |
| citations | jsonb | [{document_id,title,chunk_text,score}] |
| created_at | timestamptz NOT NULL default now() | |

Index: `(conversation_id, created_at)`.

### products
| column | type | notes |
|---|---|---|
| id | uuid PK | |
| name | text NOT NULL | |
| category | text | |
| attributes | jsonb NOT NULL default '{}' | flexible per-type attributes |
| created_at / updated_at | timestamptz NOT NULL | |

### embedding_index_meta
| column | type | notes |
|---|---|---|
| id | boolean PK default true (single row) | |
| dimensions | integer NOT NULL | |
| model | text NOT NULL | |

Full DDL: `backend/migrations/0001_init.sql`.
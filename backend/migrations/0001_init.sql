-- Work Assistant initial schema.
-- NOTE: document_chunks.embedding (vector) and its HNSW index are created at
-- startup from EMBEDDING_DIMENSIONS (see docs/decisions.md D2).

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Fixed single user (auth placeholder, §1)
CREATE TABLE users (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
INSERT INTO users (id, name)
VALUES ('00000000-0000-0000-0000-000000000001', 'Local User')
ON CONFLICT (id) DO NOTHING;

CREATE TABLE projects (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name        text NOT NULL UNIQUE,
    description text,
    color       text NOT NULL DEFAULT '#4f46e5',
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tags (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name        text NOT NULL UNIQUE,
    color       text NOT NULL DEFAULT '#6b7280',
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tasks (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    title            text NOT NULL,
    description      text,
    status           text NOT NULL DEFAULT 'todo'
                       CHECK (status IN ('todo','in_progress','done','cancelled')),
    priority         text NOT NULL DEFAULT 'medium'
                       CHECK (priority IN ('low','medium','high','urgent')),
    due_date         timestamptz,
    scheduled_start  timestamptz,
    scheduled_end    timestamptz,
    recurrence_rule  text,
    project_id       uuid REFERENCES projects(id) ON DELETE SET NULL,
    parent_task_id   uuid REFERENCES tasks(id) ON DELETE CASCADE,
    completed_at     timestamptz,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX tasks_status_idx        ON tasks (status);
CREATE INDEX tasks_due_date_idx      ON tasks (due_date);
CREATE INDEX tasks_project_id_idx    ON tasks (project_id);
CREATE INDEX tasks_parent_task_idx   ON tasks (parent_task_id);
CREATE INDEX tasks_scheduled_start_idx ON tasks (scheduled_start);
CREATE INDEX tasks_search_idx ON tasks
    USING gin (to_tsvector('english', title || ' ' || coalesce(description, '')));

CREATE TABLE task_tags (
    task_id uuid NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    tag_id  uuid NOT NULL REFERENCES tags(id)  ON DELETE CASCADE,
    PRIMARY KEY (task_id, tag_id)
);

CREATE TABLE calendar_events (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    title            text NOT NULL,
    description      text,
    start_time       timestamptz NOT NULL,
    end_time         timestamptz NOT NULL,
    all_day          boolean NOT NULL DEFAULT false,
    recurrence_rule  text,
    reminder_minutes integer,
    source_task_id   uuid REFERENCES tasks(id) ON DELETE SET NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX calendar_events_start_idx ON calendar_events (start_time);
CREATE INDEX calendar_events_source_task_idx ON calendar_events (source_task_id);

CREATE TABLE documents (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    filename     text NOT NULL,
    title        text NOT NULL,
    source       text NOT NULL DEFAULT 'upload',
    doc_type     text NOT NULL,
    upload_date  timestamptz NOT NULL DEFAULT now(),
    content_hash text,
    status       text NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','indexing','indexed','failed')),
    error        text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX documents_status_idx   ON documents (status);
CREATE INDEX documents_doc_type_idx ON documents (doc_type);
CREATE INDEX documents_search_idx ON documents
    USING gin (to_tsvector('english', title || ' ' || filename));

CREATE TABLE document_extra (
    document_id uuid PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    raw_text    text NOT NULL
);

CREATE TABLE document_chunks (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id uuid NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index integer NOT NULL,
    chunk_text  text NOT NULL,
    token_count integer NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX document_chunks_document_idx ON document_chunks (document_id, chunk_index);

CREATE TABLE embedding_index_meta (
    id         boolean PRIMARY KEY DEFAULT true,
    dimensions integer NOT NULL,
    model      text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT embedding_index_meta_single_row CHECK (id)
);

CREATE TABLE conversations (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    title      text NOT NULL DEFAULT 'New chat',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE conversation_messages (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id uuid NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            text NOT NULL CHECK (role IN ('user','assistant','system','tool')),
    content         text,
    tool_calls      jsonb,
    tool_results    jsonb,
    citations       jsonb,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX conversation_messages_conv_idx ON conversation_messages (conversation_id, created_at);

CREATE TABLE products (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name       text NOT NULL,
    category   text,
    attributes jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX products_category_idx ON products (category);

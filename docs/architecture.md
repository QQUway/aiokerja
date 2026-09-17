# Architecture

Single-VPS, self-hostable personal work assistant. Three containers only:
`postgres` (pgvector), `backend` (Rust/Axum), `frontend` (React/Vite, served by
a static file server inside the container).

## Module boundaries (backend)

```
backend/src/
  main.rs            process bootstrap: config, tracing, pool, migrations, router
  config.rs          typed env config (Config::from_env)
  state.rs           AppState { pool, config, llm, embeddings }
  error.rs           AppError + IntoResponse (explicit JSON error envelope)
  auth.rs            CurrentUser extractor (no-op single fixed user, §1)
  routes/            HTTP layer only: parse/validate request -> call db/ai -> serialize
    health.rs  tasks.rs  projects.rs  tags.rs  events.rs
    documents.rs  chat.rs  search.rs  dashboard.rs
  domain/            plain typed structs + enums + validation (no SQL, no HTTP)
    task.rs  project.rs  tag.rs  event.rs  document.rs  chat.rs  product.rs
  db/                SQL access via sqlx runtime queries, returns domain types
    tasks.rs  projects.rs  tags.rs  events.rs  documents.rs  chat.rs  search.rs
  ai/
    llm.rs           LlmProvider trait + OpenAiCompatibleProvider
    embeddings.rs    EmbeddingProvider trait + OpenAiCompatibleEmbeddings
    extract.rs       Extractor trait + Txt/Pdf/Docx impls, registry by extension
    chunk.rs         fixed-size token chunking with overlap
    rag.rs           retrieval + context construction + citation assembly
    tools.rs         JSON-schema tool definitions + dispatch to db layer
```

Dependency direction: `routes -> db -> domain`, `routes -> ai -> db`,
`ai -> domain`. `domain` depends on nothing but serde/chrono/uuid. No route
touches SQL; no db function touches HTTP; the LLM never touches SQL (it can only
call named tools in `ai/tools.rs`).

## Request flow (typical)

```
HTTP -> routes/<x>.rs handler
     -> domain validation (serde + From/TryFrom + explicit checks)
     -> db/<x>.rs (sqlx query_as -> domain struct)
     -> AppError on failure -> JSON {error:{code,message}}
```

Chat flow (Stage 4):

```
POST /api/chat/conversations/:id/messages
  -> persist user message
  -> load history
  -> loop (max N iterations):
       llm.chat(messages, tools)
       if tool_calls: dispatch each via ai/tools.rs -> db layer
                      append assistant(tool_calls) + tool(result) messages, persist
       else: break
  -> optionally RAG-ground: if user asked about knowledge, tools retrieve_knowledge
  -> persist assistant message
  -> return { message, citations[], tool_calls[] }
```

## LLM abstraction

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, AiError>;
    fn model(&self) -> &str;
}
```

`ChatRequest { messages, tools: Option<Vec<ToolDef>>, temperature, max_tokens }`.
`ChatResponse { content, tool_calls: Vec<ToolCall>, finish_reason }`.
One concrete impl `OpenAiCompatibleProvider` covers both `LLM_PROVIDER=openai`
and `openai_compatible`; only `base_url`/`api_key` differ. Anthropic/Gemini would
be added as new impls of the same trait — no call-site changes.

Embeddings mirror this: `EmbeddingProvider { embed(texts) -> Vec<Vec<f32>>, dimensions() }`.

## RAG pipeline -> code map

| Stage | Code |
|---|---|
| text extraction | `ai/extract.rs` (Extractor trait, registry keyed by file type) |
| chunking | `ai/chunk.rs` (`chunk_text(text, size, overlap)`) |
| embeddings | `ai/embeddings.rs` |
| pgvector storage | `db/documents.rs` (`insert_chunks`) |
| retrieval | `ai/rag.rs::retrieve` (cosine `<=>` via pgvector) |
| reranking (stub) | `ai/rag.rs::Reranker` trait, `NoopReranker` |
| context construction | `ai/rag.rs::build_context` (separate from system prompt) |
| generation | `ai/llm.rs` |
| citations | `domain/chat.rs::Citation`, returned in chat response + displayed in UI |

## Background work

Document ingestion runs in a `tokio::spawn` task after upload: status
`pending -> indexing -> indexed|failed`. No queue/worker infra (§9).

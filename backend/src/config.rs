use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub backend_port: u16,
    pub log_level: String,
    pub app_url: String,

    pub llm_provider: String,
    pub llm_api_key: String,
    pub llm_base_url: Option<String>,
    pub llm_model: String,

    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimensions: usize,

    pub rag_chunk_size_tokens: usize,
    pub rag_chunk_overlap_tokens: usize,
    pub rag_retrieval_top_k: usize,
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let cfg = Config {
            database_url: env_or(
                "DATABASE_URL",
                "postgres://user:password@localhost:5432/workassistant",
            ),
            backend_port: env_or("BACKEND_PORT", "8080").parse()?,
            log_level: env_or("LOG_LEVEL", "info"),
            app_url: env_or("APP_URL", "http://localhost:5173"),

            llm_provider: env_or("LLM_PROVIDER", "openai"),
            llm_api_key: env_or("LLM_API_KEY", ""),
            llm_base_url: env::var("LLM_BASE_URL").ok().filter(|s| !s.is_empty()),
            llm_model: env_or("LLM_MODEL", "gpt-4o-mini"),

            embedding_provider: env_or("EMBEDDING_PROVIDER", "openai"),
            embedding_model: env_or("EMBEDDING_MODEL", "text-embedding-3-small"),
            embedding_dimensions: env_or("EMBEDDING_DIMENSIONS", "1536").parse()?,

            rag_chunk_size_tokens: env_or("RAG_CHUNK_SIZE_TOKENS", "512").parse()?,
            rag_chunk_overlap_tokens: env_or("RAG_CHUNK_OVERLAP_TOKENS", "64").parse()?,
            rag_retrieval_top_k: env_or("RAG_RETRIEVAL_TOP_K", "6").parse()?,
        };
        Ok(cfg)
    }
}

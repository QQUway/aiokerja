use std::sync::Arc;

use sqlx::PgPool;

use crate::ai::embeddings::EmbeddingProvider;
use crate::ai::llm::LlmProvider;
use crate::ai::rag::Reranker;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub llm: Arc<dyn LlmProvider>,
    pub embeddings: Arc<dyn EmbeddingProvider>,
    pub reranker: Arc<dyn Reranker>,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        config: Arc<Config>,
        llm: Arc<dyn LlmProvider>,
        embeddings: Arc<dyn EmbeddingProvider>,
        reranker: Arc<dyn Reranker>,
    ) -> Self {
        Self {
            pool,
            config,
            llm,
            embeddings,
            reranker,
        }
    }
}

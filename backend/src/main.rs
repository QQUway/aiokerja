mod ai;
mod auth;
mod config;
mod db;
mod domain;
mod error;
mod routes;
mod state;

use std::sync::Arc;

use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use ai::embeddings::{EmbeddingProvider, OpenAiCompatibleEmbeddings};
use ai::llm::{LlmProvider, OpenAiCompatibleProvider};
use ai::rag::{NoopReranker, Reranker};
use config::Config;
use state::AppState;

/// Default base URL when `LLM_BASE_URL` is unset and the provider is [OI].
const OPENAI_DEFAULT_BASE: &str = "https://api.openai.com/v1";

fn provider_base_url(config: &Config) -> String {
    config
        .llm_base_url
        .clone()
        .unwrap_or_else(|| OPENAI_DEFAULT_BASE.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env()?);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(config.log_level.clone()));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    if config.llm_provider != "openai" && config.llm_provider != "openai_compatible" {
        tracing::warn!(
            provider = %config.llm_provider,
            "unknown LLM_PROVIDER; falling back to [OI]-compatible client"
        );
    }

    tracing::info!("connecting to database…");
    let pool = db::init_db(&config)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    // Providers. No network call happens here — requests are made lazily.
    let base_url = provider_base_url(&config);
    let llm: Arc<dyn LlmProvider> = Arc::new(OpenAiCompatibleProvider::new(
        base_url.clone(),
        config.llm_api_key.clone(),
        config.llm_model.clone(),
    ));
    let embeddings: Arc<dyn EmbeddingProvider> = Arc::new(OpenAiCompatibleEmbeddings::new(
        base_url,
        config.llm_api_key.clone(),
        config.embedding_model.clone(),
        config.embedding_dimensions,
    ));
    let reranker: Arc<dyn Reranker> = Arc::new(NoopReranker);

    // Documents left mid-ingestion by a previous process are retried.
    match db::documents::reset_stuck_indexing(&pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(count = n, "reset stuck 'indexing' documents to pending"),
        Err(e) => tracing::error!(error = %e, "failed to reset stuck indexing documents"),
    }

    let state = AppState::new(
        pool.clone(),
        config.clone(),
        llm,
        embeddings.clone(),
        reranker,
    );

    // Background ingestion of anything pending (docs/decisions.md D5).
    {
        let pool = pool.clone();
        let config = config.clone();
        let embeddings = embeddings.clone();
        tokio::spawn(async move {
            ai::ingest::process_pending(pool, config, embeddings).await;
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::api_router()
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.backend_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("backend listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

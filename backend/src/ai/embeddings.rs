use async_trait::async_trait;
use serde_json::json;

use crate::error::{AppError, AppResult};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Returns one embedding vector per input text, in order.
    async fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>>;
    fn model(&self) -> &str;
    fn dimensions(&self) -> usize;
}

pub struct OpenAiCompatibleEmbeddings {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    dimensions: usize,
}

impl OpenAiCompatibleEmbeddings {
    pub fn new(base_url: String, api_key: String, model: String, dimensions: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
            dimensions,
        }
    }

    fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/embeddings") {
            base.to_string()
        } else {
            format!("{base}/embeddings")
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatibleEmbeddings {
    async fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let body = json!({ "model": self.model, "input": texts, "dimensions": self.dimensions });

        let mut request = self.client.post(self.endpoint()).json(&body);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::ai(format!("embedding request failed: {e}")))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AppError::ai(format!("embedding response read failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::ai(format!(
                "embedding endpoint returned {status}: {}",
                crate::ai::llm::truncate(&text, 500)
            )));
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AppError::ai(format!("embedding endpoint returned invalid JSON: {e}")))?;

        let data = parsed
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| AppError::ai("embedding response missing data[]"))?;

        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let arr = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| AppError::ai("embedding item missing embedding[]"))?;
            let vector: Vec<f32> = arr
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            if vector.len() != self.dimensions {
                return Err(AppError::ai(format!(
                    "embedding dimension mismatch: provider returned {}, EMBEDDING_DIMENSIONS={}. \
                     Re-index required if you changed the model (see README).",
                    vector.len(),
                    self.dimensions
                )));
            }
            vectors.push(vector);
        }

        if vectors.len() != texts.len() {
            return Err(AppError::ai(format!(
                "embedding count mismatch: sent {}, received {}",
                texts.len(),
                vectors.len()
            )));
        }
        Ok(vectors)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

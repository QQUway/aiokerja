use serde::Serialize;
use uuid::Uuid;

use crate::ai::llm::{ChatMessage, ChatRequest};
use crate::ai::rag::{self, RetrievedChunk};
use crate::domain::chat::Citation;
use crate::domain::product::Product;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ComparisonRow {
    pub key: String,
    /// One value per product, in the same order as `products` (null if that
    /// product has no value for this key).
    pub values: Vec<Option<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct ComparisonResult {
    pub products: Vec<Product>,
    pub matrix: Vec<ComparisonRow>,
    pub summary: Option<String>,
    pub citations: Vec<Citation>,
}

fn matrix_as_text(products: &[Product], matrix: &[ComparisonRow]) -> String {
    let mut out = String::new();
    out.push_str("Spec matrix:\n");
    out.push_str("field");
    for p in products {
        out.push_str(" | ");
        out.push_str(&p.name);
    }
    out.push('\n');
    for row in matrix {
        out.push_str(&row.key);
        for value in &row.values {
            out.push_str(" | ");
            match value {
                Some(v) => out.push_str(&v.to_string()),
                None => out.push('-'),
            }
        }
        out.push('\n');
    }
    out
}

/// Build an aligned spec matrix from the union of each product's `attributes`
/// keys, then ground a natural-language comparison in the selected products'
/// source documents via the existing RAG pipeline (scoped retrieval).
pub async fn compare_products(
    state: &AppState,
    product_ids: &[Uuid],
    question: Option<&str>,
) -> AppResult<ComparisonResult> {
    if product_ids.len() < 2 {
        return Err(AppError::validation("compare requires at least 2 product_ids"));
    }

    let products = crate::db::products::get_by_ids(&state.pool, product_ids).await?;
    if products.len() != product_ids.len() {
        return Err(AppError::not_found("one or more product ids were not found"));
    }

    let mut keys: Vec<String> = products
        .iter()
        .filter_map(|p| p.attributes.as_object())
        .flat_map(|obj| obj.keys().cloned())
        .collect();
    keys.sort();
    keys.dedup();

    let matrix: Vec<ComparisonRow> = keys
        .into_iter()
        .map(|key| {
            let values = products
                .iter()
                .map(|p| p.attributes.get(&key).cloned())
                .collect();
            ComparisonRow { key, values }
        })
        .collect();

    let doc_ids: Vec<Uuid> = products.iter().filter_map(|p| p.source_document_id).collect();

    let (summary, citations) = if doc_ids.is_empty() {
        (None, vec![])
    } else {
        let query = question.map(String::from).unwrap_or_else(|| {
            format!(
                "Compare {}",
                products
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" vs ")
            )
        });

        let vectors = state.embeddings.embed(std::slice::from_ref(&query)).await?;
        let hits = crate::db::documents::retrieve_chunks_for_documents(
            &state.pool,
            &vectors[0],
            &doc_ids,
            12,
        )
        .await?;
        let chunks: Vec<RetrievedChunk> = hits
            .into_iter()
            .map(|h| RetrievedChunk {
                chunk_id: h.chunk_id,
                document_id: h.document_id,
                title: h.title,
                chunk_index: h.chunk_index,
                chunk_text: h.chunk_text,
                score: h.score as f32,
            })
            .filter(|c| c.score >= 0.30)
            .collect();

        let context = rag::build_context(&chunks);
        let citations = rag::citations_from(&chunks);

        let messages = vec![
            ChatMessage::system(
                "You compare AIDC hardware (RFID readers/antennas, handheld computers, \
                 barcode/RFID printers, scanners) using only the provided spec matrix and \
                 knowledge base excerpts. Be concise, call out concrete tradeoffs, and cite \
                 excerpts with [ref N].",
            ),
            ChatMessage::user(format!(
                "{}\n\n{}\n\nQuestion: {}",
                matrix_as_text(&products, &matrix),
                context,
                query
            )),
        ];
        let response = state.llm.chat(&ChatRequest::new(messages)).await?;
        (response.content, citations)
    };

    Ok(ComparisonResult {
        products,
        matrix,
        summary,
        citations,
    })
}

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::ai::llm::{ChatMessage, ChatRequest, LlmProvider};
use crate::domain::product::DEVICE_TYPES;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Per-category spec fields the extractor is told to look for (AIDC hardware:
/// RFID readers/antennas, handheld computers, barcode/RFID printers, scanners).
/// Not exhaustive — the LLM may add extra keys it finds in the text.
fn field_hints(device_type: &str) -> &'static str {
    match device_type {
        "rfid_reader" => {
            "frequency_band, air_protocol, read_range, antenna_ports, rf_output_power_dbm, \
             interfaces, ip_rating, power_supply, dimensions, weight"
        }
        "rfid_antenna" => {
            "frequency_band, gain_dbi, polarization, beamwidth, connector, ip_rating, \
             dimensions, weight"
        }
        "handheld_computer" => {
            "os, cpu, ram, storage, display, battery, scan_engine, barcode_support, \
             rfid_support, connectivity, ip_rating, drop_spec, dimensions, weight"
        }
        "barcode_printer" | "rfid_printer" => {
            "print_method, resolution_dpi, print_width, print_speed, media_width, \
             ribbon_support, connectivity, ip_rating, dimensions, weight"
        }
        "scanner" => {
            "symbologies, scan_engine, interface, decode_range, ip_rating, dimensions, weight"
        }
        _ => "any specifications relevant to this device",
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DatasheetExtraction {
    pub brand: Option<String>,
    pub model: Option<String>,
    pub device_type: String,
    pub attributes: Value,
}

#[derive(Debug, Deserialize)]
struct RawExtraction {
    #[serde(default)]
    brand: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    device_type: Option<String>,
    #[serde(default)]
    attributes: Value,
}

/// Strip a ```json ... ``` fence if the model wrapped its JSON in one.
fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        if let Some(end) = rest.rfind("```") {
            return rest[..end].trim();
        }
    }
    trimmed
}

pub async fn extract_datasheet(
    llm: &dyn LlmProvider,
    text: &str,
    device_type_hint: Option<&str>,
) -> AppResult<DatasheetExtraction> {
    let excerpt = crate::ai::llm::truncate(text, 16_000);
    let category_note = match device_type_hint {
        Some(hint) if DEVICE_TYPES.contains(&hint) => format!(
            "The device_type is known to be \"{hint}\". Extract these fields where present: {}.",
            field_hints(hint)
        ),
        _ => format!(
            "First infer the device_type (one of: {}) from the text, then extract the \
             fields typical for that category.",
            DEVICE_TYPES.join(", ")
        ),
    };

    let system = format!(
        "You extract structured specifications from AIDC hardware datasheets (RFID readers, \
         RFID antennas, handheld computers, barcode/RFID printers, barcode scanners — brands \
         like Zebra, Chainway, TSC, Honeywell). {category_note}\n\n\
         Respond with ONLY a JSON object, no prose, no markdown fence, matching this shape:\n\
         {{\"brand\": string|null, \"model\": string|null, \"device_type\": one of [{}], \
         \"attributes\": {{ ...key: value spec pairs, values as plain strings ... }}}}\n\
         Use null for brand/model if not found. Only include attributes you can actually \
         find in the text — do not invent values.",
        DEVICE_TYPES.join(", ")
    );

    let messages = vec![
        ChatMessage::system(system),
        ChatMessage::user(format!("Datasheet text:\n\n{excerpt}")),
    ];

    let response = llm.chat(&ChatRequest::new(messages)).await?;
    let content = response
        .content
        .ok_or_else(|| AppError::ai("datasheet extraction returned no content"))?;

    let json_text = strip_code_fence(&content);
    let raw: RawExtraction = serde_json::from_str(json_text).map_err(|e| {
        AppError::ai(format!(
            "datasheet extraction did not return valid JSON: {e} (got: {})",
            crate::ai::llm::truncate(json_text, 300)
        ))
    })?;

    let device_type = raw
        .device_type
        .filter(|d| DEVICE_TYPES.contains(&d.as_str()))
        .or_else(|| device_type_hint.map(String::from))
        .unwrap_or_else(|| "other".to_string());

    Ok(DatasheetExtraction {
        brand: raw.brand.filter(|s| !s.trim().is_empty()),
        model: raw.model.filter(|s| !s.trim().is_empty()),
        device_type,
        attributes: if raw.attributes.is_object() {
            raw.attributes
        } else {
            Value::Object(Default::default())
        },
    })
}

/// Fetch a document's extracted text, run the LLM extraction, and upsert the
/// resulting product. Shared by the HTTP route and the `extract_datasheet`
/// chat tool.
pub async fn extract_and_save(
    state: &AppState,
    document_id: Uuid,
    device_type_hint: Option<&str>,
) -> AppResult<crate::domain::product::Product> {
    let doc = crate::db::documents::get(&state.pool, document_id)
        .await?
        .ok_or_else(|| AppError::not_found("document not found"))?;
    let text = crate::db::documents::raw_text(&state.pool, document_id)
        .await?
        .ok_or_else(|| AppError::not_found("document has no extracted text yet"))?;

    let extraction = extract_datasheet(state.llm.as_ref(), &text, device_type_hint).await?;
    crate::db::products::upsert_from_datasheet(&state.pool, document_id, &doc.title, &extraction)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fence_removes_json_fence() {
        let wrapped = "```json\n{\"brand\": \"Zebra\"}\n```";
        assert_eq!(strip_code_fence(wrapped), "{\"brand\": \"Zebra\"}");
    }

    #[test]
    fn strip_code_fence_removes_plain_fence() {
        let wrapped = "```\n{\"brand\": \"TSC\"}\n```";
        assert_eq!(strip_code_fence(wrapped), "{\"brand\": \"TSC\"}");
    }

    #[test]
    fn strip_code_fence_passes_through_unfenced() {
        let plain = "{\"brand\": \"Honeywell\"}";
        assert_eq!(strip_code_fence(plain), plain);
    }

    #[test]
    fn field_hints_cover_every_device_type() {
        for device_type in DEVICE_TYPES {
            assert!(!field_hints(device_type).is_empty());
        }
    }
}

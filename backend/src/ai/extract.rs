use crate::error::{AppError, AppResult};

/// Text extraction is a trait keyed by file type so new formats are added by
/// implementing `extract` and registering the extension (docs/architecture.md).
pub trait Extractor: Send + Sync {
    fn extension(&self) -> &'static str;
    fn extract(&self, bytes: &[u8]) -> AppResult<String>;
}

pub struct TxtExtractor;
impl Extractor for TxtExtractor {
    fn extension(&self) -> &'static str {
        "txt"
    }
    fn extract(&self, bytes: &[u8]) -> AppResult<String> {
        Ok(String::from_utf8_lossy(bytes).to_string())
    }
}

pub struct MarkdownExtractor;
impl Extractor for MarkdownExtractor {
    fn extension(&self) -> &'static str {
        "md"
    }
    fn extract(&self, bytes: &[u8]) -> AppResult<String> {
        Ok(String::from_utf8_lossy(bytes).to_string())
    }
}

pub struct PdfExtractor;
impl Extractor for PdfExtractor {
    fn extension(&self) -> &'static str {
        "pdf"
    }
    fn extract(&self, bytes: &[u8]) -> AppResult<String> {
        let text = pdf_extract::extract_text_from_mem(bytes)
            .map_err(|e| AppError::bad_request(format!("PDF parse failed: {e}")))?;
        Ok(text)
    }
}

pub struct DocxExtractor;
impl Extractor for DocxExtractor {
    fn extension(&self) -> &'static str {
        "docx"
    }
    fn extract(&self, bytes: &[u8]) -> AppResult<String> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::bad_request(format!("DOCX open failed: {e}")))?;
        let mut document = archive
            .by_name("word/document.xml")
            .map_err(|e| AppError::bad_request(format!("DOCX has no word/document.xml: {e}")))?;
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut document, &mut xml)
            .map_err(|e| AppError::bad_request(format!("DOCX read failed: {e}")))?;
        Ok(docx_text(&xml))
    }
}

/// Pull plain text out of `word/document.xml` (all `<w:t>` runs joined with
/// newlines at paragraph boundaries).
fn docx_text(xml: &str) -> String {
    let mut out = String::new();
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) if e.name().as_ref() == b"w:p" => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(quick_xml::events::Event::Start(ref e)) if e.name().as_ref() == b"w:tab" => {
                out.push('\t');
            }
            Ok(quick_xml::events::Event::Text(ref t)) => {
                if let Ok(text) = t.unescape() {
                    // Skip whitespace-only text nodes (inter-element indentation);
                    // keep real run text as-is so run-boundary spaces survive.
                    if !text.trim().is_empty() {
                        out.push_str(&text);
                    }
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) if e.name().as_ref() == b"w:p" => {
                out.push('\n');
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    // Normalize duplicate blank lines caused by per-run paragraph pushes.
    let lines: Vec<&str> = out
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

fn registry() -> Vec<Box<dyn Extractor>> {
    vec![
        Box::new(TxtExtractor),
        Box::new(MarkdownExtractor),
        Box::new(PdfExtractor),
        Box::new(DocxExtractor),
    ]
}

/// Dispatch by file extension.
pub fn extract_text(filename: &str, bytes: &[u8]) -> AppResult<String> {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    for extractor in registry() {
        if extractor.extension() == ext {
            let text = extractor.extract(bytes)?;
            if text.trim().is_empty() {
                return Err(AppError::bad_request(
                    "no extractable text found (is the file empty or image-only?)",
                ));
            }
            return Ok(text);
        }
    }
    Err(AppError::bad_request(format!(
        "unsupported file type '.{ext}' (supported: pdf, txt, md, docx)"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txt_extracts_utf8() {
        let text = extract_text("notes.txt", "hello wörld".as_bytes()).unwrap();
        assert_eq!(text, "hello wörld");
    }

    #[test]
    fn docx_pulls_runs() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t> world</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second para</w:t></w:r></w:p></w:body></w:document>"#;
        let text = docx_text(xml);
        assert!(text.contains("Hello world"));
        assert!(text.contains("Second para"));
    }

    #[test]
    fn rejects_unknown_ext() {
        let err = extract_text("file.exe", b"data").unwrap_err();
        assert!(err.to_string().contains("unsupported"));
    }
}

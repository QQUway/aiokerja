//! Fixed-size token chunking with overlap (§5). Token counting is heuristic by
//! default (docs/decisions.md D4) — a real tokenizer can be swapped in behind
//! the same trait without touching the pipeline.

pub trait Tokenizer: Send + Sync {
    fn token_count(&self, text: &str) -> usize;
}

/// ~4 characters per token, rounded up. Good enough for chunk sizing; exact
/// counts are not required for retrieval quality.
pub struct HeuristicTokenizer;

impl Tokenizer for HeuristicTokenizer {
    fn token_count(&self, text: &str) -> usize {
        let chars = text.chars().count();
        if chars == 0 {
            0
        } else {
            chars.div_ceil(4)
        }
    }
}

/// How many trailing words of `words` fit within `token_budget`.
fn trailing_words_for_tokens(
    words: &[&str],
    token_budget: usize,
    tokenizer: &dyn Tokenizer,
) -> usize {
    if token_budget == 0 {
        return 0;
    }
    let mut count = 0usize;
    let mut used = 0usize;
    for word in words.iter().rev() {
        let next = used + tokenizer.token_count(word);
        if next > token_budget && count > 0 {
            break;
        }
        used = next;
        count += 1;
    }
    count
}

/// Split text into chunks of at most `chunk_size` tokens, each new chunk
/// starting `overlap` tokens before the previous chunk's end.
/// Returns (chunk_text, approx_token_count).
pub fn chunk_text(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    tokenizer: &dyn Tokenizer,
) -> Vec<(String, usize)> {
    if text.trim().is_empty() || chunk_size == 0 || overlap >= chunk_size {
        return vec![];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let mut chunks: Vec<(String, usize)> = Vec::new();
    let mut start = 0usize;

    while start < words.len() {
        // Greedily grow the window word by word until adding another word would
        // exceed `chunk_size` tokens.
        let mut window = String::new();
        let mut end = start;
        while end < words.len() {
            let candidate = if window.is_empty() {
                words[end].to_string()
            } else {
                format!("{window} {}", words[end])
            };
            let tokens = tokenizer.token_count(&candidate);
            if tokens > chunk_size && !window.is_empty() {
                break;
            }
            window = candidate;
            end += 1;
            if tokens > chunk_size {
                // Single word larger than the budget; emit it alone.
                break;
            }
        }

        if window.is_empty() {
            window = words[start].to_string();
            end = start + 1;
        }

        let count = tokenizer.token_count(&window);
        chunks.push((window, count));

        if end >= words.len() {
            break;
        }

        // Next chunk starts `overlap` tokens back from this window's end.
        let overlap_words = trailing_words_for_tokens(&words[start..end], overlap, tokenizer);
        let next_start = end.saturating_sub(overlap_words);
        start = if next_start > start { next_start } else { end };
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok() -> HeuristicTokenizer {
        HeuristicTokenizer
    }

    #[test]
    fn empty_input_yields_no_chunks() {
        assert!(chunk_text("", 512, 64, &tok()).is_empty());
        assert!(chunk_text("   \n  ", 512, 64, &tok()).is_empty());
    }

    #[test]
    fn short_text_single_chunk() {
        let chunks = chunk_text("the quick brown fox", 512, 64, &tok());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "the quick brown fox");
    }

    #[test]
    fn long_text_splits_with_overlap() {
        let text = (0..1000)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&text, 100, 20, &tok());
        assert!(chunks.len() > 1, "expected multiple chunks");
        // The next chunk starts `overlap` tokens back from the previous chunk's
        // end, so its opening words must reappear in the tail of chunk 0.
        let c0_words: Vec<&str> = chunks[0].0.split_whitespace().collect();
        let c1_words: Vec<&str> = chunks[1].0.split_whitespace().collect();
        let tail: Vec<&str> = c0_words[c0_words.len().saturating_sub(10)..].to_vec();
        assert!(
            c1_words.iter().take(3).all(|w| tail.contains(w)),
            "expected overlap: c1 head {:?} not found in c0 tail {:?}",
            &c1_words[..3.min(c1_words.len())],
            tail
        );
    }

    #[test]
    fn overlap_ge_chunk_size_is_rejected() {
        assert!(chunk_text("a b c", 512, 512, &tok()).is_empty());
    }

    #[test]
    fn all_chunks_respect_size_approximately() {
        let text = (0..2000)
            .map(|i| format!("word{i:04}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&text, 64, 8, &tok());
        assert!(!chunks.is_empty());
        for (chunk, count) in &chunks {
            assert!(*count <= 96, "chunk too large: {count} tokens in {chunk:?}");
        }
    }

    #[test]
    fn chunks_cover_the_whole_text() {
        let text = (0..500)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&text, 32, 4, &tok());
        let joined = chunks
            .iter()
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        for word in text.split_whitespace() {
            assert!(joined.contains(word), "missing word {word}");
        }
    }
}

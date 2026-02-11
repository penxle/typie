use crate::model::{Doc, NodeId};

#[derive(Clone, Debug, Default)]
pub struct SearchQuery {
    pub text: String,
    pub match_whole_word: bool,
}

impl SearchQuery {
    pub fn new(text: String, match_whole_word: bool) -> Self {
        Self {
            text,
            match_whole_word,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub node_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
}

pub fn perform_search(doc: &Doc, query: &SearchQuery) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut matches: Vec<SearchMatch> = Vec::new();
    let query_lower = query.text.to_lowercase();
    let query_char_len = query_lower.chars().count();
    let match_whole_word = query.match_whole_word;

    for (block_id, block_text) in doc.iter_blocks() {
        let block_text_lower = block_text.to_lowercase();
        let block_chars: Vec<char> = block_text_lower.chars().collect();

        if block_chars.len() < query_char_len {
            continue;
        }

        let mut char_offset = 0;
        while char_offset + query_char_len <= block_chars.len() {
            let candidate: String = block_chars[char_offset..char_offset + query_char_len]
                .iter()
                .collect();

            if candidate == query_lower {
                let is_match = if match_whole_word {
                    is_word_boundary(&block_chars, char_offset, query_char_len)
                } else {
                    true
                };

                if is_match {
                    matches.push(SearchMatch {
                        node_id: block_id,
                        start_offset: char_offset,
                        end_offset: char_offset + query_char_len,
                    });
                }
            }

            char_offset += 1;
        }
    }

    matches
}

fn is_word_boundary(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word_char(chars[start - 1]);
    let after_ok = start + len >= chars.len() || !is_word_char(chars[start + len]);
    before_ok && after_ok
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

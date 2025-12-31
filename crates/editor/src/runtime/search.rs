use crate::layout::Page;
use crate::model::{Doc, Node, NodeId};
use crate::runtime::cmd::SearchOverlay;

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

#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub query: SearchQuery,
    pub matches: Vec<SearchMatch>,
    pub current_index: usize,
}

impl SearchState {
    pub fn clear(&mut self) {
        self.query = SearchQuery::default();
        self.matches.clear();
        self.current_index = 0;
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_index)
    }

    pub fn total_count(&self) -> usize {
        self.matches.len()
    }

    pub fn move_to_next(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = (self.current_index + 1) % self.matches.len();
        }
    }

    pub fn move_to_previous(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.matches.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    pub fn refresh(&mut self, new_matches: Vec<SearchMatch>) {
        let current_ref = self.current_match().map(|m| (m.node_id, m.start_offset));

        self.matches = new_matches;

        if self.matches.is_empty() {
            self.current_index = 0;
            return;
        }

        if let Some((prev_node_id, prev_offset)) = current_ref {
            if let Some(idx) = self
                .matches
                .iter()
                .position(|m| m.node_id == prev_node_id && m.start_offset == prev_offset)
            {
                self.current_index = idx;
                return;
            }

            if let Some(idx) = self
                .matches
                .iter()
                .position(|m| m.node_id == prev_node_id && m.start_offset >= prev_offset)
            {
                self.current_index = idx;
                return;
            }

            self.current_index = self.current_index.min(self.matches.len() - 1);
        } else {
            self.current_index = 0;
        }
    }
}

pub fn perform_search(doc: &Doc, query: &SearchQuery) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let query_lower = query.text.to_lowercase();

    collect_matches_from_root(doc, &query_lower, query.match_whole_word, &mut matches);

    matches
}

fn collect_matches_from_root(
    doc: &Doc,
    query_lower: &str,
    match_whole_word: bool,
    matches: &mut Vec<SearchMatch>,
) {
    let Some(root) = doc.node(NodeId::ROOT) else {
        return;
    };

    for child in root.children() {
        collect_matches_from_block(doc, child.node_id(), query_lower, match_whole_word, matches);
    }
}

fn collect_matches_from_block(
    doc: &Doc,
    block_id: NodeId,
    query_lower: &str,
    match_whole_word: bool,
    matches: &mut Vec<SearchMatch>,
) {
    let Some(block_node) = doc.node(block_id) else {
        return;
    };

    let block_text = doc.get_block_text(block_id);
    let block_text_lower = block_text.to_lowercase();

    let query_chars: Vec<char> = query_lower.chars().collect();
    let query_char_len = query_chars.len();
    let block_chars: Vec<char> = block_text_lower.chars().collect();

    let mut char_offset = 0;
    while char_offset + query_char_len <= block_chars.len() {
        if block_chars[char_offset..char_offset + query_char_len] == query_chars[..] {
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

    for child in block_node.children() {
        let child_node = child.node();
        if !matches!(child_node, Node::Text(_)) {
            collect_matches_from_block(
                doc,
                child.node_id(),
                query_lower,
                match_whole_word,
                matches,
            );
        }
    }
}

fn is_word_boundary(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word_char(chars[start - 1]);
    let after_ok = start + len >= chars.len() || !is_word_char(chars[start + len]);
    before_ok && after_ok
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

pub fn build_search_overlays(
    pages: &[Page],
    matches: &[SearchMatch],
    current_index: usize,
) -> Vec<SearchOverlay> {
    let mut overlays = Vec::new();

    for (match_idx, search_match) in matches.iter().enumerate() {
        for (page_idx, page) in pages.iter().enumerate() {
            let bounds = page.get_text_range_bounds(
                search_match.node_id,
                search_match.start_offset,
                search_match.end_offset,
            );

            if !bounds.is_empty() {
                overlays.push(SearchOverlay {
                    page_idx,
                    bounds,
                    is_current: match_idx == current_index,
                });
                break;
            }
        }
    }

    overlays
}

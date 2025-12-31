use super::super::{Effect, Runtime};
use crate::runtime::search::{SearchQuery, perform_search};

impl Runtime {
    pub(crate) fn handle_search(&mut self, query: String, match_whole_word: bool) -> Vec<Effect> {
        let search_query = SearchQuery::new(query, match_whole_word);

        if search_query.is_empty() {
            return self.handle_clear_search();
        }

        let matches = perform_search(&self.state.doc, &search_query);

        self.search_state.query = search_query;
        self.search_state.matches = matches;
        self.search_state.current_index = 0;

        vec![Effect::SearchStateChanged]
    }

    pub(crate) fn handle_clear_search(&mut self) -> Vec<Effect> {
        if !self.search_state.query.is_empty() || !self.search_state.matches.is_empty() {
            self.search_state.clear();
            vec![Effect::SearchStateChanged]
        } else {
            vec![]
        }
    }

    pub(crate) fn handle_find_next(&mut self) -> Vec<Effect> {
        if self.search_state.matches.is_empty() {
            return vec![];
        }

        self.search_state.move_to_next();

        vec![Effect::SearchStateChanged]
    }

    pub(crate) fn handle_find_previous(&mut self) -> Vec<Effect> {
        if self.search_state.matches.is_empty() {
            return vec![];
        }

        self.search_state.move_to_previous();

        vec![Effect::SearchStateChanged]
    }

    pub(crate) fn handle_replace(&mut self, replacement: String) -> Vec<Effect> {
        let Some(current_match) = self.search_state.current_match().cloned() else {
            return vec![];
        };

        if self
            .replace_text_in_block(
                current_match.node_id,
                current_match.start_offset,
                current_match.end_offset,
                &replacement,
            )
            .is_err()
        {
            return vec![];
        }

        let matches = perform_search(&self.state.doc, &self.search_state.query);
        let old_index = self.search_state.current_index;
        self.search_state.matches = matches;

        if !self.search_state.matches.is_empty() {
            self.search_state.current_index = old_index % self.search_state.matches.len();
        } else {
            self.search_state.current_index = 0;
        }

        vec![Effect::SearchStateChanged]
    }

    pub(crate) fn handle_replace_all(&mut self, replacement: String) -> Vec<Effect> {
        if self.search_state.matches.is_empty() {
            return vec![];
        }

        let mut sorted_matches = self.search_state.matches.clone();
        sorted_matches.sort_by(|a, b| match a.node_id.cmp(&b.node_id) {
            std::cmp::Ordering::Equal => b.start_offset.cmp(&a.start_offset),
            other => other,
        });

        for search_match in sorted_matches {
            let _ = self.replace_text_in_block(
                search_match.node_id,
                search_match.start_offset,
                search_match.end_offset,
                &replacement,
            );
        }

        let matches = perform_search(&self.state.doc, &self.search_state.query);
        self.search_state.matches = matches;
        self.search_state.current_index = 0;

        vec![Effect::SearchStateChanged]
    }
}

use crate::icu_data::get_general_category_map;
use crate::model::{
    CONTINUOUS_PAGE_MARGIN, Doc, LayoutMode, Node, NodeId, ParagraphNode, TextMapping,
};
use crate::render::backend::RenderBackend;
use crate::runtime::search::{SearchQuery, perform_search};
use crate::runtime::{Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;
use icu_properties::props::GeneralCategory;
use serde::Serialize;
use std::rc::Rc;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ClipboardData {
    pub html: String,
    pub text: String,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[cfg_attr(feature = "native", repr(C))]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatchResult {
    pub node_id: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextWithMappingsResult {
    pub text: String,
    pub mappings: Vec<TextMapping>,
}

pub struct EditorCore {
    runtime: Runtime,
}

impl EditorCore {
    pub fn new(scale_factor: f64, backend: RenderBackend) -> Self {
        let doc = Rc::new(Doc::new());
        let width = Self::get_width(&doc);

        let root = doc
            .node(NodeId::ROOT)
            .expect("Doc::new: ROOT node must exist after construction");
        let paragraph_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .expect("Doc::new: failed to insert initial paragraph");

        let state = State::new(
            doc,
            Selection::collapsed(Position::new(paragraph_id, 0, Affinity::default())),
        );
        let mut runtime = Runtime::with_backend(width, scale_factor, state, backend);
        runtime.layout();
        Self { runtime }
    }

    pub fn with_snapshot(
        scale_factor: f64,
        snapshot: Vec<u8>,
        backend: RenderBackend,
        initial_position: Option<Position>,
    ) -> Result<Self, String> {
        let doc = Rc::new(Doc::from_snapshot(snapshot).map_err(|e| e.to_string())?);
        let width = Self::get_width(&doc);
        let pos = initial_position.unwrap_or(Position::new(NodeId::ROOT, 0, Affinity::default()));
        let state = State::new(doc, Selection::collapsed(pos));
        let mut runtime = Runtime::with_backend(width, scale_factor, state, backend);
        runtime.layout();
        Ok(Self { runtime })
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    fn get_width(doc: &Doc) -> f32 {
        match doc.settings().layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width + 2.0 * CONTINUOUS_PAGE_MARGIN,
        }
    }

    pub fn get_character_counts(&mut self) -> CharacterCounts {
        let doc_text = self.runtime.get_cached_plain_text();
        let selection_text = {
            let state = self.runtime.state();
            state.selection.to_plain_text(&state.doc)
        };

        let doc_counts = count_all(&doc_text);
        let sel_counts = count_all(&selection_text);

        CharacterCounts {
            doc_with_whitespace: doc_counts.0,
            doc_without_whitespace: doc_counts.1,
            doc_without_whitespace_and_punctuation: doc_counts.2,
            selection_with_whitespace: sel_counts.0,
            selection_without_whitespace: sel_counts.1,
            selection_without_whitespace_and_punctuation: sel_counts.2,
        }
    }

    pub fn get_clipboard_data(&self) -> Option<ClipboardData> {
        let state = self.runtime.state();
        if state.selection.is_collapsed() {
            return None;
        }

        let fragment = state.selection.extract_fragment(&state.doc).ok()?;
        if fragment.is_empty() {
            return None;
        }

        let html = fragment.to_html();
        let text = fragment.to_plain_text();
        Some(ClipboardData { html, text })
    }

    pub fn get_text_with_mappings(&self) -> TextWithMappingsResult {
        let (text, mappings) = self.runtime.doc().to_text_with_mappings();
        TextWithMappingsResult { text, mappings }
    }

    pub fn perform_search(&self, query: &str, match_whole_word: bool) -> Vec<SearchMatchResult> {
        let search_query = SearchQuery::new(query.to_string(), match_whole_word);
        let matches = perform_search(self.runtime.doc(), &search_query);
        matches
            .into_iter()
            .map(|m| SearchMatchResult {
                node_id: m.node_id.to_string(),
                start_offset: m.start_offset,
                end_offset: m.end_offset,
            })
            .collect()
    }
}

pub fn count_all(text: &str) -> (u32, u32, u32) {
    let Some(gc_data) = get_general_category_map() else {
        return (0, 0, 0);
    };
    let gc_map = gc_data.as_borrowed();

    let mut with_ws: u32 = 0;
    let mut without_ws: u32 = 0;
    let mut without_ws_punct: u32 = 0;
    let mut prev_whitespace = false;

    for c in text.chars() {
        if c == '\u{200B}' {
            continue;
        }

        if c.is_whitespace() {
            if !prev_whitespace {
                with_ws += 1;
            }
            prev_whitespace = true;
        } else {
            with_ws += 1;
            without_ws += 1;
            prev_whitespace = false;

            let gc = gc_map.get(c);
            if !matches!(
                gc,
                GeneralCategory::ConnectorPunctuation
                    | GeneralCategory::DashPunctuation
                    | GeneralCategory::ClosePunctuation
                    | GeneralCategory::FinalPunctuation
                    | GeneralCategory::InitialPunctuation
                    | GeneralCategory::OtherPunctuation
                    | GeneralCategory::OpenPunctuation
            ) {
                without_ws_punct += 1;
            }
        }
    }

    let first_non_ws = text
        .chars()
        .find(|&c| c != '\u{200B}' && !c.is_whitespace());

    if first_non_ws.is_none() {
        return (0, without_ws, without_ws_punct);
    }

    let starts_with_ws = text
        .chars()
        .find(|&c| c != '\u{200B}')
        .map_or(false, |c| c.is_whitespace());
    let ends_with_ws = text
        .chars()
        .rev()
        .find(|&c| c != '\u{200B}')
        .map_or(false, |c| c.is_whitespace());

    if starts_with_ws && with_ws > 0 {
        with_ws = with_ws.saturating_sub(1);
    }
    if ends_with_ws && with_ws > 0 {
        with_ws = with_ws.saturating_sub(1);
    }

    (with_ws, without_ws, without_ws_punct)
}

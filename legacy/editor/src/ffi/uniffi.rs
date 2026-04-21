use std::sync::{Arc, Mutex};

use crate::global::set_text_replacement_rules;
use crate::global::{add_font_base, add_font_chunk, set_available_fonts};
use crate::icu_data::{get_general_category_map, load_icu_data};
use crate::layout::query::{is_cursor_hit, is_selection_hit};
use crate::model::{
    CONTINUOUS_PAGE_MARGIN, Doc, DocExportMode, LayoutMode, Node, NodeId, ParagraphNode,
    TextMapping,
};
use crate::runtime::search::{SearchQuery, perform_search};
use crate::runtime::slate::get_slate_offsets;
use crate::runtime::text_replacement::RawTextReplacementRule;
use crate::runtime::tracked_items::RawTrackedItem;
use crate::runtime::{Message, Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;
use icu_properties::props::GeneralCategory;

// ── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, uniffi::Error)]
pub enum EditorError {
    General { msg: String },
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General { msg } => write!(f, "{msg}"),
        }
    }
}

impl From<String> for EditorError {
    fn from(msg: String) -> Self {
        Self::General { msg }
    }
}

impl From<anyhow::Error> for EditorError {
    fn from(e: anyhow::Error) -> Self {
        Self::General { msg: e.to_string() }
    }
}

// ── Records ─────────────────────────────────────────────────────────────────

#[derive(uniffi::Record)]
pub struct PageRenderInfo {
    pub width: u32,
    pub height: u32,
    pub buffer_size: u64,
}

#[derive(uniffi::Record)]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[derive(uniffi::Record)]
pub struct DragImageData {
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
    pub pixels: Vec<u8>,
}

// ── EditorEngine (Application-level) ────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct EditorEngine;

#[uniffi::export]
impl EditorEngine {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn load_icu_data(&self, data: Vec<u8>) -> Result<(), EditorError> {
        load_icu_data(&data).map_err(EditorError::from)
    }

    pub fn create_editor(
        &self,
        scale_factor: f64,
        snapshot: Option<Vec<u8>>,
    ) -> Result<Arc<Editor>, EditorError> {
        let inner = match snapshot {
            Some(data) if !data.is_empty() => EditorInner::with_snapshot(scale_factor, data)?,
            _ => EditorInner::new(scale_factor),
        };
        Ok(Arc::new(Editor {
            inner: Mutex::new(inner),
        }))
    }

    pub fn validate_regex(&self, pattern: String) -> bool {
        let anchored = format!("(?:{pattern})$");
        fancy_regex::Regex::new(&anchored).is_ok()
    }

    pub fn add_font_base(
        &self,
        family: String,
        weight: u16,
        data: Vec<u8>,
    ) -> Result<(), EditorError> {
        add_font_base(&family, weight, &data);
        Ok(())
    }

    pub fn add_font_chunk(
        &self,
        family: String,
        weight: u16,
        data: Vec<u8>,
    ) -> Result<(), EditorError> {
        add_font_chunk(&family, weight, &data);
        Ok(())
    }

    pub fn set_available_fonts(&self, fonts_json: String) -> Result<(), EditorError> {
        let fonts: std::collections::HashMap<String, Vec<u16>> =
            serde_json::from_str(&fonts_json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
        set_available_fonts(fonts);
        Ok(())
    }

    pub fn set_text_replacement_rules(&self, rules_json: String) -> Result<(), EditorError> {
        let rules: Vec<RawTextReplacementRule> =
            serde_json::from_str(&rules_json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
        set_text_replacement_rules(rules);
        Ok(())
    }

    pub fn get_slate_offsets(&self) -> Result<String, EditorError> {
        let offsets = get_slate_offsets();
        serde_json::to_string(&offsets).map_err(|e| format!("Failed to serialize: {e}").into())
    }
}

// ── EditorInner ─────────────────────────────────────────────────────────────

struct EditorInner {
    runtime: Runtime,
}

impl EditorInner {
    fn new(scale_factor: f64) -> Self {
        let doc = std::rc::Rc::new(Doc::new());
        let width = Self::get_width(&doc);

        let root = doc
            .node(NodeId::ROOT)
            .expect("Doc::new: ROOT node must exist after construction");
        let paragraph_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .expect("Doc::new: failed to insert initial paragraph");

        Self::create(scale_factor, doc, paragraph_id, width)
    }

    fn with_snapshot(scale_factor: f64, snapshot: Vec<u8>) -> Result<Self, EditorError> {
        let doc = std::rc::Rc::new(Doc::from_snapshot(snapshot).map_err(|e| e.to_string())?);
        let width = Self::get_width(&doc);
        Ok(Self::create(scale_factor, doc, NodeId::ROOT, width))
    }

    fn create(scale_factor: f64, doc: std::rc::Rc<Doc>, cursor_node: NodeId, width: f32) -> Self {
        let state = State::new(
            doc,
            Selection::collapsed(Position::new(cursor_node, 0, Affinity::default())),
        );
        let mut runtime = Runtime::new(width, scale_factor, state);
        runtime.layout();
        Self { runtime }
    }

    fn get_width(doc: &Doc) -> f32 {
        match doc.settings().layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width + 2.0 * CONTINUOUS_PAGE_MARGIN,
        }
    }
}

// ── Editor ──────────────────────────────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct Editor {
    inner: Mutex<EditorInner>,
}

// Safety: Mutex로 내부적으로 동기화하므로 Send+Sync을 보장한다.
// EditorInner 자체는 Send+Sync이 아니지만 (Rc<Doc>),
// Mutex를 통해 단일 접근만 허용하므로 안전하다.
// EditorInner는 반드시 Mutex를 통해서만 접근해야 한다.
unsafe impl Send for Editor {}
unsafe impl Sync for Editor {}

#[uniffi::export]
impl Editor {
    // ── Core ────────────────────────────────────────────────────────────

    pub fn dispatch(&self, message_json: String) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let message: Message = serde_json::from_str(&message_json)
            .map_err(|e| format!("Failed to parse message: {e}"))?;
        inner.runtime.enqueue_message(message);
        Ok(())
    }

    pub fn tick(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.tick();
        Ok(())
    }

    pub fn flush(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.flush();
        Ok(())
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn get_page_count(&self) -> Result<u32, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner.runtime.pages().len() as u32)
    }

    pub fn get_render_info(&self, page_index: u32) -> Result<Option<PageRenderInfo>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner
            .runtime
            .get_render_info(page_index as usize)
            .map(|info| PageRenderInfo {
                width: info.width as u32,
                height: info.height as u32,
                buffer_size: info.buffer_size as u64,
            }))
    }

    pub fn render_drag_image(
        &self,
        visible_pages: Vec<u32>,
        page_idx: u32,
    ) -> Result<Option<DragImageData>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let visible: Vec<usize> = visible_pages.into_iter().map(|p| p as usize).collect();
        Ok(inner
            .runtime
            .render_drag_image(&visible, page_idx as usize)
            .map(|result| {
                let data = unsafe { std::slice::from_raw_parts(result.ptr(), result.len()) };
                DragImageData {
                    width: result.width as u32,
                    height: result.height as u32,
                    offset_x: result.offset_x,
                    offset_y: result.offset_y,
                    scale_factor: result.scale_factor,
                    pixels: data.to_vec(),
                }
            }))
    }

    // ── Hit Testing ─────────────────────────────────────────────────────

    pub fn is_selection_hit(&self, page_idx: u32, x: f32, y: f32) -> Result<bool, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner
            .runtime
            .pages()
            .get(page_idx as usize)
            .map_or(false, |page| {
                is_selection_hit(inner.runtime.doc(), page, inner.runtime.selection(), x, y)
            }))
    }

    pub fn is_cursor_hit(&self, page_idx: u32, x: f32, y: f32) -> Result<bool, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner
            .runtime
            .pages()
            .get(page_idx as usize)
            .map_or(false, |page| {
                is_cursor_hit(inner.runtime.doc(), page, inner.runtime.selection(), x, y)
            }))
    }

    // ── Export / Import ─────────────────────────────────────────────────

    /// mode: 0=Snapshot, 1=Version, 2=AllUpdates, 3=UpdatesFrom
    pub fn export(&self, mode: i32, version: Option<Vec<u8>>) -> Result<Vec<u8>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let export_mode = match mode {
            0 => DocExportMode::Snapshot,
            1 => DocExportMode::Version,
            2 => DocExportMode::AllUpdates,
            3 => {
                let ver = version.ok_or_else(|| EditorError::General {
                    msg: "version is required for UpdatesFrom mode".into(),
                })?;
                DocExportMode::UpdatesFrom { version: ver }
            }
            _ => {
                return Err(EditorError::General {
                    msg: format!("Invalid export mode: {mode}"),
                });
            }
        };
        inner.runtime.export(export_mode).map_err(EditorError::from)
    }

    pub fn import_updates(&self, data: Vec<u8>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .runtime
            .import_updates(&data)
            .map_err(|e| format!("Failed to import updates: {e}"))?;
        Ok(())
    }

    pub fn import_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .runtime
            .import_updates_batch(&updates)
            .map_err(|e| format!("Failed to import updates batch: {e}"))?;
        Ok(())
    }

    // ── Text & Clipboard ────────────────────────────────────────────────

    pub fn get_character_counts(&self) -> Result<CharacterCounts, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let doc_text = inner.runtime.get_cached_plain_text();
        let selection_text = {
            let state = inner.runtime.state();
            state.selection.to_plain_text(&state.doc)
        };

        let doc_counts = count_all(&doc_text);
        let sel_counts = count_all(&selection_text);

        Ok(CharacterCounts {
            doc_with_whitespace: doc_counts.0,
            doc_without_whitespace: doc_counts.1,
            doc_without_whitespace_and_punctuation: doc_counts.2,
            selection_with_whitespace: sel_counts.0,
            selection_without_whitespace: sel_counts.1,
            selection_without_whitespace_and_punctuation: sel_counts.2,
        })
    }

    pub fn get_clipboard_data(&self) -> Result<Option<String>, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = inner.runtime.state();
        if state.selection.is_collapsed() {
            return Ok(None);
        }

        let fragment = match state.selection.extract_fragment(&state.doc) {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        if fragment.is_empty() {
            return Ok(None);
        }

        let html = fragment.to_html();
        let text = fragment.to_plain_text();

        let json = serde_json::json!({
            "html": html,
            "text": text,
        });
        let json_str =
            serde_json::to_string(&json).map_err(|e| format!("Failed to serialize: {e}"))?;
        Ok(Some(json_str))
    }

    pub fn get_text_with_mappings(&self) -> Result<String, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct TextWithMappingsResult {
            text: String,
            mappings: Vec<TextMapping>,
        }

        let (text, mappings) = inner.runtime.doc().to_text_with_mappings();
        let result = TextWithMappingsResult { text, mappings };
        serde_json::to_string(&result).map_err(|e| format!("Failed to serialize: {e}").into())
    }

    // ── Search ──────────────────────────────────────────────────────────

    pub fn perform_search(
        &self,
        query: String,
        match_whole_word: bool,
    ) -> Result<String, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let search_query = SearchQuery::new(query, match_whole_word);
        let matches = perform_search(inner.runtime.doc(), &search_query);

        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MatchResult {
            node_id: String,
            start_offset: usize,
            end_offset: usize,
        }

        let results: Vec<MatchResult> = matches
            .into_iter()
            .map(|m| MatchResult {
                node_id: m.node_id.to_string(),
                start_offset: m.start_offset,
                end_offset: m.end_offset,
            })
            .collect();

        serde_json::to_string(&results).map_err(|e| format!("Failed to serialize: {e}").into())
    }

    // ── Tracked Items ───────────────────────────────────────────────────

    pub fn set_tracked_items(&self, group: u32, items_json: String) -> Result<(), EditorError> {
        let items: Vec<RawTrackedItem> =
            serde_json::from_str(&items_json).map_err(|e| format!("Failed to parse items: {e}"))?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.set_tracked_items(group, items);
        Ok(())
    }

    pub fn remove_tracked_items(&self, group: u32, ids_json: String) -> Result<(), EditorError> {
        let ids: Vec<String> =
            serde_json::from_str(&ids_json).map_err(|e| format!("Failed to parse ids: {e}"))?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.remove_tracked_items(group, &ids);
        Ok(())
    }

    pub fn reveal_tracked_item(&self, group: u32, id: String) -> Result<bool, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner.runtime.reveal_tracked_item(group, &id))
    }

    // ── Block Operations ────────────────────────────────────────────────

    pub fn replace_text_in_block(
        &self,
        block_id: String,
        start_offset: u32,
        end_offset: u32,
        replacement: String,
    ) -> Result<bool, EditorError> {
        let node_id = NodeId::from_string(&block_id).ok_or_else(|| EditorError::General {
            msg: "Invalid block_id".into(),
        })?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner
            .runtime
            .replace_text_in_block(
                node_id,
                start_offset as usize,
                end_offset as usize,
                &replacement,
            )
            .is_ok())
    }

    pub fn replace_text_in_blocks(&self, items_json: String) -> Result<(), EditorError> {
        let entries: Vec<(String, usize, usize, String)> =
            serde_json::from_str(&items_json).map_err(|e| format!("Failed to parse items: {e}"))?;

        let replacements: Vec<_> = entries
            .iter()
            .filter_map(|(node_id, start, end, replacement)| {
                NodeId::from_string(node_id).map(|id| (id, *start, *end, replacement.as_str()))
            })
            .collect();

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .runtime
            .replace_text_in_blocks(&replacements)
            .map_err(|e| format!("Failed to replace: {e}"))?;
        Ok(())
    }

    pub fn insert_template_fragment(&self, snapshot: Vec<u8>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .runtime
            .insert_template_fragment(snapshot)
            .map_err(|e| format!("Failed to insert template: {e}"))?;
        Ok(())
    }

    // ── Tracing ─────────────────────────────────────────────────────────

    pub fn set_tracing(&self, trace_id: String, parent_span_id: String) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let trace_id = opentelemetry::trace::TraceId::from_hex(&trace_id)
            .map_err(|e| format!("Invalid trace_id: {e}"))?;
        let parent_span_id = opentelemetry::trace::SpanId::from_hex(&parent_span_id)
            .map_err(|e| format!("Invalid parent_span_id: {e}"))?;
        inner.runtime.tracing.set_tracing(trace_id, parent_span_id);
        Ok(())
    }

    pub fn clear_tracing(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.runtime.tracing.clear_tracing();
        Ok(())
    }

    pub fn drain_traces(&self) -> Result<String, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let traces = inner.runtime.tracing.drain();
        serde_json::to_string(&traces).map_err(|e| format!("Failed to serialize: {e}").into())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn count_all(text: &str) -> (u32, u32, u32) {
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

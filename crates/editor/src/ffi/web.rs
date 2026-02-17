use crate::global::{add_font_base, add_font_chunk, set_available_fonts, set_fallback_fonts};
use crate::global::{
    clear_text_replacement_rules, set_auto_surround_enabled, set_text_replacement_rules,
};
use crate::icu_data::{get_general_category_map, load_icu_data};
use crate::layout::query::is_selection_hit;
use crate::model::{
    Doc, DocExportMode, DocumentJson, LayoutMode, Node, NodeId, ParagraphNode, TextMapping,
    json_to_snapshot, snapshot_to_json,
};
use crate::render::DragImageResult;
use crate::runtime::search::{SearchQuery, perform_search};
use crate::runtime::{Message, RawTextReplacementRule, RawTrackedItem, Runtime, State, slate};
use crate::state::{Position, Selection, leaf_block_end};
use crate::types::Affinity;
use icu_properties::props::GeneralCategory;
use serde::Serialize;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer).unwrap()
}

#[wasm_bindgen(js_name = getMemory)]
pub fn get_memory() -> JsValue {
    wasm_bindgen::memory()
}

#[wasm_bindgen(js_name = validateRegex)]
pub fn validate_regex(pattern: &str) -> bool {
    let anchored = format!("(?:{pattern})$");
    fancy_regex::Regex::new(&anchored).is_ok()
}

#[wasm_bindgen(js_name = getFontMetadata)]
pub fn get_font_metadata(data: Vec<u8>) -> Result<JsValue, JsValue> {
    let metadata = crate::font::get_font_metadata(&data).map_err(|e| JsValue::from_str(&e))?;
    Ok(to_js_value(&metadata))
}

#[wasm_bindgen(js_name = outlineTextToSvg)]
pub fn outline_text_to_svg(font_data: Vec<u8>, text: &str) -> Result<String, JsValue> {
    crate::font::outline_text_to_svg(&font_data, text).map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen(js_name = getFontCodepoints)]
pub fn get_font_codepoints(ttf_data: Vec<u8>) -> Result<JsValue, JsValue> {
    let codepoints = crate::font::encode::get_font_codepoints(&ttf_data)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_js_value(&codepoints))
}

#[wasm_bindgen(js_name = encodeFont)]
pub fn encode_font_wasm(
    ttf_data: Vec<u8>,
    chunk_codepoints_json: &str,
) -> Result<JsValue, JsValue> {
    let chunk_codepoints: Vec<Vec<u32>> = serde_json::from_str(chunk_codepoints_json)
        .map_err(|e| JsValue::from_str(&format!("chunk_codepoints parse: {e}")))?;
    let result = crate::font::encode::encode_font(&ttf_data, &chunk_codepoints)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_js_value(&result))
}

#[wasm_bindgen(js_name = snapshotToJson)]
pub fn snapshot_to_json_wasm(snapshot: Vec<u8>) -> Result<JsValue, JsValue> {
    let doc_json = snapshot_to_json(&snapshot).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(to_js_value(&doc_json))
}

#[wasm_bindgen(js_name = jsonToSnapshot)]
pub fn json_to_snapshot_wasm(json: JsValue) -> Result<Vec<u8>, JsValue> {
    let doc_json: DocumentJson =
        serde_wasm_bindgen::from_value(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    json_to_snapshot(&doc_json).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub struct Application;

#[wasm_bindgen]
impl Application {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self
    }

    #[wasm_bindgen(js_name = loadIcuData)]
    pub fn load_icu_data(&self, icu_data: Vec<u8>) -> Result<(), JsValue> {
        load_icu_data(&icu_data)
    }

    #[wasm_bindgen(js_name = addFontBase)]
    pub fn add_font_base(&self, family: &str, weight: u16, data: Vec<u8>) {
        add_font_base(family, weight, &data);
    }

    #[wasm_bindgen(js_name = addFontChunk)]
    pub fn add_font_chunk(&self, family: &str, weight: u16, data: Vec<u8>) {
        add_font_chunk(family, weight, &data);
    }

    #[wasm_bindgen(js_name = setAvailableFonts)]
    pub fn set_available_fonts(&self, fonts: JsValue) {
        if let Ok(fonts) =
            serde_wasm_bindgen::from_value::<std::collections::HashMap<String, Vec<u16>>>(fonts)
        {
            set_available_fonts(fonts);
        }
    }

    #[wasm_bindgen(js_name = setFallbackFonts)]
    pub fn set_fallback_fonts(&self, names: JsValue) {
        if let Ok(names) = serde_wasm_bindgen::from_value::<Vec<String>>(names) {
            let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            set_fallback_fonts(&name_refs);
        }
    }

    #[wasm_bindgen(js_name = setTextReplacementRules)]
    pub fn set_text_replacement_rules(&self, rules: JsValue) {
        if let Ok(raw_rules) = serde_wasm_bindgen::from_value::<Vec<RawTextReplacementRule>>(rules)
        {
            set_text_replacement_rules(raw_rules);
        }
    }

    #[wasm_bindgen(js_name = clearTextReplacementRules)]
    pub fn clear_text_replacement_rules(&self) {
        clear_text_replacement_rules();
    }

    #[wasm_bindgen(js_name = setAutoSurroundEnabled)]
    pub fn set_auto_surround_enabled(&self, enabled: bool) {
        set_auto_surround_enabled(enabled);
    }

    #[wasm_bindgen(js_name = createEditor)]
    pub fn create_editor(&self, scale_factor: f64, snapshot: Option<Vec<u8>>) -> Editor {
        if let Some(snapshot) = snapshot {
            Editor::new_with_snapshot(scale_factor, snapshot)
        } else {
            Editor::new(scale_factor)
        }
    }
}

#[wasm_bindgen]
pub struct RenderInfo {
    pub ptr: u32,
    pub len: u32,
    pub width: u32,
    pub height: u32,
}

#[wasm_bindgen(getter_with_clone)]
pub struct ClipboardData {
    pub html: String,
    pub text: String,
}

#[wasm_bindgen]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[wasm_bindgen]
pub struct DragImageInfo {
    drag_image: DragImageResult,
}

#[wasm_bindgen]
impl DragImageInfo {
    #[wasm_bindgen(getter)]
    pub fn ptr(&self) -> u32 {
        self.drag_image.ptr() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn len(&self) -> u32 {
        self.drag_image.len() as u32
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.drag_image.width as u32
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.drag_image.height as u32
    }

    #[wasm_bindgen(getter, js_name = offsetX)]
    pub fn offset_x(&self) -> f32 {
        self.drag_image.offset_x
    }

    #[wasm_bindgen(getter, js_name = offsetY)]
    pub fn offset_y(&self) -> f32 {
        self.drag_image.offset_y
    }

    #[wasm_bindgen(getter, js_name = scaleFactor)]
    pub fn scale_factor(&self) -> f32 {
        self.drag_image.scale_factor
    }
}

#[wasm_bindgen]
pub struct Editor {
    runtime: Runtime,
}

impl Editor {
    fn new(scale_factor: f64) -> Self {
        let doc = Rc::new(Doc::new());
        let layout_mode = doc.settings().layout_mode;

        let width = match layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width,
        };

        let root = doc.node(NodeId::ROOT).unwrap();
        let paragraph_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        let state = State::new(
            doc,
            Selection::collapsed(Position::new(paragraph_id, 0, Affinity::default())),
        );

        let mut runtime = Runtime::new(width, scale_factor, state);

        runtime.layout();

        Self { runtime }
    }

    fn new_with_snapshot(scale_factor: f64, snapshot: Vec<u8>) -> Self {
        let doc = Rc::new(Doc::from_snapshot(snapshot));
        let layout_mode = doc.settings().layout_mode;
        let selection_pos = {
            let root = doc.node(NodeId::ROOT).unwrap();
            leaf_block_end(&root)
        };

        let width = match layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width,
        };

        let state = State::new(doc, Selection::collapsed(selection_pos));

        let mut runtime = Runtime::new(width, scale_factor, state);

        runtime.layout();

        Self { runtime }
    }
}

#[wasm_bindgen]
impl Editor {
    #[wasm_bindgen(js_name = renderPage)]
    pub fn render_page(&mut self, page_index: usize) -> Option<RenderInfo> {
        let result = self.runtime.render_page(page_index)?;
        Some(RenderInfo {
            ptr: result.ptr as u32,
            len: result.len as u32,
            width: result.width as u32,
            height: result.height as u32,
        })
    }

    #[wasm_bindgen(js_name = export)]
    pub fn export(&self, mode: DocExportMode) -> Vec<u8> {
        self.runtime.doc().export(mode).unwrap()
    }

    #[wasm_bindgen(js_name = importUpdates)]
    pub fn import_updates(&mut self, updates: Vec<u8>) {
        self.runtime.import_updates(&updates).unwrap()
    }

    #[wasm_bindgen(js_name = insertTemplateFragment)]
    pub fn insert_template_fragment(&mut self, snapshot: Vec<u8>) {
        self.runtime.insert_template_fragment(snapshot).unwrap()
    }

    #[wasm_bindgen(js_name = importUpdatesBatch)]
    pub fn import_updates_batch(&mut self, updates_batch: js_sys::Array) {
        let batch: Vec<Vec<u8>> = updates_batch
            .iter()
            .filter_map(|v| v.dyn_into::<js_sys::Uint8Array>().ok())
            .map(|arr| arr.to_vec())
            .collect();
        self.runtime.import_updates_batch(&batch).unwrap()
    }

    #[wasm_bindgen(js_name = checkout)]
    pub fn checkout(&mut self, version: Vec<u8>) -> Result<(), JsValue> {
        self.runtime.checkout(&version).map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = checkoutToLatest)]
    pub fn checkout_to_latest(&mut self) -> Result<(), JsValue> {
        self.runtime
            .checkout_to_latest()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = isDetached)]
    pub fn is_detached(&self) -> bool {
        self.runtime.is_detached()
    }

    #[wasm_bindgen(js_name = revertTo)]
    pub fn revert_to(&mut self, version: Vec<u8>) -> Result<(), JsValue> {
        self.runtime
            .revert_to(&version)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = isSelectionHit)]
    pub fn is_selection_hit(&self, page_idx: usize, x: f32, y: f32) -> bool {
        if let Some(page) = self.runtime.pages().get(page_idx) {
            is_selection_hit(self.runtime.doc(), page, self.runtime.selection(), x, y)
        } else {
            false
        }
    }

    #[wasm_bindgen(js_name = renderDragImage)]
    pub fn render_drag_image(
        &mut self,
        visible_pages: Vec<usize>,
        page_idx: usize,
    ) -> Option<DragImageInfo> {
        let drag_image = self.runtime.render_drag_image(&visible_pages, page_idx)?;
        Some(DragImageInfo { drag_image })
    }

    #[wasm_bindgen(js_name = dispatch)]
    pub fn dispatch(&mut self, val: JsValue) {
        if let Ok(msg) = serde_wasm_bindgen::from_value::<Message>(val) {
            self.runtime.update(msg);
        }
    }

    #[wasm_bindgen(js_name = inspectState)]
    pub fn inspect_state(&self) -> String {
        self.runtime.inspect_state()
    }

    #[wasm_bindgen(js_name = inspectStateAsMacro)]
    pub fn inspect_state_as_macro(&self) -> String {
        self.runtime.inspect_state_as_macro()
    }

    #[wasm_bindgen(js_name = inspectSelectionAsFragmentMacro)]
    pub fn inspect_selection_as_fragment_macro(&self) -> Option<String> {
        self.runtime.inspect_selection_as_fragment_macro()
    }

    #[wasm_bindgen(js_name = inspectPageElement)]
    pub fn inspect_page_element(&self, page_idx: usize, x: f32, y: f32) -> Option<String> {
        self.runtime.inspect_page_element(page_idx, x, y)
    }

    pub fn tick(&mut self) {
        self.runtime.tick();
    }

    #[wasm_bindgen(js_name = getSlatePtr)]
    pub fn get_slate_ptr(&self) -> u32 {
        &self.runtime.slate as *const _ as u32
    }

    #[wasm_bindgen(js_name = getSlateLen)]
    pub fn get_slate_len(&self) -> u32 {
        std::mem::size_of::<slate::Slate>() as u32
    }

    #[wasm_bindgen(js_name = getSlabPtr)]
    pub fn get_slab_ptr(&self) -> u32 {
        self.runtime.slab.data.as_ptr() as u32
    }

    #[wasm_bindgen(js_name = getSlabLen)]
    pub fn get_slab_len(&self) -> u32 {
        self.runtime.slab.len() as u32
    }

    #[wasm_bindgen(js_name = getSlateOffsets)]
    pub fn get_slate_offsets(&self) -> JsValue {
        let offsets = slate::get_slate_offsets();
        to_js_value(&offsets)
    }

    #[wasm_bindgen(js_name = enqueueMessage)]
    pub fn enqueue_message(&mut self, val: JsValue) {
        if let Ok(msg) = serde_wasm_bindgen::from_value::<Message>(val) {
            self.runtime.enqueue_message(msg);
        }
    }

    #[wasm_bindgen(js_name = flush)]
    pub fn flush(&mut self) {
        self.runtime.flush();
    }

    #[wasm_bindgen(js_name = getClipboardData)]
    pub fn get_clipboard_data(&self) -> Option<ClipboardData> {
        let state = self.runtime.state();
        if state.selection.is_collapsed() {
            return None;
        }

        let fragment = state.selection.extract_fragment(&state.doc).ok()?;

        if fragment.is_empty() {
            return None;
        }

        let text = fragment.to_plain_text();
        let html = fragment.to_html();

        Some(ClipboardData { html, text })
    }

    #[wasm_bindgen(js_name = getCharacterCounts)]
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

    #[wasm_bindgen(js_name = getCharacterCountAtVersion)]
    pub fn get_character_count_at_version(&self, version: Vec<u8>) -> Option<u32> {
        let vv = loro::VersionVector::decode(&version).ok()?;
        let loro_doc = self.runtime.doc().loro_doc();

        if !loro_doc.oplog_vv().includes_vv(&vv) {
            return None;
        }

        let target_frontiers = loro_doc.vv_to_frontiers(&vv);

        if target_frontiers == loro_doc.oplog_frontiers() {
            return Some(count_all(&self.runtime.doc().to_plain_text()).0);
        }

        let snapshot = loro_doc.export(loro::ExportMode::Snapshot).ok()?;
        let history_doc = Doc::from_snapshot(snapshot);
        history_doc.loro_doc().checkout(&target_frontiers).ok()?;

        Some(count_all(&history_doc.to_plain_text()).0)
    }

    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.runtime.set_read_only(read_only);
    }

    #[wasm_bindgen(js_name = isReadOnly)]
    pub fn is_read_only(&self) -> bool {
        self.runtime.is_read_only()
    }

    #[wasm_bindgen(js_name = setTrackedItems)]
    pub fn set_tracked_items(&mut self, group: u32, raw_items: Vec<RawTrackedItem>) {
        self.runtime.set_tracked_items(group, raw_items);
    }

    #[wasm_bindgen(js_name = getTextWithMappings)]
    pub fn get_text_with_mappings(&self) -> Result<JsValue, JsValue> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct TextWithMappingsResult {
            text: String,
            mappings: Vec<TextMapping>,
        }

        let (text, mappings) = self.runtime.doc().to_text_with_mappings();
        let result = TextWithMappingsResult { text, mappings };
        Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| e.to_string())?)
    }

    #[wasm_bindgen(js_name = performSearch)]
    pub fn perform_search(&self, query: &str, match_whole_word: bool) -> JsValue {
        let search_query = SearchQuery::new(query.to_string(), match_whole_word);
        let matches = perform_search(&self.runtime.doc(), &search_query);

        #[derive(Serialize)]
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

        to_js_value(&results)
    }

    #[wasm_bindgen(js_name = replaceTextInBlock)]
    pub fn replace_text_in_block(
        &mut self,
        block_id: &str,
        start_offset: usize,
        end_offset: usize,
        replacement: &str,
    ) -> bool {
        let Some(block_id) = NodeId::from_string(block_id) else {
            return false;
        };
        self.runtime
            .replace_text_in_block(block_id, start_offset, end_offset, replacement)
            .is_ok()
    }

    #[wasm_bindgen(js_name = replaceTextInBlocks)]
    pub fn replace_text_in_blocks(&mut self, items: JsValue) -> bool {
        let entries: Vec<(String, usize, usize, String)> =
            match serde_wasm_bindgen::from_value(items) {
                Ok(v) => v,
                Err(_) => return false,
            };

        let replacements: Vec<_> = entries
            .iter()
            .filter_map(|(node_id, start, end, replacement)| {
                NodeId::from_string(node_id).map(|id| (id, *start, *end, replacement.as_str()))
            })
            .collect();

        self.runtime.replace_text_in_blocks(&replacements).is_ok()
    }
}

fn count_all(text: &str) -> (u32, u32, u32) {
    let gc_data = get_general_category_map();
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

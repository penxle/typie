use crate::global::{register_fallback_font_family, register_font_family};
use crate::model::{Doc, LayoutMode, Node, NodeId, ParagraphNode};
use crate::runtime::{Message, Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;
use serde::Serialize;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer).unwrap()
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct SyncVersion {
    inner: loro::VersionVector,
}

#[wasm_bindgen]
impl SyncVersion {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: loro::VersionVector::new(),
        }
    }

    #[wasm_bindgen(js_name = encode)]
    pub fn encode(&self) -> Vec<u8> {
        self.inner.encode()
    }

    #[wasm_bindgen(js_name = decode)]
    pub fn decode(data: Vec<u8>) -> Self {
        Self {
            inner: loro::VersionVector::decode(&data).unwrap(),
        }
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct ExportedUpdates {
    pub updates: Vec<u8>,
    pub version: SyncVersion,
}

#[wasm_bindgen(js_name = getMemory)]
pub fn get_memory() -> JsValue {
    wasm_bindgen::memory()
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
        crate::icu_data::load_icu_data(&icu_data)
    }

    #[wasm_bindgen(js_name = registerFont)]
    pub fn register_font(&self, name: &str, weight: u16, data: Vec<u8>) {
        register_font_family(name, weight, &data);
    }

    #[wasm_bindgen(js_name = registerFallbackFont)]
    pub fn register_fallback_font(&self, name: &str, weight: u16, data: Vec<u8>) {
        register_fallback_font_family(name, weight, &data);
    }

    #[wasm_bindgen(js_name = setAvailableFonts)]
    pub fn set_available_fonts(&self, fonts: JsValue) {
        if let Ok(fonts) = serde_wasm_bindgen::from_value(fonts) {
            crate::global::set_available_fonts(fonts);
        }
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
    pub fragment: String,
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
    drag_image: crate::render::DragImageResult,
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

        let width = match layout_mode {
            LayoutMode::Paginated { page_width, .. } => page_width,
            LayoutMode::Continuous { max_width, .. } => max_width,
        };

        let state = State::new(
            doc,
            Selection::collapsed(Position::new(NodeId::ROOT, 0, Affinity::default())),
        );

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

    #[wasm_bindgen(js_name = getSnapshot)]
    pub fn get_snapshot(&self) -> Vec<u8> {
        self.runtime.doc().snapshot().unwrap()
    }

    #[wasm_bindgen(js_name = getVersion)]
    pub fn get_version(&self) -> Vec<u8> {
        self.runtime.doc().loro_doc().oplog_vv().encode()
    }

    #[wasm_bindgen(js_name = exportAllUpdates)]
    pub fn export_all_updates(&self) -> Vec<u8> {
        self.runtime.doc().export_all_updates().unwrap()
    }

    #[wasm_bindgen(js_name = exportUpdatesFrom)]
    pub fn export_updates_from(&self, version: Vec<u8>) -> Vec<u8> {
        let vv = loro::VersionVector::decode(&version).unwrap();
        self.runtime.doc().export_updates_from(&vv).unwrap()
    }

    #[wasm_bindgen(js_name = importUpdates)]
    pub fn import_updates(&mut self, updates: Vec<u8>) {
        self.runtime.import_updates(&updates).unwrap()
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

    #[wasm_bindgen(js_name = exportNewUpdates)]
    pub fn export_new_updates(&self) -> Result<ExportedUpdates, JsValue> {
        let (updates, version) = self
            .runtime
            .export_new_updates()
            .map_err(|e| e.to_string())?;

        Ok(ExportedUpdates {
            updates,
            version: SyncVersion { inner: version },
        })
    }

    #[wasm_bindgen(js_name = commitSync)]
    pub fn commit_sync(&mut self, version: SyncVersion) {
        self.runtime.commit_sync(version.inner);
    }

    #[wasm_bindgen(js_name = canDragAt)]
    pub fn can_drag_at(&self, page_idx: usize, x: f32, y: f32) -> bool {
        self.runtime.can_drag_at(page_idx, x, y)
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
    pub fn dispatch(&mut self, val: JsValue) -> JsValue {
        if let Ok(msg) = serde_wasm_bindgen::from_value::<Message>(val) {
            let cmd = self.runtime.update(msg);
            to_js_value(&cmd)
        } else {
            JsValue::NULL
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

    pub fn tick(&mut self) -> JsValue {
        let cmds = self.runtime.tick();
        if cmds.is_empty() {
            JsValue::NULL
        } else {
            to_js_value(&cmds)
        }
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

        let fragment_json = fragment.to_json().ok()?;
        let text = fragment.to_plain_text();
        let html = fragment.to_html();

        Some(ClipboardData {
            fragment: fragment_json,
            html,
            text,
        })
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
}

fn count_all(text: &str) -> (u32, u32, u32) {
    use icu_properties::props::GeneralCategory;

    let gc_data = crate::icu_data::get_general_category_map();
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

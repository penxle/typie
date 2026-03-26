use super::common::{CharacterCounts, ClipboardData, EditorCore, count_all};
use crate::font::FontMetadata;
use crate::font::encode::EncodedFont;
use crate::global::{add_font_base, add_font_chunk, set_available_fonts};
use crate::global::{
    clear_text_replacement_rules, set_auto_surround_enabled, set_text_replacement_rules,
};
use crate::icu_data::load_icu_data;
use crate::layout::query::{is_cursor_hit, is_selection_hit};
use crate::model::{Doc, DocExportMode, DocumentJson, NodeId, json_to_snapshot, snapshot_to_json};
use crate::render::DragImageResult;
use crate::render::backend::{GpuDevice, RenderBackend};
use crate::runtime::text_replacement::RawTextReplacementRule;
use crate::runtime::{Message, RawTrackedItem, slate};
use crate::state::{Position, leaf_block_end};
use crate::types::Affinity;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
pub struct Codepoints(Vec<u32>);

fn to_js_value<T: Serialize>(value: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub struct EditorEngine {
    gpu: Option<Arc<Mutex<GpuDevice>>>,
}

#[wasm_bindgen]
impl EditorEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self { gpu: None }
    }

    #[wasm_bindgen(js_name = initGpu)]
    pub async fn init_gpu(&mut self) -> bool {
        match GpuDevice::new().await {
            Some(ctx) => {
                self.gpu = Some(Arc::new(Mutex::new(ctx)));
                true
            }
            None => false,
        }
    }

    #[wasm_bindgen(js_name = getMemory)]
    pub fn get_memory(&self) -> Result<JsValue, JsValue> {
        Ok(wasm_bindgen::memory())
    }

    #[wasm_bindgen(js_name = loadIcuData)]
    pub fn load_icu_data(&self, icu_data: Vec<u8>) -> Result<(), JsValue> {
        load_icu_data(&icu_data)
    }

    #[wasm_bindgen(js_name = addFontBase)]
    pub fn add_font_base(&self, family: &str, weight: u16, data: Vec<u8>) -> Result<(), JsValue> {
        add_font_base(family, weight, &data);
        Ok(())
    }

    #[wasm_bindgen(js_name = addFontChunk)]
    pub fn add_font_chunk(&self, family: &str, weight: u16, data: Vec<u8>) -> Result<(), JsValue> {
        add_font_chunk(family, weight, &data);
        Ok(())
    }

    #[wasm_bindgen(js_name = setAvailableFonts)]
    pub fn set_available_fonts(&self, fonts: JsValue) -> Result<(), JsValue> {
        let fonts =
            serde_wasm_bindgen::from_value::<std::collections::HashMap<String, Vec<u16>>>(fonts)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        set_available_fonts(fonts);
        Ok(())
    }

    #[wasm_bindgen(js_name = setTextReplacementRules)]
    pub fn set_text_replacement_rules(&self, rules: JsValue) -> Result<(), JsValue> {
        let raw_rules = serde_wasm_bindgen::from_value::<Vec<RawTextReplacementRule>>(rules)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        set_text_replacement_rules(raw_rules);
        Ok(())
    }

    #[wasm_bindgen(js_name = clearTextReplacementRules)]
    pub fn clear_text_replacement_rules(&self) -> Result<(), JsValue> {
        clear_text_replacement_rules();
        Ok(())
    }

    #[wasm_bindgen(js_name = setAutoSurroundEnabled)]
    pub fn set_auto_surround_enabled(&self, enabled: bool) -> Result<(), JsValue> {
        set_auto_surround_enabled(enabled);
        Ok(())
    }

    #[wasm_bindgen(js_name = createEditor)]
    pub fn create_editor(
        &self,
        scale_factor: f64,
        snapshot: Option<Vec<u8>>,
    ) -> Result<Editor, JsValue> {
        let gpu = self.gpu.clone();
        let backend = match &self.gpu {
            Some(gpu) => RenderBackend::new_gpu(Arc::clone(gpu)),
            None => RenderBackend::new_cpu(),
        };
        if let Some(snapshot) = snapshot {
            Editor::new_with_snapshot(scale_factor, snapshot, backend, gpu)
        } else {
            Ok(Editor::new(scale_factor, backend, gpu))
        }
    }

    #[wasm_bindgen(js_name = validateRegex)]
    pub fn validate_regex(&self, pattern: &str) -> Result<bool, JsValue> {
        let anchored = format!("(?:{pattern})$");
        Ok(fancy_regex::Regex::new(&anchored).is_ok())
    }

    #[wasm_bindgen(js_name = getFontMetadata)]
    pub fn get_font_metadata(&self, data: Vec<u8>) -> Result<FontMetadata, JsValue> {
        crate::font::get_font_metadata(&data).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = outlineTextToSvg)]
    pub fn outline_text_to_svg(&self, font_data: Vec<u8>, text: &str) -> Result<String, JsValue> {
        crate::font::outline_text_to_svg(&font_data, text).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = getFontCodepoints)]
    pub fn get_font_codepoints(&self, ttf_data: Vec<u8>) -> Result<Codepoints, JsValue> {
        let codepoints = crate::font::encode::get_font_codepoints(&ttf_data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Codepoints(codepoints))
    }

    #[wasm_bindgen(js_name = encodeFont)]
    pub fn encode_font(
        &self,
        ttf_data: Vec<u8>,
        chunk_codepoints_json: &str,
    ) -> Result<EncodedFont, JsValue> {
        let chunk_codepoints: Vec<Vec<u32>> = serde_json::from_str(chunk_codepoints_json)
            .map_err(|e| JsValue::from_str(&format!("chunk_codepoints parse: {e}")))?;
        crate::font::encode::encode_font(&ttf_data, &chunk_codepoints)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = snapshotToJson)]
    pub fn snapshot_to_json(&self, snapshot: Vec<u8>) -> Result<JsValue, JsValue> {
        let doc_json =
            snapshot_to_json(&snapshot).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(to_js_value(&doc_json))
    }

    #[wasm_bindgen(js_name = jsonToSnapshot)]
    pub fn json_to_snapshot(&self, json: JsValue) -> Result<Vec<u8>, JsValue> {
        let doc_json: DocumentJson = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        json_to_snapshot(&doc_json).map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }

    #[wasm_bindgen(js_name = validateDocumentJson)]
    pub fn validate_document_json(&self, json: JsValue) -> Result<(), JsValue> {
        let doc_json: DocumentJson = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let snapshot =
            json_to_snapshot(&doc_json).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let doc =
            Doc::from_snapshot(snapshot).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        doc.validate_exhaustive()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }
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

/// WASM per-surface state
struct WasmSurface {
    #[allow(dead_code)]
    offscreen: web_sys::OffscreenCanvas,
    target: WasmRenderTarget,
    /// OffscreenCanvas 버퍼 크기 (스크롤 모드에서 max height 포함)
    width: u32,
    height: u32,
}

#[allow(dead_code)]
enum WasmRenderTarget {
    Gpu {
        surface: wgpu::Surface<'static>,
        format: wgpu::TextureFormat,
    },
    Cpu {
        ctx: web_sys::OffscreenCanvasRenderingContext2d,
    },
}

#[wasm_bindgen]
pub struct Editor {
    core: EditorCore,
    gpu: Option<Arc<Mutex<GpuDevice>>>,
    surfaces: FxHashMap<u32, WasmSurface>,
}

impl Editor {
    fn new(scale_factor: f64, backend: RenderBackend, gpu: Option<Arc<Mutex<GpuDevice>>>) -> Self {
        let core = EditorCore::new(scale_factor, backend);
        Self {
            core,
            gpu,
            surfaces: FxHashMap::default(),
        }
    }

    fn new_with_snapshot(
        scale_factor: f64,
        snapshot: Vec<u8>,
        backend: RenderBackend,
        gpu: Option<Arc<Mutex<GpuDevice>>>,
    ) -> Result<Self, JsValue> {
        // web-specific: position cursor at end of document
        let temp_doc =
            Doc::from_snapshot(snapshot.clone()).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let selection_pos = temp_doc
            .node(NodeId::ROOT)
            .and_then(|root| leaf_block_end(&root))
            .unwrap_or(Position::new(NodeId::ROOT, 0, Affinity::Downstream));

        let core = EditorCore::with_snapshot(scale_factor, snapshot, backend, Some(selection_pos))
            .map_err(|e| JsValue::from_str(&e))?;

        Ok(Self {
            core,
            gpu,
            surfaces: FxHashMap::default(),
        })
    }
}

#[wasm_bindgen]
impl Editor {
    // ── Mount / Render API ──────────────────────────────────────────

    #[wasm_bindgen(js_name = attachSurface)]
    pub fn attach_surface(&mut self, page_index: u32) -> Result<web_sys::OffscreenCanvas, JsValue> {
        let info = self
            .core
            .runtime_mut()
            .attach_surface(page_index)
            .ok_or_else(|| JsValue::from_str("invalid page index"))?;

        let offscreen = web_sys::OffscreenCanvas::new(info.width, info.height)?;

        #[allow(unused)]
        let target = if let Some(gpu_arc) = &self.gpu {
            #[cfg(target_arch = "wasm32")]
            {
                let gpu = gpu_arc.lock().unwrap_or_else(|e| e.into_inner());
                let surface = gpu
                    .instance
                    .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(offscreen.clone()))
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                let caps = surface.get_capabilities(&gpu.adapter);
                let format = caps
                    .formats
                    .first()
                    .copied()
                    .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
                surface.configure(
                    &gpu.device,
                    &wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format,
                        width: info.width,
                        height: info.height,
                        present_mode: wgpu::PresentMode::AutoVsync,
                        alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                    },
                );
                WasmRenderTarget::Gpu { surface, format }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                return Err(JsValue::from_str("GPU surface requires wasm32 target"));
            }
        } else {
            let ctx = offscreen
                .get_context("2d")
                .map_err(|e| e)?
                .ok_or_else(|| JsValue::from_str("failed to get 2d context"))?
                .dyn_into::<web_sys::OffscreenCanvasRenderingContext2d>()?;
            WasmRenderTarget::Cpu { ctx }
        };

        self.surfaces.insert(
            page_index,
            WasmSurface {
                offscreen: offscreen.clone(),
                target,
                width: info.width,
                height: info.height,
            },
        );

        Ok(offscreen)
    }

    #[wasm_bindgen(js_name = detachSurface)]
    pub fn detach_surface(&mut self, page_index: u32) -> Result<(), JsValue> {
        self.surfaces.remove(&page_index);
        self.core.runtime_mut().detach_surface(page_index);
        Ok(())
    }

    #[wasm_bindgen(js_name = renderSurface)]
    pub fn render_surface(&mut self, page_index: u32) -> Result<(), JsValue> {
        self.render_single_surface(page_index);
        Ok(())
    }

    #[wasm_bindgen(js_name = exportPage)]
    pub fn export_page(&mut self, page_index: usize) -> Result<Option<Vec<u8>>, JsValue> {
        Ok(self.core.runtime_mut().export_page(page_index))
    }

    #[wasm_bindgen(js_name = export)]
    pub fn export(&mut self, mode: DocExportMode) -> Result<Vec<u8>, JsValue> {
        self.core
            .runtime_mut()
            .export(mode)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = importUpdates)]
    pub fn import_updates(&mut self, updates: Vec<u8>) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .import_updates(&updates)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = insertTemplateFragment)]
    pub fn insert_template_fragment(&mut self, snapshot: Vec<u8>) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .insert_template_fragment(snapshot)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = importUpdatesBatch)]
    pub fn import_updates_batch(&mut self, updates_batch: js_sys::Array) -> Result<(), JsValue> {
        let batch: Vec<Vec<u8>> = updates_batch
            .iter()
            .filter_map(|v| v.dyn_into::<js_sys::Uint8Array>().ok())
            .map(|arr| arr.to_vec())
            .collect();
        self.core
            .runtime_mut()
            .import_updates_batch(&batch)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = checkout)]
    pub fn checkout(&mut self, version: Vec<u8>) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .checkout(&version)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = checkoutToLatest)]
    pub fn checkout_to_latest(&mut self) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .checkout_to_latest()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = isDetached)]
    pub fn is_detached(&self) -> Result<bool, JsValue> {
        Ok(self.core.runtime().is_detached())
    }

    #[wasm_bindgen(js_name = revertTo)]
    pub fn revert_to(&mut self, version: Vec<u8>) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .revert_to(&version)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = isSelectionHit)]
    pub fn is_selection_hit(&self, page_idx: usize, x: f32, y: f32) -> Result<bool, JsValue> {
        let rt = self.core.runtime();
        if let Some(page) = rt.pages().get(page_idx) {
            Ok(is_selection_hit(rt.doc(), page, rt.selection(), x, y))
        } else {
            Ok(false)
        }
    }

    #[wasm_bindgen(js_name = isCursorHit)]
    pub fn is_cursor_hit(&self, page_idx: usize, x: f32, y: f32) -> Result<bool, JsValue> {
        let rt = self.core.runtime();
        if let Some(page) = rt.pages().get(page_idx) {
            Ok(is_cursor_hit(rt.doc(), page, rt.selection(), x, y))
        } else {
            Ok(false)
        }
    }

    #[wasm_bindgen(js_name = renderDragImage)]
    pub fn render_drag_image(
        &mut self,
        visible_pages: Vec<usize>,
        page_idx: usize,
    ) -> Result<Option<DragImageInfo>, JsValue> {
        let drag_image = self
            .core
            .runtime_mut()
            .render_drag_image(&visible_pages, page_idx);
        Ok(drag_image.map(|d| DragImageInfo { drag_image: d }))
    }

    #[wasm_bindgen(js_name = inspectState)]
    pub fn inspect_state(&self) -> Result<String, JsValue> {
        Ok(self.core.runtime().inspect_state())
    }

    #[wasm_bindgen(js_name = inspectStateAsMacro)]
    pub fn inspect_state_as_macro(&self) -> Result<String, JsValue> {
        Ok(self.core.runtime().inspect_state_as_macro())
    }

    #[wasm_bindgen(js_name = inspectSelectionAsFragmentMacro)]
    pub fn inspect_selection_as_fragment_macro(&self) -> Result<Option<String>, JsValue> {
        Ok(self.core.runtime().inspect_selection_as_fragment_macro())
    }

    #[wasm_bindgen(js_name = inspectPageElement)]
    pub fn inspect_page_element(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Result<Option<String>, JsValue> {
        Ok(self.core.runtime().inspect_page_element(page_idx, x, y))
    }

    pub fn tick(&mut self) -> Result<(), JsValue> {
        self.core.runtime_mut().tick();
        Ok(())
    }

    #[wasm_bindgen(js_name = setTracing)]
    pub fn set_tracing(&mut self, trace_id: &str, parent_span_id: &str) -> Result<(), JsValue> {
        let trace_id = opentelemetry::trace::TraceId::from_hex(trace_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid trace_id: {e}")))?;
        let parent_span_id = opentelemetry::trace::SpanId::from_hex(parent_span_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid parent_span_id: {e}")))?;
        self.core
            .runtime_mut()
            .tracing
            .set_tracing(trace_id, parent_span_id);
        Ok(())
    }

    #[wasm_bindgen(js_name = clearTracing)]
    pub fn clear_tracing(&mut self) -> Result<(), JsValue> {
        self.core.runtime_mut().tracing.clear_tracing();
        Ok(())
    }

    #[wasm_bindgen(js_name = drainTraces)]
    pub fn drain_traces(&mut self) -> Result<JsValue, JsValue> {
        let traces = self.core.runtime_mut().tracing.drain();
        Ok(serde_wasm_bindgen::to_value(&traces).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = getSlatePtr)]
    pub fn get_slate_ptr(&self) -> Result<u32, JsValue> {
        Ok(&self.core.runtime().slate as *const _ as u32)
    }

    #[wasm_bindgen(js_name = getSlateLen)]
    pub fn get_slate_len(&self) -> Result<u32, JsValue> {
        Ok(std::mem::size_of::<slate::Slate>() as u32)
    }

    #[wasm_bindgen(js_name = getSlabPtr)]
    pub fn get_slab_ptr(&self) -> Result<u32, JsValue> {
        Ok(self.core.runtime().slab.data.as_ptr() as u32)
    }

    #[wasm_bindgen(js_name = getSlabLen)]
    pub fn get_slab_len(&self) -> Result<u32, JsValue> {
        Ok(self.core.runtime().slab.len() as u32)
    }

    #[wasm_bindgen(js_name = getSlateOffsets)]
    pub fn get_slate_offsets(&self) -> Result<JsValue, JsValue> {
        let offsets = slate::get_slate_offsets();
        Ok(to_js_value(&offsets))
    }

    #[wasm_bindgen(js_name = enqueueMessage)]
    pub fn enqueue_message(&mut self, val: JsValue) -> Result<(), JsValue> {
        let msg = serde_wasm_bindgen::from_value::<Message>(val)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.core.runtime_mut().enqueue_message(msg);
        Ok(())
    }

    #[wasm_bindgen(js_name = flush)]
    pub fn flush(&mut self) -> Result<(), JsValue> {
        self.core.runtime_mut().flush();
        Ok(())
    }

    #[wasm_bindgen(js_name = getClipboardData)]
    pub fn get_clipboard_data(&self) -> Result<Option<ClipboardData>, JsValue> {
        Ok(self.core.get_clipboard_data())
    }

    #[wasm_bindgen(js_name = getCharacterCounts)]
    pub fn get_character_counts(&mut self) -> Result<CharacterCounts, JsValue> {
        Ok(self.core.get_character_counts())
    }

    #[wasm_bindgen(js_name = getCharacterCountAtVersion)]
    pub fn get_character_count_at_version(
        &mut self,
        version: Vec<u8>,
    ) -> Result<Option<u32>, JsValue> {
        self.core.runtime_mut().flush();
        let vv = match loro::VersionVector::decode(&version) {
            Ok(vv) => vv,
            Err(_) => return Ok(None),
        };
        let loro_doc = self.core.runtime().doc().loro_doc();

        if !loro_doc.oplog_vv().includes_vv(&vv) {
            return Ok(None);
        }

        let target_frontiers = loro_doc.vv_to_frontiers(&vv);

        if target_frontiers == loro_doc.oplog_frontiers() {
            return Ok(Some(
                count_all(&self.core.runtime().doc().to_plain_text()).0,
            ));
        }

        let snapshot = match loro_doc.export(loro::ExportMode::Snapshot) {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };
        let history_doc = match Doc::from_snapshot(snapshot) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        if history_doc.loro_doc().checkout(&target_frontiers).is_err() {
            return Ok(None);
        }

        Ok(Some(count_all(&history_doc.to_plain_text()).0))
    }

    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(&mut self, read_only: bool) -> Result<(), JsValue> {
        self.core.runtime_mut().set_read_only(read_only);
        Ok(())
    }

    #[wasm_bindgen(js_name = setRenderDebug)]
    pub fn set_render_debug(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.core.runtime_mut().set_render_debug(enabled);
        Ok(())
    }

    #[wasm_bindgen(js_name = setLayoutDebug)]
    pub fn set_layout_debug(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.core.runtime_mut().set_layout_debug(enabled);
        Ok(())
    }

    #[wasm_bindgen(js_name = setAllFoldsExpanded)]
    pub fn set_all_folds_expanded(&mut self, expanded: bool) -> Result<(), JsValue> {
        self.core.runtime_mut().set_all_folds_expanded(expanded);
        Ok(())
    }

    #[wasm_bindgen(js_name = setMaxPages)]
    pub fn set_max_pages(&mut self, max_pages: Option<u32>) -> Result<(), JsValue> {
        self.core
            .runtime_mut()
            .set_max_pages(max_pages.map(|v| v as usize));
        Ok(())
    }

    #[wasm_bindgen(js_name = isReadOnly)]
    pub fn is_read_only(&self) -> Result<bool, JsValue> {
        Ok(self.core.runtime().is_read_only())
    }

    #[wasm_bindgen(js_name = setTrackedItems)]
    pub fn set_tracked_items(
        &mut self,
        group: u32,
        raw_items: Vec<RawTrackedItem>,
    ) -> Result<(), JsValue> {
        self.core.runtime_mut().set_tracked_items(group, raw_items);
        Ok(())
    }

    #[wasm_bindgen(js_name = removeTrackedItems)]
    pub fn remove_tracked_items(&mut self, group: u32, ids: Vec<String>) -> Result<(), JsValue> {
        self.core.runtime_mut().remove_tracked_items(group, &ids);
        Ok(())
    }

    #[wasm_bindgen(js_name = getTextWithMappings)]
    pub fn get_text_with_mappings(&self) -> Result<JsValue, JsValue> {
        let result = self.core.get_text_with_mappings();
        Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| e.to_string())?)
    }

    #[wasm_bindgen(js_name = performSearch)]
    pub fn perform_search(&self, query: &str, match_whole_word: bool) -> Result<JsValue, JsValue> {
        let results = self.core.perform_search(query, match_whole_word);
        Ok(to_js_value(&results))
    }

    #[wasm_bindgen(js_name = revealTrackedItem)]
    pub fn reveal_tracked_item(&mut self, group: u32, id: &str) -> Result<bool, JsValue> {
        Ok(self.core.runtime_mut().reveal_tracked_item(group, id))
    }

    #[wasm_bindgen(js_name = replaceTextInBlock)]
    pub fn replace_text_in_block(
        &mut self,
        block_id: &str,
        start_offset: usize,
        end_offset: usize,
        replacement: &str,
    ) -> Result<bool, JsValue> {
        let Some(block_id) = NodeId::from_string(block_id) else {
            return Ok(false);
        };
        Ok(self
            .core
            .runtime_mut()
            .replace_text_in_block(block_id, start_offset, end_offset, replacement)
            .is_ok())
    }

    #[wasm_bindgen(js_name = replaceTextInBlocks)]
    pub fn replace_text_in_blocks(&mut self, items: JsValue) -> Result<bool, JsValue> {
        let entries: Vec<(String, usize, usize, String)> =
            match serde_wasm_bindgen::from_value(items) {
                Ok(v) => v,
                Err(_) => return Ok(false),
            };

        let replacements: Vec<_> = entries
            .iter()
            .filter_map(|(node_id, start, end, replacement)| {
                NodeId::from_string(node_id).map(|id| (id, *start, *end, replacement.as_str()))
            })
            .collect();

        Ok(self
            .core
            .runtime_mut()
            .replace_text_in_blocks(&replacements)
            .is_ok())
    }
}

impl Editor {
    fn render_single_surface(&mut self, page_index: u32) {
        if !self.surfaces.contains_key(&page_index) {
            return;
        }
        self.resize_surface_if_stale(page_index);
        if self.gpu.is_some() {
            self.render_gpu_surface(page_index);
        } else {
            self.render_cpu_surface(page_index);
        }
    }

    /// 할당된 surface 크기가 현재 레이아웃과 다르면 OffscreenCanvas + GPU surface를 resize한다.
    fn resize_surface_if_stale(&mut self, page_index: u32) {
        let Some(new_info) = self.core.runtime_mut().resize_surface(page_index) else {
            return; // 크기 변경 없음
        };

        let Some(state) = self.surfaces.get_mut(&page_index) else {
            return;
        };

        // OffscreenCanvas resize
        state.offscreen.set_width(new_info.width);
        state.offscreen.set_height(new_info.height);
        state.width = new_info.width;
        state.height = new_info.height;

        // GPU surface 재구성
        if let WasmRenderTarget::Gpu {
            ref surface,
            format,
        } = state.target
        {
            surface.configure(
                &self
                    .gpu
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format,
                    width: new_info.width,
                    height: new_info.height,
                    present_mode: wgpu::PresentMode::AutoVsync,
                    alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                },
            );
        }
    }

    fn render_gpu_surface(&mut self, page_index: u32) {
        let Some(scene) = self.core.runtime_mut().build_surface_scene(page_index) else {
            return;
        };

        let Some(state) = self.surfaces.get(&page_index) else {
            return;
        };
        let WasmRenderTarget::Gpu {
            ref surface,
            ref format,
        } = state.target
        else {
            return;
        };
        let format = *format;

        let Ok(surface_texture) = surface.get_current_texture() else {
            return;
        };

        let gpu_arc = self.gpu.as_ref().unwrap().clone();
        let mut gpu = gpu_arc.lock().unwrap_or_else(|e| e.into_inner());
        let _ = gpu.render_to_surface(&scene, &surface_texture, format, state.width, state.height);
        surface_texture.present();
    }

    fn render_cpu_surface(&mut self, page_index: u32) {
        let Some(state) = self.surfaces.get(&page_index) else {
            return;
        };
        let width = state.width;
        let height = state.height;
        let buf_size = (width as usize) * (height as usize) * 4;

        let mut buf = vec![0u8; buf_size];
        if !self
            .core
            .runtime_mut()
            .render_surface_into(page_index as usize, &mut buf)
        {
            return;
        }

        let Some(state) = self.surfaces.get(&page_index) else {
            return;
        };
        let WasmRenderTarget::Cpu { ref ctx } = state.target else {
            return;
        };

        // premultiplied alpha -> straight alpha 변환
        let mut straight = vec![0u8; buf_size];
        for (src, dst) in buf.chunks_exact(4).zip(straight.chunks_exact_mut(4)) {
            let a = src[3] as u32;
            if a == 0 {
                dst.copy_from_slice(&[0, 0, 0, 0]);
            } else if a == 255 {
                dst.copy_from_slice(src);
            } else {
                dst[0] = ((src[0] as u32 * 255 + a / 2) / a).min(255) as u8;
                dst[1] = ((src[1] as u32 * 255 + a / 2) / a).min(255) as u8;
                dst[2] = ((src[2] as u32 * 255 + a / 2) / a).min(255) as u8;
                dst[3] = src[3];
            }
        }
        let clamped = wasm_bindgen::Clamped(&straight[..]);
        let Ok(image_data) =
            web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped, width, height)
        else {
            return;
        };
        let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
    }
}

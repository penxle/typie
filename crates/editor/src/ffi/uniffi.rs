// When modifying this file, update the following:
//   - apps/mobile2/compose/src/commonMain/kotlin/co/typie/editor/Editor.kt    (interface)
//   - apps/mobile2/compose/src/jnaMain/kotlin/co/typie/editor/Editor.jna.kt   (Android + Desktop)
//   - apps/mobile2/compose/src/iosMain/kotlin/co/typie/editor/Editor.ios.kt   (iOS)
//   - apps/mobile2/ios/Bridge/Sources/Bridge/Editor/Editor.swift              (Swift @objc bridge)

use std::sync::{Arc, Mutex};

use super::common::{CharacterCounts, EditorCore};
use crate::global::set_text_replacement_rules;
use crate::global::{add_font_base, add_font_chunk, set_available_fonts};
use crate::icu_data::load_icu_data;
use crate::layout::query::{is_cursor_hit, is_selection_hit};
use crate::model::{DocExportMode, NodeId};
use crate::render::PlatformBuffer;
use crate::render::backend::{GpuDevice, RenderBackend};
use crate::runtime::Message;
use crate::runtime::slate::get_slate_offsets;
use crate::runtime::text_replacement::RawTextReplacementRule;
use crate::runtime::tracked_items::RawTrackedItem;

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
pub struct SurfaceInfo {
    pub width: u32,
    pub height: u32,
    pub buffer_size: u64,
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

// ── SurfaceHandle ──────────────────────────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct SurfaceHandle {
    native_handle: u64,
    width: u32,
    height: u32,
    resources: std::sync::Mutex<Option<PlatformBuffer>>,
}

#[uniffi::export]
impl SurfaceHandle {
    pub fn native_handle(&self) -> u64 {
        self.native_handle
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Desktop JVM 전용: pixel buffer 데이터를 Vec<u8>로 반환.
    /// Android/iOS에서는 None 반환 (native_handle로 직접 접근).
    pub fn pixel_data(&self) -> Option<Vec<u8>> {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let guard = self.resources.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref()? {
                PlatformBuffer::Desktop { pixel_data } => Some(pixel_data.clone()),
            }
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            None
        }
    }

    // close()는 UniFFI가 자동 생성 (Disposable). Drop에서 리소스 해제.
}

// ── EditorEngine (Application-level) ────────────────────────────────────────

#[derive(uniffi::Object)]
pub struct EditorEngine {
    gpu: std::sync::Mutex<Option<Arc<std::sync::Mutex<GpuDevice>>>>,
}

#[uniffi::export]
impl EditorEngine {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            gpu: std::sync::Mutex::new(None),
        })
    }

    pub async fn init_gpu(&self) -> bool {
        match GpuDevice::new().await {
            Some(c) => {
                *self.gpu.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(Arc::new(std::sync::Mutex::new(c)));
                true
            }
            None => false,
        }
    }

    pub fn load_icu_data(&self, data: Vec<u8>) -> Result<(), EditorError> {
        load_icu_data(&data).map_err(EditorError::from)
    }

    pub fn create_editor(
        &self,
        scale_factor: f64,
        snapshot: Option<Vec<u8>>,
    ) -> Result<Arc<Editor>, EditorError> {
        let backend = {
            let gpu = self.gpu.lock().unwrap_or_else(|e| e.into_inner());
            match &*gpu {
                Some(ctx) => RenderBackend::new_gpu(Arc::clone(ctx)),
                None => RenderBackend::new_cpu(),
            }
        };
        let core = match snapshot {
            Some(data) if !data.is_empty() => {
                EditorCore::with_snapshot(scale_factor, data, backend, None)?
            }
            _ => EditorCore::new(scale_factor, backend),
        };
        Ok(Arc::new(Editor {
            inner: Mutex::new(EditorInner {
                core,
                surfaces: std::collections::HashMap::new(),
            }),
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
    core: EditorCore,
    surfaces: std::collections::HashMap<u32, Arc<SurfaceHandle>>,
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
        inner.core.runtime_mut().enqueue_message(message);
        Ok(())
    }

    pub fn tick(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.core.runtime_mut().tick();
        Ok(())
    }

    pub fn flush(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.core.runtime_mut().flush();
        Ok(())
    }

    // ── Mount / Render API ────────────────────────────────────────────

    pub fn attach_surface(&self, page_index: u32) -> Result<Arc<SurfaceHandle>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let surface_size = inner
            .core
            .runtime_mut()
            .attach_surface(page_index)
            .ok_or_else(|| EditorError::General {
                msg: format!("invalid page index: {page_index}"),
            })?;

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let (native_handle, resources) = {
            let pixel_data = vec![0u8; (surface_size.width * surface_size.height * 4) as usize];
            (0u64, PlatformBuffer::Desktop { pixel_data })
        };

        #[cfg(target_os = "android")]
        let (native_handle, resources) = {
            use crate::render::backend::gpu::platform::android::AHardwareBufferWrapper;
            let buffer = AHardwareBufferWrapper::new(surface_size.width, surface_size.height)
                .ok_or_else(|| EditorError::General {
                    msg: "failed to allocate AHardwareBuffer".into(),
                })?;
            let handle = buffer.native_handle();
            (handle, PlatformBuffer::Android { buffer })
        };

        #[cfg(target_os = "ios")]
        let (native_handle, resources) = {
            use crate::render::backend::gpu::platform::ios::IOSurfaceWrapper;
            let surface = IOSurfaceWrapper::new(surface_size.width, surface_size.height)
                .ok_or_else(|| EditorError::General {
                    msg: "failed to create IOSurface".into(),
                })?;
            let handle = surface.native_handle();
            (handle, PlatformBuffer::Ios { surface })
        };

        let texture = Arc::new(SurfaceHandle {
            native_handle,
            width: surface_size.width,
            height: surface_size.height,
            resources: std::sync::Mutex::new(Some(resources)),
        });
        inner.surfaces.insert(page_index, Arc::clone(&texture));
        Ok(texture)
    }

    pub fn detach_surface(&self, page_index: u32) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.surfaces.remove(&page_index);
        inner.core.runtime_mut().detach_surface(page_index);
        Ok(())
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn get_page_count(&self) -> Result<u32, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner.core.runtime().pages().len() as u32)
    }

    pub fn get_surface_info(&self, page_index: u32) -> Result<Option<SurfaceInfo>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner
            .core
            .runtime_mut()
            .get_surface_info(page_index as usize)
            .map(|info| SurfaceInfo {
                width: info.width as u32,
                height: info.height as u32,
                buffer_size: info.buffer_size as u64,
            }))
    }

    /// 단일 surface를 렌더링하여 SurfaceHandle의 pixel buffer에 기록한다.
    pub fn render_surface(&self, page_index: u32) -> Result<bool, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(handle) = inner.surfaces.get(&page_index).cloned() else {
            return Ok(false);
        };

        let info = inner
            .core
            .runtime_mut()
            .get_surface_info(page_index as usize)
            .ok_or_else(|| EditorError::General {
                msg: format!("invalid page index: {page_index}"),
            })?;
        let buf_size = info.buffer_size;

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let mut pixels = vec![0u8; buf_size];
            let ok = inner
                .core
                .runtime_mut()
                .render_surface_into(page_index as usize, &mut pixels);
            if ok {
                let mut guard = handle.resources.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(PlatformBuffer::Desktop { ref mut pixel_data }) = *guard {
                    pixel_data.clear();
                    pixel_data.extend_from_slice(&pixels);
                }
            }
            Ok(ok)
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let mut pixels = vec![0u8; buf_size];
            let ok = inner
                .core
                .runtime_mut()
                .render_surface_into(page_index as usize, &mut pixels);
            // TODO: 네이티브 버퍼에 복사 (AHardwareBuffer / IOSurface)
            Ok(ok)
        }
    }

    pub fn render_drag_image(
        &self,
        visible_pages: Vec<u32>,
        page_idx: u32,
    ) -> Result<Option<DragImageData>, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let visible: Vec<usize> = visible_pages.into_iter().map(|p| p as usize).collect();
        Ok(inner
            .core
            .runtime_mut()
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
        let runtime = inner.core.runtime();
        Ok(runtime
            .pages()
            .get(page_idx as usize)
            .map_or(false, |page| {
                is_selection_hit(runtime.doc(), page, runtime.selection(), x, y)
            }))
    }

    pub fn is_cursor_hit(&self, page_idx: u32, x: f32, y: f32) -> Result<bool, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let runtime = inner.core.runtime();
        Ok(runtime
            .pages()
            .get(page_idx as usize)
            .map_or(false, |page| {
                is_cursor_hit(runtime.doc(), page, runtime.selection(), x, y)
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
        inner
            .core
            .runtime_mut()
            .export(export_mode)
            .map_err(EditorError::from)
    }

    pub fn import_updates(&self, data: Vec<u8>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .core
            .runtime_mut()
            .import_updates(&data)
            .map_err(|e| format!("Failed to import updates: {e}"))?;
        Ok(())
    }

    pub fn import_updates_batch(&self, updates: Vec<Vec<u8>>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .core
            .runtime_mut()
            .import_updates_batch(&updates)
            .map_err(|e| format!("Failed to import updates batch: {e}"))?;
        Ok(())
    }

    // ── Text & Clipboard ────────────────────────────────────────────────

    pub fn get_character_counts(&self) -> Result<CharacterCounts, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner.core.get_character_counts())
    }

    pub fn get_clipboard_data(&self) -> Result<Option<String>, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let data = inner.core.get_clipboard_data();
        match data {
            None => Ok(None),
            Some(clip) => {
                let json = serde_json::json!({ "html": clip.html, "text": clip.text });
                let json_str = serde_json::to_string(&json)
                    .map_err(|e| format!("Failed to serialize: {e}"))?;
                Ok(Some(json_str))
            }
        }
    }

    pub fn get_text_with_mappings(&self) -> Result<String, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let result = inner.core.get_text_with_mappings();
        serde_json::to_string(&result).map_err(|e| format!("Failed to serialize: {e}").into())
    }

    // ── Search ──────────────────────────────────────────────────────────

    pub fn perform_search(
        &self,
        query: String,
        match_whole_word: bool,
    ) -> Result<String, EditorError> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let results = inner.core.perform_search(&query, match_whole_word);
        serde_json::to_string(&results).map_err(|e| format!("Failed to serialize: {e}").into())
    }

    // ── Tracked Items ───────────────────────────────────────────────────

    pub fn set_tracked_items(&self, group: u32, items_json: String) -> Result<(), EditorError> {
        let items: Vec<RawTrackedItem> =
            serde_json::from_str(&items_json).map_err(|e| format!("Failed to parse items: {e}"))?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.core.runtime_mut().set_tracked_items(group, items);
        Ok(())
    }

    pub fn remove_tracked_items(&self, group: u32, ids_json: String) -> Result<(), EditorError> {
        let ids: Vec<String> =
            serde_json::from_str(&ids_json).map_err(|e| format!("Failed to parse ids: {e}"))?;
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.core.runtime_mut().remove_tracked_items(group, &ids);
        Ok(())
    }

    pub fn reveal_tracked_item(&self, group: u32, id: String) -> Result<bool, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(inner.core.runtime_mut().reveal_tracked_item(group, &id))
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
            .core
            .runtime_mut()
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
            .core
            .runtime_mut()
            .replace_text_in_blocks(&replacements)
            .map_err(|e| format!("Failed to replace: {e}"))?;
        Ok(())
    }

    pub fn insert_template_fragment(&self, snapshot: Vec<u8>) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .core
            .runtime_mut()
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
        inner
            .core
            .runtime_mut()
            .tracing
            .set_tracing(trace_id, parent_span_id);
        Ok(())
    }

    pub fn clear_tracing(&self) -> Result<(), EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.core.runtime_mut().tracing.clear_tracing();
        Ok(())
    }

    pub fn drain_traces(&self) -> Result<String, EditorError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let traces = inner.core.runtime_mut().tracing.drain();
        serde_json::to_string(&traces).map_err(|e| format!("Failed to serialize: {e}").into())
    }
}

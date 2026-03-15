use crate::global::{add_font_base, add_font_chunk, set_available_fonts};
use crate::global::{clear_text_replacement_rules, set_text_replacement_rules};
use crate::icu_data::{get_general_category_map, load_icu_data};
use crate::layout::query::{is_cursor_hit, is_selection_hit};
use crate::model::{
    CONTINUOUS_PAGE_MARGIN, Doc, DocExportMode, LayoutMode, Node, NodeId, ParagraphNode,
    TextMapping,
};
use crate::runtime::search::{SearchQuery, perform_search};
use crate::runtime::slate::{Slate, get_slate_offsets};
use crate::runtime::text_replacement::RawTextReplacementRule;
use crate::runtime::tracked_items::RawTrackedItem;
use crate::runtime::{Message, Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;
use icu_properties::props::GeneralCategory;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

#[cfg(target_os = "android")]
use jni::EnvUnowned;
#[cfg(target_os = "android")]
use jni::objects::{JByteBuffer, JClass};
#[cfg(target_os = "android")]
use jni::sys::jlong;
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

static PANIC_HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

pub type LogCallback = extern "C" fn(level: i32, message: *const c_char);
static LOG_CALLBACK: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

pub const LOG_LEVEL_DEBUG: i32 = 0;
pub const LOG_LEVEL_INFO: i32 = 1;
pub const LOG_LEVEL_WARN: i32 = 2;
pub const LOG_LEVEL_ERROR: i32 = 3;

pub fn native_log(level: i32, message: &str) {
    let callback = LOG_CALLBACK.load(Ordering::Relaxed);
    if callback.is_null() {
        match level {
            LOG_LEVEL_ERROR => eprintln!("[ERROR] {message}"),
            LOG_LEVEL_WARN => eprintln!("[WARN] {message}"),
            LOG_LEVEL_INFO => println!("[INFO] {message}"),
            LOG_LEVEL_DEBUG | _ => println!("[DEBUG] {message}"),
        }
        return;
    }

    if let Ok(c_message) = CString::new(message) {
        let callback: LogCallback = unsafe { std::mem::transmute(callback) };
        callback(level, c_message.as_ptr());
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    static PANIC_BACKTRACE: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn install_panic_hook() {
    if PANIC_HOOK_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::force_capture();
        PANIC_BACKTRACE.with(|bt| *bt.borrow_mut() = Some(backtrace.to_string()));
        prev_hook(info);
    }));
}

fn set_last_error(msg: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg.into()));
}

fn handle_panic(e: Box<dyn std::any::Any + Send>) {
    let msg = e
        .downcast_ref::<&str>()
        .map(|s| format!("panic: {s}"))
        .or_else(|| e.downcast_ref::<String>().map(|s| format!("panic: {s}")))
        .unwrap_or_else(|| "panic: unknown error".to_string());

    let full_msg = PANIC_BACKTRACE
        .with(|bt| bt.borrow_mut().take())
        .map(|bt| format!("{msg}\n\nBacktrace:\n{bt}"))
        .unwrap_or(msg);

    set_last_error(full_msg);
}

type FfiResult<T> = Result<T, String>;

trait IntoFfi {
    type Output;
    fn into_ffi(self) -> Self::Output;
}

impl IntoFfi for FfiResult<()> {
    type Output = i32;

    fn into_ffi(self) -> i32 {
        match self {
            Ok(()) => 0,
            Err(e) => {
                set_last_error(e);
                -1
            }
        }
    }
}

impl IntoFfi for FfiResult<i32> {
    type Output = i32;

    fn into_ffi(self) -> i32 {
        match self {
            Ok(code) => code,
            Err(e) => {
                set_last_error(e);
                -1
            }
        }
    }
}

impl<T> IntoFfi for FfiResult<*mut T> {
    type Output = *mut T;

    fn into_ffi(self) -> *mut T {
        match self {
            Ok(ptr) => ptr,
            Err(e) => {
                set_last_error(e);
                std::ptr::null_mut()
            }
        }
    }
}

impl IntoFfi for FfiResult<usize> {
    type Output = usize;

    fn into_ffi(self) -> usize {
        match self {
            Ok(val) => val,
            Err(e) => {
                set_last_error(e);
                0
            }
        }
    }
}

macro_rules! ffi {
    ($body:expr, $default:expr) => {{
        install_panic_hook();
        match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(result) => result.into_ffi(),
            Err(e) => {
                handle_panic(e);
                $default
            }
        }
    }};
}

fn parse_cstr<'a>(ptr: *const c_char, name: &str) -> FfiResult<&'a str> {
    if ptr.is_null() {
        return Err(format!("{name} is null"));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| format!("{name} has invalid encoding"))
}

unsafe fn slice_from_raw<'a>(ptr: *const u8, len: usize, name: &str) -> FfiResult<&'a [u8]> {
    if ptr.is_null() {
        return Err(format!("{name} is null"));
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_set_log_callback(callback: Option<LogCallback>) {
    LOG_CALLBACK.store(
        callback.map_or(std::ptr::null_mut(), |cb| cb as *mut ()),
        Ordering::Relaxed,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_alloc(size: usize) -> *mut u8 {
    install_panic_hook();
    catch_unwind(AssertUnwindSafe(|| {
        let mut buf: Vec<u8> = Vec::with_capacity(size);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_free(ptr: *mut u8, len: usize, capacity: usize) {
    if !ptr.is_null() {
        unsafe { drop(Vec::from_raw_parts(ptr, len, capacity)) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_last_error() -> *mut c_char {
    LAST_ERROR.with(|e| {
        e.borrow()
            .as_deref()
            .and_then(|msg| CString::new(msg).ok())
            .map_or(std::ptr::null_mut(), CString::into_raw)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_clear_last_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
}

struct ApplicationInner;

#[repr(C)]
pub struct EditorApplication {
    _private: [u8; 0],
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_validate_regex(pattern: *const c_char) -> i32 {
    ffi!(
        {
            let pattern = parse_cstr(pattern, "pattern")?;
            let anchored = format!("(?:{pattern})$");
            Ok(if fancy_regex::Regex::new(&anchored).is_ok() {
                1
            } else {
                0
            })
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_new() -> *mut EditorApplication {
    ffi!(
        Ok(Box::into_raw(Box::new(ApplicationInner)) as *mut EditorApplication),
        std::ptr::null_mut()
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_free(app: *mut EditorApplication) {
    if !app.is_null() {
        unsafe { drop(Box::from_raw(app as *mut ApplicationInner)) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_load_icu_data(
    _app: *mut EditorApplication,
    data: *const u8,
    len: usize,
) -> i32 {
    ffi!(
        {
            let data = unsafe { slice_from_raw(data, len, "ICU data")? };
            load_icu_data(data)
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_add_font_base(
    _app: *mut EditorApplication,
    family: *const c_char,
    weight: u16,
    data: *const u8,
    data_len: usize,
) -> i32 {
    ffi!(
        {
            let family = parse_cstr(family, "Font family")?;
            let data = unsafe { slice_from_raw(data, data_len, "Font data")? };
            add_font_base(family, weight, data);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_add_font_chunk(
    _app: *mut EditorApplication,
    family: *const c_char,
    weight: u16,
    data: *const u8,
    data_len: usize,
) -> i32 {
    ffi!(
        {
            let family = parse_cstr(family, "Font family")?;
            let data = unsafe { slice_from_raw(data, data_len, "Font data")? };
            add_font_chunk(family, weight, data);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_set_available_fonts(
    _app: *mut EditorApplication,
    fonts_json: *const c_char,
) -> i32 {
    ffi!(
        {
            let json = parse_cstr(fonts_json, "Fonts JSON")?;
            let fonts: std::collections::HashMap<String, Vec<u16>> =
                serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
            set_available_fonts(fonts);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_set_text_replacement_rules(
    _app: *mut EditorApplication,
    rules_json: *const c_char,
) -> i32 {
    ffi!(
        {
            let json = parse_cstr(rules_json, "Rules JSON")?;
            let rules: Vec<RawTextReplacementRule> =
                serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
            set_text_replacement_rules(rules);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_clear_text_replacement_rules(
    _app: *mut EditorApplication,
) -> i32 {
    ffi!(
        {
            clear_text_replacement_rules();
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_create_editor(
    _app: *mut EditorApplication,
    scale_factor: f64,
    snapshot: *const u8,
    snapshot_len: usize,
) -> *mut EditorHandle {
    ffi!(
        {
            let editor = if !snapshot.is_null() && snapshot_len > 0 {
                let data = unsafe { std::slice::from_raw_parts(snapshot, snapshot_len) };
                EditorInner::with_snapshot(scale_factor, data.to_vec())?
            } else {
                EditorInner::new(scale_factor)
            };
            Ok(Box::into_raw(Box::new(editor)) as *mut EditorHandle)
        },
        std::ptr::null_mut()
    )
}

struct EditorInner {
    runtime: Runtime,
}

impl EditorInner {
    fn new(scale_factor: f64) -> Self {
        let doc = Rc::new(Doc::new());
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

    fn with_snapshot(scale_factor: f64, snapshot: Vec<u8>) -> FfiResult<Self> {
        let doc = Rc::new(Doc::from_snapshot(snapshot).map_err(|e| e.to_string())?);
        let width = Self::get_width(&doc);
        Ok(Self::create(scale_factor, doc, NodeId::ROOT, width))
    }

    fn create(scale_factor: f64, doc: Rc<Doc>, cursor_node: NodeId, width: f32) -> Self {
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

#[repr(C)]
pub struct EditorHandle {
    _private: [u8; 0],
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_handle_free(editor: *mut EditorHandle) {
    if !editor.is_null() {
        unsafe { drop(Box::from_raw(editor as *mut EditorInner)) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_dispatch(editor: *mut EditorHandle, message_json: *const c_char) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let json = parse_cstr(message_json, "Message JSON")?;
            let message: Message =
                serde_json::from_str(json).map_err(|e| format!("Failed to parse message: {e}"))?;

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.enqueue_message(message);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_tick(editor: *mut EditorHandle) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }
            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.tick();
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_set_tracing(
    editor: *mut EditorHandle,
    trace_id: *const c_char,
    parent_span_id: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }
            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let trace_id_str = unsafe { std::ffi::CStr::from_ptr(trace_id) }
                .to_str()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let parent_span_id_str = unsafe { std::ffi::CStr::from_ptr(parent_span_id) }
                .to_str()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let trace_id = opentelemetry::trace::TraceId::from_hex(trace_id_str)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let parent_span_id = opentelemetry::trace::SpanId::from_hex(parent_span_id_str)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            editor.runtime.tracing.set_tracing(trace_id, parent_span_id);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_clear_tracing(editor: *mut EditorHandle) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }
            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.tracing.clear_tracing();
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_drain_traces(editor: *mut EditorHandle) -> *mut c_char {
    install_panic_hook();
    if editor.is_null() {
        return std::ptr::null_mut();
    }
    let editor = unsafe { &mut *(editor as *mut EditorInner) };
    let traces = editor.runtime.tracing.drain();
    match serde_json::to_string(&traces) {
        Ok(json) => match std::ffi::CString::new(json) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_slate_ptr(editor: *mut EditorHandle) -> *const u8 {
    install_panic_hook();
    if editor.is_null() {
        return std::ptr::null();
    }
    let editor = unsafe { &*(editor as *const EditorInner) };
    &editor.runtime.slate as *const _ as *const u8
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_slate_len(_editor: *mut EditorHandle) -> u32 {
    std::mem::size_of::<Slate>() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_slab_ptr(editor: *mut EditorHandle) -> *const u8 {
    install_panic_hook();
    if editor.is_null() {
        return std::ptr::null();
    }
    let editor = unsafe { &*(editor as *const EditorInner) };
    editor.runtime.slab.data.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_slab_len(editor: *mut EditorHandle) -> u32 {
    install_panic_hook();
    if editor.is_null() {
        return 0;
    }
    let editor = unsafe { &*(editor as *const EditorInner) };
    editor.runtime.slab.len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_slate_offsets() -> *mut c_char {
    install_panic_hook();
    match catch_unwind(AssertUnwindSafe(|| {
        let offsets = get_slate_offsets();
        let json = serde_json::to_string(&offsets).unwrap_or_default();
        CString::new(json)
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    })) {
        Ok(ptr) => ptr,
        Err(e) => {
            handle_panic(e);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_flush(editor: *mut EditorHandle) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.flush();
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_page_count(editor: *mut EditorHandle) -> usize {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }
            let editor = unsafe { &*(editor as *const EditorInner) };
            Ok(editor.runtime.pages().len())
        },
        0
    )
}

#[repr(C)]
pub struct RenderInfo {
    pub width: u32,
    pub height: u32,
    pub buffer_size: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_render_info(
    editor: *mut EditorHandle,
    page_index: usize,
    out_info: *mut RenderInfo,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || out_info.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let info = editor
                .runtime
                .get_render_info(page_index)
                .ok_or("Page not found")?;

            unsafe {
                (*out_info).width = info.width as u32;
                (*out_info).height = info.height as u32;
                (*out_info).buffer_size = info.buffer_size;
            }
            Ok(())
        },
        -1
    )
}

pub const PIXEL_FORMAT_RGBA: i32 = 0;
pub const PIXEL_FORMAT_BGRA: i32 = 1;

thread_local! {
    static RENDER_TEMP_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_render_page_to(
    editor: *mut EditorHandle,
    page_index: usize,
    dst: *mut u8,
    dst_stride: usize,
    dst_width: usize,
    dst_height: usize,
    format: i32,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || dst.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let info = editor
                .runtime
                .get_render_info(page_index)
                .ok_or("Page not found")?;

            let width = info.width as usize;
            let height = info.height as usize;
            let tight_stride = width * 4;

            if dst_width != width || dst_height != height || dst_stride < tight_stride {
                return Ok(1);
            }

            let convert_to_bgra = format == PIXEL_FORMAT_BGRA;

            if dst_stride == tight_stride {
                let dst_slice =
                    unsafe { std::slice::from_raw_parts_mut(dst, tight_stride * height) };
                if !editor.runtime.render_page_to(page_index, dst_slice) {
                    return Err("Render failed".into());
                }
                if convert_to_bgra {
                    rgba_to_bgra_fast(dst_slice);
                }
            } else {
                let render_success = RENDER_TEMP_BUFFER.with(|buf| {
                    let mut temp_buf = buf.borrow_mut();
                    let required_size = tight_stride * height;
                    if temp_buf.len() < required_size {
                        temp_buf.resize(required_size, 0);
                    }

                    if !editor
                        .runtime
                        .render_page_to(page_index, &mut temp_buf[..required_size])
                    {
                        return false;
                    }

                    for row in 0..height {
                        let src_offset = row * tight_stride;
                        let dst_offset = row * dst_stride;
                        let dst_row = unsafe {
                            std::slice::from_raw_parts_mut(dst.add(dst_offset), tight_stride)
                        };
                        let src_row = &mut temp_buf[src_offset..src_offset + tight_stride];

                        if convert_to_bgra {
                            rgba_to_bgra_fast(src_row);
                        }
                        dst_row.copy_from_slice(src_row);
                    }
                    true
                });
                if !render_success {
                    return Err("Render failed".into());
                }
            }
            Ok(0)
        },
        -1
    )
}

#[inline]
fn rgba_to_bgra_fast(data: &mut [u8]) {
    #[cfg(target_arch = "aarch64")]
    {
        rgba_to_bgra_neon(data);
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        for chunk in data.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn rgba_to_bgra_neon(data: &mut [u8]) {
    let len = data.len();
    let mut i = 0;

    unsafe {
        while i + 64 <= len {
            let ptr = data.as_mut_ptr().add(i);

            let v0 = vld4q_u8(ptr);
            let swapped = uint8x16x4_t(v0.2, v0.1, v0.0, v0.3);
            vst4q_u8(ptr, swapped);

            i += 64;
        }

        while i + 32 <= len {
            let ptr = data.as_mut_ptr().add(i);

            let v0 = vld4_u8(ptr);
            let swapped = uint8x8x4_t(v0.2, v0.1, v0.0, v0.3);
            vst4_u8(ptr, swapped);

            i += 32;
        }
    }

    while i + 4 <= len {
        data.swap(i, i + 2);
        i += 4;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_is_selection_hit(
    editor: *mut EditorHandle,
    page_idx: usize,
    x: f32,
    y: f32,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &*(editor as *const EditorInner) };
            Ok(if let Some(page) = editor.runtime.pages().get(page_idx) {
                if is_selection_hit(editor.runtime.doc(), page, editor.runtime.selection(), x, y) {
                    1
                } else {
                    0
                }
            } else {
                0
            })
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_is_cursor_hit(
    editor: *mut EditorHandle,
    page_idx: usize,
    x: f32,
    y: f32,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &*(editor as *const EditorInner) };
            Ok(if let Some(page) = editor.runtime.pages().get(page_idx) {
                if is_cursor_hit(editor.runtime.doc(), page, editor.runtime.selection(), x, y) {
                    1
                } else {
                    0
                }
            } else {
                0
            })
        },
        -1
    )
}

#[repr(C)]
pub struct DragImageResult {
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
    pub pixels: *mut u8,
    pub len: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_render_drag_image(
    editor: *mut EditorHandle,
    visible_pages: *const usize,
    visible_pages_len: usize,
    page_idx: usize,
    out_result: *mut DragImageResult,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || visible_pages.is_null() || out_result.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let visible_pages =
                unsafe { std::slice::from_raw_parts(visible_pages, visible_pages_len) };
            let result = editor
                .runtime
                .render_drag_image(visible_pages, page_idx)
                .ok_or("Failed to render drag image")?;

            let data_slice = unsafe { std::slice::from_raw_parts(result.ptr(), result.len()) };
            let mut pixels = data_slice.to_vec();
            pixels.shrink_to_fit();
            let len = pixels.len();
            let ptr = pixels.as_mut_ptr();
            std::mem::forget(pixels);

            unsafe {
                (*out_result).width = result.width as u32;
                (*out_result).height = result.height as u32;
                (*out_result).offset_x = result.offset_x;
                (*out_result).offset_y = result.offset_y;
                (*out_result).scale_factor = result.scale_factor;
                (*out_result).pixels = ptr;
                (*out_result).len = len;
            }
            Ok(())
        },
        -1
    )
}

#[repr(C)]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_character_counts(
    editor: *mut EditorHandle,
    out_counts: *mut CharacterCounts,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || out_counts.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let doc_text = editor.runtime.get_cached_plain_text();
            let selection_text = {
                let state = editor.runtime.state();
                state.selection.to_plain_text(&state.doc)
            };

            let doc_counts = count_all(&doc_text);
            let sel_counts = count_all(&selection_text);

            unsafe {
                (*out_counts).doc_with_whitespace = doc_counts.0;
                (*out_counts).doc_without_whitespace = doc_counts.1;
                (*out_counts).doc_without_whitespace_and_punctuation = doc_counts.2;
                (*out_counts).selection_with_whitespace = sel_counts.0;
                (*out_counts).selection_without_whitespace = sel_counts.1;
                (*out_counts).selection_without_whitespace_and_punctuation = sel_counts.2;
            }
            Ok(())
        },
        -1
    )
}

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

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_clipboard_data(editor: *mut EditorHandle) -> *mut c_char {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &*(editor as *const EditorInner) };
            let state = editor.runtime.state();
            if state.selection.is_collapsed() {
                return Ok(std::ptr::null_mut());
            }

            let fragment = match state.selection.extract_fragment(&state.doc) {
                Ok(f) => f,
                Err(_) => return Ok(std::ptr::null_mut()),
            };

            if fragment.is_empty() {
                return Ok(std::ptr::null_mut());
            }

            let html = fragment.to_html();
            let text = fragment.to_plain_text();

            let json = serde_json::json!({
                "html": html,
                "text": text,
            });
            let json_str =
                serde_json::to_string(&json).map_err(|e| format!("Failed to serialize: {e}"))?;
            let c_str = CString::new(json_str).map_err(|_| "Invalid string")?;
            Ok(c_str.into_raw())
        },
        std::ptr::null_mut()
    )
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_co_typie_editortexture_EditorTexture_nativeGetDirectBufferAddress(
    mut env: EnvUnowned,
    _class: JClass,
    buffer: JByteBuffer,
) -> jlong {
    match env
        .with_env(|env| -> jni::errors::Result<_> {
            let ptr = env.get_direct_buffer_address(&buffer)?;
            Ok(ptr as jlong)
        })
        .into_outcome()
    {
        jni::Outcome::Ok(v) => v,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_export(
    editor: *mut EditorHandle,
    mode: i32,
    version: *const u8,
    version_len: usize,
    out_len: *mut usize,
) -> *mut u8 {
    ffi!(
        {
            if editor.is_null() || out_len.is_null() {
                return Err("Invalid parameters".into());
            }

            let export_mode = match mode {
                0 => DocExportMode::Snapshot,
                1 => DocExportMode::Version,
                2 => DocExportMode::AllUpdates,
                3 => {
                    let ver = unsafe { slice_from_raw(version, version_len, "version")? };
                    DocExportMode::UpdatesFrom {
                        version: ver.to_vec(),
                    }
                }
                _ => return Err(format!("Invalid export mode: {mode}")),
            };

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let data = editor
                .runtime
                .export(export_mode)
                .map_err(|e| format!("Failed to export: {e}"))?;

            let len = data.len();
            let ptr = Box::into_raw(data.into_boxed_slice()) as *mut u8;
            unsafe { *out_len = len };
            Ok(ptr)
        },
        std::ptr::null_mut()
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_import_updates(
    editor: *mut EditorHandle,
    updates: *const u8,
    len: usize,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || updates.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let data = unsafe { std::slice::from_raw_parts(updates, len) };

            editor
                .runtime
                .import_updates(data)
                .map_err(|e| format!("Failed to import updates: {e}"))?;

            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_import_updates_batch(
    editor: *mut EditorHandle,
    updates_ptrs: *const *const u8,
    updates_lens: *const usize,
    count: usize,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || updates_ptrs.is_null() || updates_lens.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let ptrs = unsafe { std::slice::from_raw_parts(updates_ptrs, count) };
            let lens = unsafe { std::slice::from_raw_parts(updates_lens, count) };

            let batch: Vec<Vec<u8>> = ptrs
                .iter()
                .zip(lens.iter())
                .map(|(&ptr, &len)| unsafe { std::slice::from_raw_parts(ptr, len).to_vec() })
                .collect();

            editor
                .runtime
                .import_updates_batch(&batch)
                .map_err(|e| format!("Failed to import updates batch: {e}"))?;

            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_get_text_with_mappings(editor: *mut EditorHandle) -> *mut c_char {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &*(editor as *const EditorInner) };

            #[derive(serde::Serialize)]
            #[serde(rename_all = "camelCase")]
            struct TextWithMappingsResult {
                text: String,
                mappings: Vec<TextMapping>,
            }

            let (text, mappings) = editor.runtime.doc().to_text_with_mappings();
            let result = TextWithMappingsResult { text, mappings };
            let json_str =
                serde_json::to_string(&result).map_err(|e| format!("Failed to serialize: {e}"))?;
            let c_str = CString::new(json_str).map_err(|_| "Invalid string")?;
            Ok(c_str.into_raw())
        },
        std::ptr::null_mut()
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_set_tracked_items(
    editor: *mut EditorHandle,
    group: u32,
    items_json: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let json_str = parse_cstr(items_json, "items_json")?;
            let items: Vec<RawTrackedItem> = serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to parse items: {e}"))?;

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.set_tracked_items(group, items);

            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_remove_tracked_items(
    editor: *mut EditorHandle,
    group: u32,
    ids_json: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let json_str = parse_cstr(ids_json, "ids_json")?;
            let ids: Vec<String> =
                serde_json::from_str(json_str).map_err(|e| format!("Failed to parse ids: {e}"))?;

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.remove_tracked_items(group, &ids);

            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_perform_search(
    editor: *mut EditorHandle,
    query: *const c_char,
    match_whole_word: i32,
) -> *mut c_char {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let query_str = parse_cstr(query, "query")?;

            let editor = unsafe { &*(editor as *const EditorInner) };
            let search_query = SearchQuery::new(query_str.to_string(), match_whole_word != 0);
            let matches = perform_search(editor.runtime.doc(), &search_query);

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

            let json_str =
                serde_json::to_string(&results).map_err(|e| format!("Failed to serialize: {e}"))?;
            let c_str = CString::new(json_str).map_err(|_| "Invalid string")?;
            Ok(c_str.into_raw())
        },
        std::ptr::null_mut()
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_reveal_tracked_item(
    editor: *mut EditorHandle,
    group: u32,
    id: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let id = parse_cstr(id, "id")?;
            let editor = unsafe { &mut *(editor as *mut EditorInner) };

            Ok(if editor.runtime.reveal_tracked_item(group, id) {
                1
            } else {
                0
            })
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_replace_text_in_block(
    editor: *mut EditorHandle,
    block_id: *const c_char,
    start_offset: usize,
    end_offset: usize,
    replacement: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let block_id_str = parse_cstr(block_id, "block_id")?;
            let replacement_str = parse_cstr(replacement, "replacement")?;

            let node_id =
                NodeId::from_string(block_id_str).ok_or_else(|| "Invalid block_id".to_string())?;

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let success = editor
                .runtime
                .replace_text_in_block(node_id, start_offset, end_offset, replacement_str)
                .is_ok();

            Ok(if success { 1 } else { 0 })
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_replace_text_in_blocks(
    editor: *mut EditorHandle,
    items_json: *const c_char,
) -> i32 {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let items_str = parse_cstr(items_json, "items_json")?;

            let entries: Vec<(String, usize, usize, String)> = serde_json::from_str(items_str)
                .map_err(|e| format!("Failed to parse items: {e}"))?;

            let replacements: Vec<_> = entries
                .iter()
                .filter_map(|(node_id, start, end, replacement)| {
                    NodeId::from_string(node_id).map(|id| (id, *start, *end, replacement.as_str()))
                })
                .collect();

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor
                .runtime
                .replace_text_in_blocks(&replacements)
                .map_err(|e| format!("Failed to replace: {e}"))?;

            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_insert_template_fragment(
    editor: *mut EditorHandle,
    snapshot: *const u8,
    snapshot_len: usize,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || snapshot.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let data = unsafe { std::slice::from_raw_parts(snapshot, snapshot_len) };

            editor
                .runtime
                .insert_template_fragment(data.to_vec())
                .map_err(|e| format!("Failed to insert template: {e}"))?;

            Ok(())
        },
        -1
    )
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_co_typie_editortexture_EditorTexture_nativeRenderPageTo(
    _env: EnvUnowned,
    _class: JClass,
    editor_ptr: jlong,
    page_index: jlong,
    dst_ptr: jlong,
    dst_stride: jlong,
    dst_width: jlong,
    dst_height: jlong,
    format: jlong,
) -> jlong {
    let editor = editor_ptr as *mut EditorHandle;
    let page_index = page_index as usize;
    let dst = dst_ptr as *mut u8;
    let dst_stride = dst_stride as usize;
    let dst_width = dst_width as usize;
    let dst_height = dst_height as usize;
    let format = format as i32;

    editor_render_page_to(
        editor, page_index, dst, dst_stride, dst_width, dst_height, format,
    ) as jlong
}

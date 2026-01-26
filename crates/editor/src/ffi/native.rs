use crate::global::register_font_family;
use crate::model::{Doc, LayoutMode, Node, NodeId, ParagraphNode};
use crate::runtime::{Runtime, State};
use crate::state::{Position, Selection};
use crate::types::Affinity;
use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

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
            _ => println!("[DEBUG] {message}"),
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

#[repr(C)]
pub struct RenderResult {
    pub ptr: *mut u8,
    pub len: usize,
    pub width: u32,
    pub height: u32,
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
            crate::icu_data::load_icu_data(data)
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_application_register_font(
    _app: *mut EditorApplication,
    name: *const c_char,
    weight: u16,
    data: *const u8,
    data_len: usize,
) -> i32 {
    ffi!(
        {
            let name = parse_cstr(name, "Font name")?;
            let data = unsafe { slice_from_raw(data, data_len, "Font data")? };
            register_font_family(name, weight, data);
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
            let fonts =
                serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {e}"))?;
            crate::global::set_available_fonts(fonts);
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
                EditorInner::with_snapshot(scale_factor, data.to_vec())
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

        let root = doc.node(NodeId::ROOT).unwrap();
        let paragraph_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        Self::create(scale_factor, doc, paragraph_id, width)
    }

    fn with_snapshot(scale_factor: f64, snapshot: Vec<u8>) -> Self {
        let doc = Rc::new(Doc::from_snapshot(snapshot));
        let width = Self::get_width(&doc);
        Self::create(scale_factor, doc, NodeId::ROOT, width)
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
            LayoutMode::Continuous { max_width, .. } => max_width,
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
            let message: crate::runtime::Message =
                serde_json::from_str(json).map_err(|e| format!("Failed to parse message: {e}"))?;

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            editor.runtime.enqueue_message(message);
            Ok(())
        },
        -1
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn editor_tick(editor: *mut EditorHandle) -> *mut c_char {
    ffi!(
        {
            if editor.is_null() {
                return Err("Editor is null".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let cmds = editor.runtime.tick();
            if cmds.is_empty() {
                return Ok(std::ptr::null_mut());
            }
            let json = serde_json::to_string(&cmds).map_err(|e| format!("Failed to serialize: {e}"))?;
            let c_str = CString::new(json).map_err(|_| "Invalid string")?;
            Ok(c_str.into_raw())
        },
        std::ptr::null_mut()
    )
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

#[unsafe(no_mangle)]
pub extern "C" fn editor_render_page(
    editor: *mut EditorHandle,
    page_index: usize,
    out_result: *mut RenderResult,
) -> i32 {
    ffi!(
        {
            if editor.is_null() || out_result.is_null() {
                return Err("Invalid parameters".into());
            }

            let editor = unsafe { &mut *(editor as *mut EditorInner) };
            let result = editor
                .runtime
                .render_page(page_index)
                .ok_or("Page not found")?;

            unsafe {
                (*out_result).ptr = result.ptr as *mut u8;
                (*out_result).len = result.len;
                (*out_result).width = result.width as u32;
                (*out_result).height = result.height as u32;
            }
            Ok(())
        },
        -1
    )
}

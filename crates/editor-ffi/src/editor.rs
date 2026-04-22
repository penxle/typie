#[cfg(not(feature = "wasm-server"))]
use hashbrown::HashMap;
use std::sync::Mutex;

#[cfg(not(feature = "wasm-server"))]
use crate::platform::{PlatformHandle, SurfaceHandle};
use crate::prelude::*;

struct EditorInner {
    editor: editor_core::Editor,
    #[cfg(not(feature = "wasm-server"))]
    surfaces: HashMap<u32, SurfaceHandle>,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<EditorInner>,
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
impl Editor {
    pub fn enqueue(&self, message: Complex<editor_core::Message>) -> EditorResult<()> {
        self.with_inner(|inner| {
            inner.editor.enqueue(message.from_ffi()?);
            Ok(())
        })
    }

    pub fn tick(&self) -> EditorResult<Vec<Complex<editor_core::EditorEvent>>> {
        self.with_inner(|inner| Ok(inner.editor.tick()?.into_ffi()?))
    }

    pub fn cursor(&self) -> EditorResult<Option<Complex<editor_view::CursorRect>>> {
        self.with_inner(|inner| {
            let selection = inner.editor.state().selection;
            if selection.is_collapsed() {
                // TODO(editor-parity): collapsed selection의 head/composition bounds도 FFI로
                // 노출해야 한다. KMP는 지금 cursor rect만 받아서 실제 selection head 표시
                // 높이보다 작은 값으로 typewriter 하단 여백과 keep-visible(cursor guard)를
                // 계산하고 있다.
                Ok(inner
                    .editor
                    .view()
                    .cursor_rect(&selection.head)
                    .into_ffi()?)
            } else {
                Ok(None)
            }
        })
    }

    pub fn selection(&self) -> EditorResult<Complex<editor_state::Selection>> {
        self.with_inner(|inner| Ok(inner.editor.state().selection.into_ffi()?))
    }

    pub fn inspect_state(
        &self,
        options: Option<Complex<editor_introspection::InspectStateOptions>>,
    ) -> EditorResult<String> {
        self.with_inner(|inner| {
            let options = match options {
                Some(o) => o.from_ffi()?,
                None => editor_introspection::InspectStateOptions::default(),
            };
            Ok(editor_introspection::inspect_state(
                inner.editor.state(),
                &options,
            ))
        })
    }

    pub fn inspect_state_as_macro(&self) -> EditorResult<String> {
        self.with_inner(|inner| {
            Ok(editor_introspection::inspect_state_as_macro(
                inner.editor.state(),
            ))
        })
    }

    pub fn page_sizes(&self) -> EditorResult<Vec<Complex<editor_common::Size>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .view()
                .pages()
                .iter()
                .map(|p| p.size)
                .collect::<Vec<_>>()
                .into_ffi()?)
        })
    }

    pub fn ime(
        &self,
        before_limit: usize,
        after_limit: usize,
    ) -> EditorResult<Complex<editor_core::Ime>> {
        self.with_inner(|inner| Ok(inner.editor.ime(before_limit, after_limit)?.into_ffi()?))
    }
}

#[cfg(not(feature = "wasm-server"))]
#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm-browser", editor_macros::ffi_export(wasm))]
impl Editor {
    pub fn attach_surface(
        &self,
        page: u32,
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> EditorResult<()> {
        let surface = SurfaceHandle::new(handle, width, height, scale_factor)?;
        self.with_inner(|inner| {
            inner.surfaces.insert(page, surface);
            Ok(())
        })
    }

    pub fn detach_surface(&self, page: u32) -> EditorResult<()> {
        self.with_inner(|inner| {
            inner.surfaces.remove(&page);
            Ok(())
        })
    }

    pub fn resize_surface(
        &self,
        page: u32,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> EditorResult<()> {
        self.with_inner(|inner| {
            if let Some(surface) = inner.surfaces.get_mut(&page) {
                surface.resize(width, height, scale_factor);
            }
            Ok(())
        })
    }

    pub fn render_surface(&self, page: u32) -> EditorResult<()> {
        self.with_inner(|inner| {
            if let Some(surface) = inner.surfaces.get_mut(&page) {
                let scale_factor = surface.scale_factor() as f32;
                inner.editor.render_page(page, surface.sink(), scale_factor);
                surface.present();
            }
            Ok(())
        })
    }
}

#[cfg(feature = "wasm-server")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl Editor {
    pub fn render_page_to_buffer(
        &self,
        page: u32,
        width: u32,
        height: u32,
    ) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let mut backend = editor_renderer::RenderBackend::new_cpu(width as u16, height as u16);
            inner.editor.render_page(page, backend.sink(), 1.0);

            let mut buf = vec![0u8; (width * height * 4) as usize];
            match &mut backend {
                editor_renderer::RenderBackend::Cpu(sink) => {
                    sink.flush_to(&mut buf);
                }
            }

            Ok(buf)
        })
    }
}

impl Editor {
    pub(crate) fn new(core: editor_core::Editor) -> Self {
        Self {
            inner: Mutex::new(EditorInner {
                editor: core,
                #[cfg(not(feature = "wasm-server"))]
                surfaces: HashMap::new(),
            }),
        }
    }

    fn with_inner<F, R>(&self, f: F) -> EditorResult<R>
    where
        F: FnOnce(&mut EditorInner) -> EditorResult<R>,
    {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        f(&mut inner)
    }
}

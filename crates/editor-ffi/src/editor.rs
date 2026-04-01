use hashbrown::HashMap;
use std::sync::Mutex;

use crate::backend::BackendMode;
use crate::convert::{FromFfi, IntoFfi};
use crate::platform::{PlatformHandle, SurfaceHandle};
use crate::prelude::*;
use crate::types::*;

struct EditorInner {
    editor: editor_core::Editor,
    surfaces: HashMap<u32, SurfaceHandle>,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<EditorInner>,
    backend: BackendMode,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl Editor {
    pub fn enqueue(&self, message: Complex<Message>) -> EditorResult<()> {
        let message = message.from_ffi()?;
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        inner.editor.enqueue(message);
        Ok(())
    }

    pub fn tick(&self) -> EditorResult<Vec<Complex<EditorEvent>>> {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        Ok(inner.editor.tick().into_ffi()?)
    }

    pub fn selection(&self) -> EditorResult<Complex<Selection>> {
        let inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        Ok(inner.editor.state().selection.into_ffi()?)
    }

    pub fn attach_surface(
        &self,
        page: u32,
        handle: PlatformHandle,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> EditorResult<()> {
        let surface = SurfaceHandle::new(&self.backend, handle, width, height, scale_factor)?;
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        inner.surfaces.insert(page, surface);
        Ok(())
    }

    pub fn detach_surface(&self, page: u32) -> EditorResult<()> {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        inner.surfaces.remove(&page);
        Ok(())
    }

    pub fn resize_surface(
        &self,
        page: u32,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> EditorResult<()> {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        if let Some(surface) = inner.surfaces.get_mut(&page) {
            surface.resize(width, height, scale_factor);
        }
        Ok(())
    }

    pub fn render_surface(&self, page: u32) -> EditorResult<()> {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        let EditorInner { editor, surfaces } = &mut *inner;
        if let Some(surface) = surfaces.get_mut(&page) {
            editor.render_page(page, surface.sink());
            surface.present();
        }
        Ok(())
    }
}

impl Editor {
    pub(crate) fn new(core: editor_core::Editor, backend: BackendMode) -> Self {
        Self {
            inner: Mutex::new(EditorInner {
                editor: core,
                surfaces: HashMap::new(),
            }),
            backend,
        }
    }
}

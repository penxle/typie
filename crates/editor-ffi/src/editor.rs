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

    pub fn cursor(&self) -> EditorResult<Option<Complex<editor_view::CursorMetrics>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let selection = state.selection;
            if selection.is_collapsed() {
                Ok(inner
                    .editor
                    .view()
                    .cursor_metrics(&state.doc, &selection.head)
                    .into_ffi()?)
            } else {
                Ok(None)
            }
        })
    }

    pub fn selection(&self) -> EditorResult<Complex<editor_state::Selection>> {
        self.with_inner(|inner| Ok(inner.editor.state().selection.into_ffi()?))
    }

    pub fn root_attrs(&self) -> EditorResult<Complex<editor_model::PlainRootNode>> {
        self.with_inner(|inner| {
            let doc = &inner.editor.state().doc;
            let entry = doc
                .get_entry(editor_model::NodeId::ROOT)
                .expect("root entry must exist");
            match &entry.node.to_plain() {
                editor_model::PlainNode::Root(r) => Ok(r.clone().into_ffi()?),
                _ => unreachable!("root entry must be Root"),
            }
        })
    }

    pub fn modifier_state(&self) -> EditorResult<Complex<editor_model::ModifierState>> {
        self.with_inner(|inner| Ok(inner.editor.modifier_state().into_ffi()?))
    }

    pub fn block_state(&self) -> EditorResult<Complex<editor_core::BlockState>> {
        self.with_inner(|inner| Ok(inner.editor.block_state().into_ffi()?))
    }

    pub fn interactive_hit_test(
        &self,
        page: u32,
        x: f32,
        y: f32,
    ) -> EditorResult<Option<Complex<editor_view::InteractiveHit>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .interactive_hit_test(page as usize, x, y)
                .into_ffi()?)
        })
    }

    pub fn pointer_style(
        &self,
        page: u32,
        x: f32,
        y: f32,
        read_only: bool,
    ) -> EditorResult<Complex<editor_view::PointerStyle>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .pointer_style(page as usize, x, y, read_only)
                .unwrap_or(editor_view::PointerStyle::Default)
                .into_ffi()?)
        })
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

    pub fn external_elements(&self) -> EditorResult<Vec<Complex<editor_view::ExternalElement>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .view()
                .external_elements(&inner.editor.state().doc, &inner.editor.state().selection)
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

    pub fn receive_remote_changeset(&self, payload: Vec<u8>) -> EditorResult<()> {
        self.with_inner(|inner| {
            let css: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
                editor_crdt::wire::decode(&payload[..])
                    .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            for changeset in css {
                inner.editor.receive_remote_changeset(changeset);
            }
            Ok(())
        })
    }

    pub fn local_changesets_since(&self, remote_heads_payload: Vec<u8>) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads_vec = editor_crdt::wire::decode_dots(&remote_heads_payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let css = inner.editor.local_changesets_since(&heads_set)?;
            let bytes = editor_crdt::wire::encode(&css)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }

    pub fn current_heads(&self) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads = inner.editor.current_heads();
            let bytes = editor_crdt::wire::encode_dots(&heads)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }
}

#[cfg(feature = "wasm")]
#[editor_macros::ffi_export(wasm)]
impl Editor {
    pub fn root_modifiers(&self) -> EditorResult<Vec<Complex<editor_model::Modifier>>> {
        self.with_inner(|inner| {
            let doc = &inner.editor.state().doc;
            let modifiers = doc
                .node(editor_model::NodeId::ROOT)
                .map(|n| n.explicit_modifiers().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            Ok(modifiers.into_ffi()?)
        })
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

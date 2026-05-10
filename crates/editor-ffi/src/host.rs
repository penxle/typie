use cfg_if::cfg_if;
use std::sync::{Arc, Mutex};

use crate::prelude::*;

fn init_logger() {
    cfg_if! {
        if #[cfg(feature = "wasm")] {
            console_log::init_with_level(log::Level::Debug)
                .expect("logger already initialized");
        } else if #[cfg(all(feature = "uniffi", target_os = "android"))] {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_filter(
                        env_filter::Builder::new()
                            .filter_level(log::LevelFilter::Warn)
                            .filter_module("editor", log::LevelFilter::Debug)
                            .build(),
                    )
                    .with_tag("editor"),
            );
        } else if #[cfg(feature = "uniffi")] {
            let _ = env_logger::builder()
                .filter_level(log::LevelFilter::Warn)
                .filter_module("editor", log::LevelFilter::Debug)
                .try_init();
        }
    }
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorHost {
    resource: Arc<Mutex<editor_resource::Resource>>,
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
impl EditorHost {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn create(icu_data: Vec<u8>) -> EditorResult<Owned<Self>> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        init_logger();

        let segmenters = Arc::new(editor_resource::TextSegmenters::from_icu_data(&icu_data)?);

        Ok(into_owned(Self {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new(segmenters))),
        }))
    }

    pub fn create_editor_from_doc(
        &self,
        doc: Complex<editor_model::PlainDoc>,
        selection: Complex<editor_state::Selection>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let plain: editor_model::PlainDoc = doc.from_ffi()?;
        let (doc, op_graph) = editor_model::Doc::from_plain(plain);
        let selection = selection.from_ffi()?;
        let state = editor_state::State::new(doc, op_graph, selection);

        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));

        Ok(into_owned(crate::editor::Editor::new(core)))
    }

    pub fn create_editor_from_graph(
        &self,
        changesets: Vec<u8>,
        selection: Complex<editor_state::Selection>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let css: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&changesets[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let selection = selection.from_ffi()?;
        let state = editor_state::State::from_changesets(css, selection)?;
        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(core)))
    }

    pub fn set_fonts(
        &self,
        families: Vec<Complex<editor_resource::FontFamily>>,
    ) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.set_fonts(families.from_ffi()?);
            Ok(())
        })
    }

    pub fn add_font_base(&self, family: String, weight: u16, data: Vec<u8>) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_font_base(&family, weight, &data)?;
            Ok(())
        })
    }

    pub fn add_font_chunk(
        &self,
        family: String,
        weight: u16,
        chunk_id: u16,
        data: Vec<u8>,
    ) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_font_chunk(&family, weight, chunk_id, &data)?;
            Ok(())
        })
    }
}

impl EditorHost {
    pub(crate) fn with_resource<F, R>(&self, f: F) -> EditorResult<R>
    where
        F: FnOnce(&mut editor_resource::Resource) -> EditorResult<R>,
    {
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        f(&mut resource)
    }
}

use cfg_if::cfg_if;
use std::sync::{Arc, Mutex};

use crate::backend::BackendMode;
use crate::convert::FromFfi;
use crate::prelude::*;

fn init_logger() {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            console_log::init_with_level(log::Level::Debug)
                .expect("logger already initialized");
        } else if #[cfg(target_os = "android")] {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Debug)
                    .with_tag("typie"),
            );
        } else if #[cfg(feature = "uniffi")] {
            let _ = env_logger::builder()
                .filter_level(log::LevelFilter::Debug)
                .try_init();
        }
    }
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorHost {
    resource: Arc<Mutex<editor_resource::Resource>>,
    backend: BackendMode,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl EditorHost {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub async fn create(kind: Complex<BackendKind>) -> EditorResult<Owned<Self>> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        init_logger();

        let kind = kind.from_ffi()?;
        let backend = match kind {
            editor_renderer::BackendKind::Cpu => BackendMode::Cpu,
            editor_renderer::BackendKind::Gpu => match editor_renderer::GpuDevice::new().await {
                Ok(device) => BackendMode::Gpu {
                    device: Arc::new(device),
                },
                Err(_) => BackendMode::Cpu,
            },
        };

        Ok(into_owned(Self {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new())),
            backend,
        }))
    }

    pub fn create_editor(
        &self,
        doc: String,
        viewport: Complex<Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let doc =
            serde_json::from_str(&doc).map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let selection = editor_state::Selection::collapsed(editor_state::Position::new(
            editor_model::NodeId::ROOT,
            0,
        ));

        let state = editor_state::State::new(doc, selection);
        let viewport = viewport.from_ffi()?;

        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(
            core,
            self.backend.clone(),
        )))
    }

    pub fn load_icu_data(&self, data: Vec<u8>) -> EditorResult<()> {
        let segmenters = editor_common::TextSegmenters::from_icu_data(&data)?;
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        resource.segmenters = Some(segmenters);
        Ok(())
    }

    pub fn load_font_base(&self, family: String, weight: u16, data: Vec<u8>) -> EditorResult<()> {
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        resource.add_font_base(&family, weight, &data)?;
        Ok(())
    }

    pub fn load_font_chunk(&self, family: String, weight: u16, data: Vec<u8>) -> EditorResult<()> {
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        resource.add_font_chunk(&family, weight, &data)?;
        Ok(())
    }

    pub fn set_fallback_font_families(&self, families: Vec<String>) -> EditorResult<()> {
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        resource.set_fallback_font_families(families)?;
        Ok(())
    }
}

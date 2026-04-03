use cfg_if::cfg_if;
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "wasm-server"))]
use crate::backend::BackendMode;
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
                    .with_tag("editor"),
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
    #[cfg(not(feature = "wasm-server"))]
    backend: BackendMode,
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl EditorHost {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub async fn create(
        kind: Option<Complex<editor_renderer::BackendKind>>,
    ) -> EditorResult<Owned<Self>> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        init_logger();

        #[cfg(not(feature = "wasm-server"))]
        let backend = {
            let kind = kind.from_ffi()?;
            match kind {
                Some(editor_renderer::BackendKind::Cpu) => BackendMode::Cpu,
                Some(editor_renderer::BackendKind::Gpu) | None => {
                    match editor_renderer::GpuDevice::new().await {
                        Ok(device) => BackendMode::Gpu {
                            device: Arc::new(device),
                        },
                        Err(_) => BackendMode::Cpu,
                    }
                }
            }
        };

        #[cfg(feature = "wasm-server")]
        let _ = kind;

        Ok(into_owned(Self {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new())),
            #[cfg(not(feature = "wasm-server"))]
            backend,
        }))
    }

    pub fn create_editor(
        &self,
        doc: Complex<editor_model::Doc>,
        selection: Complex<editor_state::Selection>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let doc = doc.from_ffi()?;
        let selection = selection.from_ffi()?;
        let state = editor_state::State::new(doc, selection);

        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));

        #[cfg(not(feature = "wasm-server"))]
        {
            Ok(into_owned(crate::editor::Editor::new(
                core,
                self.backend.clone(),
            )))
        }
        #[cfg(feature = "wasm-server")]
        {
            Ok(into_owned(crate::editor::Editor::new(core)))
        }
    }

    pub fn load_icu_data(&self, data: Vec<u8>) -> EditorResult<()> {
        let segmenters = editor_resource::TextSegmenters::from_icu_data(&data)?;
        self.with_resource(|resource| {
            resource.segmenters = Some(segmenters);
            Ok(())
        })
    }

    pub fn load_font_base(&self, family: String, weight: u16, data: Vec<u8>) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_font_base(&family, weight, &data)?;
            Ok(())
        })
    }

    pub fn load_font_chunk(&self, family: String, weight: u16, data: Vec<u8>) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_font_chunk(&family, weight, &data)?;
            Ok(())
        })
    }

    pub fn load_font_manifest(
        &self,
        family: String,
        weight: u16,
        data: Vec<u8>,
    ) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_font_manifest(&family, weight, &data)?;
            Ok(())
        })
    }

    pub fn load_fallback_font_manifests(&self, data: Vec<u8>) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.add_fallback_font_manifests(&data)?;
            Ok(())
        })
    }

    pub fn set_font_families(
        &self,
        families: Vec<Complex<editor_resource::FontFamily>>,
    ) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.set_font_families(families.from_ffi()?)?;
            Ok(())
        })
    }

    pub fn set_phantom_font_families(&self, families: Vec<String>) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.set_phantom_font_families(families)?;
            Ok(())
        })
    }

    #[cfg(not(feature = "wasm-server"))]
    pub fn backend_kind(&self) -> EditorResult<Complex<editor_renderer::BackendKind>> {
        Ok(self.backend.kind().into_ffi()?)
    }
}

impl EditorHost {
    fn with_resource<F, R>(&self, f: F) -> EditorResult<R>
    where
        F: FnOnce(&mut editor_resource::Resource) -> EditorResult<R>,
    {
        let mut resource = self.resource.lock().map_err(|_| FfiError::LockPoisoned)?;
        f(&mut resource)
    }
}

use cfg_if::cfg_if;
use std::sync::{Arc, Mutex};

use crate::prelude::*;

#[cfg(feature = "wasm")]
struct WasmFilteredLogger;

#[cfg(feature = "wasm")]
impl log::Log for WasmFilteredLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let max = if metadata.target().starts_with("editor") {
            log::Level::Debug
        } else {
            log::Level::Warn
        };
        metadata.level() <= max
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        console_log::log(record);
    }

    fn flush(&self) {}
}

fn init_logger() {
    cfg_if! {
        if #[cfg(feature = "wasm")] {
            static LOGGER: WasmFilteredLogger = WasmFilteredLogger;
            if log::set_logger(&LOGGER).is_ok() {
                log::set_max_level(log::LevelFilter::Debug);
            }
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
    pub(crate) resource: Arc<Mutex<editor_resource::Resource>>,
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
impl EditorHost {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn create(icu_data: Vec<u8>) -> EditorResult<Owned<Self>> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        init_logger();

        let icu = editor_resource::IcuResources::from_icu_data(&icu_data)?;

        Ok(into_owned(Self {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new(icu))),
        }))
    }

    pub fn create_editor_from_doc(
        &self,
        doc: Complex<editor_model::PlainDoc>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let plain: editor_model::PlainDoc = doc.from_ffi()?;
        let state = editor_state::State::from_plain(&plain).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;

        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));

        Ok(into_owned(crate::editor::Editor::new(
            core,
            crate::editor::CarrierStash::default(),
        )))
    }

    pub fn create_editor_from_graph(
        &self,
        changesets: Vec<u8>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let (state, carrier_bytes) = crate::graph::state_from_changesets(changesets)?;
        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(core, carrier_bytes)))
    }

    pub fn create_editor_from_graph_with_pending(
        &self,
        server: Vec<u8>,
        pending_encoded: Vec<u8>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let pending = crate::graph::decode_length_prefixed(&pending_encoded)?;
        let (state, carrier_bytes) =
            crate::graph::state_from_changesets_with_pending(server, pending)?;
        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(core, carrier_bytes)))
    }

    pub fn extract_text_from_graph(&self, changesets: Vec<u8>) -> EditorResult<String> {
        let (state, _) = crate::graph::state_from_changesets(changesets)?;
        let view = state.view();
        Ok(editor_state::flat_text(
            &view,
            0..editor_state::flat_size(&view),
        ))
    }

    pub fn root_attrs_from_graph(
        &self,
        changesets: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainRootNode>> {
        let (state, _) = crate::graph::state_from_changesets(changesets)?;
        let view = state.view();
        let root = crate::root::attrs(&view).ok_or(FfiError::NoInitialCursorPosition)?;
        Ok(root.into_ffi()?)
    }

    pub fn root_modifiers_from_graph(
        &self,
        changesets: Vec<u8>,
    ) -> EditorResult<Vec<Complex<editor_model::Modifier>>> {
        let (state, _) = crate::graph::state_from_changesets(changesets)?;
        Ok(crate::root::root_default_modifiers(&state).into_ffi()?)
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

    pub fn set_text_replacement_rules(
        &self,
        rules: Vec<Complex<editor_resource::RawTextReplacementRule>>,
    ) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.set_text_replacement_rules(rules.from_ffi()?);
            Ok(())
        })
    }

    pub fn set_auto_surround_enabled(&self, enabled: bool) -> EditorResult<()> {
        self.with_resource(|resource| {
            resource.set_auto_surround_enabled(enabled);
            Ok(())
        })
    }

    pub fn graph_heads(&self, changesets: Vec<u8>) -> EditorResult<Vec<u8>> {
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changesets[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let (g, _dropped) =
            editor_crdt::OpGraph::<editor_model::EditOp>::new().receive_changesets_ordered(css);
        let heads: Vec<editor_crdt::Dot> = g.current_heads().copied().collect();
        if heads.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_dots(&heads)
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn set_theme_variant(
        &self,
        variant: Complex<editor_resource::ThemeVariant>,
    ) -> EditorResult<bool> {
        self.with_resource(|resource| Ok(resource.theme.set_variant(variant.from_ffi()?)))
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

    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new_test())),
        }
    }
}

#[cfg(feature = "wasm-browser")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl EditorHost {
    pub fn set_gl_canary(&self, callback: js_sys::Function) {
        crate::platform::gl::set_canary(callback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_host() -> EditorHost {
        EditorHost {
            resource: Arc::new(Mutex::new(editor_resource::Resource::new_test())),
        }
    }

    fn test_viewport() -> editor_view::Viewport {
        editor_view::Viewport::new(320.0, 640.0, 1.0)
    }

    fn graph_from_plain(plain: &editor_model::PlainDoc) -> Vec<u8> {
        let state = editor_state::State::from_plain(plain).unwrap();
        editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
            state.graph().changesets_as_vec(),
        ))
        .unwrap()
    }

    #[test]
    fn graph_heads_of_empty_input_is_empty_bytes() {
        let host = make_host();
        let bytes = host.graph_heads(Vec::new()).unwrap();
        assert!(
            bytes.is_empty(),
            "heads of a graph built from zero changesets must be exactly 0 bytes"
        );
    }

    #[test]
    fn set_theme_variant_returns_true_on_change() {
        let host = make_host();
        let result = host
            .set_theme_variant(editor_resource::ThemeVariant::DarkBlack)
            .unwrap();
        assert!(result);
    }

    #[test]
    fn set_theme_variant_returns_false_for_same_variant() {
        let host = make_host();
        host.set_theme_variant(editor_resource::ThemeVariant::DarkBlack)
            .unwrap();
        let result = host
            .set_theme_variant(editor_resource::ThemeVariant::DarkBlack)
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn root_modifiers_from_graph_returns_root_default_modifiers() {
        let host = make_host();
        let plain = crate::doc_builder::build_default_doc(
            editor_model::PlainRootNode::default(),
            vec![
                editor_model::Modifier::FontSize { value: 1600 },
                editor_model::Modifier::BlockGap { value: 120 },
            ],
        );
        let state = editor_state::State::from_plain(&plain).unwrap();
        let graph = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(state.graph().changesets_as_vec()),
        )
        .unwrap();

        let modifiers = host.root_modifiers_from_graph(graph).unwrap();

        assert!(modifiers.contains(&editor_model::Modifier::FontSize { value: 1600 }));
        assert!(modifiers.contains(&editor_model::Modifier::BlockGap { value: 120 }));
    }

    #[test]
    fn create_editor_from_graph_preserves_missing_selection() {
        let host = make_host();
        let plain =
            crate::doc_builder::build_default_doc(editor_model::PlainRootNode::default(), vec![]);
        let graph = graph_from_plain(&plain);

        let editor = host
            .create_editor_from_graph(graph, test_viewport())
            .unwrap();

        assert!(editor.selection().unwrap().is_none());
    }

    #[test]
    fn create_editor_from_doc_preserves_missing_selection() {
        let host = make_host();
        let plain =
            crate::doc_builder::build_default_doc(editor_model::PlainRootNode::default(), vec![]);

        let editor = host.create_editor_from_doc(plain, test_viewport()).unwrap();

        assert!(editor.selection().unwrap().is_none());
    }

    #[test]
    fn create_editor_from_graph_with_pending_preserves_missing_selection() {
        let host = make_host();
        let plain =
            crate::doc_builder::build_default_doc(editor_model::PlainRootNode::default(), vec![]);
        let graph = graph_from_plain(&plain);

        let editor = host
            .create_editor_from_graph_with_pending(graph, vec![], test_viewport())
            .unwrap();

        assert!(editor.selection().unwrap().is_none());
    }
}

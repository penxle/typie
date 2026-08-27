use cfg_if::cfg_if;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::prelude::*;

const FONT_CHUNK_MAX_BYTES: usize = 32 * 1024 * 1024;

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
pub struct ResourceUpdate {
    pub(crate) inner: editor_core::ResourceUpdate,
}

impl ResourceUpdate {
    fn new(
        snapshot: Arc<editor_resource::ResourceSnapshot>,
        notices: Vec<editor_core::SystemEvent>,
    ) -> Owned<Self> {
        into_owned(Self {
            inner: editor_core::ResourceUpdate::new(snapshot, notices),
        })
    }
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
impl ResourceUpdate {}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorHost {
    // GraphIngest is an independently owned FFI object and may outlive the
    // EditorHost handle, so it shares the source owner rather than capturing a
    // snapshot before ingestion finishes.
    pub(crate) source: Arc<Mutex<editor_resource::ResourceSource>>,
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
            source: Arc::new(Mutex::new(editor_resource::ResourceSource::new(icu))),
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
        let core = editor_core::Editor::new(state, viewport, self.new_local_resource()?);

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
        let core = editor_core::Editor::new(state, viewport, self.new_local_resource()?);
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
        let core = editor_core::Editor::new(state, viewport, self.new_local_resource()?);
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
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let prepared = editor_resource::prepare_fonts(families.from_ffi()?);
        let snapshot = self.lock_source()?.set_fonts(prepared);
        Ok(snapshot.map(|snapshot| {
            ResourceUpdate::new(snapshot, vec![editor_core::SystemEvent::FontsChanged])
        }))
    }

    pub fn add_font_base(
        &self,
        family: String,
        weight: u16,
        data: Vec<u8>,
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let prepared = editor_resource::prepare_font_base(&data)?;
        let snapshot = self
            .lock_source()?
            .insert_font_base(&family, weight, prepared)?;
        Ok(snapshot.map(|snapshot| {
            ResourceUpdate::new(
                snapshot,
                vec![editor_core::SystemEvent::FontBaseLoaded { family, weight }],
            )
        }))
    }

    pub fn add_font_chunk(
        &self,
        family: String,
        weight: u16,
        chunk_id: u16,
        data: Vec<u8>,
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let payload = editor_resource::decompress_zstd_capped(&data, FONT_CHUNK_MAX_BYTES)?;
        let prepared = editor_resource::prepare_font_chunk(payload)?;
        let snapshot = self
            .lock_source()?
            .add_font_chunk(&family, weight, chunk_id, prepared)?;
        Ok(snapshot.map(|snapshot| {
            ResourceUpdate::new(
                snapshot,
                vec![editor_core::SystemEvent::FontChunkLoaded {
                    family,
                    weight,
                    chunk_id,
                }],
            )
        }))
    }

    pub fn add_font_manifest(
        &self,
        family: String,
        weight: u16,
        data: Vec<u8>,
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let bytes = editor_resource::decompress_zstd_capped(&data, 1024 * 1024)?;
        let manifest = editor_resource::FontManifest::from_bytes(&bytes)?;
        let snapshot = self
            .lock_source()?
            .add_font_manifest(&family, weight, manifest)?;
        Ok(snapshot.map(|snapshot| {
            ResourceUpdate::new(
                snapshot,
                vec![editor_core::SystemEvent::FontManifestLoaded { family, weight }],
            )
        }))
    }

    pub fn set_text_replacement_rules(
        &self,
        rules: Vec<Complex<editor_resource::RawTextReplacementRule>>,
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let prepared = editor_resource::prepare_text_replacement_rules(rules.from_ffi()?);
        let snapshot = self.lock_source()?.set_text_replacement_rules(prepared);
        Ok(snapshot.map(|snapshot| ResourceUpdate::new(snapshot, Vec::new())))
    }

    pub fn set_auto_surround_enabled(
        &self,
        enabled: bool,
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let snapshot = self.lock_source()?.set_auto_surround_enabled(enabled);
        Ok(snapshot.map(|snapshot| ResourceUpdate::new(snapshot, Vec::new())))
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
    ) -> EditorResult<Option<Owned<ResourceUpdate>>> {
        let variant = variant.from_ffi()?;
        let snapshot = self.lock_source()?.set_theme_variant(variant);
        Ok(snapshot.map(|snapshot| {
            ResourceUpdate::new(
                snapshot,
                vec![editor_core::SystemEvent::ThemeVariantChanged],
            )
        }))
    }
}

impl EditorHost {
    fn lock_source(&self) -> EditorResult<MutexGuard<'_, editor_resource::ResourceSource>> {
        self.source
            .lock()
            .map_err(|_| FfiError::LockPoisoned.into())
    }

    fn new_local_resource(&self) -> EditorResult<Arc<Mutex<editor_resource::Resource>>> {
        local_resource_from_source(&self.source)
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Self {
        Self {
            source: Arc::new(Mutex::new(editor_resource::ResourceSource::new_test())),
        }
    }
}

pub(crate) fn local_resource_from_source(
    source: &Mutex<editor_resource::ResourceSource>,
) -> EditorResult<Arc<Mutex<editor_resource::Resource>>> {
    let snapshot = source
        .lock()
        .map_err(|_| FfiError::LockPoisoned)?
        .snapshot();
    Ok(Arc::new(Mutex::new(
        editor_resource::Resource::from_snapshot(snapshot),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_host() -> EditorHost {
        EditorHost {
            source: Arc::new(Mutex::new(editor_resource::ResourceSource::new_test())),
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
    fn font_manifest_v2_must_match_base_num_glyphs_in_either_arrival_order() {
        let manifest = editor_resource::FontManifest::from_coverages(&[vec![0, 0]])
            .with_glyph_chunks(0, vec![vec![]])
            .unwrap();
        let manifest = editor_resource::compress_zstd(&manifest.to_bytes());
        let base = editor_resource::compress_zstd(include_bytes!(
            "../../editor-resource/assets/placeholder.ttf"
        ));

        let manifest_first = make_host();
        manifest_first
            .add_font_manifest("Test".into(), 400, manifest.clone())
            .unwrap();
        assert!(
            manifest_first
                .add_font_base("Test".into(), 400, base.clone())
                .is_err()
        );

        let base_first = make_host();
        base_first.add_font_base("Test".into(), 400, base).unwrap();
        assert!(
            base_first
                .add_font_manifest("Test".into(), 400, manifest)
                .is_err()
        );
    }

    #[test]
    fn set_theme_variant_returns_exact_update_and_none_for_noop() {
        let host = make_host();
        assert!(
            host.set_theme_variant(editor_resource::ThemeVariant::LightWhite)
                .unwrap()
                .is_none()
        );

        let update = host
            .set_theme_variant(editor_resource::ThemeVariant::DarkBlack)
            .unwrap()
            .expect("changed theme must return an update");
        let current = host.source.lock().unwrap().snapshot();

        assert!(Arc::ptr_eq(update.inner.snapshot(), &current));
        assert_eq!(
            update.inner.notices(),
            &[editor_core::SystemEvent::ThemeVariantChanged]
        );
        assert!(
            host.set_theme_variant(editor_resource::ThemeVariant::DarkBlack)
                .unwrap()
                .is_none()
        );
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

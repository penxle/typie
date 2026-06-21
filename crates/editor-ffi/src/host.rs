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
        let (doc, op_graph) = editor_model::Doc::from_plain(plain);
        let selection = doc_start_selection(&doc)?;
        let state = editor_state::State::new(doc, op_graph, Some(selection));

        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));

        Ok(into_owned(crate::editor::Editor::new(core)))
    }

    pub fn create_editor_from_graph(
        &self,
        changesets: Vec<u8>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let (doc, graph) = crate::graph::doc_from_changesets(changesets)?;
        let selection = doc_start_selection(&doc)?;
        let state = editor_state::State::new(doc, graph, Some(selection));
        let viewport = viewport.from_ffi()?;
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(core)))
    }

    pub fn extract_text_from_graph(&self, changesets: Vec<u8>) -> EditorResult<String> {
        let (doc, _) = crate::graph::doc_from_changesets(changesets)?;
        Ok(doc.extract_text())
    }

    pub fn root_attrs_from_graph(
        &self,
        changesets: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainRootNode>> {
        let (doc, _) = crate::graph::doc_from_changesets(changesets)?;
        let root = crate::root::attrs(&doc).ok_or(FfiError::NoInitialCursorPosition)?;
        Ok(root.into_ffi()?)
    }

    pub fn root_modifiers_from_graph(
        &self,
        changesets: Vec<u8>,
    ) -> EditorResult<Vec<Complex<editor_model::Modifier>>> {
        let (doc, _) = crate::graph::doc_from_changesets(changesets)?;
        Ok(crate::root::base_style_modifiers(&doc).into_ffi()?)
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
    fn root_modifiers_from_graph_returns_root_base_style_modifiers() {
        let host = make_host();
        let plain = crate::doc_builder::build_default_doc(
            editor_model::PlainRootNode::default(),
            vec![
                editor_model::Modifier::FontSize { value: 1600 },
                editor_model::Modifier::BlockGap { value: 120 },
            ],
        );
        let (_, graph) = editor_model::Doc::from_plain(plain);
        let graph = editor_crdt::wire::encode(&graph.changesets_as_vec()).unwrap();

        let modifiers = host.root_modifiers_from_graph(graph).unwrap();

        assert!(modifiers.contains(&editor_model::Modifier::FontSize { value: 1600 }));
        assert!(modifiers.contains(&editor_model::Modifier::BlockGap { value: 120 }));
    }
}

fn doc_start_selection(doc: &editor_model::Doc) -> EditorResult<editor_state::Selection> {
    use editor_state::NodeRefCursorExt;
    let root = doc.root().ok_or(FfiError::NoInitialCursorPosition)?;
    let pos = root
        .first_cursor_position()
        .ok_or(FfiError::NoInitialCursorPosition)?;
    // Bypass entry point: first_cursor_position can yield (root,0) collapsed on a
    // leading atom, so normalize upholds the invariant here.
    let sel = editor_state::Selection::collapsed(pos);
    Ok(sel.normalize(doc).unwrap_or(sel))
}

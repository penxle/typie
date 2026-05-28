#[cfg(not(feature = "wasm-server"))]
use hashbrown::HashMap;
use std::sync::Mutex;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

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

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackedRange {
    pub id: String,
    pub group: String,
    pub anchor: editor_state::Position,
    pub head: editor_state::Position,
    pub metadata: String,
    pub invalid: bool,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackedRangeHit {
    pub id: String,
    pub group: String,
    pub rects: Vec<editor_view::PageRect>,
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

    pub fn can(&self, message: Complex<editor_core::Message>) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.can(message.from_ffi()?)?))
    }

    pub fn tick(&self) -> EditorResult<Vec<Complex<editor_core::EditorEvent>>> {
        self.with_inner(|inner| Ok(inner.editor.tick()?.into_ffi()?))
    }

    pub fn cursor(&self) -> EditorResult<Option<Complex<editor_view::CursorMetrics>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let Some(selection) = state.selection.as_ref() else {
                return Ok(None);
            };
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

    pub fn selection(&self) -> EditorResult<Option<Complex<editor_state::Selection>>> {
        self.with_inner(|inner| Ok(inner.editor.state().selection.into_ffi()?))
    }

    pub fn copy_selection(
        &self,
    ) -> EditorResult<Option<Complex<editor_clipboard::ClipboardPayload>>> {
        self.with_inner(|inner| {
            let payload = editor_clipboard::Slice::extract(inner.editor.state())
                .map(|slice| slice.to_payload());
            Ok(payload.into_ffi()?)
        })
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

    pub fn modifier_state(&self) -> EditorResult<Option<Complex<editor_model::ModifierState>>> {
        self.with_inner(|inner| Ok(inner.editor.modifier_state().into_ffi()?))
    }

    pub fn block_state(&self) -> EditorResult<Option<Complex<editor_core::BlockState>>> {
        self.with_inner(|inner| Ok(inner.editor.block_state().into_ffi()?))
    }

    pub fn character_counts(&self) -> EditorResult<Complex<CharacterCounts>> {
        self.with_inner(|inner| {
            let (doc, sel) = inner.editor.character_counts();
            Ok(CharacterCounts {
                doc_with_whitespace: doc.with_whitespace,
                doc_without_whitespace: doc.without_whitespace,
                doc_without_whitespace_and_punctuation: doc.without_whitespace_and_punctuation,
                selection_with_whitespace: sel.with_whitespace,
                selection_without_whitespace: sel.without_whitespace,
                selection_without_whitespace_and_punctuation: sel
                    .without_whitespace_and_punctuation,
            }
            .into_ffi()?)
        })
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

    pub fn page_link_rects(&self, page: u32) -> EditorResult<Vec<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.page_link_rects(page as usize).into_ffi()?))
    }

    pub fn link_rects(&self) -> EditorResult<Vec<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.link_rects().into_ffi()?))
    }

    pub fn link_hit_test(
        &self,
        page: u32,
        x: f32,
        y: f32,
    ) -> EditorResult<Option<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.link_hit_test(page as usize, x, y).into_ffi()?))
    }

    pub fn selection_endpoints(
        &self,
    ) -> EditorResult<Option<Complex<editor_view::SelectionEndpoints>>> {
        self.with_inner(|inner| Ok(inner.editor.selection_endpoints().into_ffi()?))
    }

    pub fn selection_hit_test(&self, page: u32, x: f32, y: f32) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.selection_hit_test(page as usize, x, y)))
    }

    pub fn cursor_hit_test(&self, page: u32, x: f32, y: f32) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.cursor_hit_test(page as usize, x, y)))
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
            let state = inner.editor.state();
            Ok(inner
                .editor
                .view()
                .external_elements(&state.doc, state.selection.as_ref())
                .into_ffi()?)
        })
    }

    pub fn table_overlays(&self) -> EditorResult<Vec<Complex<editor_view::TableOverlay>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .view()
                .table_overlays(
                    &inner.editor.state().doc,
                    inner.editor.state().selection.as_ref(),
                )
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

    pub fn freeze_selection(
        &self,
        selection: Complex<editor_state::Selection>,
    ) -> EditorResult<Complex<editor_state::StableSelection>> {
        self.with_inner(|inner| {
            let sel: editor_state::Selection = selection.from_ffi()?;
            let doc = &inner.editor.state().doc;
            if !position_is_addressable(&sel.anchor, doc)
                || !position_is_addressable(&sel.head, doc)
            {
                return Err(EditorError::General {
                    msg: "freeze_selection: anchor or head not addressable in current doc".into(),
                });
            }
            Ok(editor_state::StableSelection::freeze(&sel, doc).into_ffi()?)
        })
    }

    pub fn tracked_ranges(
        &self,
        group: Option<String>,
    ) -> EditorResult<Vec<Complex<TrackedRange>>> {
        self.with_inner(|inner| {
            let doc = &inner.editor.state().doc;
            let registry = inner.editor.tracked_ranges();
            let ranges: Box<dyn Iterator<Item = &editor_core::TrackedRange>> = match &group {
                Some(g) => Box::new(registry.iter_group(g)),
                None => Box::new(registry.iter()),
            };
            let result: Vec<TrackedRange> = ranges
                .map(|r| {
                    let sel = r.selection.thaw(doc);
                    TrackedRange {
                        id: r.id.clone(),
                        group: r.group.clone(),
                        anchor: sel.anchor,
                        head: sel.head,
                        metadata: r.metadata.clone(),
                        invalid: r.explicitly_invalid || sel.is_collapsed(),
                    }
                })
                .collect();
            Ok(result.into_ffi()?)
        })
    }

    pub fn tracked_ranges_at(
        &self,
        page: u32,
        x: f32,
        y: f32,
        group: Option<String>,
    ) -> EditorResult<Vec<Complex<TrackedRangeHit>>> {
        self.with_inner(|inner| {
            let hits = inner
                .editor
                .tracked_ranges_at(page as usize, x, y, group.as_deref());
            let public: Vec<TrackedRangeHit> = hits
                .into_iter()
                .map(|h| TrackedRangeHit {
                    id: h.id,
                    group: h.group,
                    rects: h.rects,
                })
                .collect();
            Ok(public.into_ffi()?)
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

fn position_is_addressable(pos: &editor_state::Position, doc: &editor_model::Doc) -> bool {
    let Some(entry) = doc.get_entry(pos.node_id) else {
        return false;
    };
    match &entry.node {
        editor_model::Node::Text(text) => pos.offset <= text.text.len(),
        _ => pos.offset <= entry.children.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    fn make_ffi_editor(initial: editor_state::State) -> Editor {
        let mut core = editor_core::Editor::new_test(initial);
        core.apply(editor_core::Message::System {
            event: editor_core::SystemEvent::Initialize,
        });
        Editor::new(core)
    }

    #[test]
    fn ffi_selection_endpoints_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 8)
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(
            result.is_some(),
            "range selection must produce endpoints through FFI",
        );
    }

    #[test]
    fn ffi_selection_endpoints_collapsed_is_none() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_selection_hit_test_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let editor = make_ffi_editor(initial);
        let endpoints = editor
            .selection_endpoints()
            .expect("ffi call returns Ok")
            .expect("range selection has endpoints");
        let probe_x = endpoints.from.rect.x + 5.0;
        let probe_y = endpoints.from.rect.y + endpoints.from.rect.height * 0.5;
        let hit = editor
            .selection_hit_test(0, probe_x, probe_y)
            .expect("ffi call returns Ok");
        assert!(
            hit,
            "probe inside selection rect must register as hit through FFI"
        );
    }

    #[test]
    fn copy_selection_returns_payload_for_text_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let editor = make_ffi_editor(state);
        let payload = editor
            .copy_selection()
            .expect("ffi call returns Ok")
            .expect("non-collapsed selection produces payload");
        assert_eq!(payload.text, "Hello");
        assert!(payload.html.contains("data-slice"));
    }

    #[test]
    fn copy_selection_returns_none_for_collapsed() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let editor = make_ffi_editor(state);
        assert!(
            editor
                .copy_selection()
                .expect("ffi call returns Ok")
                .is_none()
        );
    }

    #[test]
    fn ffi_cursor_hit_test_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        let editor = make_ffi_editor(initial);
        let cursor = editor
            .cursor()
            .expect("ffi call returns Ok")
            .expect("collapsed cursor has metrics")
            .from_ffi()
            .expect("cursor metrics decode");
        let probe_x = cursor.caret.x;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;
        let hit = editor
            .cursor_hit_test(0, probe_x, probe_y)
            .expect("ffi call returns Ok");

        assert!(
            hit,
            "probe resolving to current cursor must register as hit through FFI"
        );
    }

    #[test]
    fn ffi_selection_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection().expect("ffi call returns Ok");
        assert!(
            result.is_none(),
            "selection FFI must return None when state.selection is None"
        );
    }

    #[test]
    fn ffi_cursor_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.cursor().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_copy_selection_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.copy_selection().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_selection_endpoints_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_modifier_state_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.modifier_state().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_block_state_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.block_state().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_external_elements_returns_empty_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.external_elements().expect("ffi call returns Ok");
        assert!(result.is_empty());
    }

    #[test]
    fn ffi_external_elements_lists_image_when_selection_is_none() {
        let (initial, ..) = state! {
            doc { root { image paragraph { text("hi") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.external_elements().expect("ffi call returns Ok");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ffi_selection_hit_test_returns_false_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let hit = editor
            .selection_hit_test(0, 10.0, 10.0)
            .expect("ffi call returns Ok");
        assert!(
            !hit,
            "selection_hit_test must return false when state.selection is None"
        );
    }

    #[test]
    fn ffi_cursor_hit_test_returns_false_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let hit = editor
            .cursor_hit_test(0, 10.0, 10.0)
            .expect("ffi call returns Ok");
        assert!(
            !hit,
            "cursor_hit_test must return false when state.selection is None"
        );
    }

    #[test]
    fn ffi_selection_unset_then_set_roundtrip() {
        use editor_core::{Message, SelectionOp};
        use editor_state::{Position, Selection};

        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let editor = make_ffi_editor(initial);

        editor
            .enqueue(Message::Selection {
                op: SelectionOp::Unset,
            })
            .expect("enqueue unset");
        let _ = editor.tick().expect("tick");
        assert!(
            editor.selection().expect("ffi ok").is_none(),
            "Unset must clear selection through FFI",
        );

        let new_sel = Selection::collapsed(Position::new(t1, 1));
        editor
            .enqueue(Message::Selection {
                op: SelectionOp::Set { selection: new_sel },
            })
            .expect("enqueue set");
        let _ = editor.tick().expect("tick");
        let after_set = editor.selection().expect("ffi ok");
        assert!(
            after_set.is_some(),
            "Set must restore selection through FFI"
        );
    }

    #[test]
    fn ffi_can_returns_true_for_insertion_with_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let editor = make_ffi_editor(initial);
        let msg = editor_core::Message::Insertion {
            op: editor_core::InsertionOp::Text { text: "x".into() },
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(probed);
    }

    #[test]
    fn ffi_can_returns_false_for_undo_empty_history() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let editor = make_ffi_editor(initial);
        let msg = editor_core::Message::History {
            op: editor_core::HistoryOp::Undo,
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(!probed);
    }

    #[test]
    fn ffi_can_returns_false_for_same_selection_set() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let editor = make_ffi_editor(initial);
        let same = editor_state::Selection::collapsed(editor_state::Position::new(t1, 2));
        let msg = editor_core::Message::Selection {
            op: editor_core::SelectionOp::Set { selection: same },
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(!probed);
    }

    #[test]
    fn ffi_can_does_not_mutate_observable_state() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let editor = make_ffi_editor(initial);

        let inspect_before = editor.inspect_state_as_macro().unwrap();
        let msg = editor_core::Message::Insertion {
            op: editor_core::InsertionOp::Text { text: "x".into() },
        };
        let _ = editor.can(msg.into_ffi().unwrap()).unwrap();
        let inspect_after = editor.inspect_state_as_macro().unwrap();

        assert_eq!(
            inspect_before, inspect_after,
            "can() must not mutate observable state visible through inspect_state_as_macro",
        );
    }

    #[test]
    fn ffi_freeze_selection_returns_stable_selection() {
        let (initial, _t1) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 1) -> (t1, 8)
        };
        let editor = make_ffi_editor(initial.clone());
        let sel = initial.selection.unwrap();

        let result = editor.freeze_selection(sel);
        assert!(
            result.is_ok(),
            "freeze_selection must Ok for valid selection"
        );

        let stable = result.unwrap();
        editor
            .enqueue(editor_core::Message::TrackedRange {
                op: editor_core::TrackedRangeOp::AddFrozen {
                    id: "r".into(),
                    group: "g".into(),
                    selection: stable,
                    metadata: String::new(),
                },
            })
            .expect("enqueue");
        let _ = editor.tick().expect("tick");

        let ranges = editor.tracked_ranges(None).expect("ffi ok");
        let r = ranges.iter().find(|x| x.id == "r").expect("range present");
        assert_eq!(r.anchor.offset, 1);
        assert_eq!(r.head.offset, 8);
        assert!(!r.invalid);
    }

    #[test]
    fn ffi_freeze_selection_returns_err_for_unresolvable() {
        let (initial, _t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let editor = make_ffi_editor(initial);

        let bogus = editor_model::NodeId::new();
        let bogus_sel = editor_state::Selection::new(
            editor_state::Position::new(bogus, 0),
            editor_state::Position::new(bogus, 0),
        );

        let result = editor.freeze_selection(bogus_sel);
        assert!(
            result.is_err(),
            "freeze_selection must Err for unresolvable selection"
        );
    }

    #[test]
    fn ffi_dnd_over_returns_events() {
        use editor_core::{DndOp, DndPayloadKind, EditorEvent, InputModifiers, Message};

        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        let editor = make_ffi_editor(initial);
        let cursor = editor
            .cursor()
            .expect("ffi call returns Ok")
            .expect("collapsed cursor has metrics");

        editor
            .enqueue(Message::Dnd {
                op: DndOp::Over {
                    page: 0,
                    x: cursor.caret.x,
                    y: cursor.line.y + cursor.line.height * 0.5,
                    payload: DndPayloadKind::Text,
                    modifiers: InputModifiers::default(),
                },
            })
            .expect("ffi enqueue returns Ok");
        let events = editor.tick().expect("ffi tick returns Ok");

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated)),
            "immediate dnd over must return render invalidation events",
        );
    }

    #[test]
    fn ffi_tracked_ranges_at_returns_hits() {
        let (initial, _t1) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let editor = make_ffi_editor(initial.clone());
        let sel = initial.selection.unwrap();

        editor
            .enqueue(editor_core::Message::TrackedRange {
                op: editor_core::TrackedRangeOp::Add {
                    id: "thread-a".into(),
                    group: "comment".into(),
                    selection: sel,
                    metadata: String::new(),
                },
            })
            .expect("enqueue tracked-range add");
        let _ = editor.tick().expect("tick");

        let endpoints = editor
            .selection_endpoints()
            .expect("ffi ok")
            .expect("selection produces endpoints");
        let cx = endpoints.from.rect.x + 2.0;
        let cy = endpoints.from.rect.y + endpoints.from.rect.height * 0.5;

        let page = endpoints.from.page_idx as u32;
        let hits = editor
            .tracked_ranges_at(page, cx, cy, Some("comment".into()))
            .expect("ffi ok");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "thread-a");
        assert_eq!(hits[0].group, "comment");
        assert!(
            !hits[0].rects.is_empty(),
            "FFI wrapper must forward range rects (hit point itself proves at least one)"
        );
        assert_eq!(
            hits[0].rects[0].page_idx, endpoints.from.page_idx,
            "rects must be page-local"
        );
    }
}

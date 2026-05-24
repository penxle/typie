use editor_clipboard::Slice;
use editor_commands::{self as commands};
use editor_transaction::{HistoryMeta, HistoryTag};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_op(editor: &mut Editor, op: ClipboardOp) -> Result<(), EditorError> {
    match op {
        ClipboardOp::Cut => editor.transact(|tr| {
            commands::delete_selection(tr)?;
            Ok(())
        }),
        ClipboardOp::Paste { html, text } => {
            let slice = Slice::from_payload(html.as_deref(), &text);
            let is_html_paste = html.as_deref().is_some_and(|h| !h.is_empty());
            let plain_text = is_html_paste.then(|| text.clone());
            editor.transact(|tr| {
                if let Some(plain_text) = plain_text {
                    tr.update_meta(|m| {
                        m.history = HistoryMeta::Tagged {
                            tag: HistoryTag::PasteHtml { plain_text },
                        };
                    });
                }
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(tr, slice.clone()),
                )?;
                Ok(())
            })
        }
        ClipboardOp::RepasteAsText => {
            let Some(HistoryTag::PasteHtml { plain_text }) = editor.history_last_tag() else {
                return Ok(());
            };
            let plain_text = plain_text.clone();
            let Some(inverse_steps) = editor.history_last_inverse_steps() else {
                return Ok(());
            };
            let plain_slice = Slice::from_text(&plain_text);
            editor.transact(|tr| {
                tr.apply_steps(inverse_steps)?;
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(tr, plain_slice.clone()),
                )?;
                Ok(())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::ModifierType;
    use editor_state::{DocFlatExt, ResolvedPositionFlatExt, assert_state_eq};
    use editor_transaction::HistoryTag;

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_cut_with_collapsed_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Clipboard {
                op: ClipboardOp::Cut,
            },
        );
    }

    #[test]
    fn probe_paste_empty() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Clipboard {
                op: ClipboardOp::Paste {
                    text: "".into(),
                    html: None,
                },
            },
        );
    }

    #[test]
    fn paste_text_replaces_node_selection() {
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                text: "b".into(),
                html: None,
            },
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { t1: text("b") }
                paragraph { text("c") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_text_at_leading_gap_creates_paragraph() {
        let (s, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                text: "pasted".into(),
                html: None,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("pasted") } image paragraph { text("b") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cut_deletes_selection() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Ho") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cut_then_paste_round_trip_identity() {
        let (s, ..) = state! {
            doc { r1: root {
                callout { paragraph { text("1") } }
                paragraph {}
            } }
            selection: (r1, 0, >) -> (r1, 2, <)
        };

        let payload = Slice::extract(&s).expect("non-collapsed").to_payload();

        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { r: root {
                callout { paragraph { text("1") } }
                paragraph {}
            } }
            selection: (r, 0)
        };
        editor_model::assert_doc_eq!(&editor.state().doc, &expected.doc);
    }

    #[test]
    fn paste_crlf_text_does_not_fail() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "a\r\nb".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") hard_break t2: text("b") } } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_html_with_meta_lossless() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("source") } } }
            selection: (t1, 0) -> (t1, 6)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t3: text("Hsourcei") } } }
            selection: (t3, 7)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_html_sets_paste_html_tag() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text.clone(),
            },
        });

        let tag = editor.history_last_tag().cloned();
        assert!(
            matches!(tag, Some(HistoryTag::PasteHtml { ref plain_text }) if plain_text == &payload.text),
            "expected PasteHtml tag with plain_text == payload.text, got {tag:?}"
        );
    }

    #[test]
    fn paste_with_text_only_does_not_set_tag() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "plain".into(),
            },
        });
        assert!(editor.history_last_tag().is_none());
    }

    #[test]
    fn paste_with_empty_html_does_not_set_tag() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(String::new()),
                text: "plain".into(),
            },
        });
        assert!(editor.history_last_tag().is_none());
    }

    #[test]
    fn repaste_as_text_replaces_paste_region_with_plain() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let (expected, ..) = state! {
            doc { root { paragraph { t3: text("Hhelloi") } } }
            selection: (t3, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn repaste_as_text_is_noop_when_last_tag_absent() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let initial = s.clone();
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn repaste_as_text_expires_after_other_edit() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_deletion() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_modifier_toggle() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 0) -> (t2, 2)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_ime_composition_start() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let state = editor.state();
        let head_flat = state
            .selection
            .unwrap()
            .head
            .resolve(&state.doc)
            .unwrap()
            .to_flat();
        editor.apply(Message::Composition {
            op: CompositionOp::SetRegion {
                start: head_flat,
                end: head_flat,
            },
        });

        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_selection_change() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, t2) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(t2, 0)),
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_undo() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_undo_returns_to_html_paste_state() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        let after_paste = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor_state::assert_state_eq!(editor.state(), &after_paste);
    }

    #[test]
    fn repaste_as_text_undo_twice_returns_to_pre_paste_state() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let pre_paste = s_target.clone();
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor_state::assert_state_eq!(editor.state(), &pre_paste);
    }

    #[test]
    fn repaste_as_text_is_one_shot() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        let after_first = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &after_first);
    }

    #[test]
    fn repaste_as_text_expires_after_remote_changeset() {
        use std::collections::BTreeMap;

        use editor_crdt::TextOp;
        use editor_model::{
            Doc, DocOp, Modifier, ModifierType, NodeId, PlainDoc, PlainNode, PlainNodeEntry,
            PlainParagraphNode, PlainRootNode, PlainTextNode,
        };
        use editor_state::{Position, Selection, State};
        use hashbrown::HashSet;

        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let root_modifiers = BTreeMap::from([
            (
                ModifierType::FontFamily,
                Modifier::FontFamily {
                    value: "Pretendard".into(),
                },
            ),
            (
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 400 },
            ),
        ]);

        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId::ROOT,
            PlainNodeEntry {
                parent: None,
                children: vec![para_id],
                modifiers: root_modifiers,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para_id,
            PlainNodeEntry {
                parent: Some(NodeId::ROOT),
                children: vec![text_id],
                modifiers: BTreeMap::new(),
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text_id,
            PlainNodeEntry {
                parent: Some(para_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                node: PlainNode::Text(PlainTextNode {
                    text: String::new(),
                }),
            },
        );
        let plain = PlainDoc { nodes };

        let (doc, graph) = Doc::from_plain(plain);
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let seed = State::new(doc, graph, Some(sel));
        let seed_css = seed.graph.changesets_as_vec();
        let replica_b = State::from_changesets(seed_css, Some(sel)).expect("from_changesets");

        let baseline: HashSet<_> = seed.graph.current_heads().copied().collect();
        let (state_a, _) = seed
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'r',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let mut css = state_a.local_changesets_since(&baseline).unwrap();
        let remote_cs = css.remove(0);

        let mut editor = Editor::new_test(replica_b);

        let source_payload = {
            let (s_source, ..) = state! {
                doc { root { paragraph { t1: text("hello") } } }
                selection: (t1, 0) -> (t1, 5)
            };
            Slice::extract(&s_source).unwrap().to_payload()
        };
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(source_payload.html),
                text: source_payload.text,
            },
        });

        editor.receive_remote_changeset(remote_cs);
        editor.tick().unwrap();

        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_preserves_list_marker_from_text_payload() {
        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some("<ul><li>a</li><li>b</li></ul>".into()),
                text: "1. a\n2. b".into(),
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let plain_text_in_doc = editor
            .state()
            .doc
            .flat_text(0..editor.state().doc.flat_size());
        assert!(
            plain_text_in_doc.contains("1. a") && plain_text_in_doc.contains("2. b"),
            "expected list marker preserved from text payload, got {plain_text_in_doc:?}"
        );
    }
}

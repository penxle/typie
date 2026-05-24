use editor_commands::{self as commands};
use editor_state::{
    PendingModifiers, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
    farther_endpoint,
};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_selection_op(editor: &mut Editor, op: SelectionOp) -> Result<(), EditorError> {
    if matches!(op, SelectionOp::Unset) {
        editor.clear_preferred_x();
        return editor.transact(|tr| {
            tr.set_composition(None)?;
            tr.set_pending_modifiers(PendingModifiers::new())?;
            tr.set_selection(None)?;
            Ok(())
        });
    }

    let extend_to_selection = resolve_extend_to_selection(editor, &op);
    let resource = editor.resource.clone();

    editor.clear_preferred_x();
    editor.transact(|tr| {
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        match op {
            SelectionOp::Set { selection } => {
                commands::set_selection(tr, selection)?;
            }
            SelectionOp::SetFlat { start, end } => {
                let doc = tr.doc();
                let start_pos = match ResolvedPosition::from_flat(&doc, start) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let end_pos = match ResolvedPosition::from_flat(&doc, end) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                commands::set_selection(
                    tr,
                    Selection::new(Position::from(&start_pos), Position::from(&end_pos)),
                )?;
            }
            SelectionOp::ExtendTo { .. } => {
                if let Some(selection) = extend_to_selection
                    && tr.selection() != Some(selection)
                {
                    commands::set_selection(tr, selection)?;
                }
            }
            SelectionOp::Expand { unit } => match (unit, tr.selection()) {
                (SelectionExpansionUnit::Word, Some(selection)) => {
                    let resource = resource.lock().unwrap();
                    commands::select_word_at(tr, selection, &resource)?;
                }
                (SelectionExpansionUnit::Sentence, Some(selection)) => {
                    let resource = resource.lock().unwrap();
                    commands::select_sentence_at(tr, selection, &resource)?;
                }
                (SelectionExpansionUnit::Paragraph, Some(selection)) => {
                    commands::select_paragraph_at(tr, selection)?;
                }
                (SelectionExpansionUnit::All, _) => {
                    commands::select_all(tr)?;
                }
                (_, None) => {}
            },
            SelectionOp::Unset => unreachable!("handled above"),
        }
        Ok(())
    })
}

fn resolve_extend_to_selection(editor: &Editor, op: &SelectionOp) -> Option<Selection> {
    let SelectionOp::ExtendTo {
        anchor_page,
        anchor_x,
        anchor_y,
        head_page,
        head_x,
        head_y,
        initial_selection,
    } = op
    else {
        return None;
    };

    let doc = &editor.state().doc;
    let head_hit = editor
        .view
        .hit_test_extending(*head_page, *head_x, *head_y)?;

    let selection = if let Some(initial_selection) = initial_selection {
        let initial = initial_selection.resolve(doc)?;
        let head = head_hit.resolve(doc)?;
        let initial_from = Position::from(initial.from());
        let initial_to = Position::from(initial.to());

        if head.from() < initial.from() {
            let head = farther_endpoint(doc, initial_to, head_hit.anchor, head_hit.head);
            Selection::new(initial_to, head)
        } else if head.to() > initial.to() {
            let head = farther_endpoint(doc, initial_from, head_hit.anchor, head_hit.head);
            Selection::new(initial_from, head)
        } else {
            Selection::new(initial_from, initial_to)
        }
    } else {
        let anchor_hit = editor
            .view
            .hit_test_extending(*anchor_page, *anchor_x, *anchor_y)?;
        let anchor = farther_endpoint(doc, head_hit.head, anchor_hit.anchor, anchor_hit.head);
        let head = farther_endpoint(doc, anchor, head_hit.anchor, head_hit.head);
        Selection::new(anchor, head)
    };

    (!selection.is_collapsed()).then_some(selection)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;
    use editor_state::{Composition, PendingModifier, Position, Selection};

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_set_same_selection_noop() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let sel = Selection::collapsed(Position::new(t1, 2));
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::Set { selection: sel },
            },
        );
    }

    #[test]
    fn probe_set_different_selection_changes() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let sel = Selection::collapsed(Position::new(t1, 4));
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::Set { selection: sel },
            },
        );
    }

    #[test]
    fn probe_unset_with_active_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::Unset,
            },
        );
    }

    #[test]
    fn probe_select_all() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::All,
            },
        );
    }

    #[test]
    fn probe_unset_already_unset() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: none
        };
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::Unset,
            },
        );
    }

    #[test]
    fn select_set() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let target = Selection::collapsed(Position::new(t1, 3));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });
        assert_eq!(editor.state().selection, Some(target));
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_with_initial_selection_expands_word_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let initial = editor.state().selection.expect("selection exists in test");
        assert_ne!(initial.anchor, initial.head);

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor_page: 0,
                anchor_x: 0.0,
                anchor_y: 5.0,
                head_page: 0,
                head_x: 9999.0,
                head_y: 5.0,
                initial_selection: Some(initial),
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, initial.anchor);
        assert_eq!(sel.head.node_id, initial.head.node_id);
        assert!(
            sel.head.offset > initial.head.offset,
            "selection should extend beyond the initially selected word, got {:?}",
            sel
        );
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_without_initial_selection_ignores_collapsed_result() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor_page: 0,
                anchor_x: 20.0,
                anchor_y: -100.0,
                head_page: 0,
                head_x: 20.0,
                head_y: -100.0,
                initial_selection: None,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(!before.expect("selection exists in test").is_collapsed());
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn expand_word_expands_selection_inside_one_word() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Word,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn expand_word_does_not_require_layout() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 3)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Word,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn expand_word_does_not_shrink_selection_across_words() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 8)
        };
        let before = state.selection;
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Word,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn expand_sentence_expands_selection_inside_one_sentence() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("Hello world. Next sentence.") } } }
            selection: (t, 1) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Sentence,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert!(
            sel.head.offset > 5,
            "sentence expansion should grow past the initial selection: {:?}",
            sel
        );
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn expand_all_selects_document_range() {
        let (state, ..) = state! {
            doc { root { paragraph { a: text("hello") } paragraph { b: text("world") } } }
            selection: (a, 1) -> (a, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::All,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 2);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn selection_op_unset_cascades_composition_pending_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(initial);

        editor
            .transact(|tr| {
                tr.set_composition(Some(Composition { start: 1, end: 3 }))?;
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();

        assert!(editor.state().composition.is_some());
        assert!(!editor.state().pending_modifiers.is_empty());
        assert!(editor.state().selection.is_some());

        editor.apply(Message::Selection {
            op: SelectionOp::Unset,
        });

        assert!(editor.state().selection.is_none(), "selection cleared");
        assert!(editor.state().composition.is_none(), "composition cleared");
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending cleared"
        );
    }

    #[test]
    fn selection_op_unset_undo_restores_all_three() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(initial);

        editor
            .transact(|tr| {
                tr.set_composition(Some(Composition { start: 1, end: 3 }))?;
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();

        // Clear history so the setup transact doesn't interfere with undo.
        editor.history =
            crate::history::History::new(editor_common::time::Duration::from_millis(300));

        let pre_unset_selection = editor.state().selection;
        let pre_unset_composition = editor.state().composition;
        let pre_unset_pending = editor.state().pending_modifiers.clone();

        editor.apply(Message::Selection {
            op: SelectionOp::Unset,
        });

        assert!(editor.state().selection.is_none(), "selection cleared");
        assert!(editor.state().composition.is_none(), "composition cleared");
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending cleared"
        );

        assert!(editor.history.can_undo(), "Unset must be undoable");
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_eq!(
            editor.state().selection,
            pre_unset_selection,
            "selection restored"
        );
        assert_eq!(
            editor.state().composition,
            pre_unset_composition,
            "composition restored"
        );
        assert_eq!(
            editor.state().pending_modifiers,
            pre_unset_pending,
            "pending restored"
        );
    }
}

use editor_commands::{self as commands};
use editor_model::NodeId;
use editor_state::{
    PendingModifiers, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
    cell_rect_selection, enclosing_table, enclosing_table_cell, farther_endpoint,
    resolve_paragraph_selection_expansion, resolve_sentence_selection_expansion,
    resolve_word_selection_expansion, table_cell_ids,
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
            tr.set_pending_style(None)?;
            tr.set_selection(None)?;
            Ok(())
        });
    }

    editor.clear_preferred_x();
    match op {
        SelectionOp::Set { selection } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            commands::set_selection(tr, selection)?;
            Ok(())
        }),
        SelectionOp::SetFrozen { selection } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            let live = selection.thaw(&tr.doc());
            commands::set_selection(tr, live)?;
            Ok(())
        }),
        SelectionOp::SetAt { page, x, y } => {
            let selection = editor.view.hit_test(page, x, y);
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection {
                    commands::set_selection(tr, selection)?;
                }
                Ok(())
            })
        }
        SelectionOp::SetFlat { start, end } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
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
            Ok(())
        }),
        SelectionOp::ExtendTo {
            anchor,
            head_page,
            head_x,
            head_y,
            base_selection,
        } => {
            let selection = resolve_extend_to_selection(
                editor,
                anchor,
                head_page,
                head_x,
                head_y,
                base_selection,
            );
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection
                    && tr.selection() != Some(selection)
                {
                    commands::set_selection(tr, selection)?;
                }
                Ok(())
            })
        }
        SelectionOp::SelectUnitAt { page, x, y, unit } => {
            let selection = resolve_select_unit_at_selection(editor, page, x, y, unit);
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection {
                    commands::set_selection(tr, selection)?;
                }
                Ok(())
            })
        }
        SelectionOp::Expand { unit } => {
            let resource = editor.resource.clone();
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                match (unit, tr.selection()) {
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
                }
                Ok(())
            })
        }
        SelectionOp::Unset => unreachable!("handled above"),
    }
}

fn drag_anchor_cell(editor: &Editor, pos: &Position) -> Option<NodeId> {
    if let Some(resolved) = editor
        .state
        .selection
        .as_ref()
        .and_then(|selection| selection.resolve(&editor.state.doc))
        && let Some(cell_rect) = resolved.as_cell_rect()
    {
        return Some(cell_rect.anchor_cell.id());
    }
    enclosing_table_cell(&editor.state.doc, pos.node_id)
}

fn resolve_select_unit_at_selection(
    editor: &Editor,
    page: usize,
    x: f32,
    y: f32,
    unit: SelectionPointUnit,
) -> Option<Selection> {
    let hit = editor.view.hit_test(page, x, y)?;
    match unit {
        SelectionPointUnit::Word => {
            let resource = editor.resource.lock().unwrap();
            Some(resolve_word_selection_expansion(&editor.state.doc, hit, &resource).unwrap_or(hit))
        }
        SelectionPointUnit::Sentence => {
            let resource = editor.resource.lock().unwrap();
            Some(
                resolve_sentence_selection_expansion(&editor.state.doc, hit, &resource)
                    .unwrap_or(hit),
            )
        }
        SelectionPointUnit::Paragraph => {
            Some(resolve_paragraph_selection_expansion(&editor.state.doc, hit).unwrap_or(hit))
        }
    }
}

fn resolve_extend_to_selection(
    editor: &Editor,
    anchor: Position,
    head_page: usize,
    head_x: f32,
    head_y: f32,
    base_selection: Option<Selection>,
) -> Option<Selection> {
    let doc = &editor.state().doc;
    if let Some(anchor_cell) = drag_anchor_cell(editor, &anchor) {
        if let Some(table_id) = enclosing_table(doc, anchor_cell) {
            let head_inside_table = editor
                .view
                .node_box_contains(head_page, head_x, head_y, table_id);
            if head_inside_table {
                let cells = table_cell_ids(doc, anchor_cell);
                if let Some(head_cell) = editor
                    .view
                    .nearest_node_box(head_page, head_x, head_y, &cells)
                {
                    let is_cell_mode = editor
                        .state
                        .selection
                        .as_ref()
                        .and_then(|selection| selection.resolve(doc))
                        .is_some_and(|resolved| resolved.as_cell_rect().is_some());
                    if (head_cell != anchor_cell || is_cell_mode)
                        && let Some(selection) = cell_rect_selection(doc, anchor_cell, head_cell)
                    {
                        return Some(selection);
                    }
                }
            }
            // head outside table: fall through; normalize promotes anchor to table boundary
        }
    }

    let head_hit = editor.view.hit_test_extending(head_page, head_x, head_y)?;

    let selection = if let Some(base_selection) = base_selection {
        let initial = base_selection.resolve(doc)?;
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
        let head = farther_endpoint(doc, anchor, head_hit.anchor, head_hit.head);
        Selection::new(anchor, head)
    };

    (!selection.is_collapsed()).then_some(selection)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;
    use editor_state::{Composition, PendingModifier, Position, Selection, StableSelection};

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
                op: SelectionOp::Expand {
                    unit: SelectionExpansionUnit::All,
                },
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
    fn select_set_frozen_restores_selection() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let target = Selection::collapsed(Position::new(t1, 3));
        let frozen = StableSelection::freeze(&target, &state.doc);
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::SetFrozen { selection: frozen },
        });
        assert_eq!(editor.state().selection, Some(target));
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn select_set_with_unaddressable_node_is_noop() {
        // A NodeId that doesn't exist in the doc (e.g. left over from a
        // previous session) must not panic — the request is silently dropped.
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let before = state.selection;
        let bogus = Selection::collapsed(Position::new(editor_model::NodeId::new(), 0));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: bogus },
        });
        assert_eq!(editor.state().selection, before);
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
    fn set_at_sets_hit_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::SetAt {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor.offset, 5);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn select_unit_at_word_expands_hit_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::SelectUnitAt {
                page: 0,
                x: 20.0,
                y: 5.0,
                unit: SelectionPointUnit::Word,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_with_base_selection_expands_word_range() {
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
                anchor: initial.anchor,
                head_page: 0,
                head_x: 9999.0,
                head_y: 5.0,
                base_selection: Some(initial),
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
    fn extend_to_without_base_selection_uses_anchor_position() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(t, 1),
                head_page: 0,
                head_x: 9999.0,
                head_y: 5.0,
                base_selection: None,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(t, 1));
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn extend_to_without_base_selection_ignores_collapsed_result() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: before.expect("selection exists in test").anchor,
                head_page: 0,
                head_x: 20.0,
                head_y: -100.0,
                base_selection: None,
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
    fn selection_op_unset_clears_pending_style() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(initial);

        editor
            .transact(|tr| {
                tr.set_pending_style(Some(editor_state::PendingStyle::Set {
                    style_id: "s1".to_string(),
                }))?;
                Ok(())
            })
            .unwrap();

        assert!(editor.state().pending_style.is_some());

        editor.apply(Message::Selection {
            op: SelectionOp::Unset,
        });

        assert!(editor.state().selection.is_none(), "selection cleared");
        assert!(
            editor.state().pending_style.is_none(),
            "pending_style cleared"
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

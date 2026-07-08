use editor_commands::{self as commands};
use editor_crdt::Dot;
use editor_model::DocView;
use editor_state::{
    Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, StableResolveCtx,
    cell_rect_selection, enclosing_table, enclosing_table_cell,
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
            tr.clear_pending_format()?;
            tr.set_selection(None)?;
            Ok(())
        });
    }

    editor.clear_preferred_x();
    match op {
        SelectionOp::Set { selection } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            if selection.resolve(&tr.view()).is_some() {
                if tr.selection() != Some(selection) {
                    tr.clear_pending_format()?;
                }
                tr.set_selection(Some(selection))?;
            }
            Ok(())
        }),
        SelectionOp::SetFrozen { selection } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            let live = {
                let view = tr.view();
                let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
                selection.resolve(&ctx)
            };
            if let Some(live) = live {
                if tr.selection() != Some(live) {
                    tr.clear_pending_format()?;
                }
                tr.set_selection(Some(live))?;
            }
            Ok(())
        }),
        SelectionOp::SetAt { page, x, y } => {
            let selection = editor.view.hit_test(page, x, y);
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection {
                    if tr.selection() != Some(selection) {
                        tr.clear_pending_format()?;
                    }
                    tr.set_selection(Some(selection))?;
                }
                Ok(())
            })
        }
        SelectionOp::SetFlat { start, end } => editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            let selection = {
                let view = tr.view();
                let start_pos = match ResolvedPosition::from_flat(&view, start) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let end_pos = match ResolvedPosition::from_flat(&view, end) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                Selection::new(Position::from(&start_pos), Position::from(&end_pos))
            };
            if tr.selection() != Some(selection) {
                tr.clear_pending_format()?;
            }
            tr.set_selection(Some(selection))?;
            Ok(())
        }),
        SelectionOp::ExtendTo {
            anchor,
            head_page,
            head_x,
            head_y,
            base_selection,
            allow_collapse,
        } => {
            let selection = resolve_extend_to_selection(
                editor,
                anchor,
                head_page,
                head_x,
                head_y,
                base_selection,
                allow_collapse,
            );
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection
                    && tr.selection() != Some(selection)
                {
                    tr.clear_pending_format()?;
                    tr.set_selection(Some(selection))?;
                }
                Ok(())
            })
        }
        SelectionOp::SelectUnitAt { page, x, y, unit } => {
            let selection = resolve_select_unit_at_selection(editor, page, x, y, unit);
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                if let Some(selection) = selection {
                    if tr.selection() != Some(selection) {
                        tr.clear_pending_format()?;
                    }
                    tr.set_selection(Some(selection))?;
                }
                Ok(())
            })
        }
        SelectionOp::Expand { unit } => {
            let resource = editor.resource.clone();
            editor.transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                let before = tr.selection();
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
                if tr.selection() != before {
                    tr.clear_pending_format()?;
                }
                Ok(())
            })
        }
        SelectionOp::Unset => unreachable!("handled above"),
    }
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
            Some(
                resolve_word_selection_expansion(&hit, &editor.state.view(), &resource)
                    .unwrap_or(hit),
            )
        }
        SelectionPointUnit::Sentence => {
            let resource = editor.resource.lock().unwrap();
            Some(
                resolve_sentence_selection_expansion(&hit, &editor.state.view(), &resource)
                    .unwrap_or(hit),
            )
        }
        SelectionPointUnit::Paragraph => {
            Some(resolve_paragraph_selection_expansion(&hit, &editor.state.view()).unwrap_or(hit))
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
    allow_collapse: bool,
) -> Option<Selection> {
    let view = editor.state().view();
    let base_cell_rect_anchor = base_selection
        .as_ref()
        .and_then(|selection| selection.resolve(&view))
        .and_then(|resolved| resolved.as_cell_rect())
        .map(|cell_rect| cell_rect.anchor_cell.id());
    let started_from_cell_rect = base_cell_rect_anchor.is_some();

    if let Some(anchor_cell) =
        base_cell_rect_anchor.or_else(|| enclosing_table_cell(&view, anchor.node))
        && let Some(table_id) = enclosing_table(&view, anchor_cell)
    {
        let head_inside_table = editor
            .view
            .node_box_contains(head_page, head_x, head_y, table_id);
        if head_inside_table {
            let cells = table_cell_ids(&view, anchor_cell);
            if let Some(head_cell) = editor
                .view
                .nearest_node_box(head_page, head_x, head_y, &cells)
            {
                let is_cell_mode = started_from_cell_rect
                    || editor
                        .state
                        .selection
                        .as_ref()
                        .and_then(|selection| selection.resolve(&view))
                        .is_some_and(|resolved| resolved.as_cell_rect().is_some());
                let is_same_cell_content_hit = head_cell == anchor_cell
                    && !is_cell_mode
                    && editor
                        .view
                        .hit_test_extending(editor.state(), &anchor, head_page, head_x, head_y)
                        .is_some_and(|hit| {
                            selection_endpoints_inside_cell(&view, &hit, anchor_cell)
                        });
                if (head_cell != anchor_cell || is_cell_mode || !is_same_cell_content_hit)
                    && let Some(selection) = cell_rect_selection(anchor_cell, head_cell, &view)
                {
                    return Some(selection);
                }
            }
        }
        // head outside table: fall through; normalize promotes anchor to table boundary
    }

    let head_hit =
        editor
            .view
            .hit_test_extending(editor.state(), &anchor, head_page, head_x, head_y)?;

    let selection = if let Some(base_selection) = base_selection {
        extend_base_selection(&view, base_selection, head_hit)?
    } else {
        extend_drag_hit(&view, anchor, head_hit)?
    };
    let selection = selection.normalize(&view).unwrap_or(selection);

    (allow_collapse || !selection.is_collapsed()).then_some(selection)
}

fn selection_endpoints_inside_cell(view: &DocView, selection: &Selection, cell: Dot) -> bool {
    let Some(resolved) = selection.resolve(view) else {
        return false;
    };
    [resolved.anchor().node(), resolved.head().node()]
        .into_iter()
        .all(|node| enclosing_table_cell(view, node) == Some(cell))
}

fn extend_base_selection(
    doc: &DocView,
    base_selection: Selection,
    head_hit: Selection,
) -> Option<Selection> {
    let initial = base_selection.resolve(doc)?;
    let head = head_hit.resolve(doc)?;
    let initial_from = Position::from(initial.from());
    let initial_to = Position::from(initial.to());

    if head.from() < initial.from() {
        extend_drag_hit(doc, initial_to, head_hit)
    } else if head.to() > initial.to() {
        extend_drag_hit(doc, initial_from, head_hit)
    } else {
        Some(Selection::new(initial_from, initial_to))
    }
}

fn extend_drag_hit(doc: &DocView, anchor: Position, hit: Selection) -> Option<Selection> {
    let anchor_resolved = anchor.resolve(doc)?;
    let hit_resolved = hit.resolve(doc)?;

    if &anchor_resolved <= hit_resolved.from() {
        Some(Selection::new(anchor, Position::from(hit_resolved.to())))
    } else if &anchor_resolved >= hit_resolved.to() {
        Some(Selection::new(anchor, Position::from(hit_resolved.from())))
    } else {
        Some(Selection::new(
            Position::from(hit_resolved.from()),
            Position::from(hit_resolved.to()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{Modifier, PlainNode};
    use editor_state::{
        Affinity, Composition, PendingModifier, Position, Selection, StableSelection,
        is_unit_node_selection,
    };

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn assert_table_selection_visible(editor: &Editor, table: Dot) {
        let view = editor.state().view();
        let sel = editor.state().selection.expect("selection exists in test");
        let resolved = sel.resolve(&view).expect("selection resolves in test");
        let rects = editor.view.selection_rects(&resolved);
        assert!(
            rects
                .iter()
                .any(|rect| rect.meta == editor_view::SelectionRectKind::Block),
            "table selection should render as a block rect, got {rects:?}"
        );
        let block_state = editor.block_state().expect("block state exists in test");
        assert!(
            block_state
                .nodes
                .iter()
                .any(|block| block.id == table && matches!(block.node, PlainNode::Table(_))),
            "block state should include the selected table, got {:?}",
            block_state.nodes
        );
    }

    #[test]
    fn probe_set_same_selection_noop() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let sel = Selection::collapsed(Position::new(p1, 2));
        assert_probe_predicts_apply(
            state,
            Message::Selection {
                op: SelectionOp::Set { selection: sel },
            },
        );
    }

    #[test]
    fn probe_set_different_selection_changes() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let sel = Selection::collapsed(Position::new(p1, 4));
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
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
            doc { root { p1: paragraph { text("hi") } } }
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
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let target = Selection::collapsed(Position::new(p1, 3));
        let frozen = StableSelection::capture(&target, &state.view());
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::SetFrozen { selection: frozen },
        });
        assert_eq!(editor.state().selection, Some(target));
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn select_set_with_unaddressable_node_is_noop() {
        // A node id that doesn't exist in the doc (e.g. left over from a
        // previous session) must not panic — the request is silently dropped.
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let before = state.selection;
        let bogus = Selection::collapsed(Position::new(Dot::new(u64::MAX, 1), 0));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: bogus },
        });
        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn select_set() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let target = Selection::collapsed(Position::new(p1, 3));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });
        assert_eq!(editor.state().selection, Some(target));
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn set_at_sets_hit_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

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
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn select_unit_at_word_expands_hit_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

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
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_with_base_selection_expands_word_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

        let initial = editor.state().selection.expect("selection exists in test");
        assert_ne!(initial.anchor, initial.head);

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: initial.anchor,
                head_page: 0,
                head_x: 9999.0,
                head_y: 30.0,
                base_selection: Some(initial),
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, initial.anchor);
        assert_eq!(sel.head.node, initial.head.node);
        assert!(
            sel.head.offset > initial.head.offset,
            "selection should extend beyond the initially selected word, got {:?}",
            sel
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_without_base_selection_uses_anchor_position() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 1),
                head_page: 0,
                head_x: 9999.0,
                head_y: 30.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p1, 1));
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_without_base_selection_ignores_collapsed_result() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let before = editor.state().selection;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: before.expect("selection exists in test").anchor,
                head_page: 0,
                head_x: 20.0,
                head_y: -100.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(!before.expect("selection exists in test").is_collapsed());
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_visual_paragraph_break_selects_next_paragraph_start() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("aa") }
                    p2: paragraph { text("bb") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let rect = {
            let view = editor.state.view();
            let paragraph_break =
                editor_state::paragraph_break_at_end(&Position::new(p1, 2), &view)
                    .expect("P -> P has PB");
            let resolved = paragraph_break
                .resolve(&view)
                .expect("paragraph break selection resolves");
            editor
                .view
                .selection_rects(&resolved)
                .into_iter()
                .find(|rect| rect.meta == editor_view::SelectionRectKind::ParagraphBreak)
                .expect("paragraph break rect exists")
        };

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 0),
                head_page: rect.page_idx,
                head_x: rect.rect.right() + 4.0,
                head_y: rect.rect.y + rect.rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            sel,
            Selection::new(
                Position::new(p1, 0),
                Position {
                    node: p2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                }
            )
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_visual_removable_empty_paragraph_break_stops_before_next_block() {
        let (state, p1, empty) = state! {
            doc {
                root {
                    p1: paragraph { text("bb") }
                    empty: paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let (paragraph_break, rect) = {
            let view = editor.state.view();
            let paragraph_break =
                editor_state::paragraph_break_at_end(&Position::new(empty, 0), &view)
                    .expect("removable empty paragraph has PB");
            let resolved = paragraph_break
                .resolve(&view)
                .expect("paragraph break selection resolves");
            let rect = editor
                .view
                .selection_rects(&resolved)
                .into_iter()
                .find(|rect| rect.meta == editor_view::SelectionRectKind::ParagraphBreak)
                .expect("paragraph break rect exists");
            (paragraph_break, rect)
        };

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 0),
                head_page: rect.page_idx,
                head_x: rect.rect.right() + 4.0,
                head_y: rect.rect.y + rect.rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            sel,
            Selection::new(Position::new(p1, 0), paragraph_break.head)
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_with_allow_collapse_applies_collapsed_result() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let before = editor.state().selection.expect("selection exists in test");

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: before.anchor,
                head_page: 0,
                head_x: 20.0,
                head_y: -100.0,
                base_selection: None,
                allow_collapse: true,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            sel.is_collapsed(),
            "expected collapsed selection, got {sel:?}"
        );
        assert_ne!(sel, before);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_paragraph_gap_from_text_middle_selects_paragraph_break() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let p1_rect = editor.view.node_box_rects(&[p1])[0].rect;
        let p2_rect = editor.view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: p1_rect.x + p1_rect.width / 2.0,
                head_y: (p1_rect.bottom() + p2_rect.y) / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(!sel.is_collapsed(), "block-gap drag collapsed to {:?}", sel);
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(
            sel.head,
            Position {
                node: p2,
                offset: 0,
                affinity: Affinity::Upstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_up_to_paragraph_gap_from_text_middle_excludes_previous_paragraph_break() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
            }
            selection: (p2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let p1_rect = editor.view.node_box_rects(&[p1])[0].rect;
        let p2_rect = editor.view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p2, 2),
                head_page: 0,
                head_x: p2_rect.x + p2_rect.width / 2.0,
                head_y: (p1_rect.bottom() + p2_rect.y) / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(!sel.is_collapsed(), "block-gap drag collapsed to {:?}", sel);
        assert_eq!(sel.anchor, Position::new(p2, 2));
        assert_eq!(
            sel.head,
            Position {
                node: p2,
                offset: 0,
                affinity: Affinity::Downstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_top_margin_from_document_middle_selects_to_document_start() {
        let (state, p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph { text("hello world") }
                    _p2: paragraph { text("later text") }
                }
            }
            selection: (_p2, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let p1_rect = editor.view.node_box_rects(&[p1])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(_p2, 5),
                head_page: 0,
                head_x: p1_rect.x + p1_rect.width / 2.0,
                head_y: -100.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            !sel.is_collapsed(),
            "top-margin drag collapsed to {:?}",
            sel
        );
        assert_eq!(sel.anchor, Position::new(_p2, 5));
        assert_eq!(sel.head, Position::new(p1, 0));
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_monolithic_internal_paragraph_gap_selects_paragraph_break() {
        let (state, callout, p1, p2) = state! {
            doc {
                root {
                    callout: callout {
                        p1: paragraph { text("one") }
                        p2: paragraph { text("two") }
                    }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let callout_rect = editor.view.node_box_rects(&[callout])[0].rect;
        let p1_rect = editor.view.node_box_rects(&[p1])[0].rect;
        let p2_rect = editor.view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 1),
                head_page: 0,
                head_x: callout_rect.x + callout_rect.width / 2.0,
                head_y: (p1_rect.bottom() + p2_rect.y) / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(!sel.is_collapsed(), "block-gap drag collapsed to {:?}", sel);
        assert_eq!(sel.anchor, Position::new(p1, 1));
        assert_eq!(
            sel.head,
            Position {
                node: p2,
                offset: 0,
                affinity: Affinity::Upstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_empty_paragraph_from_text_middle_uses_empty_caret() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("before") }
                    p2: paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let p2_metrics = editor
            .view
            .cursor_metrics(&editor.state, &Position::new(p2, 0))
            .expect("empty paragraph has cursor metrics");

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: p2_metrics.page_idx,
                head_x: p2_metrics.caret.x,
                head_y: p2_metrics.line.y + p2_metrics.line.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(
            sel.head,
            Position {
                node: p2,
                offset: 0,
                affinity: Affinity::Upstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_to_trailing_empty_line_uses_paragraph_caret() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("before") hard_break }
                    p2: paragraph { text("after") }
                }
            }
            selection: (p2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let line_metrics = editor
            .view
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("trailing empty line has cursor metrics");

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p2, 2),
                head_page: line_metrics.page_idx,
                head_x: line_metrics.caret.x,
                head_y: line_metrics.line.y + line_metrics.line.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p2, 2));
        assert_eq!(sel.head.node, p1);
        assert_eq!(sel.head.offset, 2);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_before_callout_to_gap_after_callout_selects_callout() {
        let (state, p1, callout, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("before") }
                    callout: callout {
                        paragraph { text("inside") }
                    }
                    p2: paragraph { text("after") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let callout_rect = editor.view.node_box_rects(&[callout])[0].rect;
        let p2_rect = editor.view.node_box_rects(&[p2])[0].rect;
        assert!(
            callout_rect.bottom() < p2_rect.y,
            "test requires a visible block gap"
        );

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: callout_rect.x + callout_rect.width / 2.0,
                head_y: (callout_rect.bottom() + p2_rect.y) / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(
            sel.head,
            Position {
                node: Dot::ROOT,
                offset: 2,
                affinity: Affinity::Upstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_after_callout_to_gap_after_callout_excludes_callout() {
        let (state, callout, p2) = state! {
            doc {
                root {
                    paragraph { text("before") }
                    callout: callout {
                        paragraph { text("inside") }
                    }
                    p2: paragraph { text("after") }
                }
            }
            selection: (p2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let callout_rect = editor.view.node_box_rects(&[callout])[0].rect;
        let p2_rect = editor.view.node_box_rects(&[p2])[0].rect;
        assert!(
            callout_rect.bottom() < p2_rect.y,
            "test requires a visible block gap"
        );

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p2, 2),
                head_page: 0,
                head_x: p2_rect.x + p2_rect.width / 2.0,
                head_y: (callout_rect.bottom() + p2_rect.y) / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p2, 2));
        assert_eq!(sel.head, Position::new(p2, 0));
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_before_table_to_inside_table_selects_table() {
        let (state, p1, table, cell) = state! {
            doc {
                root {
                    p1: paragraph { text("before") }
                    table: table {
                        table_row {
                            cell: table_cell { paragraph { text("inside") } }
                        }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_rect = editor.view.node_box_rects(&[cell])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: cell_rect.x + cell_rect.width / 2.0,
                head_y: cell_rect.y + cell_rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(
            sel.head,
            Position {
                node: Dot::ROOT,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            "dragging from before the table into a cell should select the whole table"
        );
        assert_eq!(editor.state().view().node(table).unwrap().index(), Some(1));
        assert_table_selection_visible(&editor, table);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_before_table_to_inside_multi_cell_table_selects_table() {
        let (state, p1, table, cell) = state! {
            doc {
                root {
                    p1: paragraph { text("before") }
                    table: table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            cell: table_cell { paragraph { text("b") } }
                        }
                        table_row {
                            table_cell { paragraph { text("c") } }
                            table_cell { paragraph { text("d") } }
                        }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_rect = editor.view.node_box_rects(&[cell])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: cell_rect.x + cell_rect.width / 2.0,
                head_y: cell_rect.y + cell_rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(
            sel.head,
            Position {
                node: Dot::ROOT,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            "dragging from before the table into any cell should select the whole table"
        );
        assert_table_selection_visible(&editor, table);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_after_table_to_inside_table_selects_table() {
        let (state, table, cell, p2) = state! {
            doc {
                root {
                    paragraph { text("before") }
                    table: table {
                        table_row {
                            cell: table_cell { paragraph { text("inside") } }
                        }
                    }
                    p2: paragraph { text("after") }
                }
            }
            selection: (p2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_rect = editor.view.node_box_rects(&[cell])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p2, 2),
                head_page: 0,
                head_x: cell_rect.x + cell_rect.width / 2.0,
                head_y: cell_rect.y + cell_rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, Position::new(p2, 2));
        assert_eq!(
            sel.head,
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            "dragging from after the table into a cell should select the whole table"
        );
        assert_eq!(editor.state().view().node(table).unwrap().index(), Some(1));
        assert_table_selection_visible(&editor, table);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_before_table_boundary_to_inside_table_selects_table() {
        let (state, root, table, cell) = state! {
            doc {
                root: root {
                    paragraph { text("before") }
                    table: table {
                        table_row {
                            cell: table_cell { paragraph { text("inside") } }
                        }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (root, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_rect = editor.view.node_box_rects(&[cell])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(root, 1),
                head_page: 0,
                head_x: cell_rect.x + cell_rect.width / 2.0,
                head_y: cell_rect.y + cell_rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            is_unit_node_selection(&sel, &editor.state().view()),
            "dragging from the table's front boundary into a cell should select the table, got {sel:?}"
        );
        assert_eq!(editor.state().view().node(table).unwrap().index(), Some(1));
        assert_table_selection_visible(&editor, table);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_inside_cell_to_same_cell_text_stays_textual() {
        let (state, p1) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { p1: paragraph { text("inside") } }
                            table_cell { paragraph { text("next") } }
                        }
                    }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let text_rect = {
            let view = editor.state().view();
            let text_selection = Selection::new(Position::new(p1, 3), Position::new(p1, 5));
            let resolved = text_selection.resolve(&view).unwrap();
            editor.view.selection_rects(&resolved)[0].rect
        };
        let head_x = text_rect.x + text_rect.width / 2.0;
        let head_y = text_rect.y + text_rect.height / 2.0;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x,
                head_y,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        let view = editor.state().view();
        let resolved = sel.resolve(&view).unwrap();
        assert!(
            resolved.as_cell_rect().is_none(),
            "dragging within the same cell's text should stay textual, got {sel:?}"
        );
        assert!(
            !sel.is_collapsed(),
            "same-cell text drag collapsed to {sel:?}"
        );
        assert_eq!(sel.anchor, Position::new(p1, 2));
        assert_eq!(sel.head.node, p1);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_inside_cell_to_same_cell_padding_selects_single_cell() {
        let (state, cell, p1) = state! {
            doc {
                root {
                    table {
                        table_row {
                            cell: table_cell { p1: paragraph { text("inside") } }
                            table_cell { paragraph { text("next") } }
                        }
                        table_row {
                            table_cell { paragraph { text("below") } }
                            table_cell { paragraph { text("more") } }
                        }
                    }
                }
            }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_rect = editor.view.node_box_rects(&[cell])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: cell_rect.x + cell_rect.width / 2.0,
                head_y: cell_rect.y + 4.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        let view = editor.state().view();
        let cell_rect = sel
            .resolve(&view)
            .and_then(|resolved| resolved.as_cell_rect())
            .expect("same-cell padding should select the cell, got {sel:?}");
        assert_eq!(cell_rect.anchor_cell.id(), cell);
        assert_eq!(cell_rect.head_cell.id(), cell);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_inside_cell_to_other_cell_selects_cell_rect() {
        let (state, c00, p1, c01) = state! {
            doc {
                root {
                    table {
                        table_row {
                            c00: table_cell { p1: paragraph { text("inside") } }
                            c01: table_cell { paragraph { text("next") } }
                        }
                        table_row {
                            table_cell { paragraph { text("below") } }
                            table_cell { paragraph { text("more") } }
                        }
                    }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let c01_rect = editor.view.node_box_rects(&[c01])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: c01_rect.x + c01_rect.width / 2.0,
                head_y: c01_rect.y + c01_rect.height / 2.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        let view = editor.state().view();
        let cell_rect = sel
            .resolve(&view)
            .and_then(|resolved| resolved.as_cell_rect())
            .expect("dragging to another cell should select a cell rect");
        assert_eq!(cell_rect.anchor_cell.id(), c00);
        assert_eq!(cell_rect.head_cell.id(), c01);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_cell_rect_uses_base_selection_shape_after_leaving_table() {
        let (state, table, c00, _c01, c10, p2) = state! {
            doc {
                root {
                    table: table {
                        table_row {
                            c00: table_cell { paragraph { text("a") } }
                            _c01: table_cell { paragraph { text("b") } }
                        }
                        table_row {
                            c10: table_cell { paragraph { text("c") } }
                            table_cell { paragraph { text("d") } }
                        }
                    }
                    p2: paragraph { text("after") }
                }
            }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let initial = {
            let view = editor.state().view();
            cell_rect_selection(c00, c00, &view).expect("single-cell selection builds")
        };
        let outside = Selection::new(initial.anchor, Position::new(p2, 2));
        editor
            .transact(|tr| {
                tr.update_meta(|m| m.history = HistoryMeta::Skip);
                tr.set_selection(Some(outside))?;
                Ok(())
            })
            .unwrap();
        let target_rect = editor.view.node_box_rects(&[c10])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: initial.anchor,
                head_page: 0,
                head_x: target_rect.x + target_rect.width / 2.0,
                head_y: target_rect.y + target_rect.height / 2.0,
                base_selection: Some(initial),
                allow_collapse: false,
            },
        });

        let view = editor.state().view();
        let sel = editor.state().selection.expect("selection exists in test");
        let rect = sel
            .resolve(&view)
            .and_then(|resolved| resolved.as_cell_rect())
            .expect("re-entering the table should restore cell-rect selection");
        assert_eq!(rect.table_id(), table);
        assert_eq!(rect.anchor_cell.id(), c00);
        assert_eq!(rect.head_cell.id(), c10);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn extend_from_inside_callout_to_callout_chrome_selects_callout() {
        let (state, callout, p1) = state! {
            doc {
                root {
                    callout: callout {
                        p1: paragraph { text("inside") }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let callout_rect = editor.view.node_box_rects(&[callout])[0].rect;

        editor.apply(Message::Selection {
            op: SelectionOp::ExtendTo {
                anchor: Position::new(p1, 2),
                head_page: 0,
                head_x: callout_rect.x + callout_rect.width / 2.0,
                head_y: callout_rect.y + 4.0,
                base_selection: None,
                allow_collapse: false,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(is_unit_node_selection(&sel, &editor.state().view()));
        assert_eq!(
            sel.anchor,
            Position {
                node: Dot::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            }
        );
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn expand_word_expands_selection_inside_one_word() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Word,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 5);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn expand_word_does_not_require_layout() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 3)
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
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn expand_word_does_not_shrink_selection_across_words() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 8)
        };
        let before = state.selection;
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::Word,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn expand_sentence_expands_selection_inside_one_sentence() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello world. Next sentence.") } } }
            selection: (p1, 1) -> (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

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
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn expand_all_selects_document_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } p2: paragraph { text("world") } } }
            selection: (p1, 1) -> (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

        editor.apply(Message::Selection {
            op: SelectionOp::Expand {
                unit: SelectionExpansionUnit::All,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.offset, 2);
        assert!(!editor.undo_history.can_undo());
    }

    #[test]
    fn selection_op_unset_cascades_composition_pending_selection() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
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
    fn selection_op_set_to_different_position_clears_pending_format() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(initial);

        editor
            .transact(|tr| {
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();

        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(p1, 4)),
            },
        });

        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending modifiers cleared"
        );
    }

    #[test]
    fn selection_op_set_to_same_position_preserves_pending_format() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(initial);
        let pending_modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];

        editor
            .transact(|tr| {
                tr.set_pending_modifiers(pending_modifiers.clone())?;
                Ok(())
            })
            .unwrap();

        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(p1, 3)),
            },
        });

        assert_eq!(editor.state().pending_modifiers, pending_modifiers);
    }

    #[test]
    fn selection_op_unset_undo_restores_selection_and_dissolves_composition_and_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
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
        editor.undo_history =
            editor_state::undo::UndoHistory::new(editor_common::time::Duration::from_millis(300));

        let pre_unset_selection = editor.state().selection;

        editor.apply(Message::Selection {
            op: SelectionOp::Unset,
        });

        assert!(editor.state().selection.is_none(), "selection cleared");
        assert!(editor.state().composition.is_none(), "composition cleared");
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending cleared"
        );

        assert!(editor.undo_history.can_undo(), "Unset must be undoable");
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_eq!(
            editor.state().selection,
            pre_unset_selection,
            "selection restored"
        );
        assert!(
            editor.state().composition.is_none(),
            "composition dissolved on undo, not restored"
        );
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending dissolved on undo, not restored"
        );
    }
}

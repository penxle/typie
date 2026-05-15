use editor_state::{Selection, cell_rect_selection, enclosing_table_cell, table_cell_ids};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

/// The anchor cell for the current drag: the live cell-rect's anchor cell if
/// the selection is already a cell-rect (the latch's source of truth — also
/// correct after Shift-click, where `drag_anchor` is a `(TableRow, _)`
/// position with no `TableCell` ancestor), else the cell enclosing the drag
/// anchor position.
fn drag_anchor_cell(
    editor: &Editor,
    drag_anchor: &editor_state::Position,
) -> Option<editor_model::NodeId> {
    if let Some(resolved) = editor.state.selection.resolve(&editor.state.doc)
        && let Some(cr) = resolved.as_cell_rect()
    {
        return Some(cr.anchor_cell.id());
    }
    enclosing_table_cell(&editor.state.doc, drag_anchor.node_id)
}

pub fn handle_pointer_event(editor: &mut Editor, event: PointerEvent) -> Result<(), EditorError> {
    match event {
        PointerEvent::Down {
            page,
            x,
            y,
            count,
            modifiers,
        } => {
            let raw_hit = editor.view.hit_test(page, x, y);
            let ext_hit = editor.view.hit_test_extending(page, x, y);

            let selection = match count {
                0 => return Ok(()),
                1 => {
                    if modifiers.shift {
                        let anchor = editor.state.selection.anchor;
                        let cell_sel = drag_anchor_cell(editor, &anchor).and_then(|a| {
                            let cells = table_cell_ids(&editor.state.doc, a);
                            editor
                                .view
                                .nearest_node_box(page, x, y, &cells)
                                .filter(|&h| h != a)
                                .and_then(|h| cell_rect_selection(&editor.state.doc, a, h))
                        });
                        cell_sel
                            .or_else(|| ext_hit.as_ref().map(|h| Selection::new(anchor, h.head)))
                    } else {
                        raw_hit
                    }
                }
                2 => {
                    let resolved = raw_hit
                        .as_ref()
                        .and_then(|s| s.head.resolve(&editor.state.doc));
                    let resource = editor.resource.lock().unwrap();
                    resolved
                        .and_then(|rp| editor.view.select_word_at(&rp, &resource))
                        .or(raw_hit)
                }
                3.. => {
                    let pos = raw_hit.as_ref().map(|s| &s.head);
                    pos.and_then(|p| editor.view.select_paragraph_at(p))
                        .or(raw_hit)
                }
            };

            if let Some(new_selection) = selection {
                editor.view.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }

            // Drag anchor is promotion-aware: a drag started in the gutter near
            // a monolithic block must anchor at the block boundary, not at the
            // nearest leaf inside it — otherwise the promoted Move forms an
            // adjacent slot/descendant range that normalize collapses. Plain
            // (non-promoting) positions make ext_hit == raw_hit, so the anchor
            // is unchanged where promotion does not apply.
            editor.drag_anchor = (count == 1).then(|| {
                if modifiers.shift {
                    editor.state.selection.anchor
                } else {
                    ext_hit
                        .as_ref()
                        .map(|h| h.head)
                        .unwrap_or(editor.state.selection.anchor)
                }
            });
        }

        PointerEvent::Move { page, x, y } => {
            let Some(anchor) = editor.drag_anchor else {
                return Ok(());
            };

            if let Some(a) = drag_anchor_cell(editor, &anchor) {
                let cells = table_cell_ids(&editor.state.doc, a);
                if let Some(h) = editor.view.nearest_node_box(page, x, y, &cells) {
                    let is_cell_mode = editor
                        .state
                        .selection
                        .resolve(&editor.state.doc)
                        .is_some_and(|r| r.as_cell_rect().is_some());
                    if (h != a || is_cell_mode)
                        && let Some(sel) = cell_rect_selection(&editor.state.doc, a, h)
                    {
                        editor.view.clear_preferred_x();
                        editor.transact(|tr| {
                            tr.set_selection(sel)?;
                            Ok(())
                        })?;
                        return Ok(());
                    }
                }
            }

            if let Some(hit) = editor.view.hit_test_extending(page, x, y) {
                let new_selection = Selection::new(anchor, hit.head);
                editor.view.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(new_selection)?;
                    Ok(())
                })?;
            }
        }

        PointerEvent::Up => {
            editor.drag_anchor = None;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn double_click_fallback_when_no_layout() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 2,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn triple_click_fallback_when_no_layout() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 3,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn high_click_count_treated_as_paragraph() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 10.0,
                count: 5,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn shift_click_extends_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 9999.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        });

        let sel = editor.state().selection;
        assert_eq!(sel.anchor, anchor);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn drag_extends_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(sel.anchor, anchor);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn drag_anchor_survives_intermediate_collapse() {
        use editor_state::Position;

        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let drag_anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.state.selection = Selection::collapsed(Position::new(t, 11));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor, drag_anchor,
            "drag anchor must survive an intermediate collapse"
        );
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn move_without_drag_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 50.0,
                y: 5.0,
            },
        });

        assert_eq!(editor.state().selection, before);
        assert!(editor.drag_anchor.is_none());
    }

    #[test]
    fn up_resets_dragging() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 0.0,
                y: 0.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });

        assert!(editor.drag_anchor.is_none());
    }

    #[test]
    fn drag_envelopes_leading_fold_without_anchor_collapse() {
        let (state, ta) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;
        assert!(editor.drag_anchor.is_some());

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "drag anchor must not jump to another node"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert_eq!(
            sel.anchor.node_id, ta,
            "anchor must remain in the trailing paragraph text"
        );
        assert!(
            !sel.is_collapsed(),
            "selection enveloping the leading fold must survive normalize, got {:?}",
            sel
        );
    }

    #[test]
    fn plain_gutter_down_does_not_promote_to_block() {
        let (state, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: -9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let sel = editor.state().selection;
        assert!(sel.is_collapsed());
        assert_ne!(
            sel.head.node_id,
            editor_model::NodeId::ROOT,
            "plain gutter Down must not promote to a container slot"
        );
    }

    #[test]
    fn drag_started_in_gutter_above_leading_fold_envelopes_it() {
        let (state, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after the fold") }
                }
            }
            selection: (ta, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: -9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            editor.drag_anchor.is_some(),
            "Down must arm a drag anchor (count==1)"
        );

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection;
        assert!(
            !sel.is_collapsed(),
            "gutter-started drag must envelope the leading fold, not collapse, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor.node_id,
            editor_model::NodeId::ROOT,
            "drag_anchor must use ext_hit (promoted block boundary = ROOT slot 0), \
             not raw_hit (a leaf inside fold_title): this assertion is the load-bearing \
             discriminator for the promotion-aware drag_anchor — !is_collapsed() alone \
             would still pass under a raw_hit regression"
        );
        assert_eq!(
            sel.anchor.offset, 0,
            "promoted Front slot of the leading fold is offset 0 under ROOT"
        );
    }

    #[test]
    fn drag_below_envelopes_fold_with_trailing_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { ta: text("before") }
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { text("after") }
                }
            }
            selection: (ta, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 5.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "anchor must stay in the leading paragraph (affinity may flip)"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert!(
            !sel.is_collapsed(),
            "drag below a fold (with trailing paragraph) must span it, got {:?}",
            sel
        );
    }

    #[test]
    fn drag_up_past_fold_with_textless_neighbor_envelopes_it() {
        let (state, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("body") } }
                    }
                    paragraph { ta: text("after") }
                }
            }
            selection: (ta, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 5.0,
                y: 9999.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let anchor = editor.state().selection.anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection;
        assert_eq!(
            sel.anchor.node_id, anchor.node_id,
            "anchor must stay in the trailing paragraph (affinity may flip)"
        );
        assert_eq!(
            sel.anchor.offset, anchor.offset,
            "anchor offset must be stable"
        );
        assert!(
            !sel.is_collapsed(),
            "drag up past a fold with a text-less neighbor must span it, got {:?}",
            sel
        );
    }

    fn cell_center(editor: &Editor, cell: editor_model::NodeId) -> (f32, f32) {
        let r = editor.view.node_box_rects(&[cell])[0].rect;
        (r.x + r.width / 2.0, r.y + r.height / 2.0)
    }

    fn table_state() -> (
        editor_state::State,
        editor_model::NodeId,
        editor_model::NodeId,
    ) {
        let (state, _, c00, _, _, _, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { t00: text("aa") } }
                    c01: table_cell { paragraph { text("bb") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("cc") } }
                    c11: table_cell { paragraph { text("dd") } }
                }
            } } }
            selection: (t00, 0)
        };
        (state, c00, c11)
    }

    #[test]
    fn drag_across_cells_produces_cell_rect() {
        let (state, c00, c11) = table_state();
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let (ax, ay) = cell_center(&editor, c00);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: ax,
                y: ay,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let (hx, hy) = cell_center(&editor, c11);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: hx,
                y: hy,
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        let cr = resolved
            .as_cell_rect()
            .expect("cross-cell drag must be a cell-rect");
        assert_eq!(cr.anchor_cell.id(), c00);
        assert_eq!(cr.head_cell.id(), c11);
    }

    #[test]
    fn drag_within_one_cell_stays_text() {
        let (state, c00, _) = table_state();
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let r = editor.view.node_box_rects(&[c00])[0].rect;
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: r.x + 2.0,
                y: r.y + r.height / 2.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: r.x + r.width - 2.0,
                y: r.y + r.height / 2.0,
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        assert!(
            resolved.as_cell_rect().is_none(),
            "same-cell drag stays text"
        );
    }

    #[test]
    fn drag_outside_table_keeps_clamped_cell_rect_no_revert() {
        let (state, c00, c11) = table_state();
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let (ax, ay) = cell_center(&editor, c00);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: ax,
                y: ay,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let (hx, hy) = cell_center(&editor, c11);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: hx,
                y: hy,
            },
        });
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 99999.0,
                y: 99999.0,
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        assert!(
            resolved.as_cell_rect().is_some(),
            "leaving the table must keep a clamped cell-rect, not revert to text"
        );
    }

    #[test]
    fn shift_click_other_cell_produces_cell_rect() {
        let (state, c00, c11) = table_state();
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let (ax, ay) = cell_center(&editor, c00);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: ax,
                y: ay,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let (hx, hy) = cell_center(&editor, c11);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: hx,
                y: hy,
                count: 1,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        let cr = resolved
            .as_cell_rect()
            .expect("shift+click other cell → cell-rect");
        assert_eq!(cr.anchor_cell.id(), c00);
        assert_eq!(cr.head_cell.id(), c11);
    }

    #[test]
    fn drag_back_to_anchor_cell_stays_cell_rect_no_text_revert() {
        let (state, c00, c11) = table_state();
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let (ax, ay) = cell_center(&editor, c00);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: ax,
                y: ay,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        // enter cell mode by crossing into c11
        let (bx, by) = cell_center(&editor, c11);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: bx,
                y: by,
            },
        });
        // drag back into the anchor cell c00
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: ax,
                y: ay,
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        let cr = resolved.as_cell_rect().expect(
            "returning to the anchor cell mid-drag must stay a 1x1 cell-rect, not revert to text",
        );
        // 1x1 rectangle of the anchor cell
        assert_eq!(cr.anchor_cell.id(), c00);
        assert_eq!(cr.head_cell.id(), c00);
    }

    #[test]
    fn shift_click_extends_an_existing_cell_rect_not_collapse_to_text() {
        let (state, _, c00, _, c01, _, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { t00: text("aa") } }
                    c01: table_cell { paragraph { text("bb") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("cc") } }
                    c11: table_cell { paragraph { text("dd") } }
                }
            } } }
            selection: (t00, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        // Enter cell mode: Down in c00, drag to c11 → cell-rect c00..c11.
        let (ax, ay) = cell_center(&editor, c00);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: ax,
                y: ay,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        let (bx, by) = cell_center(&editor, c11);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: bx,
                y: by,
            },
        });
        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });
        // Precondition: selection is now a cell-rect.
        assert!(
            editor
                .state
                .selection
                .resolve(&editor.state.doc)
                .unwrap()
                .as_cell_rect()
                .is_some(),
            "precondition: drag produced a cell-rect"
        );

        // Shift+click into c01 must EXTEND the rectangle (anchor stays c00),
        // not collapse to a text selection.
        let (sx, sy) = cell_center(&editor, c01);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: sx,
                y: sy,
                count: 1,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        });

        let resolved = editor.state.selection.resolve(&editor.state.doc).unwrap();
        let cr = resolved
            .as_cell_rect()
            .expect("shift-click extending an existing cell-rect must stay a cell-rect");
        assert_eq!(
            cr.anchor_cell.id(),
            c00,
            "anchor cell preserved from the existing cell-rect"
        );
        assert_eq!(
            cr.head_cell.id(),
            c01,
            "head extends to the shift-clicked cell"
        );
    }
}

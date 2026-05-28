use editor_state::{
    Selection, cell_rect_selection, enclosing_table_cell, farther_endpoint,
    resolve_paragraph_selection_expansion, resolve_word_selection_expansion, table_cell_ids,
};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::interaction::{InteractionState, PressContext};
use crate::message::*;

const DRAG_START_THRESHOLD_PX: f32 = 5.0;

/// The anchor cell for the current drag: the live cell-rect's anchor cell if
/// the selection is already a cell-rect (the latch's source of truth — also
/// correct after Shift-click, where `pos` is a `(TableRow, _)` position with
/// no `TableCell` ancestor), else the cell enclosing `pos`.
fn drag_anchor_cell(editor: &Editor, pos: &editor_state::Position) -> Option<editor_model::NodeId> {
    if let Some(resolved) = editor
        .state
        .selection
        .as_ref()
        .and_then(|s| s.resolve(&editor.state.doc))
        && let Some(cr) = resolved.as_cell_rect()
    {
        return Some(cr.anchor_cell.id());
    }
    enclosing_table_cell(&editor.state.doc, pos.node_id)
}

fn set_selection(editor: &mut Editor, selection: Selection) -> Result<(), EditorError> {
    editor.clear_preferred_x();
    editor.transact(|tr| {
        tr.set_selection(Some(selection))?;
        Ok(())
    })
}

fn selection_drag_anchor(
    editor: &Editor,
    ext_hit: Option<Selection>,
    shift: bool,
) -> Option<Selection> {
    let anchor = editor.state.selection.map(|s| s.anchor);
    if shift {
        anchor.map(Selection::collapsed)
    } else {
        ext_hit.or_else(|| anchor.map(Selection::collapsed))
    }
}

fn drag_start_threshold_crossed(
    start_page: usize,
    start_x: f32,
    start_y: f32,
    page: usize,
    x: f32,
    y: f32,
) -> bool {
    if page != start_page {
        return true;
    }

    let dx = x - start_x;
    let dy = y - start_y;
    dx * dx + dy * dy >= DRAG_START_THRESHOLD_PX * DRAG_START_THRESHOLD_PX
}

fn extend_drag_selection(
    editor: &mut Editor,
    armed: Selection,
    page: usize,
    x: f32,
    y: f32,
) -> Result<(), EditorError> {
    if let Some(a) = drag_anchor_cell(editor, &armed.head) {
        let cells = table_cell_ids(&editor.state.doc, a);
        if let Some(h) = editor.view.nearest_node_box(page, x, y, &cells) {
            let is_cell_mode = editor
                .state
                .selection
                .as_ref()
                .and_then(|s| s.resolve(&editor.state.doc))
                .is_some_and(|r| r.as_cell_rect().is_some());
            if (h != a || is_cell_mode)
                && let Some(sel) = cell_rect_selection(&editor.state.doc, a, h)
            {
                return set_selection(editor, sel);
            }
        }
    }

    if let Some(hit) = editor.view.hit_test_extending(page, x, y) {
        // Re-anchor to the armed edge farther from the pointer, then take the
        // hit edge farther from that anchor. A unit (image/monolithic) armed
        // click or a unit under the pointer stays fully enveloped whichever
        // way the drag goes; a collapsed text caret or promoted slot has equal
        // endpoints, so both steps degenerate to the plain anchor->hit
        // selection.
        let doc = &editor.state.doc;
        let fixed = farther_endpoint(doc, hit.head, armed.anchor, armed.head);
        let head = farther_endpoint(doc, fixed, hit.anchor, hit.head);
        set_selection(editor, Selection::new(fixed, head))?;
    }

    Ok(())
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
                        if let Some(cur) = editor.state.selection {
                            let anchor = cur.anchor;
                            let cell_sel = drag_anchor_cell(editor, &anchor).and_then(|a| {
                                let cells = table_cell_ids(&editor.state.doc, a);
                                editor
                                    .view
                                    .nearest_node_box(page, x, y, &cells)
                                    .filter(|&h| h != a)
                                    .and_then(|h| cell_rect_selection(&editor.state.doc, a, h))
                            });
                            cell_sel.or_else(|| {
                                ext_hit.as_ref().map(|h| {
                                    let doc = &editor.state.doc;
                                    let fixed = farther_endpoint(doc, h.head, cur.anchor, cur.head);
                                    let head = farther_endpoint(doc, fixed, h.anchor, h.head);
                                    Selection::new(fixed, head)
                                })
                            })
                        } else {
                            raw_hit
                        }
                    } else {
                        raw_hit
                    }
                }
                2 => match raw_hit {
                    Some(hit) => {
                        let resource = editor.resource.lock().unwrap();
                        Some(
                            resolve_word_selection_expansion(&editor.state.doc, hit, &resource)
                                .unwrap_or(hit),
                        )
                    }
                    None => None,
                },
                3.. => match raw_hit {
                    Some(hit) => Some(
                        resolve_paragraph_selection_expansion(&editor.state.doc, hit)
                            .unwrap_or(hit),
                    ),
                    None => None,
                },
            };

            if let Some(new_selection) = selection {
                let position = new_selection.head;
                let selection_hit =
                    count == 1 && !modifiers.shift && editor.selection_hit_test(page, x, y);
                let context = if count == 1
                    && !modifiers.shift
                    && new_selection.is_unit_node_selection(&editor.state.doc)
                {
                    PressContext::OnSelectable(new_selection)
                } else if selection_hit {
                    PressContext::InSelection
                } else {
                    PressContext::Empty
                };
                let should_update_selection_on_down = match context {
                    PressContext::InSelection => false,
                    PressContext::OnSelectable(_) => !selection_hit,
                    PressContext::Empty => true,
                };

                if should_update_selection_on_down {
                    set_selection(editor, new_selection)?;
                }

                let selection_anchor = (count == 1 && !context.can_drag_content())
                    .then(|| selection_drag_anchor(editor, ext_hit, modifiers.shift))
                    .flatten();
                editor.interaction = InteractionState::Pressed {
                    page,
                    start_x: x,
                    start_y: y,
                    position,
                    selection_anchor,
                    context,
                };
            } else {
                editor.interaction = InteractionState::Idle;
            }
        }

        PointerEvent::SecondaryDown { page, x, y } => {
            let Some(hit) = editor.view.hit_test(page, x, y) else {
                return Ok(());
            };

            let next_selection = match editor.state.selection {
                Some(selection)
                    if !selection.is_collapsed() && editor.selection_hit_test(page, x, y) =>
                {
                    None
                }
                _ => Some(hit),
            };

            if let Some(selection) = next_selection {
                editor.clear_preferred_x();
                editor.transact(|tr| {
                    tr.set_selection(Some(selection))?;
                    Ok(())
                })?;
            }
        }

        PointerEvent::Move { page, x, y } => match editor.interaction.clone() {
            InteractionState::Pressed {
                page: start_page,
                start_x,
                start_y,
                selection_anchor: Some(armed),
                context,
                ..
            } if !context.can_drag_content()
                && drag_start_threshold_crossed(start_page, start_x, start_y, page, x, y) =>
            {
                editor.interaction = InteractionState::DraggingSelection { anchor: armed };
                extend_drag_selection(editor, armed, page, x, y)?;
            }
            InteractionState::DraggingSelection { anchor } => {
                extend_drag_selection(editor, anchor, page, x, y)?;
            }
            InteractionState::Pressed { .. }
            | InteractionState::Idle
            | InteractionState::InternalDnd { .. }
            | InteractionState::ExternalDnd { .. } => {}
        },

        PointerEvent::Up => {
            let mut clear_pointer_interaction = false;
            if let InteractionState::Pressed {
                position, context, ..
            } = editor.interaction.clone()
            {
                clear_pointer_interaction = true;
                match context {
                    PressContext::InSelection => {
                        set_selection(editor, Selection::collapsed(position))?;
                    }
                    PressContext::OnSelectable(selection) => {
                        // TODO(TR-100): v2 core pointer messages do not carry
                        // read-only state yet. Legacy collapses to the
                        // selectable anchor in read-only mode.
                        set_selection(editor, selection)?;
                    }
                    PressContext::Empty => {}
                }
            }
            if matches!(
                editor.interaction,
                InteractionState::DraggingSelection { .. }
            ) {
                clear_pointer_interaction = true;
            }
            if clear_pointer_interaction {
                editor.interaction = InteractionState::Idle;
            }
        }

        PointerEvent::Cancel => {
            if matches!(
                editor.interaction,
                InteractionState::Pressed { .. } | InteractionState::DraggingSelection { .. }
            ) {
                editor.interaction = InteractionState::Idle;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    fn selection_drag_armed(editor: &Editor) -> bool {
        matches!(
            editor.interaction,
            InteractionState::Pressed {
                selection_anchor: Some(_),
                ..
            } | InteractionState::DraggingSelection { .. }
        )
    }

    fn pointer_idle(editor: &Editor) -> bool {
        matches!(editor.interaction, InteractionState::Idle)
    }

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
    fn secondary_down_inside_range_keeps_selection() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let caret = editor
            .view
            .cursor_metrics(&editor.state.doc, &editor_state::Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Pointer {
            event: PointerEvent::SecondaryDown {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
            },
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                editor_state::Position::new(t, 0),
                editor_state::Position::new(t, 5),
            ))
        );
    }

    #[test]
    fn secondary_down_outside_range_collapses_to_hit_position() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let caret = editor
            .view
            .cursor_metrics(&editor.state.doc, &editor_state::Position::new(t, 8))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Pointer {
            event: PointerEvent::SecondaryDown {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
            },
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::collapsed(editor_state::Position::new(t, 8)))
        );
    }

    #[test]
    fn secondary_down_outside_node_selection_selects_hit_node() {
        use editor_state::{Affinity, Position};

        let (state, r1, i0, i1) = state! {
            doc { r1: root { i0: image i1: image paragraph {} } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
        editor.view.layout(&editor.state.doc);

        let elements = editor
            .view
            .external_elements(&editor.state.doc, editor.state().selection.as_ref());
        let second_image = &elements[1];

        editor.apply(Message::Pointer {
            event: PointerEvent::SecondaryDown {
                page: second_image.page_idx,
                x: second_image.bounds.x + second_image.bounds.width / 2.0,
                y: second_image.bounds.y + second_image.bounds.height / 2.0,
            },
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: r1,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r1,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn primary_down_inside_range_keeps_selection_until_up() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let caret = editor
            .view
            .cursor_metrics(&editor.state.doc, &editor_state::Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                editor_state::Position::new(t, 0),
                editor_state::Position::new(t, 5),
            )),
            "primary down inside the current selection must preserve it so native DnD can start"
        );

        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::collapsed(editor_state::Position::new(t, 2))),
            "a plain click inside the selection collapses on pointer up"
        );
    }

    #[test]
    fn dnd_start_inside_range_prevents_pointer_up_collapse() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let caret = editor
            .view
            .cursor_metrics(&editor.state.doc, &editor_state::Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                editor_state::Position::new(t, 0),
                editor_state::Position::new(t, 5),
            )),
            "once native DnD starts, the pending click collapse must be canceled"
        );
    }

    #[test]
    fn primary_down_on_unit_selects_it_without_drag_extending() {
        use editor_state::{Affinity, Position};

        let (state, r1, i0, i1) = state! {
            doc { r1: root { i0: image i1: image paragraph {} } }
            selection: (r1, 2)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
        editor.view.layout(&editor.state.doc);

        let atom_center = |editor: &Editor, idx: usize| {
            let elements = editor
                .view
                .external_elements(&editor.state.doc, editor.state().selection.as_ref());
            let e = &elements[idx];
            (
                e.page_idx,
                e.bounds.x + e.bounds.width / 2.0,
                e.bounds.y + e.bounds.height / 2.0,
            )
        };

        let (p0, x0, y0) = atom_center(&editor, 0);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: p0,
                x: x0,
                y: y0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let selected_image = Some(Selection::new(
            Position {
                node_id: r1,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: r1,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ));
        assert_eq!(editor.state().selection, selected_image);

        let (p1, x1, y1) = atom_center(&editor, 1);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: p1,
                x: x1,
                y: y1,
            },
        });

        assert_eq!(
            editor.state().selection,
            selected_image,
            "unit presses are native-DnD candidates, not drag-selection anchors"
        );
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

        let anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;

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

        let sel = editor.state().selection.expect("selection exists in test");
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

        let anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;
        assert!(selection_drag_armed(&editor));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(sel.anchor, anchor);
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn move_within_drag_threshold_keeps_press_pending() {
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
        let selection_after_down = editor.state().selection;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 3.0,
                y: 5.0,
            },
        });

        assert_eq!(editor.state().selection, selection_after_down);
        assert!(matches!(
            editor.interaction,
            InteractionState::Pressed { .. }
        ));
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
        let drag_anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;
        assert!(selection_drag_armed(&editor));

        editor.state.selection = Some(Selection::collapsed(Position::new(t, 11)));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            sel.anchor, drag_anchor,
            "drag anchor must survive an intermediate collapse"
        );
        assert_ne!(sel.anchor, sel.head);
    }

    #[test]
    fn shift_click_first_image_from_selected_second_image_envelops_both() {
        let (state, r1, i0, i1) = state! {
            doc { r1: root { i0: image i1: image paragraph {} } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
        editor.view.layout(&editor.state.doc);

        let atom_center = |editor: &Editor, idx: usize| {
            let elements = editor
                .view
                .external_elements(&editor.state.doc, editor.state().selection.as_ref());
            let e = &elements[idx];
            (
                e.page_idx,
                e.bounds.x + e.bounds.width / 2.0,
                e.bounds.y + e.bounds.height / 2.0,
            )
        };

        let (p0, x0, y0) = atom_center(&editor, 0);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: p0,
                x: x0,
                y: y0,
                count: 1,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        });

        let s = editor.state().selection.expect("selection exists in test");
        let (lo, hi) = if s.anchor.offset <= s.head.offset {
            (s.anchor.offset, s.head.offset)
        } else {
            (s.head.offset, s.anchor.offset)
        };
        assert_eq!(s.anchor.node_id, r1);
        assert_eq!(s.head.node_id, r1);
        assert_eq!(
            (lo, hi),
            (0, 2),
            "shift+click image#0 from image#1 node-selection must envelop both images: \
             anchor must re-anchor to the unit's trailing edge (r1,2), not stay at (r1,1)"
        );
    }

    #[test]
    fn drag_down_from_first_image_keeps_it_selected_for_native_dnd() {
        use editor_state::{Affinity, Position};

        let (state, r1, i0, i1, i2) = state! {
            doc { r1: root { i0: image i1: image i2: image paragraph {} } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1, i2] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
        editor.view.layout(&editor.state.doc);

        let atom_center = |editor: &Editor, idx: usize| {
            let elements = editor
                .view
                .external_elements(&editor.state.doc, editor.state().selection.as_ref());
            let e = &elements[idx];
            (
                e.page_idx,
                e.bounds.x + e.bounds.width / 2.0,
                e.bounds.y + e.bounds.height / 2.0,
            )
        };

        let (p0, x0, y0) = atom_center(&editor, 0);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: p0,
                x: x0,
                y: y0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });
        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: r1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r1,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            )),
            "Down on the first image must node-select it"
        );

        let selected_image = editor.state().selection;
        let (p2, x2, y2) = atom_center(&editor, 2);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: p2,
                x: x2,
                y: y2,
            },
        });

        assert_eq!(
            editor.state().selection,
            selected_image,
            "moving after a unit press must keep the unit selected; browser native DnD owns the content drag"
        );
    }

    #[test]
    fn drag_up_from_trailing_paragraph_to_top_selects_all() {
        use editor_state::{Affinity, Position};

        let (state, r1, i0, i1, i2, p1) = state! {
            doc { r1: root { i0: image i1: image i2: image p1: paragraph {} } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1, i2] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
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
        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            sel.is_collapsed(),
            "Down on the trailing paragraph collapses"
        );
        assert_eq!(sel.head.node_id, p1);
        assert_eq!(sel.head.offset, 0);
        assert!(selection_drag_armed(&editor));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node_id: r1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
            )),
            "dragging up to the top must envelope the first image: \
             head reaches the doc start (r1,0), not the gap after image0 (r1,1)"
        );
    }

    #[test]
    fn drag_up_from_last_image_keeps_it_selected_for_native_dnd() {
        let (state, _, i0, i1, i2) = state! {
            doc { r1: root { i0: image i1: image i2: image paragraph {} } }
            selection: (r1, 2, >) -> (r1, 3, <)
        };
        let mut editor = Editor::new_test(state);
        for id in [i0, i1, i2] {
            editor
                .view
                .set_external_height(&editor.state.doc, id, 100.0);
        }
        editor.view.layout(&editor.state.doc);

        let atom_center = |editor: &Editor, idx: usize| {
            let elements = editor
                .view
                .external_elements(&editor.state.doc, editor.state().selection.as_ref());
            let e = &elements[idx];
            (
                e.page_idx,
                e.bounds.x + e.bounds.width / 2.0,
                e.bounds.y + e.bounds.height / 2.0,
            )
        };

        let (p2, x2, y2) = atom_center(&editor, 2);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: p2,
                x: x2,
                y: y2,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let selected_image = editor.state().selection;
        let (p0, x0, y0) = atom_center(&editor, 0);
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: p0,
                x: x0,
                y: y0,
            },
        });

        assert_eq!(
            editor.state().selection,
            selected_image,
            "moving after a unit press must keep the unit selected; browser native DnD owns the content drag"
        );
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
        assert!(pointer_idle(&editor));
    }

    #[test]
    fn up_resets_dragging() {
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
                y: 0.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(selection_drag_armed(&editor));

        editor.apply(Message::Pointer {
            event: PointerEvent::Up,
        });

        assert!(pointer_idle(&editor));
    }

    #[test]
    fn cancel_resets_dragging() {
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
                y: 0.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(selection_drag_armed(&editor));

        editor.apply(Message::Pointer {
            event: PointerEvent::Cancel,
        });

        assert!(pointer_idle(&editor));
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
        let anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;
        assert!(selection_drag_armed(&editor));

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
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

        let sel = editor.state().selection.expect("selection exists in test");
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
            selection_drag_armed(&editor),
            "Down must arm a drag anchor (count==1)"
        );

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
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
        let anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: 9999.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
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
        let anchor = editor
            .state()
            .selection
            .expect("selection exists in test")
            .anchor;

        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: 5.0,
                y: -9999.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
        let cr = resolved
            .as_cell_rect()
            .expect("shift+click other cell → cell-rect");
        assert_eq!(cr.anchor_cell.id(), c00);
        assert_eq!(cr.head_cell.id(), c11);
    }

    #[test]
    fn primary_down_outside_cell_rect_updates_selection() {
        let (state, _, c00, _, c01, _, c10, _) = state! {
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

        let cell_rect =
            cell_rect_selection(&editor.state.doc, c00, c10).expect("first column cell-rect");
        editor.state.selection = Some(cell_rect);

        let (x, y) = cell_center(&editor, c01);
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x,
                y,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        assert_ne!(
            editor.state().selection,
            Some(cell_rect),
            "clicking outside a cell-rect must not be treated as InSelection"
        );
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
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
                .as_ref()
                .and_then(|s| s.resolve(&editor.state.doc))
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

        let resolved = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .unwrap();
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

    #[test]
    fn drag_down_between_callouts_selects_crossed_callout() {
        use editor_state::{Affinity, Position};

        let (state, c1, _, c2) = state! {
            doc {
                root {
                    c1: callout { paragraph { t1: text("1234") } }
                    c2: callout { paragraph { text("asdf") } }
                    paragraph {}
                }
            }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let c1_rect = editor.view.node_box_rects(&[c1])[0].rect;
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: c1_rect.x + 20.0,
                y: c1_rect.y + c1_rect.height / 2.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let c2_rect = editor.view.node_box_rects(&[c2])[0].rect;
        let between_y = (c1_rect.y + c1_rect.height + c2_rect.y) / 2.0;
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: c1_rect.x + 20.0,
                y: between_y,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            sel.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "drag between two callouts must not collapse into a gap cursor, got {sel:?}"
        );
        let root_id = editor_model::NodeId::ROOT;
        assert_eq!(
            sel,
            Selection::new(
                Position {
                    node_id: root_id,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node_id: root_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
            ),
            "drag past callout 1's bottom must node-select callout 1 (the crossed unit), got {sel:?}"
        );
    }

    #[test]
    fn drag_up_between_callouts_selects_crossed_callout() {
        use editor_state::{Affinity, Position};

        let (state, c1, c2, _) = state! {
            doc {
                root {
                    c1: callout { paragraph { text("1234") } }
                    c2: callout { paragraph { t2: text("asdf") } }
                    paragraph {}
                }
            }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let c2_rect = editor.view.node_box_rects(&[c2])[0].rect;
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: c2_rect.x + 20.0,
                y: c2_rect.y + c2_rect.height / 2.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let c1_rect = editor.view.node_box_rects(&[c1])[0].rect;
        let between_y = (c1_rect.y + c1_rect.height + c2_rect.y) / 2.0;
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: c2_rect.x + 20.0,
                y: between_y,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert!(
            sel.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "drag-up between two callouts must not collapse into a gap cursor, got {sel:?}"
        );
        let root_id = editor_model::NodeId::ROOT;
        assert_eq!(
            sel,
            Selection::new(
                Position {
                    node_id: root_id,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node_id: root_id,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
            ),
            "drag-up past callout 2's top must node-select callout 2 (the crossed unit), got {sel:?}"
        );
    }

    #[test]
    fn drag_down_into_leading_gap_of_fold_keeps_text_selection() {
        // Caret in the empty paragraph above a fold; drag straight down so the
        // pointer lands in the inter-block gap directly above the fold, with x
        // over the fold-title text. The gap is the *approach* to the fold, not
        // an escape past it, so the head must stay a plain text caret in the
        // fold-title text — not promote the whole fold to a ROOT-slot unit
        // selection.
        let (state, p1, f1, _, t1) = state! {
            doc {
                root {
                    p1: paragraph {}
                    f1: fold {
                        ft1: fold_title {
                            t1: text("1234")
                        }
                        fold_content {
                            paragraph {
                                text("12341234")
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let p1_rect = editor.view.node_box_rects(&[p1])[0].rect;
        editor.apply(Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: p1_rect.x + 5.0,
                y: p1_rect.y + p1_rect.height / 2.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        });

        let f1_rect = editor.view.node_box_rects(&[f1])[0].rect;
        editor.apply(Message::Pointer {
            event: PointerEvent::Move {
                page: 0,
                x: f1_rect.x + 50.0,
                y: f1_rect.y - 2.0,
            },
        });

        let sel = editor.state().selection.expect("selection exists in test");
        assert_ne!(
            sel.anchor.node_id,
            editor_model::NodeId::ROOT,
            "anchor must stay a text caret, not a ROOT slot, got {sel:?}"
        );
        assert_ne!(
            sel.head.node_id,
            editor_model::NodeId::ROOT,
            "head must not promote the fold to a ROOT-slot unit selection, got {sel:?}"
        );
        assert_eq!(
            sel.anchor.node_id, p1,
            "anchor stays in the leading empty paragraph"
        );
        assert_eq!(
            sel.head.node_id, t1,
            "head lands in the fold-title text, not on the fold node"
        );
    }
}

#[cfg(test)]
mod tests_probe {
    use editor_macros::state;

    use crate::editor::Editor;
    use crate::message::*;
    use crate::test_utils::EditorSnapshot;

    fn build_pointer_fixture() -> Editor {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let mut e = Editor::new_test(state);
        e.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        e.apply(Message::System {
            event: SystemEvent::Resize {
                width: 800.0,
                height: 600.0,
                scale_factor: 1.0,
            },
        });
        e
    }

    #[test]
    fn probe_pointer_down_is_safe() {
        // Pointer hit-test is layout-dependent and also mutates drag_anchor (non-applicability
        // state). Prediction accuracy is not tested here; only probe-safety is verified.
        let msg = Message::Pointer {
            event: PointerEvent::Down {
                page: 0,
                x: 50.0,
                y: 50.0,
                count: 1,
                modifiers: InputModifiers::default(),
            },
        };
        let mut e = build_pointer_fixture();
        let before = EditorSnapshot::capture(&e);
        let _ = e.can(msg);
        let after = EditorSnapshot::capture(&e);
        assert_eq!(before, after, "probe must not mutate observable state");
    }

    #[test]
    fn probe_pointer_up_with_no_drag() {
        let mut e = build_pointer_fixture();
        let before = EditorSnapshot::capture(&e);
        let _ = e.can(Message::Pointer {
            event: PointerEvent::Up,
        });
        let after = EditorSnapshot::capture(&e);
        assert_eq!(before, after);
    }

    #[test]
    fn probe_pointer_cancel() {
        let mut e = build_pointer_fixture();
        let before = EditorSnapshot::capture(&e);
        let _ = e.can(Message::Pointer {
            event: PointerEvent::Cancel,
        });
        let after = EditorSnapshot::capture(&e);
        assert_eq!(before, after);
    }
}

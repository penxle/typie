use editor_model::{Doc, NodeId, Schema};
use editor_state::{
    Affinity, GapCursor, Position, Selection, cell_rect_selection, enclosing_table_cell,
    gap_cursor_selection_between, gap_cursor_selection_leading,
};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::*;

pub fn handle_navigation_op(editor: &mut Editor, op: NavigationOp) -> Result<(), EditorError> {
    match op {
        NavigationOp::Move { movement, extend } => {
            let Some(selection) = editor.state.selection else {
                if !extend && let Movement::Document { .. } = movement {
                    let probe_pos = Position {
                        node_id: NodeId::ROOT,
                        offset: 0,
                        affinity: Affinity::Upstream,
                    };
                    if let Some(target) = editor.resolve_movement(&probe_pos, &movement) {
                        editor.transact(|tr| {
                            tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                            tr.set_selection(Some(target))?;
                            Ok(())
                        })?;
                    }
                }
                return Ok(());
            };

            // Backward/up is the reverse direction across every movement kind.
            let backward = matches!(
                movement,
                Movement::Grapheme {
                    direction: Direction::Backward
                } | Movement::Word {
                    direction: Direction::Backward
                } | Movement::Sentence {
                    direction: Direction::Backward
                } | Movement::Page {
                    direction: Direction::Backward
                } | Movement::Document {
                    direction: Direction::Backward
                } | Movement::Line {
                    direction: Direction::Backward,
                    ..
                }
            );

            // ArrowUp at the document's first navigable position (or the
            // leading gap cursor) hands focus off to the surrounding shell
            // UI (e.g. the subtitle field). Only the vertical single-step
            // upward movement triggers this — Page/Document backward are
            // scroll/jump commands and should not exit.
            let is_upward_exit = !extend
                && matches!(
                    movement,
                    Movement::Line {
                        direction: Direction::Backward,
                        axis: Axis::Vertical,
                    }
                );

            if is_upward_exit && selection.is_collapsed() {
                let at_document_start = match selection
                    .resolve(&editor.state.doc)
                    .and_then(|rs| rs.as_gap_cursor())
                {
                    Some(GapCursor::LeadingUnit { .. }) => true,
                    Some(_) => false,
                    None => {
                        gap_cursor_selection_leading(&editor.state.doc).is_none()
                            && editor
                                .resolve_movement(
                                    &selection.head,
                                    &Movement::Document {
                                        direction: Direction::Backward,
                                    },
                                )
                                .is_some_and(|sel| sel.head == selection.head)
                    }
                };
                if at_document_start {
                    editor.transact(|tr| {
                        tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                        tr.set_selection(None)?;
                        Ok(())
                    })?;
                    editor.push_event(EditorEvent::CursorExitedDocumentStart);
                    return Ok(());
                }
            }

            // Gap logic only for non-extend moves. Evaluated here and applied
            // with an early return so it does not depend on resolve_movement
            // returning Some (e.g. Up at the document top returns None but
            // must still exit/enter the gap).
            if !extend {
                // Exit: the current selection is itself a gap cursor.
                if let Some(gap) = selection
                    .resolve(&editor.state.doc)
                    .and_then(|rs| rs.as_gap_cursor())
                {
                    let exit: Option<Selection> = match gap {
                        GapCursor::LeadingUnit { unit } => {
                            if backward {
                                // Document start — nothing before it.
                                None
                            } else {
                                Some(exit_into_or_node_select(
                                    editor,
                                    unit.id(),
                                    NodeId::ROOT,
                                    0,
                                    false,
                                    &movement,
                                ))
                            }
                        }
                        GapCursor::BetweenMonolithic {
                            parent,
                            before,
                            after,
                            index,
                        } => {
                            let p = parent.id();
                            if backward {
                                Some(exit_into_or_node_select(
                                    editor,
                                    before.id(),
                                    p,
                                    index - 1,
                                    true,
                                    &movement,
                                ))
                            } else {
                                Some(exit_into_or_node_select(
                                    editor,
                                    after.id(),
                                    p,
                                    index,
                                    false,
                                    &movement,
                                ))
                            }
                        }
                    };
                    if let Some(sel) = exit {
                        editor.transact(|tr| {
                            tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                            tr.set_selection(Some(sel))?;
                            Ok(())
                        })?;
                    }
                    return Ok(());
                }

                // Entry: a unit node-selection moving (non-extend) toward an
                // adjacent gap becomes that gap cursor. The builders return
                // Some only for a valid leading-unit / between-monolithic gap
                // (they encapsulate the both-monolithic + paragraph-admittable
                // checks), so a non-gap move falls through to the range
                // endpoint policy below. Independent of resolve_movement's
                // result.
                if selection.is_unit_node_selection(&editor.state.doc) {
                    let parent = selection.anchor.node_id;
                    let idx = selection.anchor.offset.min(selection.head.offset);
                    let doc = &editor.state.doc;
                    let entry = if backward && parent == NodeId::ROOT && idx == 0 {
                        gap_cursor_selection_leading(doc)
                    } else if backward {
                        gap_cursor_selection_between(doc, parent, idx)
                    } else {
                        gap_cursor_selection_between(doc, parent, idx + 1)
                    };
                    if let Some(sel) = entry {
                        editor.transact(|tr| {
                            tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                            tr.set_selection(Some(sel))?;
                            Ok(())
                        })?;
                        return Ok(());
                    }
                }

                let can_stop_at_gap = matches!(
                    movement,
                    Movement::Grapheme { .. }
                        | Movement::Word { .. }
                        | Movement::Sentence { .. }
                        | Movement::Line { .. }
                );

                // Non-extended range navigation uses a sorted endpoint as the
                // movement base: backward starts from `from`, forward starts
                // from `to`.
                if !selection.is_collapsed() {
                    let Some(resolved) = selection.resolve(&editor.state.doc) else {
                        return Ok(());
                    };
                    let base = if backward {
                        resolved.from()
                    } else {
                        resolved.to()
                    };
                    let left_or_right = matches!(movement, Movement::Grapheme { .. });
                    let base_position = Position::from(base);
                    let base_is_inline = base.is_inline_position();
                    // Drop the doc borrow before calling the mutable wrapper.
                    drop(resolved);

                    let target = if left_or_right && base_is_inline {
                        // Left/Right from inline content only collapses the range.
                        Selection::collapsed(base_position)
                    } else if !left_or_right
                        && can_stop_at_gap
                        && let Some(sel) =
                            gap_cursor_from_inner_edge(editor, base_position, backward, &movement)
                    {
                        // Vertical/word/block movement can stop at an adjacent
                        // gap before asking the view for geometric movement.
                        if matches!(movement, Movement::Line { .. }) {
                            editor.ensure_preferred_x_at(&base_position);
                        }
                        sel
                    } else {
                        let view_target = editor.resolve_movement(&base_position, &movement);

                        view_target.unwrap_or_else(|| Selection::collapsed(base_position))
                    };

                    editor.transact(|tr| {
                        tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                        tr.set_selection(Some(target))?;
                        Ok(())
                    })?;
                    return Ok(());
                }

                // Inner-edge entry: when the caret borders the gap, the
                // keystroke crosses into it. Horizontal/word/sentence/block
                // moves use the exact start/end position (so a left-arrow
                // from mid-text walks within the text first); vertical
                // (Line) moves use the cursor's containing Line (so an
                // arrow-up/down from any column on the first/last line of
                // the monolithic — including a non-edge offset of a
                // multi-position line — leaves it into the gap). Only
                // movements that can stop at a gap
                // (Grapheme/Word/Sentence/Block/Line) trigger this; Page
                // and Document are excluded because paged/absolute jumps
                // must not be intercepted by an intermediate gap.
                // Ancestors are innermost-first, so one keypress crosses
                // exactly one boundary.
                if can_stop_at_gap
                    && let Some(sel) =
                        gap_cursor_from_inner_edge(editor, selection.head, backward, &movement)
                {
                    if matches!(movement, Movement::Line { .. }) {
                        editor.ensure_preferred_x_at(&selection.head);
                    }
                    editor.transact(|tr| {
                        tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                        tr.set_selection(Some(sel))?;
                        Ok(())
                    })?;
                    return Ok(());
                }
            }

            if extend {
                let is_cell_movement = matches!(
                    movement,
                    Movement::Grapheme { .. }
                        | Movement::Line {
                            axis: Axis::Vertical,
                            ..
                        }
                );
                if is_cell_movement {
                    let doc = &editor.state.doc;
                    if let Some(rect) = selection.resolve(doc).and_then(|rs| rs.as_cell_rect()) {
                        let anchor_cell_id = rect.anchor_cell.id();
                        let head_cell_id = rect.head_cell.id();
                        let is_vertical = matches!(movement, Movement::Line { .. });
                        let new_head_cell_id = step_cell(doc, head_cell_id, backward, is_vertical)
                            .unwrap_or(head_cell_id);
                        if let Some(new_sel) =
                            cell_rect_selection(doc, anchor_cell_id, new_head_cell_id)
                        {
                            editor.transact(|tr| {
                                tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                                tr.set_selection(Some(new_sel))?;
                                Ok(())
                            })?;
                            return Ok(());
                        }
                    }
                }
            }

            let new_selection = if extend {
                editor.resolve_extend_movement(selection, &movement)
            } else {
                editor.resolve_movement(&selection.head, &movement)
            };

            if let Some(new_selection) = new_selection {
                if extend {
                    let is_cell_movement = matches!(
                        movement,
                        Movement::Grapheme { .. }
                            | Movement::Line {
                                axis: Axis::Vertical,
                                ..
                            }
                    );
                    if is_cell_movement {
                        let doc = &editor.state.doc;
                        if let Some(current_cell) =
                            enclosing_table_cell(doc, selection.head.node_id)
                        {
                            let stays_in_cell =
                                enclosing_table_cell(doc, new_selection.head.node_id)
                                    == Some(current_cell);
                            if !stays_in_cell {
                                if let Some(cell_sel) =
                                    cell_rect_selection(doc, current_cell, current_cell)
                                {
                                    editor.transact(|tr| {
                                        tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                                        tr.set_selection(Some(cell_sel))?;
                                        Ok(())
                                    })?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }

                editor.transact(|tr| {
                    tr.update_meta(|meta| meta.history = HistoryMeta::Skip);
                    tr.set_selection(Some(new_selection))?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

fn gap_cursor_from_inner_edge(
    editor: &Editor,
    head: Position,
    backward: bool,
    movement: &Movement,
) -> Option<Selection> {
    let doc = &editor.state.doc;
    let start = doc.node(head.node_id)?;
    let vertical = matches!(movement, Movement::Line { .. });
    for monolithic in start.ancestors() {
        if !Schema::node_spec(monolithic.as_type()).monolithic {
            continue;
        }
        let (Some(parent), Some(index)) = (monolithic.parent(), monolithic.index()) else {
            continue;
        };
        let at_edge = if vertical {
            editor
                .view
                .is_at_edge_line_of(monolithic.id(), &head, !backward)
        } else {
            let Some(edge) = editor
                .view
                .editable_position_inside(monolithic.id(), !backward)
            else {
                continue;
            };
            edge.node_id == head.node_id && edge.offset == head.offset
        };
        if !at_edge {
            continue;
        }
        let parent_id = parent.id();
        return if backward {
            if parent_id == NodeId::ROOT && index == 0 {
                gap_cursor_selection_leading(doc)
            } else {
                gap_cursor_selection_between(doc, parent_id, index)
            }
        } else {
            gap_cursor_selection_between(doc, parent_id, index + 1)
        };
    }
    None
}

/// Exit a gap into `node_id`'s inner content; if it has none (e.g. an
/// atom / horizontal-rule leaf), node-select that node at its
/// `(parent, idx)` bracket instead — the only representable state for a
/// unit with no inner navigable. `at_end` encodes direction: `true` =
/// backward exit (enter the node from its end), `false` = forward exit.
/// The node-select fallback must carry that direction (forward =
/// front→back, backward = back→front), because set_selection's
/// normalization preserves anchor/head direction and an inverted bracket
/// would carry the wrong shift-extend anchor intent.
///
/// For vertical (Line) exit the landing column is the recorded
/// preferred_x (seeded at gap entry, preserved across the gap by
/// `reconcile_with_ops`). When the edge navigable is an Atom — no
/// column concept — the call returns `None` and the default
/// first/last-position landing applies.
fn exit_into_or_node_select(
    editor: &Editor,
    node_id: NodeId,
    parent: NodeId,
    idx: usize,
    at_end: bool,
    movement: &Movement,
) -> Selection {
    if matches!(movement, Movement::Line { .. })
        && let Some(pos) = editor.view.position_at_preferred_x_in(node_id, at_end)
    {
        return Selection::collapsed(pos);
    }
    if let Some(pos) = editor.view.editable_position_inside(node_id, at_end) {
        return Selection::collapsed(pos);
    }
    let front = Position {
        node_id: parent,
        offset: idx,
        affinity: Affinity::Downstream,
    };
    let back = Position {
        node_id: parent,
        offset: idx + 1,
        affinity: Affinity::Upstream,
    };
    if at_end {
        Selection::new(back, front)
    } else {
        Selection::new(front, back)
    }
}

fn step_cell(doc: &Doc, cell_id: NodeId, backward: bool, vertical: bool) -> Option<NodeId> {
    let cell = doc.node(cell_id)?;
    let row = cell.parent()?;
    let col = cell.index()?;
    if vertical {
        let table = row.parent()?;
        let row_idx = row.index()?;
        let new_row_idx = if backward {
            row_idx.checked_sub(1)?
        } else {
            row_idx + 1
        };
        let new_row = table.children().nth(new_row_idx)?;
        Some(new_row.children().nth(col)?.id())
    } else {
        let new_col = if backward {
            col.checked_sub(1)?
        } else {
            col + 1
        };
        Some(row.children().nth(new_col)?.id())
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{
        Affinity, Position, Selection, assert_state_eq, gap_cursor_selection_between,
        gap_cursor_selection_leading,
    };

    use crate::editor::Editor;
    use crate::message::*;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_move_forward_in_middle() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Navigation {
                op: NavigationOp::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                    extend: false,
                },
            },
        );
    }

    #[test]
    fn probe_move_forward_at_end() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_probe_predicts_apply(
            state,
            Message::Navigation {
                op: NavigationOp::Move {
                    movement: Movement::Grapheme {
                        direction: Direction::Forward,
                    },
                    extend: false,
                },
            },
        );
    }

    fn shift_up(editor: &mut Editor) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Line {
                    direction: Direction::Backward,
                    axis: Axis::Vertical,
                },
                extend: true,
            },
        });
    }

    fn shift_down(editor: &mut Editor) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical,
                },
                extend: true,
            },
        });
    }

    fn shift_left(editor: &mut Editor) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
                extend: true,
            },
        });
    }

    fn shift_right(editor: &mut Editor) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
                extend: true,
            },
        });
    }

    fn shift_word_right(editor: &mut Editor) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Word {
                    direction: Direction::Forward,
                },
                extend: true,
            },
        });
    }

    #[test]
    fn shift_right_from_range_ending_at_empty_paragraph_start_stops_at_empty_paragraph_break() {
        let (state, r, t1, _p1) = state! {
            doc {
                r: root [block_gap(200)] {
                    paragraph {
                        t1: text("bb")
                    }
                    p1: paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (t1, 1, >) -> (p1, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: t1,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_right_from_range_ending_after_trailing_hard_break_extends_to_paragraph_break() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        hard_break
                    }
                    p2: paragraph {}
                }
            }
            selection: (p1, 0, >) -> (p1, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: p2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_word_right_from_range_ending_at_empty_paragraph_start_stops_at_empty_paragraph_break()
    {
        let (state, r, t1, _p1) = state! {
            doc {
                r: root [block_gap(200)] {
                    paragraph {
                        t1: text("bb")
                    }
                    p1: paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (t1, 1, >) -> (p1, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_word_right(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: t1,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_left_from_range_ending_at_empty_paragraph_start_stops_at_previous_paragraph_break() {
        let (state, t1, _p1) = state! {
            doc {
                root [block_gap(200)] {
                    paragraph {
                        t1: text("bb")
                    }
                    p1: paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (t1, 0, >) -> (p1, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_left(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: t1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: t1,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_left_from_empty_paragraph_break_plus_atom_returns_to_paragraph_break() {
        let (state, r, p1) = state! {
            doc {
                r: root [block_gap(200)] {
                    p1: paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);
        shift_right(&mut editor);
        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );

        shift_left(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );

        shift_left(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::collapsed(Position::new(p1, 0)))
        );
    }

    #[test]
    fn shift_right_from_empty_paragraph_break_enters_following_callout_content() {
        let (state, _r, p1, p2) = state! {
            doc {
                r: root [block_gap(200)] {
                    p1: paragraph {}
                    callout {
                        p2: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);
        shift_right(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: p2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_left_from_empty_paragraph_break_plus_callout_returns_to_paragraph_break() {
        let (state, r, p1, p2) = state! {
            doc {
                r: root [block_gap(200)] {
                    p1: paragraph {}
                    callout {
                        p2: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);
        shift_right(&mut editor);
        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: p2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );

        shift_left(&mut editor);

        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: r,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn shift_left_from_selected_lower_atom_includes_both() {
        let (state, r) = state! {
            doc {
                r: root {
                    image
                    image
                    paragraph { text("bottom") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_left(&mut editor);

        let s = editor.state().selection.expect("selection exists in test");
        let (lo, hi) = if s.anchor.offset <= s.head.offset {
            (s.anchor.offset, s.head.offset)
        } else {
            (s.head.offset, s.anchor.offset)
        };
        assert_eq!(s.anchor.node_id, r);
        assert_eq!(s.head.node_id, r);
        assert_eq!(
            (lo, hi),
            (0, 2),
            "shift+left from image#1 node-selection must envelop both images"
        );
    }

    #[test]
    fn shift_right_from_selected_upper_atom_includes_both() {
        let (state, r) = state! {
            doc {
                r: root {
                    paragraph { text("top") }
                    image
                    image
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_right(&mut editor);

        let s = editor.state().selection.expect("selection exists in test");
        let (lo, hi) = if s.anchor.offset <= s.head.offset {
            (s.anchor.offset, s.head.offset)
        } else {
            (s.head.offset, s.anchor.offset)
        };
        assert_eq!(s.anchor.node_id, r);
        assert_eq!(s.head.node_id, r);
        assert_eq!(
            (lo, hi),
            (1, 3),
            "shift+right from image#1 node-selection must envelop both images"
        );
    }

    #[test]
    fn shift_up_extends_over_consecutive_top_atoms_from_text_below() {
        let (state, ..) = state! {
            doc {
                root {
                    image
                    image
                    paragraph { t1: text("bottom") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_up(&mut editor);
        shift_up(&mut editor);
        shift_up(&mut editor);

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });

        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("bottom") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_up_from_selected_lower_atom_includes_both() {
        let (state, ..) = state! {
            doc {
                r: root {
                    image
                    image
                    paragraph { text("bottom") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_up(&mut editor);
        shift_up(&mut editor);

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });

        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("bottom") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_down_over_consecutive_atoms_unchanged() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("top") }
                    image
                    image
                }
            }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_down(&mut editor);
        shift_down(&mut editor);
        shift_down(&mut editor);

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });

        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("top") }
                }
            }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    /// Strict monotonic head progression is precisely what the shared-boundary
    /// bug breaks, so it is the decisive signal that the fix works.
    #[test]
    fn shift_up_progresses_through_consecutive_atoms_in_mid_document() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { text("top") }
                    image
                    image
                    paragraph { t1: text("bottom") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_up(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_up(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_up(&mut editor);
        let h3 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(r(h1) < r(h0), "1st Shift+Up must move head up");
        assert!(
            r(h2) < r(h1),
            "2nd Shift+Up must move head up (not stuck at shared boundary)"
        );
        assert!(r(h3) < r(h2), "3rd Shift+Up must reach the paragraph above");
    }

    #[test]
    fn shift_right_through_consecutive_hard_breaks_visits_each_offset() {
        let (state, _p1, _t_a, _t_b) = state! {
            doc {
                root {
                    p1: paragraph { t_a: text("a") hard_break hard_break t_b: text("b") }
                }
            }
            selection: (t_a, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_right(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_right(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_right(&mut editor);
        let h3 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_right(&mut editor);
        let h4 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(r(h1) > r(h0), "1st Shift+Right selects 'a'");
        assert!(r(h2) > r(h1), "2nd Shift+Right passes first hard_break");
        assert!(
            r(h3) > r(h2),
            "3rd Shift+Right passes second hard_break (not stuck/skipped)"
        );
        assert!(r(h4) > r(h3), "4th Shift+Right enters text 'b'");
    }

    #[test]
    fn shift_left_through_consecutive_hard_breaks_visits_each_offset() {
        let (state, _p1, _t_a, _t_b) = state! {
            doc {
                root {
                    p1: paragraph { t_a: text("a") hard_break hard_break t_b: text("b") }
                }
            }
            selection: (t_b, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_left(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_left(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_left(&mut editor);
        let h3 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_left(&mut editor);
        let h4 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(r(h1) < r(h0), "1st Shift+Left selects 'b'");
        assert!(r(h2) < r(h1), "2nd Shift+Left passes second hard_break");
        assert!(r(h3) < r(h2), "3rd Shift+Left passes first hard_break");
        assert!(r(h4) < r(h3), "4th Shift+Left enters text 'a'");
    }

    #[test]
    fn shift_down_through_consecutive_hard_breaks_visits_each_line() {
        let (state, _p1, _t_a, _t_b) = state! {
            doc {
                root {
                    p1: paragraph { t_a: text("a") hard_break hard_break t_b: text("b") }
                }
            }
            selection: (t_a, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_down(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_down(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(
            r(h1) > r(h0),
            "1st Shift+Down lands on the empty hard_break line"
        );
        assert!(
            r(h2) > r(h1),
            "2nd Shift+Down reaches the 'b' line (not stuck)"
        );
    }

    #[test]
    fn shift_down_from_wrapped_line_start_keeps_extending_downward() {
        let (state, _t) = state! {
            doc {
                root (layout_mode: editor_model::LayoutMode::Continuous { max_width: 40 }) {
                    paragraph { t: text("aaaaaaaaaaaaaaaaaaaa") }
                }
            }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_down(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_down(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert_eq!(
            h1.affinity,
            editor_state::Affinity::Downstream,
            "1st Shift+Down must keep the wrapped-line head on the lower line"
        );
        assert!(
            r(h1) > r(h0),
            "1st Shift+Down must reach the wrapped second line"
        );
        assert!(
            r(h2) > r(h1),
            "2nd Shift+Down must keep extending to the next wrapped line"
        );
    }

    #[test]
    fn shift_up_through_consecutive_hard_breaks_visits_each_line() {
        let (state, _p1, _t_a, _t_b) = state! {
            doc {
                root {
                    p1: paragraph { t_a: text("a") hard_break hard_break t_b: text("b") }
                }
            }
            selection: (t_b, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let doc = editor.state.doc.clone();

        let h0 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_up(&mut editor);
        let h1 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;
        shift_up(&mut editor);
        let h2 = editor
            .state()
            .selection
            .expect("selection exists in test")
            .head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(
            r(h1) < r(h0),
            "1st Shift+Up lands on the empty hard_break line"
        );
        assert!(
            r(h2) < r(h1),
            "2nd Shift+Up reaches the 'a' line (not stuck)"
        );
    }

    #[test]
    fn replace_over_extended_consecutive_atoms_removes_both() {
        let (state, _, i1, i2) = state! {
            doc {
                r: root {
                    i1: image
                    i2: image
                    paragraph { text("bottom") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_up(&mut editor);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                text: "X".into(),
                html: None,
            },
        });

        assert!(
            editor.state().doc.node(i1).is_none(),
            "upper atom must be replaced away"
        );
        assert!(
            editor.state().doc.node(i2).is_none(),
            "lower atom must be replaced away"
        );
        assert!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .is_collapsed(),
            "replace must collapse the selection, not leave a stale node-selection"
        );
    }

    fn arrow(editor: &mut Editor, movement: Movement) {
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement,
                extend: false,
            },
        });
    }

    #[test]
    fn arrow_left_from_text_selection_collapses_to_sorted_start_without_extra_move() {
        let (state, t) = state! {
            doc {
                root {
                    paragraph { t: text("abcd") }
                }
            }
            selection: (t, 1) -> (t, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t);
        assert_eq!(s.head.offset, 1);
    }

    #[test]
    fn arrow_right_from_backward_text_selection_collapses_to_sorted_end_without_extra_move() {
        let (state, t) = state! {
            doc {
                root {
                    paragraph { t: text("abcd") }
                }
            }
            selection: (t, 3) -> (t, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t);
        assert_eq!(s.head.offset, 3);
    }

    #[test]
    fn word_left_from_text_selection_moves_from_sorted_start() {
        let (state, t) = state! {
            doc {
                root {
                    paragraph { t: text("hello world") }
                }
            }
            selection: (t, 6) -> (t, 11)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        arrow(
            &mut editor,
            Movement::Word {
                direction: Direction::Backward,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t);
        assert_eq!(s.head.offset, 0);
    }

    #[test]
    fn arrow_down_from_backward_text_selection_moves_from_sorted_end() {
        let (state, _, next) = state! {
            doc {
                root {
                    paragraph { text("abcdef") }
                    paragraph { current: text("abcdef") }
                    paragraph { next: text("abcdef") }
                }
            }
            selection: (current, 6) -> (current, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, next);
        assert_eq!(s.head.offset, 6);
    }

    #[test]
    fn arrow_left_from_block_boundary_selection_lands_at_previous_leaf_end() {
        let (state, _, prev) = state! {
            doc {
                r: root {
                    paragraph { prev: text("prev") }
                    image
                    paragraph { text("next") }
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, prev);
        assert_eq!(s.head.offset, 4);
    }

    #[test]
    fn arrow_right_from_fold_node_selection_moves_off_the_fold() {
        let (state, _, after) = state! {
            doc { r1: root {
                fold { fold_title { text("123123") } fold_content { paragraph { text("123") } } }
                paragraph { after: text("1231231232131") }
            } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        let before = editor.state().selection.expect("selection exists in test");
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let moved = editor.state().selection.expect("selection exists in test");

        assert_ne!(
            (before.anchor, before.head),
            (moved.anchor, moved.head),
            "arrow-right from a fold node-selection must change the selection (was a silent no-op)"
        );
        assert!(moved.is_collapsed());
        assert_eq!(moved.head.node_id, after);
        assert_eq!(moved.head.offset, 0);
    }

    #[test]
    fn arrow_down_from_fold_node_selection_is_not_a_noop() {
        let (state, ..) = state! {
            doc { r1: root {
                fold { fold_title { text("123123") } fold_content { paragraph { text("123") } } }
                paragraph { text("1231231232131") }
            } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection.expect("selection exists in test");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let after = editor.state().selection.expect("selection exists in test");
        assert_ne!(
            (before.anchor, before.head),
            (after.anchor, after.head),
            "arrow-down from a fold node-selection must not be a silent no-op"
        );
    }

    #[test]
    fn arrow_right_from_leading_gap_enters_fold_inner() {
        let (state, _, ft) = state! {
            doc {
                r: root {
                    fold { fold_title { ft: text("T") } fold_content { paragraph { text("c") } } }
                    paragraph { text("after") }
                }
            }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, ft);
        assert_eq!(s.head.offset, 0);
    }

    #[test]
    fn arrow_left_from_leading_gap_is_noop() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let before = editor.state().selection.expect("selection exists in test");
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let after = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            (before.anchor, before.head),
            (after.anchor, after.head),
            "leading gap + left = doc-start no-op"
        );
    }

    #[test]
    fn arrow_right_from_leading_gap_before_image_node_selects() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            !s.is_collapsed(),
            "image gap-exit must node-select the image"
        );
        assert!(s.is_unit_node_selection(&editor.state.doc));
    }

    /// Forward enters the *first* inner of the fold ahead (its title);
    /// backward enters the *last* inner of the fold behind. A fold is not
    /// internally symmetric, so "symmetric" here means the gesture enters
    /// the fold on the matching side (2nd fold forward, 1st fold backward),
    /// not that it lands on the same kind of inner node.
    #[test]
    fn arrow_into_two_folds_gap_exits_symmetric() {
        let make = || {
            let (state, _, a, b) = state! {
                doc {
                    r: root {
                        fold { fold_title { a: text("A") } fold_content { paragraph { text("x") } } }
                        fold { fold_title { b: text("B") } fold_content { paragraph { text("y") } } }
                        paragraph {}
                    }
                }
                selection: (r, 1)
            };
            let mut editor = Editor::new_test(state);
            editor.view.layout(&editor.state.doc);
            (editor, a, b)
        };

        let (mut e_fwd, _a, b) = make();
        arrow(
            &mut e_fwd,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        assert_eq!(
            e_fwd
                .state()
                .selection
                .expect("selection exists in test")
                .head
                .node_id,
            b,
            "right enters 2nd fold inner"
        );

        let (mut e_bwd, a, _b) = make();
        arrow(
            &mut e_bwd,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let doc = &e_bwd.state().doc;
        let head = e_bwd
            .state()
            .selection
            .expect("selection exists in test")
            .head
            .node_id;
        let first_fold = doc
            .node(a)
            .unwrap()
            .ancestors()
            .find(|n| matches!(n.node(), editor_model::Node::Fold(_)))
            .unwrap()
            .id();
        let inside_first_fold = doc
            .node(head)
            .unwrap()
            .ancestors()
            .any(|n| n.id() == first_fold);
        assert!(inside_first_fold, "left enters 1st fold inner (symmetric)");
    }

    #[test]
    fn arrow_left_from_leading_fold_node_selection_enters_gap() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("T") } fold_content { paragraph { text("c") } } }
                paragraph { text("after") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, editor_model::NodeId::ROOT);
        assert_eq!(s.head.offset, 0);
        assert_eq!(s.head.affinity, editor_state::Affinity::Upstream);
    }

    #[test]
    fn arrow_up_from_leading_fold_node_selection_also_enters_gap() {
        // Up at the document top makes resolve_movement return None; entry must
        // not depend on that.
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("T") } fold_content { paragraph { text("c") } } }
                paragraph { text("after") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.is_collapsed(),
            "Up from leading-unit node-sel must enter the gap (not no-op)"
        );
        assert_eq!(s.head.node_id, editor_model::NodeId::ROOT);
        assert_eq!(s.head.offset, 0);
    }

    #[test]
    fn arrow_up_from_leading_fold_title_range_enters_gap() {
        let (state, ..) = state! {
            doc { root {
                fold { fold_title { title: text("Title") } fold_content { paragraph { text("c") } } }
                paragraph { text("after") }
            } }
            selection: (title, 0) -> (title, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected =
            gap_cursor_selection_leading(&editor.state.doc).expect("leading fold gap is valid");

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_right_from_first_fold_node_selection_enters_between_gap() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, editor_model::NodeId::ROOT);
        assert_eq!(s.head.offset, 1);
    }

    #[test]
    fn arrow_right_from_leading_image_node_selection_no_gap() {
        // image (non-monolithic) <-> paragraph: between-gap builder returns
        // None -> existing behavior.
        let (state, _, t) = state! {
            doc { r: root { image paragraph { t: text("b") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t);
    }

    #[test]
    fn arrow_right_from_empty_paragraph_selection_before_image_selects_image() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    image
                    paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            s.anchor,
            Position {
                node_id: r,
                offset: 2,
                affinity: editor_state::Affinity::Downstream,
            },
        );
        assert_eq!(
            s.head,
            Position {
                node_id: r,
                offset: 3,
                affinity: editor_state::Affinity::Upstream,
            },
        );
    }

    #[test]
    fn arrow_down_from_empty_paragraph_selection_before_image_selects_image() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    image
                    paragraph {}
                    image
                    paragraph {}
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );

        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            s.anchor,
            Position {
                node_id: r,
                offset: 2,
                affinity: editor_state::Affinity::Downstream,
            },
        );
        assert_eq!(
            s.head,
            Position {
                node_id: r,
                offset: 3,
                affinity: editor_state::Affinity::Upstream,
            },
        );
    }

    #[test]
    fn arrow_left_from_second_fold_node_selection_enters_between_gap() {
        // Backward from a non-leading selected unit hits the
        // else-if-backward between-gap branch (mirror of the forward
        // first-fold test). Two monolithic folds + trailing paragraph,
        // so the between gap at index 1 is paragraph-admittable.
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, editor_model::NodeId::ROOT);
        assert_eq!(s.head.offset, 1);
    }

    #[test]
    fn arrow_up_from_inside_second_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph {} }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_left_from_inside_second_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph {} }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_down_from_inside_first_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { p1: paragraph {} }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_down_through_between_gap_preserves_column() {
        let (state, _, t2) = state! {
            doc {
                root {
                    callout { paragraph { t1: text("cdcdcdcdcd") } }
                    callout { paragraph { t2: text("efefefefef") } }
                    paragraph {}
                }
            }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_up(&mut editor);

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        assert!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_some(),
        );

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t2);
        assert_eq!(s.head.offset, 3);
    }

    #[test]
    fn arrow_up_through_between_gap_preserves_column() {
        let (state, t1, _) = state! {
            doc {
                root {
                    callout { paragraph { t1: text("cdcdcdcdcd") } }
                    callout { paragraph { t2: text("efefefefef") } }
                    paragraph {}
                }
            }
            selection: (t2, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        shift_down(&mut editor);

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        assert!(
            editor
                .state()
                .selection
                .expect("selection exists in test")
                .resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_some(),
        );

        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t1);
        assert_eq!(s.head.offset, 3);
    }

    #[test]
    fn arrow_down_from_text_start_in_first_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph { t1: text("1234") } }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_up_from_text_end_in_leading_callout_enters_leading_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { paragraph { t1: text("1234") } }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected =
            gap_cursor_selection_leading(&editor.state.doc).expect("leading-callout gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_up_from_text_mid_in_second_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph {} }
                    callout { paragraph { t2: text("abcd") } }
                    paragraph {}
                }
            }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_down_from_text_in_second_callout_with_paragraph_neighbor_no_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { paragraph {} }
                    callout { paragraph { t2: text("abcd") } }
                    paragraph {}
                }
            }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "no between-gap when the next sibling is a paragraph; normal exit"
        );
    }

    #[test]
    fn arrow_left_from_text_mid_in_callout_does_not_enter_gap() {
        let (state, t2) = state! {
            doc {
                root {
                    callout { paragraph {} }
                    callout { paragraph { t2: text("abcd") } }
                    paragraph {}
                }
            }
            selection: (t2, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "grapheme-left from mid-text must not jump straight into the gap"
        );
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t2);
        assert_eq!(s.head.offset, 1);
    }

    #[test]
    fn arrow_right_from_inside_first_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { p1: paragraph {} }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn word_backward_from_inside_second_callout_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph {} }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Word {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn arrow_up_from_inside_leading_callout_enters_leading_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected =
            gap_cursor_selection_leading(&editor.state.doc).expect("leading-callout gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    // The two callouts are nested inside a fold's `fold_content` — a
    // container whose content model admits a Paragraph between two
    // monolithic siblings, so the inner boundary is a valid gap. The
    // innermost monolithic ancestor (callout#2) must win, so the caret
    // stops at the inner gap, not the fold's outer boundary.
    #[test]
    fn arrow_up_nested_callouts_stops_at_innermost_boundary_first() {
        let (state, fc, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("t") }
                        fc: fold_content {
                            callout { paragraph {} }
                            callout { p1: paragraph {} }
                            paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, fc, 1)
            .expect("inner between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
    }

    #[test]
    fn document_backward_from_inside_callout_ignores_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { paragraph {} }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Document {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "Document-backward must not stop at an intermediate gap"
        );
    }

    #[test]
    fn page_backward_from_inside_callout_ignores_gap() {
        // Caret in the FIRST callout. If Page were wrongly admitted to the
        // stepping allowlist, the valid leading gap here would catch it.
        // A caret in the second callout would make the adjacent forward gap
        // structurally invalid, so the guard would pass vacuously.
        let (state, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph {} }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Page {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "Page is outside the stepping allowlist; Page-backward must not enter a gap"
        );
    }

    #[test]
    fn arrow_up_from_inside_callout_with_paragraph_neighbor_no_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "no between-gap when the previous sibling is a paragraph; normal exit"
        );
    }

    #[test]
    fn shift_up_from_inside_second_callout_does_not_enter_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { paragraph {} }
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Line {
                    direction: Direction::Backward,
                    axis: Axis::Vertical,
                },
                extend: true,
            },
        });
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "extend (Shift) must not enter the gap"
        );
    }

    #[test]
    fn caret_not_at_inner_edge_does_not_enter_gap() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { paragraph {} }
                    callout { paragraph { t: text("xy") } }
                    paragraph {}
                }
            }
            selection: (t, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "a caret not at the monolithic's inner edge must not enter the gap"
        );
    }

    #[test]
    fn round_trip_gap_into_callout_and_back() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    callout { paragraph {} }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let gap = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("between-callouts gap is valid");
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        assert_eq!(
            {
                let s = editor.state().selection.expect("selection exists in test");
                (s.anchor, s.head)
            },
            (gap.anchor, gap.head),
            "node-selection of callout#1 + forward enters the gap"
        );
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let inside = editor.state().selection.expect("selection exists in test");
        assert!(inside.is_collapsed() && inside.head.node_id != r);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let back = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            (back.anchor, back.head),
            (gap.anchor, gap.head),
            "backward from callout#2 inner-first returns to the same gap (round-trip)"
        );
    }

    // TR-199: 테이블 뒤에 폴드가 오면 그 경계는 두 monolithic 사이의 유효한
    // 갭이다. 마지막 행에서 아래로 이동하면 마지막 열뿐 아니라 어느 열에서든
    // 이 갭으로 진입해야 한다. 버그는 테이블의 유일한 "마지막 navigable 라인"이
    // 마지막 셀에 있어 마지막 열에서만 테이블을 벗어나게 했다.
    #[test]
    fn arrow_down_from_non_last_column_of_table_last_row_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                            table_cell { paragraph { text("c") } }
                        }
                        table_row {
                            table_cell { paragraph { t: text("d") } }
                            table_cell { paragraph { text("e") } }
                            table_cell { paragraph { text("f") } }
                        }
                    }
                    fold { fold_title { text("T") } fold_content { paragraph { text("x") } } }
                    paragraph {}
                }
            }
            selection: (t, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("table↔fold between gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            (s.anchor, s.head),
            (expected.anchor, expected.head),
            "down from the first column of the last row must enter the table↔fold gap"
        );
    }

    // 회귀 방지: 마지막 열은 원래 동작했으므로 수정 후에도 유지되어야 한다.
    #[test]
    fn arrow_down_from_last_column_of_table_last_row_enters_between_gap() {
        let (state, r, ..) = state! {
            doc {
                r: root {
                    table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                            table_cell { paragraph { text("c") } }
                        }
                        table_row {
                            table_cell { paragraph { text("d") } }
                            table_cell { paragraph { text("e") } }
                            table_cell { paragraph { t: text("f") } }
                        }
                    }
                    fold { fold_title { text("T") } fold_content { paragraph { text("x") } } }
                    paragraph {}
                }
            }
            selection: (t, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        let expected = gap_cursor_selection_between(&editor.state.doc, r, 1)
            .expect("table↔fold between gap is valid");
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert_eq!(
            (s.anchor, s.head),
            (expected.anchor, expected.head),
            "down from the last column of the last row must enter the table↔fold gap"
        );
    }

    // 테이블의 이웃이 일반 문단(monolithic 아님)이면 그 경계는 갭이 아니다.
    // 이때 마지막 행에서 아래로 이동하면 갭에 멈추지 않고 일반 기하학적 이동으로
    // 넘어가, 테이블 아래 문단에 도착해야 한다. 버그가 가두던 비-마지막 열에서도
    // 마찬가지여야 한다.
    #[test]
    fn arrow_down_from_non_last_column_of_table_last_row_with_paragraph_neighbor_moves_out() {
        let (state, _t, below) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                            table_cell { paragraph { text("c") } }
                        }
                        table_row {
                            table_cell { paragraph { t: text("d") } }
                            table_cell { paragraph { text("e") } }
                            table_cell { paragraph { text("f") } }
                        }
                    }
                    paragraph { below: text("below") }
                }
            }
            selection: (t, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "paragraph neighbor is not a between-gap; must not enter a gap cursor"
        );
        assert!(s.is_collapsed());
        assert_eq!(
            s.head.node_id, below,
            "down must move into the paragraph below the table, not stay in the cell"
        );
    }

    // TR-199 회귀 방지: 가장자리 행 판정이 "마지막 직계 자식 서브트리"로 넓어지면서,
    // 자식이 단락 하나뿐이고 그 단락이 여러 줄로 접히는 monolithic(콜아웃 등)에서
    // 첫 줄에 있어도 가장자리로 오인될 수 있다. 첫 줄에서 down은 갭으로 나가지 말고
    // 같은 단락의 둘째 줄로 가야 한다.
    #[test]
    fn arrow_down_within_multiline_paragraph_in_callout_does_not_exit() {
        let (state, t_a, t_b) = state! {
            doc {
                root {
                    callout { paragraph { t_a: text("a") hard_break t_b: text("b") } }
                    callout { paragraph {} }
                    paragraph {}
                }
            }
            selection: (t_a, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        let s = editor.state().selection.expect("selection exists in test");
        assert!(
            s.resolve(&editor.state().doc)
                .and_then(|rs| rs.as_gap_cursor())
                .is_none(),
            "down from the first wrapped line must not exit the callout into the gap"
        );
        assert!(s.is_collapsed());
        assert_eq!(
            s.head.node_id, t_b,
            "down must move to the second line within the same paragraph"
        );
    }
}

use editor_model::{NodeId, Schema};
use editor_state::{
    Affinity, GapCursor, Position, Selection, farther_endpoint, gap_cursor_selection_between,
    gap_cursor_selection_leading,
};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_navigation_op(editor: &mut Editor, op: NavigationOp) -> Result<(), EditorError> {
    match op {
        NavigationOp::Move { movement, extend } => {
            let selection = editor.state.selection;

            // Backward/up is the reverse direction across every movement kind.
            let backward = matches!(
                movement,
                Movement::Grapheme {
                    direction: Direction::Backward
                } | Movement::Word {
                    direction: Direction::Backward
                } | Movement::Sentence {
                    direction: Direction::Backward
                } | Movement::Block {
                    direction: Direction::Backward
                } | Movement::Document {
                    direction: Direction::Backward
                } | Movement::Line {
                    direction: Direction::Backward,
                    ..
                }
            );

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
                                ))
                            } else {
                                Some(exit_into_or_node_select(
                                    editor,
                                    after.id(),
                                    p,
                                    index,
                                    false,
                                ))
                            }
                        }
                    };
                    if let Some(sel) = exit {
                        editor.transact(|tr| {
                            tr.set_selection(sel)?;
                            Ok(())
                        })?;
                    }
                    return Ok(());
                }

                // Entry: a unit node-selection moving (non-extend) toward an
                // adjacent gap becomes that gap cursor. The builders return
                // Some only for a valid leading-unit / between-monolithic gap
                // (they encapsulate the both-monolithic + paragraph-admittable
                // checks), so a non-gap move falls through to normal movement.
                // Independent of resolve_movement's result.
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
                            tr.set_selection(sel)?;
                            Ok(())
                        })?;
                        return Ok(());
                    }
                }

                // Inner-edge entry: a collapsed caret at the first
                // (backward) / last (forward) inner navigable of a
                // monolithic that borders a valid gap becomes that gap
                // cursor — the exact inverse of gap exit, so the gap is a
                // stable stop in both directions. Only stepping movements
                // (Grapheme/Word/Sentence/Block/Line) trigger this; Page
                // and Document are excluded — a paged/absolute jump must
                // not be intercepted by an intermediate gap. (A positive
                // allowlist is required: the upstream `backward` flag does
                // not classify Page, so a negative `!Document` gate would
                // misclassify Page's direction.) Ancestors are
                // innermost-first, so one keypress crosses exactly one
                // boundary. The owned target is resolved while the
                // doc/view borrows are live, then applied, so the mutable
                // transact does not overlap them.
                let stepping = matches!(
                    movement,
                    Movement::Grapheme { .. }
                        | Movement::Word { .. }
                        | Movement::Sentence { .. }
                        | Movement::Block { .. }
                        | Movement::Line { .. }
                );
                if selection.is_collapsed() && stepping {
                    let head = selection.head;
                    let target: Option<Selection> = {
                        let doc = &editor.state.doc;
                        let mut found = None;
                        if let Some(start) = doc.node(head.node_id) {
                            for m in start.ancestors() {
                                if !Schema::node_spec(m.as_type()).monolithic {
                                    continue;
                                }
                                let (Some(p), Some(i)) = (m.parent(), m.index()) else {
                                    continue;
                                };
                                let pid = p.id();
                                let Some(edge) =
                                    editor.view.editable_position_inside(m.id(), !backward)
                                else {
                                    continue;
                                };
                                if edge.node_id != head.node_id || edge.offset != head.offset {
                                    continue;
                                }
                                let cand = if backward {
                                    if pid == NodeId::ROOT && i == 0 {
                                        gap_cursor_selection_leading(doc)
                                    } else {
                                        gap_cursor_selection_between(doc, pid, i)
                                    }
                                } else {
                                    gap_cursor_selection_between(doc, pid, i + 1)
                                };
                                if let Some(sel) = cand {
                                    found = Some(sel);
                                    break;
                                }
                            }
                        }
                        found
                    };
                    if let Some(sel) = target {
                        editor.transact(|tr| {
                            tr.set_selection(sel)?;
                            Ok(())
                        })?;
                        return Ok(());
                    }
                }
            }

            let resource_guard = editor.resource.lock().unwrap();
            let new_selection =
                editor
                    .view
                    .resolve_movement(&selection.head, &movement, &resource_guard);
            drop(resource_guard);
            if let Some(new_selection) = new_selection {
                let final_selection = if extend {
                    let doc = &editor.state.doc;
                    // Re-anchoring is sound only when the current selection
                    // already brackets a unit (atom or monolithic block) and the
                    // move is a vertical line step; otherwise the gesture anchor
                    // must stay put so ordinary Shift selection (shrink/reverse,
                    // horizontal, word) is unaffected.
                    let vertical_line = matches!(
                        movement,
                        Movement::Line {
                            axis: Axis::Vertical,
                            ..
                        }
                    );
                    let fixed = if vertical_line && selection.is_unit_node_selection(doc) {
                        // Re-anchor to the unit edge opposite the travel
                        // direction so the already-selected unit stays in the
                        // range as it grows.
                        farther_endpoint(doc, new_selection.head, selection.anchor, selection.head)
                    } else {
                        selection.anchor
                    };
                    // The farther-from-fixed head rule generalizes to every
                    // extend: a no-op for collapsed targets, and for a unit
                    // target it pulls the whole unit in one step.
                    let head =
                        farther_endpoint(doc, fixed, new_selection.anchor, new_selection.head);
                    Selection::new(fixed, head)
                } else {
                    new_selection
                };

                editor.transact(|tr| {
                    tr.set_selection(final_selection)?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
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
fn exit_into_or_node_select(
    editor: &Editor,
    node_id: NodeId,
    parent: NodeId,
    idx: usize,
    at_end: bool,
) -> Selection {
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

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{
        Position, assert_state_eq, gap_cursor_selection_between, gap_cursor_selection_leading,
    };

    use crate::editor::Editor;
    use crate::message::*;

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

        let h0 = editor.state().selection.head;
        shift_up(&mut editor);
        let h1 = editor.state().selection.head;
        shift_up(&mut editor);
        let h2 = editor.state().selection.head;
        shift_up(&mut editor);
        let h3 = editor.state().selection.head;

        let r = |p: Position| p.resolve(&doc).expect("resolves");
        assert!(r(h1) < r(h0), "1st Shift+Up must move head up");
        assert!(
            r(h2) < r(h1),
            "2nd Shift+Up must move head up (not stuck at shared boundary)"
        );
        assert!(r(h3) < r(h2), "3rd Shift+Up must reach the paragraph above");
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
            editor.state().selection.is_collapsed(),
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

        let before = editor.state().selection;
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        );
        let moved = editor.state().selection;

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
        let before = editor.state().selection;
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        );
        assert_ne!(
            (before.anchor, before.head),
            (
                editor.state().selection.anchor,
                editor.state().selection.head
            ),
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
        let s = editor.state().selection;
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
        let before = editor.state().selection;
        arrow(
            &mut editor,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        );
        assert_eq!(
            (before.anchor, before.head),
            (
                editor.state().selection.anchor,
                editor.state().selection.head
            ),
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
        let s = editor.state().selection;
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
            e_fwd.state().selection.head.node_id,
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
        let head = e_bwd.state().selection.head.node_id;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
        assert!(
            s.is_collapsed(),
            "Up from leading-unit node-sel must enter the gap (not no-op)"
        );
        assert_eq!(s.head.node_id, editor_model::NodeId::ROOT);
        assert_eq!(s.head.offset, 0);
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
        assert!(s.is_collapsed());
        assert_eq!(s.head.node_id, t);
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
        assert_eq!((s.anchor, s.head), (expected.anchor, expected.head));
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        // stepping allowlist it would take the forward branch (the
        // `backward` flag does not classify Page), and the forward
        // between-gap here is valid (callout#1 | callout#2, with a
        // paragraph-admittable slot) — so a broken allowlist would enter a
        // gap and fail this guard. A caret in the second callout instead
        // makes that forward between-gap land on the trailing paragraph
        // (structurally invalid), so the guard would pass vacuously and
        // catch nothing — do not use that shape.
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
        let s = editor.state().selection;
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
                let s = editor.state().selection;
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
        let inside = editor.state().selection;
        assert!(inside.is_collapsed() && inside.head.node_id != r);
        arrow(
            &mut editor,
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        );
        let back = editor.state().selection;
        assert_eq!(
            (back.anchor, back.head),
            (gap.anchor, gap.head),
            "backward from callout#2 inner-first returns to the same gap (round-trip)"
        );
    }
}

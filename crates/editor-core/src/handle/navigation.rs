use std::cmp::Ordering;

use editor_model::Doc;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

/// Returns whichever of `e1`/`e2` lies farther from `reference` in document
/// order (`ResolvedPosition` Ord is `(path, affinity)`, `Upstream < Downstream`).
/// A collapsed target has `e1 == e2`, so the unchanged endpoint is returned and
/// ordinary non-atom navigation behaves exactly as before.
fn farther_endpoint(doc: &Doc, reference: Position, e1: Position, e2: Position) -> Position {
    if e1 == e2 {
        return e2;
    }
    let (Some(r), Some(r1), Some(r2)) = (reference.resolve(doc), e1.resolve(doc), e2.resolve(doc))
    else {
        return e2;
    };
    match (r1.cmp(&r), r2.cmp(&r)) {
        (Ordering::Less | Ordering::Equal, Ordering::Less | Ordering::Equal) => {
            if r1 <= r2 {
                e1
            } else {
                e2
            }
        }
        (Ordering::Greater | Ordering::Equal, Ordering::Greater | Ordering::Equal) => {
            if r1 >= r2 {
                e1
            } else {
                e2
            }
        }
        // Real callers only pass adjacent atom-bracket pairs or equal endpoints,
        // so a reference strictly between the two never occurs; this arm is a
        // defensive fallback, not a meaningful answer.
        _ => e2,
    }
}

pub fn handle_navigation_op(editor: &mut Editor, op: NavigationOp) -> Result<(), EditorError> {
    match op {
        NavigationOp::Move { movement, extend } => {
            let selection = editor.state.selection;
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
                    // already brackets an atom and the move is a vertical line
                    // step; otherwise the gesture anchor must stay put so
                    // ordinary Shift selection (shrink/reverse, horizontal,
                    // word) is unaffected.
                    let vertical_line = matches!(
                        movement,
                        Movement::Line {
                            axis: Axis::Vertical,
                            ..
                        }
                    );
                    let fixed = if vertical_line && selection.is_atom_node_selection(doc) {
                        // Re-anchor to the atom edge opposite the travel
                        // direction so the already-selected atom stays in the
                        // range as it grows.
                        farther_endpoint(doc, new_selection.head, selection.anchor, selection.head)
                    } else {
                        selection.anchor
                    };
                    // The farther-from-fixed head rule generalizes to every
                    // extend: a no-op for collapsed targets, and for an atom
                    // target it pulls the whole atom in one step.
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

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::Affinity;
    use editor_state::Position;
    use editor_state::assert_state_eq;

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

    #[test]
    fn farther_endpoint_picks_correct_edge() {
        let (state, ..) = state! {
            doc {
                root {
                    image
                    image
                    paragraph { t1: text("x") }
                }
            }
            selection: (t1, 0)
        };
        let doc = &state.doc;
        let text = state.selection.head.node_id;
        let root = state
            .doc
            .node(text)
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .id();
        // The second atom sits at root child index 1, so its node selection is
        // front=(root,1,Down), back=(root,2,Up).
        let front = Position {
            node_id: root,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let back = Position {
            node_id: root,
            offset: 2,
            affinity: Affinity::Upstream,
        };

        let ref_below = Position {
            node_id: text,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        assert_eq!(super::farther_endpoint(doc, ref_below, front, back), front);

        let ref_above = Position {
            node_id: root,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        assert_eq!(super::farther_endpoint(doc, ref_above, front, back), back);

        let collapsed = Position {
            node_id: text,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        assert_eq!(
            super::farther_endpoint(doc, ref_below, collapsed, collapsed),
            collapsed
        );
    }

    /// The fallback arm is unreachable for real navigation callers (adjacent
    /// atom-bracket endpoints leave no addressable position between them), so
    /// it is exercised here only as a pure-function contract check.
    #[test]
    fn farther_endpoint_reference_between_returns_e2() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("ab") }
                    paragraph { t2: text("cd") }
                }
            }
            selection: (t1, 0)
        };
        let doc = &state.doc;
        let e1 = Position::new(t1, 0);
        let e2 = Position::new(t2, 2);
        let between = Position::new(t1, 1);
        assert_eq!(super::farther_endpoint(doc, between, e1, e2), e2);
    }
}

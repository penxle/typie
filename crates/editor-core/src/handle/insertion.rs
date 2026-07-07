use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_insertion_op(editor: &mut Editor, op: InsertionOp) -> Result<(), EditorError> {
    // Auto surround: when the user types a bracket/quote over a non-collapsed selection
    // and IME is not active, wrap the selection instead of replacing it.
    if let InsertionOp::Text { text } = &op {
        let enabled = editor.resource.lock().unwrap().auto_surround_enabled;
        if enabled && editor.state.composition.is_none() {
            let text = text.clone();
            let mut surround_applied = false;
            editor.transact(|tr| {
                surround_applied = commands::auto_surround(tr, &text)?;
                Ok(())
            })?;
            if surround_applied {
                return Ok(());
            }
        }
    }

    editor.transact(|tr| {
        match &op {
            InsertionOp::Text { text } => {
                let coalesces = tr.composition().is_none()
                    && tr.selection().is_some_and(|s| s.anchor == s.head);
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::materialize_gap_paragraph(),
                        commands::insert_paragraph_after_unit_selection(),
                        |_tr| Ok(true),
                    ),
                    commands::replace_selection_with_text(text, None),
                )?;
                if coalesces && tr.doc_changed() {
                    tr.update_meta(|m| m.merge = editor_transaction::MergeKind::Typing);
                }
            }
            InsertionOp::Break {
                kind: Break::Paragraph,
            } => {
                let applied = commands::first!(
                    tr,
                    commands::materialize_gap_paragraph(),
                    commands::insert_paragraph_after_unit_selection(),
                    |tr| commands::chain!(
                        tr,
                        commands::optional!(commands::ensure_paragraph()),
                        commands::optional!(commands::delete_selection()),
                        |tr| commands::first!(
                            tr,
                            commands::lift_paragraph_forward(),
                            commands::split_paragraph(),
                        ),
                    ),
                )?;
                if applied {
                    tr.clear_pending_format()?;
                }
            }
            InsertionOp::Break { kind: Break::Line } => {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::materialize_gap_paragraph(),
                        commands::insert_paragraph_after_unit_selection(),
                        |tr| commands::chain!(
                            tr,
                            commands::optional!(commands::ensure_paragraph()),
                            commands::optional!(commands::delete_selection()),
                        ),
                    ),
                    commands::insert_hard_break(),
                )?;
            }
            InsertionOp::Break { kind: Break::Page } => {
                let applied = commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::materialize_gap_paragraph(),
                        commands::insert_paragraph_after_unit_selection(),
                        |tr| commands::chain!(
                            tr,
                            commands::optional!(commands::ensure_paragraph()),
                            commands::optional!(commands::delete_selection()),
                        ),
                    ),
                    commands::split_paragraph(),
                    commands::insert_page_break_into_prev_paragraph(),
                )?;
                if applied {
                    tr.clear_pending_format()?;
                }
            }
            InsertionOp::Fragment { fragment } => {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::materialize_gap_paragraph(),
                        commands::insert_paragraph_after_unit_selection(),
                        |tr| commands::chain!(
                            tr,
                            commands::optional!(commands::ensure_paragraph()),
                            commands::optional!(commands::delete_selection()),
                        ),
                    ),
                    commands::insert_fragment(fragment.clone()),
                )?;
            }
        }
        Ok(())
    })?;

    if matches!(op, InsertionOp::Text { .. }) {
        let resource = std::sync::Arc::clone(&editor.resource);
        let resource = resource.lock().unwrap();
        editor.transact(|tr| {
            commands::optional!(commands::try_text_replacement(&resource))(tr)?;
            Ok(())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    #[test]
    fn probe_insert_text_into_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        assert_probe_predicts_apply(
            state,
            Message::Insertion {
                op: InsertionOp::Text { text: "X".into() },
            },
        );
    }

    #[test]
    fn probe_insert_break_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        assert_probe_predicts_apply(
            state,
            Message::Insertion {
                op: InsertionOp::Break {
                    kind: Break::Paragraph,
                },
            },
        );
    }

    #[test]
    fn insert_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: " world".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 11)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_text_into_synthetic_trailing_paragraph_after_unit() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        // The doc's real content ends with a horizontal rule; the Root schema
        // derives a synthetic trailing paragraph. Placing the caret in it and
        // typing must materialize it into a real paragraph — previously this
        // panicked with OffsetOutOfBounds and no text was inserted at all.
        let (state, _root) = state! {
            doc { r: root { horizontal_rule } }
            selection: (r, 0)
        };
        let synth_p = {
            let view = state.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(synth_p.is_synthetic());

        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(synth_p, 0)),
            },
        });
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "hi".into() },
        });

        let (expected, ..) = state! {
            doc { root {
                horizontal_rule
                p1: paragraph { text("hi") }
            } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_block() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") } p1: paragraph { text("lo") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn range_select_enter_equals_range_delete_then_enter() {
        let (fused_state, ..) = state! {
            doc { root { p1: paragraph { text("abcde") [bold] } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let (split_state, ..) = state! {
            doc { root { p1: paragraph { text("abcde") [bold] } } }
            selection: (p1, 1) -> (p1, 4)
        };

        let mut fused = Editor::new_test(fused_state);
        fused.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });

        let mut split = Editor::new_test(split_state);
        split.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        split.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });

        editor_state::assert_doc_eq!(fused.state(), split.state());
    }

    #[test]
    fn range_select_enter_equals_range_delete_then_enter_across_style_boundary() {
        let (fused_state, ..) = state! {
            doc { root { p1: paragraph { text("ab") [bold] text("cde") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let (split_state, ..) = state! {
            doc { root { p1: paragraph { text("ab") [bold] text("cde") } } }
            selection: (p1, 1) -> (p1, 4)
        };

        let mut fused = Editor::new_test(fused_state);
        fused.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });

        let mut split = Editor::new_test(split_state);
        split.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        split.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });

        editor_state::assert_doc_eq!(fused.state(), split.state());
    }

    #[test]
    fn insert_break_line() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Line },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hel") hard_break {} text("lo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_page() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Page },
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("hel") page_break {} }
                p2: paragraph { text("lo") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_text_preserves_unit_selection_inserts_after() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "b".into() },
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph { text("b") }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_text_replaces_multi_leaf_selection() {
        let (state, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "b".into() },
        });
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("b") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_paragraph_on_node_selection() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        // A paragraph break after a selected unit is just the new paragraph
        // itself: the unit is preserved and one empty paragraph is inserted
        // after it with the cursor inside. No split runs on top of that.
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_line_on_node_selection() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Line },
        });
        // Unit preserved; an empty paragraph is inserted after it and the line
        // break lands inside that fresh paragraph.
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph { hard_break }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_page_on_node_selection() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Page },
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { page_break }
                p2: paragraph
                paragraph { text("c") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_fragment_preserves_unit_selection_inserts_after() {
        // Use hr fragment as a stable default-constructible block-level leaf.
        // The selected hr is preserved and the fragment is inserted after it.
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Fragment {
                fragment: editor_model::Fragment::leaf(editor_model::PlainNode::HorizontalRule(
                    editor_model::PlainHorizontalRuleNode::default(),
                )),
            },
        });
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 2, >) -> (r, 3, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn type_text_at_leading_gap_creates_paragraph_with_text() {
        // Leading-unit gap: collapsed Upstream caret before root's first
        // child (an image). Typing must materialize a real paragraph
        // there and land the text in it.
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "hi".into() },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hi") } image paragraph { text("b") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn type_text_at_between_folds_gap_creates_paragraph() {
        // Between-monolithic gap between two folds (the trailing paragraph
        // makes the slot paragraph-admittable). Typing materializes a
        // paragraph at that slot and lands the text in it.
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "z".into() },
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("z") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn fragment_at_leading_gap_places_block_no_leftover_paragraph() {
        // Inserting a block fragment into the materialized empty paragraph
        // replaces it (existing "block into empty paragraph" behavior), so
        // the gap yields the block at index 0 with no leftover paragraph.
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Fragment {
                fragment: editor_model::Fragment::leaf(editor_model::PlainNode::HorizontalRule(
                    editor_model::PlainHorizontalRuleNode::default(),
                )),
            },
        });
        let view = editor.state().view();
        let root = view.node(editor_crdt::Dot::ROOT).unwrap();
        let kinds: Vec<editor_model::NodeType> = root
            .children()
            .map(|c| match c {
                editor_model::ChildView::Block(b) => b.node_type(),
                editor_model::ChildView::Leaf(l) => l.node_type(),
            })
            .collect();
        assert_eq!(
            kinds.first(),
            Some(&editor_model::NodeType::HorizontalRule),
            "gap fragment must place the block at index 0 (no leftover empty paragraph)"
        );
        assert_eq!(kinds.get(1), Some(&editor_model::NodeType::Image));
        assert!(
            !root
                .child_blocks()
                .any(|b| b.node_type() == editor_model::NodeType::Paragraph
                    && b.children().next().is_none()),
            "no leftover empty paragraph from materialization"
        );
        // Caret position is insert_fragment-internal behavior already
        // covered by existing insert_fragment tests; only structure is
        // asserted here.
    }

    #[test]
    fn type_text_with_normal_caret_unaffected() {
        // Non-gap caret: materialize_gap_paragraph returns Ok(false) so
        // the existing first! fallback path is preserved exactly.
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: " w".into() },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello w") } } }
            selection: (p1, 7)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn auto_surround_wraps_selection_with_parens() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6) -> (p1, 11)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "(".into() },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello (world)") } } }
            selection: (p1, 6) -> (p1, 13)
        };
        assert_state_eq!(editor.state(), &expected);
        assert!(editor.undo_history.can_undo());
    }

    #[test]
    fn auto_surround_disabled_replaces_selection_normally() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6) -> (p1, 11)
        };
        let mut editor = Editor::new_test(state);
        editor
            .resource
            .lock()
            .unwrap()
            .set_auto_surround_enabled(false);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "(".into() },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello (") } } }
            selection: (p1, 7)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn auto_surround_collapsed_selection_inserts_normally() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "(".into() },
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello(") } } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_preserves_font_family_and_weight() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: "World".into(),
            },
        });

        let view = editor.state().view();
        let second = view
            .root()
            .unwrap()
            .child_blocks()
            .nth(1)
            .expect("second paragraph");
        let items = second.inline();
        let first = items.first().expect("text exists in second paragraph");
        assert!(first.effective.values().any(
            |m| matches!(m, editor_model::Modifier::FontFamily { value } if value == "KoPubBatang")
        ));
        assert!(
            first
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn shift_enter_preserves_font_family_and_weight() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Line },
        });
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: "World".into(),
            },
        });

        let view = editor.state().view();
        let paragraph = view.root().unwrap().child_blocks().next().unwrap();
        let items = paragraph.inline();
        let hard_break_index = items
            .iter()
            .position(|item| {
                matches!(
                    item.kind,
                    editor_model::InlineKind::Atom(editor_model::NodeType::HardBreak)
                )
            })
            .expect("hard break atom");
        let hard_break = &items[hard_break_index];
        assert!(hard_break.effective.values().any(
            |m| matches!(m, editor_model::Modifier::FontFamily { value } if value == "KoPubBatang")
        ));
        assert!(
            hard_break
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontWeight { value: 700 }))
        );
        let world_leaf = items
            .get(hard_break_index + 1)
            .expect("text after hard_break");
        assert!(world_leaf.effective.values().any(
            |m| matches!(m, editor_model::Modifier::FontFamily { value } if value == "KoPubBatang")
        ));
        assert!(
            world_leaf
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn cursor_away_and_back_preserves_marker() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") [bold] }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });

        let third_paragraph_id = {
            let view = editor.state().view();
            view.root().unwrap().child_blocks().nth(2).unwrap().id()
        };
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(
                    third_paragraph_id,
                    0,
                )),
            },
        });

        let second_paragraph_id = {
            let view = editor.state().view();
            view.root().unwrap().child_blocks().nth(1).unwrap().id()
        };
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(
                    second_paragraph_id,
                    0,
                )),
            },
        });

        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });

        let view = editor.state().view();
        let second = view.root().unwrap().child_blocks().nth(1).unwrap();
        let items = second.inline();
        let first = items.first().expect("typed text in second paragraph");
        assert!(
            first
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
    }

    fn type_text(editor: &mut Editor, text: &str) {
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: text.into() },
        });
    }

    fn set_range(editor: &mut Editor, block: editor_crdt::Dot, from: usize, to: usize) {
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::new(
                    editor_state::Position::new(block, from),
                    editor_state::Position::new(block, to),
                ),
            },
        });
    }

    fn set_caret(editor: &mut Editor, block: editor_crdt::Dot, offset: usize) {
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(
                    block, offset,
                )),
            },
        });
    }

    #[test]
    fn typing_within_block_coalesces_into_one_unit() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "a");
        type_text(&mut editor, "b");
        type_text(&mut editor, "c");
        assert_eq!(
            editor.undo_history.undos_len(),
            1,
            "continuous same-block typing within the interval is one undo unit"
        );
    }

    #[test]
    fn range_format_between_typing_yields_three_units() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "a");
        set_range(&mut editor, p1, 0, 3);
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: editor_model::ModifierType::Bold,
            },
        });
        set_caret(&mut editor, p1, 6);
        type_text(&mut editor, "b");
        assert_eq!(
            editor.undo_history.undos_len(),
            3,
            "typing, a range bold, then typing are three units — formatting never coalesces"
        );
    }

    #[test]
    fn typing_in_different_paragraph_does_not_coalesce() {
        let (state, _p1, p2) = state! {
            doc { root { p1: paragraph { text("aa") } p2: paragraph { text("bb") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "x");
        set_caret(&mut editor, p2, 2);
        type_text(&mut editor, "y");
        assert_eq!(
            editor.undo_history.undos_len(),
            2,
            "typing in a different block starts a fresh undo unit"
        );
    }

    #[test]
    fn noncontiguous_caret_in_same_block_does_not_coalesce() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "x");
        set_caret(&mut editor, p1, 4);
        type_text(&mut editor, "y");
        assert_eq!(
            editor.undo_history.undos_len(),
            2,
            "a caret jump within the same block breaks coalescing (same block alone is not enough)"
        );
    }

    #[test]
    fn selection_overwrite_does_not_chain_typing_runs() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "a");
        set_range(&mut editor, p1, 0, 3);
        type_text(&mut editor, "X");
        type_text(&mut editor, "Y");
        assert_eq!(
            editor.undo_history.undos_len(),
            3,
            "a selection overwrite is isolated: typing does not coalesce across it"
        );
    }

    #[test]
    fn auto_surround_is_its_own_unit() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        type_text(&mut editor, "a");
        set_range(&mut editor, p1, 7, 12);
        type_text(&mut editor, "(");
        assert_eq!(
            editor.undo_history.undos_len(),
            2,
            "auto-surround wrapping does not coalesce with a preceding typing run"
        );
    }

    #[test]
    fn delete_all_restores_marker_then_retype() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("X") [bold] } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        });

        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "Y".into() },
        });

        let view = editor.state().view();
        let paragraph = view.root().unwrap().child_blocks().next().unwrap();
        let items = paragraph.inline();
        let first = items.first().expect("retyped text in paragraph");
        assert!(
            first
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
    }
}

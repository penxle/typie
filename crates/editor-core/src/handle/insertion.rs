use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_insertion_op(editor: &mut Editor, op: InsertionOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match op {
            InsertionOp::Text { text } => {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::insert_paragraph_after_unit_selection(),
                        |tr| commands::chain!(
                            tr,
                            commands::optional!(commands::ensure_paragraph()),
                            commands::optional!(commands::delete_selection()),
                        ),
                    ),
                    commands::insert_text(&text),
                )?;
            }
            InsertionOp::Break {
                kind: Break::Paragraph,
            } => {
                commands::first!(
                    tr,
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
            }
            InsertionOp::Break { kind: Break::Line } => {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
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
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
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
            }
            InsertionOp::Fragment { fragment } => {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
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
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn insert_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: " world".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_block() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") } paragraph { t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_line() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Line },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") hard_break {} t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_page() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Page },
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("hel") page_break {} }
                paragraph { t2: text("lo") }
            } }
            selection: (t2, 0)
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
                paragraph { t1: text("b") }
                paragraph { text("c") }
            } }
            selection: (t1, 1)
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
                paragraph { t1: text("b") }
            } }
            selection: (t1, 1)
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
        // Unit preserved; the page break needs a paragraph host, so the inserted
        // paragraph is split and the marker lands in the leading half.
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
}

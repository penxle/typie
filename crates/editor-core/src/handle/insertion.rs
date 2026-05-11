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
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    commands::insert_text(&text),
                )?;
            }
            InsertionOp::Break {
                kind: Break::Paragraph,
            } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::first!(
                        tr,
                        commands::lift_paragraph_forward(),
                        commands::split_paragraph(),
                    ),
                )?;
            }
            InsertionOp::Break { kind: Break::Line } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    commands::insert_hard_break(),
                )?;
            }
            InsertionOp::Break { kind: Break::Page } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    commands::split_paragraph(),
                    commands::insert_page_break_into_prev_paragraph(),
                )?;
            }
            InsertionOp::Fragment { fragment } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
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
    fn insert_text_replaces_node_selection_with_paragraph() {
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
        // Predicted: ensure_paragraph → [a, p_new(empty), c].
        // lift_paragraph_forward returns false (next sibling is paragraph).
        // split_paragraph splits empty paragraph at offset 0 → 2 empty paragraphs.
        // Cursor at the new (second) paragraph.
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph
                p2: paragraph
                paragraph { text("c") }
            } }
            selection: (p2, 0)
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
        // Predicted: ensure_paragraph → [a, p_new(empty), c], cursor (p_new, 0).
        // insert_hard_break Case D: inserts hard_break into empty paragraph,
        // cursor moves to (p_new, 1).
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
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
        // Predicted:
        //   ensure_paragraph → [a, p_new(empty), c], cursor (p_new, 0).
        //   split_paragraph: splits empty p_new at offset 0 → 2 empty paragraphs,
        //     cursor at new (second) paragraph.
        //   insert_page_break_into_prev_paragraph: appends page_break to p_new
        //     (the first empty paragraph, now the prev sibling).
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { page_break }
                p2: paragraph
                paragraph { text("c") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_fragment_replaces_node_selection() {
        // Use hr fragment as a stable default-constructible block-level leaf.
        // The chain replaces the selected hr with a fresh hr at the same position.
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
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { t1: text("c") }
            } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

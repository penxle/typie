use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_op(editor: &mut Editor, op: ClipboardOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        if let ClipboardOp::Paste { text, html } = op {
            if html.is_some() {
                // not yet implemented
            } else {
                commands::chain!(
                    tr,
                    |tr| commands::first!(
                        tr,
                        commands::materialize_gap_paragraph(),
                        |tr| commands::chain!(
                            tr,
                            commands::optional!(commands::ensure_paragraph()),
                            commands::optional!(commands::delete_selection()),
                        ),
                    ),
                    commands::insert_text(&text),
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
    fn paste_text_replaces_node_selection() {
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                text: "b".into(),
                html: None,
            },
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
    fn paste_text_at_leading_gap_creates_paragraph() {
        // Leading-unit gap: collapsed Upstream caret before root's first
        // child. Pasting text must materialize a real paragraph there and
        // land the pasted text in it.
        let (s, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                text: "pasted".into(),
                html: None,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("pasted") } image paragraph { text("b") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

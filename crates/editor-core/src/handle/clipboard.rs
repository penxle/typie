use editor_clipboard::Slice;
use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_op(editor: &mut Editor, op: ClipboardOp) -> Result<(), EditorError> {
    match op {
        ClipboardOp::Cut => editor.transact(|tr| {
            commands::delete_selection(tr)?;
            Ok(())
        }),
        ClipboardOp::Paste { html, text } => {
            let slice = Slice::from_payload(html.as_deref(), &text);
            editor.transact(|tr| {
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(tr, slice.clone()),
                )?;
                Ok(())
            })
        }
    }
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

    #[test]
    fn cut_deletes_selection() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Ho") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cut_then_paste_round_trip_identity() {
        let (s, ..) = state! {
            doc { r1: root {
                callout { paragraph { text("1") } }
                paragraph {}
            } }
            selection: (r1, 0, >) -> (r1, 2, <)
        };

        let payload = Slice::extract(&s).expect("non-collapsed").to_payload();

        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { r: root {
                callout { paragraph { text("1") } }
                paragraph {}
            } }
            selection: (r, 0)
        };
        editor_model::assert_doc_eq!(&editor.state().doc, &expected.doc);
    }

    #[test]
    fn paste_crlf_text_does_not_fail() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "a\r\nb".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") hard_break t2: text("b") } } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_html_with_meta_lossless() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("source") } } }
            selection: (t1, 0) -> (t1, 6)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t3: text("Hsourcei") } } }
            selection: (t3, 7)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

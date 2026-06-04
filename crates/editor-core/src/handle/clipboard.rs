use editor_clipboard::Slice;
use editor_commands::{self as commands};
use editor_model::{Fragment, PlainNode};
use editor_state::enclosing_table_cell;
use editor_transaction::{HistoryMeta, HistoryTag};

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
            let slice = {
                let resource = editor.resource.lock().unwrap();
                Slice::from_payload(html.as_deref(), &text, &resource)
            };
            let is_html_paste = html.as_deref().is_some_and(|h| !h.is_empty());
            let plain_text = is_html_paste.then(|| text.clone());
            editor.transact(|tr| {
                if let Some(plain_text) = plain_text {
                    tr.update_meta(|m| {
                        m.history = HistoryMeta::Tagged {
                            tag: HistoryTag::PasteHtml { plain_text },
                        };
                    });
                }
                let in_cell_context = is_cell_rect_selection(tr) || caret_in_table_cell(tr);
                if in_cell_context && slice_has_table(&slice) {
                    commands::paste_cells_into_cell_rect(tr, slice.clone())?;
                    return Ok(());
                }
                if is_cell_rect_selection(tr) {
                    commands::fill_cell_rect_with_slice(tr, slice.clone())?;
                    return Ok(());
                }
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
        ClipboardOp::RepasteAsText => {
            let Some(HistoryTag::PasteHtml { plain_text }) = editor.last_history_tag() else {
                return Ok(());
            };
            let plain_text = plain_text.clone();
            let Some(inverse_steps) = editor.history_last_inverse_steps() else {
                return Ok(());
            };
            let plain_slice = Slice::from_text(&plain_text);
            editor.transact(|tr| {
                tr.apply_steps(inverse_steps)?;
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(tr, plain_slice.clone()),
                )?;
                Ok(())
            })
        }
    }
}

fn is_cell_rect_selection(tr: &editor_transaction::Transaction) -> bool {
    let Some(sel) = tr.selection() else {
        return false;
    };
    let doc = tr.doc();
    sel.resolve(&doc).and_then(|rs| rs.as_cell_rect()).is_some()
}

fn caret_in_table_cell(tr: &editor_transaction::Transaction) -> bool {
    let Some(sel) = tr.selection() else {
        return false;
    };
    if !sel.is_collapsed() {
        return false;
    }
    let doc = tr.doc();
    enclosing_table_cell(&doc, sel.head.node_id).is_some()
}

fn slice_has_table(slice: &Slice) -> bool {
    fn walk(f: &Fragment) -> bool {
        if matches!(f.node, PlainNode::Table(_)) {
            return true;
        }
        f.children.iter().any(walk)
    }
    walk(&slice.fragment)
}

#[cfg(test)]
mod tests {
    use crate::event::EditorEvent;
    use crate::state_field::StateField;
    use editor_macros::state;
    use editor_model::ModifierType;
    use editor_state::{DocFlatExt, ResolvedPositionFlatExt, assert_state_eq};
    use editor_transaction::HistoryTag;

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn has_state_field(events: &[EditorEvent], field: StateField) -> bool {
        events.iter().any(|event| {
            matches!(event, EditorEvent::StateChanged { fields } if fields.contains(&field))
        })
    }

    #[test]
    fn probe_cut_with_collapsed_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Clipboard {
                op: ClipboardOp::Cut,
            },
        );
    }

    #[test]
    fn probe_paste_empty() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_probe_predicts_apply(
            state,
            Message::Clipboard {
                op: ClipboardOp::Paste {
                    text: "".into(),
                    html: None,
                },
            },
        );
    }

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
    fn paste_backward_open_paragraph_copy_at_text_end() {
        let (source, _t1, t2) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("a")
                    }
                    paragraph {
                        t2: text("b")
                    }
                }
            }
            selection: (t2, 0, <) -> (t1, 0, >)
        };
        let payload = Slice::extract(&source).expect("non-collapsed").to_payload();
        let target = editor_state::State {
            selection: Some(editor_state::Selection::collapsed(
                editor_state::Position::new(t2, 1),
            )),
            ..source
        };
        let mut editor = Editor::new_test(target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("a")
                    }
                    paragraph {
                        text("ba")
                    }
                    p: paragraph {
                    }
                }
            }
            selection: (p, 0)
        };
        assert_state_eq!(editor.state(), &expected);
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
    fn cut_on_cell_rect_clears_cells_keeps_structure() {
        let (s, c00, c01, c10, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(&s.doc, c00, c11).unwrap();
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        for cid in [c00, c01, c10, c11] {
            let cell = editor.state().doc.node(cid).expect("cell survives cut");
            assert_eq!(cell.children().count(), 1);
            assert_eq!(cell.first_child().unwrap().children().count(), 0);
        }
    }

    #[test]
    fn cut_on_full_table_cell_rect_removes_table() {
        let (s, tbl, c00, c11) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(&s.doc, c00, c11).unwrap();
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        assert!(editor.state().doc.node(tbl).is_none());
    }

    #[test]
    fn paste_table_payload_into_cell_rect_overwrites_cells() {
        let (s_src, _, c00s, _, _, _, c11s) = state! {
            doc { root { table {
                tr0: table_row {
                    c00s: table_cell { paragraph { text("X") } }
                    c01s: table_cell { paragraph { text("Y") } }
                }
                tr1: table_row {
                    c10s: table_cell { paragraph { text("Z") } }
                    c11s: table_cell { paragraph { text("W") } }
                }
            } } }
            selection: (c00s, 0)
        };
        let sel_src = editor_state::cell_rect_selection(&s_src.doc, c00s, c11s).unwrap();
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src).unwrap().to_payload();

        let (s_tgt, tbl, c00t, c11t) = state! {
            doc { root { tbl: table {
                table_row {
                    c00t: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11t: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00t, 0)
        };
        let sel_tgt = editor_state::cell_rect_selection(&s_tgt.doc, c00t, c11t).unwrap();
        let s_tgt = editor_state::State {
            selection: Some(sel_tgt),
            ..s_tgt
        };
        let mut editor = Editor::new_test(s_tgt);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let tbl = editor.state().doc.node(tbl).expect("table survives paste");
        assert_eq!(tbl.children().count(), 2);
        let texts: Vec<String> = tbl
            .children()
            .flat_map(|row| {
                row.children().map(|cell| {
                    let mut out = String::new();
                    fn walk(n: editor_model::NodeRef<'_>, out: &mut String) {
                        if let editor_model::Node::Text(t) = n.node() {
                            out.push_str(&t.text.to_string());
                        }
                        for c in n.children() {
                            walk(c, out);
                        }
                    }
                    walk(cell, &mut out);
                    out
                })
            })
            .collect();
        assert_eq!(texts, vec!["X", "Y", "x", "Z", "W", "y"]);
    }

    #[test]
    fn paste_5x1_table_into_3x2_extends_target_to_5x2() {
        let (s_src, _, sc00, _, _, _, _, _, _, _, sc40) = state! {
            doc { root { table {
                tr0: table_row { sc00: table_cell { paragraph { text("A") } } }
                tr1: table_row { sc10: table_cell { paragraph { text("B") } } }
                tr2: table_row { sc20: table_cell { paragraph { text("C") } } }
                tr3: table_row { sc30: table_cell { paragraph { text("D") } } }
                tr4: table_row { sc40: table_cell { paragraph { text("E") } } }
            } } }
            selection: (sc00, 0)
        };
        let sel_src = editor_state::cell_rect_selection(&s_src.doc, sc00, sc40).unwrap();
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src).unwrap().to_payload();

        let (s_tgt, tbl, c00, c21) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
                table_row {
                    table_cell { paragraph { text("e") } }
                    c21: table_cell { paragraph { text("f") } }
                    table_cell { paragraph { text("z") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel_tgt = editor_state::cell_rect_selection(&s_tgt.doc, c00, c21).unwrap();
        let s_tgt = editor_state::State {
            selection: Some(sel_tgt),
            ..s_tgt
        };
        let mut editor = Editor::new_test(s_tgt);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        fn cell_text_at(
            doc: &editor_model::Doc,
            tbl: editor_model::NodeId,
            row: usize,
            col: usize,
        ) -> String {
            let table = doc.node(tbl).expect("table");
            let row_ref = table.children().nth(row).expect("row");
            let cell = row_ref.children().nth(col).expect("cell");
            let mut out = String::new();
            fn walk(n: editor_model::NodeRef<'_>, out: &mut String) {
                if let editor_model::Node::Text(t) = n.node() {
                    out.push_str(&t.text.to_string());
                }
                for c in n.children() {
                    walk(c, out);
                }
            }
            walk(cell, &mut out);
            out
        }

        let doc = &editor.state().doc;
        let table = doc.node(tbl).expect("table survives");
        assert_eq!(table.children().count(), 5, "target now has 5 rows");
        for row in table.children() {
            assert_eq!(row.children().count(), 3, "every row keeps 3 cols");
        }
        for (row, ch) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            assert_eq!(cell_text_at(doc, tbl, row, 0), *ch);
        }
        assert_eq!(cell_text_at(doc, tbl, 0, 1), "b");
        assert_eq!(cell_text_at(doc, tbl, 1, 1), "d");
        assert_eq!(cell_text_at(doc, tbl, 2, 1), "f");
        assert_eq!(cell_text_at(doc, tbl, 3, 1), "");
        assert_eq!(cell_text_at(doc, tbl, 4, 1), "");
        assert_eq!(cell_text_at(doc, tbl, 0, 2), "x");
        assert_eq!(cell_text_at(doc, tbl, 1, 2), "y");
        assert_eq!(cell_text_at(doc, tbl, 2, 2), "z");
        assert_eq!(cell_text_at(doc, tbl, 3, 2), "");
        assert_eq!(cell_text_at(doc, tbl, 4, 2), "");
    }

    #[test]
    fn paste_table_at_cell_caret_extends_target() {
        let (s_src, _, sc00, _, _, _, sc20) = state! {
            doc { root { table {
                tr0: table_row { sc00: table_cell { paragraph { text("A") } } }
                tr1: table_row { sc10: table_cell { paragraph { text("B") } } }
                tr2: table_row { sc20: table_cell { paragraph { text("C") } } }
            } } }
            selection: (sc00, 0)
        };
        let sel_src = editor_state::cell_rect_selection(&s_src.doc, sc00, sc20).unwrap();
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src).unwrap().to_payload();

        let (s_tgt, tbl, _, _, ct, _, _, _, _) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { ct: text("hi") } }
                    c01: table_cell { paragraph { text("x") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("y") } }
                    c11: table_cell { paragraph { text("z") } }
                }
            } } }
            selection: (ct, 1)
        };
        let mut editor = Editor::new_test(s_tgt);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let _ = ct;

        let doc = &editor.state().doc;
        let table = doc.node(tbl).expect("table survives");
        assert_eq!(table.children().count(), 3);
        for row in table.children() {
            assert_eq!(row.children().count(), 2);
        }

        fn cell_text_at(
            doc: &editor_model::Doc,
            tbl: editor_model::NodeId,
            row: usize,
            col: usize,
        ) -> String {
            let table = doc.node(tbl).expect("table");
            let row_ref = table.children().nth(row).expect("row");
            let cell = row_ref.children().nth(col).expect("cell");
            let mut out = String::new();
            fn walk(n: editor_model::NodeRef<'_>, out: &mut String) {
                if let editor_model::Node::Text(t) = n.node() {
                    out.push_str(&t.text.to_string());
                }
                for c in n.children() {
                    walk(c, out);
                }
            }
            walk(cell, &mut out);
            out
        }

        assert_eq!(cell_text_at(doc, tbl, 0, 0), "A");
        assert_eq!(cell_text_at(doc, tbl, 1, 0), "B");
        assert_eq!(cell_text_at(doc, tbl, 2, 0), "C");
        assert_eq!(cell_text_at(doc, tbl, 0, 1), "x");
        assert_eq!(cell_text_at(doc, tbl, 1, 1), "z");
        assert_eq!(cell_text_at(doc, tbl, 2, 1), "");
    }

    #[test]
    fn paste_plain_text_into_cell_rect_fills_every_cell() {
        let (s, c00, c01, c10, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(&s.doc, c00, c11).unwrap();
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "hello".into(),
            },
        });
        fn cell_text(doc: &editor_model::Doc, id: editor_model::NodeId) -> String {
            fn walk(n: editor_model::NodeRef<'_>, out: &mut String) {
                if let editor_model::Node::Text(t) = n.node() {
                    out.push_str(&t.text.to_string());
                }
                for c in n.children() {
                    walk(c, out);
                }
            }
            let mut out = String::new();
            if let Some(n) = doc.node(id) {
                walk(n, &mut out);
            }
            out
        }
        let doc = &editor.state().doc;
        for cid in [c00, c01, c10, c11] {
            assert_eq!(cell_text(doc, cid), "hello");
        }
    }

    #[test]
    fn paste_table_into_cell_rect_undo_restores_state() {
        let (s_src, _, c00s, _, _, _, c11s) = state! {
            doc { root { table {
                tr0: table_row {
                    c00s: table_cell { paragraph { text("X") } }
                    c01s: table_cell { paragraph { text("Y") } }
                }
                tr1: table_row {
                    c10s: table_cell { paragraph { text("Z") } }
                    c11s: table_cell { paragraph { text("W") } }
                }
            } } }
            selection: (c00s, 0)
        };
        let sel_src = editor_state::cell_rect_selection(&s_src.doc, c00s, c11s).unwrap();
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src).unwrap().to_payload();

        let (s_tgt, _tbl, c00t, c11t) = state! {
            doc { root { tbl: table {
                table_row {
                    c00t: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11t: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00t, 0)
        };
        let sel_tgt = editor_state::cell_rect_selection(&s_tgt.doc, c00t, c11t).unwrap();
        let s_tgt = editor_state::State {
            selection: Some(sel_tgt),
            ..s_tgt
        };
        let before = s_tgt.doc.clone();
        let mut editor = Editor::new_test(s_tgt);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor_model::assert_doc_eq!(&editor.state().doc, &before);
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

    #[test]
    fn paste_html_sets_paste_html_tag() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text.clone(),
            },
        });

        let tag = editor.last_history_tag().cloned();
        assert!(
            matches!(tag, Some(HistoryTag::PasteHtml { ref plain_text }) if plain_text == &payload.text),
            "expected PasteHtml tag with plain_text == payload.text, got {tag:?}"
        );
    }

    #[test]
    fn paste_with_text_only_does_not_set_tag() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "plain".into(),
            },
        });
        assert!(editor.last_history_tag().is_none());
    }

    #[test]
    fn paste_with_empty_html_does_not_set_tag() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(String::new()),
                text: "plain".into(),
            },
        });
        assert!(editor.last_history_tag().is_none());
    }

    #[test]
    fn repaste_as_text_replaces_paste_region_with_plain() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
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
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let (expected, ..) = state! {
            doc { root { paragraph { t3: text("Hhelloi") } } }
            selection: (t3, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn last_history_tag_field_tracks_repaste_as_text_availability() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);
        let expected_text = payload.text.clone();

        let paste_events = editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        assert!(matches!(
            editor.last_history_tag(),
            Some(HistoryTag::PasteHtml { plain_text }) if plain_text == &expected_text
        ));
        assert!(has_state_field(&paste_events, StateField::LastHistoryTag));

        let repaste_events = editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        assert!(editor.last_history_tag().is_none());
        assert!(has_state_field(&repaste_events, StateField::LastHistoryTag));
    }

    #[test]
    fn last_history_tag_field_emits_for_repeated_equal_html_paste() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let mut editor = Editor::new_test(s_target);
        let html = payload.html.clone();
        let text = payload.text.clone();

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(html.clone()),
                text: text.clone(),
            },
        });
        let repeated_events = editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(html),
                text,
            },
        });

        assert!(has_state_field(
            &repeated_events,
            StateField::LastHistoryTag
        ));
    }

    #[test]
    fn repaste_as_text_is_noop_when_last_tag_absent() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let initial = s.clone();
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn repaste_as_text_expires_after_other_edit() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_deletion() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
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
        editor.apply(Message::Deletion {
            op: DeletionOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_modifier_toggle() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 0) -> (t2, 2)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_ime_composition_start() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
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

        let state = editor.state();
        let head_flat = state
            .selection
            .unwrap()
            .head
            .resolve(&state.doc)
            .unwrap()
            .to_flat();
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition {
                start: head_flat,
                end: head_flat,
            }],
        });

        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_selection_change() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, t2) = state! {
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
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(t2, 0)),
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_undo() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
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
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_undo_returns_to_html_paste_state() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
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
        let after_paste = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor_state::assert_state_eq!(editor.state(), &after_paste);
    }

    #[test]
    fn repaste_as_text_undo_twice_returns_to_pre_paste_state() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let pre_paste = s_target.clone();
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor_state::assert_state_eq!(editor.state(), &pre_paste);
    }

    #[test]
    fn repaste_as_text_is_one_shot() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
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
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        let after_first = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &after_first);
    }

    #[test]
    fn repaste_as_text_expires_after_remote_changeset() {
        use std::collections::BTreeMap;

        use editor_crdt::TextOp;
        use editor_model::{
            Doc, DocOp, Modifier, ModifierType, NodeId, PlainDoc, PlainNode, PlainNodeEntry,
            PlainParagraphNode, PlainRootNode, PlainTextNode,
        };
        use editor_state::{Position, Selection, State};
        use hashbrown::HashSet;

        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let root_modifiers = BTreeMap::from([
            (
                ModifierType::FontFamily,
                Modifier::FontFamily {
                    value: "Pretendard".into(),
                },
            ),
            (
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 400 },
            ),
        ]);

        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId::ROOT,
            PlainNodeEntry {
                parent: None,
                children: vec![para_id],
                modifiers: root_modifiers,
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para_id,
            PlainNodeEntry {
                parent: Some(NodeId::ROOT),
                children: vec![text_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text_id,
            PlainNodeEntry {
                parent: Some(para_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Text(PlainTextNode {
                    text: String::new(),
                }),
            },
        );
        let plain = PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        };

        let (doc, graph) = Doc::from_plain(plain);
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let seed = State::new(doc, graph, Some(sel));
        let seed_css = seed.graph.changesets_as_vec();
        let replica_b = State::from_changesets(seed_css, Some(sel)).expect("from_changesets");

        let baseline: HashSet<_> = seed.graph.current_heads().copied().collect();
        let (state_a, _) = seed
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'r',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let mut css = state_a.local_changesets_since(&baseline).unwrap();
        let remote_cs = css.remove(0);

        let mut editor = Editor::new_test(replica_b);

        let source_payload = {
            let (s_source, ..) = state! {
                doc { root { paragraph { t1: text("hello") } } }
                selection: (t1, 0) -> (t1, 5)
            };
            Slice::extract(&s_source).unwrap().to_payload()
        };
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(source_payload.html),
                text: source_payload.text,
            },
        });

        editor.receive_remote_changeset(remote_cs);
        editor.tick().unwrap();

        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_survives_duplicate_remote_changeset() {
        let (s_source, ..) = state! {
            doc { root { paragraph { t1: text("hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let payload = Slice::extract(&s_source).unwrap().to_payload();

        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("Hi") } } }
            selection: (t2, 1)
        };
        let duplicate_cs = s_target
            .graph
            .changesets_as_vec()
            .into_iter()
            .next()
            .expect("fixture graph has an initial changeset");
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        editor.receive_remote_changeset(duplicate_cs);
        let remote_events = editor.tick().unwrap();
        assert!(
            !has_state_field(&remote_events, StateField::LastHistoryTag),
            "duplicate remote changeset must not change last history tag"
        );
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let (expected, ..) = state! {
            doc { root { paragraph { t3: text("Hhelloi") } } }
            selection: (t3, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn repaste_as_text_preserves_list_marker_from_text_payload() {
        let (s_target, ..) = state! {
            doc { root { paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some("<ul><li>a</li><li>b</li></ul>".into()),
                text: "1. a\n2. b".into(),
            },
        });
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let plain_text_in_doc = editor
            .state()
            .doc
            .flat_text(0..editor.state().doc.flat_size());
        assert!(
            plain_text_in_doc.contains("1. a") && plain_text_in_doc.contains("2. b"),
            "expected list marker preserved from text payload, got {plain_text_in_doc:?}"
        );
    }
}

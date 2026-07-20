use editor_clipboard::{PayloadSource, Slice};
use editor_commands::{self as commands};
use editor_common::HistoryTag;
use editor_model::{Fragment, PlainNode};
use editor_state::{
    ResolvedPosition, ResolvedPositionFlatExt, Selection, StableSelection, enclosing_table_cell,
};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::message::*;
use crate::state_field::StateField;

pub fn handle_clipboard_op(editor: &mut Editor, op: ClipboardOp) -> Result<(), EditorError> {
    match op {
        ClipboardOp::Cut => editor.transact(|tr| {
            commands::delete_selection(tr)?;
            Ok(())
        }),
        ClipboardOp::Paste { html, text } => {
            let (slice, source) = {
                let resource = editor.resource.lock().unwrap();
                Slice::from_payload(html.as_deref(), &text, &resource)
            };
            let is_html_paste = source == PayloadSource::Html;
            let provenance = if is_html_paste {
                commands::types::SliceProvenance::Formatted
            } else {
                commands::types::SliceProvenance::Plain
            };
            let plain_text = is_html_paste.then(|| text.clone());
            let (start_flat, pre_block) = if is_html_paste {
                let view = editor.state().view();
                let sel = editor.state().selection;
                let start_flat = sel
                    .and_then(|s| s.resolve(&view))
                    .map(|rs| rs.from().to_flat());
                (start_flat, sel.map(|s| s.head.node))
            } else {
                (None, None)
            };
            editor.transact(|tr| {
                if let Some(plain_text) = plain_text.clone() {
                    tr.update_meta(|m| {
                        m.history = HistoryMeta::Tagged {
                            tag: HistoryTag::PasteHtml {
                                plain_text,
                                start: None,
                            },
                        };
                    });
                }
                let in_cell_context = is_cell_rect_selection(tr) || caret_in_table_cell(tr);
                if in_cell_context && slice_has_table(&slice) {
                    commands::paste_cells_into_cell_rect(tr, slice.clone())?;
                    return Ok(());
                }
                if is_cell_rect_selection(tr) {
                    commands::fill_cell_rect_with_slice(tr, slice.clone(), provenance)?;
                    return Ok(());
                }
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(tr, slice.clone(), provenance),
                )?;
                Ok(())
            })?;
            if !editor.is_probing()
                && let (Some(start), Some(pre), Some(plain_text)) =
                    (start_flat, pre_block, plain_text)
                && matches!(
                    editor.last_history_tag(),
                    Some(HistoryTag::PasteHtml { .. })
                )
                && editor.state().selection.map(|s| s.head.node) == Some(pre)
            {
                editor
                    .undo_history
                    .set_last_tag(Some(HistoryTag::PasteHtml {
                        plain_text,
                        start: Some(start),
                    }));
            }
            Ok(())
        }
        ClipboardOp::RepasteAsText => {
            let Some(HistoryTag::PasteHtml { plain_text, start }) = editor.last_history_tag()
            else {
                return Ok(());
            };
            let plain_slice = Slice::from_text(&plain_text);

            let inline = start.and_then(|start_flat| {
                let view = editor.state().view();
                let end_flat = editor
                    .state()
                    .selection
                    .and_then(|s| s.resolve(&view))
                    .map(|rs| rs.to().to_flat())?;
                let anchor = ResolvedPosition::from_flat(&view, start_flat)?;
                let head = ResolvedPosition::from_flat(&view, end_flat)?;
                let a: editor_state::Position = (&anchor).into();
                let h: editor_state::Position = (&head).into();
                (a.node == h.node).then_some((Selection::new(a, h), start_flat, end_flat))
            });

            if let Some((range, span_start, span_end)) = inline {
                let reanchors: Vec<(String, usize, usize)> = {
                    let state = editor.state();
                    let view = state.view();
                    editor
                        .tracked_ranges()
                        .iter()
                        .filter_map(|range| {
                            let resolved = range.locate(state)?.resolve(&view)?;
                            let a = resolved.from().to_flat();
                            let b = resolved.to().to_flat();
                            (a >= span_start && b <= span_end).then(|| (range.id.clone(), a, b))
                        })
                        .collect()
                };
                editor.transact(|tr| {
                    tr.set_selection(Some(range))?;
                    commands::chain!(
                        tr,
                        commands::optional!(commands::delete_selection()),
                        |tr| commands::insert_slice(
                            tr,
                            plain_slice.clone(),
                            commands::types::SliceProvenance::Plain
                        ),
                    )?;
                    Ok(())
                })?;
                if !editor.is_probing() {
                    let mut changed = false;
                    for (id, a, b) in reanchors {
                        let stable = {
                            let view = editor.state().view();
                            match (
                                ResolvedPosition::from_flat(&view, a),
                                ResolvedPosition::from_flat(&view, b),
                            ) {
                                (Some(anchor), Some(head)) => Some(StableSelection::capture(
                                    &Selection::new((&anchor).into(), (&head).into()),
                                    &view,
                                )),
                                _ => None,
                            }
                        };
                        if let Some(stable) = stable {
                            changed |= editor.tracked_ranges_mut().set_selection(&id, stable);
                        }
                    }
                    if changed {
                        editor.push_event(EditorEvent::StateChanged {
                            fields: vec![StateField::TrackedRanges],
                        });
                    }
                }
                return Ok(());
            }

            if !editor.try_undo() {
                return Ok(());
            }
            editor.transact(|tr| {
                commands::chain!(
                    tr,
                    commands::optional!(commands::materialize_gap_paragraph()),
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::insert_slice(
                        tr,
                        plain_slice.clone(),
                        commands::types::SliceProvenance::Plain
                    ),
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
    let doc = tr.view();
    sel.resolve(&doc).and_then(|rs| rs.as_cell_rect()).is_some()
}

fn caret_in_table_cell(tr: &editor_transaction::Transaction) -> bool {
    let Some(sel) = tr.selection() else {
        return false;
    };
    if !sel.is_collapsed() {
        return false;
    }
    let doc = tr.view();
    enclosing_table_cell(&doc, sel.head.node).is_some()
}

fn slice_has_table(slice: &Slice) -> bool {
    fn walk(f: &Fragment) -> bool {
        if matches!(f.node, PlainNode::Table(_)) {
            return true;
        }
        f.children.iter().any(walk)
    }
    slice.content.iter().any(walk)
}

#[cfg(test)]
mod tests {
    use crate::event::EditorEvent;
    use crate::state_field::StateField;
    use editor_common::HistoryTag;
    use editor_macros::state;
    use editor_model::ModifierType;
    use editor_resource::Resource;
    use editor_state::{ResolvedPositionFlatExt, Selection, assert_doc_eq, assert_state_eq};

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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
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
                p1: paragraph { text("b") }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
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
            doc { root { p1: paragraph { text("pasted") } image paragraph { text("b") } } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn cut_deletes_selection() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Ho") } } }
            selection: (p1, 1)
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

        let payload = Slice::extract(&s)
            .expect("non-collapsed")
            .to_payload(&Resource::new_test());

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
        assert_doc_eq!(editor.state().clone(), expected);
    }

    #[test]
    fn paste_backward_open_paragraph_copy_at_text_end() {
        let (source, _p1, p2) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("a")
                    }
                    p2: paragraph {
                        text("b")
                    }
                }
            }
            selection: (p2, 0, <) -> (p1, 0, >)
        };
        let payload = Slice::extract(&source)
            .expect("non-collapsed")
            .to_payload(&Resource::new_test());
        let target = editor_state::State {
            selection: Some(editor_state::Selection::collapsed(
                editor_state::Position::new(p2, 1),
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
    fn paste_empty_paragraph_copy_into_text_middle_inserts_block() {
        let (source, ..) = state! {
            doc { r: root { paragraph {} } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let payload = Slice::extract(&source)
            .expect("non-collapsed")
            .to_payload(&Resource::new_test());
        let (target, _p1) = state! {
            doc { root { p1: paragraph { text("asd") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p: paragraph {}
                paragraph { text("sd") }
            } }
            selection: (p, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_empty_paragraph_break_before_non_paragraph_into_text_middle_splits_once() {
        let (source, p1, ..) = state! {
            doc { root {
                p1: paragraph {}
                image
                paragraph {}
            } }
            selection: none
        };
        let selection = {
            let view = source.view();
            editor_state::paragraph_break_at_end(&editor_state::Position::new(p1, 0), &view)
                .expect("empty paragraph before non-paragraph has break")
        };
        let source = editor_state::State {
            selection: Some(selection),
            ..source
        };
        let payload = Slice::extract(&source)
            .expect("non-collapsed")
            .to_payload(&Resource::new_test());

        let (target, _p1) = state! {
            doc { root { p1: paragraph { text("asd") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p2: paragraph { text("sd") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_open_empty_paragraph_range_into_text_middle_inserts_boundary() {
        let (source, _p1, _p2) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph { text("asd") }
                paragraph {}
            } }
            selection: (p1, 0, >) -> (p2, 0, <)
        };
        let payload = Slice::extract(&source)
            .expect("non-collapsed")
            .to_payload(&Resource::new_test());
        let (target, _p2) = state! {
            doc { root {
                paragraph {}
                p2: paragraph { text("asd") }
                paragraph {}
            } }
            selection: (p2, 1)
        };
        let mut editor = Editor::new_test(target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                paragraph { text("a") }
                p3: paragraph { text("sd") }
                paragraph {}
            } }
            selection: (p3, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_crlf_text_splits_into_paragraphs() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "a\r\nb".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: (p2, 1)
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
        let sel = {
            let view = s.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        let view = editor.state().view();
        for cid in [c00, c01, c10, c11] {
            let cell = view.node(cid).expect("cell survives cut");
            assert_eq!(cell.children().count(), 1);
            let editor_model::ChildView::Block(para) = cell.first_child().unwrap() else {
                panic!("cell child must be a paragraph block");
            };
            assert_eq!(para.children().count(), 0);
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
        let sel = {
            let view = s.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Cut,
        });
        assert!(editor.state().view().node(tbl).is_none());
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
        let sel_src = {
            let view = s_src.view();
            editor_state::cell_rect_selection(c00s, c11s, &view).unwrap()
        };
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src)
            .unwrap()
            .to_payload(&Resource::new_test());

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
        let sel_tgt = {
            let view = s_tgt.view();
            editor_state::cell_rect_selection(c00t, c11t, &view).unwrap()
        };
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

        let view = editor.state().view();
        let tbl = view.node(tbl).expect("table survives paste");
        assert_eq!(tbl.child_blocks().count(), 2);
        let mut texts: Vec<String> = Vec::new();
        for row in tbl.child_blocks() {
            for cell in row.child_blocks() {
                texts.push(cell.child_blocks().map(|p| p.inline_text()).collect());
            }
        }
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
        let sel_src = {
            let view = s_src.view();
            editor_state::cell_rect_selection(sc00, sc40, &view).unwrap()
        };
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src)
            .unwrap()
            .to_payload(&Resource::new_test());

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
        let sel_tgt = {
            let view = s_tgt.view();
            editor_state::cell_rect_selection(c00, c21, &view).unwrap()
        };
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

        fn cell_text_at<'a>(
            view: &'a editor_model::DocView<'a>,
            tbl: editor_crdt::Dot,
            row: usize,
            col: usize,
        ) -> String {
            let table = view.node(tbl).expect("table");
            let row_nv = table.child_blocks().nth(row).expect("row");
            let cell = row_nv.child_blocks().nth(col).expect("cell");
            cell.child_blocks().map(|p| p.inline_text()).collect()
        }

        let view = editor.state().view();
        let table = view.node(tbl).expect("table survives");
        assert_eq!(table.child_blocks().count(), 5, "target now has 5 rows");
        for row in table.child_blocks() {
            assert_eq!(row.child_blocks().count(), 3, "every row keeps 3 cols");
        }
        for (row, ch) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            assert_eq!(cell_text_at(&view, tbl, row, 0), *ch);
        }
        assert_eq!(cell_text_at(&view, tbl, 0, 1), "b");
        assert_eq!(cell_text_at(&view, tbl, 1, 1), "d");
        assert_eq!(cell_text_at(&view, tbl, 2, 1), "f");
        assert_eq!(cell_text_at(&view, tbl, 3, 1), "");
        assert_eq!(cell_text_at(&view, tbl, 4, 1), "");
        assert_eq!(cell_text_at(&view, tbl, 0, 2), "x");
        assert_eq!(cell_text_at(&view, tbl, 1, 2), "y");
        assert_eq!(cell_text_at(&view, tbl, 2, 2), "z");
        assert_eq!(cell_text_at(&view, tbl, 3, 2), "");
        assert_eq!(cell_text_at(&view, tbl, 4, 2), "");
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
        let sel_src = {
            let view = s_src.view();
            editor_state::cell_rect_selection(sc00, sc20, &view).unwrap()
        };
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_tgt, tbl, _, _, ct, _, _, _, _) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { ct: paragraph { text("hi") } }
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

        let view = editor.state().view();
        let table = view.node(tbl).expect("table survives");
        assert_eq!(table.child_blocks().count(), 3);
        for row in table.child_blocks() {
            assert_eq!(row.child_blocks().count(), 2);
        }

        fn cell_text_at<'a>(
            view: &'a editor_model::DocView<'a>,
            tbl: editor_crdt::Dot,
            row: usize,
            col: usize,
        ) -> String {
            let table = view.node(tbl).expect("table");
            let row_nv = table.child_blocks().nth(row).expect("row");
            let cell = row_nv.child_blocks().nth(col).expect("cell");
            cell.child_blocks().map(|p| p.inline_text()).collect()
        }

        assert_eq!(cell_text_at(&view, tbl, 0, 0), "A");
        assert_eq!(cell_text_at(&view, tbl, 1, 0), "B");
        assert_eq!(cell_text_at(&view, tbl, 2, 0), "C");
        assert_eq!(cell_text_at(&view, tbl, 0, 1), "x");
        assert_eq!(cell_text_at(&view, tbl, 1, 1), "z");
        assert_eq!(cell_text_at(&view, tbl, 2, 1), "");
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
        let sel = {
            let view = s.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
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
        fn cell_text<'a>(view: &'a editor_model::DocView<'a>, id: editor_crdt::Dot) -> String {
            match view.node(id) {
                Some(cell) => cell.child_blocks().map(|p| p.inline_text()).collect(),
                None => String::new(),
            }
        }
        let view = editor.state().view();
        for cid in [c00, c01, c10, c11] {
            assert_eq!(cell_text(&view, cid), "hello");
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
        let sel_src = {
            let view = s_src.view();
            editor_state::cell_rect_selection(c00s, c11s, &view).unwrap()
        };
        let s_src = editor_state::State {
            selection: Some(sel_src),
            ..s_src
        };
        let payload = Slice::extract(&s_src)
            .unwrap()
            .to_payload(&Resource::new_test());

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
        let sel_tgt = {
            let view = s_tgt.view();
            editor_state::cell_rect_selection(c00t, c11t, &view).unwrap()
        };
        let s_tgt = editor_state::State {
            selection: Some(sel_tgt),
            ..s_tgt
        };
        let before = s_tgt.clone();
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
        assert_doc_eq!(editor.state().clone(), before);
    }

    #[test]
    fn paste_html_with_meta_lossless() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("source") } } }
            selection: (p1, 0) -> (p1, 6)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        let (expected, ..) = state! {
            doc { root { p3: paragraph { text("Hsourcei") } } }
            selection: (p3, 7)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_page_break_slice_in_root_paragraph_splits_at_terminal() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("lo") page_break } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let payload = Slice::extract(&source)
            .unwrap()
            .to_payload(&Resource::new_test());
        let (target, ..) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Worlo") page_break }
                p2: paragraph { text("ld") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn paste_page_break_only_into_nested_selection_preserves_selection() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("a") page_break } } }
            selection: (p1, 1) -> (p1, 2)
        };
        let payload = Slice::extract(&source)
            .unwrap()
            .to_payload(&Resource::new_test());
        let (target, ..) = state! {
            doc { root {
                blockquote { p2: paragraph { text("Nested") } }
                paragraph {}
            } }
            selection: (p2, 1) -> (p2, 4)
        };
        let mut editor = Editor::new_test(target.clone());

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        assert_state_eq!(editor.state(), &target);
    }

    #[test]
    fn paste_html_sets_paste_html_tag() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("") } } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(s_target);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text.clone(),
            },
        });

        let tag = editor.last_history_tag();
        assert!(
            matches!(tag, Some(HistoryTag::PasteHtml { ref plain_text, .. }) if plain_text == &payload.text),
            "expected PasteHtml tag with plain_text == payload.text, got {tag:?}"
        );
    }

    #[test]
    fn paste_with_text_only_does_not_set_tag() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
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
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
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
    fn paste_html_that_yields_empty_slice_falls_back_to_plain_text() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(s);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some("<script>ignored</script>".into()),
                text: "plain".into(),
            },
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("plain") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
        assert!(editor.last_history_tag().is_none());
    }

    #[test]
    fn repaste_as_text_replaces_paste_region_with_plain() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p3: paragraph { text("Hhelloi") } } }
            selection: (p3, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn repaste_as_text_remaps_tracked_range_on_pasted_text() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        let selection = editor.state().selection.unwrap();
        let pasted = Selection::new(
            editor_state::Position::new(selection.head.node, selection.head.offset - 5),
            selection.head,
        );
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Add {
                id: "r".into(),
                group: "comment".into(),
                selection: pasted,
                metadata: String::new(),
            },
        });

        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });

        let range = editor.tracked_ranges().get("r").expect("range present");
        let view = editor.state().view();
        let resolved = range
            .locate(editor.state())
            .and_then(|sel| sel.resolve(&view))
            .map(|resolved| resolved.collect_text());
        assert_eq!(resolved.as_deref(), Some("hello"));
    }

    #[test]
    fn last_history_tag_field_tracks_repaste_as_text_availability() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            Some(HistoryTag::PasteHtml { ref plain_text, .. }) if plain_text == &expected_text
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("") } } }
            selection: (p2, 0)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 0) -> (p2, 2)
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });

        let state = editor.state();
        let view = state.view();
        let head_flat = state
            .selection
            .unwrap()
            .head
            .resolve(&view)
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
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, p2) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
                selection: editor_state::Selection::collapsed(editor_state::Position::new(p2, 0)),
            },
        });
        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_does_not_revive_after_selection_change_then_undo() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, p2) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let mut editor = Editor::new_test(s_target);

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html.clone()),
                text: payload.text.clone(),
            },
        });
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: editor_state::Selection::collapsed(editor_state::Position::new(p2, 0)),
            },
        });
        assert!(
            editor.last_history_tag().is_none(),
            "selection movement invalidates the first paste"
        );

        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some(payload.html),
                text: payload.text,
            },
        });
        assert!(
            matches!(
                editor.last_history_tag(),
                Some(HistoryTag::PasteHtml { .. })
            ),
            "second paste exposes a fresh repaste affordance"
        );

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert!(
            editor.last_history_tag().is_none(),
            "undoing the second paste must not revive the invalidated first paste"
        );

        let before = editor.state().clone();
        editor.apply(Message::Clipboard {
            op: ClipboardOp::RepasteAsText,
        });
        editor_state::assert_state_eq!(editor.state(), &before);
    }

    #[test]
    fn repaste_as_text_expires_after_undo() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
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
        use editor_crdt::{Dot, ListOp};
        use editor_model::{EditOp, SeqItem};
        use editor_state::State;
        use hashbrown::HashSet;

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let replica_b =
            State::from_changesets(css_a, replica_a.selection).expect("from_changesets");

        // Produce a remote changeset by inserting a char on a fork of replica_a's
        // projected graph (continuing actor 1's clock, unknown to replica_b).
        let remote_cs = {
            let mut pa = replica_a.projected.as_ref().clone();
            let baseline: HashSet<Dot> = pa.graph().current_heads().copied().collect();
            pa.apply_batch(vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('r'),
            })])
            .unwrap();
            pa.commit();
            pa.graph()
                .local_changesets_since(&baseline)
                .unwrap()
                .remove(0)
        };

        let mut editor = Editor::new_test(replica_b);

        let source_payload = {
            let (s_source, ..) = state! {
                doc { root { p1: paragraph { text("hello") } } }
                selection: (p1, 0) -> (p1, 5)
            };
            Slice::extract(&s_source)
                .unwrap()
                .to_payload(&Resource::new_test())
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
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let duplicate_cs = s_target
            .graph()
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
            doc { root { p3: paragraph { text("Hhelloi") } } }
            selection: (p3, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn repaste_as_text_preserves_list_marker_from_text_payload() {
        let (s_target, ..) = state! {
            doc { root { p2: paragraph { text("") } } }
            selection: (p2, 0)
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

        let view = editor.state().view();
        let plain_text_in_doc = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));
        assert!(
            plain_text_in_doc.contains("1. a") && plain_text_in_doc.contains("2. b"),
            "expected list marker preserved from text payload, got {plain_text_in_doc:?}"
        );
    }

    #[test]
    fn paste_plain_two_lines_after_bold_paints_and_carries() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "a\nb".into(),
            },
        });
        let view = editor.state().view();
        let paras: Vec<editor_crdt::Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(paras.len(), 2);
        assert_eq!(view.node(paras[0]).unwrap().inline_text(), "가a");
        assert_eq!(view.node(paras[1]).unwrap().inline_text(), "b");
        assert!(
            view.node(paras[1])
                .unwrap()
                .leaf_own_modifiers_at(0)
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "the pasted new paragraph text is painted bold"
        );
        let carry: Vec<editor_model::Modifier> = editor
            .state()
            .projected
            .carry_modifiers(paras[1])
            .into_values()
            .collect();
        assert!(
            carry
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "the pasted new paragraph records bold carry, got {carry:?}"
        );
    }

    #[test]
    fn repaste_as_text_paints_plain_with_continuation() {
        let (s_source, ..) = state! {
            doc { root { p1: paragraph { text("x") [italic] } } }
            selection: (p1, 0) -> (p1, 1)
        };
        let payload = Slice::extract(&s_source)
            .unwrap()
            .to_payload(&Resource::new_test());

        let (s_target, p2) = state! {
            doc { root { p2: paragraph { text("AB") [bold] } } }
            selection: (p2, 1)
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

        let view = editor.state().view();
        let p = view.node(p2).unwrap();
        assert_eq!(p.inline_text(), "AxB");
        assert!(
            p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "repaste-as-text paints the plain char with the caret continuation (bold)"
        );
        assert!(
            !p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Italic)),
            "the italic formatting from the html paste is dropped"
        );
    }

    #[test]
    fn paste_plain_into_cell_rect_paints_every_cell_with_pending() {
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
        let sel = {
            let view = s.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
        let s = editor_state::State {
            selection: Some(sel),
            pending_modifiers: vec![editor_state::PendingModifier::Set {
                modifier: editor_model::Modifier::Bold,
            }],
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: None,
                text: "p\nq".into(),
            },
        });
        let view = editor.state().view();
        for cid in [c00, c01, c10, c11] {
            let cell = view.node(cid).expect("cell survives");
            let paras: Vec<editor_crdt::Dot> = cell.child_blocks().map(|b| b.id()).collect();
            assert_eq!(paras.len(), 2, "each cell gets both plain lines");
            assert_eq!(view.node(paras[0]).unwrap().inline_text(), "p");
            assert_eq!(view.node(paras[1]).unwrap().inline_text(), "q");
            for pid in &paras {
                assert!(
                    view.node(*pid)
                        .unwrap()
                        .leaf_own_modifiers_at(0)
                        .iter()
                        .any(|m| matches!(m, editor_model::Modifier::Bold)),
                    "every pasted cell line is painted with the pending bold"
                );
                let carry: Vec<editor_model::Modifier> = editor
                    .state()
                    .projected
                    .carry_modifiers(*pid)
                    .into_values()
                    .collect();
                assert!(
                    carry
                        .iter()
                        .any(|m| matches!(m, editor_model::Modifier::Bold)),
                    "every pasted cell line records bold carry, got {carry:?}"
                );
            }
        }
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "the plain cell-rect paste consumes the pending format once"
        );
    }

    #[test]
    fn paste_list_item_with_two_paragraphs_splits_into_sibling_items_without_zombies() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some("<li><p>a</p><p>b</p></li>".into()),
                text: "a\nb".into(),
            },
        });

        let view = editor.state().view();
        let list = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.node_type() == editor_model::NodeType::BulletList)
            .expect("pasted list survives as a bullet list");
        let items: Vec<_> = list.child_blocks().collect();
        assert_eq!(
            items.len(),
            2,
            "list item with two paragraphs becomes two sibling list items"
        );
        assert!(
            items
                .iter()
                .all(|it| it.node_type() == editor_model::NodeType::ListItem)
        );
        assert_eq!(items[0].child_blocks().count(), 1);
        assert_eq!(items[1].child_blocks().count(), 1);
        assert_eq!(items[0].child_blocks().next().unwrap().inline_text(), "a");
        assert_eq!(items[1].child_blocks().next().unwrap().inline_text(), "b");

        let ps = &editor.state().projected;
        let visible: hashbrown::HashSet<editor_crdt::Dot> = ps
            .seq_checkout()
            .snapshot(ps.seq())
            .into_iter()
            .map(|(dot, _)| dot)
            .collect();
        let reachable: hashbrown::HashSet<editor_crdt::Dot> = ps
            .subtree_real_dots(editor_crdt::Dot::ROOT)
            .into_iter()
            .collect();
        let zombies: Vec<editor_crdt::Dot> = visible.difference(&reachable).copied().collect();
        assert!(
            zombies.is_empty(),
            "no visible-but-unreachable ops after paste: {zombies:?}"
        );

        // Fail-first witness: the inserted sequence is already schema-valid, so a
        // fresh cold reprojection has nothing to heal. If the fragment repair were
        // not wired, the invalid `ListItem`-with-two-paragraphs would reach the
        // sequence and the reprojection would repair it (repairs > 0).
        let css = editor.state().graph().changesets_as_vec();
        let cold = editor_state::State::from_changesets(css, None).expect("reprojects");
        assert_eq!(
            cold.projected.repair_stats().repairs,
            0,
            "paste inserted a schema-valid slice; no projection repair should be needed"
        );
    }

    #[test]
    fn paste_violating_list_into_cell_rect_is_repaired_before_insert() {
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
        let sel = {
            let view = s.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        let mut editor = Editor::new_test(s);
        editor.apply(Message::Clipboard {
            op: ClipboardOp::Paste {
                html: Some("<li><p>a</p><p>b</p></li>".into()),
                text: "a\nb".into(),
            },
        });

        // Every filled cell holds a valid list: two sibling items, one paragraph each.
        let view = editor.state().view();
        for cid in [c00, c01, c10, c11] {
            let cell = view.node(cid).expect("cell survives fill");
            let list = cell
                .child_blocks()
                .find(|b| b.node_type() == editor_model::NodeType::BulletList)
                .expect("cell holds the pasted bullet list");
            let items: Vec<_> = list.child_blocks().collect();
            assert_eq!(items.len(), 2, "filled cell splits into two sibling items");
            assert_eq!(items[0].child_blocks().next().unwrap().inline_text(), "a");
            assert_eq!(items[1].child_blocks().next().unwrap().inline_text(), "b");
        }

        // Coverage witness: the cell-rect path also repairs before insert, so the
        // reprojected sequence needs no healing.
        let css = editor.state().graph().changesets_as_vec();
        let cold = editor_state::State::from_changesets(css, None).expect("reprojects");
        assert_eq!(
            cold.projected.repair_stats().repairs,
            0,
            "cell-rect fill inserted a schema-valid slice; no projection repair should be needed"
        );
    }
}

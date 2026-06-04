use editor_clipboard::Slice;
use editor_model::{Fragment, NodeId, PlainNode, PlainParagraphNode};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{find_first_text_position, replace_cell_children};
use crate::{CommandError, CommandResult};

/// Replace every cell in a cell-rect selection with the same slice content.
/// Inline-only slices are wrapped in a single paragraph so the cell stays
/// schema-valid; block slices are inserted verbatim.
pub fn fill_cell_rect_with_slice(tr: &mut Transaction, slice: Slice) -> CommandResult {
    let blocks = slice_to_cell_blocks(&slice);
    if blocks.is_empty() {
        return Ok(false);
    }

    let (anchor_cell_id, cell_ids) = {
        let Some(sel) = tr.selection() else {
            return Ok(false);
        };
        let doc = tr.doc();
        let Some(rs) = sel.resolve(&doc) else {
            return Ok(false);
        };
        let Some(rect) = rs.as_cell_rect() else {
            return Ok(false);
        };
        let ids: Vec<NodeId> = rect.cells().map(|c| c.id()).collect();
        if ids.is_empty() {
            return Ok(false);
        }
        (ids[0], ids)
    };

    tr.batch::<_, CommandError>(|tr| {
        for cell_id in &cell_ids {
            replace_cell_children(tr, *cell_id, &blocks)?;
        }
        Ok(())
    })?;

    let cursor = find_first_text_position(&tr.doc(), anchor_cell_id)
        .unwrap_or_else(|| Position::new(anchor_cell_id, 0));
    tr.set_selection(Some(Selection::collapsed(cursor)))?;
    Ok(true)
}

fn slice_to_cell_blocks(slice: &Slice) -> Vec<Fragment> {
    let top_children: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };

    let mut out: Vec<Fragment> = Vec::new();
    let mut inline_run: Vec<Fragment> = Vec::new();
    let flush = |run: &mut Vec<Fragment>, out: &mut Vec<Fragment>| {
        if !run.is_empty() {
            out.push(Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                children: std::mem::take(run),
            });
        }
    };
    for child in top_children {
        match &child.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) => inline_run.push(child.clone()),
            _ => {
                flush(&mut inline_run, &mut out);
                out.push(child.clone());
            }
        }
    }
    flush(&mut inline_run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::Node;
    use editor_state::cell_rect_selection;

    use super::*;
    use crate::test_utils::*;

    fn with_cell_rect(
        initial: editor_state::State,
        anchor: NodeId,
        head: NodeId,
    ) -> editor_state::State {
        let sel = cell_rect_selection(&initial.doc, anchor, head).unwrap();
        editor_state::State {
            selection: Some(sel),
            ..initial
        }
    }

    fn cell_text(doc: &editor_model::Doc, id: NodeId) -> String {
        fn walk(node: editor_model::NodeRef<'_>, out: &mut String) {
            match node.node() {
                Node::Text(t) => out.push_str(&t.text.to_string()),
                _ => {
                    for c in node.children() {
                        walk(c, out);
                    }
                }
            }
        }
        let mut out = String::new();
        if let Some(n) = doc.node(id) {
            walk(n, &mut out);
        }
        out
    }

    #[test]
    fn returns_false_when_selection_is_not_cell_rect() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 5)
        };
        transact_fail!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            Slice::from_text("hi")
        ));
    }

    #[test]
    fn fills_every_selected_cell_with_inline_slice() {
        let (state, c00, c01, c10, c11) = state! {
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
        let initial = with_cell_rect(state, c00, c11);
        let slice = Slice::from_text("hi");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(&mut tr, slice));
        for cid in [c00, c01, c10, c11] {
            assert_eq!(cell_text(&after.doc, cid), "hi");
        }
    }

    #[test]
    fn keeps_table_structure_and_cell_ids() {
        let (state, tbl, c00, c01, c10, c11) = state! {
            doc { root { tbl: table {
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
        let initial = with_cell_rect(state, c00, c11);
        let slice = Slice::from_text("X");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(&mut tr, slice));
        let table = after.doc.node(tbl).expect("table survives fill");
        assert!(matches!(table.node(), Node::Table(_)));
        assert_eq!(table.children().count(), 2);
        for cid in [c00, c01, c10, c11] {
            let cell = after.doc.node(cid).expect("cell id stable");
            assert!(matches!(cell.node(), Node::TableCell(_)));
            assert_eq!(cell.children().count(), 1);
        }
    }

    #[test]
    fn fills_partial_rect_only() {
        let (state, _, c00, c01, c02) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("a") } }
                c01: table_cell { paragraph { text("b") } }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("Z");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(&mut tr, slice));
        assert_eq!(cell_text(&after.doc, c00), "Z");
        assert_eq!(cell_text(&after.doc, c01), "Z");
        assert_eq!(cell_text(&after.doc, c02), "c");
    }
}

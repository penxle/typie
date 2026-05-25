use editor_clipboard::Slice;
use editor_model::{Fragment, PlainNode};
use editor_state::{Position, Selection, enclosing_table_cell};
use editor_transaction::Transaction;

use crate::helpers::{
    find_first_text_position, insert_empty_table_column, insert_empty_table_row, nth_table_cell,
    replace_cell_children, table_col_count, table_row_count,
};
use crate::{CommandError, CommandResult};

/// Cell-wise paste of a table slice. Anchor is the top-left of the target
/// cell-rect, or — for a collapsed caret inside a cell — that cell as a 1×1
/// rect. Missing rows/columns are appended (never inserted in the middle) so
/// cells outside the source rectangle keep their content and position.
pub fn paste_cells_into_cell_rect(tr: &mut Transaction, slice: Slice) -> CommandResult {
    let Some(source_rows) = extract_source_rows(&slice) else {
        return Ok(false);
    };
    let sr = source_rows.len();
    let sc = source_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if sr == 0 || sc == 0 {
        return Ok(false);
    }

    let (table_id, anchor_row, anchor_col, total_r, total_c) = {
        let Some(sel) = tr.selection() else {
            return Ok(false);
        };
        let doc = tr.doc();
        let Some(rs) = sel.resolve(&doc) else {
            return Ok(false);
        };
        let (table, anchor_row, anchor_col) = if let Some(rect) = rs.as_cell_rect() {
            (rect.table, *rect.rows.start(), *rect.cols.start())
        } else {
            let head_id = rs.head().node_id();
            let cell_id = enclosing_table_cell(&doc, head_id);
            let Some(cell_id) = cell_id else {
                return Ok(false);
            };
            let cell = doc
                .node(cell_id)
                .ok_or(CommandError::NodeNotFound(cell_id))?;
            let row = cell.parent().ok_or(CommandError::NoParent(cell_id))?;
            let table = row.parent().ok_or(CommandError::NoParent(row.id()))?;
            let row_idx = row
                .index()
                .ok_or_else(|| CommandError::orphan_child(row.id(), table.id()))?;
            let col_idx = cell
                .index()
                .ok_or_else(|| CommandError::orphan_child(cell_id, row.id()))?;
            (table, row_idx, col_idx)
        };
        let total_r = table.children().count();
        let total_c = table
            .children()
            .next()
            .map(|row| row.children().count())
            .unwrap_or(0);
        if total_r == 0 || total_c == 0 {
            return Ok(false);
        }
        (table.id(), anchor_row, anchor_col, total_r, total_c)
    };

    let extra_r = (anchor_row + sr).saturating_sub(total_r);
    let extra_c = (anchor_col + sc).saturating_sub(total_c);

    tr.batch::<_, CommandError>(|tr| {
        // Columns first so subsequent row insertions inherit the new width.
        for _ in 0..extra_c {
            let col_count = table_col_count(tr, table_id)?;
            insert_empty_table_column(tr, table_id, col_count)?;
        }
        for _ in 0..extra_r {
            let row_count = table_row_count(tr, table_id)?;
            insert_empty_table_row(tr, table_id, row_count)?;
        }

        for sr_i in 0..sr {
            for sc_i in 0..sc {
                let Some(source_cell) = source_rows.get(sr_i).and_then(|r| r.get(sc_i)) else {
                    continue;
                };
                let target_cell_id =
                    nth_table_cell(tr, table_id, anchor_row + sr_i, anchor_col + sc_i)?;
                replace_cell_children(tr, target_cell_id, &source_cell.children)?;
            }
        }
        Ok(())
    })?;

    let anchor_cell_id = nth_table_cell(tr, table_id, anchor_row, anchor_col)?;
    let cursor = find_first_text_position(&tr.doc(), anchor_cell_id)
        .unwrap_or_else(|| Position::new(anchor_cell_id, 0));
    tr.set_selection(Some(Selection::collapsed(cursor)))?;
    Ok(true)
}

fn extract_source_rows(slice: &Slice) -> Option<Vec<Vec<Fragment>>> {
    let table = find_table_fragment(&slice.fragment)?;
    let mut rows: Vec<Vec<Fragment>> = Vec::new();
    for r in &table.children {
        if !matches!(r.node, PlainNode::TableRow(_)) {
            continue;
        }
        let cells: Vec<Fragment> = r
            .children
            .iter()
            .filter(|c| matches!(c.node, PlainNode::TableCell(_)))
            .cloned()
            .collect();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    (!rows.is_empty()).then_some(rows)
}

fn find_table_fragment(frag: &Fragment) -> Option<&Fragment> {
    if matches!(frag.node, PlainNode::Table(_)) {
        return Some(frag);
    }
    for c in &frag.children {
        if let Some(t) = find_table_fragment(c) {
            return Some(t);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::{Node, NodeId};
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

    fn cell_text_at(doc: &editor_model::Doc, table_id: NodeId, row: usize, col: usize) -> String {
        let table = doc.node(table_id).expect("table");
        let row_ref = table.children().nth(row).expect("row");
        let cell = row_ref.children().nth(col).expect("cell");
        cell_text(doc, cell.id())
    }

    fn table_dims(doc: &editor_model::Doc, table_id: NodeId) -> (usize, usize) {
        let table = doc.node(table_id).expect("table");
        let rows = table.children().count();
        let cols = table
            .children()
            .next()
            .map(|r| r.children().count())
            .unwrap_or(0);
        (rows, cols)
    }

    #[test]
    fn returns_false_when_slice_has_no_table() {
        let (state, _, c00) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("x") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c00);
        let slice = Slice::from_text("plain");
        transact_fail!(initial, |tr| paste_cells_into_cell_rect(&mut tr, slice));
    }

    #[test]
    fn returns_false_when_selection_is_not_cell_rect() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 5)
        };
        let source = source_slice_2x2();
        transact_fail!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
    }

    fn source_slice_2x2() -> Slice {
        let (s, _, c00, _, _, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("X") } }
                    c01: table_cell { paragraph { text("Y") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("Z") } }
                    c11: table_cell { paragraph { text("W") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_selection(&s.doc, c00, c11).unwrap();
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        Slice::extract(&s).unwrap()
    }

    fn source_slice_5x1() -> Slice {
        let (s, _, c00, _, _, _, _, _, _, _, c40) = state! {
            doc { root { table {
                tr0: table_row { c00: table_cell { paragraph { text("A") } } }
                tr1: table_row { c10: table_cell { paragraph { text("B") } } }
                tr2: table_row { c20: table_cell { paragraph { text("C") } } }
                tr3: table_row { c30: table_cell { paragraph { text("D") } } }
                tr4: table_row { c40: table_cell { paragraph { text("E") } } }
            } } }
            selection: (c00, 0)
        };
        let sel = cell_rect_selection(&s.doc, c00, c40).unwrap();
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        Slice::extract(&s).unwrap()
    }

    #[test]
    fn equal_dimensions_no_extension_just_overwrites() {
        let (state, tbl, _, c00, c01, _, c10, c11) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c11);
        let source = source_slice_2x2();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (2, 2));
        assert_eq!(cell_text(&after.doc, c00), "X");
        assert_eq!(cell_text(&after.doc, c01), "Y");
        assert_eq!(cell_text(&after.doc, c10), "Z");
        assert_eq!(cell_text(&after.doc, c11), "W");
    }

    #[test]
    fn extends_rows_when_source_is_taller_than_target() {
        let (state, tbl, _, c00, c01, _, _, c11, _, _, c21) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
                tr2: table_row {
                    c20: table_cell { paragraph { text("e") } }
                    c21: table_cell { paragraph { text("f") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c21);
        let source = source_slice_5x1();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (5, 2));
        for (row, ch) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            assert_eq!(cell_text_at(&after.doc, tbl, row, 0), *ch);
        }
        assert_eq!(cell_text(&after.doc, c01), "b");
        assert_eq!(cell_text(&after.doc, c11), "d");
        assert_eq!(cell_text(&after.doc, c21), "f");
        assert_eq!(cell_text_at(&after.doc, tbl, 3, 1), "");
        assert_eq!(cell_text_at(&after.doc, tbl, 4, 1), "");
    }

    #[test]
    fn extends_cols_when_source_is_wider_than_target() {
        let (state, tbl, _, c00) = state! {
            doc { root { tbl: table { tr0: table_row {
                c00: table_cell { paragraph { text("a") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c00);
        let (src, _, sc00, _, sc02) = state! {
            doc { root { table { tr0: table_row {
                sc00: table_cell { paragraph { text("P") } }
                sc01: table_cell { paragraph { text("Q") } }
                sc02: table_cell { paragraph { text("R") } }
            } } } }
            selection: (sc00, 0)
        };
        let sel = cell_rect_selection(&src.doc, sc00, sc02).unwrap();
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (1, 3));
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 0), "P");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 1), "Q");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 2), "R");
    }

    #[test]
    fn extends_both_axes_when_source_overflows_in_both() {
        let (state, tbl, _, c00) = state! {
            doc { root { tbl: table { tr0: table_row {
                c00: table_cell { paragraph { text("x") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c00);

        let (src, _, sc00, _, _, _, _, _, _, _, _, _, sc22) = state! {
            doc { root { table {
                tr0: table_row {
                    sc00: table_cell { paragraph { text("1") } }
                    sc01: table_cell { paragraph { text("2") } }
                    sc02: table_cell { paragraph { text("3") } }
                }
                tr1: table_row {
                    sc10: table_cell { paragraph { text("4") } }
                    sc11: table_cell { paragraph { text("5") } }
                    sc12: table_cell { paragraph { text("6") } }
                }
                tr2: table_row {
                    sc20: table_cell { paragraph { text("7") } }
                    sc21: table_cell { paragraph { text("8") } }
                    sc22: table_cell { paragraph { text("9") } }
                }
            } } }
            selection: (sc00, 0)
        };
        let sel = cell_rect_selection(&src.doc, sc00, sc22).unwrap();
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (3, 3));
        for (i, ch) in ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
            .iter()
            .enumerate()
        {
            assert_eq!(cell_text_at(&after.doc, tbl, i / 3, i % 3), *ch);
        }
    }

    #[test]
    fn extends_from_offset_anchor() {
        let (state, tbl, _, _, _, _, _, _, c11, _, _, _, _, _) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("x") } }
                    c01: table_cell { paragraph { text("x") } }
                    c02: table_cell { paragraph { text("x") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("x") } }
                    c11: table_cell { paragraph { text("x") } }
                    c12: table_cell { paragraph { text("x") } }
                }
                tr2: table_row {
                    c20: table_cell { paragraph { text("x") } }
                    c21: table_cell { paragraph { text("x") } }
                    c22: table_cell { paragraph { text("x") } }
                }
            } } }
            selection: (c11, 0)
        };
        let initial = with_cell_rect(state, c11, c11);

        let (src, _, sc00, _, _, _, _, _, _, _, _, _, sc22) = state! {
            doc { root { table {
                tr0: table_row {
                    sc00: table_cell { paragraph { text("A") } }
                    sc01: table_cell { paragraph { text("B") } }
                    sc02: table_cell { paragraph { text("C") } }
                }
                tr1: table_row {
                    sc10: table_cell { paragraph { text("D") } }
                    sc11: table_cell { paragraph { text("E") } }
                    sc12: table_cell { paragraph { text("F") } }
                }
                tr2: table_row {
                    sc20: table_cell { paragraph { text("G") } }
                    sc21: table_cell { paragraph { text("H") } }
                    sc22: table_cell { paragraph { text("I") } }
                }
            } } }
            selection: (sc00, 0)
        };
        let sel = cell_rect_selection(&src.doc, sc00, sc22).unwrap();
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (4, 4));
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 1), "A");
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 2), "B");
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 3), "C");
        assert_eq!(cell_text_at(&after.doc, tbl, 3, 3), "I");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 0), "x");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 3), "");
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 0), "x");
    }

    #[test]
    fn caret_inside_cell_acts_as_1x1_anchor() {
        let (state, tbl, _, _, _, _, c10, _, c11) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { ct: text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (ct, 1)
        };
        assert!(
            state
                .selection
                .as_ref()
                .and_then(|s| s.resolve(&state.doc))
                .and_then(|rs| rs.as_cell_rect())
                .is_none(),
            "test setup must be a non-cell-rect caret"
        );

        let (src, _, sc00, _, _, _, sc20) = state! {
            doc { root { table {
                tr0: table_row { sc00: table_cell { paragraph { text("A") } } }
                tr1: table_row { sc10: table_cell { paragraph { text("B") } } }
                tr2: table_row { sc20: table_cell { paragraph { text("C") } } }
            } } }
            selection: (sc00, 0)
        };
        let sel = cell_rect_selection(&src.doc, sc00, sc20).unwrap();
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(state, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (4, 2));
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 0), "A");
        assert_eq!(cell_text_at(&after.doc, tbl, 2, 0), "B");
        assert_eq!(cell_text_at(&after.doc, tbl, 3, 0), "C");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 0), "a");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 1), "b");
        assert_eq!(cell_text(&after.doc, c11), "d");
        assert!(after.doc.node(c10).is_some());
    }

    #[test]
    fn never_shrinks_when_target_is_larger_than_source() {
        let (state, tbl, _, c00, _, c02, _, _, _, c12, _, c20, c21, c22) = state! {
            doc { root { tbl: table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("0") } }
                    c01: table_cell { paragraph { text("1") } }
                    c02: table_cell { paragraph { text("2") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("3") } }
                    c11: table_cell { paragraph { text("4") } }
                    c12: table_cell { paragraph { text("5") } }
                }
                tr2: table_row {
                    c20: table_cell { paragraph { text("6") } }
                    c21: table_cell { paragraph { text("7") } }
                    c22: table_cell { paragraph { text("8") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c22);
        let source = source_slice_2x2();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        assert_eq!(table_dims(&after.doc, tbl), (3, 3));
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 0), "X");
        assert_eq!(cell_text_at(&after.doc, tbl, 0, 1), "Y");
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 0), "Z");
        assert_eq!(cell_text_at(&after.doc, tbl, 1, 1), "W");
        assert_eq!(cell_text(&after.doc, c02), "2");
        assert_eq!(cell_text(&after.doc, c12), "5");
        assert_eq!(cell_text(&after.doc, c20), "6");
        assert_eq!(cell_text(&after.doc, c21), "7");
        assert_eq!(cell_text(&after.doc, c22), "8");
    }
}

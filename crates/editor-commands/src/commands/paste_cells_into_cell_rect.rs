use editor_clipboard::Slice;
use editor_model::{Fragment, PlainNode};
use editor_state::enclosing_table_cell;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    find_first_text_position, insert_empty_table_column, insert_empty_table_row, nth_table_cell,
    repair_slice_fragments, replace_cell_children, table_col_count, table_row_count,
};
use crate::{CommandError, CommandResult};

/// Cell-wise paste of a table slice. Anchor is the top-left of the target
/// cell-rect, or — for a collapsed caret inside a cell — that cell as a 1×1
/// rect. Missing rows/columns are appended (never inserted in the middle) so
/// cells outside the source rectangle keep their content and position.
pub fn paste_cells_into_cell_rect(tr: &mut Transaction, mut slice: Slice) -> CommandResult {
    repair_slice_fragments(&mut slice.content);
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
        let view = tr.view();
        let Some(rs) = sel.resolve(&view) else {
            return Ok(false);
        };
        let (table_id, anchor_row, anchor_col) = if let Some(rect) = rs.as_cell_rect() {
            (rect.table.id(), *rect.rows.start(), *rect.cols.start())
        } else {
            let head_id = rs.head().node();
            let Some(cell_id) = enclosing_table_cell(&view, head_id) else {
                return Ok(false);
            };
            let cell = view
                .node(cell_id)
                .ok_or(CommandError::NodeNotFound(cell_id))?;
            let row = cell.parent().ok_or(CommandError::NoParent(cell_id))?;
            let table = row
                .parent()
                .ok_or_else(|| CommandError::NoParent(row.id()))?;
            let row_idx = row
                .index()
                .ok_or_else(|| CommandError::orphan_child(row.id(), table.id()))?;
            let col_idx = cell
                .index()
                .ok_or_else(|| CommandError::orphan_child(cell_id, row.id()))?;
            (table.id(), row_idx, col_idx)
        };
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        let total_r = table.child_blocks().count();
        let total_c = table
            .child_blocks()
            .next()
            .map(|row| row.child_blocks().count())
            .unwrap_or(0);
        if total_r == 0 || total_c == 0 {
            return Ok(false);
        }
        (table_id, anchor_row, anchor_col, total_r, total_c)
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
    let cursor = find_first_text_position(&tr.view(), anchor_cell_id)
        .unwrap_or_else(|| Position::new(anchor_cell_id, 0));
    tr.set_selection(Some(Selection::collapsed(cursor)))?;
    Ok(true)
}

fn extract_source_rows(slice: &Slice) -> Option<Vec<Vec<Fragment>>> {
    let table = slice.content.iter().find_map(find_table_fragment)?;
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
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_state::cell_rect_selection;

    use super::*;
    use crate::test_utils::*;

    fn with_cell_rect(initial: editor_state::State, anchor: Dot, head: Dot) -> editor_state::State {
        let sel = {
            let v = initial.view();
            cell_rect_selection(anchor, head, &v).unwrap()
        };
        editor_state::State {
            selection: Some(sel),
            ..initial
        }
    }

    fn cell_text(view: &editor_model::DocView, id: Dot) -> String {
        let mut s = String::new();
        if let Some(n) = view.node(id) {
            for d in n.descendants() {
                if let editor_model::ChildView::Leaf(l) = d
                    && let Some(c) = l.as_char()
                {
                    s.push(c);
                }
            }
        }
        s
    }

    fn cell_text_at(view: &editor_model::DocView, table_id: Dot, row: usize, col: usize) -> String {
        let table = view.node(table_id).expect("table");
        let row_ref = table.child_blocks().nth(row).expect("row");
        let cell = row_ref.child_blocks().nth(col).expect("cell");
        let id = cell.id();
        cell_text(view, id)
    }

    fn table_dims(view: &editor_model::DocView, table_id: Dot) -> (usize, usize) {
        let table = view.node(table_id).expect("table");
        let rows = table.child_blocks().count();
        let cols = table
            .child_blocks()
            .next()
            .map(|r| r.child_blocks().count())
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
        let (initial, _) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
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
        let sel = {
            let v = s.view();
            cell_rect_selection(c00, c11, &v).unwrap()
        };
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
        let sel = {
            let v = s.view();
            cell_rect_selection(c00, c40, &v).unwrap()
        };
        let s = editor_state::State {
            selection: Some(sel),
            ..s
        };
        Slice::extract(&s).unwrap()
    }

    #[test]
    fn equal_dimensions_no_extension_just_overwrites() {
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
        let source = source_slice_2x2();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (2, 3));
        assert_eq!(cell_text(&v, c00), "X");
        assert_eq!(cell_text(&v, c01), "Y");
        assert_eq!(cell_text(&v, c10), "Z");
        assert_eq!(cell_text(&v, c11), "W");
    }

    #[test]
    fn extends_rows_when_source_is_taller_than_target() {
        let (state, tbl, c00, c01, c11, c21) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
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
        let initial = with_cell_rect(state, c00, c21);
        let source = source_slice_5x1();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (5, 3));
        for (row, ch) in ["A", "B", "C", "D", "E"].iter().enumerate() {
            assert_eq!(cell_text_at(&v, tbl, row, 0), *ch);
        }
        assert_eq!(cell_text(&v, c01), "b");
        assert_eq!(cell_text(&v, c11), "d");
        assert_eq!(cell_text(&v, c21), "f");
        assert_eq!(cell_text_at(&v, tbl, 3, 1), "");
        assert_eq!(cell_text_at(&v, tbl, 4, 1), "");
        assert_eq!(cell_text_at(&v, tbl, 0, 2), "x");
        assert_eq!(cell_text_at(&v, tbl, 1, 2), "y");
        assert_eq!(cell_text_at(&v, tbl, 2, 2), "z");
        assert_eq!(cell_text_at(&v, tbl, 3, 2), "");
        assert_eq!(cell_text_at(&v, tbl, 4, 2), "");
    }

    #[test]
    fn extends_cols_when_source_is_wider_than_target() {
        let (state, tbl, c00, c10) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("b") } }
                }
            } } }
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
        let sel = {
            let v = src.view();
            cell_rect_selection(sc00, sc02, &v).unwrap()
        };
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (2, 3));
        assert_eq!(cell_text_at(&v, tbl, 0, 0), "P");
        assert_eq!(cell_text_at(&v, tbl, 0, 1), "Q");
        assert_eq!(cell_text_at(&v, tbl, 0, 2), "R");
        assert_eq!(cell_text(&v, c10), "b");
        assert_eq!(cell_text_at(&v, tbl, 1, 1), "");
        assert_eq!(cell_text_at(&v, tbl, 1, 2), "");
    }

    #[test]
    fn extends_both_axes_when_source_overflows_in_both() {
        let (state, tbl, c00) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("y") } }
                }
            } } }
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
        let sel = {
            let v = src.view();
            cell_rect_selection(sc00, sc22, &v).unwrap()
        };
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (3, 3));
        for (i, ch) in ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
            .iter()
            .enumerate()
        {
            assert_eq!(cell_text_at(&v, tbl, i / 3, i % 3), *ch);
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
        let sel = {
            let v = src.view();
            cell_rect_selection(sc00, sc22, &v).unwrap()
        };
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (4, 4));
        assert_eq!(cell_text_at(&v, tbl, 1, 1), "A");
        assert_eq!(cell_text_at(&v, tbl, 1, 2), "B");
        assert_eq!(cell_text_at(&v, tbl, 1, 3), "C");
        assert_eq!(cell_text_at(&v, tbl, 3, 3), "I");
        assert_eq!(cell_text_at(&v, tbl, 0, 0), "x");
        assert_eq!(cell_text_at(&v, tbl, 0, 3), "");
        assert_eq!(cell_text_at(&v, tbl, 1, 0), "x");
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
                    c10: table_cell { p1: paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (p1, 1)
        };
        let is_cell_rect = {
            let v = state.view();
            state
                .selection
                .as_ref()
                .and_then(|s| s.resolve(&v))
                .and_then(|rs| rs.as_cell_rect())
                .is_some()
        };
        assert!(!is_cell_rect, "test setup must be a non-cell-rect caret");

        let (src, _, sc00, _, _, _, sc20) = state! {
            doc { root { table {
                tr0: table_row { sc00: table_cell { paragraph { text("A") } } }
                tr1: table_row { sc10: table_cell { paragraph { text("B") } } }
                tr2: table_row { sc20: table_cell { paragraph { text("C") } } }
            } } }
            selection: (sc00, 0)
        };
        let sel = {
            let v = src.view();
            cell_rect_selection(sc00, sc20, &v).unwrap()
        };
        let src = editor_state::State {
            selection: Some(sel),
            ..src
        };
        let source = Slice::extract(&src).unwrap();

        let (after, ..) = transact!(state, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (4, 2));
        assert_eq!(cell_text_at(&v, tbl, 1, 0), "A");
        assert_eq!(cell_text_at(&v, tbl, 2, 0), "B");
        assert_eq!(cell_text_at(&v, tbl, 3, 0), "C");
        assert_eq!(cell_text_at(&v, tbl, 0, 0), "a");
        assert_eq!(cell_text_at(&v, tbl, 0, 1), "b");
        assert_eq!(cell_text(&v, c11), "d");
        assert!(v.node(c10).is_some());
    }

    #[test]
    fn never_shrinks_when_target_is_larger_than_source() {
        let (state, tbl, c00, c02, c03, c12, c13, c20, c21, c22, c23) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("0") } }
                    table_cell { paragraph { text("1") } }
                    c02: table_cell { paragraph { text("2") } }
                    c03: table_cell { paragraph { text("9") } }
                }
                table_row {
                    table_cell { paragraph { text("3") } }
                    table_cell { paragraph { text("4") } }
                    c12: table_cell { paragraph { text("5") } }
                    c13: table_cell { paragraph { text("10") } }
                }
                table_row {
                    c20: table_cell { paragraph { text("6") } }
                    c21: table_cell { paragraph { text("7") } }
                    c22: table_cell { paragraph { text("8") } }
                    c23: table_cell { paragraph { text("11") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c22);
        let source = source_slice_2x2();
        let (after, ..) = transact!(initial, |tr| paste_cells_into_cell_rect(&mut tr, source));
        let v = after.view();
        assert_eq!(table_dims(&v, tbl), (3, 4));
        assert_eq!(cell_text_at(&v, tbl, 0, 0), "X");
        assert_eq!(cell_text_at(&v, tbl, 0, 1), "Y");
        assert_eq!(cell_text_at(&v, tbl, 1, 0), "Z");
        assert_eq!(cell_text_at(&v, tbl, 1, 1), "W");
        assert_eq!(cell_text(&v, c02), "2");
        assert_eq!(cell_text(&v, c12), "5");
        assert_eq!(cell_text(&v, c20), "6");
        assert_eq!(cell_text(&v, c21), "7");
        assert_eq!(cell_text(&v, c22), "8");
        assert_eq!(cell_text(&v, c03), "9");
        assert_eq!(cell_text(&v, c13), "10");
        assert_eq!(cell_text(&v, c23), "11");
    }
}

//! Guards the load-bearing premise of the cell-selection discriminator: the
//! "both endpoints resolve to `TableRow`s in a common `Table`" shape is a
//! free encoding slot. Coverage is the **exact enumerated set** below and
//! nothing more: `select_all`, `select_node_forward`, `select_node_backward`,
//! and `Selection::normalize` (a cross-cell text range). This is NOT a claim
//! about all `editor-commands` producers — other producers/consumers (e.g.
//! `insert_fragment`, `ensure_paragraph`, every command that calls
//! `set_selection`) and all `editor-view` producers (navigation, hit-test)
//! are explicitly out of scope and are named follow-ups. If an enumerated
//! producer ever emits the shape, the corresponding test fails instead of
//! the selection being silently misclassified.

mod lift_differential;
mod lift_golden;

use editor_macros::state;
use editor_state::{Position, Selection};

use crate::commands::{select_all, select_node_backward, select_node_forward};
use crate::test_utils::*;

#[test]
fn select_all_over_a_table_is_not_a_cell_rect() {
    let (initial, _p1) = state! {
        doc { root { table { table_row {
            table_cell { p1: paragraph { text("a") } }
            table_cell { paragraph { text("b") } }
        } } } }
        selection: (p1, 0)
    };
    let (actual, ..) = transact!(initial, |tr| select_all(&mut tr));
    let view = actual.view();
    let rs = actual.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_all must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn select_node_forward_inside_a_cell_is_not_a_cell_rect() {
    let (initial, _p1) = state! {
        doc { root { table { table_row {
            table_cell { p1: paragraph { text("hi") } }
            table_cell { paragraph { text("b") } }
        } } } }
        selection: (p1, 2)
    };
    let mut tr = editor_transaction::Transaction::new(&initial);
    let _ = select_node_forward(&mut tr);
    let (next, ..) = tr.commit();
    let view = next.view();
    let rs = next.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_node_forward must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn select_node_backward_inside_a_cell_is_not_a_cell_rect() {
    let (initial, _p1) = state! {
        doc { root { table { table_row {
            table_cell { paragraph { text("a") } }
            table_cell { p1: paragraph { text("hi") } }
        } } } }
        selection: (p1, 0)
    };
    let mut tr = editor_transaction::Transaction::new(&initial);
    let _ = select_node_backward(&mut tr);
    let (next, ..) = tr.commit();
    let view = next.view();
    let rs = next.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_node_backward must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn cell_rect_contains_subtree_covers_rect_cells_only() {
    let (state, tr1, c00, p00, c01, _tr2) = state! {
        doc { root { table {
            tr1: table_row {
                c00: table_cell { p00: paragraph { text("1") } }
                c01: table_cell { paragraph { text("2") } }
            }
            tr2: table_row {
                table_cell { paragraph { text("3") } }
                table_cell { paragraph { text("4") } }
            }
        } } }
        selection: (tr1, 0, >) -> (tr2, 1, <)
    };
    let view = state.view();
    let rs = state.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(rs.as_cell_rect().is_some(), "selection must be a cell rect");

    assert!(
        rs.contains_subtree(&view.node(c00).unwrap()),
        "rect-column cell is contained"
    );
    assert!(
        rs.contains_subtree(&view.node(p00).unwrap()),
        "paragraph inside a rect cell is contained"
    );
    assert!(
        !rs.contains_subtree(&view.node(c01).unwrap()),
        "cell outside the rect's columns is not contained"
    );
    assert!(
        !rs.contains_subtree(&view.node(tr1).unwrap()),
        "a partially covered row is not contained"
    );
}

#[test]
fn cell_rect_intersects_subtree_follows_rect_not_flat_range() {
    let (state, table, tr1, c01, _tr2) = state! {
        doc { root { table: table {
            tr1: table_row {
                table_cell { paragraph { text("1") } }
                c01: table_cell { paragraph { text("2") } }
            }
            tr2: table_row {
                table_cell { paragraph { text("3") } }
                table_cell { paragraph { text("4") } }
            }
        } } }
        selection: (tr1, 0, >) -> (tr2, 1, <)
    };
    let view = state.view();
    let rs = state.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(rs.as_cell_rect().is_some(), "selection must be a cell rect");

    assert!(
        rs.intersects_subtree(&view.node(table).unwrap()),
        "table holds rect cells"
    );
    assert!(
        rs.intersects_subtree(&view.node(tr1).unwrap()),
        "rect row holds a rect cell"
    );
    assert!(
        !rs.intersects_subtree(&view.node(c01).unwrap()),
        "cell outside the rect's columns does not intersect"
    );
}

#[test]
fn cell_rect_contains_leaf_slot_follows_rect_not_flat_range() {
    let (state, _tr1, p00, p01, _tr2) = state! {
        doc { root { table {
            tr1: table_row {
                table_cell { p00: paragraph { text("1") } }
                table_cell { p01: paragraph { text("2") } }
            }
            tr2: table_row {
                table_cell { paragraph { text("3") } }
                table_cell { paragraph { text("4") } }
            }
        } } }
        selection: (tr1, 0, >) -> (tr2, 1, <)
    };
    let view = state.view();
    let rs = state.selection.as_ref().unwrap().resolve(&view).unwrap();
    assert!(rs.as_cell_rect().is_some(), "selection must be a cell rect");

    assert!(
        rs.contains_leaf_slot(&view.node(p00).unwrap(), 0),
        "leaf inside a rect cell is covered"
    );
    assert!(
        !rs.contains_leaf_slot(&view.node(p01).unwrap(), 0),
        "leaf in a cell outside the rect's columns is not covered"
    );
}

#[test]
fn normalized_cross_cell_text_range_is_not_a_cell_rect() {
    let (state, p1, p2) = state! {
        doc { root { table { table_row {
            table_cell { p1: paragraph { text("hello") } }
            table_cell { p2: paragraph { text("world") } }
        } } } }
        selection: (p1, 0)
    };
    let sel = Selection::new(Position::new(p1, 5), Position::new(p2, 0));
    let view = state.view();
    let rs = sel.resolve(&view).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "normalize must not produce the cell-rect discriminator shape"
    );
}

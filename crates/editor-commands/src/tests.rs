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

use editor_macros::state;
use editor_state::{Position, Selection};

use crate::commands::{select_all, select_node_backward, select_node_forward};
use crate::test_utils::*;

#[test]
fn select_all_over_a_table_is_not_a_cell_rect() {
    let (initial, ..) = state! {
        doc { root { table { table_row {
            table_cell { paragraph { t: text("a") } }
            table_cell { paragraph { text("b") } }
        } } } }
        selection: (t, 0)
    };
    let (actual, ..) = transact!(initial, |tr| select_all(&mut tr));
    let rs = actual
        .selection
        .as_ref()
        .unwrap()
        .resolve(&actual.doc)
        .unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_all must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn select_node_forward_inside_a_cell_is_not_a_cell_rect() {
    let (initial, ..) = state! {
        doc { root { table { table_row {
            table_cell { paragraph { t: text("hi") } }
            table_cell { paragraph { text("b") } }
        } } } }
        selection: (t, 2)
    };
    let mut tr = editor_transaction::Transaction::new(&initial);
    let _ = select_node_forward(&mut tr);
    let (next, ..) = tr.commit();
    let rs = next.selection.as_ref().unwrap().resolve(&next.doc).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_node_forward must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn select_node_backward_inside_a_cell_is_not_a_cell_rect() {
    let (initial, ..) = state! {
        doc { root { table { table_row {
            table_cell { paragraph { text("a") } }
            table_cell { paragraph { t: text("hi") } }
        } } } }
        selection: (t, 0)
    };
    let mut tr = editor_transaction::Transaction::new(&initial);
    let _ = select_node_backward(&mut tr);
    let (next, ..) = tr.commit();
    let rs = next.selection.as_ref().unwrap().resolve(&next.doc).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "select_node_backward must not produce the cell-rect discriminator shape"
    );
}

#[test]
fn normalized_cross_cell_text_range_is_not_a_cell_rect() {
    let (state, ta, tb) = state! {
        doc { root { table { table_row {
            table_cell { paragraph { ta: text("hello") } }
            table_cell { paragraph { tb: text("world") } }
        } } } }
        selection: (ta, 0)
    };
    let sel = Selection::new(Position::new(ta, 5), Position::new(tb, 0));
    let normalized = sel.normalize(&state.doc).expect("range normalizes");
    let rs = normalized.resolve(&state.doc).unwrap();
    assert!(
        rs.as_cell_rect().is_none(),
        "normalize must not produce the cell-rect discriminator shape"
    );
}

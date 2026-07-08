use editor_common::Axis;
use editor_crdt::Dot;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::table_axis_selection;

pub fn select_table_axis(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Option<Axis>,
    index: Option<usize>,
) -> CommandResult {
    let selection = table_axis_selection(tr, table_id, axis, index)?;
    tr.set_selection(Some(selection))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::CellRect;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    fn as_cell_rect<'a>(view: &'a editor_model::DocView<'a>, sel: &Selection) -> CellRect<'a> {
        sel.resolve(view)
            .unwrap()
            .as_cell_rect()
            .expect("expected CellRect selection")
    }

    #[test]
    fn select_full_table() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| select_table_axis(&mut tr, tbl, None, None));
        let root_id = actual.view().root().unwrap().id();
        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position {
                    node: root_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root_id,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_row() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| select_table_axis(
            &mut tr,
            tbl,
            Some(Axis::Horizontal),
            None,
        ));
        let view = actual.view();
        let rect = as_cell_rect(&view, actual.selection.as_ref().unwrap());
        assert!(rect.is_full_row());
    }

    #[test]
    fn select_col() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| select_table_axis(
            &mut tr,
            tbl,
            Some(Axis::Vertical),
            None,
        ));
        let view = actual.view();
        let rect = as_cell_rect(&view, actual.selection.as_ref().unwrap());
        assert!(rect.is_full_column());
    }
}

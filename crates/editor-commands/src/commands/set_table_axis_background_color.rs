use editor_common::Axis;
use editor_crdt::Dot;
use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_table_axis_background_color(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Axis,
    index: usize,
    color: Option<String>,
) -> CommandResult {
    let cell_ids: Vec<Dot> = {
        let view = tr.view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;

        match axis {
            Axis::Horizontal => {
                let row = table
                    .child_blocks()
                    .nth(index)
                    .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
                row.child_blocks().map(|cell| cell.id()).collect()
            }
            Axis::Vertical => table
                .child_blocks()
                .map(|row| -> Result<Dot, CommandError> {
                    let cell = row
                        .child_blocks()
                        .nth(index)
                        .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?;
                    Ok(cell.id())
                })
                .collect::<Result<Vec<_>, _>>()?,
        }
    };

    for cell_id in cell_ids {
        match &color {
            Some(value) => tr.add_modifier(
                cell_id,
                Modifier::BackgroundColor {
                    value: value.clone(),
                },
            )?,
            None => tr.remove_modifier(
                cell_id,
                Modifier::BackgroundColor {
                    value: String::new(),
                },
            )?,
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    fn cell_bg(view: &editor_model::DocView, id: Dot) -> Option<String> {
        match view
            .node(id)?
            .block_modifier(editor_model::ModifierType::BackgroundColor)?
        {
            Modifier::BackgroundColor { value } => Some(value.clone()),
            _ => None,
        }
    }

    #[test]
    fn sets_row_background_color() {
        let (initial, tbl, r0c0, r0c1, r1c0, r1c1, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        r1c0: table_cell { paragraph { text("C") } }
                        r1c1: table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_axis_background_color(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            Some("red".to_string())
        ));
        let view = actual.view();
        assert_eq!(cell_bg(&view, r0c0), Some("red".to_string()));
        assert_eq!(cell_bg(&view, r0c1), Some("red".to_string()));
        assert_eq!(cell_bg(&view, r1c0), None);
        assert_eq!(cell_bg(&view, r1c1), None);
    }

    #[test]
    fn sets_col_background_color() {
        let (initial, tbl, r0c0, r0c1, r1c0, r1c1, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        r1c0: table_cell { paragraph { text("C") } }
                        r1c1: table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_axis_background_color(
            &mut tr,
            tbl,
            Axis::Vertical,
            1,
            Some("blue".to_string())
        ));
        let view = actual.view();
        assert_eq!(cell_bg(&view, r0c0), None);
        assert_eq!(cell_bg(&view, r0c1), Some("blue".to_string()));
        assert_eq!(cell_bg(&view, r1c0), None);
        assert_eq!(cell_bg(&view, r1c1), Some("blue".to_string()));
    }

    #[test]
    fn clears_background_color_with_none() {
        let (initial, tbl, r0c0, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (with_color, ..) = transact!(initial, |tr| set_table_axis_background_color(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            Some("green".to_string())
        ));
        let (cleared, ..) = transact!(with_color, |tr| set_table_axis_background_color(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            None
        ));
        let view = cleared.view();
        assert_eq!(cell_bg(&view, r0c0), None);
    }
}

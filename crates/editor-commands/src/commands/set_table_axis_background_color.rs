use editor_common::Axis;
use editor_model::{Modifier, NodeId};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_table_axis_background_color(
    tr: &mut Transaction,
    table_id: NodeId,
    axis: Axis,
    index: usize,
    color: Option<String>,
) -> CommandResult {
    let cell_ids: Vec<NodeId> = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;

        match axis {
            Axis::Horizontal => {
                let row = table
                    .children()
                    .nth(index)
                    .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
                row.children().map(|cell| cell.id()).collect()
            }
            Axis::Vertical => table
                .children()
                .map(|row| -> Result<NodeId, CommandError> {
                    let cell = row
                        .children()
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
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    fn cell_bg(doc: &editor_model::Doc, cell_id: NodeId) -> Option<String> {
        doc.node(cell_id)?
            .explicit_modifiers()
            .find_map(|m| match m {
                Modifier::BackgroundColor { value } => Some(value.clone()),
                _ => None,
            })
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
        let doc = &actual.doc;
        assert_eq!(cell_bg(doc, r0c0), Some("red".to_string()));
        assert_eq!(cell_bg(doc, r0c1), Some("red".to_string()));
        assert_eq!(cell_bg(doc, r1c0), None);
        assert_eq!(cell_bg(doc, r1c1), None);
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
        let doc = &actual.doc;
        assert_eq!(cell_bg(doc, r0c0), None);
        assert_eq!(cell_bg(doc, r0c1), Some("blue".to_string()));
        assert_eq!(cell_bg(doc, r1c0), None);
        assert_eq!(cell_bg(doc, r1c1), Some("blue".to_string()));
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
        assert_eq!(cell_bg(&cleared.doc, r0c0), None);
    }
}

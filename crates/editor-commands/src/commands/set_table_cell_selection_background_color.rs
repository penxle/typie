use editor_crdt::Dot;
use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn set_table_cell_selection_background_color(
    tr: &mut Transaction,
    table_id: Dot,
    color: Option<String>,
) -> CommandResult {
    let cell_ids: Vec<Dot> = {
        let view = tr.view();
        let sel = tr
            .selection()
            .ok_or_else(|| CommandError::Corrupted("no selection".into()))?;
        let resolved = sel
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("selection could not be resolved".into()))?;
        let rect = resolved
            .as_cell_rect()
            .ok_or_else(|| CommandError::Corrupted("selection is not a cell rect".into()))?;
        if rect.table.id() != table_id {
            return Err(CommandError::Corrupted(
                "selection is not in this table".into(),
            ));
        }
        rect.cells().into_iter().map(|cell| cell.id()).collect()
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
    use editor_state::cell_rect_selection;

    use super::*;
    use crate::test_utils::*;

    fn with_cell_rect(initial: editor_state::State, anchor: Dot, head: Dot) -> editor_state::State {
        let sel = {
            let view = initial.view();
            cell_rect_selection(anchor, head, &view)
        }
        .unwrap();
        editor_state::State {
            selection: Some(sel),
            ..initial
        }
    }

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
    fn sets_background_on_selected_cells() {
        let (initial, tbl, r0c0, r0c1, r1c0, r1c1, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                        table_cell { paragraph { text("X") } }
                    }
                    table_row {
                        r1c0: table_cell { paragraph { text("C") } }
                        r1c1: table_cell { paragraph { text("D") } }
                        table_cell { paragraph { text("Y") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let initial = with_cell_rect(initial, r0c0, r1c1);
        let (actual, ..) = transact!(initial, |tr| set_table_cell_selection_background_color(
            &mut tr,
            tbl,
            Some("red".to_string())
        ));
        let view = actual.view();
        assert_eq!(cell_bg(&view, r0c0), Some("red".to_string()));
        assert_eq!(cell_bg(&view, r0c1), Some("red".to_string()));
        assert_eq!(cell_bg(&view, r1c0), Some("red".to_string()));
        assert_eq!(cell_bg(&view, r1c1), Some("red".to_string()));
    }

    #[test]
    fn clears_background_on_selected_cells() {
        let (initial, tbl, r0c0, _r0c1, r1c0, _r1c1, ..) = state! {
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
        let initial = with_cell_rect(initial, r0c0, r1c0);
        let (actual, ..) = transact!(initial, |tr| set_table_cell_selection_background_color(
            &mut tr, tbl, None
        ));
        let view = actual.view();
        assert_eq!(cell_bg(&view, r0c0), None);
        assert_eq!(cell_bg(&view, r1c0), None);
    }
}

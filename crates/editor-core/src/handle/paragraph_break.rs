use editor_commands::{self as commands, CommandResult};
use editor_transaction::Transaction;

pub(super) fn apply_paragraph_break(tr: &mut Transaction) -> CommandResult {
    let applied = commands::chain!(
        tr,
        commands::optional!(commands::materialize_synthetic_selection_blocks()),
        |tr| commands::first!(
            tr,
            commands::materialize_gap_paragraph(),
            commands::insert_paragraph_after_unit_selection(),
            |tr| {
                let selection_was_range = tr
                    .selection()
                    .is_some_and(|selection| !selection.is_collapsed());
                commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::first!(
                        tr,
                        |tr| apply_list_paragraph_break(tr, selection_was_range),
                        commands::lift_last_paragraph(),
                        commands::split_paragraph(),
                    ),
                )
            },
        ),
    )?;
    if applied {
        tr.clear_pending_format()?;
    }
    Ok(applied)
}

pub(super) fn apply_list_paragraph_break(
    tr: &mut Transaction,
    selection_was_range: bool,
) -> CommandResult {
    if selection_was_range {
        commands::split_list_item(tr)
    } else {
        commands::first!(
            tr,
            commands::lift_empty_list_item(),
            commands::lift_trailing_empty_list_item_paragraph(),
            commands::split_list_item(),
        )
    }
}

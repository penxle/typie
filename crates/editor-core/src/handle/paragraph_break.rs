use editor_commands::{self as commands, CommandResult};
use editor_transaction::Transaction;

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

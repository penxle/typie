use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::clear_all_modifiers_range;

pub fn clear_all_modifiers_in_selection(
    tr: &mut Transaction,
    selection: Selection,
) -> CommandResult {
    clear_all_modifiers_range(tr, selection)
}

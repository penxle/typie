use editor_model::ModifierType;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::toggle_modifier_range;

pub fn toggle_modifier_in_selection(
    tr: &mut Transaction,
    selection: Selection,
    modifier_type: ModifierType,
) -> CommandResult {
    toggle_modifier_range(tr, selection, modifier_type)
}

use editor_model::{Modifier, ModifierType};
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{edit_modifier_range, validate_edit};

pub fn edit_modifier_in_selection(
    tr: &mut Transaction,
    selection: Selection,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    if !validate_edit(modifier_type, &modifier)? {
        return Ok(false);
    }
    edit_modifier_range(tr, selection, modifier_type, modifier)
}

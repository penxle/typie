use editor_model::Modifier;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::helpers::{is_unit_variant, set_modifier_range_text};
use crate::{CommandError, CommandResult};

pub fn set_modifier_text_in_selection(
    tr: &mut Transaction,
    selection: Selection,
    modifier: Modifier,
) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }
    if !modifier.is_valid() {
        return Ok(false);
    }
    if !modifier.as_type().is_text_applicable() {
        return Ok(false);
    }
    set_modifier_range_text(tr, selection, &modifier)
}

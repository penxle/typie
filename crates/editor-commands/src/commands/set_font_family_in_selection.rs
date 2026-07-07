use editor_model::Modifier;
use editor_resource::Resource;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::set_font_family_range;

pub fn set_font_family_in_selection(
    tr: &mut Transaction,
    selection: Selection,
    value: String,
    resource: &Resource,
) -> CommandResult {
    let Some(weights) = resource
        .font_registry
        .weights(&value)
        .filter(|w| !w.is_empty())
    else {
        return Ok(false);
    };
    set_font_family_range(tr, selection, Modifier::FontFamily { value }, weights)
}

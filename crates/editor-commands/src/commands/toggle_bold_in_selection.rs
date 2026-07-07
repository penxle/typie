use editor_resource::Resource;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::toggle_bold_range;

pub fn toggle_bold_in_selection(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    toggle_bold_range(tr, selection, resource)
}

use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;

pub(crate) fn set_selection_if_changed(
    tr: &mut Transaction,
    selection: Option<Selection>,
) -> CommandResult {
    let Some(selection) = selection else {
        return Ok(false);
    };
    if tr.selection() == Some(selection) {
        return Ok(true);
    }
    tr.set_selection(Some(selection))?;
    Ok(true)
}

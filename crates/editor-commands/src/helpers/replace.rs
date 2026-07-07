use std::collections::BTreeMap;

use editor_model::{Modifier, ModifierType};
use editor_state::{Selection, apply_pending, replacement_paint};
use editor_transaction::Transaction;

use super::ensure_paragraph;
use crate::CommandResult;
use crate::helpers::{
    consume_pending_modifiers, delete_selection_range, delete_selection_range_no_carry,
    insert_text_at_caret,
};

pub(crate) fn replace_range_with_text(
    tr: &mut Transaction,
    selection: Selection,
    replacement: &str,
    paint_override: Option<Vec<Modifier>>,
) -> CommandResult {
    if replacement.contains(['\n', '\r']) {
        return Ok(false);
    }

    let base_paint = paint_override
        .or_else(|| replacement_paint(&tr.state().projected, selection.anchor, selection.head));

    tr.set_selection(Some(selection))?;
    ensure_paragraph(tr)?;

    let current = tr.selection();

    if replacement.is_empty() {
        if let Some(sel) = current
            && sel.anchor != sel.head
        {
            delete_selection_range(tr, sel)?;
        }
        return Ok(true);
    }

    if let Some(sel) = current
        && sel.anchor != sel.head
    {
        delete_selection_range_no_carry(tr, sel)?;
    }

    let changed = match &base_paint {
        Some(paint) => {
            let mut effective: BTreeMap<ModifierType, Modifier> =
                paint.iter().map(|m| (m.as_type(), m.clone())).collect();
            apply_pending(&mut effective, tr.pending_modifiers());
            let effective: Vec<Modifier> = effective.into_values().collect();
            insert_text_at_caret(tr, replacement, Some(&effective))?
        }
        None => insert_text_at_caret(tr, replacement, None)?,
    };

    if changed {
        consume_pending_modifiers(tr)?;
    }

    Ok(true)
}

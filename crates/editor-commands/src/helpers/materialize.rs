use editor_crdt::Dot;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) fn materialize_target(tr: &mut Transaction, target: Dot) -> Result<Dot, CommandError> {
    let before = tr.selection();
    let remapped = editor_transaction::materialize_repair_target(tr, target)?;
    if remapped == target {
        return Ok(remapped);
    }
    if let Some(before) = before {
        let selection = {
            let view = tr.view();
            let remap = |pos: Position| {
                let node = if pos.node == target {
                    remapped
                } else {
                    view.alias_classes().resolve_with(pos.node, |d| {
                        view.node(d).is_some() || view.leaf(d).is_some()
                    })
                };
                Position { node, ..pos }
            };
            Selection::new(remap(before.anchor), remap(before.head))
        };
        tr.set_selection(Some(selection))?;
    }
    Ok(remapped)
}

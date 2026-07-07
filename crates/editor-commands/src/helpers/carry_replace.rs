use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, Modifier, ModifierType, NodeView, OwnModifier};
use editor_state::State;
use editor_transaction::Transaction;

use super::find_ancestor_textblock;
use crate::CommandError;

pub(crate) struct CapturedCarry {
    had_charlike: bool,
    paint: Vec<Modifier>,
}

pub(crate) fn capture_first_charlike_paint(state: &State, block_id: Dot) -> CapturedCarry {
    let view = state.view();
    let Some(block) = view.node(block_id) else {
        return CapturedCarry {
            had_charlike: false,
            paint: Vec::new(),
        };
    };
    if !block.spec().is_textblock() {
        return CapturedCarry {
            had_charlike: false,
            paint: Vec::new(),
        };
    }
    match first_charlike_slot(&block) {
        Some(slot) => {
            let paint = block
                .leaf_state_at(slot)
                .map(|s| carry_paint(s.own))
                .unwrap_or_default();
            CapturedCarry {
                had_charlike: true,
                paint,
            }
        }
        None => CapturedCarry {
            had_charlike: false,
            paint: Vec::new(),
        },
    }
}

pub(crate) fn apply_carry_from_selection(
    tr: &mut Transaction,
    captured: &CapturedCarry,
) -> Result<(), CommandError> {
    let target = {
        let Some(selection) = tr.selection() else {
            return Ok(());
        };
        let view = tr.state().view();
        find_ancestor_textblock(&view, selection.head.node)
    };
    match target {
        Some(target) => apply_carry_on_emptied(tr, target, captured),
        None => Ok(()),
    }
}

pub(crate) fn apply_carry_on_emptied(
    tr: &mut Transaction,
    target_block: Dot,
    captured: &CapturedCarry,
) -> Result<(), CommandError> {
    if !captured.had_charlike || target_block.as_op_dot().is_none() {
        return Ok(());
    }
    {
        let view = tr.state().view();
        let Some(block) = view.node(target_block) else {
            return Ok(());
        };
        if !block.spec().is_textblock() || first_charlike_slot(&block).is_some() {
            return Ok(());
        }
    }
    tr.replace_carry(target_block, captured.paint.clone())?;
    Ok(())
}

pub(crate) fn cell_first_charlike_block(view: &DocView, cell_id: Dot) -> Option<Dot> {
    let cell = view.node(cell_id)?;
    cell.descendants().find_map(|c| match c {
        ChildView::Leaf(l) if l.is_charlike() => l.parent().map(|p| p.id()),
        _ => None,
    })
}

fn first_charlike_slot(block: &NodeView) -> Option<usize> {
    block.children().enumerate().find_map(|(i, c)| match c {
        ChildView::Leaf(l) if l.is_charlike() => Some(i),
        _ => None,
    })
}

fn carry_paint(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    own.iter()
        .filter(|(t, _)| t.is_carry_kind())
        .map(|(_, o)| o.value.clone())
        .collect()
}

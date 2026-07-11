use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{ChildView, LeafStateRef, Modifier, ModifierType, NodeView};

use crate::pending_modifier::PendingModifier;
use crate::projected_state::ProjectedState;

pub fn apply_pending(out: &mut BTreeMap<ModifierType, Modifier>, pending: &[PendingModifier]) {
    for pm in pending {
        match pm {
            PendingModifier::Set { modifier } => {
                out.insert(modifier.as_type(), modifier.clone());
            }
            PendingModifier::Unset { ty } => {
                out.remove(ty);
            }
        }
    }
}

fn nearest_charlike_left<'a>(host: &NodeView<'a>, offset: usize) -> Option<LeafStateRef<'a>> {
    let mut i = offset;
    while i > 0 {
        i -= 1;
        if let Some(ChildView::Leaf(l)) = host.child_at(i)
            && l.is_charlike()
        {
            return host.leaf_state_at(i);
        }
    }
    None
}

fn first_charlike_right<'a>(host: &NodeView<'a>, offset: usize) -> Option<LeafStateRef<'a>> {
    let mut i = offset;
    loop {
        match host.child_at(i) {
            None => return None,
            Some(ChildView::Leaf(l)) if l.is_charlike() => return host.leaf_state_at(i),
            Some(_) => i += 1,
        }
    }
}

pub fn continuation_from_neighbors(
    state: &ProjectedState,
    block: Dot,
    offset: usize,
) -> Option<BTreeMap<ModifierType, Modifier>> {
    let view = state.view();
    let host = view.node(block)?;
    let left = nearest_charlike_left(&host, offset);
    let right = first_charlike_right(&host, offset);
    if left.is_none() && right.is_none() {
        return None;
    }

    let mut out: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
    if let Some(source) = left.or(right) {
        for (ty, om) in source.own {
            if ty.is_carry_kind() {
                out.insert(*ty, om.value.clone());
            }
        }
    }
    if let (Some(l), Some(r)) = (left, right) {
        for ty in [ModifierType::Link, ModifierType::Ruby] {
            if let (Some(lo), Some(ro)) = (l.own.get(&ty), r.own.get(&ty))
                && lo.value == ro.value
            {
                out.insert(ty, lo.value.clone());
            }
        }
    }
    Some(out)
}

pub fn continuation_at(
    state: &ProjectedState,
    block: Dot,
    offset: usize,
) -> BTreeMap<ModifierType, Modifier> {
    match continuation_from_neighbors(state, block, offset) {
        Some(map) => map,
        None => state.carry_modifiers(block),
    }
}

/// The value a collapsed caret at `(block, offset)` surfaces for `ty` once its
/// explicit override is ignored, and whether such an override is present. Reads
/// the same source the caret paint does: the nearest charlike neighbor (left,
/// then right), falling back to the block's carry.
pub fn caret_provided_and_override(
    state: &ProjectedState,
    block: Dot,
    offset: usize,
    ty: ModifierType,
) -> (Option<Modifier>, bool) {
    let view = state.view();
    let Some(host) = view.node(block) else {
        return (None, false);
    };
    let block_eff = host.effective().get(&ty).cloned();
    match nearest_charlike_left(&host, offset).or_else(|| first_charlike_right(&host, offset)) {
        Some(st) => match st.own.get(&ty) {
            Some(_) => (block_eff, true),
            None => (st.eff.get(&ty).cloned().or(block_eff), false),
        },
        None => (block_eff, state.carry_modifiers(block).contains_key(&ty)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_pending_set_and_unset() {
        let mut m = BTreeMap::new();
        m.insert(ModifierType::Bold, Modifier::Bold);
        apply_pending(
            &mut m,
            &[
                PendingModifier::Set {
                    modifier: Modifier::Italic,
                },
                PendingModifier::Unset {
                    ty: ModifierType::Bold,
                },
            ],
        );
        assert!(m.contains_key(&ModifierType::Italic));
        assert!(!m.contains_key(&ModifierType::Bold));
    }
}

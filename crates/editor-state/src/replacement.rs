use editor_model::{ChildView, Modifier};

use crate::position::{Position, inline_leaf_dots_in_range};
use crate::projected_state::ProjectedState;
use crate::selection::Selection;

pub fn replacement_paint(
    state: &ProjectedState,
    from: Position,
    to: Position,
) -> Option<Vec<Modifier>> {
    let view = state.view();
    let resolved = Selection::new(from, to).resolve(&view)?;
    let lo = resolved.from().position();
    let hi = resolved.to().position();

    if lo.node == hi.node {
        let block = view.node(lo.node)?;
        for slot in lo.offset..hi.offset {
            if let Some(ChildView::Leaf(l)) = block.child_at(slot)
                && l.is_charlike()
            {
                return Some(
                    block
                        .leaf_state_at(slot)
                        .map(|s| s.own_modifiers())
                        .unwrap_or_default(),
                );
            }
        }
        return None;
    }

    for dot in inline_leaf_dots_in_range(&view, &lo, &hi) {
        let Some(l) = view.leaf(dot) else { continue };
        if l.is_charlike() {
            return Some(
                view.leaf_state_by_dot_slow(dot)
                    .map(|s| s.own_modifiers())
                    .unwrap_or_default(),
            );
        }
    }

    None
}

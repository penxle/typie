use editor_model::{InlineKind, Modifier, ModifierType};

use crate::state::State;
use crate::{Position, Selection};

/// Selection covering the whole inline span of `modifier_type` containing `pos`.
///
/// Walks the inline leaves of the block at `pos`, finds the modifier value under
/// the caret, and extends over the contiguous run of leaves sharing that value.
/// Returns `None` when `pos` is not inside such a span.
pub fn resolve_modifier_span_selection(
    state: &State,
    pos: &Position,
    modifier_type: ModifierType,
) -> Option<Selection> {
    let view = state.view();
    let block = view.node(pos.node)?;

    let chars: Vec<(usize, Option<Modifier>)> = block
        .inline()
        .into_iter()
        .filter_map(|it| match it.kind {
            InlineKind::Char { char_index, .. } => {
                Some((char_index, it.effective.get(&modifier_type).cloned()))
            }
            _ => None,
        })
        .collect();
    if chars.is_empty() {
        return None;
    }

    let at = |idx: usize| {
        chars
            .iter()
            .find(|(ci, _)| *ci == idx)
            .and_then(|(_, v)| v.clone())
    };

    // The caret at offset O sits between char O-1 and char O. Prefer the value on
    // the char to the right; fall back to the char on the left.
    let (anchor, value) = if let Some(v) = at(pos.offset) {
        (pos.offset, v)
    } else if let Some(v) = pos.offset.checked_sub(1).and_then(at) {
        (pos.offset - 1, v)
    } else {
        return None;
    };

    let mut start = anchor;
    while start > 0 && at(start - 1).as_ref() == Some(&value) {
        start -= 1;
    }
    let mut end = anchor;
    while at(end + 1).as_ref() == Some(&value) {
        end += 1;
    }

    Some(Selection::new(
        Position::new(pos.node, start),
        Position::new(pos.node, end + 1),
    ))
}

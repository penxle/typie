use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, Expand, LeafView, Marker, Modifier, ModifierType, NodeType, NodeView, OwnModifier,
    Schema,
};
use editor_state::State;
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) struct CapturedFirstTextMarker {
    paragraph_id: Dot,
    had_text: bool,
    first_text_carryable: Vec<Modifier>,
}

pub(crate) fn capture_first_text_marker(
    state: &State,
    paragraph_id: Dot,
) -> Option<CapturedFirstTextMarker> {
    let view = state.view();
    let paragraph = view.node(paragraph_id)?;
    if paragraph.node_type() != NodeType::Paragraph {
        return None;
    }
    let first_text = first_char_leaf(&paragraph);
    let (had_text, first_text_carryable) = match first_text {
        Some((slot, _leaf)) => {
            let carryable = paragraph
                .leaf_state_at(slot)
                .map(|s| collect_carryable(s.own))
                .unwrap_or_default();
            (true, carryable)
        }
        None => (false, Vec::new()),
    };
    Some(CapturedFirstTextMarker {
        paragraph_id,
        had_text,
        first_text_carryable,
    })
}

pub(crate) fn apply_first_text_marker_lift(
    tr: &mut Transaction,
    captured: &CapturedFirstTextMarker,
) -> Result<(), CommandError> {
    if !captured.had_text || captured.first_text_carryable.is_empty() {
        return Ok(());
    }
    let still_has_text = {
        let view = tr.state().view();
        let Some(paragraph) = view.node(captured.paragraph_id) else {
            return Ok(());
        };
        if paragraph.node_type() != NodeType::Paragraph {
            return Ok(());
        }
        first_char_leaf(&paragraph).is_some()
    };
    if still_has_text {
        return Ok(());
    }
    let marker = Marker {
        modifiers: captured.first_text_carryable.clone(),
    };
    if !marker.is_empty() {
        tr.set_marker(captured.paragraph_id, Some(marker))?;
    }
    Ok(())
}

fn first_char_leaf<'a>(paragraph: &NodeView<'a>) -> Option<(usize, LeafView<'a>)> {
    paragraph.children().enumerate().find_map(|(i, c)| match c {
        ChildView::Leaf(l) if l.as_char().is_some() => Some((i, l)),
        _ => None,
    })
}

fn collect_carryable(own: &BTreeMap<ModifierType, OwnModifier>) -> Vec<Modifier> {
    own.iter()
        .filter(|(t, _)| {
            matches!(
                Schema::modifier_spec(**t).expand,
                Expand::After | Expand::Both
            )
        })
        .map(|(_, o)| o.value.clone())
        .collect()
}

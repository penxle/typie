use editor_crdt::Dot;
use editor_model::{Expand, LeafView, Marker, Modifier, NodeType, Schema};
use editor_state::State;
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) struct CapturedFirstTextMarker {
    paragraph_id: Dot,
    had_text: bool,
    first_text_carryable: Vec<Modifier>,
    first_text_style: Option<String>,
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
    let (had_text, first_text_carryable, first_text_style) = match first_text {
        Some(leaf) => {
            let style = state.projected.node_styles().value_of(leaf.dot());
            (true, collect_carryable(&leaf), style)
        }
        None => (false, Vec::new(), None),
    };
    Some(CapturedFirstTextMarker {
        paragraph_id,
        had_text,
        first_text_carryable,
        first_text_style,
    })
}

pub(crate) fn apply_first_text_marker_lift(
    tr: &mut Transaction,
    captured: &CapturedFirstTextMarker,
) -> Result<(), CommandError> {
    if !captured.had_text
        || (captured.first_text_carryable.is_empty() && captured.first_text_style.is_none())
    {
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
        style: captured.first_text_style.clone(),
    };
    if !marker.is_empty() {
        tr.set_marker(captured.paragraph_id, Some(marker))?;
    }
    Ok(())
}

fn first_char_leaf<'a>(paragraph: &editor_model::NodeView<'a>) -> Option<LeafView<'a>> {
    paragraph.children().find_map(|c| match c {
        editor_model::ChildView::Leaf(l) if l.as_char().is_some() => Some(l),
        _ => None,
    })
}

fn collect_carryable(leaf: &LeafView) -> Vec<Modifier> {
    leaf.own_modifiers()
        .iter()
        .filter(|(_, o)| !o.from_style)
        .filter(|(t, _)| {
            matches!(
                Schema::modifier_spec(**t).expand,
                Expand::After | Expand::Both
            )
        })
        .map(|(_, o)| o.value.clone())
        .collect()
}

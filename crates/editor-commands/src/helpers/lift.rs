use editor_model::{Node, NodeId, NodeType};
use editor_schema::NodeSpecExt;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, dissolve, prune};

use crate::{CommandError, CommandResult};

pub(crate) enum LiftDirection {
    Front,
    End,
}

pub(crate) fn lift(
    tr: &mut Transaction,
    paragraph_id: NodeId,
    direction: LiftDirection,
) -> CommandResult {
    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let wrapper = match paragraph.parent() {
        Some(parent) if !matches!(parent.node(), Node::Root(_)) => parent,
        _ => return Ok(false),
    };

    if wrapper.spec().isolating {
        return Ok(false);
    }

    let wrapper_id = wrapper.id();
    let wrapper_parent = wrapper.parent().ok_or(CommandError::NoParent(wrapper_id))?;
    let wrapper_parent_id = wrapper_parent.id();

    let wrapper_index = wrapper
        .index()
        .ok_or(CommandError::Corrupted("wrapper has no index".into()))?;

    let target_index = match direction {
        LiftDirection::Front => wrapper_index,
        LiftDirection::End => wrapper_index + 1,
    };

    let mut children_types: Vec<NodeType> =
        wrapper_parent.children().map(|c| c.as_type()).collect();
    children_types.insert(target_index, NodeType::Paragraph);
    if !wrapper_parent
        .spec()
        .content
        .matches_sequence(&children_types)
    {
        return Ok(false);
    }

    tr.batch::<_, CommandError>(|tr| {
        tr.move_node(paragraph_id, wrapper_parent_id, target_index)?;

        let doc = tr.doc();
        if let Some(wrapper) = doc.node(wrapper_id) {
            let remaining: Vec<NodeType> = wrapper.children().map(|c| c.as_type()).collect();

            if wrapper.entry().children.is_empty() {
                tr.apply_steps(prune(&wrapper))?;
            } else if !wrapper.spec().content.matches_sequence(&remaining) {
                tr.apply_steps(dissolve(&wrapper))?;
            }
        }
        Ok(())
    })?;

    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let new_selection = match paragraph.first_child() {
        Some(child) if matches!(child.node(), Node::Text(_)) => Selection::collapsed(Position {
            node_id: child.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        }),
        _ => Selection::collapsed(Position {
            node_id: paragraph_id,
            offset: 0,
            affinity: Affinity::Downstream,
        }),
    };
    tr.set_selection(new_selection)?;

    Ok(true)
}

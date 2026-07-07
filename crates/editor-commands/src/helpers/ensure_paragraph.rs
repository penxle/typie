use editor_model::{ChildView, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{is_block_container, remove_child_at};
use crate::{CommandError, CommandResult};

pub(crate) fn ensure_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        return Ok(false);
    }

    let (parent_id, from_offset, remove_count) = {
        let view = tr.state().view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let from = resolved.from();
        let to = resolved.to();

        let from_node = from.node();
        if from_node != to.node() {
            return Ok(false);
        }

        let parent = view
            .node(from_node)
            .ok_or(CommandError::NodeNotFound(from_node))?;

        if !is_block_container(&parent) {
            return Ok(false);
        }

        if !parent.spec().content.matches(NodeType::Paragraph) {
            return Ok(false);
        }

        let from_offset = from.offset();
        let to_offset = to.offset();
        let remove_count = to_offset - from_offset;

        (from_node, from_offset, remove_count)
    };

    tr.batch::<_, CommandError>(|tr| {
        for index in (from_offset..from_offset + remove_count).rev() {
            remove_child_at(tr, parent_id, index)?;
        }

        let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
        tr.insert_subtree(parent_id, from_offset, subtree)?;

        let steps = {
            let view = tr.state().view();
            let parent = view
                .node(parent_id)
                .ok_or(CommandError::NodeNotFound(parent_id))?;
            fulfill(&parent)
        };
        tr.apply_steps(steps)?;
        Ok(())
    })?;

    let new_para_id = {
        let view = tr.state().view();
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        match parent.child_at(from_offset) {
            Some(ChildView::Block(b)) => b.id(),
            _ => return Err(CommandError::Corrupted("inserted paragraph missing".into())),
        }
    };

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_para_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

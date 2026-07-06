use editor_crdt::Dot;
use editor_model::NodeType;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, dissolve};

use crate::helpers::{child_elem_id, child_node_type};
use crate::{CommandError, CommandResult};

pub(crate) enum LiftDirection {
    Front,
    End,
}

pub(crate) fn lift(
    tr: &mut Transaction,
    paragraph_id: Dot,
    direction: LiftDirection,
) -> CommandResult {
    let (wrapper_id, wrapper_parent_id, target_index) = {
        let view = tr.state().view();
        let paragraph = view
            .node(paragraph_id)
            .ok_or(CommandError::NodeNotFound(paragraph_id))?;

        let wrapper = match paragraph.parent() {
            Some(parent) if parent.node_type() != NodeType::Root => parent,
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
            .ok_or_else(|| CommandError::Corrupted("wrapper has no index".into()))?;

        let target_index = match direction {
            LiftDirection::Front => wrapper_index,
            LiftDirection::End => wrapper_index + 1,
        };

        let mut children_types: Vec<NodeType> = wrapper_parent
            .children()
            .map(|c| child_node_type(&c))
            .collect();
        children_types.insert(target_index, NodeType::Paragraph);
        if !wrapper_parent
            .spec()
            .content
            .matches_sequence(&children_types)
        {
            return Ok(false);
        }
        (wrapper_id, wrapper_parent_id, target_index)
    };

    let mut lifted_id = paragraph_id;
    tr.batch::<_, CommandError>(|tr| {
        tr.move_node(paragraph_id, wrapper_parent_id, target_index)?;

        lifted_id = {
            let view = tr.state().view();
            view.node(wrapper_parent_id)
                .and_then(|parent| match parent.child_at(target_index) {
                    Some(editor_model::ChildView::Block(block)) => Some(block.id()),
                    _ => None,
                })
                .ok_or_else(|| CommandError::Corrupted("lifted paragraph not found".into()))?
        };

        let (remove_wrapper, steps) = {
            let view = tr.state().view();
            match view.node(wrapper_id) {
                Some(wrapper) => {
                    // The projection synthesizes a Derived placeholder child for
                    // an empty required container, so emptiness must be judged by
                    // the presence of REAL children, not by `children().is_none()`.
                    let has_real_child = wrapper
                        .children()
                        .any(|c| child_elem_id(&c).as_op_dot().is_some());
                    if !has_real_child {
                        (true, Vec::new())
                    } else {
                        let remaining: Vec<NodeType> =
                            wrapper.children().map(|c| child_node_type(&c)).collect();
                        if !wrapper.spec().content.matches_sequence(&remaining) {
                            (false, dissolve(&wrapper))
                        } else {
                            (false, Vec::new())
                        }
                    }
                }
                None => (false, Vec::new()),
            }
        };
        if remove_wrapper {
            tr.remove_subtree(wrapper_id)?;
        } else {
            tr.apply_steps(steps)?;
        }
        Ok(())
    })?;

    let new_selection = Selection::collapsed(Position {
        node: lifted_id,
        offset: 0,
        affinity: Affinity::Downstream,
    });
    tr.set_selection(Some(new_selection))?;

    Ok(true)
}

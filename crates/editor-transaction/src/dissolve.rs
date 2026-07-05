use editor_crdt::Dot;
use editor_model::{ChildView, NodeSpec, NodeType, NodeView, Subtree};

use crate::Step;

/// Returns steps to promote `node`'s children into the parent and remove `node`.
/// Recursively dissolves promoted children that don't fit in the parent's content.
pub fn dissolve(node: &NodeView) -> Vec<Step> {
    let child_types: Vec<NodeType> = node
        .children()
        .map(|c| match c {
            ChildView::Block(b) => b.node_type(),
            ChildView::Leaf(l) => l.node_type(),
        })
        .collect();

    if node.spec().content.matches_sequence(&child_types) {
        return vec![];
    }

    let parent = match node.parent() {
        Some(p) => p,
        None => return vec![],
    };

    let node_index = match parent.child_blocks().position(|b| b.id() == node.id()) {
        Some(i) => i,
        None => return vec![],
    };
    dissolve_into(node, parent.id(), parent.spec(), node_index)
}

fn dissolve_into(
    node: &NodeView,
    effective_parent_id: Dot,
    effective_parent_spec: &'static NodeSpec,
    node_index: usize,
) -> Vec<Step> {
    let children: Vec<(Dot, NodeType)> = node
        .child_blocks()
        .map(|c| (c.id(), c.node_type()))
        .collect();

    let mut steps = Vec::new();

    for (j, (child_id, _)) in children.iter().enumerate() {
        steps.push(Step::MoveNode {
            block: *child_id,
            old_parent: node.id(),
            old_index: 0,
            new_parent: effective_parent_id,
            new_index: node_index + 1 + j,
        });
    }

    steps.push(Step::RemoveSubtree {
        parent: effective_parent_id,
        index: node_index,
        subtree: Subtree {
            node: node.node().to_plain(),
            modifiers: vec![],
            marker: None,
            children: vec![],
        },
    });

    for (j, (child_id, child_type)) in children.iter().enumerate() {
        if !effective_parent_spec.content.matches(*child_type)
            && let Some(child_ref) = node.child_blocks().find(|c| c.id() == *child_id)
        {
            let child_effective_index = node_index + j;
            steps.extend(dissolve_into(
                &child_ref,
                effective_parent_id,
                effective_parent_spec,
                child_effective_index,
            ));
        }
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    #[test]
    fn dissolve_valid_node_returns_empty() {
        let (state, bq1) = state! {
            doc { root { bq1: blockquote { paragraph } paragraph } }
            selection: (bq1, 0)
        };
        let view = state.view();
        let bq = view.node(bq1).unwrap();
        assert!(dissolve(&bq).is_empty());
    }
}

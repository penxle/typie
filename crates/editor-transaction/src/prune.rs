use editor_model::{ChildView, NodeView, Subtree};

use crate::Step;

/// Analyzes a node and returns RemoveSubtree steps needed to remove it and any
/// parent containers that would become empty as a result. Returns empty vec if
/// the node is non-empty or if empty is valid for this node.
pub fn prune(node: &NodeView) -> Vec<Step> {
    if node.children().next().is_some() {
        return vec![];
    }

    if node.spec().content.min_required() == 0 {
        return vec![];
    }

    // A structural node is a fixed part of its parent's shape; it can never be
    // removed, only emptied and re-fulfilled by the caller.
    if node.spec().structural {
        return vec![];
    }

    prune_empty(node)
}

fn prune_empty(node: &NodeView) -> Vec<Step> {
    let parent = match node.parent() {
        Some(p) => p,
        None => return vec![],
    };

    let index = match parent
        .children()
        .position(|c| matches!(&c, ChildView::Block(b) if b.id() == node.id()))
    {
        Some(i) => i,
        None => return vec![],
    };

    let mut steps = vec![Step::RemoveSubtree {
        parent: parent.id(),
        index,
        subtree: Subtree {
            node: node.node().to_plain(),
            modifiers: vec![],
            style: None,
            marker: None,
            children: vec![],
        },
    }];

    if parent.child_blocks().count() == 1
        && parent.spec().content.min_required() > 0
        && !parent.spec().structural
    {
        steps.extend(prune_empty(&parent));
    }

    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    #[test]
    fn prune_valid_empty_paragraph_returns_empty() {
        let (state, p1) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let view = state.view();
        let para = view.node(p1).unwrap();
        assert!(prune(&para).is_empty());
    }
}

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView};

/// Dot of a child view (block id, or the leaf's char/atom dot).
pub(crate) fn child_elem_id(child: &ChildView) -> Dot {
    match child {
        ChildView::Block(b) => b.id(),
        ChildView::Leaf(l) => l.dot(),
    }
}

/// Node type of a child view.
pub(crate) fn child_node_type(child: &ChildView) -> NodeType {
    match child {
        ChildView::Block(b) => b.node_type(),
        ChildView::Leaf(l) => l.node_type(),
    }
}

/// Previous sibling (block or leaf) of `node`, if any.
pub(crate) fn prev_sibling<'a>(node: &NodeView<'a>) -> Option<ChildView<'a>> {
    let parent = node.parent()?;
    let index = node.index()?;
    if index == 0 {
        return None;
    }
    parent.child_at(index - 1)
}

/// Next sibling (block or leaf) of `node`, if any.
pub(crate) fn next_sibling<'a>(node: &NodeView<'a>) -> Option<ChildView<'a>> {
    let parent = node.parent()?;
    let index = node.index()?;
    parent.child_at(index + 1)
}

/// Find the lowest common ancestor of two nodes.
pub(crate) fn find_lowest_common_ancestor(view: &DocView, a: Dot, b: Dot) -> Option<Dot> {
    let ancestors_a: Vec<Dot> = view.node(a)?.ancestors().map(|n| n.id()).collect();
    let ancestors_b: Vec<Dot> = view.node(b)?.ancestors().map(|n| n.id()).collect();

    let mut lca = None;
    for (la, lb) in ancestors_a.iter().rev().zip(ancestors_b.iter().rev()) {
        if la == lb {
            lca = Some(*la);
        } else {
            break;
        }
    }
    lca
}

/// Compute the index path from `ancestor` down to `node`.
/// Returns None if `node` is not a descendant of `ancestor`.
pub(crate) fn path_from_ancestor(view: &DocView, node: Dot, ancestor: Dot) -> Option<Vec<usize>> {
    if node == ancestor {
        return Some(Vec::new());
    }
    let mut path = Vec::new();
    let mut current = node;
    loop {
        let nv = view.node(current)?;
        let idx = nv.index()?;
        path.push(idx);
        let parent_id = nv.parent()?.id();
        if parent_id == ancestor {
            path.reverse();
            return Some(path);
        }
        current = parent_id;
    }
}

/// Find the nearest textblock ancestor (node whose content is all-inline).
pub(crate) fn find_ancestor_textblock(view: &DocView, node: Dot) -> Option<Dot> {
    let mut current = node;
    loop {
        let nv = view.node(current)?;
        if nv.spec().is_textblock() {
            return Some(current);
        }
        current = nv.parent()?.id();
    }
}

/// True when the node is a block-level container — a non-textblock node that
/// holds block-level children (e.g. the doc root, blockquote, list_item).
pub(crate) fn is_block_container(node: &NodeView) -> bool {
    let spec = node.spec();
    !spec.is_leaf() && !spec.is_textblock()
}

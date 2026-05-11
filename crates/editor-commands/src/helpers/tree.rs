use editor_model::{Doc, Node, NodeId, NodeRef};
use editor_state::Position;

/// Find the lowest common ancestor of two nodes.
pub(crate) fn find_lowest_common_ancestor(doc: &Doc, a: NodeId, b: NodeId) -> Option<NodeId> {
    let ancestors_a: Vec<NodeId> = doc.node(a)?.ancestors().map(|n| n.id()).collect();
    let ancestors_b: Vec<NodeId> = doc.node(b)?.ancestors().map(|n| n.id()).collect();

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

/// Compute the index path from `ancestor_id` down to `node_id`.
/// Returns None if `node_id` is not a descendant of `ancestor_id`.
pub(crate) fn path_from_ancestor(
    doc: &Doc,
    node_id: NodeId,
    ancestor_id: NodeId,
) -> Option<Vec<usize>> {
    if node_id == ancestor_id {
        return Some(Vec::new());
    }
    let mut path = Vec::new();
    let mut current = node_id;
    loop {
        let node = doc.node(current)?;
        let idx = node.index()?;
        path.push(idx);
        let parent_id = node.parent()?.id();
        if parent_id == ancestor_id {
            path.reverse();
            return Some(path);
        }
        current = parent_id;
    }
}

/// Find the nearest textblock ancestor (node whose content is all-inline).
pub(crate) fn find_ancestor_textblock(doc: &Doc, node_id: NodeId) -> Option<NodeId> {
    let mut current = node_id;
    loop {
        let node = doc.node(current)?;
        if node.spec().is_textblock() {
            return Some(current);
        }
        current = node.parent()?.id();
    }
}

/// Find the first cursor position within a node's subtree.
pub(crate) fn find_first_cursor_position(node: &NodeRef) -> Option<Position> {
    if let Node::Text(_) = node.node() {
        return Some(Position::new(node.id(), 0));
    }

    match node.first_child() {
        Some(child) => find_first_cursor_position(&child),
        None => {
            if node.spec().content.is_leaf() {
                let parent = node.parent()?;
                let idx = node.index()?;
                Some(Position::new(parent.id(), idx))
            } else {
                Some(Position::new(node.id(), 0))
            }
        }
    }
}

/// Find the last cursor position within a node's subtree.
pub(crate) fn find_last_cursor_position(node: &NodeRef) -> Option<Position> {
    if let Node::Text(text_node) = node.node() {
        return Some(Position::new(node.id(), text_node.text.len()));
    }

    match node.last_child() {
        Some(child) => find_last_cursor_position(&child),
        None => {
            if node.spec().content.is_leaf() {
                let parent = node.parent()?;
                let idx = node.index()?;
                Some(Position::new(parent.id(), idx + 1))
            } else {
                Some(Position::new(node.id(), 0))
            }
        }
    }
}

/// True when the node is a block-level container — a non-text, non-textblock node
/// that holds block-level children (e.g. the doc root, blockquote, list_item).
pub(crate) fn is_block_container(node: &Node) -> bool {
    !matches!(node, Node::Text(_)) && !node.spec().is_textblock()
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn lca_of_siblings_is_parent() {
        let (doc, p1, p2) = doc! {
            root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            }
        };
        assert_eq!(
            find_lowest_common_ancestor(&doc, p1, p2),
            Some(NodeId::ROOT)
        );
    }

    #[test]
    fn lca_of_cousins() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            }
        };
        assert_eq!(
            find_lowest_common_ancestor(&doc, t1, t2),
            Some(NodeId::ROOT)
        );
    }

    #[test]
    fn textblock_of_text_node_is_paragraph() {
        let (doc, p, t) = doc! {
            root {
                p: paragraph { t: text("Hello") }
            }
        };
        assert_eq!(find_ancestor_textblock(&doc, t), Some(p));
    }

    #[test]
    fn textblock_of_paragraph_is_itself() {
        let (doc, p) = doc! {
            root {
                p: paragraph { text("Hello") }
            }
        };
        assert_eq!(find_ancestor_textblock(&doc, p), Some(p));
    }

    #[test]
    fn path_from_ancestor_same_node() {
        let (doc,) = doc! {
            root { paragraph { text("Hello") } }
        };
        assert_eq!(
            path_from_ancestor(&doc, NodeId::ROOT, NodeId::ROOT),
            Some(vec![])
        );
    }

    #[test]
    fn path_from_ancestor_direct_child() {
        let (doc, p) = doc! {
            root { p: paragraph { text("Hello") } }
        };
        assert_eq!(path_from_ancestor(&doc, p, NodeId::ROOT), Some(vec![0]));
    }

    #[test]
    fn path_from_ancestor_grandchild() {
        let (doc, t) = doc! {
            root { paragraph { t: text("Hello") } }
        };
        assert_eq!(path_from_ancestor(&doc, t, NodeId::ROOT), Some(vec![0, 0]));
    }

    #[test]
    fn path_from_ancestor_second_branch() {
        let (doc, _, t2) = doc! {
            root {
                paragraph { _t1: text("Hello") }
                paragraph { t2: text("World") }
            }
        };
        assert_eq!(path_from_ancestor(&doc, t2, NodeId::ROOT), Some(vec![1, 0]));
    }

    #[test]
    fn first_cursor_position_in_paragraph() {
        let (doc, t) = doc! {
            root { paragraph { t: text("Hello") } }
        };
        assert_eq!(
            find_first_cursor_position(&doc.root().unwrap()),
            Some(Position::new(t, 0))
        );
    }

    #[test]
    fn last_cursor_position_in_paragraph() {
        let (doc, _, t2) = doc! {
            root {
                paragraph { _t1: text("Hello") }
                paragraph { t2: text("World") }
            }
        };
        assert_eq!(
            find_last_cursor_position(&doc.root().unwrap()),
            Some(Position::new(t2, 5))
        );
    }

    #[test]
    fn first_cursor_position_images_only() {
        let (doc,) = doc! {
            root { image image image }
        };
        assert_eq!(
            find_first_cursor_position(&doc.root().unwrap()),
            Some(Position::new(NodeId::ROOT, 0))
        );
    }

    #[test]
    fn last_cursor_position_images_only() {
        let (doc,) = doc! {
            root { image image image }
        };
        assert_eq!(
            find_last_cursor_position(&doc.root().unwrap()),
            Some(Position::new(NodeId::ROOT, 3))
        );
    }

    #[test]
    fn first_cursor_position_empty_paragraph() {
        let (doc, p) = doc! {
            root { p: paragraph {} }
        };
        assert_eq!(
            find_first_cursor_position(&doc.root().unwrap()),
            Some(Position::new(p, 0))
        );
    }
}

use editor_macros::ffi;
use editor_model::{Doc, NodeId, NodeRef, PlainNode};
use editor_state::{ResolvedSelection, State};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: NodeId,
    pub node: PlainNode,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct BlockState {
    pub ancestors: Vec<Block>,
    pub nodes: Vec<Block>,
}

pub fn resolve_block_state(state: &State) -> BlockState {
    let ancestors = resolve_ancestors(state);
    let nodes = resolve_nodes(state);
    BlockState { ancestors, nodes }
}

fn resolve_ancestors(state: &State) -> Vec<Block> {
    let doc = &state.doc;
    let head_chain = ancestor_chain_from(doc, state.selection.head.node_id);
    if state.selection.is_collapsed() {
        return head_chain;
    }
    let anchor_chain = ancestor_chain_from(doc, state.selection.anchor.node_id);
    common_suffix(&head_chain, &anchor_chain)
}

fn ancestor_chain_from(doc: &Doc, leaf_id: NodeId) -> Vec<Block> {
    let Some(leaf) = doc.node(leaf_id) else {
        return vec![];
    };
    let mut chain: Vec<Block> = Vec::new();
    let mut current: Option<NodeRef> = Some(leaf);
    while let Some(n) = current {
        if !n.spec().inline {
            chain.push(Block {
                id: n.id(),
                node: n.node().to_plain(),
            });
        }
        current = n.parent();
    }
    chain
}

/// Both chains are stored leaf-first → root-last. Returns their common
/// root-side prefix (i.e. last-common-suffix of the leaf-first lists),
/// re-ordered leaf-first → root-last to match the input convention.
fn common_suffix(a: &[Block], b: &[Block]) -> Vec<Block> {
    let mut ra = a.iter().rev().peekable();
    let mut rb = b.iter().rev().peekable();
    let mut shared_from_root: Vec<Block> = Vec::new();
    while let (Some(x), Some(y)) = (ra.peek(), rb.peek()) {
        if x.id == y.id {
            shared_from_root.push((*x).clone());
            ra.next();
            rb.next();
        } else {
            break;
        }
    }
    shared_from_root.reverse();
    shared_from_root
}

fn resolve_nodes(state: &State) -> Vec<Block> {
    let sel = state.selection;
    if sel.is_collapsed() {
        return Vec::new();
    }
    let Some(rs) = sel.resolve(&state.doc) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_contained(&state.doc.root().unwrap(), &rs, &mut out);
    out
}

/// Collect `node` itself and any descendant non-inline nodes wholly
/// contained in `rs`, in document (preorder) order.
fn collect_contained<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection, out: &mut Vec<Block>) {
    if rs.contains_subtree(node) && !node.spec().inline {
        out.push(Block {
            id: node.id(),
            node: node.node().to_plain(),
        });
    }
    // Descend even when self is wholly contained: nested non-inline children
    // (e.g., paragraphs inside a blockquote) must be enumerated too.
    if rs.intersects_subtree(node) {
        for child in node.children() {
            collect_contained(&child, rs, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::PlainNode;

    use super::*;

    #[test]
    fn ancestors_from_text_skips_inline() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let bs = resolve_block_state(&state);
        assert_eq!(bs.ancestors.len(), 2);
        assert!(matches!(bs.ancestors[0].node, PlainNode::Paragraph(_)));
        assert!(matches!(bs.ancestors[1].node, PlainNode::Root(_)));
    }

    #[test]
    fn ancestors_common_prefix_when_chain_diverges() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("Hi") }
                p2: paragraph { t2: text("Lo") }
            } }
            selection: (t1, 0) -> (t2, 2)
        };
        let bs = resolve_block_state(&state);
        assert_eq!(bs.ancestors.len(), 1);
        assert!(matches!(bs.ancestors[0].node, PlainNode::Root(_)));
    }

    #[test]
    fn nodes_empty_for_collapsed() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let bs = resolve_block_state(&state);
        assert!(bs.nodes.is_empty());
    }

    #[test]
    fn nodes_includes_image_wholly_contained() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("ab") }
                img: image
                p2: paragraph { t2: text("cd") }
            } }
            selection: (t1, 0) -> (t2, 2)
        };
        let bs = resolve_block_state(&state);
        assert!(
            bs.nodes
                .iter()
                .any(|b| matches!(b.node, PlainNode::Image(_)))
        );
        assert_eq!(bs.ancestors.len(), 1);
    }

    #[test]
    fn nodes_includes_blockquote_and_inner_paragraph_when_both_wholly_contained() {
        // Selection wraps blockquote + trailing paragraph at root level.
        // Both the blockquote and its inner paragraph are non-inline and
        // wholly contained, so both must surface in `nodes` —
        // collect_contained does NOT short-circuit on contains_subtree.
        let (state, ..) = state! {
            doc { r: root {
                blockquote {
                    paragraph { _t_inner: text("inside") }
                }
                paragraph { t_after: text("after") }
            } }
            selection: (r, 0) -> (t_after, 5)
        };
        let bs = resolve_block_state(&state);
        let has_blockquote = bs
            .nodes
            .iter()
            .any(|b| matches!(b.node, PlainNode::Blockquote(_)));
        let inner_paragraph_count = bs
            .nodes
            .iter()
            .filter(|b| matches!(b.node, PlainNode::Paragraph(_)))
            .count();
        assert!(has_blockquote, "blockquote must be in nodes");
        assert!(
            inner_paragraph_count >= 1,
            "inner paragraph must be in nodes (recursion descends into wholly contained blockquote)"
        );
    }
}

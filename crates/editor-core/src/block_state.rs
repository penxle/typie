use editor_commands as commands;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{ChildView, DocView, NodeType, NodeView, PlainNode, Schema};
use editor_resource::Resource;
use editor_state::{ResolvedSelection, Selection, State};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: Dot,
    pub node: PlainNode,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ListAffordances {
    pub toggle_bullet: bool,
    pub toggle_ordered: bool,
    pub indent: bool,
    pub outdent: bool,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct ExpansionAffordances {
    pub word: bool,
    pub sentence: bool,
    pub paragraph: bool,
    pub all: bool,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct BlockState {
    pub ancestors: Vec<Block>,
    pub nodes: Vec<Block>,
    pub intersecting_nodes: Vec<Block>,
    pub list: ListAffordances,
    pub expansion: ExpansionAffordances,
}

pub fn resolve_block_state(state: &State, resource: &Resource) -> Option<BlockState> {
    let selection = state.selection.as_ref()?;
    let ancestors = resolve_ancestors(state, selection);
    let (nodes, intersecting_nodes) = resolve_range_nodes(state, selection);
    let list = resolve_list_affordances(state, selection);
    let expansion = resolve_expansion_affordances(state, selection, resource);
    Some(BlockState {
        ancestors,
        nodes,
        intersecting_nodes,
        list,
        expansion,
    })
}

fn resolve_expansion_affordances(
    state: &State,
    selection: &Selection,
    resource: &Resource,
) -> ExpansionAffordances {
    let view = state.view();
    let selection = Some(*selection);
    ExpansionAffordances {
        word: commands::judge_expand_word(&view, selection, resource).changes(),
        sentence: commands::judge_expand_sentence(&view, selection, resource).changes(),
        paragraph: commands::judge_expand_paragraph(&view, selection).changes(),
        all: commands::judge_expand_all(&view, selection).changes(),
    }
}

fn resolve_list_affordances(state: &State, selection: &Selection) -> ListAffordances {
    let view = state.view();
    ListAffordances {
        toggle_bullet: commands::judge_toggle_list_kind(&view, selection, NodeType::BulletList)
            .changes(),
        toggle_ordered: commands::judge_toggle_list_kind(&view, selection, NodeType::OrderedList)
            .changes(),
        indent: commands::judge_indent_list(&view, selection).changes(),
        outdent: commands::judge_outdent_list(&view, selection).changes(),
    }
}

fn resolve_ancestors(state: &State, selection: &Selection) -> Vec<Block> {
    let view = state.view();
    let head_chain = ancestor_chain_from(&view, selection.head.node);
    if selection.is_collapsed() {
        return head_chain;
    }
    let anchor_chain = ancestor_chain_from(&view, selection.anchor.node);
    common_suffix(&head_chain, &anchor_chain)
}

fn ancestor_chain_from(doc: &DocView, leaf_id: Dot) -> Vec<Block> {
    let Some(leaf) = doc.node(leaf_id) else {
        return vec![];
    };
    let mut chain: Vec<Block> = Vec::new();
    let mut current: Option<NodeView> = Some(leaf);
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

fn resolve_range_nodes(state: &State, selection: &Selection) -> (Vec<Block>, Vec<Block>) {
    if selection.is_collapsed() {
        return (Vec::new(), Vec::new());
    }
    let view = state.view();
    let Some(rs) = selection.resolve(&view) else {
        return (Vec::new(), Vec::new());
    };
    let mut out = RangeNodes::default();
    collect_range_nodes(&view.root().unwrap(), &rs, &mut out);
    (out.contained, out.intersecting)
}

#[derive(Default)]
struct RangeNodes {
    contained: Vec<Block>,
    intersecting: Vec<Block>,
}

/// Collect non-inline nodes touched by `rs`, in document (preorder) order.
/// `contained` keeps the historical `nodes` contract: only nodes wholly
/// contained in the range. `intersecting` is wider and includes nodes whose
/// subtree merely overlaps the range; root is omitted because it would be
/// present for every non-collapsed range.
fn collect_range_nodes(node: &NodeView, rs: &ResolvedSelection, out: &mut RangeNodes) {
    if !rs.intersects_subtree(node) {
        return;
    }

    let contains = rs.contains_subtree(node);
    if !node.spec().inline {
        let block = Block {
            id: node.id(),
            node: node.node().to_plain(),
        };
        if contains {
            out.contained.push(block.clone());
        }
        if node.node_type() != NodeType::Root {
            out.intersecting.push(block);
        }
    }

    // Descend even when self is wholly contained: nested non-inline children
    // (e.g., paragraphs inside a blockquote) must be enumerated too.
    for (i, child) in node.children().enumerate() {
        match child {
            ChildView::Block(b) => collect_range_nodes(&b, rs, out),
            ChildView::Leaf(l) => {
                let Some(leaf_node) = l.node() else { continue };
                if Schema::node_spec(l.node_type()).inline {
                    continue;
                }
                if rs.contains_leaf_slot(node, i) {
                    let block = Block {
                        id: l.dot(),
                        node: leaf_node.to_plain(),
                    };
                    out.contained.push(block.clone());
                    out.intersecting.push(block);
                }
            }
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
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 1)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert_eq!(bs.ancestors.len(), 2);
        assert!(matches!(bs.ancestors[0].node, PlainNode::Paragraph(_)));
        assert!(matches!(bs.ancestors[1].node, PlainNode::Root(_)));
    }

    #[test]
    fn ancestors_common_prefix_when_chain_diverges() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("Hi") }
                p2: paragraph { text("Lo") }
            } }
            selection: (p1, 0) -> (p2, 2)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert_eq!(bs.ancestors.len(), 1);
        assert!(matches!(bs.ancestors[0].node, PlainNode::Root(_)));
    }

    #[test]
    fn nodes_empty_for_collapsed() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 1)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(bs.nodes.is_empty());
        assert!(bs.intersecting_nodes.is_empty());
    }

    #[test]
    fn nodes_includes_image_wholly_contained() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("ab") }
                img: image
                p2: paragraph { text("cd") }
            } }
            selection: (p1, 0) -> (p2, 2)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(
            bs.nodes
                .iter()
                .any(|b| matches!(b.node, PlainNode::Image(_)))
        );
        assert_eq!(bs.ancestors.len(), 1);
    }

    #[test]
    fn intersecting_nodes_include_list_when_endpoint_is_inside_list() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("plain") }
                bullet_list {
                    list_item {
                        p2: paragraph { text("item") }
                    }
                }
            } }
            selection: (p1, 2) -> (p2, 2)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();

        assert!(bs.intersecting_nodes.iter().any(|block| matches!(
            block.node,
            PlainNode::BulletList(_) | PlainNode::OrderedList(_) | PlainNode::ListItem(_)
        )));
        assert!(!bs.nodes.iter().any(|block| matches!(
            block.node,
            PlainNode::BulletList(_) | PlainNode::OrderedList(_) | PlainNode::ListItem(_)
        )));
    }

    #[test]
    fn intersecting_nodes_include_plain_blocks_for_plain_paragraph_range() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("one") }
                p2: paragraph { text("two") }
            } }
            selection: (p1, 1) -> (p2, 2)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();

        assert_eq!(
            bs.intersecting_nodes
                .iter()
                .filter(|block| matches!(block.node, PlainNode::Paragraph(_)))
                .count(),
            2
        );
        assert!(
            !bs.intersecting_nodes
                .iter()
                .any(|block| matches!(block.node, PlainNode::Root(_)))
        );
        assert!(!bs.intersecting_nodes.iter().any(|block| matches!(
            block.node,
            PlainNode::BulletList(_) | PlainNode::OrderedList(_) | PlainNode::ListItem(_)
        )));
    }

    #[test]
    fn nodes_includes_image_for_exact_unit_selection() {
        let (state, ..) = state! {
            doc { root: root {
                img: image
                p1: paragraph { text("ab") }
            } }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(
            bs.nodes
                .iter()
                .any(|b| matches!(b.node, PlainNode::Image(_)))
        );
    }

    #[test]
    fn list_affordances_inside_bullet_list() {
        let (state, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item { p1: paragraph { text("B") } }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(bs.list.toggle_bullet);
        assert!(bs.list.toggle_ordered);
        assert!(bs.list.indent);
        assert!(bs.list.outdent);
    }

    #[test]
    fn list_affordances_plain_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 1)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(bs.list.toggle_bullet);
        assert!(bs.list.toggle_ordered);
        assert!(!bs.list.indent);
        assert!(!bs.list.outdent);
    }

    #[test]
    fn list_affordances_first_item_cannot_indent() {
        let (state, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("A") } } }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(!bs.list.indent);
        assert!(bs.list.outdent);
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
                    _p_inner: paragraph { text("inside") }
                }
                p_after: paragraph { text("after") }
            } }
            selection: (r, 0) -> (p_after, 5)
        };
        let resource = editor_resource::Resource::new_test();
        let bs = resolve_block_state(&state, &resource).unwrap();
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

    #[test]
    fn expansion_affordances_for_text_caret() {
        let resource = editor_resource::Resource::new_test();
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello world. Second sentence.") } } }
            selection: (p1, 2)
        };
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(bs.expansion.word);
        assert!(bs.expansion.sentence);
        assert!(bs.expansion.paragraph);
        assert!(bs.expansion.all);
    }

    #[test]
    fn expansion_affordances_when_all_selected() {
        let resource = editor_resource::Resource::new_test();
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut state = state;
        state.selection = Some(editor_state::Selection::new(
            editor_state::Position::new(p1, 0),
            editor_state::Position {
                node: p1,
                offset: 2,
                affinity: editor_state::Affinity::Upstream,
            },
        ));
        let bs = resolve_block_state(&state, &resource).unwrap();
        assert!(!bs.expansion.all);
        assert!(!bs.expansion.paragraph);
    }
}

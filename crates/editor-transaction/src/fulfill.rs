use editor_model::{ChildView, ContentExpr, NodeType, NodeView, Subtree, content_placement};

use crate::Step;

/// Analyzes a node's content expression and returns InsertSubtree steps
/// needed to make it valid. Returns empty vec if already valid.
pub fn fulfill(node: &NodeView) -> Vec<Step> {
    let insertions = content_placement(node.node_type(), &child_types(node))
        .completion_insertions
        .unwrap_or_default();
    insertions
        .into_iter()
        .map(|insertion| {
            let subtree = scaffold(insertion.node_type);
            Step::InsertSubtree {
                parent: node.id(),
                index: insertion.index,
                subtree,
            }
        })
        .collect()
}

/// First child type `content` accepts, or `None` when the expression admits no
/// typed child (`Empty`/`Any`).
pub fn first_child_type(content: &ContentExpr) -> Option<NodeType> {
    match content {
        ContentExpr::Empty | ContentExpr::Any => None,
        _ => Some(first_type(content)),
    }
}

/// Minimum valid subtree for `node_type`, recursively filling required children.
pub fn minimal_subtree(node_type: NodeType) -> Subtree {
    scaffold(node_type)
}

fn child_types(node: &NodeView) -> Vec<NodeType> {
    node.children()
        .map(|child| match child {
            ChildView::Block(block) => block.node_type(),
            ChildView::Leaf(leaf) => leaf.node_type(),
        })
        .collect()
}

fn first_type(expr: &ContentExpr) -> NodeType {
    match expr {
        ContentExpr::Single(t) => *t,
        ContentExpr::Choice(choices) => first_type(&choices[0]),
        ContentExpr::OneOrMore(inner)
        | ContentExpr::ZeroOrMore(inner)
        | ContentExpr::Optional(inner) => first_type(inner),
        ContentExpr::Seq(exprs) => first_type(&exprs[0]),
        ContentExpr::Empty | ContentExpr::Any => unreachable!("Empty/Any content has no type"),
    }
}

/// Build minimum valid subtree for a NodeType, recursively filling required children.
fn scaffold(node_type: NodeType) -> Subtree {
    let node = node_type.into_node().to_plain();
    let spec = node_type.spec();
    let children = scaffold_children(&spec.content);

    Subtree {
        node,
        modifiers: vec![],
        carry: Vec::new(),
        children,
        source_dots: Vec::new(),
    }
}

fn scaffold_children(content: &ContentExpr) -> Vec<Subtree> {
    match content {
        ContentExpr::Empty
        | ContentExpr::Any
        | ContentExpr::ZeroOrMore(_)
        | ContentExpr::Optional(_) => vec![],
        ContentExpr::Single(t) => vec![scaffold(*t)],
        ContentExpr::OneOrMore(inner) => vec![scaffold(first_type(inner))],
        ContentExpr::Choice(choices) => vec![scaffold(first_type(&choices[0]))],
        ContentExpr::Seq(exprs) => exprs.iter().flat_map(scaffold_children).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::FastMap;
    use editor_macros::state;

    // The projected DocView is always normalized (missing required children are
    // synthesized as derived nodes), so `fulfill` observes only already-valid
    // nodes here and returns no repair steps. Repair-step generation against
    // partially-built structures is exercised through the command layer (M2).
    #[test]
    fn fulfill_valid_root_returns_empty() {
        let (state, ..) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let view = state.view();
        let root = view.root().unwrap();
        assert!(fulfill(&root).is_empty());
    }

    #[test]
    fn fulfill_skips_unknown_children_and_remaps_insertion_index() {
        use editor_crdt::Dot;
        use editor_model::{BlockTree, DocView, ProjectedDoc, RawChild, RawNode, RawTree};

        let fold_id = Dot::new(1, 0);
        let unknown_id = Dot::new(1, 1);
        let title_id = Dot::new(1, 2);

        let fold = RawNode {
            id: fold_id,
            node_type: NodeType::Fold,
            attrs: vec![],
            children: vec![
                RawChild::Block(RawNode {
                    id: unknown_id,
                    node_type: NodeType::Unknown,
                    attrs: vec![],
                    children: vec![],
                }),
                RawChild::Block(RawNode {
                    id: title_id,
                    node_type: NodeType::FoldTitle,
                    attrs: vec![],
                    children: vec![],
                }),
            ],
        };
        let tree = BlockTree::from_raw(&RawTree { roots: vec![fold] });

        let doc = ProjectedDoc {
            tree,
            block_effective: FastMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: FastMap::new(),
            node_attrs: FastMap::new(),
            node_carries: FastMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
            hidden: editor_model::HiddenCopies::default(),
            redirected: FastMap::new(),
            repair_stats: editor_model::RepairStats::default(),
        };
        let view = DocView::new(&doc);
        let node = view.node(fold_id).unwrap();

        let steps = fulfill(&node);
        assert_eq!(
            steps,
            vec![Step::InsertSubtree {
                parent: fold_id,
                index: 2,
                subtree: scaffold(NodeType::FoldContent),
            }]
        );
    }

    /// Physical order is [FoldContent, Unknown], so the shared placement result
    /// inserts the missing FoldTitle at index 0, not after the Unknown.
    #[test]
    fn fulfill_inserts_missing_role_before_trailing_unknown() {
        use editor_crdt::Dot;
        use editor_model::{BlockTree, DocView, ProjectedDoc, RawChild, RawNode, RawTree};

        let fold_id = Dot::new(1, 0);
        let content_id = Dot::new(1, 1);
        let unknown_id = Dot::new(1, 2);

        let fold = RawNode {
            id: fold_id,
            node_type: NodeType::Fold,
            attrs: vec![],
            children: vec![
                RawChild::Block(RawNode {
                    id: content_id,
                    node_type: NodeType::FoldContent,
                    attrs: vec![],
                    children: vec![],
                }),
                RawChild::Block(RawNode {
                    id: unknown_id,
                    node_type: NodeType::Unknown,
                    attrs: vec![],
                    children: vec![],
                }),
            ],
        };
        let tree = BlockTree::from_raw(&RawTree { roots: vec![fold] });

        let doc = ProjectedDoc {
            tree,
            block_effective: FastMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: FastMap::new(),
            node_attrs: FastMap::new(),
            node_carries: FastMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
            hidden: editor_model::HiddenCopies::default(),
            redirected: FastMap::new(),
            repair_stats: editor_model::RepairStats::default(),
        };
        let view = DocView::new(&doc);
        let node = view.node(fold_id).unwrap();

        let steps = fulfill(&node);
        assert_eq!(
            steps,
            vec![Step::InsertSubtree {
                parent: fold_id,
                index: 0,
                subtree: scaffold(NodeType::FoldTitle),
            }]
        );
    }

    /// Physical order is [Unknown, FoldContent], so the shared placement result
    /// keeps the Unknown transparent while inserting FoldTitle after it.
    #[test]
    fn fulfill_inserts_missing_role_after_leading_unknown() {
        use editor_crdt::Dot;
        use editor_model::{BlockTree, DocView, ProjectedDoc, RawChild, RawNode, RawTree};

        let fold_id = Dot::new(1, 0);
        let unknown_id = Dot::new(1, 1);
        let content_id = Dot::new(1, 2);

        let fold = RawNode {
            id: fold_id,
            node_type: NodeType::Fold,
            attrs: vec![],
            children: vec![
                RawChild::Block(RawNode {
                    id: unknown_id,
                    node_type: NodeType::Unknown,
                    attrs: vec![],
                    children: vec![],
                }),
                RawChild::Block(RawNode {
                    id: content_id,
                    node_type: NodeType::FoldContent,
                    attrs: vec![],
                    children: vec![],
                }),
            ],
        };
        let tree = BlockTree::from_raw(&RawTree { roots: vec![fold] });

        let doc = ProjectedDoc {
            tree,
            block_effective: FastMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: FastMap::new(),
            node_attrs: FastMap::new(),
            node_carries: FastMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
            hidden: editor_model::HiddenCopies::default(),
            redirected: FastMap::new(),
            repair_stats: editor_model::RepairStats::default(),
        };
        let view = DocView::new(&doc);
        let node = view.node(fold_id).unwrap();

        let steps = fulfill(&node);
        assert_eq!(
            steps,
            vec![Step::InsertSubtree {
                parent: fold_id,
                index: 1,
                subtree: scaffold(NodeType::FoldTitle),
            }]
        );
    }
}

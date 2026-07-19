use editor_model::{ChildView, ContentExpr, NodeType, NodeView, Subtree};

use crate::Step;

/// Analyzes a node's content expression and returns InsertSubtree steps
/// needed to make it valid. Returns empty vec if already valid.
pub fn fulfill(node: &NodeView) -> Vec<Step> {
    let spec = node.spec();
    let (child_types, real_indices) = known_child_types(node);

    if spec.content.matches_sequence(&child_types) {
        return vec![];
    }

    let insertions = spec
        .content
        .completion_insertions(&child_types)
        .unwrap_or_default();
    let total_children = node.child_count();
    insertions
        .into_iter()
        .enumerate()
        .map(|(k, (filtered_index, node_type))| {
            let unshifted = filtered_index - k;
            let real_index = real_indices
                .get(unshifted)
                .copied()
                .unwrap_or(total_children);
            let subtree = scaffold(node_type);
            Step::InsertSubtree {
                parent: node.id(),
                index: real_index + k,
                subtree,
            }
        })
        .collect()
}

fn known_child_types(node: &NodeView) -> (Vec<NodeType>, Vec<usize>) {
    let mut types = Vec::new();
    let mut real_indices = Vec::new();
    for (i, c) in node.children().enumerate() {
        let node_type = match c {
            ChildView::Block(b) => b.node_type(),
            ChildView::Leaf(l) => l.node_type(),
        };
        if node_type == NodeType::Unknown {
            continue;
        }
        types.push(node_type);
        real_indices.push(i);
    }
    (types, real_indices)
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
        use editor_model::{BlockNode, BlockTree, Child, ChildList, DocView, ProjectedDoc};

        let fold_id = Dot::new(1, 0);
        let unknown_id = Dot::new(1, 1);
        let title_id = Dot::new(1, 2);

        let mut nodes = editor_model::imbl::HashMap::new();
        nodes.insert(
            fold_id,
            BlockNode {
                id: fold_id,
                node_type: NodeType::Fold,
                attrs: vec![],
                children: ChildList::from(vec![Child::Block(unknown_id), Child::Block(title_id)]),
            },
        );
        nodes.insert(
            unknown_id,
            BlockNode {
                id: unknown_id,
                node_type: NodeType::Unknown,
                attrs: vec![],
                children: ChildList::new(),
            },
        );
        nodes.insert(
            title_id,
            BlockNode {
                id: title_id,
                node_type: NodeType::FoldTitle,
                attrs: vec![],
                children: ChildList::new(),
            },
        );

        let doc = ProjectedDoc {
            tree: BlockTree {
                nodes,
                root: fold_id,
            },
            block_effective: editor_model::imbl::HashMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: editor_model::imbl::HashMap::new(),
            node_attrs: editor_model::imbl::HashMap::new(),
            node_carries: editor_model::imbl::HashMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
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

    /// The append-fallback case above hits `real_indices.get(unshifted) == None`
    /// (the missing type sorts after every known child, so the repair index
    /// falls back to `total_children`). This oracle instead hits `Some(_)`: an
    /// interior real_indices lookup succeeds because the missing child must be
    /// inserted *before* an already-present known child. Physical order here
    /// is [FoldContent, Unknown], so `fulfill` must insert the missing FoldTitle
    /// at real index 0, not append it after the Unknown.
    #[test]
    fn fulfill_remaps_insertion_index_via_real_indices_hit_before_unknown() {
        use editor_crdt::Dot;
        use editor_model::{BlockNode, BlockTree, Child, ChildList, DocView, ProjectedDoc};

        let fold_id = Dot::new(1, 0);
        let content_id = Dot::new(1, 1);
        let unknown_id = Dot::new(1, 2);

        let mut nodes = editor_model::imbl::HashMap::new();
        nodes.insert(
            fold_id,
            BlockNode {
                id: fold_id,
                node_type: NodeType::Fold,
                attrs: vec![],
                children: ChildList::from(vec![Child::Block(content_id), Child::Block(unknown_id)]),
            },
        );
        nodes.insert(
            content_id,
            BlockNode {
                id: content_id,
                node_type: NodeType::FoldContent,
                attrs: vec![],
                children: ChildList::new(),
            },
        );
        nodes.insert(
            unknown_id,
            BlockNode {
                id: unknown_id,
                node_type: NodeType::Unknown,
                attrs: vec![],
                children: ChildList::new(),
            },
        );
        let doc = ProjectedDoc {
            tree: BlockTree {
                nodes,
                root: fold_id,
            },
            block_effective: editor_model::imbl::HashMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: editor_model::imbl::HashMap::new(),
            node_attrs: editor_model::imbl::HashMap::new(),
            node_carries: editor_model::imbl::HashMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
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

    /// The two oracles above both land on `real_index == unshifted` (0 in both
    /// cases) because their Unknown sits *after* the insertion point — a
    /// regression that dropped the `real_indices` remap entirely (using the
    /// filtered index directly as the real index) would slip through both
    /// unnoticed. This oracle puts the Unknown *before* the insertion point
    /// (`[Unknown, FoldContent]`), so the real index (1, past the
    /// Unknown) diverges from the filtered index (0, Unknown excluded) —
    /// only the `real_indices` lookup, not the raw filtered index, produces
    /// the expected step.
    #[test]
    fn fulfill_remaps_insertion_index_when_real_and_filtered_indices_diverge() {
        use editor_crdt::Dot;
        use editor_model::{BlockNode, BlockTree, Child, ChildList, DocView, ProjectedDoc};

        let fold_id = Dot::new(1, 0);
        let unknown_id = Dot::new(1, 1);
        let content_id = Dot::new(1, 2);

        let mut nodes = editor_model::imbl::HashMap::new();
        nodes.insert(
            fold_id,
            BlockNode {
                id: fold_id,
                node_type: NodeType::Fold,
                attrs: vec![],
                children: ChildList::from(vec![Child::Block(unknown_id), Child::Block(content_id)]),
            },
        );
        nodes.insert(
            unknown_id,
            BlockNode {
                id: unknown_id,
                node_type: NodeType::Unknown,
                attrs: vec![],
                children: ChildList::new(),
            },
        );
        nodes.insert(
            content_id,
            BlockNode {
                id: content_id,
                node_type: NodeType::FoldContent,
                attrs: vec![],
                children: ChildList::new(),
            },
        );
        let doc = ProjectedDoc {
            tree: BlockTree {
                nodes,
                root: fold_id,
            },
            block_effective: editor_model::imbl::HashMap::new(),
            seg_index: editor_model::BlockSegs::default(),
            block_modifiers: editor_model::imbl::HashMap::new(),
            node_attrs: editor_model::imbl::HashMap::new(),
            node_carries: editor_model::imbl::HashMap::new(),
            alias_classes: editor_model::AliasClasses::default(),
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

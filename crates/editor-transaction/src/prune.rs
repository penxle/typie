use editor_model::*;

use crate::Step;

/// Analyzes a node and returns RemoveSubtree steps needed to remove it and any
/// parent containers that would become empty as a result. Returns empty vec if
/// the node is non-empty or if empty is valid for this node.
pub fn prune(node: &NodeRef) -> Vec<Step> {
    if !node.entry().children.is_empty() {
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

/// Generates RemoveSubtree steps for a node that is known to be empty and requires children.
/// Recurses into the parent if it would also become empty.
fn prune_empty(node: &NodeRef) -> Vec<Step> {
    // Can't remove root — no parent to attach RemoveSubtree to
    let parent = match node.parent() {
        Some(p) => p,
        None => return vec![],
    };

    let index = node.index().unwrap();

    let mut steps = vec![Step::RemoveSubtree {
        parent_id: parent.id(),
        index,
        subtree: Subtree {
            id: node.id(),
            node: node.node().to_plain(),
            modifiers: node.explicit_modifiers().cloned().collect(),
            children: vec![],
        },
    }];

    // If parent will also become empty after removal, cascade
    if parent.entry().children.len() == 1
        && parent.spec().content.min_required() > 0
        && !parent.spec().structural
    {
        steps.extend(prune_empty(&parent));
    }

    steps
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn prune_empty_blockquote() {
        // Blockquote content: (P|BL|OL)+, min_required=1 — empty blockquote must be removed
        let (doc, bq1, ..) = doc! {
            root {
                bq1: blockquote
                paragraph
            }
        };

        let bq = doc.node(bq1).unwrap();
        let steps = prune(&bq);

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, NodeId::ROOT);
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, bq1);
                assert!(matches!(subtree.node, PlainNode::Blockquote(_)));
                assert!(subtree.children.is_empty());
            }
            _ => panic!("expected RemoveSubtree"),
        }
    }

    #[test]
    fn prune_nonempty_returns_empty() {
        // Blockquote with a Paragraph child — not empty, nothing to prune
        let (doc, bq1, ..) = doc! {
            root {
                bq1: blockquote {
                    paragraph
                }
                paragraph
            }
        };

        let bq = doc.node(bq1).unwrap();
        let steps = prune(&bq);
        assert!(steps.is_empty());
    }

    #[test]
    fn prune_valid_empty_node_returns_empty() {
        // Paragraph content: (Text|HardBreak)*, min_required=0 — empty is valid
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph
            }
        };

        let para = doc.node(p1).unwrap();
        let steps = prune(&para);
        assert!(steps.is_empty());
    }

    #[test]
    fn prune_skips_structural_node() {
        // TableCell: content (Paragraph|...)+, min_required > 0, structural = true.
        // An empty structural node must NEVER be pruned — it is fulfilled instead.
        let (doc, c1, ..) = doc! {
            root {
                table {
                    table_row {
                        c1: table_cell
                        table_cell { paragraph }
                    }
                }
                paragraph
            }
        };
        let cell = doc.node(c1).unwrap();
        let steps = prune(&cell);
        assert!(
            steps.is_empty(),
            "structural TableCell must not be pruned, got {steps:?}"
        );
    }

    #[test]
    fn prune_recursive_cascade() {
        // Callout inside Blockquote; Callout is empty, Blockquote has only Callout
        // → prune(callout) should produce 2 steps: remove callout, then remove blockquote
        let (doc, bq1, co1, ..) = doc! {
            root {
                bq1: blockquote {
                    co1: callout
                }
                paragraph
            }
        };

        let callout = doc.node(co1).unwrap();
        let steps = prune(&callout);

        assert_eq!(steps.len(), 2);

        // First step: remove callout from blockquote
        match &steps[0] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, bq1);
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, co1);
                assert!(matches!(subtree.node, PlainNode::Callout(_)));
            }
            _ => panic!("expected RemoveSubtree for callout"),
        }

        // Second step: remove blockquote from root (it would be empty)
        match &steps[1] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, NodeId::ROOT);
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, bq1);
                assert!(matches!(subtree.node, PlainNode::Blockquote(_)));
            }
            _ => panic!("expected RemoveSubtree for blockquote"),
        }
    }
}

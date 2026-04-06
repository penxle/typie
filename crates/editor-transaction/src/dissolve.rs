use editor_model::*;
use editor_schema::NodeSpecExt;

use crate::Step;

/// Returns steps to promote `node`'s children into the parent and remove `node`.
/// Recursively dissolves promoted children that don't fit in the parent's content.
pub fn dissolve(node: &NodeRef) -> Vec<Step> {
    let child_types: Vec<NodeType> = node.children().map(|c| c.as_type()).collect();

    if node.spec().content.matches_sequence(&child_types) {
        return vec![];
    }

    let parent = match node.parent() {
        Some(p) => p,
        None => return vec![],
    };

    let node_index = node.index().unwrap();
    dissolve_into(node, parent.id(), parent.spec(), node_index)
}

/// Promotes all children of `node` into `effective_parent` and removes `node`.
/// `node_index` reflects the post-move position within the step sequence being built.
fn dissolve_into(
    node: &NodeRef,
    effective_parent_id: NodeId,
    effective_parent_spec: &'static editor_schema::NodeSpec,
    node_index: usize,
) -> Vec<Step> {
    let children: Vec<(NodeId, NodeType)> =
        node.children().map(|c| (c.id(), c.as_type())).collect();

    let mut steps = Vec::new();

    for (j, (child_id, _)) in children.iter().enumerate() {
        steps.push(Step::MoveNode {
            node_id: *child_id,
            old_parent: node.id(),
            old_index: 0,
            new_parent: effective_parent_id,
            new_index: node_index + 1 + j,
        });
    }

    steps.push(Step::RemoveSubtree {
        parent_id: effective_parent_id,
        index: node_index,
        subtree: Subtree {
            id: node.id(),
            node: node.node().clone(),
            modifiers: node.modifiers().to_vec(),
            children: vec![],
        },
    });

    for (j, (child_id, child_type)) in children.iter().enumerate() {
        if !effective_parent_spec.content.matches(*child_type)
            && let Some(child_ref) = node.children().find(|c| c.id() == *child_id)
        {
            // After RemoveSubtree of node, children that were at node_index+1+j
            // shift down by 1, so the child is now at node_index+j
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
    use editor_macros::doc;

    use super::*;

    #[test]
    fn dissolve_valid_node_returns_empty() {
        // Blockquote with a valid Paragraph child — content is satisfied
        let (doc, bq1, ..) = doc! {
            root {
                bq1: blockquote {
                    paragraph
                }
                paragraph
            }
        };

        let bq = doc.node(bq1).unwrap();
        let steps = dissolve(&bq);
        assert!(steps.is_empty());
    }

    #[test]
    fn dissolve_list_item_promotes_children() {
        // bl1 > li1 > bl2 > li2 > paragraph
        // Plus a trailing paragraph at root so root content is valid.
        //
        // li1 has only [BulletList] — invalid (needs Paragraph first).
        // dissolve(li1) should:
        //   1. MoveNode bl2 from li1/0 to bl1/1
        //   2. RemoveSubtree li1
        //   3. bl2 doesn't match ListItem+ → recurse
        //     3a. MoveNode li2 from bl2/0 to bl1/new_pos
        //     3b. RemoveSubtree bl2
        //     3c. li2 IS ListItem → matches → done
        let (doc, bl1, li1, bl2, li2, ..) = doc! {
            root {
                bl1: bullet_list {
                    li1: list_item {
                        bl2: bullet_list {
                            li2: list_item {
                                paragraph
                            }
                        }
                    }
                }
                paragraph
            }
        };

        let outer_list_item = doc.node(li1).unwrap();
        let steps = dissolve(&outer_list_item);

        // At least 4 steps: MoveNode + RemoveSubtree for li1,
        // then MoveNode + RemoveSubtree for bl2
        assert!(
            steps.len() >= 4,
            "expected at least 4 steps, got {}",
            steps.len()
        );

        // Step 0: MoveNode bl2 from li1 to bl1
        match &steps[0] {
            Step::MoveNode {
                node_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => {
                assert_eq!(*node_id, bl2);
                assert_eq!(*old_parent, li1);
                assert_eq!(*old_index, 0);
                assert_eq!(*new_parent, bl1);
                assert_eq!(*new_index, 1); // node_index(0) + 1 + 0
            }
            _ => panic!("expected MoveNode for bl2"),
        }

        // Step 1: RemoveSubtree li1
        match &steps[1] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, bl1);
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, li1);
                assert!(matches!(subtree.node, Node::ListItem(_)));
                assert!(subtree.children.is_empty());
            }
            _ => panic!("expected RemoveSubtree for li1"),
        }

        // Step 2: MoveNode li2 from bl2 (recursive dissolve)
        match &steps[2] {
            Step::MoveNode {
                node_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => {
                assert_eq!(*node_id, li2);
                assert_eq!(*old_parent, bl2);
                assert_eq!(*old_index, 0);
                assert_eq!(*new_parent, bl1);
                assert_eq!(*new_index, 1); // bl2 is at index 0 in parent, so 0+1+0=1
            }
            _ => panic!("expected MoveNode for li2"),
        }

        // Step 3: RemoveSubtree bl2
        match &steps[3] {
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => {
                assert_eq!(*parent_id, bl1);
                assert_eq!(*index, 0);
                assert_eq!(subtree.id, bl2);
                assert!(matches!(subtree.node, Node::BulletList(_)));
                assert!(subtree.children.is_empty());
            }
            _ => panic!("expected RemoveSubtree for bl2"),
        }
    }

    #[test]
    fn dissolve_empty_node_returns_empty() {
        // Empty paragraph — content is (Text|HardBreak)*, PageBreak? — empty is valid
        let (doc, p1, ..) = doc! {
            root {
                p1: paragraph
            }
        };

        let para = doc.node(p1).unwrap();
        let steps = dissolve(&para);
        assert!(steps.is_empty());
    }
}

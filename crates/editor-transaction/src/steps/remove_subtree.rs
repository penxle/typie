use editor_crdt::{Dot, OrMapOp, RgaOp, TextOp};
use editor_model::{DocOp, ModifierType, Node, NodeId, Subtree};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::InsertSubtree {
        parent_id,
        index,
        subtree,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    parent_id: NodeId,
    _index: usize,
    subtree: &Subtree,
) -> Result<(), StepError> {
    let root_id = subtree.id;

    let (parent_children_dots, presence_dots_per_node) = {
        let parent_entry = batched
            .doc
            .get_entry(parent_id)
            .ok_or(StepError::NodeNotFound(parent_id))?;
        let parent_dots: Vec<Dot> = parent_entry
            .children
            .iter_with_dot()
            .filter(|&(_, &v)| v == root_id)
            .map(|(d, _)| d)
            .collect();
        if parent_dots.is_empty() {
            return Err(StepError::NodeNotFound(root_id));
        }

        let mut dfs_ids: Vec<NodeId> = Vec::new();
        collect_dfs_leaves_first(subtree, &mut dfs_ids);

        let mut per_node: Vec<(NodeId, Vec<Dot>)> = Vec::new();
        for sub_id in dfs_ids {
            let mut dots: Vec<Dot> = batched.doc.nodes_tags_for(&sub_id).copied().collect();
            dots.sort_unstable();
            dots.dedup();
            per_node.push((sub_id, dots));
        }
        (parent_dots, per_node)
    };

    let dfs_ids: Vec<NodeId> = presence_dots_per_node.iter().map(|(id, _)| *id).collect();
    for sub_id in &dfs_ids {
        let entry = match batched.doc.get_entry(*sub_id) {
            Some(e) => e.clone(),
            None => continue,
        };

        for (target_dot, _) in entry.children.iter_with_dot() {
            batched.apply(DocOp::Children {
                node_id: *sub_id,
                op: RgaOp::Remove {
                    observed: target_dot,
                },
            })?;
        }

        if let Node::Text(text_node) = &entry.node {
            for (target_entry, _) in text_node.text.iter_visible_entries() {
                batched.apply(DocOp::Text {
                    node_id: *sub_id,
                    op: TextOp::RemoveChar {
                        observed: target_entry,
                    },
                })?;
            }
        }

        let modifier_keys: Vec<ModifierType> = entry.modifiers.iter().map(|(k, _)| *k).collect();
        for key in modifier_keys {
            let mut observed: Vec<Dot> = entry.modifiers.tags_for(&key).copied().collect();
            observed.sort_unstable();
            observed.dedup();
            if !observed.is_empty() {
                batched.apply(DocOp::Modifier {
                    node_id: *sub_id,
                    op: OrMapOp::Unset { observed },
                })?;
            }
        }
    }

    for (sub_id, observed) in presence_dots_per_node {
        if observed.is_empty() {
            continue;
        }
        batched.apply(DocOp::Presence {
            node_id: sub_id,
            op: OrMapOp::Unset { observed },
        })?;
    }

    for target in parent_children_dots {
        batched.apply(DocOp::Children {
            node_id: parent_id,
            op: RgaOp::Remove { observed: target },
        })?;
    }

    validations.push(Validation::Node(parent_id));
    Ok(())
}

fn collect_dfs_leaves_first(subtree: &Subtree, ids: &mut Vec<NodeId>) {
    for child in &subtree.children {
        collect_dfs_leaves_first(child, ids);
    }
    ids.push(subtree.id);
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::Transaction;

    #[test]
    fn remove_fold_title_content_violation() {
        let (state, ft1, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title {
                            t1: text("Title")
                        }
                        fold_content {
                            paragraph
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(tr.remove_subtree(ft1).is_err());
    }

    #[test]
    fn remove_last_list_item_content_violation() {
        let (state, li1, ..) = state! {
            doc {
                root {
                    bullet_list {
                        li1: list_item {
                            paragraph {
                                t1: text("A")
                            }
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(tr.remove_subtree(li1).is_err());
    }
}

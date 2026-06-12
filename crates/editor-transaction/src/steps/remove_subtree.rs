use editor_crdt::{Dot, OrMapOp, RgaOp, TextOp};
use editor_model::{DocOp, ModifierType, Node, NodeId, Subtree};
use editor_state::BatchedState;

use crate::{Step, StepEffect, StepError, TextRemoveEffect, Validation};

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
    effect: &mut StepEffect,
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
            let mut entries = Vec::new();
            let mut text = String::new();
            for (target_entry, ch) in text_node.text.iter_visible_entries() {
                entries.push(target_entry);
                text.push(ch);
                batched.apply(DocOp::Text {
                    node_id: *sub_id,
                    op: TextOp::RemoveChar {
                        observed: target_entry,
                    },
                })?;
            }
            if !entries.is_empty() {
                effect.text_removes.push(TextRemoveEffect {
                    node_id: *sub_id,
                    offset: 0,
                    entries,
                    text,
                });
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
    use editor_model::{NodeId, Subtree};

    use crate::{Step, Transaction};

    #[test]
    fn remove_subtree_apply_reports_removed_text_entries() {
        let (state, p1, t1, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hi")
                    }
                }
            }
            selection: (t1, 0)
        };
        let removed_entries = state
            .doc
            .text_view(t1)
            .unwrap()
            .visible_entries()
            .map(|(entry, _)| entry)
            .collect::<Vec<_>>();
        let subtree = Subtree::capture(&state.doc, p1).expect("fixture subtree");
        let step = Step::RemoveSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree,
        };

        let output = step.apply(&state).unwrap();
        let remove = output
            .effect
            .text_removes
            .iter()
            .find(|effect| effect.node_id == t1)
            .expect("removed text effect");

        assert_eq!(remove.offset, 0);
        assert_eq!(remove.text, "Hi");
        assert_eq!(remove.entries, removed_entries);
    }

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

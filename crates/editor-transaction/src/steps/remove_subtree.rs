use editor_model::{Doc, NodeId, Subtree};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput, Validation};

pub(crate) fn build_mapping(parent_id: NodeId, index: usize, subtree: &Subtree) -> Mapping {
    let mut m = Mapping::single(MapAction::Remove {
        parent: parent_id,
        start: index,
        count: 1,
    });
    for node in subtree.node_ids() {
        m.push(MapAction::NodeDeleted { node });
    }
    m
}

pub(crate) fn apply(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<StepOutput, StepError> {
    let node_id = subtree.id;
    state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index >= parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    let node_ids = collect_doc_descendants(&state.doc, subtree.id);
    let mut doc = state.doc.clone();
    for id in node_ids {
        doc = doc.remove_node(id);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.remove(index);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        mapping: build_mapping(parent_id, index, subtree),
        validations: vec![Validation::Node(parent_id)],
    })
}

// Walk doc, not step.subtree: concurrent steps may have inserted descendants
// after the subtree was captured.
fn collect_doc_descendants(doc: &Doc, root: NodeId) -> Vec<NodeId> {
    let mut ids = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        ids.push(id);
        if let Some(entry) = doc.get_entry(id) {
            for child in &entry.children {
                stack.push(*child);
            }
        }
    }
    ids
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::InsertSubtree {
        parent_id,
        index,
        subtree,
    }
}

pub(crate) fn rebase_against(
    parent_id: NodeId,
    mut index: usize,
    subtree: &Subtree,
    mapping: &Mapping,
) -> Vec<Step> {
    let target_id = subtree.id;
    for action in mapping.actions() {
        match *action {
            MapAction::NodeDeleted { node } if node == parent_id || node == target_id => {
                return vec![];
            }
            MapAction::Insert {
                parent,
                start,
                count,
                subtree_id: _,
            } if parent == parent_id => {
                if start <= index {
                    index += count;
                }
            }
            MapAction::Remove {
                parent,
                start,
                count,
            } if parent == parent_id => {
                if start + count <= index {
                    index -= count;
                }
            }
            _ => {}
        }
    }
    vec![Step::RemoveSubtree {
        parent_id,
        index,
        subtree: subtree.clone(),
    }]
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;

    use super::*;
    use crate::Transaction;
    use crate::test_utils::DocTestExt;

    fn make_fold_state() -> (State, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let (state, f1, ft1, t1, fc1, p1) = state! {
            doc {
                root {
                    f1: fold {
                        ft1: fold_title {
                            t1: text("Title")
                        }
                        fc1: fold_content {
                            p1: paragraph
                        }
                    }
                }
            }
            selection: (t1, 0)
        };
        (state, f1, ft1, fc1, t1, p1)
    }

    #[test]
    fn remove_fold_title_content_violation() {
        let (state, _, ft1, ..) = make_fold_state();

        let mut tr = Transaction::new(&state);
        let result = tr.remove_subtree(ft1);

        assert!(result.is_err());
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
        let result = tr.remove_subtree(li1);

        assert!(result.is_err());
    }

    #[test]
    fn build_mapping_yields_remove_plus_node_deleted_for_each_node() {
        let parent = NodeId::new();
        let root_id = NodeId::new();
        let child_id = NodeId::new();
        let subtree =
            Subtree::leaf(root_id, Node::Paragraph(ParagraphNode::default())).with_children(vec![
                Subtree::leaf(child_id, Node::Text(TextNode { text: "x".into() })),
            ]);

        let m = build_mapping(parent, 0, &subtree);
        let actions = m.actions();
        assert_eq!(actions.len(), 3);
        assert_eq!(
            actions[0],
            MapAction::Remove {
                parent,
                start: 0,
                count: 1,
            }
        );
        assert_eq!(actions[1], MapAction::NodeDeleted { node: root_id });
        assert_eq!(actions[2], MapAction::NodeDeleted { node: child_id });
    }

    fn paragraph_subtree(id: NodeId) -> Subtree {
        Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()))
    }

    #[test]
    fn rebase_swallowed_by_node_deleted_on_parent() {
        let parent = NodeId::new();
        let mapping = Mapping::single(MapAction::NodeDeleted { node: parent });
        let subtree = paragraph_subtree(NodeId::new());
        let result = rebase_against(parent, 0, &subtree, &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_swallowed_when_target_node_deleted() {
        let parent = NodeId::new();
        let target = NodeId::new();
        let subtree = paragraph_subtree(target);
        let mapping = Mapping::single(MapAction::NodeDeleted { node: target });
        let result = rebase_against(parent, 0, &subtree, &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_insert_at_lower_index_shifts() {
        let p = NodeId::new();
        let mapping = Mapping::single(MapAction::Insert {
            parent: p,
            start: 0,
            count: 1,
            subtree_id: NodeId::new(),
        });
        let subtree = paragraph_subtree(NodeId::new());
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::RemoveSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 3);
        } else {
            panic!("expected single RemoveSubtree, got {:?}", result);
        }
    }

    #[test]
    fn rebase_insert_at_same_index_shifts() {
        let p = NodeId::new();
        let mapping = Mapping::single(MapAction::Insert {
            parent: p,
            start: 2,
            count: 1,
            subtree_id: NodeId::new(),
        });
        let subtree = paragraph_subtree(NodeId::new());
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::RemoveSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 3);
        } else {
            panic!("expected single RemoveSubtree, got {:?}", result);
        }
    }

    #[test]
    fn rebase_insert_at_higher_index_no_shift() {
        let p = NodeId::new();
        let mapping = Mapping::single(MapAction::Insert {
            parent: p,
            start: 5,
            count: 1,
            subtree_id: NodeId::new(),
        });
        let subtree = paragraph_subtree(NodeId::new());
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::RemoveSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 2);
        } else {
            panic!("expected single RemoveSubtree, got {:?}", result);
        }
    }

    #[test]
    fn rebase_remove_at_same_index_swallow() {
        let p = NodeId::new();
        let id = NodeId::new();
        let subtree = paragraph_subtree(id);
        let mapping = Mapping::single(MapAction::Remove {
            parent: p,
            start: 0,
            count: 1,
        })
        .compose(&Mapping::single(MapAction::NodeDeleted { node: id }));
        let result = rebase_against(p, 0, &subtree, &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_remove_at_lower_index_shifts_back() {
        let p = NodeId::new();
        let mapping = Mapping::single(MapAction::Remove {
            parent: p,
            start: 0,
            count: 1,
        });
        let subtree = paragraph_subtree(NodeId::new());
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::RemoveSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 1);
        } else {
            panic!("expected single RemoveSubtree, got {:?}", result);
        }
    }

    #[test]
    fn apply_removes_descendants_added_after_subtree_capture() {
        let (state, p1, t1) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("original")
                    }
                }
            }
            selection: (t1, 0)
        };

        let captured = Subtree::capture(&state.doc, p1).unwrap();

        let new_text_id = NodeId::new();
        let inserted_state = Step::InsertSubtree {
            parent_id: p1,
            index: 0,
            subtree: Subtree::leaf(new_text_id, Node::Text(TextNode { text: "new".into() })),
        }
        .apply(&state)
        .unwrap()
        .state;

        let result = Step::RemoveSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree: captured,
        }
        .apply(&inserted_state)
        .unwrap()
        .state;

        assert!(!result.has_node(t1));
        assert!(!result.has_node(new_text_id));
        assert!(!result.has_node(p1));
    }
}

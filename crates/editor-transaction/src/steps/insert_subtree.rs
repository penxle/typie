use editor_model::{NodeId, Subtree};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput, Validation};

pub(crate) fn build_mapping(parent_id: NodeId, index: usize, subtree_id: NodeId) -> Mapping {
    Mapping::single(MapAction::Insert {
        parent: parent_id,
        start: index,
        count: 1,
        subtree_id,
    })
}

pub(crate) fn apply(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<StepOutput, StepError> {
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index > parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    let entries = subtree.clone().into_entries(parent_id);
    let mut doc = state.doc.clone();
    for (id, entry) in entries {
        doc = doc.insert_node(id, entry);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.insert(index, subtree.id);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        mapping: build_mapping(parent_id, index, subtree.id),
        validations: vec![Validation::Node(parent_id), Validation::Subtree(subtree.id)],
    })
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
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
    let local_id = subtree.id;
    for action in mapping.actions() {
        match *action {
            MapAction::NodeDeleted { node } if node == parent_id || node == local_id => {
                return vec![];
            }
            MapAction::Insert {
                parent,
                start,
                count,
                subtree_id: against_id,
            } if parent == parent_id => {
                if start < index {
                    index += count;
                } else if start == index && against_id < local_id {
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
    vec![Step::InsertSubtree {
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

    #[test]
    fn build_mapping_yields_insert_action_with_subtree_id() {
        let parent = NodeId::new();
        let id = NodeId::new();
        let m = build_mapping(parent, 2, id);
        assert_eq!(
            m.actions(),
            &[MapAction::Insert {
                parent,
                start: 2,
                count: 1,
                subtree_id: id,
            }]
        );
    }

    #[test]
    fn insert_subtree_apply() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));

        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert!(new_state.has_node(new_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().len(), 2);
        assert_eq!(new_state.node(NodeId::ROOT).entry().children[1], new_id);
        assert_eq!(new_state.node(new_id).entry().parent, Some(NodeId::ROOT));
    }

    #[test]
    fn insert_subtree_index_out_of_bounds() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 99,
            subtree: Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default())),
        };

        assert!(step.apply(&state).is_err());
    }

    #[test]
    fn insert_subtree_content_violation() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, Node::Text(TextNode { text: "Bad".into() }));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(NodeId::ROOT, 0, subtree);

        assert!(result.is_err());
    }

    #[test]
    fn insert_subtree_context_violation() {
        let (state, tc1, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            tc1: table_cell {
                                paragraph {
                                    t1: text("Hi")
                                }
                            }
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_table_id = NodeId::new();
        let subtree = Subtree::leaf(new_table_id, Node::Table(TableNode::default()));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(tc1, 0, subtree);

        assert!(result.is_err());
    }

    #[test]
    fn insert_then_remove_roundtrip() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;
        assert!(!state3.has_node(new_id));
        assert_eq!(state3.node(NodeId::ROOT).children().len(), 1);
    }

    #[test]
    fn insert_empty_container_fails_content_validation() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let list_id = NodeId::new();
        // BulletList requires at least one ListItem child — empty should fail
        let subtree = Subtree::leaf(list_id, Node::BulletList(BulletListNode {}));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(NodeId::ROOT, 1, subtree);

        assert!(result.is_err());
    }

    fn paragraph_subtree(id: NodeId) -> Subtree {
        Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()))
    }

    fn ordered_ids() -> (NodeId, NodeId) {
        let a = NodeId::new();
        let b = NodeId::new();
        if a < b { (a, b) } else { (b, a) }
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
        if let [Step::InsertSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 3);
        } else {
            panic!("expected single InsertSubtree, got {:?}", result);
        }
    }

    #[test]
    fn rebase_insert_at_same_index_smaller_against_id_shifts_local() {
        let p = NodeId::new();
        let (small_id, large_id) = ordered_ids();
        let mapping = Mapping::single(MapAction::Insert {
            parent: p,
            start: 2,
            count: 1,
            subtree_id: small_id,
        });
        let subtree = paragraph_subtree(large_id);
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::InsertSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 3);
        } else {
            panic!("expected single InsertSubtree, got {:?}", result);
        }
    }

    #[test]
    fn rebase_insert_at_same_index_larger_against_id_keeps_local() {
        let p = NodeId::new();
        let (small_id, large_id) = ordered_ids();
        let mapping = Mapping::single(MapAction::Insert {
            parent: p,
            start: 2,
            count: 1,
            subtree_id: large_id,
        });
        let subtree = paragraph_subtree(small_id);
        let result = rebase_against(p, 2, &subtree, &mapping);
        if let [Step::InsertSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 2);
        } else {
            panic!("expected single InsertSubtree, got {:?}", result);
        }
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
        if let [Step::InsertSubtree { index, .. }] = result.as_slice() {
            assert_eq!(*index, 1);
        } else {
            panic!("expected single InsertSubtree, got {:?}", result);
        }
    }

    #[test]
    fn insert_valid_subtree_succeeds() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let list_id = NodeId::new();
        let item_id = NodeId::new();
        let para_id = NodeId::new();
        let subtree =
            Subtree::leaf(list_id, Node::BulletList(BulletListNode {})).with_children(vec![
                Subtree::leaf(item_id, Node::ListItem(ListItemNode {})).with_children(vec![
                    Subtree::leaf(para_id, Node::Paragraph(ParagraphNode::default())),
                ]),
            ]);

        // Insert before existing Paragraph so the trailing Paragraph requirement is satisfied
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        assert!(new_state.has_node(list_id));
        assert!(new_state.has_node(item_id));
        assert!(new_state.has_node(para_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().len(), 2);
    }
}

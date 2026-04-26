use editor_model::NodeId;

use crate::{Step, StepScope, StepType};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Conflict {
    #[error("structural step transform unsupported (scope A): local={local:?} against={against:?}")]
    UnsupportedStructural { local: StepType, against: StepType },

    #[error("non-syncable step passed to transform: {ty:?}")]
    NonSyncable { ty: StepType },
}

pub(crate) fn transform(local: &Step, against: &Step) -> Result<Vec<Step>, Conflict> {
    if !local.is_syncable() {
        return Err(Conflict::NonSyncable {
            ty: StepType::from(local),
        });
    }
    if !against.is_syncable() {
        return Err(Conflict::NonSyncable {
            ty: StepType::from(against),
        });
    }

    match local {
        Step::InsertText {
            node_id,
            offset,
            text,
        } => crate::steps::insert_text::transform_against(*node_id, *offset, text, against),
        Step::RemoveText {
            node_id,
            offset,
            text,
        } => crate::steps::remove_text::transform_against(*node_id, *offset, text, against),
        Step::InsertSubtree {
            parent_id,
            index,
            subtree,
        } => crate::steps::insert_subtree::transform_against(*parent_id, *index, subtree, against),
        Step::RemoveSubtree {
            parent_id,
            index,
            subtree,
        } => crate::steps::remove_subtree::transform_against(*parent_id, *index, subtree, against),
        Step::AddModifier { node_id, modifier } => {
            crate::steps::add_modifier::transform_against(*node_id, modifier, against)
        }
        Step::RemoveModifier { node_id, modifier } => {
            crate::steps::remove_modifier::transform_against(*node_id, modifier, against)
        }
        Step::SetModifiers {
            node_id,
            old_modifiers,
            new_modifiers,
        } => crate::steps::set_modifiers::transform_against(
            *node_id,
            old_modifiers,
            new_modifiers,
            against,
        ),
        Step::SetNode {
            node_id,
            old_node,
            new_node,
        } => crate::steps::set_node::transform_against(*node_id, old_node, new_node, against),
        Step::SetDocumentAttrs { old, new } => {
            crate::steps::set_document_attrs::transform_against(old, new, against)
        }
        Step::MoveNode {
            node_id,
            old_parent,
            old_index,
            new_parent,
            new_index,
        } => crate::steps::move_node::transform_against(
            *node_id,
            *old_parent,
            *old_index,
            *new_parent,
            *new_index,
            against,
        ),
        Step::SplitNode {
            node_id,
            offset,
            new_node_id,
        } => crate::steps::split_node::transform_against(*node_id, *offset, *new_node_id, against),
        Step::MergeNode {
            node_id,
            target_id,
            offset,
        } => crate::steps::merge_node::transform_against(*node_id, *target_id, *offset, against),
        _ => unimplemented!(
            "transform not yet implemented for local={:?} against={:?}",
            StepType::from(local),
            StepType::from(against)
        ),
    }
}

pub(crate) fn transform_default(local: Step, against: &Step) -> Result<Vec<Step>, Conflict> {
    let local_scope = local.scope();
    let against_scope = against.scope();

    if scopes_conflict(&local_scope, &against_scope) {
        return Err(Conflict::UnsupportedStructural {
            local: StepType::from(&local),
            against: StepType::from(against),
        });
    }

    if let Step::RemoveSubtree { subtree, .. } = against {
        if scope_inside_subtree(&local_scope, subtree) {
            return Ok(vec![]);
        }
    }

    Ok(vec![local])
}

fn scopes_conflict(local: &StepScope, against: &StepScope) -> bool {
    let local_structural = matches!(local, StepScope::Structural(_));
    let against_structural = matches!(against, StepScope::Structural(_));
    if !local_structural && !against_structural {
        return false;
    }

    let local_ids = scope_node_ids(local);
    let against_ids = scope_node_ids(against);
    local_ids.iter().any(|id| against_ids.contains(id))
}

fn scope_node_ids(scope: &StepScope) -> smallvec::SmallVec<[NodeId; 2]> {
    use smallvec::smallvec;
    match scope {
        StepScope::Node(id) | StepScope::Children { parent: id } => smallvec![*id],
        StepScope::Structural(ids) => ids.clone(),
        StepScope::Document | StepScope::Local => smallvec![],
    }
}

fn scope_inside_subtree(scope: &StepScope, subtree: &editor_model::Subtree) -> bool {
    match scope {
        StepScope::Node(id) | StepScope::Children { parent: id } => subtree.contains_node(*id),
        StepScope::Structural(ids) => ids.iter().any(|id| subtree.contains_node(*id)),
        StepScope::Document | StepScope::Local => false,
    }
}

pub fn transform_many(local: &[Step], against: &[Step]) -> Result<Vec<Step>, Conflict> {
    let mut current: Vec<Step> = local.to_vec();
    for a in against {
        let mut next = Vec::with_capacity(current.len());
        for c in &current {
            next.extend(transform(c, a)?);
        }
        current = next;
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_syncable_local_returns_error() {
        let local = Step::SetComposition {
            old: None,
            new: None,
        };
        let against = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        };
        assert!(matches!(
            transform(&local, &against),
            Err(Conflict::NonSyncable { .. })
        ));
    }

    #[test]
    fn non_syncable_against_returns_error() {
        let local = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        };
        let against = Step::SetComposition {
            old: None,
            new: None,
        };
        assert!(matches!(
            transform(&local, &against),
            Err(Conflict::NonSyncable { .. })
        ));
    }

    #[test]
    fn transform_many_empty_inputs_returns_empty() {
        assert_eq!(transform_many(&[], &[]).unwrap(), Vec::<Step>::new());
    }

    #[test]
    fn insert_text_invalidated_by_remove_subtree_containing_anchor() {
        let parent = NodeId::new();
        let removed = NodeId::new();
        let text_node = NodeId::new();

        let subtree = editor_model::Subtree::leaf(
            removed,
            editor_model::Node::Paragraph(editor_model::ParagraphNode::default()),
        )
        .with_children(vec![editor_model::Subtree::leaf(
            text_node,
            editor_model::Node::Text(editor_model::TextNode {
                text: "hello".into(),
            }),
        )]);

        let local = Step::InsertText {
            node_id: text_node,
            offset: 0,
            text: "x".into(),
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 0,
            subtree,
        };
        assert_eq!(transform(&local, &against).unwrap(), Vec::<Step>::new(),);
    }

    #[test]
    fn modifier_step_invalidated_by_remove_subtree_containing_anchor() {
        let parent = NodeId::new();
        let removed = NodeId::new();
        let subtree = editor_model::Subtree::leaf(
            removed,
            editor_model::Node::Paragraph(editor_model::ParagraphNode::default()),
        );
        let local = Step::AddModifier {
            node_id: removed,
            modifier: editor_model::Modifier::Bold,
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 0,
            subtree,
        };
        assert_eq!(transform(&local, &against).unwrap(), Vec::<Step>::new(),);
    }

    #[test]
    fn unrelated_steps_commute() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let local = Step::AddModifier {
            node_id: n1,
            modifier: editor_model::Modifier::Bold,
        };
        let against = Step::AddModifier {
            node_id: n2,
            modifier: editor_model::Modifier::Italic,
        };
        assert_eq!(transform(&local, &against).unwrap(), vec![local],);
    }

    #[test]
    fn split_node_against_insert_text_same_anchor_returns_conflict() {
        let n = NodeId::new();
        let new_id = NodeId::new();
        let local = Step::SplitNode {
            node_id: n,
            offset: 3,
            new_node_id: new_id,
        };
        let against = Step::InsertText {
            node_id: n,
            offset: 1,
            text: "x".into(),
        };
        assert!(matches!(
            transform(&local, &against),
            Err(Conflict::UnsupportedStructural { .. })
        ));
    }

    #[test]
    fn insert_text_against_split_node_same_anchor_returns_conflict() {
        let n = NodeId::new();
        let new_id = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 1,
            text: "x".into(),
        };
        let against = Step::SplitNode {
            node_id: n,
            offset: 3,
            new_node_id: new_id,
        };
        assert!(matches!(
            transform(&local, &against),
            Err(Conflict::UnsupportedStructural { .. })
        ));
    }

    #[test]
    fn split_node_against_unrelated_insert_commutes() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let new_id = NodeId::new();
        let local = Step::SplitNode {
            node_id: n1,
            offset: 3,
            new_node_id: new_id,
        };
        let against = Step::InsertText {
            node_id: n2,
            offset: 0,
            text: "x".into(),
        };
        assert_eq!(transform(&local, &against).unwrap(), vec![local],);
    }

    #[test]
    fn move_node_against_move_node_intersecting_parents_returns_conflict() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let p_a = NodeId::new();
        let p_b = NodeId::new();
        let p_c = NodeId::new();
        let local = Step::MoveNode {
            node_id: n1,
            old_parent: p_a,
            old_index: 0,
            new_parent: p_b,
            new_index: 0,
        };
        let against = Step::MoveNode {
            node_id: n2,
            old_parent: p_b,
            old_index: 1,
            new_parent: p_c,
            new_index: 0,
        };
        assert!(matches!(
            transform(&local, &against),
            Err(Conflict::UnsupportedStructural { .. })
        ));
    }

    #[test]
    fn transform_many_fold_two_against_steps() {
        let n = NodeId::new();
        let local = vec![Step::InsertText {
            node_id: n,
            offset: 5,
            text: "ab".into(),
        }];
        let against = vec![
            Step::InsertText {
                node_id: n,
                offset: 1,
                text: "X".into(),
            },
            Step::InsertText {
                node_id: n,
                offset: 0,
                text: "YY".into(),
            },
        ];
        assert_eq!(
            transform_many(&local, &against).unwrap(),
            vec![Step::InsertText {
                node_id: n,
                offset: 8,
                text: "ab".into()
            }],
        );
    }

    #[test]
    fn transform_many_propagates_conflict() {
        let n = NodeId::new();
        let new_id = NodeId::new();
        let local = vec![Step::InsertText {
            node_id: n,
            offset: 0,
            text: "x".into(),
        }];
        let against = vec![Step::SplitNode {
            node_id: n,
            offset: 0,
            new_node_id: new_id,
        }];
        assert!(matches!(
            transform_many(&local, &against),
            Err(Conflict::UnsupportedStructural { .. })
        ));
    }

    #[test]
    fn transform_many_handles_branching() {
        let n = NodeId::new();
        let local = vec![Step::RemoveText {
            node_id: n,
            offset: 2,
            text: "abcde".into(),
        }];
        let against = vec![Step::InsertText {
            node_id: n,
            offset: 4,
            text: "XY".into(),
        }];
        let out = transform_many(&local, &against).unwrap();
        assert_eq!(out.len(), 2);
    }
}

#[cfg(test)]
mod ot_invariant {
    use super::*;
    use crate::test_utils::proptest::{TransformScenario, transform_scenario};
    use editor_state::State;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn ot_invariant_holds(scenario in transform_scenario()) {
            let TransformScenario { state, a, b, .. } = scenario;

            let b_primes = match transform(&b, &a) {
                Ok(v) => v,
                Err(Conflict::UnsupportedStructural { .. }) => return Ok(()),
                Err(e) => return Err(TestCaseError::reject(format!("unexpected error: {e}"))),
            };
            let a_primes = match transform(&a, &b) {
                Ok(v) => v,
                Err(Conflict::UnsupportedStructural { .. }) => return Ok(()),
                Err(e) => return Err(TestCaseError::reject(format!("unexpected error: {e}"))),
            };

            let after_a = a.apply(&state).unwrap().state;
            let after_b = b.apply(&state).unwrap().state;
            let lhs = apply_seq(&after_a, &b_primes);
            let rhs = apply_seq(&after_b, &a_primes);

            prop_assert_eq!(lhs.doc, rhs.doc);
        }
    }

    fn apply_seq(state: &State, steps: &[Step]) -> State {
        steps
            .iter()
            .fold(state.clone(), |s, st| st.apply(&s).unwrap().state)
    }
}

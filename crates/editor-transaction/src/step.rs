use editor_macros::ffi;
use editor_model::{Modifier, ModifierType, Node, NodeId, Subtree};
use editor_state::{Composition, PendingModifiers, Selection, State};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use strum::{EnumDiscriminants, IntoStaticStr};

use crate::StepError;
use crate::steps;

#[derive(Clone, Debug)]
pub enum Validation {
    /// Validate this node's content expression (children satisfy content)
    Node(NodeId),
    /// Validate content + context for this node and all descendants
    Subtree(NodeId),
    /// Validate modifier is allowed at this node's context
    Modifier(NodeId, ModifierType),
}

pub struct StepOutput {
    pub state: State,
    pub validations: Vec<Validation>,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(StepType))]
#[strum_discriminants(derive(Hash, Serialize, Deserialize, IntoStaticStr))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    InsertText {
        node_id: NodeId,
        offset: usize,
        text: String,
    },
    RemoveText {
        node_id: NodeId,
        offset: usize,
        text: String,
    },
    InsertSubtree {
        parent_id: NodeId,
        index: usize,
        subtree: Subtree,
    },
    RemoveSubtree {
        parent_id: NodeId,
        index: usize,
        subtree: Subtree,
    },
    MoveNode {
        node_id: NodeId,
        old_parent: NodeId,
        old_index: usize,
        new_parent: NodeId,
        new_index: usize,
    },
    SplitNode {
        node_id: NodeId,
        offset: usize,
        new_node_id: NodeId,
    },
    MergeNode {
        node_id: NodeId,
        target_id: NodeId,
        offset: usize,
    },
    SetNode {
        node_id: NodeId,
        old_node: Node,
        new_node: Node,
    },
    AddModifier {
        node_id: NodeId,
        modifier: Modifier,
    },
    RemoveModifier {
        node_id: NodeId,
        modifier: Modifier,
    },
    SetSelection {
        old: Selection,
        new: Selection,
    },
    SetPendingModifiers {
        old: PendingModifiers,
        new: PendingModifiers,
    },
    SetModifiers {
        node_id: NodeId,
        old_modifiers: Vec<Modifier>,
        new_modifiers: Vec<Modifier>,
    },
    SetComposition {
        old: Option<Composition>,
        new: Option<Composition>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepScope {
    Node(NodeId),
    Children { parent: NodeId },
    Structural(SmallVec<[NodeId; 2]>),
    Local,
}

impl Step {
    pub fn is_doc_step(&self) -> bool {
        !matches!(
            self,
            Step::SetSelection { .. }
                | Step::SetPendingModifiers { .. }
                | Step::SetComposition { .. }
        )
    }

    pub fn is_selection_step(&self) -> bool {
        matches!(self, Step::SetSelection { .. })
    }

    pub fn is_pending_modifiers_step(&self) -> bool {
        matches!(self, Step::SetPendingModifiers { .. })
    }

    pub fn is_commitable(&self) -> bool {
        !matches!(
            self,
            Step::SetSelection { .. }
                | Step::SetPendingModifiers { .. }
                | Step::SetComposition { .. }
        )
    }

    pub fn scope(&self) -> StepScope {
        match self {
            Step::InsertText { node_id, .. }
            | Step::RemoveText { node_id, .. }
            | Step::SetNode { node_id, .. }
            | Step::AddModifier { node_id, .. }
            | Step::RemoveModifier { node_id, .. }
            | Step::SetModifiers { node_id, .. } => StepScope::Node(*node_id),

            Step::InsertSubtree { parent_id, .. } | Step::RemoveSubtree { parent_id, .. } => {
                StepScope::Children { parent: *parent_id }
            }

            Step::SplitNode { node_id, .. } => StepScope::Structural(smallvec![*node_id]),
            Step::MergeNode {
                node_id, target_id, ..
            } => StepScope::Structural(smallvec![*node_id, *target_id]),
            Step::MoveNode {
                old_parent,
                new_parent,
                ..
            } => StepScope::Structural(smallvec![*old_parent, *new_parent]),

            Step::SetSelection { .. }
            | Step::SetPendingModifiers { .. }
            | Step::SetComposition { .. } => StepScope::Local,
        }
    }

    pub fn affected_node_ids(&self) -> Vec<NodeId> {
        match self {
            Step::InsertText { node_id, .. }
            | Step::RemoveText { node_id, .. }
            | Step::AddModifier { node_id, .. }
            | Step::RemoveModifier { node_id, .. }
            | Step::SetModifiers { node_id, .. }
            | Step::SetNode { node_id, .. } => vec![*node_id],
            Step::InsertSubtree {
                parent_id, subtree, ..
            } => {
                let mut ids = vec![*parent_id];
                ids.extend(subtree.all_ids());
                ids
            }
            // RemoveSubtree affects only the parent (its children list shrinks).
            // Removed nodes are not in the new doc, so listing them would cause
            // derive_objects_for_path to panic when looking up missing entries.
            Step::RemoveSubtree { parent_id, .. } => vec![*parent_id],
            Step::SplitNode {
                node_id,
                new_node_id,
                ..
            } => vec![*node_id, *new_node_id],
            // MergeNode collapses node_id into target_id; node_id is removed from the
            // new doc, so listing it would cause derive_objects_for_path to panic.
            Step::MergeNode { target_id, .. } => vec![*target_id],
            Step::MoveNode {
                node_id,
                old_parent,
                new_parent,
                ..
            } => vec![*node_id, *old_parent, *new_parent],
            Step::SetSelection { .. }
            | Step::SetPendingModifiers { .. }
            | Step::SetComposition { .. } => vec![],
        }
    }

    pub fn apply(&self, state: &State) -> Result<StepOutput, StepError> {
        match self {
            Step::InsertText {
                node_id,
                offset,
                text,
            } => steps::insert_text::apply(state, *node_id, *offset, text),
            Step::RemoveText {
                node_id,
                offset,
                text,
            } => steps::remove_text::apply(state, *node_id, *offset, text),
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => steps::insert_subtree::apply(state, *parent_id, *index, subtree),
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => steps::remove_subtree::apply(state, *parent_id, *index, subtree),
            Step::MoveNode {
                node_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => steps::move_node::apply(
                state,
                *node_id,
                *old_parent,
                *old_index,
                *new_parent,
                *new_index,
            ),
            Step::SplitNode {
                node_id,
                offset,
                new_node_id,
            } => steps::split_node::apply(state, *node_id, *offset, *new_node_id),
            Step::MergeNode {
                node_id,
                target_id,
                offset: _,
            } => steps::merge_node::apply(state, *node_id, *target_id),
            Step::SetNode {
                node_id,
                old_node: _,
                new_node,
            } => steps::set_node::apply(state, *node_id, new_node),
            Step::AddModifier { node_id, modifier } => {
                steps::add_modifier::apply(state, *node_id, modifier)
            }
            Step::RemoveModifier { node_id, modifier } => {
                steps::remove_modifier::apply(state, *node_id, modifier)
            }
            Step::SetSelection { old: _, new: sel } => steps::set_selection::apply(state, sel),
            Step::SetPendingModifiers { old: _, new } => {
                steps::set_pending_modifiers::apply(state, new)
            }
            Step::SetModifiers {
                node_id,
                old_modifiers: _,
                new_modifiers,
            } => steps::set_modifiers::apply(state, *node_id, new_modifiers),
            Step::SetComposition { old: _, new } => steps::set_composition::apply(state, new),
        }
    }

    pub fn inverse(&self) -> Step {
        match self {
            Step::InsertText {
                node_id,
                offset,
                text,
            } => steps::insert_text::inverse(*node_id, *offset, text.clone()),
            Step::RemoveText {
                node_id,
                offset,
                text,
            } => steps::remove_text::inverse(*node_id, *offset, text.clone()),
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => steps::insert_subtree::inverse(*parent_id, *index, subtree.clone()),
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => steps::remove_subtree::inverse(*parent_id, *index, subtree.clone()),
            Step::MoveNode {
                node_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => steps::move_node::inverse(
                *node_id,
                *old_parent,
                *old_index,
                *new_parent,
                *new_index,
            ),
            Step::SplitNode {
                node_id,
                offset,
                new_node_id,
            } => steps::split_node::inverse(*node_id, *offset, *new_node_id),
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => steps::merge_node::inverse(*node_id, *target_id, *offset),
            Step::SetNode {
                node_id,
                old_node,
                new_node,
            } => steps::set_node::inverse(*node_id, old_node.clone(), new_node.clone()),
            Step::AddModifier { node_id, modifier } => {
                steps::add_modifier::inverse(*node_id, modifier.clone())
            }
            Step::RemoveModifier { node_id, modifier } => {
                steps::remove_modifier::inverse(*node_id, modifier.clone())
            }
            Step::SetSelection { old, new } => steps::set_selection::inverse(*old, *new),
            Step::SetPendingModifiers { old, new } => {
                steps::set_pending_modifiers::inverse(old.clone(), new.clone())
            }
            Step::SetModifiers {
                node_id,
                old_modifiers,
                new_modifiers,
            } => steps::set_modifiers::inverse(
                *node_id,
                old_modifiers.clone(),
                new_modifiers.clone(),
            ),
            Step::SetComposition { old, new } => steps::set_composition::inverse(*old, *new),
        }
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn step_serde_roundtrip_insert_text() {
        let step = Step::InsertText {
            node_id: NodeId::new(),
            offset: 3,
            text: "hello".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: Step = serde_json::from_str(&json).unwrap();
        assert_eq!(step, back);
    }

    #[test]
    fn step_serde_internally_tagged() {
        let step = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"type\":\"insert_text\""));
    }

    #[test]
    fn step_type_from_step() {
        let step = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: String::new(),
        };
        assert_eq!(StepType::from(&step), StepType::InsertText);
    }
}

#[cfg(test)]
mod predicate_tests {
    use super::*;
    use editor_state::Position;

    #[test]
    fn is_commitable_true_for_insert_text() {
        let step = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        };
        assert!(step.is_commitable());
    }

    #[test]
    fn is_commitable_false_for_set_selection() {
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let step = Step::SetSelection { old: sel, new: sel };
        assert!(!step.is_commitable());
    }

    #[test]
    fn is_commitable_false_for_set_composition() {
        let step = Step::SetComposition {
            old: None,
            new: None,
        };
        assert!(!step.is_commitable());
    }

    #[test]
    fn is_commitable_false_for_set_pending_modifiers() {
        let step = Step::SetPendingModifiers {
            old: PendingModifiers::new(),
            new: PendingModifiers::new(),
        };
        assert!(!step.is_commitable());
    }

    #[test]
    fn is_commitable_for_all_variants_matches_spec() {
        let node_id = NodeId::new();
        let parent_id = NodeId::new();
        let other_id = NodeId::new();
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let subtree = editor_model::Subtree::leaf(
            NodeId::new(),
            editor_model::Node::Paragraph(editor_model::ParagraphNode::default()),
        );

        // non-commitable (3)
        let non_commitable: Vec<Step> = vec![
            Step::SetSelection { old: sel, new: sel },
            Step::SetPendingModifiers {
                old: PendingModifiers::new(),
                new: PendingModifiers::new(),
            },
            Step::SetComposition {
                old: None,
                new: None,
            },
        ];

        // commitable (11)
        let commitable: Vec<Step> = vec![
            Step::InsertText {
                node_id,
                offset: 0,
                text: "x".into(),
            },
            Step::RemoveText {
                node_id,
                offset: 0,
                text: "x".into(),
            },
            Step::InsertSubtree {
                parent_id,
                index: 0,
                subtree: subtree.clone(),
            },
            Step::RemoveSubtree {
                parent_id,
                index: 0,
                subtree: subtree.clone(),
            },
            Step::MoveNode {
                node_id,
                old_parent: parent_id,
                old_index: 0,
                new_parent: other_id,
                new_index: 0,
            },
            Step::SplitNode {
                node_id,
                offset: 0,
                new_node_id: other_id,
            },
            Step::MergeNode {
                node_id,
                target_id: other_id,
                offset: 0,
            },
            Step::SetNode {
                node_id,
                old_node: Node::Paragraph(editor_model::ParagraphNode::default()),
                new_node: Node::Paragraph(editor_model::ParagraphNode::default()),
            },
            Step::AddModifier {
                node_id,
                modifier: Modifier::Bold,
            },
            Step::RemoveModifier {
                node_id,
                modifier: Modifier::Bold,
            },
            Step::SetModifiers {
                node_id,
                old_modifiers: vec![],
                new_modifiers: vec![Modifier::Bold],
            },
        ];

        assert_eq!(non_commitable.len() + commitable.len(), 14);

        for step in &non_commitable {
            assert!(!step.is_commitable(), "{step:?}");
        }
        for step in &commitable {
            assert!(step.is_commitable(), "{step:?}");
        }
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;

    #[test]
    fn scope_node_for_text_steps() {
        let n = NodeId::new();
        let step = Step::InsertText {
            node_id: n,
            offset: 0,
            text: "x".into(),
        };
        assert!(matches!(step.scope(), StepScope::Node(id) if id == n));
    }

    #[test]
    fn scope_children_for_subtree_steps() {
        let parent = NodeId::new();
        let id = NodeId::new();
        let subtree = editor_model::Subtree::leaf(
            id,
            editor_model::Node::Paragraph(editor_model::ParagraphNode::default()),
        );
        let step = Step::InsertSubtree {
            parent_id: parent,
            index: 0,
            subtree,
        };
        assert!(matches!(step.scope(), StepScope::Children { parent: p } if p == parent));
    }

    #[test]
    fn scope_structural_for_split_node() {
        let n = NodeId::new();
        let new_n = NodeId::new();
        let step = Step::SplitNode {
            node_id: n,
            offset: 0,
            new_node_id: new_n,
        };
        match step.scope() {
            StepScope::Structural(ids) => {
                assert_eq!(ids.as_slice(), &[n]);
            }
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn scope_structural_for_merge_node() {
        let n = NodeId::new();
        let target = NodeId::new();
        let step = Step::MergeNode {
            node_id: n,
            target_id: target,
            offset: 0,
        };
        match step.scope() {
            StepScope::Structural(ids) => {
                assert_eq!(ids.as_slice(), &[n, target]);
            }
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn scope_structural_for_move_node() {
        let n = NodeId::new();
        let old_p = NodeId::new();
        let new_p = NodeId::new();
        let step = Step::MoveNode {
            node_id: n,
            old_parent: old_p,
            old_index: 0,
            new_parent: new_p,
            new_index: 0,
        };
        match step.scope() {
            StepScope::Structural(ids) => {
                assert_eq!(ids.as_slice(), &[old_p, new_p]);
            }
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn scope_local_for_set_composition() {
        let step = Step::SetComposition {
            old: None,
            new: None,
        };
        assert!(matches!(step.scope(), StepScope::Local));
    }
}

#[cfg(test)]
mod affected_tests {
    use super::*;

    #[test]
    fn affected_includes_split_new_node_id() {
        let old = NodeId::new();
        let new = NodeId::new();
        let step = Step::SplitNode {
            node_id: old,
            offset: 3,
            new_node_id: new,
        };
        let ids = step.affected_node_ids();
        assert!(ids.contains(&old));
        assert!(ids.contains(&new));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn affected_includes_move_node_id() {
        let id = NodeId::new();
        let old_parent = NodeId::new();
        let new_parent = NodeId::new();
        let step = Step::MoveNode {
            node_id: id,
            old_parent,
            old_index: 0,
            new_parent,
            new_index: 0,
        };
        let ids = step.affected_node_ids();
        assert!(ids.contains(&id));
        assert!(ids.contains(&old_parent));
        assert!(ids.contains(&new_parent));
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn affected_merge_node_excludes_removed_node() {
        let surviving = NodeId::new();
        let removed = NodeId::new();
        let step = Step::MergeNode {
            node_id: removed,
            target_id: surviving,
            offset: 0,
        };
        let ids = step.affected_node_ids();
        assert_eq!(ids, vec![surviving]);
    }

    #[test]
    fn affected_includes_subtree_inserted_nodes() {
        use editor_model::{Node, RootNode, Subtree};
        let parent = NodeId::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let n3 = NodeId::new();
        // 3-level: parent → n1 → n2 → n3
        let subtree = Subtree::leaf(n1, Node::Root(RootNode::default())).with_children(vec![
            Subtree::leaf(n2, Node::Root(RootNode::default()))
                .with_children(vec![Subtree::leaf(n3, Node::Root(RootNode::default()))]),
        ]);
        let step = Step::InsertSubtree {
            parent_id: parent,
            index: 0,
            subtree,
        };
        let ids = step.affected_node_ids();
        assert!(ids.contains(&parent));
        assert!(ids.contains(&n1));
        assert!(ids.contains(&n2));
        assert!(ids.contains(&n3));
        assert_eq!(ids.len(), 4);
    }
}

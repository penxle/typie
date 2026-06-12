use editor_crdt::{EntryDot, Op};
use editor_model::{
    DocOp, Marker, Modifier, ModifierType, NodeId, PlainNode, PlainStyleEntry, Subtree,
};
use editor_state::{
    BatchedState, Composition, PendingModifiers, PendingStyle, StableSelection, State,
};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use strum::{EnumDiscriminants, IntoStaticStr};

use crate::{StepError, steps};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub ops: Vec<Op<DocOp>>,
    pub validations: Vec<Validation>,
    pub effect: StepEffect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepEffect {
    pub text_inserts: Vec<TextInsertEffect>,
    pub text_removes: Vec<TextRemoveEffect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInsertEffect {
    pub node_id: NodeId,
    pub offset: usize,
    pub entries: Vec<EntryDot>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRemoveEffect {
    pub node_id: NodeId,
    pub offset: usize,
    pub entries: Vec<EntryDot>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepRecord {
    pub step: Step,
    pub effect: StepEffect,
}

#[derive(Clone, Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(name(StepType))]
#[strum_discriminants(derive(Hash, Serialize, Deserialize, IntoStaticStr))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
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
        old_node: PlainNode,
        new_node: PlainNode,
    },
    AddModifier {
        node_id: NodeId,
        modifier: Modifier,
    },
    RemoveModifier {
        node_id: NodeId,
        modifier: Modifier,
    },
    SetNodeStyle {
        node_id: NodeId,
        old: Option<String>,
        new: Option<String>,
    },
    SetNodeMarker {
        node_id: NodeId,
        old: Option<Marker>,
        new: Option<Marker>,
    },
    SetStyle {
        style_id: String,
        old: Option<PlainStyleEntry>,
        new: Option<PlainStyleEntry>,
    },
    SetSelection {
        old: Option<StableSelection>,
        new: Option<StableSelection>,
    },
    SetPendingModifiers {
        old: PendingModifiers,
        new: PendingModifiers,
    },
    SetPendingStyle {
        old: Option<PendingStyle>,
        new: Option<PendingStyle>,
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
                | Step::SetPendingStyle { .. }
                | Step::SetComposition { .. }
        )
    }

    pub fn is_selection_step(&self) -> bool {
        matches!(self, Step::SetSelection { .. })
    }

    pub fn is_pending_modifiers_step(&self) -> bool {
        matches!(self, Step::SetPendingModifiers { .. })
    }

    pub fn is_pending_style_step(&self) -> bool {
        matches!(self, Step::SetPendingStyle { .. })
    }

    pub fn is_commitable(&self) -> bool {
        !matches!(
            self,
            Step::SetSelection { .. }
                | Step::SetPendingModifiers { .. }
                | Step::SetPendingStyle { .. }
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
            | Step::SetNodeStyle { node_id, .. }
            | Step::SetNodeMarker { node_id, .. } => StepScope::Node(*node_id),

            Step::SetStyle { .. } => StepScope::Node(NodeId::ROOT),

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
            | Step::SetPendingStyle { .. }
            | Step::SetComposition { .. } => StepScope::Local,
        }
    }

    pub fn apply(&self, state: &State) -> Result<StepOutput, StepError> {
        let mut validations = Vec::new();
        let mut effect = StepEffect::default();
        let (new_state, ops) = state
            .batch_with_ops(|s| self.apply_to_with_effect(s, &mut validations, &mut effect))?;
        Ok(StepOutput {
            state: new_state,
            ops,
            validations,
            effect,
        })
    }

    pub(crate) fn apply_to_with_effect(
        &self,
        batched: &mut BatchedState,
        validations: &mut Vec<Validation>,
        effect: &mut StepEffect,
    ) -> Result<(), StepError> {
        match self {
            Step::InsertText {
                node_id,
                offset,
                text,
            } => {
                steps::insert_text::apply_to(batched, validations, effect, *node_id, *offset, text)
            }
            Step::RemoveText {
                node_id,
                offset,
                text,
            } => {
                steps::remove_text::apply_to(batched, validations, effect, *node_id, *offset, text)
            }
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => steps::insert_subtree::apply_to(
                batched,
                validations,
                effect,
                *parent_id,
                *index,
                subtree,
            ),
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => steps::remove_subtree::apply_to(
                batched,
                validations,
                effect,
                *parent_id,
                *index,
                subtree,
            ),
            Step::MoveNode {
                node_id,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => steps::move_node::apply_to(
                batched,
                validations,
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
            } => steps::split_node::apply_to(batched, validations, *node_id, *offset, *new_node_id),
            Step::MergeNode {
                node_id,
                target_id,
                offset,
            } => steps::merge_node::apply_to(batched, validations, *node_id, *target_id, *offset),
            Step::SetNode {
                node_id,
                old_node,
                new_node,
            } => steps::set_node::apply_to(batched, validations, *node_id, old_node, new_node),
            Step::AddModifier { node_id, modifier } => {
                steps::add_modifier::apply_to(batched, validations, *node_id, modifier)
            }
            Step::RemoveModifier { node_id, modifier } => {
                steps::remove_modifier::apply_to(batched, validations, *node_id, modifier)
            }
            Step::SetNodeStyle { node_id, new, .. } => {
                steps::set_node_style::apply_to(batched, validations, *node_id, new.clone())
            }
            Step::SetNodeMarker { node_id, new, .. } => {
                steps::set_node_marker::apply_to(batched, validations, *node_id, new.clone())
            }
            Step::SetStyle { style_id, new, .. } => {
                steps::set_style::apply_to(batched, validations, style_id, new.clone())
            }
            Step::SetSelection { old, new } => {
                steps::set_selection::apply_to(batched, validations, old.clone(), new.clone())
            }
            Step::SetPendingModifiers { old, new } => {
                steps::set_pending_modifiers::apply_to(batched, validations, old, new)
            }
            Step::SetPendingStyle { old, new } => {
                steps::set_pending_style::apply_to(batched, validations, old, new)
            }
            Step::SetComposition { old, new } => {
                steps::set_composition::apply_to(batched, validations, *old, *new)
            }
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
            Step::SetNodeStyle { node_id, old, new } => {
                steps::set_node_style::inverse(*node_id, old.clone(), new.clone())
            }
            Step::SetNodeMarker { node_id, old, new } => {
                steps::set_node_marker::inverse(*node_id, old.clone(), new.clone())
            }
            Step::SetStyle { style_id, old, new } => {
                steps::set_style::inverse(style_id.clone(), old.clone(), new.clone())
            }
            Step::SetSelection { old, new } => {
                steps::set_selection::inverse(old.clone(), new.clone())
            }
            Step::SetPendingModifiers { old, new } => {
                steps::set_pending_modifiers::inverse(old.clone(), new.clone())
            }
            Step::SetPendingStyle { old, new } => {
                steps::set_pending_style::inverse(old.clone(), new.clone())
            }
            Step::SetComposition { old, new } => steps::set_composition::inverse(*old, *new),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::{PlainNode, PlainParagraphNode};

    fn fixture_stable_selection() -> Option<StableSelection> {
        let (s, _t1) = state! {
            doc { root { paragraph { _t1: text("x") } } }
            selection: (_t1, 0)
        };
        s.selection
            .as_ref()
            .map(|sel| StableSelection::capture(sel, &s.doc))
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
        let sel = fixture_stable_selection();
        let step = Step::SetSelection {
            old: sel.clone(),
            new: sel,
        };
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
    fn is_commitable_false_for_set_pending_style() {
        let step = Step::SetPendingStyle {
            old: None,
            new: None,
        };
        assert!(!step.is_commitable());
    }

    #[test]
    fn is_commitable_for_all_variants_matches_spec() {
        let node_id = NodeId::new();
        let parent_id = NodeId::new();
        let other_id = NodeId::new();
        let sel = fixture_stable_selection();
        let subtree = Subtree::leaf(
            NodeId::new(),
            PlainNode::Paragraph(PlainParagraphNode::default()),
        );

        // non-commitable (4)
        let non_commitable: Vec<Step> = vec![
            Step::SetSelection {
                old: sel.clone(),
                new: sel,
            },
            Step::SetPendingModifiers {
                old: PendingModifiers::new(),
                new: PendingModifiers::new(),
            },
            Step::SetPendingStyle {
                old: None,
                new: None,
            },
            Step::SetComposition {
                old: None,
                new: None,
            },
        ];

        // commitable (13)
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
                old_node: PlainNode::Paragraph(PlainParagraphNode::default()),
                new_node: PlainNode::Paragraph(PlainParagraphNode::default()),
            },
            Step::AddModifier {
                node_id,
                modifier: Modifier::Bold,
            },
            Step::RemoveModifier {
                node_id,
                modifier: Modifier::Bold,
            },
            Step::SetNodeStyle {
                node_id,
                old: None,
                new: Some("h".into()),
            },
            Step::SetNodeMarker {
                node_id,
                old: None,
                new: None,
            },
            Step::SetStyle {
                style_id: "h".into(),
                old: None,
                new: None,
            },
        ];

        assert_eq!(non_commitable.len() + commitable.len(), 17);

        for step in &non_commitable {
            assert!(!step.is_commitable(), "{step:?}");
        }
        for step in &commitable {
            assert!(step.is_commitable(), "{step:?}");
        }
    }

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
        let subtree = Subtree::leaf(id, PlainNode::Paragraph(PlainParagraphNode::default()));
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

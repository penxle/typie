use editor_model::{DocumentAttrs, Modifier, ModifierType, Node, NodeId, Subtree};
use editor_state::{Composition, PendingModifiers, Selection, State};
use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use strum::{EnumDiscriminants, IntoStaticStr};

use crate::Mapping;
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
    pub mapping: Mapping,
    pub validations: Vec<Validation>,
}

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
    SetDocumentAttrs {
        old: DocumentAttrs,
        new: DocumentAttrs,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepScope {
    Node(NodeId),
    Children { parent: NodeId },
    Structural(SmallVec<[NodeId; 3]>),
    Document,
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

    pub fn is_doc_attr_step(&self) -> bool {
        matches!(self, Step::SetDocumentAttrs { .. })
    }

    pub fn is_selection_step(&self) -> bool {
        matches!(self, Step::SetSelection { .. })
    }

    pub fn is_pending_modifiers_step(&self) -> bool {
        matches!(self, Step::SetPendingModifiers { .. })
    }

    pub fn is_syncable(&self) -> bool {
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
                node_id,
                old_parent,
                new_parent,
                ..
            } => StepScope::Structural(smallvec![*node_id, *old_parent, *new_parent]),

            Step::SetDocumentAttrs { .. } => StepScope::Document,

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
            Step::InsertSubtree { parent_id, .. } | Step::RemoveSubtree { parent_id, .. } => {
                vec![*parent_id]
            }
            Step::SplitNode { node_id, .. } => vec![*node_id],
            Step::MergeNode {
                node_id, target_id, ..
            } => vec![*node_id, *target_id],
            Step::MoveNode {
                old_parent,
                new_parent,
                ..
            } => vec![*old_parent, *new_parent],
            Step::SetSelection { .. }
            | Step::SetPendingModifiers { .. }
            | Step::SetComposition { .. }
            | Step::SetDocumentAttrs { .. } => vec![],
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
            Step::SetDocumentAttrs { old: _, new } => steps::set_document_attrs::apply(state, new),
        }
    }

    pub fn mapping(&self, state: &State) -> Result<Mapping, StepError> {
        let _ = state;
        match self {
            Step::InsertText {
                node_id,
                offset,
                text,
            } => Ok(steps::insert_text::build_mapping(*node_id, *offset, text)),
            Step::RemoveText {
                node_id,
                offset,
                text,
            } => Ok(steps::remove_text::build_mapping(*node_id, *offset, text)),
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => Ok(steps::insert_subtree::build_mapping(
                *parent_id, *index, subtree.id,
            )),
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => Ok(steps::remove_subtree::build_mapping(
                *parent_id, *index, subtree,
            )),
            Step::AddModifier { .. } => Ok(steps::add_modifier::build_mapping()),
            Step::RemoveModifier { .. } => Ok(steps::remove_modifier::build_mapping()),
            Step::SetModifiers { .. } => Ok(steps::set_modifiers::build_mapping()),
            Step::SetNode { .. } => Ok(steps::set_node::build_mapping()),
            Step::SetDocumentAttrs { .. } => Ok(steps::set_document_attrs::build_mapping()),

            Step::SplitNode { .. } | Step::MergeNode { .. } | Step::MoveNode { .. } => {
                unreachable!(
                    "structural step.mapping() called — caller must dispatch via scope: {:?}",
                    StepType::from(self)
                );
            }

            Step::SetSelection { .. }
            | Step::SetPendingModifiers { .. }
            | Step::SetComposition { .. } => Ok(Mapping::identity()),
        }
    }

    pub fn rebase(&self, mapping: &Mapping) -> Vec<Step> {
        match self {
            Step::InsertText {
                node_id,
                offset,
                text,
            } => steps::insert_text::rebase_against(*node_id, *offset, text, mapping),
            Step::RemoveText {
                node_id,
                offset,
                text,
            } => steps::remove_text::rebase_against(*node_id, *offset, text, mapping),
            Step::InsertSubtree {
                parent_id,
                index,
                subtree,
            } => steps::insert_subtree::rebase_against(*parent_id, *index, subtree, mapping),
            Step::RemoveSubtree {
                parent_id,
                index,
                subtree,
            } => steps::remove_subtree::rebase_against(*parent_id, *index, subtree, mapping),
            Step::AddModifier { node_id, modifier } => {
                steps::add_modifier::rebase_against(*node_id, modifier, mapping)
            }
            Step::RemoveModifier { node_id, modifier } => {
                steps::remove_modifier::rebase_against(*node_id, modifier, mapping)
            }
            Step::SetModifiers {
                node_id,
                old_modifiers,
                new_modifiers,
            } => steps::set_modifiers::rebase_against(
                *node_id,
                old_modifiers,
                new_modifiers,
                mapping,
            ),
            Step::SetNode {
                node_id,
                old_node,
                new_node,
            } => steps::set_node::rebase_against(*node_id, old_node, new_node, mapping),
            Step::SetDocumentAttrs { old, new } => {
                steps::set_document_attrs::rebase_against(old, new, mapping)
            }

            Step::SplitNode { .. } | Step::MergeNode { .. } | Step::MoveNode { .. } => {
                unreachable!(
                    "structural step.rebase() called — caller must dispatch via scope: {:?}",
                    StepType::from(self)
                );
            }

            Step::SetSelection { .. }
            | Step::SetPendingModifiers { .. }
            | Step::SetComposition { .. } => {
                unreachable!(
                    "non-syncable step.rebase() called: {:?}",
                    StepType::from(self)
                );
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
            Step::SetDocumentAttrs { old, new } => {
                steps::set_document_attrs::inverse(old.clone(), new.clone())
            }
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
    fn is_syncable_true_for_insert_text() {
        let step = Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        };
        assert!(step.is_syncable());
    }

    #[test]
    fn is_syncable_false_for_set_selection() {
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let step = Step::SetSelection { old: sel, new: sel };
        assert!(!step.is_syncable());
    }

    #[test]
    fn is_syncable_false_for_set_composition() {
        let step = Step::SetComposition {
            old: None,
            new: None,
        };
        assert!(!step.is_syncable());
    }

    #[test]
    fn is_syncable_false_for_set_pending_modifiers() {
        let step = Step::SetPendingModifiers {
            old: PendingModifiers::new(),
            new: PendingModifiers::new(),
        };
        assert!(!step.is_syncable());
    }

    #[test]
    fn is_syncable_true_for_set_document_attrs() {
        let attrs = editor_model::DocumentAttrs::default();
        let step = Step::SetDocumentAttrs {
            old: attrs.clone(),
            new: attrs,
        };
        assert!(step.is_syncable());
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
                assert_eq!(ids.as_slice(), &[n, old_p, new_p]);
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

    #[test]
    fn scope_document_for_set_document_attrs() {
        let attrs = editor_model::DocumentAttrs::default();
        let step = Step::SetDocumentAttrs {
            old: attrs.clone(),
            new: attrs,
        };
        assert!(matches!(step.scope(), StepScope::Document));
    }
}

#[cfg(test)]
mod mapping_method_tests {
    use super::*;
    use crate::MapAction;
    use crate::test_utils::empty_state;
    use editor_state::{Position, Selection};

    #[test]
    fn insert_text_step_yields_text_insert_mapping() {
        let n = NodeId::new();
        let step = Step::InsertText {
            node_id: n,
            offset: 0,
            text: "ab".into(),
        };
        let m = step.mapping(&empty_state()).unwrap();
        assert_eq!(
            m.actions(),
            &[MapAction::TextInsert {
                node: n,
                offset: 0,
                len: 2,
                text: "ab".into(),
            }]
        );
    }

    #[test]
    fn add_modifier_step_yields_identity_mapping() {
        let step = Step::AddModifier {
            node_id: NodeId::new(),
            modifier: editor_model::Modifier::Bold,
        };
        let m = step.mapping(&empty_state()).unwrap();
        assert_eq!(m, Mapping::identity());
    }

    #[test]
    fn set_selection_step_yields_identity_mapping() {
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let step = Step::SetSelection { old: sel, new: sel };
        let m = step.mapping(&empty_state()).unwrap();
        assert_eq!(m, Mapping::identity());
    }

    #[test]
    #[should_panic(expected = "structural step.mapping() called")]
    fn split_node_step_panics() {
        let step = Step::SplitNode {
            node_id: NodeId::new(),
            offset: 0,
            new_node_id: NodeId::new(),
        };
        let _ = step.mapping(&empty_state());
    }
}

#[cfg(test)]
mod rebase_method_tests {
    use super::*;
    use crate::MapAction;

    #[test]
    fn insert_text_dispatches_to_step_module() {
        let n = NodeId::new();
        let step = Step::InsertText {
            node_id: n,
            offset: 5,
            text: "x".into(),
        };
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 2,
            len: 3,
            text: "abc".into(),
        });
        let result = step.rebase(&mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 8,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn add_modifier_dispatches_to_step_module() {
        let n = NodeId::new();
        let step = Step::AddModifier {
            node_id: n,
            modifier: Modifier::Bold,
        };
        let mapping = Mapping::single(MapAction::NodeDeleted { node: n });
        let result = step.rebase(&mapping);
        assert!(result.is_empty());
    }

    #[test]
    #[should_panic(expected = "structural step.rebase() called")]
    fn split_node_step_panics() {
        let step = Step::SplitNode {
            node_id: NodeId::new(),
            offset: 0,
            new_node_id: NodeId::new(),
        };
        let _ = step.rebase(&Mapping::identity());
    }
}

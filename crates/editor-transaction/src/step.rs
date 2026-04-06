use editor_model::{DocumentAttrs, Modifier, ModifierType, Node, NodeId, Subtree};
use editor_state::{Composition, PendingModifiers, Selection, State};

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

#[derive(Clone, Debug)]
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

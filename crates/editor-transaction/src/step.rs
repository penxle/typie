use editor_crdt::{Dot, Op};
use editor_model::{EditOp, Marker, Modifier, PlainNode, PlainStyleEntry, Subtree};
use editor_state::Selection;
use editor_state::{BatchedState, Composition, PendingModifiers, PendingStyle, State};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoStaticStr};

use crate::{StepError, steps};

pub struct StepOutput {
    pub state: State,
    pub ops: Vec<Op<EditOp>>,
    pub effect: StepEffect,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepEffect;

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
        block: Dot,
        offset: usize,
        text: String,
    },
    RemoveText {
        block: Dot,
        offset: usize,
        text: String,
    },
    InsertSubtree {
        parent: Dot,
        index: usize,
        subtree: Subtree,
    },
    RemoveSubtree {
        parent: Dot,
        index: usize,
        subtree: Subtree,
    },
    MoveNode {
        block: Dot,
        old_parent: Dot,
        old_index: usize,
        new_parent: Dot,
        new_index: usize,
    },
    SplitNode {
        block: Dot,
        offset: usize,
    },
    MergeNode {
        block: Dot,
        offset: usize,
    },
    SetNode {
        block: Dot,
        old_node: PlainNode,
        new_node: PlainNode,
    },
    AddModifier {
        block: Dot,
        modifier: Modifier,
    },
    RemoveModifier {
        block: Dot,
        modifier: Modifier,
    },
    AddSpanModifier {
        first: Dot,
        last: Dot,
        modifier: Modifier,
    },
    RemoveSpanModifier {
        first: Dot,
        last: Dot,
        modifier: Modifier,
    },
    SetNodeStyle {
        block: Dot,
        old: Option<String>,
        new: Option<String>,
    },
    SetNodeMarker {
        block: Dot,
        old: Option<Marker>,
        new: Option<Marker>,
    },
    SetStyle {
        style_id: String,
        old: Option<PlainStyleEntry>,
        new: Option<PlainStyleEntry>,
    },
    SetSelection {
        old: Option<Selection>,
        new: Option<Selection>,
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
        self.is_doc_step()
    }

    pub fn apply(&self, state: &State) -> Result<StepOutput, StepError> {
        let (new_state, ops) = state.batch_with_ops(|s| self.apply_to_with_effect(s))?;
        Ok(StepOutput {
            state: new_state,
            ops,
            effect: StepEffect,
        })
    }

    pub(crate) fn apply_to_with_effect(&self, batched: &mut BatchedState) -> Result<(), StepError> {
        match self {
            Step::InsertText {
                block,
                offset,
                text,
            } => steps::insert_text::apply_to(batched, *block, *offset, text),
            Step::RemoveText {
                block,
                offset,
                text,
            } => steps::remove_text::apply_to(batched, *block, *offset, text),
            Step::InsertSubtree {
                parent,
                index,
                subtree,
            } => steps::insert_subtree::apply_to(batched, *parent, *index, subtree),
            Step::RemoveSubtree {
                parent,
                index,
                subtree,
            } => steps::remove_subtree::apply_to(batched, *parent, *index, subtree),
            Step::MoveNode {
                block,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => steps::move_node::apply_to(
                batched,
                *block,
                *old_parent,
                *old_index,
                *new_parent,
                *new_index,
            ),
            Step::SplitNode { block, offset } => {
                steps::split_node::apply_to(batched, *block, *offset)
            }
            Step::MergeNode { block, offset } => {
                steps::merge_node::apply_to(batched, *block, *offset)
            }
            Step::SetNode {
                block,
                old_node,
                new_node,
            } => steps::set_node::apply_to(batched, *block, old_node, new_node),
            Step::AddModifier { block, modifier } => {
                steps::add_modifier::apply_to(batched, *block, modifier)
            }
            Step::RemoveModifier { block, modifier } => {
                steps::remove_modifier::apply_to(batched, *block, modifier)
            }
            Step::AddSpanModifier {
                first,
                last,
                modifier,
            } => steps::add_span_modifier::apply_to(batched, *first, *last, modifier),
            Step::RemoveSpanModifier {
                first,
                last,
                modifier,
            } => steps::remove_span_modifier::apply_to(batched, *first, *last, modifier),
            Step::SetNodeStyle { block, new, .. } => {
                steps::set_node_style::apply_to(batched, *block, new.clone())
            }
            Step::SetNodeMarker { block, new, .. } => {
                steps::set_node_marker::apply_to(batched, *block, new.clone())
            }
            Step::SetStyle { style_id, new, .. } => {
                steps::set_style::apply_to(batched, style_id, new.clone())
            }
            Step::SetSelection { new, .. } => steps::set_selection::apply_to(batched, *new),
            Step::SetPendingModifiers { new, .. } => {
                steps::set_pending_modifiers::apply_to(batched, new)
            }
            Step::SetPendingStyle { new, .. } => steps::set_pending_style::apply_to(batched, new),
            Step::SetComposition { new, .. } => steps::set_composition::apply_to(batched, *new),
        }
    }

    pub fn inverse(&self) -> Step {
        match self {
            Step::InsertText {
                block,
                offset,
                text,
            } => steps::insert_text::inverse(*block, *offset, text.clone()),
            Step::RemoveText {
                block,
                offset,
                text,
            } => steps::remove_text::inverse(*block, *offset, text.clone()),
            Step::InsertSubtree {
                parent,
                index,
                subtree,
            } => steps::insert_subtree::inverse(*parent, *index, subtree.clone()),
            Step::RemoveSubtree {
                parent,
                index,
                subtree,
            } => steps::remove_subtree::inverse(*parent, *index, subtree.clone()),
            Step::MoveNode {
                block,
                old_parent,
                old_index,
                new_parent,
                new_index,
            } => {
                steps::move_node::inverse(*block, *old_parent, *old_index, *new_parent, *new_index)
            }
            Step::SplitNode { block, offset } => steps::split_node::inverse(*block, *offset),
            Step::MergeNode { block, offset } => steps::merge_node::inverse(*block, *offset),
            Step::SetNode {
                block,
                old_node,
                new_node,
            } => steps::set_node::inverse(*block, old_node.clone(), new_node.clone()),
            Step::AddModifier { block, modifier } => {
                steps::add_modifier::inverse(*block, modifier.clone())
            }
            Step::RemoveModifier { block, modifier } => {
                steps::remove_modifier::inverse(*block, modifier.clone())
            }
            Step::AddSpanModifier {
                first,
                last,
                modifier,
            } => steps::add_span_modifier::inverse(*first, *last, modifier.clone()),
            Step::RemoveSpanModifier {
                first,
                last,
                modifier,
            } => steps::remove_span_modifier::inverse(*first, *last, modifier.clone()),
            Step::SetNodeStyle { block, old, new } => {
                steps::set_node_style::inverse(*block, old.clone(), new.clone())
            }
            Step::SetNodeMarker { block, old, new } => {
                steps::set_node_marker::inverse(*block, old.clone(), new.clone())
            }
            Step::SetStyle { style_id, old, new } => {
                steps::set_style::inverse(style_id.clone(), old.clone(), new.clone())
            }
            Step::SetSelection { old, new } => steps::set_selection::inverse(*old, *new),
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

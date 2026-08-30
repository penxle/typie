use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{DocView, Fragment};
use editor_state::{Affinity, Position};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::{
    ExistingBlock, HoistBoundaryStep, HoistInitialBoundary, HoistedBlockInsertionPlan,
    OpenAncestorSplice, SliceInsertionExecution, SliceInsertionOutputPlan, SliceInsertionTarget,
    SliceInsertionTargetShape, SliceOutputPositionSpec, SliceOutputRelation, SliceOutputSource,
    TextblockSplicePlan, block_boundary_fragments, build_inline_mode,
    fit_slice_for_textblock_target_parent, fragments_are_inline, fragments_fit_parent,
    inline_fragments_fit_target, insert_blocks_at_block_boundary,
    insert_blocks_in_textblock_at_position, insert_content_as_inline_at_position,
    insert_hoisted_blocks_at_position, insert_open_ancestor_splice_at_position,
    is_insertable_inline_fragment, is_supported_inline_fragment, materialize_repair_position,
    open_ancestor_splice_for_target, open_ancestor_splice_is_complete_no_output,
    open_inline_content_for_target, plan_textblock_splice_target, planned_block_output,
    planned_inline_output, planned_open_splice_output, resolve_live_slice_output_dot,
    top_level_fragments,
};
use crate::types::SliceProvenance;

mod fitter;
mod fragment_fitter;
mod linear_fitter;
mod table_fitter;
pub(crate) use fitter::SliceFitPlanKind;
pub use fitter::{FitOutcome, SliceFitPlan, fit_slice};
pub(crate) use fragment_fitter::fit_fragment_forest;
pub(crate) use linear_fitter::{
    JoinedReplacementPlan, LinearFinalSelection, LinearFitPlan, LinearMutation,
    PlannedBoundaryInsertion, PlannedBranchInsertion, PlannedBranchNode, PlannedBranchSplit,
    PlannedJoin, PlannedOutputKey, RangePlacement,
};
pub(crate) use table_fitter::{CellFillPlan, TableFinalSelection, TableGridPlan};

pub(crate) struct SliceInsertionPlan {
    kind: SliceInsertionKind,
    output: SliceInsertionOutputPlan,
}

enum SliceInsertionKind {
    DirectInline {
        fragments: Vec<Fragment>,
    },
    SpliceOpenAncestors {
        destination: Vec<editor_crdt::Dot>,
        source: Fragment,
    },
    SpliceBlocks {
        plan: TextblockSplicePlan,
    },
    HoistBlocks {
        plan: HoistedBlockInsertionPlan,
    },
    OpenInline {
        fragments: Vec<Fragment>,
    },
    BlockBoundary {
        blocks: Vec<Fragment>,
        list_merges: Vec<crate::helpers::PlannedListMerge>,
    },
}

pub(super) enum PlacementOutcome {
    Placed(Box<SliceInsertionPlan>),
    CompleteNoOutput,
    NoFit,
}

pub(super) fn place_slice_at_position(
    view: &DocView,
    position: Position,
    slice: Slice,
) -> PlacementOutcome {
    let Some(target) = SliceInsertionTarget::from_view(view, position) else {
        return PlacementOutcome::NoFit;
    };
    place_slice_at_frontier(&target, slice)
}

pub(super) fn place_slice_at_frontier(
    target: &SliceInsertionTarget,
    slice: Slice,
) -> PlacementOutcome {
    if placement_is_complete_no_output(target, &slice) {
        return PlacementOutcome::CompleteNoOutput;
    }
    try_place_slice_at_frontier(target, slice)
        .map(|plan| PlacementOutcome::Placed(Box::new(plan)))
        .unwrap_or(PlacementOutcome::NoFit)
}

fn try_place_slice_at_frontier(
    target: &SliceInsertionTarget,
    slice: Slice,
) -> Option<SliceInsertionPlan> {
    if let Some(textblock) = target
        .path()
        .iter()
        .rev()
        .find(|node| editor_model::Schema::node_spec(node.node_type).is_textblock())
    {
        let textblock_id = textblock.id;
        let textblock_path = target
            .path()
            .iter()
            .take_while(|node| node.id != textblock_id)
            .chain(std::iter::once(textblock))
            .map(|node| node.node_type)
            .collect::<Vec<_>>();
        let top_level = top_level_fragments(&slice);
        let direct_inline = fragments_are_inline(&top_level)
            && top_level
                .iter()
                .all(|fragment| is_supported_inline_fragment(fragment))
            && fragments_fit_parent(textblock.node_type, &top_level)
            && top_level
                .iter()
                .any(|fragment| is_insertable_inline_fragment(fragment));
        if direct_inline
            && let Some(fragments) = fit_fragment_forest(slice.content.clone(), &textblock_path)
            && inline_fragments_fit_target(target, &fragments)
            && fragments.iter().any(is_insertable_inline_fragment)
        {
            let output = planned_inline_output(&fragments, target.position().affinity)?;
            return Some(SliceInsertionPlan {
                kind: SliceInsertionKind::DirectInline { fragments },
                output,
            });
        }

        if let Some(mut splice) = open_ancestor_splice_for_target(target, &slice) {
            let outer_index = target
                .path()
                .iter()
                .position(|node| node.id == splice.destination[0])?;
            let parent_path = outer_index.checked_sub(1).map(|parent_index| {
                target.path()[..=parent_index]
                    .iter()
                    .map(|node| node.node_type)
                    .collect::<Vec<_>>()
            })?;
            let mut fitted = fit_fragment_forest(vec![splice.source.clone()], &parent_path)?;
            if fitted.len() != 1 {
                return None;
            }
            splice.source = fitted.pop().expect("cardinality checked");
            let output = planned_open_splice_output(&splice)?;
            return Some(SliceInsertionPlan {
                kind: SliceInsertionKind::SpliceOpenAncestors {
                    destination: splice.destination,
                    source: splice.source,
                },
                output,
            });
        }

        let textblock_index = target
            .path()
            .iter()
            .position(|node| node.id == textblock_id)?;
        let parent = textblock_index
            .checked_sub(1)
            .and_then(|index| target.path().get(index));
        let parent_path = textblock_index.checked_sub(1).map(|index| {
            target.path()[..=index]
                .iter()
                .map(|node| node.node_type)
                .collect::<Vec<_>>()
        });
        let parent_fitted = parent_path.as_ref().and_then(|parent_path| {
            fit_slice_for_textblock_target_parent(target, &slice)
                .and_then(|candidate| {
                    let content = fit_fragment_forest(candidate.content, parent_path)?;
                    Some(Slice::new(
                        content,
                        candidate.open_start,
                        candidate.open_end,
                    ))
                })
                .or_else(|| {
                    let has_meaningful_top_level = slice.content.iter().any(|fragment| {
                        is_insertable_inline_fragment(fragment)
                            || fragment.node.as_type() == editor_model::NodeType::PageBreak
                            || !editor_model::Schema::node_spec(fragment.node.as_type()).inline
                    });
                    if !has_meaningful_top_level {
                        return None;
                    }
                    let parent_type = parent?.node_type;
                    let blocks = block_boundary_fragments(&slice, parent_type)?;
                    let content = fit_fragment_forest(blocks, parent_path)?;
                    Some(Slice::new(content, 1, 1))
                })
        });
        if let Some(plan) = parent_fitted
            .as_ref()
            .and_then(|candidate| plan_textblock_splice_target(target, candidate))
        {
            let output = plan.planned_output()?;
            return Some(SliceInsertionPlan {
                kind: SliceInsertionKind::SpliceBlocks { plan },
                output,
            });
        }
        if let Some(plan) = plan_hoisted_block_insertion(target, &slice) {
            let output = planned_block_output(&plan.blocks, &plan.list_merges)?;
            return Some(SliceInsertionPlan {
                kind: SliceInsertionKind::HoistBlocks { plan },
                output,
            });
        }

        let candidate = parent_fitted.as_ref().unwrap_or(&slice);
        if let Some(fragments) = open_inline_content_for_target(target, candidate) {
            let fragments =
                fit_fragment_forest(fragments.into_iter().cloned().collect(), &textblock_path)?;
            if !inline_fragments_fit_target(target, &fragments) {
                return None;
            }
            if !fragments.iter().all(is_supported_inline_fragment) {
                return None;
            }
            if !fragments.iter().any(is_insertable_inline_fragment) {
                return None;
            }
            let output = planned_inline_output(&fragments, target.position().affinity)?;
            return Some(SliceInsertionPlan {
                kind: SliceInsertionKind::OpenInline { fragments },
                output,
            });
        }
        None
    } else {
        let container = target.path().last()?;
        // A fixed-arity container (e.g. Fold) can never absorb an extra child:
        // the inserted blocks would stay permanent, projection-suppressed
        // misfits, so the insertion can't produce an observable change.
        if !editor_model::Schema::node_spec(container.node_type)
            .content
            .admits_additional_child()
        {
            return None;
        }
        let blocks = block_boundary_fragments(&slice, container.node_type)?;
        let container_path = target
            .path()
            .iter()
            .map(|node| node.node_type)
            .collect::<Vec<_>>();
        let blocks = fit_fragment_forest(blocks, &container_path)?;
        let position = target.position();
        let left = position.offset.checked_sub(1).and_then(|index| {
            Some(ExistingBlock {
                id: container.child_ids.get(index).copied().flatten(),
                node_type: *container.child_types.get(index)?,
            })
        });
        let right = container
            .child_types
            .get(position.offset)
            .map(|node_type| ExistingBlock {
                id: container.child_ids.get(position.offset).copied().flatten(),
                node_type: *node_type,
            });
        let list_merges = crate::helpers::plan_adjacent_list_merges(left, &blocks, right)?;
        let output = planned_block_output(&blocks, &list_merges)?;
        Some(SliceInsertionPlan {
            kind: SliceInsertionKind::BlockBoundary {
                blocks,
                list_merges,
            },
            output,
        })
    }
}

fn plan_hoisted_block_insertion(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> Option<HoistedBlockInsertionPlan> {
    let has_meaningful_content = slice.content.iter().any(|fragment| {
        is_insertable_inline_fragment(fragment)
            || fragment.node.as_type() == editor_model::NodeType::PageBreak
            || !editor_model::Schema::node_spec(fragment.node.as_type()).inline
    });
    if !has_meaningful_content {
        return None;
    }
    let textblock_index = target
        .path()
        .iter()
        .rposition(|node| editor_model::Schema::node_spec(node.node_type).is_textblock())?;
    let mut current_index = textblock_index;
    let target_offset = target.position().offset;
    let textblock_child_count = target.path()[textblock_index].child_types.len();
    let initial_boundary = if target_offset == 0 {
        HoistInitialBoundary::Before
    } else if target_offset == textblock_child_count {
        HoistInitialBoundary::After
    } else {
        HoistInitialBoundary::Split
    };
    let textblock_child_index = target.path()[textblock_index].index?;
    let (mut boundary_before, mut boundary_index, mut current_was_split) = match initial_boundary {
        HoistInitialBoundary::Before => (true, textblock_child_index, false),
        HoistInitialBoundary::After => (false, textblock_child_index, false),
        HoistInitialBoundary::Split => (true, textblock_child_index + 1, true),
    };
    let mut boundary_steps = Vec::new();

    while let Some(parent_index) = current_index.checked_sub(1) {
        let current = &target.path()[current_index];
        let parent = &target.path()[parent_index];
        let current_child_index = current.index?;
        let mut destination_types = parent.child_types.clone();
        let mut destination_ids = parent.child_ids.clone();
        if current_was_split {
            if editor_model::Schema::node_spec(current.node_type).isolating {
                return None;
            }
            destination_types.insert(current_child_index + 1, current.node_type);
            destination_ids.insert(current_child_index + 1, None);
            if !editor_model::content_placement(parent.node_type, &destination_types).is_valid() {
                return None;
            }
        }
        let parent_boundary = boundary_index + usize::from(!boundary_before);
        if parent_boundary > destination_types.len() {
            return None;
        }

        if editor_model::Schema::node_spec(parent.node_type)
            .content
            .admits_additional_child()
            && let Some(blocks) = block_boundary_fragments(slice, parent.node_type)
        {
            let parent_path = target.path()[..=parent_index]
                .iter()
                .map(|node| node.node_type)
                .collect::<Vec<_>>();
            if let Some(blocks) = fit_fragment_forest(blocks, &parent_path) {
                let left = parent_boundary.checked_sub(1).and_then(|index| {
                    Some(ExistingBlock {
                        id: destination_ids.get(index).copied().flatten(),
                        node_type: *destination_types.get(index)?,
                    })
                });
                let right = destination_types
                    .get(parent_boundary)
                    .map(|node_type| ExistingBlock {
                        id: destination_ids.get(parent_boundary).copied().flatten(),
                        node_type: *node_type,
                    });
                let list_merges = crate::helpers::plan_adjacent_list_merges(left, &blocks, right)?;
                let mut final_types = destination_types.clone();
                final_types.splice(
                    parent_boundary..parent_boundary,
                    blocks.iter().map(|fragment| fragment.node.as_type()),
                );
                if editor_model::content_placement(parent.node_type, &final_types).is_valid() {
                    return Some(HoistedBlockInsertionPlan {
                        target: SliceInsertionTargetShape::capture(target),
                        parent_depth: parent_index,
                        initial_boundary,
                        boundary_steps,
                        blocks,
                        list_merges,
                    });
                }
            }
        }

        // Reaching the next frontier would move Slice content outside this
        // destination ancestor, which an isolating node forbids even at an edge.
        if editor_model::Schema::node_spec(parent.node_type).isolating {
            return None;
        }
        let step = if boundary_before {
            if boundary_index == 0 {
                HoistBoundaryStep::LiftBefore
            } else {
                HoistBoundaryStep::SplitBefore
            }
        } else if boundary_index + 1 == destination_types.len() {
            HoistBoundaryStep::LiftAfter
        } else {
            HoistBoundaryStep::SplitAfter
        };
        let parent_child_index = parent.index?;
        match step {
            HoistBoundaryStep::LiftBefore => {
                boundary_before = true;
                boundary_index = parent_child_index;
                current_was_split = false;
            }
            HoistBoundaryStep::LiftAfter => {
                boundary_before = false;
                boundary_index = parent_child_index;
                current_was_split = false;
            }
            HoistBoundaryStep::SplitBefore | HoistBoundaryStep::SplitAfter => {
                boundary_before = true;
                boundary_index = parent_child_index + 1;
                current_was_split = true;
            }
        }
        boundary_steps.push(step);
        current_index = parent_index;
    }
    None
}

fn placement_is_complete_no_output(target: &SliceInsertionTarget, slice: &Slice) -> bool {
    if slice.is_empty() {
        return true;
    }
    let Some(textblock) = target
        .path()
        .iter()
        .rev()
        .find(|node| editor_model::Schema::node_spec(node.node_type).is_textblock())
    else {
        return false;
    };
    let textblock_path = target
        .path()
        .iter()
        .take_while(|node| node.id != textblock.id)
        .chain(std::iter::once(textblock))
        .map(|node| node.node_type)
        .collect::<Vec<_>>();
    let top_level = top_level_fragments(slice);
    let empty_inline = fragments_are_inline(&top_level)
        && top_level
            .iter()
            .all(|fragment| is_supported_inline_fragment(fragment))
        && !top_level
            .iter()
            .any(|fragment| is_insertable_inline_fragment(fragment))
        && fit_fragment_forest(slice.content.clone(), &textblock_path)
            .is_some_and(|fragments| inline_fragments_fit_target(target, &fragments));
    if empty_inline || open_ancestor_splice_is_complete_no_output(target, slice) {
        return true;
    }
    open_inline_content_for_target(target, slice).is_some_and(|fragments| {
        let fragments = fragments.into_iter().cloned().collect::<Vec<_>>();
        fit_fragment_forest(fragments, &textblock_path).is_some_and(|fragments| {
            fragments.iter().all(is_supported_inline_fragment)
                && !fragments.iter().any(is_insertable_inline_fragment)
                && inline_fragments_fit_target(target, &fragments)
        })
    })
}

pub(crate) struct AppliedSliceInsertion {
    pub(crate) caret: BoundSliceOutputPosition,
    pub(crate) anchor: BoundSliceOutputPosition,
    pub(crate) head: BoundSliceOutputPosition,
}

#[derive(Clone, Copy)]
pub(crate) struct BoundSliceOutputPosition {
    pub(crate) dot: Dot,
    pub(crate) relation: SliceOutputRelation,
    pub(crate) affinity: Affinity,
}

pub(crate) fn apply_slice_insertion_plan(
    tr: &mut Transaction,
    position: Position,
    plan: SliceInsertionPlan,
    provenance: SliceProvenance,
) -> Result<AppliedSliceInsertion, CommandError> {
    let SliceInsertionPlan { kind, output } = plan;
    let execution = match kind {
        SliceInsertionKind::DirectInline { fragments }
        | SliceInsertionKind::OpenInline { fragments } => {
            let position = materialize_repair_position(tr, position)?;
            let mode = build_inline_mode(tr, &position, provenance)?;
            insert_content_as_inline_at_position(tr, position, fragments, &mode)?
        }
        SliceInsertionKind::SpliceOpenAncestors {
            destination,
            source,
        } => {
            let mut inserted = None;
            tr.batch::<_, CommandError>(|tr| {
                let position = materialize_repair_position(tr, position)?;
                let mode = build_inline_mode(tr, &position, provenance)?;
                inserted = insert_open_ancestor_splice_at_position(
                    tr,
                    position,
                    OpenAncestorSplice {
                        destination: destination.clone(),
                        source: source.clone(),
                    },
                    &mode,
                )?;
                Ok(())
            })?;
            inserted
        }
        SliceInsertionKind::SpliceBlocks { plan } => {
            let mut inserted = None;
            tr.batch::<_, CommandError>(|tr| {
                let position = materialize_repair_position(tr, position)?;
                let mode = build_inline_mode(tr, &position, provenance)?;
                inserted = insert_blocks_in_textblock_at_position(tr, position, &plan, &mode)?;
                Ok(())
            })?;
            inserted
        }
        SliceInsertionKind::HoistBlocks { plan } => {
            insert_hoisted_blocks_at_position(tr, position, plan)?
        }
        SliceInsertionKind::BlockBoundary {
            blocks,
            list_merges,
        } => {
            let position = materialize_repair_position(tr, position)?;
            insert_blocks_at_block_boundary(tr, position, blocks, list_merges)?
        }
    };
    let execution = execution.ok_or_else(|| {
        CommandError::Corrupted("planned Slice insertion produced no insertion output".into())
    })?;
    bind_planned_output_nodes(tr, &output, execution)
}

fn bind_planned_output_nodes(
    tr: &Transaction,
    output: &SliceInsertionOutputPlan,
    execution: SliceInsertionExecution,
) -> Result<AppliedSliceInsertion, CommandError> {
    let SliceInsertionExecution {
        outputs,
        split_left,
        split_right,
    } = execution;
    let mut outputs = outputs.into_iter();
    let mut caret = None;
    let mut anchor = None;
    let mut head = None;
    for spec in &output.nodes {
        let dot = match &spec.source {
            SliceOutputSource::SplitLeft => split_left,
            SliceOutputSource::SplitRight => split_right,
            source => {
                let output = outputs.next().ok_or_else(|| {
                    CommandError::Corrupted(
                        "planned Slice output node was not bound by its insertion action".into(),
                    )
                })?;
                if &output.source != source {
                    return Err(CommandError::Corrupted(
                        "authored Slice output came from a different source path".into(),
                    ));
                }
                Some(output.dot)
            }
        }
        .ok_or_else(|| {
            CommandError::Corrupted(
                "planned Slice output node was not bound by its insertion action".into(),
            )
        })?;
        let actual_type =
            resolve_output_node_type(tr, dot).ok_or(CommandError::NodeNotFound(dot))?;
        if actual_type != spec.node_type {
            return Err(CommandError::Corrupted(
                "planned Slice output node has a different type".into(),
            ));
        }
        bind_planned_output_position(&mut caret, &output.caret, &spec.source, dot)?;
        bind_planned_output_position(&mut anchor, &output.anchor, &spec.source, dot)?;
        bind_planned_output_position(&mut head, &output.head, &spec.source, dot)?;
    }
    if outputs.next().is_some() {
        return Err(CommandError::Corrupted(
            "Slice insertion authored an undeclared output node".into(),
        ));
    }
    let require_bound = |position: Option<BoundSliceOutputPosition>| {
        position.ok_or_else(|| {
            CommandError::Corrupted(
                "planned Slice endpoint was not bound by its insertion action".into(),
            )
        })
    };
    Ok(AppliedSliceInsertion {
        caret: require_bound(caret)?,
        anchor: require_bound(anchor)?,
        head: require_bound(head)?,
    })
}

fn bind_planned_output_position(
    bound: &mut Option<BoundSliceOutputPosition>,
    planned: &SliceOutputPositionSpec,
    source: &SliceOutputSource,
    dot: Dot,
) -> Result<(), CommandError> {
    if source != &planned.source {
        return Ok(());
    }
    if bound.is_some() {
        return Err(CommandError::Corrupted(
            "planned Slice endpoint was bound more than once".into(),
        ));
    }
    *bound = Some(BoundSliceOutputPosition {
        dot,
        relation: planned.relation,
        affinity: planned.affinity,
    });
    Ok(())
}

fn resolve_output_node_type(
    tr: &Transaction,
    dot: editor_crdt::Dot,
) -> Option<editor_model::NodeType> {
    let view = tr.view();
    let live = resolve_live_slice_output_dot(&view, dot)?;
    if let Some(node) = view.node(live) {
        return Some(node.node_type());
    }
    view.leaf(live).map(|leaf| leaf.node_type())
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::{
        AliasOp, AliasRun, ChildView, EditOp, Fragment, PlainBulletListNode,
        PlainHorizontalRuleNode, PlainListItemNode, PlainNode, PlainParagraphNode, PlainTextNode,
    };
    use editor_state::{Affinity, Position, Selection};

    use super::*;

    fn text_slice(text: &str) -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn paragraph_slice(text: &str) -> Slice {
        Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: text.into(),
                }))],
            }],
            open_start: 0,
            open_end: 0,
        }
    }

    #[test]
    fn inline_text_into_paragraph_is_direct_inline() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let position = Position::new(p1, 2);
        let outcome = fit_slice(&state, Selection::collapsed(position), text_slice("x")).unwrap();
        let FitOutcome::Plan(plan) = outcome else {
            panic!("inline insertion must fit");
        };
        let SliceFitPlanKind::Linear(linear) = plan.kind else {
            panic!("inline insertion must be linear");
        };
        let LinearMutation::PointInsertion { insertion } = linear.mutation else {
            panic!("inline insertion must be a point insertion");
        };
        assert!(matches!(
            insertion.kind,
            SliceInsertionKind::DirectInline { .. }
        ));
    }

    #[test]
    fn binding_rejects_reordered_same_type_inline_outputs() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        let position = Position::new(p1, 1);
        let fragments = text_slice("XY").content;
        let output = planned_inline_output(&fragments, position.affinity).unwrap();
        let mut tr = Transaction::new(&state);
        let mode = build_inline_mode(&mut tr, &position, SliceProvenance::Formatted).unwrap();
        let mut execution =
            insert_content_as_inline_at_position(&mut tr, position, fragments, &mode)
                .unwrap()
                .unwrap();
        execution.outputs.swap(0, 1);

        assert!(matches!(
            bind_planned_output_nodes(&tr, &output, execution),
            Err(CommandError::Corrupted(_))
        ));
    }

    #[test]
    fn binding_preserves_raw_output_identity_until_final_resolution() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("abc") } } }
            selection: (p1, 0)
        };
        let chars = initial
            .view()
            .node(p1)
            .expect("paragraph")
            .children()
            .filter_map(|child| match child {
                ChildView::Leaf(leaf) => Some(leaf.dot()),
                ChildView::Block(_) => None,
            })
            .collect::<Vec<_>>();
        let [raw, first_live, final_live] = chars.as_slice() else {
            panic!("expected three inline leaves");
        };

        let mut deletion = Transaction::new(&initial);
        deletion.remove_text(p1, 0, 1).unwrap();
        let (mut state, ..) = deletion.commit();
        for live in [first_live, final_live] {
            state
                .projected_mut()
                .apply(EditOp::Alias(AliasOp {
                    pairs: vec![AliasRun {
                        old_start: *raw,
                        len: 1,
                        new_start: *live,
                    }],
                }))
                .unwrap();
        }

        let tr = Transaction::new(&state);
        let view = tr.view();
        assert_eq!(
            view.alias_classes().resolve_with(*raw, |candidate| {
                view.node(candidate).is_some() || view.block_of(candidate).is_some()
            }),
            *final_live,
            "the fixture must have more than one live alias"
        );
        let output = planned_inline_output(&text_slice("x").content, Affinity::Downstream)
            .expect("one inline output");
        let execution = SliceInsertionExecution {
            outputs: vec![crate::helpers::SliceOutputBinding {
                source: SliceOutputSource::InlineSlot { index: 0 },
                dot: *raw,
            }],
            split_left: None,
            split_right: None,
        };

        let applied = bind_planned_output_nodes(&tr, &output, execution).unwrap();
        assert_eq!(applied.caret.dot, *raw);
        assert_eq!(applied.anchor.dot, *raw);
        assert_eq!(applied.head.dot, *raw);
        for bound in [&applied.caret, &applied.anchor, &applied.head] {
            assert_eq!(
                resolve_live_slice_output_dot(&view, bound.dot),
                Some(*final_live)
            );
        }
    }

    #[test]
    fn empty_slice_is_none() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let outcome = fit_slice(
            &state,
            Selection::collapsed(Position::new(p1, 2)),
            Slice {
                content: vec![],
                open_start: 0,
                open_end: 0,
            },
        )
        .unwrap();
        assert!(matches!(outcome, FitOutcome::NoOp));
    }

    #[test]
    fn block_slice_at_root_boundary_has_a_linear_plan() {
        let (state, r, ..) = state! {
            doc { r: root { paragraph { text("a") } paragraph { text("b") } } }
            selection: none
        };
        let position = Position {
            node: r,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let outcome =
            fit_slice(&state, Selection::collapsed(position), paragraph_slice("x")).unwrap();
        let FitOutcome::Plan(plan) = outcome else {
            panic!("block insertion must fit");
        };
        assert!(matches!(plan.kind, SliceFitPlanKind::Linear(_)));
    }

    #[test]
    fn inline_text_into_fold_title_is_direct_inline_or_none_consistent_with_schema() {
        let (state, t) = state! {
            doc { root {
                fold {
                    t: fold_title { text("title") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph {}
            } }
            selection: (t, 1)
        };
        let selection = Selection::collapsed(Position::new(t, 1));
        let inline = fit_slice(&state, selection, text_slice("x")).unwrap();
        let block = fit_slice(&state, selection, paragraph_slice("x")).unwrap();
        assert!(matches!(inline, FitOutcome::Plan(_)));
        match block {
            FitOutcome::NoFit => {}
            FitOutcome::Plan(plan) => {
                let SliceFitPlanKind::Linear(linear) = plan.kind else {
                    panic!("fold title insertion must be linear");
                };
                let LinearMutation::PointInsertion { insertion } = linear.mutation else {
                    panic!("fold title insertion must be a point insertion");
                };
                assert!(matches!(
                    insertion.kind,
                    SliceInsertionKind::OpenInline { .. }
                ));
            }
            FitOutcome::NoOp => panic!("fold title insertion must not be a no-op"),
        }
    }

    fn state_under_distinct_actor(source: &editor_state::State, actor: u64) -> editor_state::State {
        let plain = editor_state::to_plain(source.projected.projected());
        let (state, _) = editor_state::test_utils::build_state_from_plain_with_actor(plain, actor);
        state
    }

    #[test]
    fn missing_node_with_empty_slice_is_noop_not_error() {
        let (_foreign, f1) = state! {
            doc { root { f1: paragraph { text("zz") } } }
            selection: none
        };
        let (state_actor1, ..) = state! {
            doc { root { paragraph { text("a") } } }
            selection: none
        };
        let state = state_under_distinct_actor(&state_actor1, 2);

        let mut tr = editor_transaction::Transaction::new(&state);
        let result = crate::insert_slice_at(
            &mut tr,
            Position::new(f1, 0),
            Slice {
                content: vec![],
                open_start: 0,
                open_end: 0,
            },
            crate::types::SliceProvenance::Plain,
        );
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn missing_node_with_content_slice_propagates_node_not_found() {
        let (_foreign, f1) = state! {
            doc { root { f1: paragraph { text("zz") } } }
            selection: none
        };
        let (state_actor1, ..) = state! {
            doc { root { paragraph { text("a") } } }
            selection: none
        };
        let state = state_under_distinct_actor(&state_actor1, 2);

        let mut tr = editor_transaction::Transaction::new(&state);
        let result = crate::insert_slice_at(
            &mut tr,
            Position::new(f1, 0),
            paragraph_slice("x"),
            crate::types::SliceProvenance::Plain,
        );
        assert!(matches!(result, Err(crate::CommandError::NodeNotFound(_))));
    }

    #[test]
    fn task3_open_ancestor_fit_rejects_a_multi_root_repair_result() {
        let (state, p) = state! {
            doc { root {
                bullet_list { list_item { p: paragraph { text("target") } } }
                paragraph {}
            } }
            selection: (p, 3)
        };
        let paragraph = |text: &str| {
            Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default())).with_children(vec![
                Fragment::leaf(PlainNode::Text(PlainTextNode { text: text.into() })),
            ])
        };
        let source = Fragment::leaf(PlainNode::BulletList(PlainBulletListNode::default()))
            .with_children(vec![
                Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default())).with_children(
                    vec![
                        paragraph("left"),
                        Fragment::leaf(PlainNode::HorizontalRule(
                            PlainHorizontalRuleNode::default(),
                        )),
                        paragraph("right"),
                    ],
                ),
            ]);
        let slice = Slice {
            content: vec![source],
            open_start: 3,
            open_end: 3,
        };

        assert!(matches!(
            place_slice_at_position(&state.view(), Position::new(p, 3), slice),
            PlacementOutcome::NoFit
        ));
    }
}

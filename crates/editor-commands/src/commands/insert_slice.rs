use std::collections::HashMap;

use editor_clipboard::Slice;
use editor_state::{Position, Selection, is_unit_node_selection};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::commands::{apply_cell_fill_plan, apply_table_grid_plan};
use crate::helpers::{
    LinearJoinExecution, SliceInsertionTarget, SliceOutputPositionSpec, SliceOutputRelation,
    apply_cross_range_removal_without_join, apply_linear_deletion_plan, block_parent_and_index,
    install_planned_selection, materialize_planned_endpoint, materialize_planned_selection,
    remap_linear_deletion_plan, split_block_wrapper_before_child,
};
use crate::judgments::{
    AppliedSliceInsertion, FitOutcome, JoinedReplacementPlan, LinearFinalSelection, LinearFitPlan,
    LinearMutation, PlannedBoundaryInsertion, PlannedBranchInsertion, PlannedBranchNode,
    PlannedBranchSplit, PlannedJoin, PlannedOutputKey, RangePlacement, SliceFitPlan,
    SliceFitPlanKind, apply_slice_insertion_plan, fit_slice,
};
use crate::types::SliceProvenance;

pub fn insert_slice(
    tr: &mut Transaction,
    slice: Slice,
    provenance: SliceProvenance,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let plan = match fit_slice(tr.state(), selection, slice)? {
        FitOutcome::Plan(plan) => plan,
        FitOutcome::NoOp | FitOutcome::NoFit => return Ok(false),
    };
    let inserted = apply_fitted_slice(tr, plan, provenance)?;
    let unit = is_unit_node_selection(&inserted, &tr.view());
    if unit {
        tr.set_selection(Some(inserted))?;
    }
    Ok(true)
}

pub(crate) fn apply_fitted_slice(
    tr: &mut Transaction,
    plan: SliceFitPlan,
    provenance: SliceProvenance,
) -> Result<editor_state::Selection, crate::CommandError> {
    let mut inserted = None;
    tr.batch::<_, crate::CommandError>(|tr| {
        inserted = match plan.kind {
            SliceFitPlanKind::Linear(plan) => apply_linear_fit_plan(tr, plan, provenance)?,
            SliceFitPlanKind::TableGrid(plan) => {
                apply_table_grid_plan(tr, plan)?;
                tr.selection()
            }
            SliceFitPlanKind::CellFill(plan) => {
                apply_cell_fill_plan(tr, plan, provenance)?;
                tr.selection()
            }
        };
        if inserted.is_none() {
            return Err(crate::CommandError::Corrupted(
                "fitted Slice plan produced no observable change".into(),
            ));
        }
        Ok(())
    })?;
    inserted.ok_or_else(|| {
        crate::CommandError::Corrupted("fitted Slice plan produced no observable change".into())
    })
}

fn apply_linear_fit_plan(
    tr: &mut Transaction,
    plan: LinearFitPlan,
    provenance: SliceProvenance,
) -> Result<Option<editor_state::Selection>, crate::CommandError> {
    let LinearFitPlan {
        selection: planned_selection,
        mutation,
        final_selection,
    } = plan;
    let mut applied_insertion = None;
    let mut deletion_boundary = None;
    match mutation {
        LinearMutation::PointInsertion { insertion } => {
            let planned = install_planned_selection(tr, &planned_selection)?;
            let applied = apply_slice_insertion_plan(tr, planned.head, insertion, provenance)?;
            bind_insertion_result(&mut applied_insertion, applied)?;
        }
        LinearMutation::RangeReplacement {
            deletion,
            placement,
        } => match placement {
            RangePlacement::Joined(JoinedReplacementPlan {
                destination,
                join,
                insertion,
            }) => {
                materialize_planned_selection(tr, &planned_selection)?;
                let destination = materialize_planned_endpoint(tr, &destination)?;
                let join = join
                    .map(|join| materialize_planned_join(tr, join))
                    .transpose()?;
                let actual = current_materialized_selection(tr)?;
                let deletion = remap_linear_deletion_plan(&tr.view(), &deletion, actual)?;
                let deleted = apply_linear_deletion_plan(tr, &deletion, join.as_ref(), true)?;
                if !deleted {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice replacement did not delete its range".into(),
                    ));
                }
                if tr.view().node(destination.node).is_none() {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice replacement lost its target survivor".into(),
                    ));
                }
                let applied = apply_slice_insertion_plan(tr, destination, insertion, provenance)?;
                bind_insertion_result(&mut applied_insertion, applied)?;
            }
            RangePlacement::PreservedBoundary(insertion) => {
                materialize_planned_selection(tr, &planned_selection)?;
                let insertion = materialize_boundary_insertion(tr, insertion)?;
                let actual = current_materialized_selection(tr)?;
                let deletion = remap_linear_deletion_plan(&tr.view(), &deletion, actual)?;
                if !apply_cross_range_removal_without_join(tr, &deletion)? {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice did not remove its preserved-boundary range".into(),
                    ));
                }
                let applied = apply_boundary_insertion(tr, insertion, provenance)?;
                bind_insertion_result(&mut applied_insertion, applied)?;
            }
            RangePlacement::SeparatedBranches { boundary } => {
                materialize_planned_selection(tr, &planned_selection)?;
                let boundary = materialize_branch_insertion(tr, boundary)?;
                let actual = current_materialized_selection(tr)?;
                let deletion = remap_linear_deletion_plan(&tr.view(), &deletion, actual)?;
                if !apply_cross_range_removal_without_join(tr, &deletion)? {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice did not remove its separated range".into(),
                    ));
                }
                let applied = apply_branch_insertion(tr, boundary, provenance)?;
                bind_insertion_result(&mut applied_insertion, applied)?;
            }
            RangePlacement::SeparatedOpenEdges {
                left,
                middle,
                right,
            } => {
                materialize_planned_selection(tr, &planned_selection)?;
                let left = materialize_boundary_insertion(tr, left)?;
                let middle = materialize_branch_insertion(tr, middle)?;
                let right = materialize_boundary_insertion(tr, right)?;
                let actual = current_materialized_selection(tr)?;
                let deletion = remap_linear_deletion_plan(&tr.view(), &deletion, actual)?;
                if !apply_cross_range_removal_without_join(tr, &deletion)? {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice did not remove its open-edge range".into(),
                    ));
                }
                let left = apply_boundary_insertion(tr, left, provenance)?;
                validate_applied_insertion(tr, &left)?;
                let right = apply_boundary_insertion(tr, right, provenance)?;
                validate_applied_insertion(tr, &right)?;
                let middle = apply_branch_insertion(tr, middle, provenance)?;
                validate_applied_insertion(tr, &middle)?;
                if applied_insertion
                    .replace(AppliedLinearInsertion::SeparatedOpenEdges { left, right })
                    .is_some()
                {
                    return Err(crate::CommandError::Corrupted(
                        "Slice insertion result was bound more than once".into(),
                    ));
                }
            }
            RangePlacement::DeletionOnly { join } => {
                materialize_planned_selection(tr, &planned_selection)?;
                let join = join
                    .map(|join| materialize_planned_join(tr, join))
                    .transpose()?;
                let actual = current_materialized_selection(tr)?;
                let deletion = remap_linear_deletion_plan(&tr.view(), &deletion, actual)?;
                if !apply_linear_deletion_plan(tr, &deletion, join.as_ref(), true)? {
                    return Err(crate::CommandError::Corrupted(
                        "fitted Slice deletion-only replacement did not delete its range".into(),
                    ));
                }
                deletion_boundary = Some(
                    tr.selection()
                        .filter(|selection| selection.is_collapsed())
                        .map(|selection| selection.head)
                        .ok_or_else(|| {
                            crate::CommandError::Corrupted(
                                "Slice deletion-only plan produced no final boundary".into(),
                            )
                        })?,
                );
            }
        },
    }
    let (caret, inserted) = match final_selection {
        LinearFinalSelection::InsertedContent => {
            let applied = applied_insertion.ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "Slice plan produced no declared insertion result".into(),
                )
            })?;
            match applied {
                AppliedLinearInsertion::Single(applied) => {
                    let caret =
                        resolve_planned_output_position(tr, &applied, applied.output.caret)?;
                    let inserted = Selection::new(
                        resolve_planned_output_position(tr, &applied, applied.output.anchor)?,
                        resolve_planned_output_position(tr, &applied, applied.output.head)?,
                    );
                    validate_observed_insertion_selection(tr, &applied, caret, inserted)?;
                    (caret, inserted)
                }
                AppliedLinearInsertion::SeparatedOpenEdges { left, right } => {
                    let caret = resolve_planned_output_position(tr, &right, right.output.caret)?;
                    let inserted = Selection::new(
                        resolve_planned_output_position(tr, &left, left.output.anchor)?,
                        resolve_planned_output_position(tr, &right, right.output.head)?,
                    );
                    (caret, inserted)
                }
            }
        }
        LinearFinalSelection::DeletionBoundary => {
            if applied_insertion.is_some() {
                return Err(crate::CommandError::Corrupted(
                    "deletion-only Slice plan unexpectedly inserted content".into(),
                ));
            }
            let boundary = deletion_boundary.ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "Slice deletion-only plan produced no final boundary".into(),
                )
            })?;
            let boundary = resolve_final_position(tr, boundary)?;
            (boundary, Selection::collapsed(boundary))
        }
    };
    let caret = canonicalize_downstream_block_boundary(&tr.view(), caret);
    tr.set_selection(Some(Selection::collapsed(caret)))?;
    Ok(Some(inserted))
}

enum AppliedLinearInsertion {
    Single(AppliedSliceInsertion),
    SeparatedOpenEdges {
        left: AppliedSliceInsertion,
        right: AppliedSliceInsertion,
    },
}

fn current_materialized_selection(tr: &Transaction) -> Result<Selection, crate::CommandError> {
    let selection = tr.selection().ok_or_else(|| {
        crate::CommandError::Corrupted(
            "fitted Slice lost its materialized replacement selection".into(),
        )
    })?;
    selection.normalize(&tr.view()).ok_or_else(|| {
        crate::CommandError::Corrupted(
            "fitted Slice materialized replacement no longer resolves".into(),
        )
    })
}

fn canonicalize_downstream_block_boundary(
    view: &editor_model::DocView,
    position: Position,
) -> Position {
    if position.affinity != editor_state::Affinity::Downstream {
        return position;
    }
    let Some(parent) = view.node(position.node) else {
        return position;
    };
    let Some(editor_model::ChildView::Block(next)) = parent.child_at(position.offset) else {
        return position;
    };
    editor_state::first_cursor_position(&next).unwrap_or(position)
}

fn bind_insertion_result(
    slot: &mut Option<AppliedLinearInsertion>,
    applied: AppliedSliceInsertion,
) -> Result<(), crate::CommandError> {
    if slot
        .replace(AppliedLinearInsertion::Single(applied))
        .is_some()
    {
        return Err(crate::CommandError::Corrupted(
            "Slice insertion result was bound more than once".into(),
        ));
    }
    Ok(())
}

fn resolve_final_position(
    tr: &Transaction,
    position: Position,
) -> Result<Position, crate::CommandError> {
    position
        .resolve(&tr.view())
        .map(|resolved| resolved.position())
        .ok_or_else(|| {
            crate::CommandError::Corrupted("Slice final selection output no longer resolves".into())
        })
}

fn validate_observed_insertion_selection(
    tr: &Transaction,
    applied: &AppliedSliceInsertion,
    caret: Position,
    inserted: Selection,
) -> Result<(), crate::CommandError> {
    validate_observed_selection(
        tr,
        caret,
        inserted,
        applied.observed_caret,
        applied.observed_inserted,
    )
}

fn validate_applied_insertion(
    tr: &Transaction,
    applied: &AppliedSliceInsertion,
) -> Result<(), crate::CommandError> {
    let caret = resolve_planned_output_position(tr, applied, applied.output.caret)?;
    let inserted = Selection::new(
        resolve_planned_output_position(tr, applied, applied.output.anchor)?,
        resolve_planned_output_position(tr, applied, applied.output.head)?,
    );
    validate_observed_insertion_selection(tr, applied, caret, inserted)
}

fn validate_observed_selection(
    tr: &Transaction,
    caret: Position,
    inserted: Selection,
    observed_caret: Position,
    observed_inserted: Selection,
) -> Result<(), crate::CommandError> {
    let view = tr.view();
    let normalize = |selection: Selection| selection.normalize(&view).unwrap_or(selection);
    let planned_caret = normalize(Selection::collapsed(caret));
    let observed_caret = normalize(Selection::collapsed(resolve_final_position(
        tr,
        observed_caret,
    )?));
    if planned_caret != observed_caret {
        return Err(crate::CommandError::Corrupted(format!(
            "Slice executor produced a different caret than the planned output: planned={planned_caret:?}, observed={observed_caret:?}"
        )));
    }
    let planned_inserted = normalize(inserted);
    let observed_inserted = normalize(Selection::new(
        resolve_final_position(tr, observed_inserted.anchor)?,
        resolve_final_position(tr, observed_inserted.head)?,
    ));
    if planned_inserted != observed_inserted {
        return Err(crate::CommandError::Corrupted(format!(
            "Slice executor produced a different inserted range than the planned output: planned={planned_inserted:?}, observed={observed_inserted:?}"
        )));
    }
    Ok(())
}

fn resolve_planned_output_position(
    tr: &Transaction,
    applied: &AppliedSliceInsertion,
    planned: SliceOutputPositionSpec,
) -> Result<Position, crate::CommandError> {
    let dot = applied.nodes.get(planned.node).copied().ok_or_else(|| {
        crate::CommandError::Corrupted(
            "Slice final selection referenced an unavailable planned output node".into(),
        )
    })?;
    let dot = resolve_live_output_dot(tr, dot).ok_or_else(|| {
        crate::CommandError::Corrupted(
            "Slice final selection output node no longer resolves".into(),
        )
    })?;
    let view = tr.view();
    let mut position = match planned.relation {
        SliceOutputRelation::Before | SliceOutputRelation::After => {
            let (parent, index) = output_parent_and_index(&view, dot).ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "Slice output node has no structural boundary".into(),
                )
            })?;
            Position::new(
                parent,
                index + usize::from(matches!(planned.relation, SliceOutputRelation::After)),
            )
        }
        SliceOutputRelation::AfterTerminalPageBreak => {
            let paragraph = if view
                .node(dot)
                .is_some_and(|node| node.node_type() == editor_model::NodeType::Paragraph)
            {
                dot
            } else {
                view.block_of(dot).ok_or_else(|| {
                    crate::CommandError::Corrupted(
                        "terminal PageBreak output has no paragraph".into(),
                    )
                })?
            };
            let paragraph = view.node(paragraph).ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "terminal PageBreak output paragraph no longer resolves".into(),
                )
            })?;
            let parent = paragraph.parent().ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "terminal PageBreak output paragraph has no parent".into(),
                )
            })?;
            let index = paragraph.index().ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "terminal PageBreak output paragraph has no parent slot".into(),
                )
            })?;
            parent
                .child_at(index + 1)
                .and_then(|child| match child {
                    editor_model::ChildView::Block(block) => {
                        editor_state::first_cursor_position(&block)
                    }
                    editor_model::ChildView::Leaf(_) => None,
                })
                .unwrap_or(Position::new(parent.id(), index + 1))
        }
        SliceOutputRelation::Start => {
            if let Some(node) = view.node(dot) {
                editor_state::first_cursor_position(&node).unwrap_or(Position::new(dot, 0))
            } else {
                let (parent, index) = output_parent_and_index(&view, dot).ok_or_else(|| {
                    crate::CommandError::Corrupted("Slice leaf output has no start boundary".into())
                })?;
                Position::new(parent, index)
            }
        }
        SliceOutputRelation::End => {
            if let Some(node) = view.node(dot) {
                Position::new(dot, node.children().count())
            } else {
                let (parent, index) = output_parent_and_index(&view, dot).ok_or_else(|| {
                    crate::CommandError::Corrupted("Slice leaf output has no end boundary".into())
                })?;
                Position::new(parent, index + 1)
            }
        }
    };
    position.affinity = planned.affinity;
    Ok(position)
}

fn resolve_live_output_dot(tr: &Transaction, dot: editor_crdt::Dot) -> Option<editor_crdt::Dot> {
    let view = tr.view();
    if view.node(dot).is_some() || view.block_of(dot).is_some() {
        return Some(dot);
    }
    let resolved = view.alias_classes().resolve_with(dot, |candidate| {
        view.node(candidate).is_some() || view.block_of(candidate).is_some()
    });
    (view.node(resolved).is_some() || view.block_of(resolved).is_some()).then_some(resolved)
}

fn output_parent_and_index(
    view: &editor_model::DocView,
    dot: editor_crdt::Dot,
) -> Option<(editor_crdt::Dot, usize)> {
    if let Some(node) = view.node(dot) {
        let parent = node.parent()?;
        return Some((parent.id(), node.index()?));
    }
    let parent = view.block_of(dot)?;
    let index = view.node(parent)?.children().position(
        |child| matches!(child, editor_model::ChildView::Leaf(leaf) if leaf.dot() == dot),
    )?;
    Some((parent, index))
}

fn materialize_planned_join(
    tr: &mut Transaction,
    join: PlannedJoin,
) -> Result<LinearJoinExecution, crate::CommandError> {
    let from_textblock = materialize_planned_endpoint(tr, &join.from_textblock)?.node;
    let to_textblock = materialize_planned_endpoint(tr, &join.to_textblock)?.node;
    let prune = join
        .prune
        .iter()
        .map(|endpoint| materialize_planned_endpoint(tr, endpoint).map(|position| position.node))
        .collect::<Result<Vec<_>, _>>()?;
    let container_merges = join
        .container_merges
        .iter()
        .map(|merge| {
            Ok((
                materialize_planned_endpoint(tr, &merge.target)?.node,
                materialize_planned_endpoint(tr, &merge.source)?.node,
            ))
        })
        .collect::<Result<Vec<_>, crate::CommandError>>()?;
    let affected = join
        .affected
        .iter()
        .map(|endpoint| materialize_planned_endpoint(tr, endpoint).map(|position| position.node))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LinearJoinExecution {
        from_textblock,
        to_textblock,
        trailing_page_break: None,
        prune,
        container_merges,
        affected,
    })
}

struct MaterializedBoundaryInsertion {
    destination: Position,
    target: crate::helpers::SliceInsertionTargetShape,
    insertion: crate::judgments::SliceInsertionPlan,
}

fn materialize_boundary_insertion(
    tr: &mut Transaction,
    insertion: PlannedBoundaryInsertion,
) -> Result<MaterializedBoundaryInsertion, crate::CommandError> {
    Ok(MaterializedBoundaryInsertion {
        destination: materialize_planned_endpoint(tr, &insertion.destination)?,
        target: insertion.target,
        insertion: insertion.insertion,
    })
}

fn apply_boundary_insertion(
    tr: &mut Transaction,
    insertion: MaterializedBoundaryInsertion,
    provenance: SliceProvenance,
) -> Result<AppliedSliceInsertion, crate::CommandError> {
    let actual_target = SliceInsertionTarget::from_view(&tr.view(), insertion.destination)
        .ok_or_else(|| {
            crate::CommandError::Corrupted("fitted Slice lost its preserved boundary".into())
        })?;
    if !insertion.target.matches(&actual_target) {
        return Err(crate::CommandError::Corrupted(
            "fitted Slice changed its preserved boundary".into(),
        ));
    }
    apply_slice_insertion_plan(tr, insertion.destination, insertion.insertion, provenance)
}

struct MaterializedBranchInsertion {
    parent: editor_crdt::Dot,
    splits: Vec<ResolvedBranchSplit>,
    right_boundary: ResolvedBranchNode,
    insertion: crate::judgments::SliceInsertionPlan,
}

fn materialize_branch_insertion(
    tr: &mut Transaction,
    insertion: PlannedBranchInsertion,
) -> Result<MaterializedBranchInsertion, crate::CommandError> {
    let parent = materialize_planned_endpoint(tr, &insertion.parent)?.node;
    let (splits, right_boundary) =
        materialize_branch_boundary(tr, insertion.splits, insertion.right_boundary)?;
    Ok(MaterializedBranchInsertion {
        parent,
        splits,
        right_boundary,
        insertion: insertion.insertion,
    })
}

fn apply_branch_insertion(
    tr: &mut Transaction,
    insertion: MaterializedBranchInsertion,
    provenance: SliceProvenance,
) -> Result<AppliedSliceInsertion, crate::CommandError> {
    apply_planned_branch_boundary(
        tr,
        insertion.parent,
        insertion.splits,
        insertion.right_boundary,
        insertion.insertion,
        provenance,
    )
}

enum ResolvedBranchNode {
    Existing(editor_crdt::Dot),
    Output(PlannedOutputKey),
}

struct ResolvedBranchSplit {
    wrapper: editor_crdt::Dot,
    first_right: ResolvedBranchNode,
    output: PlannedOutputKey,
}

fn materialize_branch_boundary(
    tr: &mut Transaction,
    splits: Vec<PlannedBranchSplit>,
    right_boundary: PlannedBranchNode,
) -> Result<(Vec<ResolvedBranchSplit>, ResolvedBranchNode), crate::CommandError> {
    let materialize_node =
        |tr: &mut Transaction, node: PlannedBranchNode| -> Result<_, crate::CommandError> {
            Ok(match node {
                PlannedBranchNode::Existing(endpoint) => {
                    ResolvedBranchNode::Existing(materialize_planned_endpoint(tr, &endpoint)?.node)
                }
                PlannedBranchNode::Output(output) => ResolvedBranchNode::Output(output),
            })
        };
    let mut resolved = Vec::with_capacity(splits.len());
    for split in splits {
        resolved.push(ResolvedBranchSplit {
            wrapper: materialize_planned_endpoint(tr, &split.wrapper)?.node,
            first_right: materialize_node(tr, split.first_right)?,
            output: split.output,
        });
    }
    let right_boundary = materialize_node(tr, right_boundary)?;
    Ok((resolved, right_boundary))
}

fn apply_planned_branch_boundary(
    tr: &mut Transaction,
    parent: editor_crdt::Dot,
    splits: Vec<ResolvedBranchSplit>,
    right_boundary: ResolvedBranchNode,
    insertion: crate::judgments::SliceInsertionPlan,
    provenance: SliceProvenance,
) -> Result<AppliedSliceInsertion, crate::CommandError> {
    let mut outputs = HashMap::with_capacity(splits.len());
    let resolve_node = |node: ResolvedBranchNode,
                        outputs: &HashMap<PlannedOutputKey, editor_crdt::Dot>|
     -> Result<editor_crdt::Dot, crate::CommandError> {
        match node {
            ResolvedBranchNode::Existing(node) => Ok(node),
            ResolvedBranchNode::Output(key) => outputs.get(&key).copied().ok_or_else(|| {
                crate::CommandError::Corrupted(
                    "fitted Slice boundary referenced an unavailable planned output".into(),
                )
            }),
        }
    };
    for split in splits {
        let first_right = resolve_node(split.first_right, &outputs)?;
        let (right_wrapper, _) = split_block_wrapper_before_child(tr, split.wrapper, first_right)?;
        outputs.insert(split.output, right_wrapper);
    }
    let right_boundary = resolve_node(right_boundary, &outputs)?;
    let (actual_parent, index) = block_parent_and_index(&tr.view(), right_boundary)
        .ok_or(crate::CommandError::NodeNotFound(right_boundary))?;
    if actual_parent != parent {
        return Err(crate::CommandError::Corrupted(
            "fitted Slice right boundary did not reach its planned parent".into(),
        ));
    }
    apply_slice_insertion_plan(tr, Position::new(parent, index), insertion, provenance)
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_crdt::{Dot, ListOp, sequence::Bias as SeqBias};
    use editor_macros::state;
    use editor_model::{
        Alignment, ChildView, EditOp, Fragment, Modifier, NodeType, PlainBlockquoteNode,
        PlainHorizontalRuleNode, PlainListItemNode, PlainNode, PlainOrderedListNode,
        PlainPageBreakNode, PlainParagraphNode, PlainTextNode, SeqItem,
    };
    use editor_resource::Resource;
    use editor_state::{Position, Selection, State};
    use editor_transaction::Step;

    use super::*;
    use crate::helpers::PlannedEndpoint;
    use crate::test_utils::*;

    fn root_child_dots(state: &State) -> Vec<Dot> {
        let view = state.view();
        view.root()
            .expect("root exists")
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.id(),
                ChildView::Leaf(l) => l.dot(),
            })
            .collect()
    }

    fn carry_of(state: &State, dot: Dot) -> Vec<Modifier> {
        state.projected.carry_modifiers(dot).into_values().collect()
    }

    fn block_modifiers_of(state: &State, dot: Dot) -> Vec<Modifier> {
        state
            .projected
            .block_modifiers()
            .modifiers_of(dot)
            .into_values()
            .collect()
    }

    fn invert_recorded_ops(
        state: &mut State,
        ops: &[editor_state::undo::RecordedOp],
    ) -> Vec<editor_state::undo::RecordedOp> {
        use editor_state::undo::{RecordedOp, capture_prior, invert};

        let mut out = Vec::new();
        for recorded in ops.iter().rev() {
            for payload in invert(&state.projected, recorded) {
                let prior = capture_prior(&state.projected, &payload);
                let op = state.projected_mut().apply(payload).unwrap();
                out.push(RecordedOp { op, prior });
            }
        }
        out
    }

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: text.into(),
                }))],
            }],
            open_start: 1,
            open_end: 1,
        }
    }

    #[test]
    fn non_empty_lossless_empty_slice_deletes_the_selected_text() {
        let (initial, _p) = state! {
            doc { root { p: paragraph { text("abc") } } }
            selection: (p, 1) -> (p, 2)
        };
        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice(
                &mut tr,
                Slice::new(
                    vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: String::new(),
                    }))],
                    0,
                    0,
                ),
                SliceProvenance::Formatted,
            )
            .unwrap()
        );
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root { p: paragraph { text("ac") } } }
            selection: (p, 1)
        };
        editor_state::assert_state_eq!(&actual, &expected);
    }

    fn paragraph_fragment(text: &str) -> Fragment {
        Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
        }
    }

    fn paragraph_break_slice() -> Slice {
        let empty_paragraph = || Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: vec![],
        };
        Slice {
            content: vec![empty_paragraph(), empty_paragraph()],
            open_start: 1,
            open_end: 1,
        }
    }

    fn split_step_count(steps: &[editor_transaction::StepRecord]) -> usize {
        steps
            .iter()
            .filter(|record| matches!(record.step, Step::SplitNode { .. }))
            .count()
    }

    fn open_fold_title_slice(text: &str) -> Slice {
        Slice {
            content: vec![Fragment {
                node: PlainNode::Text(PlainTextNode { text: text.into() }),
                modifiers: vec![],
                carry: vec![],
                children: vec![],
            }],
            open_start: 0,
            open_end: 0,
        }
    }

    #[test]
    fn insert_empty_slice_no_op() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let empty = Slice::new(vec![], 0, 0);
        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(
            &mut tr,
            empty,
            SliceProvenance::Formatted
        ));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn fitted_replacement_rolls_back_if_execution_diverges() {
        let (initial, p) = state! {
            doc { root { p: paragraph { text("abc") } } }
            selection: (p, 1) -> (p, 2)
        };
        let selection = initial.selection.expect("selection");
        let target = PlannedEndpoint::capture(&initial.view(), Position::new(p, 0)).unwrap();
        let FitOutcome::Plan(mut plan) =
            fit_slice(&initial, selection, root_with_paragraph("X")).unwrap()
        else {
            panic!("replacement must fit");
        };
        let SliceFitPlanKind::Linear(LinearFitPlan {
            mutation:
                LinearMutation::RangeReplacement {
                    placement: RangePlacement::Joined(JoinedReplacementPlan { destination, .. }),
                    ..
                },
            ..
        }) = &mut plan.kind
        else {
            panic!("expected joined replacement");
        };
        *destination = target;
        let mut tr = Transaction::new(&initial);

        assert!(
            apply_fitted_slice(&mut tr, plan, SliceProvenance::Formatted).is_err(),
            "the deliberately stale plan must fail"
        );
        let (actual, steps, ..) = tr.commit();

        assert_state_eq!(&actual, &initial);
        assert!(steps.is_empty(), "the failed plan must be atomic");
    }

    #[test]
    fn fitted_insertion_rolls_back_if_final_selection_contract_is_corrupted() {
        let (initial, _p) = state! {
            doc { root { p: paragraph { text("abc") } } }
            selection: (p, 1)
        };
        let FitOutcome::Plan(mut plan) = fit_slice(
            &initial,
            initial.selection.expect("selection"),
            Slice::new(
                vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "X".into(),
                }))],
                0,
                0,
            ),
        )
        .unwrap() else {
            panic!("inline insertion must fit");
        };
        let SliceFitPlanKind::Linear(LinearFitPlan {
            final_selection, ..
        }) = &mut plan.kind
        else {
            panic!("expected a linear plan");
        };
        *final_selection = LinearFinalSelection::DeletionBoundary;

        let mut tr = Transaction::new(&initial);
        assert!(
            apply_fitted_slice(&mut tr, plan, SliceProvenance::Formatted).is_err(),
            "an inconsistent planned selection contract must reject the whole execution"
        );
        let (actual, steps, ..) = tr.commit();
        assert_state_eq!(&actual, &initial);
        assert!(
            steps.is_empty(),
            "the failed output resolution must be atomic"
        );
    }

    #[test]
    fn fitted_insertion_rejects_a_corrupted_output_source_path_before_mutation() {
        let (initial, _p) = state! {
            doc { root { p: paragraph { text("abc") } } }
            selection: (p, 1)
        };
        let FitOutcome::Plan(mut plan) = fit_slice(
            &initial,
            initial.selection.expect("selection"),
            Slice::new(
                vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "X".into(),
                }))],
                0,
                0,
            ),
        )
        .unwrap() else {
            panic!("inline insertion must fit");
        };
        let SliceFitPlanKind::Linear(LinearFitPlan {
            mutation: LinearMutation::PointInsertion { insertion },
            ..
        }) = &mut plan.kind
        else {
            panic!("expected a point insertion plan");
        };
        let crate::judgments::SliceInsertionPlan::DirectInline { output, .. } = insertion else {
            panic!("expected direct inline insertion");
        };
        output.nodes[0].source = crate::helpers::SliceOutputSource::InlineSlot { index: 99 };

        let mut tr = Transaction::new(&initial);
        assert!(
            apply_fitted_slice(&mut tr, plan, SliceProvenance::Formatted).is_err(),
            "a planned output key detached from its source slot must be rejected"
        );
        let (actual, steps, ..) = tr.commit();
        assert_state_eq!(&actual, &initial);
        assert!(steps.is_empty(), "the stale output plan must be atomic");
    }

    #[test]
    fn replacement_aware_closed_block_keeps_different_list_boundaries_in_both_directions() {
        for reversed in [false, true] {
            let (mut initial, ordered, left, bullet, right) = state! {
                doc { root {
                    ordered: ordered_list {
                        list_item { left: paragraph { text("AB") } }
                    }
                    bullet: bullet_list {
                        list_item { right: paragraph { text("CD") } }
                    }
                    paragraph {}
                } }
                selection: (left, 1) -> (right, 1)
            };
            if reversed {
                initial.selection = Some(Selection::new(
                    Position::new(right, 1),
                    Position::new(left, 1),
                ));
            }
            let slice = Slice::new(
                vec![Fragment::leaf(PlainNode::HorizontalRule(
                    PlainHorizontalRuleNode::default(),
                ))],
                0,
                0,
            );

            let (actual, ..) = transact!(initial, |tr| insert_slice(
                &mut tr,
                slice,
                SliceProvenance::Formatted
            ));
            let view = actual.view();
            let root = view.root().unwrap();
            let types = root
                .children()
                .map(|child| match child {
                    ChildView::Block(block) => block.node_type(),
                    ChildView::Leaf(leaf) => leaf.node_type(),
                })
                .collect::<Vec<_>>();

            assert_eq!(
                types,
                vec![
                    NodeType::OrderedList,
                    NodeType::HorizontalRule,
                    NodeType::BulletList,
                    NodeType::Paragraph,
                ]
            );
            assert_eq!(
                view.node(ordered).unwrap().node_type(),
                NodeType::OrderedList
            );
            assert_eq!(view.node(bullet).unwrap().node_type(), NodeType::BulletList);
            assert_eq!(view.node(left).unwrap().inline_text(), "A");
            assert_eq!(view.node(right).unwrap().inline_text(), "D");
            assert!(editor_state::is_unit_node_selection(
                &actual.selection.unwrap(),
                &view
            ));
            assert_projection_integrity(&actual);
        }
    }

    #[test]
    fn insert_open_single_paragraph_into_paragraph_middle_merges_both_edges() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let slice = root_with_paragraph("XY");
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HeXYllo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 1);
    }

    #[test]
    fn insert_paragraph_break_slice_into_paragraph_middle_splits_once() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("asd") } } }
            selection: (p1, 1)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            paragraph_break_slice(),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("sd") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 1);
    }

    #[test]
    fn insert_paragraph_break_slice_at_paragraph_start_preserves_boundary() {
        let (initial, ..) = state! {
            doc { root { target: paragraph { text("x") } } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            paragraph_break_slice(),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                target: paragraph { text("x") }
            } }
            selection: (target, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn insert_paragraph_break_slice_at_paragraph_end_preserves_boundary() {
        let (initial, ..) = state! {
            doc { root { target: paragraph { text("x") } } }
            selection: (target, 1)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            paragraph_break_slice(),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("x") }
                target: paragraph {}
            } }
            selection: (target, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn insert_paragraph_break_slice_replaces_empty_destination() {
        let (initial, ..) = state! {
            doc { root { target: paragraph {} } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            paragraph_break_slice(),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                target: paragraph {}
            } }
            selection: (target, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn insert_open_paragraph_at_block_boundary_inserts_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = root_with_paragraph("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p2: paragraph { text("X") }
                paragraph { text("b") }
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_bare_inline_at_block_boundary_materializes_default_paragraph() {
        let (source, ..) = state! {
            doc { root {
                source: paragraph [alignment(Alignment::Right)] carry([italic]) {
                    text("XY") [bold]
                }
                paragraph { text("after") }
            } }
            selection: (source, 0) -> (source, 2)
        };
        let slice = Slice::extract(&source).expect("non-collapsed");
        assert!(
            slice
                .content
                .iter()
                .all(|fragment| fragment.carry.is_empty())
        );

        let (initial, ..) = state! {
            doc { root: root { paragraph { text("Z") } } }
            selection: (root, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Z") }
                inserted: paragraph { text("XY") [bold] }
            } }
            selection: (inserted, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_paragraph_text_into_fold_title_uses_open_inline_content() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("body") [bold] } } }
            selection: (p1, 0) -> (p1, 4)
        };
        let slice = Slice::extract(&source).expect("non-collapsed");

        let (initial, ..) = state! {
            doc { root { fold {
                ft: fold_title {}
                fold_content { paragraph {} }
            } } }
            selection: (ft, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { fold {
                ft1: fold_title { text("body") }
                fold_content { paragraph {} }
            } } }
            selection: (ft1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_plain_text_slice_into_fold_title_opens_paragraph_context() {
        let slice = Slice::from_text("body");
        assert_eq!((slice.open_start, slice.open_end), (1, 1));
        assert!(matches!(slice.content[0].node, PlainNode::Paragraph(_)));

        let (initial, ..) = state! {
            doc { root { fold {
                ft: fold_title { text("title") }
                fold_content { paragraph {} }
            } } }
            selection: (ft, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { fold {
                ft: fold_title { text("titlebody") }
                fold_content { paragraph {} }
            } } }
            selection: (ft, 9)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_fold_title_text_into_paragraph_uses_open_inline_content() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open_fold_title_slice("title"),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("title") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn open_edge_inline_fallback_does_not_flatten_closed_middle_textblock() {
        let (initial, ..) = state! {
            doc { root { fold {
                ft: fold_title { text("title") }
                fold_content { paragraph {} }
            } } }
            selection: (ft, 5)
        };
        let slice = Slice {
            content: vec![
                paragraph_fragment("A"),
                paragraph_fragment("M"),
                paragraph_fragment("B"),
            ],
            open_start: 1,
            open_end: 1,
        };

        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_block_slice_into_paragraph_preserves_block_structure() {
        use editor_model::{PlainBulletListNode, PlainListItemNode};
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::BulletList(PlainBulletListNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::ListItem(PlainListItemNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![paragraph_fragment("X")],
                }],
            }],
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hello") }
                bl: bullet_list { list_item { paragraph { text("X") } } }
                paragraph {}
            } }
            selection: (bl, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pasting_text_with_tab_yields_inline_tab_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let view = actual.view();
        let para = view
            .root()
            .expect("root exists")
            .child_blocks()
            .next()
            .expect("paragraph exists");
        let children: Vec<ChildView> = para.children().collect();
        assert_eq!(children.len(), 3, "paragraph must have 3 inline children");
        match &children[0] {
            ChildView::Leaf(l) => assert_eq!(l.as_char(), Some('a'), "first child must be 'a'"),
            _ => panic!("first child must be a char leaf"),
        }
        match &children[1] {
            ChildView::Leaf(l) => assert_eq!(
                l.node_type(),
                NodeType::Tab,
                "second child must be a Tab atom"
            ),
            _ => panic!("second child must be a tab leaf"),
        }
        match &children[2] {
            ChildView::Leaf(l) => assert_eq!(l.as_char(), Some('b'), "third child must be 'b'"),
            _ => panic!("third child must be a char leaf"),
        }
    }

    #[test]
    fn non_collapsed_selection_is_replaced_atomically() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let slice = Slice::from_text("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HXo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_at_block_boundary() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice {
            content: vec![paragraph_fragment("X"), paragraph_fragment("Y")],
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { text("X") }
                p3: paragraph { text("Y") }
                paragraph { text("b") }
            } }
            selection: (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_empty_paragraph_replaces_without_extra_empties() {
        use editor_model::PlainCalloutNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice {
            content: vec![
                Fragment {
                    node: PlainNode::Callout(PlainCalloutNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![paragraph_fragment("1")],
                },
                Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![],
                },
            ],
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                callout { paragraph { text("1") } }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_paragraph_middle_splits_and_merges() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 5)
        };
        let slice = Slice {
            content: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            open_start: 1,
            open_end: 1,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hellofirst") }
                p2: paragraph { text("second World") }
            } }
            selection: (p2, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn structural_insert_opens_only_edge_paragraphs_and_preserves_closed_middle() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("xy") } } }
            selection: (p1, 1)
        };
        let slice = Slice {
            content: vec![
                paragraph_fragment("A"),
                Fragment {
                    node: NodeType::Callout.into_node().to_plain(),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![paragraph_fragment("M")],
                },
                paragraph_fragment("B"),
            ],
            open_start: 1,
            open_end: 1,
        };

        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("xA") }
                callout { paragraph { text("M") } }
                p2: paragraph { text("By") }
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn structural_insert_opens_compatible_outer_context_before_merging_textblock_edges() {
        let (initial, ..) = state! {
            doc { root { blockquote { p1: paragraph { text("xy") } } } }
            selection: (p1, 1)
        };
        let slice = Slice {
            content: vec![Fragment {
                node: NodeType::Blockquote.into_node().to_plain(),
                modifiers: vec![],
                carry: vec![],
                children: vec![paragraph_fragment("A"), paragraph_fragment("B")],
            }],
            open_start: 2,
            open_end: 2,
        };

        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root { blockquote {
                paragraph { text("xA") }
                p2: paragraph { text("By") }
            } } }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn open_table_slice_at_cell_caret_is_a_whole_no_fit() {
        let wrap = |node_type: NodeType, children| Fragment {
            node: node_type.into_node().to_plain(),
            modifiers: vec![],
            carry: vec![],
            children,
        };
        let slice = Slice {
            content: vec![wrap(
                NodeType::Table,
                vec![
                    wrap(
                        NodeType::TableRow,
                        vec![wrap(NodeType::TableCell, vec![paragraph_fragment("A")])],
                    ),
                    wrap(
                        NodeType::TableRow,
                        vec![wrap(NodeType::TableCell, vec![paragraph_fragment("B")])],
                    ),
                ],
            )],
            open_start: 4,
            open_end: 4,
        };
        let (initial, ..) = state! {
            doc { root {
                table {
                    table_row {
                        table_cell {
                            target: paragraph { text("xy") }
                        }
                    }
                }
                paragraph {}
            } }
            selection: (target, 1)
            pending_modifiers: [bold]
        };

        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn empty_open_wrapper_is_noop_and_preserves_pending_format() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    target: paragraph { text("xy") }
                }
                paragraph {}
            } }
            selection: (target, 1)
            pending_modifiers: [bold]
        };
        let slice = Slice {
            content: vec![Fragment {
                node: NodeType::Blockquote.into_node().to_plain(),
                modifiers: vec![],
                carry: vec![],
                children: vec![paragraph_fragment("")],
            }],
            open_start: 2,
            open_end: 2,
        };

        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn open_page_break_that_cannot_cross_isolation_is_whole_no_fit() {
        let fixtures = [
            {
                let (state, target) = state! {
                    doc { root {
                        fold {
                            fold_title { text("title") }
                            fold_content {
                                blockquote {
                                    target: paragraph { text("ab") }
                                }
                            }
                        }
                        paragraph {}
                    } }
                    selection: (target, 0)
                    pending_modifiers: [bold]
                };
                (state, target)
            },
            {
                let (state, target) = state! {
                    doc { root {
                        table {
                            table_row {
                                table_cell {
                                    blockquote {
                                        target: paragraph { text("ab") }
                                    }
                                }
                            }
                        }
                        paragraph {}
                    } }
                    selection: (target, 0)
                    pending_modifiers: [bold]
                };
                (state, target)
            },
        ];
        let slice = Slice::new(
            vec![
                Fragment::leaf(PlainNode::Blockquote(PlainBlockquoteNode::default()))
                    .with_children(vec![
                        Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default()))
                            .with_children(vec![Fragment::leaf(PlainNode::PageBreak(
                                PlainPageBreakNode::default(),
                            ))]),
                    ]),
            ],
            2,
            2,
        );

        for (base, target) in fixtures {
            for selection in [
                Selection::collapsed(Position::new(target, 0)),
                Selection::new(Position::new(target, 0), Position::new(target, 1)),
                Selection::new(Position::new(target, 1), Position::new(target, 0)),
            ] {
                let mut initial = base.clone();
                initial.selection = Some(selection);
                assert!(matches!(
                    fit_slice(&initial, selection, slice.clone()).unwrap(),
                    FitOutcome::NoFit
                ));

                let mut tr = Transaction::new(&initial);
                assert!(!insert_slice(&mut tr, slice.clone(), SliceProvenance::Formatted).unwrap());
                let (actual, ..) = tr.commit();
                assert_state_eq!(&actual, &initial);
            }
        }
    }

    #[test]
    fn insert_open_list_slice_into_list_item_preserves_sibling_items() {
        let (source, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("first") } }
                    list_item { p2: paragraph { text("second") } }
                }
                paragraph {}
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let slice = Slice::extract(&source).expect("open list slice");

        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item {
                        target: paragraph { text("xy") }
                        paragraph { text("tail") }
                    }
                }
                paragraph {}
            } }
            selection: (target, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { text("xrst") } }
                    list_item {
                        p2: paragraph { text("secy") }
                        paragraph { text("tail") }
                    }
                }
                paragraph {}
            } }
            selection: (p2, 3)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn replace_across_list_items_with_open_list_slice_joins_both_destination_edges() {
        let (source, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { first: paragraph { text("first") } }
                    list_item { second: paragraph { text("second") } }
                }
                paragraph {}
            } }
            selection: (first, 2) -> (second, 3)
        };
        let slice = Slice::extract(&source).expect("open list slice");

        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { left: paragraph { text("AB") } }
                    list_item { right: paragraph { text("CD") } }
                }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { text("Arst") } }
                    list_item { caret: paragraph { text("secD") } }
                }
                paragraph {}
            } }
            selection: (caret, 3)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn ranged_page_break_replacement_hoists_without_losing_list_survivors() {
        let (source, ..) = state! {
            doc { source_root: root {
                paragraph { page_break }
            } }
            selection: (source_root, 0, >) -> (source_root, 1, <)
        };
        let slice = Slice::extract(&source).expect("closed page-break paragraph slice");

        let (initial, left, right) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        left: paragraph { text("a") }
                        right: paragraph { text("bd") }
                    }
                }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
            pending_modifiers: [bold]
        };
        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap(),
            "the ranged replacement must climb to the Root frontier"
        );
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(view.node(left).expect("left survivor").inline_text(), "a");
        let right = view
            .alias_classes()
            .members_of(right)
            .into_iter()
            .flatten()
            .copied()
            .find(|dot| {
                view.node(*dot)
                    .is_some_and(|node| node.inline_text() == "d")
            })
            .expect("right survivor alias");
        assert_eq!(view.node(right).expect("right survivor").inline_text(), "d");
        let page_breaks = view
            .root()
            .expect("root")
            .descendants()
            .filter(|child| {
                matches!(child, ChildView::Leaf(leaf) if leaf.node_type() == NodeType::PageBreak)
            })
            .count();
        assert_eq!(page_breaks, 1, "the admitted PageBreak is preserved once");
        assert_projection_integrity(&actual);
    }

    #[test]
    fn destination_page_break_outside_range_blocks_join_and_survives() {
        let (initial, left, right) = state! {
            doc { root {
                left: paragraph { text("a") page_break }
                right: paragraph { text("b") }
            } }
            selection: (left, 2) -> (right, 0)
        };
        let page_break = initial
            .view()
            .node(left)
            .expect("left")
            .children()
            .find_map(|child| match child {
                ChildView::Leaf(leaf) if leaf.node_type() == NodeType::PageBreak => {
                    Some(leaf.dot())
                }
                _ => None,
            })
            .expect("page break");

        let mut tr = Transaction::new(&initial);
        assert!(insert_slice(&mut tr, Slice::from_text("X"), SliceProvenance::Formatted,).unwrap());
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert!(
            view.leaf(page_break).is_some(),
            "the PageBreak dot survives"
        );
        assert_eq!(view.node(left).expect("left survives").inline_text(), "a");
        assert_eq!(
            view.node(right).expect("right survives").inline_text(),
            "Xb"
        );
        assert_projection_integrity(&actual);
    }

    #[test]
    fn asymmetric_closed_start_keeps_left_boundary_and_opens_into_right_boundary() {
        let (source, ..) = state! {
            doc { source_root: root {
                image
                source_paragraph: paragraph { text("X") }
            } }
            selection: (source_root, 0, >) -> (source_paragraph, 1, <)
        };
        let slice = Slice::extract(&source).expect("asymmetric Slice");
        assert_eq!((slice.open_start, slice.open_end), (0, 1));
        assert_eq!(
            slice
                .content
                .iter()
                .map(|fragment| fragment.node.as_type())
                .collect::<Vec<_>>(),
            [NodeType::Image, NodeType::Paragraph]
        );

        let (initial, left, right) = state! {
            doc { root {
                left: paragraph { text("ab") }
                right: paragraph { text("cd") }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
        };
        let mut tr = Transaction::new(&initial);
        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let child_types = view
            .root()
            .expect("root")
            .children()
            .map(|child| match child {
                ChildView::Block(block) => block.node_type(),
                ChildView::Leaf(leaf) => leaf.node_type(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            child_types,
            [
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::Paragraph,
                NodeType::Paragraph,
            ]
        );
        assert_eq!(view.node(left).expect("left survivor").inline_text(), "a");
        let right_members = view
            .alias_classes()
            .members_of(right)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let right = right_members
            .iter()
            .copied()
            .find(|dot| view.node(*dot).is_some())
            .unwrap_or(right);
        assert_eq!(
            view.node(right)
                .unwrap_or_else(|| {
                    panic!(
                        "right survivor missing; aliases={right_members:?}, root={child_types:?}"
                    )
                })
                .inline_text(),
            "Xd"
        );
        assert_projection_integrity(&actual);
    }

    #[test]
    fn asymmetric_open_start_opens_into_left_boundary_and_keeps_right_boundary() {
        let (source, ..) = state! {
            doc { source_root: root {
                source_paragraph: paragraph { text("X") }
                image
            } }
            selection: (source_paragraph, 0, >) -> (source_root, 2, <)
        };
        let slice = Slice::extract(&source).expect("asymmetric Slice");
        assert_eq!((slice.open_start, slice.open_end), (1, 0));
        assert_eq!(
            slice
                .content
                .iter()
                .map(|fragment| fragment.node.as_type())
                .collect::<Vec<_>>(),
            [NodeType::Paragraph, NodeType::Image]
        );

        let (initial, left, right) = state! {
            doc { root {
                left: paragraph { text("ab") }
                right: paragraph { text("cd") }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
        };
        let mut tr = Transaction::new(&initial);
        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let child_types = view
            .root()
            .expect("root")
            .children()
            .map(|child| match child {
                ChildView::Block(block) => block.node_type(),
                ChildView::Leaf(leaf) => leaf.node_type(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            child_types,
            [
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::Paragraph,
                NodeType::Paragraph,
            ]
        );
        assert_eq!(view.node(left).expect("left survivor").inline_text(), "aX");
        let right_members = view
            .alias_classes()
            .members_of(right)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let right = right_members
            .iter()
            .copied()
            .find(|dot| view.node(*dot).is_some())
            .unwrap_or(right);
        assert_eq!(
            view.node(right)
                .unwrap_or_else(|| {
                    panic!(
                        "right survivor missing; aliases={right_members:?}, root={child_types:?}"
                    )
                })
                .inline_text(),
            "d"
        );
        assert_projection_integrity(&actual);
    }

    #[test]
    fn open_edges_with_a_closed_middle_block_keep_distinct_list_kinds() {
        let (initial, ordered, left, bullet, right) = state! {
            doc { root {
                ordered: ordered_list {
                    list_item { left: paragraph { text("AB") } }
                }
                bullet: bullet_list {
                    list_item { right: paragraph { text("CD") } }
                }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
        };
        let slice = Slice::new(
            vec![
                paragraph_fragment("x"),
                Fragment::leaf(PlainNode::HorizontalRule(PlainHorizontalRuleNode::default())),
                paragraph_fragment("y"),
            ],
            1,
            1,
        );

        let mut tr = Transaction::new(&initial);
        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let types = view
            .root()
            .expect("root")
            .children()
            .map(|child| match child {
                ChildView::Block(block) => block.node_type(),
                ChildView::Leaf(leaf) => leaf.node_type(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            types,
            [
                NodeType::OrderedList,
                NodeType::HorizontalRule,
                NodeType::BulletList,
                NodeType::Paragraph,
            ]
        );
        assert_eq!(
            view.node(ordered).expect("ordered survivor").node_type(),
            NodeType::OrderedList
        );
        assert_eq!(view.node(left).expect("left survivor").inline_text(), "Ax");
        assert_eq!(
            view.node(bullet).expect("bullet survivor").node_type(),
            NodeType::BulletList
        );
        let right = view
            .alias_classes()
            .members_of(right)
            .into_iter()
            .flatten()
            .copied()
            .find(|dot| view.node(*dot).is_some())
            .unwrap_or(right);
        assert_eq!(
            view.node(right).expect("right survivor").inline_text(),
            "yD"
        );
        assert_projection_integrity(&actual);
    }

    #[test]
    fn open_edges_are_inserted_before_a_middle_list_merges_their_containers() {
        let (initial, _left, _right) = state! {
            doc { root {
                ordered_list {
                    list_item { left: paragraph { text("AB") } }
                }
                bullet_list {
                    list_item { right: paragraph { text("CD") } }
                }
                paragraph {}
            } }
            selection: (left, 1) -> (right, 1)
        };
        let middle = Fragment::leaf(PlainNode::OrderedList(PlainOrderedListNode::default()))
            .with_children(vec![
                Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default()))
                    .with_children(vec![paragraph_fragment("M")]),
            ]);
        let slice = Slice::new(
            vec![paragraph_fragment("x"), middle, paragraph_fragment("y")],
            1,
            1,
        );

        let mut tr = Transaction::new(&initial);
        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, _, recorded, ..) = tr.commit();

        {
            let view = actual.view();
            let list = view
                .root()
                .expect("root")
                .child_blocks()
                .next()
                .expect("merged list");
            assert_eq!(list.node_type(), NodeType::OrderedList);
            let mut texts = Vec::new();
            for item in list.child_blocks() {
                texts.extend(item.child_blocks().map(|paragraph| paragraph.inline_text()));
            }
            assert_eq!(texts, ["Ax", "M", "yD"]);
        }
        assert_projection_integrity(&actual);

        let mut restored = actual.clone();
        let redo = invert_recorded_ops(&mut restored, &recorded);
        assert_eq!(restored.to_plain(), initial.to_plain());
        assert_projection_integrity(&restored);

        invert_recorded_ops(&mut restored, &redo);
        assert_eq!(restored.to_plain(), actual.to_plain());
        assert_projection_integrity(&restored);
    }

    #[test]
    fn over_budget_programmatic_slice_is_whole_no_fit() {
        let (initial, ..) = state! {
            doc { root { target: paragraph {} } }
            selection: (target, 0)
            pending_modifiers: [bold]
        };
        let mut inner = paragraph_fragment("deep");
        for _ in 0..31 {
            inner = Fragment::leaf(PlainNode::Blockquote(PlainBlockquoteNode::default()))
                .with_children(vec![inner]);
        }

        let mut tr = Transaction::new(&initial);
        assert!(
            !insert_slice(
                &mut tr,
                Slice::new(vec![inner], 0, 0),
                SliceProvenance::Formatted,
            )
            .unwrap()
        );
        let (actual, ..) = tr.commit();
        assert_state_eq!(&actual, &initial);
    }

    fn inject_root_list_item(state: &mut State, before: Dot, text: &str) -> (Dot, Dot, Dot) {
        let pos = state
            .projected
            .seq_boundary_pos(before, SeqBias::Before)
            .expect("root insertion boundary");
        let raw_item = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        let raw_paragraph = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: pos + 1,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, raw_item],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        for (index, ch) in text.chars().enumerate() {
            state
                .projected_mut()
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: pos + 2 + index,
                    item: SeqItem::Char(ch),
                }))
                .unwrap();
        }
        let synthetic_list = state
            .view()
            .root()
            .expect("root")
            .child_blocks()
            .find(|node| node.node_type() == NodeType::BulletList)
            .expect("projected list")
            .id();
        assert!(synthetic_list.is_synthetic());
        (raw_item, raw_paragraph, synthetic_list)
    }

    fn insert_raw_block(
        state: &mut State,
        pos: usize,
        node_type: NodeType,
        parents: Vec<Dot>,
    ) -> Dot {
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Block {
                    node_type,
                    parents,
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id
    }

    fn insert_raw_text(state: &mut State, mut pos: usize, text: &str) -> usize {
        for ch in text.chars() {
            state
                .projected_mut()
                .apply(EditOp::Seq(ListOp::Ins {
                    pos,
                    item: SeqItem::Char(ch),
                }))
                .unwrap();
            pos += 1;
        }
        pos
    }

    fn inject_list_with_direct_paragraphs(state: &mut State, before: Dot) -> (Dot, Dot, Dot) {
        let mut pos = state
            .projected
            .seq_boundary_pos(before, SeqBias::Before)
            .expect("root insertion boundary");
        let list = insert_raw_block(state, pos, NodeType::BulletList, vec![Dot::ROOT]);
        pos += 1;
        let right = insert_raw_block(state, pos, NodeType::Paragraph, vec![Dot::ROOT, list]);
        pos = insert_raw_text(state, pos + 1, "CD");
        let tail = insert_raw_block(state, pos, NodeType::Paragraph, vec![Dot::ROOT, list]);
        insert_raw_text(state, pos + 1, "TAIL");
        (list, right, tail)
    }

    fn closed_page_break_slice() -> Slice {
        let (source, _source_root) = state! {
            doc { source_root: root {
                paragraph { page_break }
            } }
            selection: (source_root, 0, >) -> (source_root, 1, <)
        };
        Slice::extract(&source).expect("closed page-break paragraph slice")
    }

    #[test]
    fn joined_replacement_preserves_all_content_owned_by_synthetic_list_items() {
        for reversed in [false, true] {
            let (mut initial, left, trailing) = state! {
                doc { root {
                    ordered_list {
                        list_item { left: paragraph { text("AB") } }
                    }
                    trailing: paragraph {}
                } }
                selection: none
            };
            let (_later, right, tail) = inject_list_with_direct_paragraphs(&mut initial, trailing);
            initial.selection = Some(if reversed {
                Selection::new(Position::new(right, 1), Position::new(left, 1))
            } else {
                Selection::new(Position::new(left, 1), Position::new(right, 1))
            });

            let mut tr = Transaction::new(&initial);
            assert!(
                insert_slice(&mut tr, Slice::from_text("X"), SliceProvenance::Formatted,).unwrap(),
                "the accepted joined replacement must execute"
            );
            let (actual, _, recorded, ..) = tr.commit();

            let view = actual.view();
            let list = view
                .root()
                .expect("root")
                .child_blocks()
                .next()
                .expect("earlier list survives");
            assert_eq!(list.node_type(), NodeType::OrderedList);
            let texts = list
                .child_blocks()
                .flat_map(|item| {
                    item.child_blocks()
                        .map(|paragraph| paragraph.inline_text())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(texts, ["AXD", "TAIL"]);
            let tail = view
                .alias_classes()
                .resolve_with(tail, |dot| view.node(dot).is_some());
            assert!(
                view.node(tail).is_some(),
                "the untouched authored tail remains reachable through its alias"
            );
            assert_projection_integrity(&actual);

            let mut restored = actual.clone();
            let redo = invert_recorded_ops(&mut restored, &recorded);
            assert_eq!(restored.to_plain(), initial.to_plain());
            assert_projection_integrity(&restored);
            invert_recorded_ops(&mut restored, &redo);
            assert_eq!(restored.to_plain(), actual.to_plain());
            assert_projection_integrity(&restored);
        }
    }

    #[test]
    fn page_break_hoist_keeps_content_owning_synthetic_suffix_on_the_right() {
        let (mut initial, quote, paragraph, trailing) = state! {
            doc { root {
                quote: blockquote {
                    paragraph: paragraph { text("AB") }
                }
                trailing: paragraph {}
            } }
            selection: none
        };
        let mut pos = initial
            .projected
            .seq_boundary_pos(trailing, SeqBias::Before)
            .expect("blockquote insertion boundary");
        let item = insert_raw_block(
            &mut initial,
            pos,
            NodeType::ListItem,
            vec![Dot::ROOT, quote],
        );
        pos += 1;
        let tail = insert_raw_block(
            &mut initial,
            pos,
            NodeType::Paragraph,
            vec![Dot::ROOT, quote, item],
        );
        insert_raw_text(&mut initial, pos + 1, "TAIL");
        let paragraph = {
            let view = initial.view();
            view.alias_classes()
                .resolve_with(paragraph, |dot| view.node(dot).is_some())
        };
        initial.selection = Some(Selection::collapsed(Position::new(paragraph, 1)));
        assert!(
            initial
                .selection
                .expect("selection")
                .resolve(&initial.view())
                .is_some(),
            "the repaired fixture caret must resolve before fitting"
        );

        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice(
                &mut tr,
                closed_page_break_slice(),
                SliceProvenance::Formatted,
            )
            .unwrap()
        );
        let (actual, _, recorded, ..) = tr.commit();

        let view = actual.view();
        let root_blocks = view
            .root()
            .expect("root")
            .child_blocks()
            .collect::<Vec<_>>();
        assert_eq!(
            root_blocks
                .iter()
                .map(|block| block.node_type())
                .collect::<Vec<_>>(),
            [
                NodeType::Blockquote,
                NodeType::Paragraph,
                NodeType::Blockquote,
                NodeType::Paragraph,
            ]
        );
        let descendant_text = |block: &editor_model::NodeView<'_>| {
            block
                .descendants()
                .filter_map(|child| match child {
                    ChildView::Leaf(leaf) => leaf.as_char(),
                    ChildView::Block(_) => None,
                })
                .collect::<String>()
        };
        assert_eq!(descendant_text(&root_blocks[0]), "A");
        assert_eq!(descendant_text(&root_blocks[2]), "BTAIL");
        let tail = view
            .alias_classes()
            .resolve_with(tail, |dot| view.node(dot).is_some());
        let tail_ancestor = view
            .node(tail)
            .expect("tail survives")
            .ancestors()
            .find(|ancestor| ancestor.node_type() == NodeType::Blockquote)
            .expect("tail remains under a blockquote");
        assert_eq!(tail_ancestor.id(), root_blocks[2].id());
        assert_projection_integrity(&actual);

        let mut restored = actual.clone();
        let redo = invert_recorded_ops(&mut restored, &recorded);
        assert_eq!(restored.to_plain(), initial.to_plain());
        assert_projection_integrity(&restored);
        invert_recorded_ops(&mut restored, &redo);
        assert_eq!(restored.to_plain(), actual.to_plain());
        assert_projection_integrity(&restored);
    }

    #[test]
    fn mixed_empty_and_non_empty_inline_fragments_ignore_zero_output_text() {
        for (reversed, empty_first) in [(false, true), (false, false), (true, true), (true, false)]
        {
            let (mut initial, paragraph) = state! {
                doc { root {
                    paragraph: paragraph { text("ab") }
                } }
                selection: none
            };
            initial.selection = Some(if reversed {
                Selection::new(Position::new(paragraph, 2), Position::new(paragraph, 1))
            } else {
                Selection::new(Position::new(paragraph, 1), Position::new(paragraph, 2))
            });
            let empty = Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: String::new(),
            }));
            let text = Fragment::leaf(PlainNode::Text(PlainTextNode { text: "X".into() }));
            let content = if empty_first {
                vec![empty, text]
            } else {
                vec![text, empty]
            };

            let mut tr = Transaction::new(&initial);
            assert!(
                insert_slice(
                    &mut tr,
                    Slice::new(content, 0, 0),
                    SliceProvenance::Formatted,
                )
                .unwrap()
            );
            let (actual, ..) = tr.commit();
            assert_eq!(
                actual
                    .view()
                    .node(paragraph)
                    .expect("paragraph")
                    .inline_text(),
                "aX"
            );
            assert_projection_integrity(&actual);
        }
    }

    #[test]
    fn projected_content_owning_list_slot_materializes_before_slice_insertion() {
        let (mut initial, tail) = state! {
            doc { root { tail: paragraph {} } }
            selection: none
        };
        let (raw_item, raw_paragraph, synthetic_list) =
            inject_root_list_item(&mut initial, tail, "A");
        initial.selection = Some(Selection::collapsed(Position::new(synthetic_list, 1)));
        let slice = Slice::new(
            vec![
                Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default()))
                    .with_children(vec![paragraph_fragment("B")]),
            ],
            0,
            0,
        );

        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap(),
            "the accepted projected-slot plan must execute"
        );
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let remapped_item = view
            .alias_classes()
            .resolve_with(raw_item, |dot| view.node(dot).is_some());
        let remapped_paragraph = view
            .alias_classes()
            .resolve_with(raw_paragraph, |dot| view.node(dot).is_some());
        assert!(
            view.node(remapped_item).is_some() && view.node(remapped_paragraph).is_some(),
            "the original authored list content survives through explicit aliases"
        );
        let list = view
            .root()
            .expect("root")
            .child_blocks()
            .find(|node| node.node_type() == NodeType::BulletList)
            .expect("materialized list");
        assert!(!list.id().is_synthetic());
        let mut texts = Vec::new();
        for item in list.child_blocks() {
            texts.extend(item.child_blocks().map(|paragraph| paragraph.inline_text()));
        }
        assert_eq!(texts, ["A", "B"]);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn adjacent_synthetic_list_participant_is_whole_no_fit() {
        for insertion_offset in [0, 1] {
            let (mut initial, root, tail) = state! {
                doc { root: root {
                    tail: paragraph {}
                } }
                selection: none
                pending_modifiers: [bold]
            };
            let (_, _, synthetic_list) = inject_root_list_item(&mut initial, tail, "A");
            assert!(synthetic_list.is_synthetic());
            let selection = Selection::collapsed(Position::new(root, insertion_offset));
            initial.selection = Some(selection);
            let slice = Slice::new(
                vec![
                    Fragment::leaf(PlainNode::OrderedList(PlainOrderedListNode::default()))
                        .with_children(vec![
                            Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default()))
                                .with_children(vec![paragraph_fragment("B")]),
                        ]),
                ],
                0,
                0,
            );

            assert!(matches!(
                fit_slice(&initial, selection, slice.clone()).unwrap(),
                FitOutcome::NoFit
            ));
            let mut tr = Transaction::new(&initial);
            assert!(!insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
            let (actual, ..) = tr.commit();
            assert_state_eq!(&actual, &initial);
        }
    }

    #[test]
    fn structure_budget_boundary_executes_and_cold_projects_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let (initial, _root) = state! {
                    doc { root: root {
                        paragraph {}
                    } }
                    selection: (root, 0)
                };
                let mut inner = paragraph_fragment("deep");
                for _ in 0..15 {
                    inner = Fragment::leaf(PlainNode::OrderedList(PlainOrderedListNode::default()))
                        .with_children(vec![
                            Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default()))
                                .with_children(vec![paragraph_fragment("level"), inner]),
                        ]);
                }
                let slice = Slice::new(vec![inner], 0, 0);
                assert!(slice.preflight().is_some(), "fixture must be admitted");

                let mut tr = Transaction::new(&initial);
                assert!(
                    insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap(),
                    "a Slice at the structure budget must execute"
                );
                let (actual, ..) = tr.commit();
                assert_projection_integrity(&actual);
            })
            .expect("spawn structure-budget test")
            .join()
            .expect("admitted Slice must remain stack-safe");
    }

    #[test]
    fn range_in_authored_content_under_a_synthetic_wrapper_materializes_losslessly() {
        let (mut initial, tail) = state! {
            doc { root { tail: paragraph {} } }
            selection: none
        };
        let (_raw_item, raw_paragraph, synthetic_list) =
            inject_root_list_item(&mut initial, tail, "AB");
        initial.selection = Some(Selection::new(
            Position::new(raw_paragraph, 0),
            Position::new(raw_paragraph, 1),
        ));

        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice(&mut tr, Slice::from_text("X"), SliceProvenance::Formatted,).unwrap(),
            "the replacement must bind its remapped authored endpoints"
        );
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert!(
            view.node(synthetic_list).is_none(),
            "the projection scaffold is replaced by authored structure"
        );
        let paragraph = view
            .alias_classes()
            .members_of(raw_paragraph)
            .into_iter()
            .flatten()
            .copied()
            .find_map(|dot| view.node(dot).filter(|node| node.inline_text() == "XB"))
            .expect("the authored paragraph survives through its alias");
        assert_eq!(paragraph.inline_text(), "XB");
        assert_projection_integrity(&actual);
    }

    #[test]
    fn closed_block_replacement_materializes_synthetic_endpoint_before_planning() {
        let (mut initial, _root, left) = state! {
            doc { root: root {
                left: paragraph { text("A") }
                image
            } }
            selection: none
        };
        let trailing = {
            let view = initial.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Paragraph && block.id().is_synthetic())
                .map(|block| block.id())
                .expect("synthetic trailing paragraph")
        };
        initial.selection = Some(Selection::new(
            Position::new(left, 1),
            Position::new(trailing, 0),
        ));
        let slice = Slice::new(vec![paragraph_fragment("X")], 0, 0);
        let FitOutcome::Plan(planned) = fit_slice(
            &initial,
            initial.selection.expect("selection"),
            slice.clone(),
        )
        .unwrap() else {
            panic!("closed block replacement must fit");
        };
        let SliceFitPlanKind::Linear(LinearFitPlan {
            final_selection, ..
        }) = planned.kind
        else {
            panic!("replacement must declare a linear final selection");
        };
        assert!(matches!(
            final_selection,
            LinearFinalSelection::InsertedContent
        ));

        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("A") }
                inserted: paragraph { text("X") }
                paragraph {}
            } }
            selection: (inserted, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn architecture_a_inline_range_ending_at_projected_slot_is_replaced() {
        let (mut initial, _root, left) = state! {
            doc { root: root {
                left: paragraph { text("A") }
                image
            } }
            selection: none
        };
        let trailing = {
            let view = initial.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Paragraph && block.id().is_synthetic())
                .map(|block| block.id())
                .expect("synthetic trailing paragraph")
        };
        initial.selection = Some(Selection::new(
            Position::new(left, 1),
            Position::new(trailing, 0),
        ));

        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            Slice::new(
                vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "X".into(),
                }))],
                0,
                0,
            ),
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                inserted: paragraph { text("AX") }
            } }
            selection: (inserted, 2)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn insert_image_at_text_middle_splits_paragraph_and_inserts() {
        use editor_model::PlainImageNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let slice = Slice {
            content: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { text("hel") }
                image
                paragraph { text("lo") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_into_empty_paragraph_replaces_it() {
        use editor_model::PlainImageNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice {
            content: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { r: root {
                image
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_slice_materializes_synthetic_trailing_paragraph() {
        use editor_model::PlainImageNode;
        use editor_state::{Position, Selection};

        let (initial, ..) = state! {
            doc { root { image } }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(
            synth_p.is_synthetic(),
            "trailing paragraph must be synthetic"
        );

        let slice = Slice {
            content: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            open_start: 0,
            open_end: 0,
        };
        let mut tr = Transaction::new(&initial);
        tr.set_selection(Some(Selection::collapsed(Position::new(synth_p, 0))))
            .unwrap();

        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { r: root {
                image
                image
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn formatted_slice_insert_preserves_pending_modifiers() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::Text(PlainTextNode { text: "XY".into() }),
                    modifiers: vec![Modifier::Italic],
                    carry: vec![],
                    children: vec![],
                }],
            }],
            open_start: 1,
            open_end: 1,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("He")
                        text("XY") [italic]
                        text("llo")
                    }
                }
            }
            selection: (p1, 4)
            pending_modifiers: [bold]
        };
        assert_state_eq!(&actual, &expected);
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn round_trip_paint_block_format_and_carry_survive_full_copy_paste() {
        let (source, ..) = state! {
            doc { r: root {
                s1: paragraph { text("A") [bold] }
                s2: paragraph { text("B") [link(href: "https://e.com".to_string())] }
                s3: paragraph { text("C") [font_size(2000)] }
                s4: paragraph carry([bold]) { text("D") }
            } }
            selection: (r, 0, >) -> (r, 4, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        assert!(
            original.content[3]
                .carry
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "sanity: extracted carry paragraph carries bold"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            original.clone(),
            SliceProvenance::Formatted
        ));

        let root = actual.view().root().expect("root exists").id();
        let reextracted = {
            let sel = Selection::new(Position::new(root, 1), Position::new(root, 5));
            let pasted = State {
                selection: Some(sel),
                ..actual
            };
            Slice::extract(&pasted).expect("re-extract pasted blocks")
        };
        assert_eq!(
            reextracted.content, original.content,
            "paint, block format, and carry all survive the copy-paste round trip"
        );
    }

    #[test]
    fn round_trip_center_aligned_carry_paragraph_preserves_alignment_and_carry() {
        let (source, ..) = state! {
            doc { r: root {
                s1: paragraph [alignment(Alignment::Center)] carry([bold]) { text("X") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        assert!(
            original.content[0].modifiers.iter().any(|m| matches!(
                m,
                Modifier::Alignment {
                    value: Alignment::Center
                }
            )),
            "sanity: extracted paragraph is center-aligned"
        );
        assert!(
            original.content[0]
                .carry
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "sanity: extracted paragraph carries bold"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            original,
            SliceProvenance::Formatted
        ));

        let pasted = root_child_dots(&actual)[1];
        assert!(
            carry_of(&actual, pasted)
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "carry survives paste, got {:?}",
            carry_of(&actual, pasted)
        );
        assert!(
            block_modifiers_of(&actual, pasted).iter().any(|m| matches!(
                m,
                Modifier::Alignment {
                    value: Alignment::Center
                }
            )),
            "alignment (block format) survives paste, got {:?}",
            block_modifiers_of(&actual, pasted)
        );
    }

    #[test]
    fn full_document_paragraph_replaces_empty_destination_with_source_context() {
        let (source, ..) = state! {
            doc { root {
                source_paragraph: paragraph [alignment(Alignment::Right)] { text("hello") }
            } }
            selection: (source_paragraph, 0) -> (source_paragraph, 5, <)
        };
        let slice = Slice::extract(&source).expect("full-document slice");

        let (initial, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Left)] {}
            } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                pasted: paragraph [alignment(Alignment::Right)] { text("hello") }
            } }
            selection: (pasted, 5)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn structural_slice_at_paragraph_start_preserves_only_unjoined_source_context() {
        let (source, ..) = state! {
            doc { root {
                first: paragraph [alignment(Alignment::Right)] { text("A") }
                last: paragraph [alignment(Alignment::Left)] { text("B") }
            } }
            selection: (first, 0) -> (last, 1, <)
        };
        let slice = Slice::extract(&source).expect("full-document slice");

        let (initial, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Center)] { text("x") }
            } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                paragraph [alignment(Alignment::Right)] { text("A") }
                target: paragraph [alignment(Alignment::Center)] { text("Bx") }
            } }
            selection: (target, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn structural_slice_at_paragraph_end_preserves_only_unjoined_source_context() {
        let (source, ..) = state! {
            doc { root {
                first: paragraph [alignment(Alignment::Right)] { text("A") }
                last: paragraph [alignment(Alignment::Left)] { text("B") }
            } }
            selection: (first, 0) -> (last, 1, <)
        };
        let slice = Slice::extract(&source).expect("full-document slice");

        let (initial, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Center)] { text("x") }
            } }
            selection: (target, 1)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                paragraph [alignment(Alignment::Center)] { text("xA") }
                target: paragraph [alignment(Alignment::Left)] { text("B") }
            } }
            selection: (target, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn structural_slice_replaces_empty_paragraph_with_all_source_contexts() {
        let (source, ..) = state! {
            doc { root {
                first: paragraph [alignment(Alignment::Right)] { text("A") }
                last: paragraph [alignment(Alignment::Center)] { text("B") }
            } }
            selection: (first, 0) -> (last, 1, <)
        };
        let slice = Slice::extract(&source).expect("full-document slice");

        let (initial, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Left)] {}
            } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                paragraph [alignment(Alignment::Right)] { text("A") }
                target: paragraph [alignment(Alignment::Center)] { text("B") }
            } }
            selection: (target, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn full_document_paragraph_replaces_empty_list_item_paragraph() {
        let (source, ..) = state! {
            doc { root {
                source_paragraph: paragraph [alignment(Alignment::Right)] { text("A") }
            } }
            selection: (source_paragraph, 0) -> (source_paragraph, 1, <)
        };
        let slice = Slice::extract(&source).expect("full-document slice");

        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item {
                    target: paragraph [alignment(Alignment::Left)] {}
                } }
                paragraph {}
            } }
            selection: (target, 0)
        };
        let (actual, steps, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                bullet_list { list_item {
                    target: paragraph [alignment(Alignment::Right)] { text("A") }
                } }
                paragraph {}
            } }
            selection: (target, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(split_step_count(&steps), 0);
    }

    #[test]
    fn bare_inline_slice_keeps_empty_destination_context() {
        let (source, ..) = state! {
            doc { root {
                source_paragraph: paragraph [alignment(Alignment::Right)] { text("A") }
                paragraph { text("after") }
            } }
            selection: (source_paragraph, 0) -> (source_paragraph, 1, <)
        };
        let slice = Slice::extract(&source).expect("inline slice");
        assert!(
            slice
                .content
                .iter()
                .all(|fragment| fragment.node.as_type().spec().inline)
        );

        let (initial, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Left)] {}
            } }
            selection: (target, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));

        let (expected, ..) = state! {
            doc { root {
                target: paragraph [alignment(Alignment::Left)] { text("A") }
            } }
            selection: (target, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn round_trip_aligned_unit_image_via_payload_preserves_alignment() {
        let (source, ..) = state! {
            doc { r: root { img: image [alignment(Alignment::Center)] paragraph {} } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        let payload = original.to_payload(&Resource::new_test());
        let (parsed, source) =
            Slice::from_payload(Some(&payload.html), &payload.text, &Resource::new_test());
        assert_eq!(source, editor_clipboard::PayloadSource::Html);
        assert!(
            matches!(parsed.content[0].node, PlainNode::Image(_)),
            "sanity: payload carries the image"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            parsed,
            SliceProvenance::Formatted
        ));

        let img_dot = {
            let view = actual.view();
            view.root()
                .expect("root exists")
                .children()
                .find_map(|c| match c {
                    ChildView::Leaf(l) if l.node_type() == NodeType::Image => Some(l.dot()),
                    _ => None,
                })
                .expect("pasted image present")
        };
        assert!(
            block_modifiers_of(&actual, img_dot)
                .iter()
                .any(|m| matches!(
                    m,
                    Modifier::Alignment {
                        value: Alignment::Center
                    }
                )),
            "the pasted unit image keeps its alignment (block format), got {:?}",
            block_modifiers_of(&actual, img_dot)
        );
    }

    #[test]
    fn paste_open_fragment_leaves_target_carry_untouched() {
        let (src, ..) = state! {
            doc { root { sp: paragraph { text("XY") } } }
            selection: (sp, 0) -> (sp, 2)
        };
        let open = Slice::extract(&src).expect("non-collapsed");
        assert!(
            open.content
                .iter()
                .all(|fragment| fragment.carry.is_empty()),
            "sanity: bare inline fragments carry no block carry"
        );

        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open,
            SliceProvenance::Formatted
        ));
        assert_eq!(
            carry_of(&actual, p1),
            vec![Modifier::Italic],
            "pasting a carry-less open fragment must not disturb the target's carry"
        );
    }

    #[test]
    fn paste_open_bold_fragment_into_italic_para_keeps_paint_and_target_block_format() {
        let (src, ..) = state! {
            doc { root { sp: paragraph [alignment(Alignment::Right)] { text("XY") [bold] } } }
            selection: (sp, 0) -> (sp, 2)
        };
        let open = Slice::extract(&src).expect("non-collapsed");

        let (initial, ..) = state! {
            doc { root { p1: paragraph [alignment(Alignment::Center)] { text("ab") [italic] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph [alignment(Alignment::Center)] {
                    text("a") [italic]
                    text("XY") [bold]
                    text("b") [italic]
                }
            } }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    fn inline_all_have(view: &editor_model::DocView, block: Dot, modifier: &Modifier) -> bool {
        let Some(node) = view.node(block) else {
            return false;
        };
        let mut count = 0;
        for (i, c) in node.children().enumerate() {
            if matches!(c, ChildView::Leaf(_)) {
                count += 1;
                if !node.leaf_own_modifiers_at(i).iter().any(|m| m == modifier) {
                    return false;
                }
            }
        }
        count > 0
    }

    #[test]
    fn plain_paste_two_lines_paints_all_and_carries_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\nb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(paras.len(), 2);
        assert_eq!(view.node(paras[0]).unwrap().inline_text(), "가a");
        assert_eq!(view.node(paras[1]).unwrap().inline_text(), "b");
        assert!(inline_all_have(&view, paras[0], &Modifier::Bold));
        assert!(inline_all_have(&view, paras[1], &Modifier::Bold));
        assert!(
            carry_of(&actual, paras[1])
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the new paragraph records bold carry, got {:?}",
            carry_of(&actual, paras[1])
        );
        assert!(
            carry_of(&actual, paras[0]).is_empty(),
            "the original left paragraph keeps its untouched carry"
        );
    }

    #[test]
    fn plain_paste_blank_line_carries_empty_middle_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\n\nb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(paras.len(), 3);
        assert_eq!(view.node(paras[0]).unwrap().inline_text(), "가a");
        assert_eq!(view.node(paras[1]).unwrap().inline_text(), "");
        assert_eq!(view.node(paras[2]).unwrap().inline_text(), "b");
        assert!(
            carry_of(&actual, paras[1])
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the empty middle paragraph records bold carry, got {:?}",
            carry_of(&actual, paras[1])
        );
        assert!(
            carry_of(&actual, paras[2])
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(inline_all_have(&view, paras[2], &Modifier::Bold));
    }

    #[test]
    fn plain_paste_with_tab_paints_tab_uniformly() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(
            paras.len(),
            1,
            "single-line tab paste creates no new paragraph"
        );
        let p = view.node(paras[0]).unwrap();
        assert!(
            p.children()
                .any(|c| matches!(c, ChildView::Leaf(l) if l.node_type() == NodeType::Tab)),
            "the pasted tab is a Tab atom"
        );
        assert!(
            inline_all_have(&view, paras[0], &Modifier::Bold),
            "every inline leaf including the Tab is painted bold"
        );
    }

    #[test]
    fn plain_paste_at_block_boundary_uses_document_default() {
        let (initial, r) = state! {
            doc { r: root {
                paragraph { text("a") [bold] }
                paragraph { text("b") [bold] }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice::from_text("x\ny");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        let texts: Vec<String> = paras
            .iter()
            .map(|id| view.node(*id).unwrap().inline_text())
            .collect();
        assert_eq!(texts, vec!["a", "x", "y", "b"]);
        assert!(
            view.node(paras[1])
                .unwrap()
                .leaf_own_modifiers_at(0)
                .is_empty()
        );
        assert!(
            view.node(paras[2])
                .unwrap()
                .leaf_own_modifiers_at(0)
                .is_empty()
        );
        assert!(carry_of(&actual, paras[1]).is_empty());
        assert!(carry_of(&actual, paras[2]).is_empty());
        let _ = r;
    }

    #[test]
    fn plain_paste_in_link_middle_copies_link() {
        let href = "https://e.com".to_string();
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("ab") [link(href: href.clone())] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "aXb");
        assert!(
            p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, Modifier::Link { .. })),
            "plain paste in the middle of a link copies the link onto the pasted char, got {:?}",
            p.leaf_own_modifiers_at(1)
        );
    }

    #[test]
    fn plain_paste_consumes_pending_once() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let slice = Slice::from_text("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        assert!(
            actual.pending_modifiers.is_empty(),
            "plain paste consumes the pending format once"
        );
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "hXi");
        assert!(
            p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the pasted char inherits the consumed pending bold"
        );
        assert!(
            !p.leaf_own_modifiers_at(0)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            !p.leaf_own_modifiers_at(2)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
    }

    #[test]
    fn formatted_slice_unpainted_run_ignores_pending_and_continuation() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
            pending_modifiers: [italic]
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        assert!(
            !actual.pending_modifiers.is_empty(),
            "a formatted paste never consumes the pending format"
        );
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "가XY");
        assert!(
            p.leaf_own_modifiers_at(0)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        for slot in [1usize, 2] {
            assert!(
                p.leaf_own_modifiers_at(slot).is_empty(),
                "an unpainted formatted run must not inherit caret pending/continuation, slot {slot}: {:?}",
                p.leaf_own_modifiers_at(slot)
            );
        }
    }
}

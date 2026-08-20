//! Pure planning for ordinary linear Slice insertion and replacement.

use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{ChildView, DocView, Fragment, content_placement};
use editor_state::{Position, Selection};

use super::{
    PlacementOutcome, SliceInsertionKind, SliceInsertionPlan, place_slice_at_frontier,
    place_slice_at_position,
};
use crate::CommandError;
use crate::helpers::{
    ExistingBlock, LinearDeletionPlan, LinearDeletionSemantics, PlannedEndpoint, PlannedSelection,
    SliceInsertionTarget, SliceInsertionTargetShape, block_boundary_fragments,
    find_lowest_common_ancestor, path_from_ancestor, plan_joined_replacement_frontier,
    plan_linear_deletion, plan_unjoined_replacement_frontiers,
};

pub(super) enum LinearFitOutcome {
    Plan(LinearFitPlan),
    NoOp,
    NoFit,
}

pub(crate) struct LinearFitPlan {
    pub(crate) selection: PlannedSelection,
    pub(crate) mutation: LinearMutation,
    pub(crate) final_selection: LinearFinalSelection,
}

pub(crate) enum LinearFinalSelection {
    InsertedContent,
    DeletionBoundary,
}

pub(crate) enum LinearMutation {
    PointInsertion {
        insertion: SliceInsertionPlan,
    },
    RangeReplacement {
        deletion: LinearDeletionSemantics,
        placement: RangePlacement,
    },
}

pub(crate) enum RangePlacement {
    Joined(JoinedReplacementPlan),
    PreservedBoundary(PlannedBoundaryInsertion),
    SeparatedBranches {
        boundary: PlannedBranchInsertion,
    },
    SeparatedOpenEdges {
        left: PlannedBoundaryInsertion,
        middle: PlannedBranchInsertion,
        right: PlannedBoundaryInsertion,
    },
    DeletionOnly {
        join: Option<PlannedJoin>,
    },
}

pub(crate) struct JoinedReplacementPlan {
    /// The authored or materializable left survivor that owns the replacement
    /// seam. The executor binds this endpoint before mutation and never
    /// reconstructs an insertion frontier from the mutated document.
    pub(crate) destination: PlannedEndpoint,
    /// Complete right-boundary reattachment and container-survivor decisions
    /// made from the original selection.
    pub(crate) join: Option<PlannedJoin>,
    /// Already-fitted Slice output, including its semantic output identities.
    pub(crate) insertion: SliceInsertionPlan,
}

pub(crate) struct PlannedBoundaryInsertion {
    pub(crate) destination: PlannedEndpoint,
    pub(crate) target: SliceInsertionTargetShape,
    pub(crate) insertion: SliceInsertionPlan,
}

pub(crate) struct PlannedBranchInsertion {
    pub(crate) parent: PlannedEndpoint,
    pub(crate) splits: Vec<PlannedBranchSplit>,
    pub(crate) right_boundary: PlannedBranchNode,
    pub(crate) insertion: SliceInsertionPlan,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PlannedOutputKey(u32);

#[derive(Clone)]
pub(crate) enum PlannedBranchNode {
    Existing(PlannedEndpoint),
    Output(PlannedOutputKey),
}

pub(crate) struct PlannedBranchSplit {
    pub(crate) wrapper: PlannedEndpoint,
    pub(crate) first_right: PlannedBranchNode,
    pub(crate) output: PlannedOutputKey,
}

pub(crate) struct PlannedJoin {
    pub(crate) from_textblock: PlannedEndpoint,
    pub(crate) to_textblock: PlannedEndpoint,
    pub(crate) prune: Vec<PlannedEndpoint>,
    pub(crate) container_merges: Vec<PlannedContainerMerge>,
    pub(crate) affected: Vec<PlannedEndpoint>,
}

pub(crate) struct PlannedContainerMerge {
    pub(crate) target: PlannedEndpoint,
    pub(crate) source: PlannedEndpoint,
}

pub(super) fn fit_linear_slice(
    view: &DocView,
    selection: Selection,
    slice: Slice,
) -> Result<LinearFitOutcome, CommandError> {
    let planned_selection = PlannedSelection::capture(view, selection).ok_or_else(|| {
        CommandError::Corrupted("cannot capture Slice replacement endpoints".into())
    })?;
    if selection.is_collapsed() {
        let target = SliceInsertionTarget::from_view(view, selection.head).ok_or_else(|| {
            CommandError::Corrupted("cannot capture Slice insertion target".into())
        })?;
        return Ok(match place_slice_at_frontier(&target, slice) {
            PlacementOutcome::Placed(insertion) => LinearFitOutcome::Plan(LinearFitPlan {
                selection: planned_selection,
                mutation: LinearMutation::PointInsertion { insertion },
                final_selection: LinearFinalSelection::InsertedContent,
            }),
            PlacementOutcome::CompleteNoOutput => LinearFitOutcome::NoOp,
            PlacementOutcome::NoFit => LinearFitOutcome::NoFit,
        });
    }

    let Some(deletion) = plan_linear_deletion(view, selection)? else {
        return Ok(LinearFitOutcome::NoFit);
    };
    let fit = fit_range_replacement(view, selection, &deletion, &slice)?;
    Ok(match fit {
        RangeFit::Placed(placement) => LinearFitOutcome::Plan(LinearFitPlan {
            selection: planned_selection,
            mutation: LinearMutation::RangeReplacement {
                deletion: deletion.semantics(),
                placement,
            },
            final_selection: LinearFinalSelection::InsertedContent,
        }),
        RangeFit::DeletionOnly(join) => LinearFitOutcome::Plan(LinearFitPlan {
            selection: planned_selection,
            mutation: LinearMutation::RangeReplacement {
                deletion: deletion.semantics(),
                placement: RangePlacement::DeletionOnly { join },
            },
            final_selection: LinearFinalSelection::DeletionBoundary,
        }),
        RangeFit::NoFit => LinearFitOutcome::NoFit,
    })
}

enum RangeFit {
    Placed(RangePlacement),
    DeletionOnly(Option<PlannedJoin>),
    NoFit,
}

fn fit_range_replacement(
    view: &DocView,
    selection: Selection,
    deletion: &LinearDeletionPlan,
    slice: &Slice,
) -> Result<RangeFit, CommandError> {
    OriginalRangeFrontiers::capture(view, selection, deletion)?.fit(slice)
}

struct OriginalRangeFrontiers<'a, 'doc> {
    view: &'a DocView<'doc>,
    selection: Selection,
    deletion: &'a LinearDeletionPlan,
    separated_lca: Option<Dot>,
    unjoined: Option<crate::helpers::UnjoinedReplacementFrontiers>,
}

impl<'a, 'doc> OriginalRangeFrontiers<'a, 'doc> {
    fn capture(
        view: &'a DocView<'doc>,
        selection: Selection,
        deletion: &'a LinearDeletionPlan,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            view,
            selection,
            deletion,
            separated_lca: separated_branch_lca(view, selection),
            unjoined: plan_unjoined_replacement_frontiers(view, deletion)?,
        })
    }

    fn fit(&self, slice: &Slice) -> Result<RangeFit, CommandError> {
        if self.separated_lca.is_none() {
            return Ok(self.fit_joined(slice)?.unwrap_or(RangeFit::NoFit));
        }
        match slice_boundary_contribution(slice) {
            SliceBoundaryContribution::Both => {
                if let Some(placement) = self.fit_separated_open_edges(slice)? {
                    return Ok(RangeFit::Placed(placement));
                }
                if let Some(placed) = self.fit_joined(slice)? {
                    return Ok(placed);
                }
                if let Some(placement) = self.fit_unjoined_boundary(false, slice) {
                    return Ok(RangeFit::Placed(placement));
                }
                if let Some(placement) = self.fit_unjoined_boundary(true, slice) {
                    return Ok(RangeFit::Placed(placement));
                }
            }
            SliceBoundaryContribution::Start => {
                if let Some(placement) = self.fit_unjoined_boundary(true, slice) {
                    return Ok(RangeFit::Placed(placement));
                }
            }
            SliceBoundaryContribution::End => {
                if let Some(placement) = self.fit_unjoined_boundary(false, slice) {
                    return Ok(RangeFit::Placed(placement));
                }
            }
            SliceBoundaryContribution::Neither => {}
        }
        Ok(self
            .fit_separated_branches(slice)?
            .map(|boundary| RangeFit::Placed(RangePlacement::SeparatedBranches { boundary }))
            .unwrap_or(RangeFit::NoFit))
    }

    fn fit_joined(&self, slice: &Slice) -> Result<Option<RangeFit>, CommandError> {
        // The virtual tree is a pure planning workspace for the original
        // selection's survivors. It is not an independently executable delete
        // plan: the accepted result below owns the survivor endpoint, join,
        // fitted Slice output, and final output selection before any mutation.
        let Some(preview) = plan_joined_replacement_frontier(self.view, self.deletion)? else {
            return Ok(None);
        };
        let join = preview
            .join
            .map(|join| capture_planned_join(self.view, join))
            .transpose()?;
        let target = preview.target;
        Ok(match place_slice_at_frontier(&target, slice.clone()) {
            PlacementOutcome::Placed(insertion) => Some(RangeFit::Placed(RangePlacement::Joined(
                JoinedReplacementPlan {
                    destination: PlannedEndpoint::capture(self.view, target.position())
                        .ok_or_else(|| {
                            CommandError::Corrupted(
                                "cannot capture joined Slice replacement destination".into(),
                            )
                        })?,
                    join,
                    insertion,
                },
            ))),
            PlacementOutcome::CompleteNoOutput => Some(RangeFit::DeletionOnly(join)),
            PlacementOutcome::NoFit => None,
        })
    }

    fn fit_separated_branches(
        &self,
        slice: &Slice,
    ) -> Result<Option<PlannedBranchInsertion>, CommandError> {
        let Some(lca) = self.separated_lca else {
            return Ok(None);
        };
        let Some((parent, splits, right_boundary, blocks, list_merges)) =
            fit_at_preserved_branch_boundary(self.view, self.selection, lca, slice)?
        else {
            return Ok(None);
        };
        let output =
            crate::helpers::planned_block_output(&blocks, &list_merges).ok_or_else(|| {
                CommandError::Corrupted(
                    "fitted separated Slice boundary produced no planned output".into(),
                )
            })?;
        Ok(Some(PlannedBranchInsertion {
            parent,
            splits,
            right_boundary,
            insertion: SliceInsertionPlan {
                kind: SliceInsertionKind::BlockBoundary {
                    blocks,
                    list_merges,
                },
                output,
            },
        }))
    }

    fn fit_separated_open_edges(
        &self,
        slice: &Slice,
    ) -> Result<Option<RangePlacement>, CommandError> {
        if slice.open_start == 0 || slice.open_end == 0 || slice.content.len() < 3 {
            return Ok(None);
        }
        let left_slice = Slice::new(vec![slice.content[0].clone()], slice.open_start, 0);
        let middle_slice = Slice::new(slice.content[1..slice.content.len() - 1].to_vec(), 0, 0);
        let right_slice = Slice::new(
            vec![slice.content[slice.content.len() - 1].clone()],
            0,
            slice.open_end,
        );
        let Some(left) = self.fit_unjoined_insertion(true, &left_slice) else {
            return Ok(None);
        };
        let Some(middle) = self.fit_separated_branches(&middle_slice)? else {
            return Ok(None);
        };
        let Some(right) = self.fit_unjoined_insertion(false, &right_slice) else {
            return Ok(None);
        };
        Ok(Some(RangePlacement::SeparatedOpenEdges {
            left,
            middle,
            right,
        }))
    }

    fn fit_unjoined_boundary(&self, use_left: bool, slice: &Slice) -> Option<RangePlacement> {
        self.fit_unjoined_insertion(use_left, slice)
            .map(RangePlacement::PreservedBoundary)
    }

    fn fit_unjoined_insertion(
        &self,
        use_left: bool,
        slice: &Slice,
    ) -> Option<PlannedBoundaryInsertion> {
        let unjoined = self.unjoined.as_ref()?;
        let resolved = self.selection.resolve(self.view)?;
        let target = if use_left {
            &unjoined.left
        } else {
            &unjoined.right
        };
        let position = if use_left {
            resolved.from().position()
        } else {
            Position {
                node: resolved.to().position().node,
                offset: 0,
                affinity: resolved.to().position().affinity,
            }
        };
        let PlacementOutcome::Placed(insertion) = place_slice_at_frontier(&target, slice.clone())
        else {
            return None;
        };
        Some(PlannedBoundaryInsertion {
            destination: PlannedEndpoint::capture(self.view, position)?,
            target: SliceInsertionTargetShape::capture(&target),
            insertion,
        })
    }
}

enum SliceBoundaryContribution {
    Both,
    Start,
    End,
    Neither,
}

/// Which original destination edges the Slice can contribute into. Bare inline
/// content lives directly in a textblock and therefore contributes at both
/// edges without an open wrapper. Structural roots contribute only through
/// the edges advertised by Slice openness.
fn slice_boundary_contribution(slice: &Slice) -> SliceBoundaryContribution {
    if slice
        .content
        .iter()
        .all(|fragment| editor_model::Schema::node_spec(fragment.node.as_type()).inline)
    {
        return SliceBoundaryContribution::Both;
    }
    match (slice.open_start > 0, slice.open_end > 0) {
        (true, true) => SliceBoundaryContribution::Both,
        (true, false) => SliceBoundaryContribution::Start,
        (false, true) => SliceBoundaryContribution::End,
        (false, false) => SliceBoundaryContribution::Neither,
    }
}

fn capture_planned_join(
    view: &DocView,
    topology: crate::helpers::LinearJoinExecution,
) -> Result<PlannedJoin, CommandError> {
    if topology.trailing_page_break.is_some() {
        return Err(CommandError::Corrupted(
            "Slice replacement join cannot consume a surviving PageBreak".into(),
        ));
    }
    let capture = |node| {
        PlannedEndpoint::capture(view, Position::new(node, 0))
            .ok_or_else(|| CommandError::Corrupted("cannot capture planned Slice join node".into()))
    };
    let container_merges = topology
        .container_merges
        .into_iter()
        .map(|(target, source)| {
            Ok(PlannedContainerMerge {
                target: capture(target)?,
                source: capture(source)?,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    let affected = topology
        .affected
        .into_iter()
        .map(capture)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlannedJoin {
        from_textblock: capture(topology.from_textblock)?,
        to_textblock: capture(topology.to_textblock)?,
        prune: topology
            .prune
            .into_iter()
            .map(capture)
            .collect::<Result<Vec<_>, _>>()?,
        container_merges,
        affected,
    })
}

/// Lowest common branch frontier retained from the original range. The Slice
/// is deliberately not consulted here: source placement chooses among the
/// joined and separated candidates after both destination boundaries exist.
fn separated_branch_lca(view: &DocView, selection: Selection) -> Option<Dot> {
    let Some(resolved) = selection.resolve(view) else {
        return None;
    };
    let from = resolved.from().position();
    let to = resolved.to().position();
    if from.node == to.node {
        return None;
    }

    let Some(lca) = find_lowest_common_ancestor(view, from.node, to.node) else {
        return None;
    };
    let Some(from_path) = path_from_ancestor(view, from.node, lca) else {
        return None;
    };
    let Some(to_path) = path_from_ancestor(view, to.node, lca) else {
        return None;
    };
    let Some((&from_branch, &to_branch)) = from_path.first().zip(to_path.first()) else {
        return None;
    };
    (from_branch < to_branch).then_some(lca)
}

/// Fit a closed structural source at the first schema-valid frontier that
/// retains both original destination branches.
fn fit_at_preserved_branch_boundary(
    view: &DocView,
    selection: Selection,
    lca: Dot,
    slice: &Slice,
) -> Result<
    Option<(
        PlannedEndpoint,
        Vec<PlannedBranchSplit>,
        PlannedBranchNode,
        Vec<Fragment>,
        Vec<crate::helpers::PlannedListMerge>,
    )>,
    CommandError,
> {
    let (parent, blocks) = {
        let mut candidate = lca;
        loop {
            let Some(node) = view.node(candidate) else {
                return Ok(None);
            };
            if block_boundary_fragments(slice, node.node_type()).is_some()
                && can_split_from_lca_to(view, lca, candidate)
                && let PlacementOutcome::Placed(SliceInsertionPlan {
                    kind: SliceInsertionKind::BlockBoundary { blocks, .. },
                    ..
                }) = place_slice_at_position(view, Position::new(candidate, 0), slice.clone())
            {
                break (candidate, blocks);
            }
            if node.spec().isolating {
                return Ok(None);
            }
            let Some(parent) = node.parent() else {
                return Ok(None);
            };
            candidate = parent.id();
        }
    };
    let resolved = selection
        .resolve(view)
        .ok_or_else(|| CommandError::Corrupted("cannot resolve Slice replacement range".into()))?;
    let from = resolved.from().node();
    let to = resolved.to().node();
    let left = dot_path_from_ancestor(view, parent, from).ok_or_else(|| {
        CommandError::Corrupted("left Slice boundary escaped its fitted parent".into())
    })?;
    let right = dot_path_from_ancestor(view, parent, to).ok_or_else(|| {
        CommandError::Corrupted("right Slice boundary escaped its fitted parent".into())
    })?;
    let common = left
        .iter()
        .zip(&right)
        .take_while(|(left, right)| left == right)
        .count();
    if common == left.len() || common == right.len() {
        return Ok(None);
    }
    let left_block = view.node(left[0]).map(|node| ExistingBlock {
        id: Some(node.id()),
        node_type: node.node_type(),
    });
    let right_block = view.node(right[0]).map(|node| ExistingBlock {
        id: Some(node.id()),
        node_type: node.node_type(),
    });
    let Some(list_merges) =
        crate::helpers::plan_adjacent_list_merges(left_block, &blocks, right_block)
    else {
        return Ok(None);
    };

    let capture = |node| {
        PlannedEndpoint::capture(view, Position::new(node, 0)).ok_or_else(|| {
            CommandError::Corrupted("cannot capture planned Slice boundary node".into())
        })
    };
    let mut right_boundary = PlannedBranchNode::Existing(capture(right[common])?);
    let mut splits = Vec::with_capacity(common);
    for (index, wrapper) in left[..common].iter().rev().enumerate() {
        let output = PlannedOutputKey(index as u32);
        splits.push(PlannedBranchSplit {
            wrapper: capture(*wrapper)?,
            first_right: right_boundary,
            output,
        });
        right_boundary = PlannedBranchNode::Output(output);
    }
    let parent = capture(parent)?;
    Ok(Some((parent, splits, right_boundary, blocks, list_merges)))
}

fn dot_path_from_ancestor(view: &DocView, ancestor: Dot, node: Dot) -> Option<Vec<Dot>> {
    let mut path = Vec::new();
    let mut current = node;
    while current != ancestor {
        path.push(current);
        current = view.node(current)?.parent()?.id();
    }
    path.reverse();
    Some(path)
}

fn can_split_from_lca_to(view: &DocView, lca: Dot, candidate: Dot) -> bool {
    let mut split = lca;
    while split != candidate {
        let Some(split_node) = view.node(split) else {
            return false;
        };
        if split_node.spec().isolating || !can_duplicate_under_parent(&split_node) {
            return false;
        }
        let Some(parent) = split_node.parent() else {
            return false;
        };
        split = parent.id();
    }
    true
}

fn can_duplicate_under_parent(node: &editor_model::NodeView<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let Some(index) = node.index() else {
        return false;
    };
    let mut child_types: Vec<_> = parent
        .children()
        .map(|child| match child {
            ChildView::Block(block) => block.node_type(),
            ChildView::Leaf(leaf) => leaf.node_type(),
        })
        .collect();
    child_types.insert(index + 1, node.node_type());
    content_placement(parent.node_type(), &child_types).is_valid()
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{
        Fragment, PlainNode, PlainPageBreakNode, PlainParagraphNode, PlainTextNode,
    };

    use super::super::{FitOutcome, SliceFitPlan, SliceFitPlanKind, fit_slice};
    use super::*;

    #[test]
    fn empty_slice_is_no_op() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        assert!(matches!(
            fit_slice(
                &state,
                Selection::collapsed(Position::new(p, 0)),
                Slice::new(vec![], 0, 0),
            )
            .unwrap(),
            FitOutcome::NoOp
        ));
    }

    #[test]
    fn page_break_in_non_isolating_nested_paragraph_has_a_plan() {
        let (state, p) = state! {
            doc { root {
                blockquote { p: paragraph { text("target") } }
                paragraph {}
            } }
            selection: (p, 3)
        };
        let slice = Slice::new(
            vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment::leaf(PlainNode::Text(PlainTextNode { text: "lo".into() })),
                    Fragment::leaf(PlainNode::PageBreak(PlainPageBreakNode::default())),
                ],
            }],
            0,
            0,
        );

        assert!(matches!(
            fit_slice(&state, Selection::collapsed(Position::new(p, 3)), slice,).unwrap(),
            FitOutcome::Plan(_)
        ));
    }

    #[test]
    fn task3_programmatic_slice_with_invalid_open_edge_is_no_fit() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("target") } } }
            selection: (p, 3)
        };
        let slice = Slice {
            content: vec![Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "x".into(),
                }))],
            }],
            open_start: u32::MAX,
            open_end: 0,
        };

        assert!(matches!(
            fit_slice(&state, Selection::collapsed(Position::new(p, 3)), slice).unwrap(),
            FitOutcome::NoFit
        ));
    }

    #[test]
    fn non_empty_but_lossless_empty_inline_slice_is_no_op() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("target") } } }
            selection: (p, 3)
        };
        let slice = Slice::new(
            vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: String::new(),
            }))],
            0,
            0,
        );

        assert!(matches!(
            fit_slice(&state, Selection::collapsed(Position::new(p, 3)), slice).unwrap(),
            FitOutcome::NoOp
        ));
    }

    #[test]
    fn non_empty_lossless_empty_slice_replaces_a_range_with_deletion() {
        let (state, _p) = state! {
            doc { root { p: paragraph { text("abc") } } }
            selection: (p, 1) -> (p, 2)
        };
        let slice = Slice::new(
            vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: String::new(),
            }))],
            0,
            0,
        );

        assert!(matches!(
            fit_slice(&state, state.selection.unwrap(), slice).unwrap(),
            FitOutcome::Plan(SliceFitPlan {
                kind: SliceFitPlanKind::Linear(LinearFitPlan {
                    mutation: LinearMutation::RangeReplacement {
                        placement: RangePlacement::DeletionOnly { .. },
                        ..
                    },
                    ..
                }),
            })
        ));
    }

    #[test]
    fn structurally_valid_programmatic_slice_within_the_structure_budget_has_a_plan() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        let mut nested = Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default()))
            .with_children(vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: "deep".into(),
            }))]);
        for _ in 0..14 {
            nested = Fragment::leaf(PlainNode::BulletList(
                editor_model::PlainBulletListNode::default(),
            ))
            .with_children(vec![
                Fragment::leaf(PlainNode::ListItem(
                    editor_model::PlainListItemNode::default(),
                ))
                .with_children(vec![
                    Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default())),
                    nested,
                ]),
            ]);
        }
        let slice = Slice::new(vec![nested], 0, 0);

        assert!(matches!(
            fit_slice(&state, Selection::collapsed(Position::new(p, 0)), slice).unwrap(),
            FitOutcome::Plan(_)
        ));
    }

    #[test]
    fn programmatic_slice_beyond_the_structure_budget_is_no_fit() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        let mut inner = Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
        for _ in 0..16 {
            inner = Fragment::leaf(editor_model::PlainNode::BulletList(
                editor_model::PlainBulletListNode::default(),
            ))
            .with_children(vec![
                Fragment::leaf(editor_model::PlainNode::ListItem(
                    editor_model::PlainListItemNode::default(),
                ))
                .with_children(vec![inner]),
            ]);
        }
        let slice = Slice::new(vec![inner], 0, 0);

        let outcome = fit_slice(&state, Selection::collapsed(Position::new(p, 0)), slice).unwrap();
        assert!(matches!(outcome, FitOutcome::NoFit));
    }
}

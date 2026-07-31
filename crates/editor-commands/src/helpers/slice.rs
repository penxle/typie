use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Fragment, Modifier, NodeType, PlainNode, PlainParagraphNode, Schema,
    Subtree, content_placement,
};
use editor_state::{Affinity, Position, Selection, StableSelection, first_cursor_position};
use editor_transaction::{Transaction, fulfill, minimal_subtree};

use super::{
    apply_inline_modifiers, child_node_type, consume_pending_modifiers, insert_hard_break_at_caret,
    insert_tab_at_caret, insert_text_at_caret, is_list_type, merge_adjacent_list_pair,
    next_sibling, resolve_effective_modifiers, resolve_selection_after_adjacent_list_merge,
    restore_selection_after_adjacent_list_merge,
};
use crate::types::SliceProvenance;
use crate::{CommandError, CommandResult};

mod page_break;

use page_break::{insert_terminal_page_break_from_edge, paragraph_ends_with_page_break};

pub(crate) enum InlineMode {
    Formatted,
    Plain(Vec<Modifier>),
}

impl InlineMode {
    fn paint_for<'a>(&'a self, fragment: &'a Fragment) -> &'a [Modifier] {
        match self {
            InlineMode::Formatted => &fragment.modifiers,
            InlineMode::Plain(paint) => paint,
        }
    }

    fn plain_paint(&self) -> Option<&[Modifier]> {
        match self {
            InlineMode::Plain(paint) => Some(paint),
            InlineMode::Formatted => None,
        }
    }
}

fn carry_from_paint(paint: &[Modifier]) -> Vec<Modifier> {
    paint
        .iter()
        .filter(|m| m.as_type().is_carry_kind())
        .cloned()
        .collect()
}

pub(crate) fn subtree_to_fragment(subtree: Subtree) -> Fragment {
    let (node, modifiers, carry, children, _) = subtree.into_parts();
    Fragment {
        node,
        modifiers,
        carry,
        children: children.into_iter().map(subtree_to_fragment).collect(),
    }
}

pub(crate) fn build_inline_mode(
    tr: &mut Transaction,
    position: &Position,
    provenance: SliceProvenance,
) -> Result<InlineMode, CommandError> {
    if !provenance.is_plain() {
        return Ok(InlineMode::Formatted);
    }
    let pending = tr.pending_modifiers().clone();
    let paint = resolve_effective_modifiers(
        &tr.state().projected,
        position.node,
        position.offset,
        &pending,
    );
    consume_pending_modifiers(tr)?;
    Ok(InlineMode::Plain(paint))
}

pub(crate) fn paint_block_uniformly(
    tr: &mut Transaction,
    block: Dot,
    paint: &[Modifier],
) -> Result<(), CommandError> {
    let is_textblock = {
        let view = tr.state().view();
        view.node(block)
            .is_some_and(|node| node.spec().is_textblock())
    };
    if !is_textblock {
        return Ok(());
    }
    let dots: Vec<Dot> = {
        let view = tr.state().view();
        match view.node(block) {
            Some(node) => node
                .children()
                .filter_map(|c| match c {
                    ChildView::Leaf(l) => Some(l.dot()),
                    ChildView::Block(_) => None,
                })
                .collect(),
            None => Vec::new(),
        }
    };
    apply_inline_modifiers(tr, &dots, paint)?;
    tr.replace_carry(block, carry_from_paint(paint))?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct SliceInsertionTarget {
    position: Position,
    path: Vec<SliceInsertionTargetNode>,
}

#[derive(Clone)]
pub(crate) struct SliceInsertionTargetShape {
    offset: usize,
    path: Vec<SliceInsertionTargetNodeShape>,
}

#[derive(Clone)]
struct SliceInsertionTargetNodeShape {
    node_type: NodeType,
    child_types: Vec<NodeType>,
    index: Option<usize>,
}

#[derive(Clone)]
pub(crate) struct SliceInsertionTargetNode {
    pub id: Dot,
    pub node_type: NodeType,
    pub child_types: Vec<NodeType>,
    pub child_ids: Vec<Option<Dot>>,
    pub index: Option<usize>,
}

impl SliceInsertionTarget {
    pub(crate) fn from_view(view: &DocView, position: Position) -> Option<Self> {
        position.resolve(view)?;
        let target = view.node(position.node)?;
        let mut nodes: Vec<_> = target.ancestors().collect();
        nodes.reverse();
        let path = nodes
            .into_iter()
            .map(|node| SliceInsertionTargetNode {
                id: node.id(),
                node_type: node.node_type(),
                child_types: node
                    .children()
                    .map(|child| child_node_type(&child))
                    .collect(),
                child_ids: node
                    .children()
                    .map(|child| {
                        Some(match child {
                            ChildView::Block(block) => block.id(),
                            ChildView::Leaf(leaf) => leaf.dot(),
                        })
                    })
                    .collect(),
                index: node.index(),
            })
            .collect();
        Some(Self { position, path })
    }

    pub(crate) fn from_path(
        position: Position,
        path: Vec<SliceInsertionTargetNode>,
    ) -> Option<Self> {
        let target = path.last()?;
        if target.id != position.node || position.offset > target.child_types.len() {
            return None;
        }
        Some(Self { position, path })
    }

    pub(crate) fn path(&self) -> &[SliceInsertionTargetNode] {
        &self.path
    }

    pub(crate) fn position(&self) -> Position {
        self.position
    }

    fn node_index(&self, id: Dot) -> Option<usize> {
        self.path.iter().position(|node| node.id == id)
    }

    fn node(&self, id: Dot) -> Option<&SliceInsertionTargetNode> {
        self.node_index(id).and_then(|index| self.path.get(index))
    }

    fn parent(&self, id: Dot) -> Option<&SliceInsertionTargetNode> {
        let index = self.node_index(id)?;
        index.checked_sub(1).and_then(|index| self.path.get(index))
    }

    fn textblock(&self) -> Option<&SliceInsertionTargetNode> {
        self.path
            .iter()
            .rev()
            .find(|node| Schema::node_spec(node.node_type).is_textblock())
    }

    fn can_duplicate(&self, id: Dot) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        if Schema::node_spec(node.node_type).isolating {
            return false;
        }
        let Some(parent) = self.parent(id) else {
            return false;
        };
        let Some(index) = node.index else {
            return false;
        };
        let mut child_types = parent.child_types.clone();
        if index > child_types.len() {
            return false;
        }
        child_types.insert(index + 1, node.node_type);
        content_placement(parent.node_type, &child_types).is_valid()
    }
}

impl SliceInsertionTargetShape {
    pub(crate) fn capture(target: &SliceInsertionTarget) -> Self {
        Self {
            offset: target.position.offset,
            path: target
                .path
                .iter()
                .map(|node| SliceInsertionTargetNodeShape {
                    node_type: node.node_type,
                    child_types: node.child_types.clone(),
                    index: node.index,
                })
                .collect(),
        }
    }

    pub(crate) fn matches(&self, actual: &SliceInsertionTarget) -> bool {
        self.offset == actual.position.offset
            && self.path.len() == actual.path.len()
            && self
                .path
                .iter()
                .zip(&actual.path)
                .all(|(expected, actual)| {
                    expected.node_type == actual.node_type
                        && expected.child_types == actual.child_types
                        && expected.index == actual.index
                })
    }
}

pub(crate) fn node_type_path(view: &DocView, node_id: Dot) -> Option<Vec<NodeType>> {
    let node = view.node(node_id)?;
    let mut path: Vec<NodeType> = node
        .ancestors()
        .map(|ancestor| ancestor.node_type())
        .collect();
    path.reverse();
    Some(path)
}

pub(crate) fn top_level_fragments(slice: &Slice) -> Vec<&Fragment> {
    slice.content.iter().collect()
}

pub(crate) fn fragments_fit_parent(parent_type: NodeType, fragments: &[&Fragment]) -> bool {
    content_placement(
        parent_type,
        &fragments
            .iter()
            .map(|fragment| fragment.node.as_type())
            .collect::<Vec<_>>(),
    )
    .is_completable()
}

pub(crate) fn fragments_are_inline(fragments: &[&Fragment]) -> bool {
    !fragments.is_empty()
        && fragments
            .iter()
            .all(|fragment| Schema::node_spec(fragment.node.as_type()).inline)
}

pub(crate) fn inline_fragments_fit_target(
    target: &SliceInsertionTarget,
    fragments: &[Fragment],
) -> bool {
    let Some(textblock) = target.textblock() else {
        return false;
    };
    let mut types = textblock.child_types.clone();
    if target.position.offset > types.len() {
        return false;
    }
    types.splice(
        target.position.offset..target.position.offset,
        fragments.iter().map(|fragment| fragment.node.as_type()),
    );
    content_placement(textblock.node_type, &types).is_valid()
}

fn can_split_textblock_for_structural_insert(
    target: &SliceInsertionTarget,
    textblock_id: Dot,
) -> bool {
    target.can_duplicate(textblock_id)
}

pub(crate) fn fit_slice_for_textblock_target_parent(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> Option<Slice> {
    let textblock = target.textblock()?;
    let parent_type = target.parent(textblock.id)?.node_type;
    let (content, open_start, open_end) = open_fragments_for_parent(
        top_level_fragments(slice),
        slice.open_start,
        slice.open_end,
        parent_type,
    )?;
    Some(Slice::new(
        content.into_iter().cloned().collect(),
        open_start,
        open_end,
    ))
}

#[derive(Clone)]
pub(crate) struct OpenAncestorSplice {
    pub destination: Vec<Dot>,
    pub source: Fragment,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SliceInsertionOutputPlan {
    pub(crate) nodes: Vec<SliceOutputNodeSpec>,
    pub(crate) caret: SliceOutputPositionSpec,
    pub(crate) anchor: SliceOutputPositionSpec,
    pub(crate) head: SliceOutputPositionSpec,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SliceOutputNodeSpec {
    pub(crate) source: SliceOutputSource,
    pub(crate) node_type: NodeType,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum SliceOutputSource {
    InlineSlot {
        index: usize,
    },
    OpenSpliceUnit {
        index: usize,
    },
    TextblockStartInline {
        index: usize,
    },
    TextblockBlockPath {
        block_index: usize,
        child_path: Vec<usize>,
    },
    TextblockEndInline {
        index: usize,
    },
    BlockPath {
        block_index: usize,
        child_path: Vec<usize>,
    },
    SplitLeft,
    SplitRight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SliceOutputPositionSpec {
    pub(crate) node: usize,
    pub(crate) relation: SliceOutputRelation,
    pub(crate) affinity: Affinity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SliceOutputRelation {
    Before,
    After,
    AfterTerminalPageBreak,
    Start,
    End,
}

pub(crate) fn planned_inline_output(
    fragments: &[Fragment],
    anchor_affinity: Affinity,
) -> Option<SliceInsertionOutputPlan> {
    let node_types = inline_output_node_types(fragments);
    let nodes = node_types
        .into_iter()
        .enumerate()
        .map(|(index, node_type)| SliceOutputNodeSpec {
            source: SliceOutputSource::InlineSlot { index },
            node_type,
        })
        .collect::<Vec<_>>();
    output_plan_for_units(
        nodes,
        SliceOutputRelation::After,
        Affinity::Downstream,
        anchor_affinity,
        Affinity::Downstream,
    )
}

pub(crate) fn planned_open_splice_output(
    splice: &OpenAncestorSplice,
) -> Option<SliceInsertionOutputPlan> {
    let mut node_types = Vec::new();
    collect_open_splice_output_types(
        &splice.source,
        splice.destination.len().checked_sub(1)?,
        &mut node_types,
    )?;
    let nodes = node_types
        .into_iter()
        .enumerate()
        .map(|(index, node_type)| SliceOutputNodeSpec {
            source: SliceOutputSource::OpenSpliceUnit { index },
            node_type,
        })
        .collect::<Vec<_>>();
    output_plan_for_units(
        nodes,
        SliceOutputRelation::After,
        Affinity::Downstream,
        Affinity::Upstream,
        Affinity::Downstream,
    )
}

#[derive(Clone)]
pub(crate) struct HoistedBlockInsertionPlan {
    pub target: SliceInsertionTargetShape,
    pub parent_depth: usize,
    pub initial_boundary: HoistInitialBoundary,
    pub boundary_steps: Vec<HoistBoundaryStep>,
    pub blocks: Vec<Fragment>,
    pub list_merges: Vec<PlannedListMerge>,
}

#[derive(Clone, Copy)]
pub(crate) enum HoistInitialBoundary {
    Before,
    After,
    Split,
}

#[derive(Clone, Copy)]
pub(crate) enum HoistBoundaryStep {
    LiftBefore,
    LiftAfter,
    SplitBefore,
    SplitAfter,
}

#[derive(Clone, Copy)]
pub(crate) enum PlannedListMember {
    Left(NodeType),
    Inserted { index: usize, node_type: NodeType },
    Right(NodeType),
}

#[derive(Clone, Copy)]
pub(crate) struct ExistingBlock {
    pub(crate) id: Option<Dot>,
    pub(crate) node_type: NodeType,
}

impl PlannedListMember {
    fn node_type(self) -> NodeType {
        match self {
            Self::Left(node_type) | Self::Right(node_type) | Self::Inserted { node_type, .. } => {
                node_type
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PlannedListMerge {
    pub(crate) earlier: PlannedListMember,
    pub(crate) later: PlannedListMember,
}

pub(crate) fn planned_block_output(
    blocks: &[Fragment],
    list_merges: &[PlannedListMerge],
) -> Option<SliceInsertionOutputPlan> {
    let nodes = planned_block_output_nodes(blocks, list_merges, false);
    let terminal_page_break = blocks.last().is_some_and(paragraph_ends_with_page_break);
    let last_merged = blocks
        .len()
        .checked_sub(1)
        .is_some_and(|index| inserted_member_is_merged(list_merges, index));
    let caret_relation = if terminal_page_break {
        SliceOutputRelation::AfterTerminalPageBreak
    } else if last_merged {
        SliceOutputRelation::After
    } else {
        SliceOutputRelation::End
    };
    output_plan_for_units(
        nodes,
        caret_relation,
        Affinity::Downstream,
        Affinity::Downstream,
        Affinity::Upstream,
    )
}

fn output_plan_for_units(
    nodes: Vec<SliceOutputNodeSpec>,
    caret_relation: SliceOutputRelation,
    caret_affinity: Affinity,
    anchor_affinity: Affinity,
    head_affinity: Affinity,
) -> Option<SliceInsertionOutputPlan> {
    let last = nodes.len().checked_sub(1)?;
    Some(SliceInsertionOutputPlan {
        nodes,
        caret: SliceOutputPositionSpec {
            node: last,
            relation: caret_relation,
            affinity: caret_affinity,
        },
        anchor: SliceOutputPositionSpec {
            node: 0,
            relation: SliceOutputRelation::Before,
            affinity: anchor_affinity,
        },
        head: SliceOutputPositionSpec {
            node: last,
            relation: SliceOutputRelation::After,
            affinity: head_affinity,
        },
    })
}

fn inline_output_node_types(fragments: &[Fragment]) -> Vec<NodeType> {
    let mut output = Vec::new();
    for fragment in fragments {
        match &fragment.node {
            PlainNode::Text(text) => {
                output.extend(std::iter::repeat_n(
                    NodeType::Text,
                    text.text.chars().count(),
                ));
            }
            PlainNode::HardBreak(_) => output.push(NodeType::HardBreak),
            PlainNode::Tab(_) => output.push(NodeType::Tab),
            PlainNode::PageBreak(_) => output.push(NodeType::PageBreak),
            _ => {}
        }
    }
    output
}

fn planned_block_output_nodes(
    blocks: &[Fragment],
    list_merges: &[PlannedListMerge],
    textblock: bool,
) -> Vec<SliceOutputNodeSpec> {
    let mut output = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        let source = |child_path| {
            if textblock {
                SliceOutputSource::TextblockBlockPath {
                    block_index,
                    child_path,
                }
            } else {
                SliceOutputSource::BlockPath {
                    block_index,
                    child_path,
                }
            }
        };
        if inserted_member_is_merged(list_merges, block_index) && is_list_type(block.node.as_type())
        {
            output.extend(block.children.iter().enumerate().map(|(index, child)| {
                SliceOutputNodeSpec {
                    source: source(vec![index]),
                    node_type: child.node.as_type(),
                }
            }));
        } else {
            output.push(SliceOutputNodeSpec {
                source: source(Vec::new()),
                node_type: block.node.as_type(),
            });
        }
    }
    output
}

fn inserted_member_is_merged(merges: &[PlannedListMerge], index: usize) -> bool {
    merges.iter().any(|merge| {
        [merge.earlier, merge.later].into_iter().any(|member| {
            matches!(
                member,
                PlannedListMember::Inserted {
                    index: member_index,
                    ..
                } if member_index == index
            )
        })
    })
}

fn collect_open_splice_output_types(
    source: &Fragment,
    depth: usize,
    output: &mut Vec<NodeType>,
) -> Option<()> {
    if depth == 0 {
        return None;
    }
    match source.children.as_slice() {
        [] => None,
        [only] => collect_open_join_output_types(only, depth, output),
        [first, middle @ .., last] => {
            collect_open_append_output_types(first, depth, output)?;
            output.extend(middle.iter().map(|fragment| fragment.node.as_type()));
            collect_open_prepend_output_types(last, depth, output)
        }
    }
}

fn collect_open_join_output_types(
    source: &Fragment,
    depth: usize,
    output: &mut Vec<NodeType>,
) -> Option<()> {
    if depth == 0 {
        return None;
    }
    if depth == 1 {
        output.extend(inline_output_node_types(&source.children));
        return Some(());
    }
    match source.children.as_slice() {
        [] => None,
        [only] => collect_open_join_output_types(only, depth - 1, output),
        [first, middle @ .., last] => {
            collect_open_append_output_types(first, depth - 1, output)?;
            output.extend(middle.iter().map(|fragment| fragment.node.as_type()));
            collect_open_prepend_output_types(last, depth - 1, output)
        }
    }
}

fn collect_open_append_output_types(
    source: &Fragment,
    depth: usize,
    output: &mut Vec<NodeType>,
) -> Option<()> {
    if depth == 0 {
        return None;
    }
    if depth == 1 {
        output.extend(inline_output_node_types(&source.children));
        return Some(());
    }
    let first = source.children.first()?;
    collect_open_append_output_types(first, depth - 1, output)?;
    output.extend(
        source
            .children
            .iter()
            .skip(1)
            .map(|fragment| fragment.node.as_type()),
    );
    Some(())
}

fn collect_open_prepend_output_types(
    source: &Fragment,
    depth: usize,
    output: &mut Vec<NodeType>,
) -> Option<()> {
    if depth == 0 {
        return None;
    }
    if depth == 1 {
        output.extend(inline_output_node_types(&source.children));
        return Some(());
    }
    let last = source.children.last()?;
    output.extend(
        source
            .children
            .iter()
            .take(source.children.len() - 1)
            .map(|fragment| fragment.node.as_type()),
    );
    collect_open_prepend_output_types(last, depth - 1, output)
}

pub(crate) fn plan_adjacent_list_merges(
    left: Option<ExistingBlock>,
    blocks: &[Fragment],
    right: Option<ExistingBlock>,
) -> Option<Vec<PlannedListMerge>> {
    let mut members = Vec::with_capacity(
        blocks.len() + usize::from(left.is_some()) + usize::from(right.is_some()),
    );
    members.extend(left.map(|block| PlannedListMember::Left(block.node_type)));
    let insertion_start = members.len();
    members.extend(
        blocks
            .iter()
            .enumerate()
            .map(|(index, block)| PlannedListMember::Inserted {
                index,
                node_type: block.node.as_type(),
            }),
    );
    members.extend(right.map(|block| PlannedListMember::Right(block.node_type)));

    let mut planned = Vec::new();
    let mut index = insertion_start.saturating_sub(1);
    let mut scan_end = insertion_start.saturating_add(blocks.len());
    while index < scan_end && index + 1 < members.len() {
        if is_list_type(members[index].node_type()) && is_list_type(members[index + 1].node_type())
        {
            planned.push(PlannedListMerge {
                earlier: members[index],
                later: members[index + 1],
            });
            members.remove(index + 1);
            scan_end = scan_end.saturating_sub(1).max(index + 1);
        } else {
            index += 1;
        }
    }
    let existing_participant_is_synthetic = planned.iter().any(|merge_plan| {
        [merge_plan.earlier, merge_plan.later]
            .into_iter()
            .any(|member| match member {
                PlannedListMember::Left(_) => left
                    .and_then(|block| block.id)
                    .is_some_and(|id| id.is_synthetic()),
                PlannedListMember::Right(_) => right
                    .and_then(|block| block.id)
                    .is_some_and(|id| id.is_synthetic()),
                PlannedListMember::Inserted { .. } => false,
            })
    });
    (!existing_participant_is_synthetic).then_some(planned)
}

/// Match a symmetrically open Slice edge to the destination ancestor chain.
///
/// The matched source wrappers are context, not inserted nodes. Their children
/// are spliced through the surviving destination wrappers. This is the generic
/// form of list-item and blockquote open-edge insertion.
pub(crate) fn open_ancestor_splice_for_target(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> Option<OpenAncestorSplice> {
    let (splice, emits_change) = open_ancestor_splice_match_for_target(target, slice)?;
    emits_change.then_some(splice)
}

pub(crate) fn open_ancestor_splice_is_complete_no_output(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> bool {
    open_ancestor_splice_match_for_target(target, slice).is_some_and(|(splice, _)| {
        open_splice_is_complete_no_output(&splice.source, splice.destination.len())
    })
}

fn open_ancestor_splice_match_for_target(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> Option<(OpenAncestorSplice, bool)> {
    if slice.open_start != slice.open_end || slice.open_start < 2 || slice.content.len() != 1 {
        return None;
    }
    let depth = usize::try_from(slice.open_start).ok()?;
    let source = slice.content.first()?;
    let source_left = fragment_edge_types(source, depth, true)?;
    let source_right = fragment_edge_types(source, depth, false)?;

    let textblock_index = target.node_index(target.textblock()?.id)?;
    let destination_start = textblock_index.checked_add(1)?.checked_sub(depth)?;
    let destination: Vec<Dot> = target.path[destination_start..=textblock_index]
        .iter()
        .map(|node| node.id)
        .collect();
    if destination
        .iter()
        .any(|node| node.as_op_dot().is_none() && *node != Dot::ROOT)
    {
        return None;
    }

    for ((destination, left), right) in destination.iter().zip(source_left).zip(source_right) {
        let destination_type = target.node(*destination)?.node_type;
        if !open_wrapper_compatible(destination_type, left)
            || !open_wrapper_compatible(destination_type, right)
        {
            return None;
        }
    }
    if !destination[1..]
        .iter()
        .all(|node| target.can_duplicate(*node))
    {
        return None;
    }

    Some((
        OpenAncestorSplice {
            destination,
            source: source.clone(),
        },
        open_splice_emits_change(source, depth),
    ))
}

fn fragment_edge_types(fragment: &Fragment, depth: usize, first: bool) -> Option<Vec<NodeType>> {
    let mut current = fragment;
    let mut types = Vec::with_capacity(depth);
    for level in 0..depth {
        types.push(current.node.as_type());
        if level + 1 < depth {
            current = if first {
                current.children.first()?
            } else {
                current.children.last()?
            };
        }
    }
    Some(types)
}

fn open_wrapper_compatible(destination: NodeType, source: NodeType) -> bool {
    destination == source
        || matches!(
            (destination, source),
            (
                NodeType::BulletList | NodeType::OrderedList,
                NodeType::BulletList | NodeType::OrderedList
            )
        )
}

fn open_splice_emits_change(source: &Fragment, depth: usize) -> bool {
    let mut current = source;
    for level in 0..depth {
        if level + 1 == depth {
            return current.children.iter().any(is_insertable_inline_fragment);
        }
        if current.children.len() != 1 {
            return !current.children.is_empty();
        }
        current = &current.children[0];
    }
    false
}

fn open_splice_is_complete_no_output(source: &Fragment, depth: usize) -> bool {
    let mut current = source;
    for level in 0..depth {
        if level + 1 == depth {
            return current.children.iter().all(is_supported_inline_fragment)
                && !current.children.iter().any(is_insertable_inline_fragment);
        }
        let [only] = current.children.as_slice() else {
            return false;
        };
        current = only;
    }
    false
}

pub(crate) fn open_inline_content_for_target<'a>(
    target: &SliceInsertionTarget,
    slice: &'a Slice,
) -> Option<Vec<&'a Fragment>> {
    let textblock_type = target.textblock()?.node_type;

    let top_level = top_level_fragments(slice);
    if slice.open_start == 0 && slice.open_end == 0 {
        return None;
    }

    let (open_content, _, _) = open_fragments_for_parent(
        top_level.clone(),
        slice.open_start,
        slice.open_end,
        textblock_type,
    )?;
    if !fragments_are_inline(&open_content) {
        return None;
    }
    fragments_fit_parent(textblock_type, &open_content).then_some(open_content)
}

pub(crate) fn insert_content_as_inline_at_position(
    tr: &mut Transaction,
    position: Position,
    fragments: Vec<Fragment>,
    mode: &InlineMode,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    if fragments.is_empty() {
        return Ok(None);
    }

    tr.set_selection(Some(Selection::collapsed(position)))?;
    let start = tr
        .selection()
        .expect("selection preserved through mutations")
        .head;
    let inserted = insert_inline_fragments(tr, &fragments, mode)?;
    if !inserted {
        return Ok(None);
    }
    let mut end = tr
        .selection()
        .expect("selection preserved through mutations")
        .head;
    end.affinity = Affinity::Downstream;
    tr.set_selection(Some(Selection::collapsed(end)))?;
    let units = output_units_between(tr, start, end)?;
    Ok(Some(SliceInsertionExecution {
        inserted: Selection::new(start, end),
        units,
        split_left: None,
        split_right: None,
    }))
}

pub(crate) fn insert_blocks_in_textblock_at_position(
    tr: &mut Transaction,
    position: Position,
    plan: &TextblockSplicePlan,
    mode: &InlineMode,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    tr.set_selection(Some(Selection::collapsed(position)))?;
    insert_blocks_in_textblock(tr, plan, mode)
}

pub(crate) fn insert_open_ancestor_splice_at_position(
    tr: &mut Transaction,
    position: Position,
    splice: OpenAncestorSplice,
    mode: &InlineMode,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    let depth = splice.destination.len();
    if depth < 2 {
        return Ok(None);
    }

    let start = Position {
        node: *splice.destination.last().unwrap(),
        offset: position.offset,
        affinity: Affinity::Upstream,
    };
    let left = splice.destination;
    let mut right = left.clone();
    let mut output_units = Vec::new();
    tr.batch::<_, CommandError>(|tr| {
        let deepest = depth - 1;
        tr.split_node(left[deepest], position.offset)?;
        right[deepest] = next_block_sibling_id(tr, left[deepest])?;

        for level in (1..deepest).rev() {
            let (wrapper, aliases) =
                split_block_wrapper_before_child(tr, left[level], right[level + 1])?;
            right[level] = wrapper;
            for descendant in &mut right[level + 1..] {
                if let Some((_, remapped)) = aliases.iter().find(|(old, _)| old == descendant) {
                    *descendant = *remapped;
                }
            }
        }

        let mut inserted = InsertedRange::default();
        let source_children = &splice.source.children;
        match source_children.as_slice() {
            [] => {
                return Err(CommandError::Corrupted(
                    "open Slice wrapper has no edge child".into(),
                ));
            }
            [only] => {
                join_open_fragment_both(tr, &left[1..], &right[1..], only, mode, &mut inserted)?;
            }
            [first, middle @ .., last] => {
                append_open_fragment(tr, &left[1..], first, mode, &mut inserted)?;
                insert_fragments_after(tr, left[0], left[1], middle, mode, &mut inserted)?;
                prepend_open_fragment(tr, &right[1..], last, mode, &mut inserted)?;
            }
        }

        let mut end = inserted
            .end
            .clone()
            .and_then(|endpoint| resolve_inserted_range_endpoint(tr, endpoint))
            .ok_or_else(|| {
                CommandError::Corrupted(
                    "open-ancestor Slice plan produced no inserted endpoint".into(),
                )
            })?;
        end.affinity = Affinity::Downstream;
        output_units = inserted.units.clone();
        tr.set_selection(Some(Selection::collapsed(end)))?;
        Ok(())
    })?;

    let end = tr
        .selection()
        .map(|selection| selection.head)
        .ok_or_else(|| {
            CommandError::Corrupted("open-ancestor Slice plan produced no final caret".into())
        })?;
    Ok(Some(SliceInsertionExecution {
        inserted: Selection::new(start, end),
        units: output_units,
        split_left: None,
        split_right: None,
    }))
}

pub(crate) fn split_block_wrapper_before_child(
    tr: &mut Transaction,
    wrapper: Dot,
    first_right: Dot,
) -> Result<(Dot, Vec<(Dot, Dot)>), CommandError> {
    let wrapper = super::materialize_target(tr, wrapper)?;
    let first_right = super::materialize_target(tr, first_right)?;
    loop {
        let scaffold = {
            let view = tr.view();
            let wrapper_view = view
                .node(wrapper)
                .ok_or(CommandError::NodeNotFound(wrapper))?;
            let first_right_index = wrapper_view
                .children()
                .position(|child| match child {
                    ChildView::Block(block) => block.id() == first_right,
                    ChildView::Leaf(leaf) => leaf.dot() == first_right,
                })
                .ok_or_else(|| CommandError::orphan_child(first_right, wrapper))?;
            wrapper_view
                .children()
                .skip(first_right_index)
                .filter_map(|child| match child {
                    ChildView::Block(block) => Some(block),
                    ChildView::Leaf(_) => None,
                })
                .find(|child| {
                    child.id().is_synthetic()
                        && child.descendants().any(|descendant| match descendant {
                            ChildView::Block(block) => block.id().as_op_dot().is_some(),
                            ChildView::Leaf(leaf) => leaf.dot().as_op_dot().is_some(),
                        })
                })
                .map(|child| child.id())
        };
        let Some(scaffold) = scaffold else {
            break;
        };
        super::materialize_target(tr, scaffold)?;
    }
    let (parent, insert_at, container, moving) = {
        let state = tr.state();
        let view = state.view();
        let wrapper_view = view
            .node(wrapper)
            .ok_or(CommandError::NodeNotFound(wrapper))?;
        let parent = wrapper_view
            .parent()
            .ok_or(CommandError::NoParent(wrapper))?;
        let wrapper_index = wrapper_view
            .index()
            .ok_or_else(|| CommandError::orphan_child(wrapper, parent.id()))?;
        let first_right_index = wrapper_view
            .children()
            .position(|child| match child {
                ChildView::Block(block) => block.id() == first_right,
                ChildView::Leaf(leaf) => leaf.dot() == first_right,
            })
            .ok_or_else(|| CommandError::orphan_child(first_right, wrapper))?;
        let moving: Vec<Dot> = wrapper_view
            .children()
            .skip(first_right_index)
            .filter_map(|child| match child {
                ChildView::Block(block) => Some(block.id()),
                ChildView::Leaf(leaf) => Some(leaf.dot()),
            })
            .filter(|dot| dot.as_op_dot().is_some())
            .collect();
        let container = Subtree {
            node: wrapper_view.node().to_plain(),
            modifiers: state
                .projected
                .block_modifiers()
                .modifiers_of(wrapper)
                .into_values()
                .collect(),
            carry: state
                .projected
                .carry_modifiers(wrapper)
                .into_values()
                .collect(),
            children: Vec::new(),
            source_dots: Vec::new(),
        };
        (parent.id(), wrapper_index + 1, container, moving)
    };
    if moving.is_empty() {
        return Err(CommandError::Corrupted(
            "open splice wrapper has no right contribution".into(),
        ));
    }
    let (right, moved) = tr.insert_subtree_with_moved(parent, insert_at, container, &moving)?;
    Ok((
        right,
        moved.into_iter().flat_map(|moved| moved.pairs).collect(),
    ))
}

pub(crate) fn insert_hoisted_blocks_at_position(
    tr: &mut Transaction,
    position: Position,
    plan: HoistedBlockInsertionPlan,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    let actual_target = SliceInsertionTarget::from_view(&tr.view(), position).ok_or_else(|| {
        CommandError::Corrupted("hoisted Slice insertion lost its target structure".into())
    })?;
    if !plan.target.matches(&actual_target) {
        return Err(CommandError::Corrupted(
            "hoisted Slice insertion target changed after planning".into(),
        ));
    }

    #[derive(Clone, Copy)]
    enum LiftedBoundary {
        Before(Dot),
        After(Dot),
    }

    let child_count = actual_target
        .path()
        .last()
        .map(|node| node.child_types.len())
        .ok_or_else(|| CommandError::Corrupted("hoisted Slice target has no path".into()))?;
    let mut boundary = match plan.initial_boundary {
        HoistInitialBoundary::Before if position.offset == 0 => {
            LiftedBoundary::Before(position.node)
        }
        HoistInitialBoundary::After if position.offset == child_count => {
            LiftedBoundary::After(position.node)
        }
        HoistInitialBoundary::Split if position.offset > 0 && position.offset < child_count => {
            tr.split_node(position.node, position.offset)?;
            tr.projected_clean()
                .map_err(editor_transaction::StepError::from)?;
            LiftedBoundary::Before(next_block_sibling_id(tr, position.node)?)
        }
        _ => {
            return Err(CommandError::Corrupted(
                "hoisted Slice initial boundary changed after planning".into(),
            ));
        }
    };

    for step in &plan.boundary_steps {
        let child = match boundary {
            LiftedBoundary::Before(child) | LiftedBoundary::After(child) => child,
        };
        let (parent, index, sibling_count, next_right) = {
            let view = tr
                .view_clean()
                .map_err(editor_transaction::StepError::from)?;
            let node = view.node(child).ok_or(CommandError::NodeNotFound(child))?;
            let parent = node.parent().ok_or(CommandError::NoParent(child))?;
            let index = node
                .index()
                .ok_or_else(|| CommandError::orphan_child(child, parent.id()))?;
            let sibling_count = parent.children().count();
            let next_right = parent.child_at(index + 1).and_then(|child| match child {
                ChildView::Block(block) => Some(block.id()),
                ChildView::Leaf(_) => None,
            });
            (parent.id(), index, sibling_count, next_right)
        };

        boundary = match (step, boundary) {
            (HoistBoundaryStep::LiftBefore, LiftedBoundary::Before(_)) if index == 0 => {
                LiftedBoundary::Before(parent)
            }
            (HoistBoundaryStep::LiftAfter, LiftedBoundary::After(_))
                if index + 1 == sibling_count =>
            {
                LiftedBoundary::After(parent)
            }
            (HoistBoundaryStep::SplitBefore, LiftedBoundary::Before(first_right)) if index > 0 => {
                let (right_wrapper, _) = split_block_wrapper_before_child(tr, parent, first_right)?;
                LiftedBoundary::Before(right_wrapper)
            }
            (HoistBoundaryStep::SplitAfter, LiftedBoundary::After(_))
                if index + 1 < sibling_count =>
            {
                let first_right = next_right.ok_or_else(|| {
                    CommandError::Corrupted("hoisted Slice boundary lost its right sibling".into())
                })?;
                let (right_wrapper, _) = split_block_wrapper_before_child(tr, parent, first_right)?;
                LiftedBoundary::Before(right_wrapper)
            }
            _ => {
                return Err(CommandError::Corrupted(
                    "hoisted Slice boundary step changed after planning".into(),
                ));
            }
        };
    }

    let child = match boundary {
        LiftedBoundary::Before(child) | LiftedBoundary::After(child) => child,
    };
    let (parent, index) = {
        let view = tr
            .view_clean()
            .map_err(editor_transaction::StepError::from)?;
        let node = view.node(child).ok_or(CommandError::NodeNotFound(child))?;
        let parent = node.parent().ok_or(CommandError::NoParent(child))?;
        let index = node
            .index()
            .ok_or_else(|| CommandError::orphan_child(child, parent.id()))?;
        (parent.id(), index)
    };
    let planned_parent = actual_target
        .path()
        .get(plan.parent_depth)
        .map(|node| node.id)
        .ok_or_else(|| {
            CommandError::Corrupted("hoisted Slice parent depth changed after planning".into())
        })?;
    if parent != planned_parent {
        return Err(CommandError::Corrupted(
            "hoisted Slice boundary did not reach its planned parent".into(),
        ));
    }
    let insertion = Position::new(
        parent,
        match boundary {
            LiftedBoundary::Before(_) => index,
            LiftedBoundary::After(_) => index + 1,
        },
    );

    tr.projected_clean()
        .map_err(editor_transaction::StepError::from)?;
    insert_blocks_at_block_boundary(tr, insertion, plan.blocks, plan.list_merges)
}

fn next_block_sibling_id(tr: &Transaction, node: Dot) -> Result<Dot, CommandError> {
    let view = tr.state().view();
    view.node(node)
        .and_then(|node| next_sibling(&node))
        .and_then(|sibling| match sibling {
            ChildView::Block(block) => Some(block.id()),
            ChildView::Leaf(_) => None,
        })
        .ok_or_else(|| CommandError::Corrupted("split produced no right wrapper".into()))
}

fn join_open_fragment_both(
    tr: &mut Transaction,
    left: &[Dot],
    right: &[Dot],
    source: &Fragment,
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    let (&left_node, &right_node) = left
        .first()
        .zip(right.first())
        .ok_or_else(|| CommandError::Corrupted("open splice exhausted destination edge".into()))?;
    if left.len() == 1 {
        insert_inline_at_edge(tr, left_node, &source.children, false, mode, inserted)?;
        tr.merge_node(left_node)?;
        return Ok(());
    }

    match source.children.as_slice() {
        [] => {
            return Err(CommandError::Corrupted(
                "open Slice wrapper has no edge child".into(),
            ));
        }
        [only] => {
            join_open_fragment_both(tr, &left[1..], &right[1..], only, mode, inserted)?;
        }
        [first, middle @ .., last] => {
            append_open_fragment(tr, &left[1..], first, mode, inserted)?;
            insert_fragments_after(tr, left_node, left[1], middle, mode, inserted)?;
            prepend_open_fragment(tr, &right[1..], last, mode, inserted)?;
        }
    }
    tr.merge_node(left_node)?;
    let _ = right_node;
    Ok(())
}

fn append_open_fragment(
    tr: &mut Transaction,
    destination: &[Dot],
    source: &Fragment,
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    let &node = destination
        .first()
        .ok_or_else(|| CommandError::Corrupted("open splice exhausted left edge".into()))?;
    if destination.len() == 1 {
        return insert_inline_at_edge(tr, node, &source.children, false, mode, inserted);
    }
    let first = source
        .children
        .first()
        .ok_or_else(|| CommandError::Corrupted("open Slice left edge is empty".into()))?;
    append_open_fragment(tr, &destination[1..], first, mode, inserted)?;
    insert_fragments_at_end(tr, node, &source.children[1..], mode, inserted)
}

fn prepend_open_fragment(
    tr: &mut Transaction,
    destination: &[Dot],
    source: &Fragment,
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    let &node = destination
        .first()
        .ok_or_else(|| CommandError::Corrupted("open splice exhausted right edge".into()))?;
    if destination.len() == 1 {
        return insert_inline_at_edge(tr, node, &source.children, true, mode, inserted);
    }
    let last = source
        .children
        .last()
        .ok_or_else(|| CommandError::Corrupted("open Slice right edge is empty".into()))?;
    insert_fragments_before(
        tr,
        node,
        destination[1],
        &source.children[..source.children.len() - 1],
        mode,
        inserted,
    )?;
    prepend_open_fragment(tr, &destination[1..], last, mode, inserted)
}

fn insert_inline_at_edge(
    tr: &mut Transaction,
    block: Dot,
    fragments: &[Fragment],
    at_start: bool,
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    let position = if at_start {
        position_at_start_of_block(tr, block)?
    } else {
        position_at_end_of_block(tr, block)?
    };
    if let Some(execution) =
        insert_content_as_inline_at_position(tr, position, fragments.to_vec(), mode)?
    {
        inserted.include_position_range(execution.inserted.anchor, execution.inserted.head);
        inserted.units.extend(execution.units);
    }
    Ok(())
}

fn insert_fragments_after(
    tr: &mut Transaction,
    parent: Dot,
    child: Dot,
    fragments: &[Fragment],
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    if fragments.is_empty() {
        return Ok(());
    }
    let index = {
        let view = tr.state().view();
        let (actual_parent, index) =
            block_parent_and_index(&view, child).ok_or(CommandError::NodeNotFound(child))?;
        if actual_parent != parent {
            return Err(CommandError::Corrupted(
                "open splice child escaped its destination wrapper".into(),
            ));
        }
        index + 1
    };
    insert_closed_fragments(tr, parent, index, fragments, mode, inserted)
}

fn insert_fragments_before(
    tr: &mut Transaction,
    parent: Dot,
    child: Dot,
    fragments: &[Fragment],
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    if fragments.is_empty() {
        return Ok(());
    }
    let index = {
        let view = tr.state().view();
        let (actual_parent, index) =
            block_parent_and_index(&view, child).ok_or(CommandError::NodeNotFound(child))?;
        if actual_parent != parent {
            return Err(CommandError::Corrupted(
                "open splice child escaped its destination wrapper".into(),
            ));
        }
        index
    };
    insert_closed_fragments(tr, parent, index, fragments, mode, inserted)
}

fn insert_fragments_at_end(
    tr: &mut Transaction,
    parent: Dot,
    fragments: &[Fragment],
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    if fragments.is_empty() {
        return Ok(());
    }
    let index = tr
        .state()
        .view()
        .node(parent)
        .map(|node| node.children().count())
        .ok_or(CommandError::NodeNotFound(parent))?;
    insert_closed_fragments(tr, parent, index, fragments, mode, inserted)
}

fn insert_closed_fragments(
    tr: &mut Transaction,
    parent: Dot,
    index: usize,
    fragments: &[Fragment],
    mode: &InlineMode,
    inserted: &mut InsertedRange,
) -> Result<(), CommandError> {
    for (offset, fragment) in fragments.iter().enumerate() {
        let inserted_id =
            tr.insert_subtree(parent, index + offset, fragment.clone().into_subtree())?;
        if let Some(inserted_id) = inserted_id {
            inserted.include_block(inserted_id);
            paint_inserted_subtree(tr, inserted_id, mode)?;
        }
    }
    Ok(())
}

fn paint_inserted_subtree(
    tr: &mut Transaction,
    root: Dot,
    mode: &InlineMode,
) -> Result<(), CommandError> {
    let Some(paint) = mode.plain_paint().map(ToOwned::to_owned) else {
        return Ok(());
    };
    let textblocks: Vec<Dot> = {
        let view = tr.state().view();
        let Some(root) = view.node(root) else {
            return Ok(());
        };
        std::iter::once(root.clone())
            .chain(root.descendants().filter_map(|child| match child {
                ChildView::Block(block) => Some(block),
                ChildView::Leaf(_) => None,
            }))
            .filter(|node| node.spec().is_textblock())
            .map(|node| node.id())
            .collect()
    };
    for textblock in textblocks {
        paint_block_uniformly(tr, textblock, &paint)?;
    }
    Ok(())
}

fn insert_inline_fragments(
    tr: &mut Transaction,
    fragments: &[Fragment],
    mode: &InlineMode,
) -> CommandResult {
    let mut any_change = false;
    for f in fragments {
        match &f.node {
            PlainNode::Text(t) if t.text.is_empty() => {}
            PlainNode::Text(t) if !t.text.is_empty() => {
                insert_text_at_caret(tr, &t.text, Some(mode.paint_for(f)))?;
                any_change = true;
            }
            PlainNode::HardBreak(_) => {
                insert_hard_break_at_caret(tr, Some(mode.paint_for(f)))?;
                any_change = true;
            }
            PlainNode::Tab(_) => {
                insert_tab_at_caret(tr, Some(mode.paint_for(f)))?;
                any_change = true;
            }
            _ => {
                return Err(CommandError::Corrupted(format!(
                    "non-inline fragment reached inline Slice executor: {:?}",
                    f.node.as_type()
                )));
            }
        }
    }
    Ok(any_change)
}

pub(crate) fn is_supported_inline_fragment(fragment: &Fragment) -> bool {
    matches!(
        &fragment.node,
        PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_)
    )
}

pub(crate) fn is_insertable_inline_fragment(fragment: &Fragment) -> bool {
    match &fragment.node {
        PlainNode::Text(t) => !t.text.is_empty(),
        PlainNode::HardBreak(_) | PlainNode::Tab(_) => true,
        _ => false,
    }
}

#[derive(Clone)]
enum InsertedRangeEndpoint {
    Position(Position),
    BeforeBlock(Dot),
    AfterBlock(Dot),
}

pub(crate) struct SliceInsertionExecution {
    pub(crate) inserted: Selection,
    pub(crate) units: Vec<Dot>,
    pub(crate) split_left: Option<Dot>,
    pub(crate) split_right: Option<Dot>,
}

#[derive(Default)]
struct InsertedRange {
    start: Option<InsertedRangeEndpoint>,
    end: Option<InsertedRangeEndpoint>,
    blocks: Vec<Dot>,
    units: Vec<Dot>,
}

impl InsertedRange {
    fn prepend_position(&mut self, start: Position) {
        self.start = Some(InsertedRangeEndpoint::Position(start));
    }

    fn include_position_range(&mut self, start: Position, end: Position) {
        self.start
            .get_or_insert(InsertedRangeEndpoint::Position(start));
        self.end = Some(InsertedRangeEndpoint::Position(end));
    }

    fn include_block(&mut self, block_id: Dot) {
        self.start
            .get_or_insert(InsertedRangeEndpoint::BeforeBlock(block_id));
        self.end = Some(InsertedRangeEndpoint::AfterBlock(block_id));
        self.blocks.push(block_id);
        self.units.push(block_id);
    }

    fn selection(&self, tr: &Transaction) -> Result<Option<Selection>, CommandError> {
        let mut endpoints = Vec::new();
        if let Some(start) = self.start.clone() {
            endpoints.push(resolve_inserted_range_endpoint(tr, start).ok_or_else(|| {
                CommandError::Corrupted("planned Slice start output no longer resolves".into())
            })?);
        }
        if let Some(end) = self.end.clone() {
            endpoints.push(resolve_inserted_range_endpoint(tr, end).ok_or_else(|| {
                CommandError::Corrupted("planned Slice end output no longer resolves".into())
            })?);
        }
        for block in &self.blocks {
            endpoints.push(
                resolve_inserted_range_endpoint(tr, InsertedRangeEndpoint::BeforeBlock(*block))
                    .ok_or_else(|| {
                        CommandError::Corrupted(
                            "planned Slice block-start output no longer resolves".into(),
                        )
                    })?,
            );
            endpoints.push(
                resolve_inserted_range_endpoint(tr, InsertedRangeEndpoint::AfterBlock(*block))
                    .ok_or_else(|| {
                        CommandError::Corrupted(
                            "planned Slice block-end output no longer resolves".into(),
                        )
                    })?,
            );
        }

        let view = tr.state().view();
        let resolved = endpoints
            .iter()
            .map(|position| {
                position.resolve(&view).ok_or_else(|| {
                    CommandError::Corrupted(
                        "planned Slice output position no longer resolves".into(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(start) = resolved.iter().min().map(|position| position.position()) else {
            return Ok(None);
        };
        let end = resolved
            .iter()
            .max()
            .map(|position| position.position())
            .ok_or_else(|| {
                CommandError::Corrupted("planned Slice output range has no end".into())
            })?;
        Ok(Some(Selection::new(start, end)))
    }
}

fn output_units_between(
    tr: &Transaction,
    start: Position,
    end: Position,
) -> Result<Vec<Dot>, CommandError> {
    if start.node != end.node || start.offset > end.offset {
        return Err(CommandError::Corrupted(
            "inline Slice output escaped its planned textblock".into(),
        ));
    }
    let view = tr.view();
    let node = view
        .node(start.node)
        .ok_or(CommandError::NodeNotFound(start.node))?;
    let units = node
        .children()
        .skip(start.offset)
        .take(end.offset - start.offset)
        .map(|child| match child {
            ChildView::Block(block) => block.id(),
            ChildView::Leaf(leaf) => leaf.dot(),
        })
        .collect::<Vec<_>>();
    if units.len() != end.offset - start.offset {
        return Err(CommandError::Corrupted(
            "inline Slice output produced a different child span".into(),
        ));
    }
    Ok(units)
}

/// Elem id at child slot `index` of `container` when it is an addressable block
/// child: a real block, or a block-level atom leaf (Image/HR/…). Inline leaves
/// (chars, Tab, HardBreak) return None.
fn block_child_id(tr: &Transaction, container: Dot, index: usize) -> Option<Dot> {
    let view = tr.state().view();
    match view.node(container)?.child_at(index)? {
        ChildView::Block(b) => Some(b.id()),
        ChildView::Leaf(l) => l.as_atom().filter(|a| a.is_block_level()).map(|_| l.dot()),
    }
}

fn textblock_edge_joins(
    destination_type: NodeType,
    destination_children: &[NodeType],
    offset: usize,
    slice: &Slice,
) -> (bool, bool) {
    if offset > destination_children.len() {
        return (false, false);
    }
    let join_start = offset > 0
        && slice.open_start > 0
        && slice.content.first().is_some_and(|source| {
            source.node.as_type() == destination_type
                && content_placement(
                    destination_type,
                    &destination_children[..offset]
                        .iter()
                        .copied()
                        .chain(source.children.iter().map(|child| child.node.as_type()))
                        .collect::<Vec<_>>(),
                )
                .is_valid()
        });
    let mut join_end = offset < destination_children.len()
        && slice.open_end > 0
        && slice.content.last().is_some_and(|source| {
            source.node.as_type() == destination_type
                && content_placement(
                    destination_type,
                    &source
                        .children
                        .iter()
                        .map(|child| child.node.as_type())
                        .chain(destination_children[offset..].iter().copied())
                        .collect::<Vec<_>>(),
                )
                .is_valid()
        });

    if join_start && join_end && slice.content.len() == 1 {
        // Pairwise-valid joins may still form an invalid three-part sequence.
        // Preserve the start join and leave the end in its destination wrapper.
        let source = &slice.content[0];
        join_end = content_placement(
            destination_type,
            &destination_children[..offset]
                .iter()
                .copied()
                .chain(source.children.iter().map(|child| child.node.as_type()))
                .chain(destination_children[offset..].iter().copied())
                .collect::<Vec<_>>(),
        )
        .is_valid();
    }

    (join_start, join_end)
}

#[derive(Clone)]
pub(crate) struct TextblockSplicePlan {
    blocks: Vec<Fragment>,
    inserted_blocks: Vec<Fragment>,
    textblock_type: NodeType,
    container_type: NodeType,
    textblock_index: usize,
    child_count: usize,
    split_index: usize,
    has_left: bool,
    has_right: bool,
    join_start: bool,
    merge_destinations: bool,
    merge_end: bool,
    unjoined_ends_with_page_break: bool,
    final_caret_at_right_boundary: bool,
    insert_at: usize,
    list_merges: Vec<PlannedListMerge>,
}

impl TextblockSplicePlan {
    pub(crate) fn planned_output(&self) -> Option<SliceInsertionOutputPlan> {
        let mut nodes = Vec::new();
        if self.join_start {
            nodes.extend(
                inline_output_node_types(&self.blocks.first()?.children)
                    .into_iter()
                    .enumerate()
                    .map(|(index, node_type)| SliceOutputNodeSpec {
                        source: SliceOutputSource::TextblockStartInline { index },
                        node_type,
                    }),
            );
        }
        nodes.extend(planned_block_output_nodes(
            &self.inserted_blocks,
            &self.list_merges,
            true,
        ));
        if self.merge_end {
            nodes.extend(
                inline_output_node_types(&self.blocks.last()?.children)
                    .into_iter()
                    .enumerate()
                    .map(|(index, node_type)| SliceOutputNodeSpec {
                        source: SliceOutputSource::TextblockEndInline { index },
                        node_type,
                    }),
            );
        }

        if nodes.is_empty() {
            if !self.final_caret_at_right_boundary {
                return None;
            }
            return Some(SliceInsertionOutputPlan {
                nodes: vec![
                    SliceOutputNodeSpec {
                        source: SliceOutputSource::SplitLeft,
                        node_type: self.textblock_type,
                    },
                    SliceOutputNodeSpec {
                        source: SliceOutputSource::SplitRight,
                        node_type: self.textblock_type,
                    },
                ],
                caret: SliceOutputPositionSpec {
                    node: 1,
                    relation: SliceOutputRelation::Start,
                    affinity: Affinity::Downstream,
                },
                anchor: SliceOutputPositionSpec {
                    node: 0,
                    relation: SliceOutputRelation::End,
                    affinity: Affinity::Upstream,
                },
                head: SliceOutputPositionSpec {
                    node: 1,
                    relation: SliceOutputRelation::Start,
                    affinity: Affinity::Downstream,
                },
            });
        }

        let last_inserted_merged = self
            .inserted_blocks
            .len()
            .checked_sub(1)
            .is_some_and(|index| inserted_member_is_merged(&self.list_merges, index));
        let caret_relation = if self.merge_end {
            SliceOutputRelation::After
        } else if !self.inserted_blocks.is_empty() {
            if self.unjoined_ends_with_page_break {
                SliceOutputRelation::AfterTerminalPageBreak
            } else if last_inserted_merged {
                SliceOutputRelation::After
            } else {
                SliceOutputRelation::End
            }
        } else if nodes
            .last()
            .is_some_and(|node| node.node_type == NodeType::PageBreak)
        {
            SliceOutputRelation::AfterTerminalPageBreak
        } else {
            SliceOutputRelation::After
        };
        let head_affinity = if self.merge_end {
            inline_output_end_affinity(&self.blocks.last()?.children)
        } else if !self.inserted_blocks.is_empty() {
            Affinity::Upstream
        } else {
            inline_output_end_affinity(&self.blocks.first()?.children)
        };
        let mut output = output_plan_for_units(
            nodes,
            caret_relation,
            Affinity::Downstream,
            if self.join_start {
                Affinity::Upstream
            } else {
                Affinity::Downstream
            },
            head_affinity,
        )?;
        if self.join_start
            && self.inserted_blocks.is_empty()
            && !self.merge_end
            && self
                .blocks
                .first()
                .is_some_and(|block| paragraph_ends_with_page_break(block))
        {
            output.head.relation = SliceOutputRelation::AfterTerminalPageBreak;
        }
        Some(output)
    }
}

fn inline_output_end_affinity(fragments: &[Fragment]) -> Affinity {
    fragments
        .iter()
        .rev()
        .find_map(|fragment| match &fragment.node {
            PlainNode::Text(text) if !text.text.is_empty() => Some(Affinity::Upstream),
            PlainNode::HardBreak(_) | PlainNode::Tab(_) => Some(Affinity::Downstream),
            PlainNode::PageBreak(_) => Some(Affinity::Upstream),
            _ => None,
        })
        .unwrap_or(Affinity::Downstream)
}

pub(crate) fn plan_textblock_splice_target(
    target: &SliceInsertionTarget,
    slice: &Slice,
) -> Option<TextblockSplicePlan> {
    let textblock = target.textblock()?;
    let container = target.parent(textblock.id)?;
    let textblock_index = textblock.index?;
    let child_count = textblock.child_types.len();
    if target.position.offset > child_count {
        return None;
    }

    let blocks: Vec<&Fragment> = slice.content.iter().collect();
    if blocks.is_empty() {
        return None;
    }

    let textblock_type = textblock.node_type;
    let incompatible_textblock = |fragment: &&Fragment| {
        let source_type = fragment.node.as_type();
        Schema::node_spec(source_type).is_textblock() && source_type != textblock_type
    };
    if blocks.first().is_some_and(incompatible_textblock)
        || blocks.last().is_some_and(incompatible_textblock)
    {
        return None;
    }

    let has_left = target.position.offset > 0;
    let has_right = target.position.offset < child_count;
    if has_left && has_right && !can_split_textblock_for_structural_insert(target, textblock.id) {
        return None;
    }

    let (join_start, join_end) = textblock_edge_joins(
        textblock_type,
        &textblock.child_types,
        target.position.offset,
        slice,
    );
    let merge_destinations = join_start && join_end && blocks.len() == 1;
    let unjoined_start = usize::from(join_start);
    let unjoined_end = if join_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };

    let mut replacement = Vec::new();
    if has_left {
        replacement.push(textblock_type);
    }
    replacement.extend(
        blocks
            .iter()
            .take(unjoined_end)
            .skip(unjoined_start)
            .map(|fragment| fragment.node.as_type()),
    );
    if has_right && !merge_destinations {
        replacement.push(textblock_type);
    }

    let mut final_types = container.child_types.clone();
    final_types.splice(textblock_index..=textblock_index, replacement);
    let placement = content_placement(container.node_type, &final_types);
    if placement.first_residue.is_some() {
        return None;
    }
    let completion = placement.completion_insertions?;
    let mut inserted_blocks = blocks
        .iter()
        .take(unjoined_end)
        .skip(unjoined_start)
        .map(|fragment| (*fragment).clone())
        .collect::<Vec<_>>();
    let mut completed_len = final_types.len();
    for insertion in completion {
        // Root's trailing editable paragraph is derived projection state. Other
        // required children belong to the accepted authored replacement and
        // must be inserted by this plan before projection observes the result.
        let projection_owned_trailing_paragraph = container.node_type == NodeType::Root
            && insertion.node_type == NodeType::Paragraph
            && insertion.index == completed_len;
        completed_len += 1;
        if projection_owned_trailing_paragraph {
            continue;
        }

        let relative = insertion.index.checked_sub(textblock_index)?;
        let inserted_index = relative.checked_sub(usize::from(has_left))?;
        // The original container was valid, so a new required child must fall
        // inside the one textblock replacement. A completion outside that span
        // would alter an unrelated destination survivor and is not this plan's
        // content to author.
        if inserted_index > inserted_blocks.len() {
            return None;
        }
        inserted_blocks.insert(
            inserted_index,
            subtree_to_fragment(minimal_subtree(insertion.node_type)),
        );
    }

    let merge_end = join_end && !merge_destinations;
    let join_start_emits = join_start
        && (blocks[0].children.iter().any(is_insertable_inline_fragment)
            || blocks[0]
                .children
                .last()
                .is_some_and(|child| child.node.as_type() == NodeType::PageBreak));
    let merge_end_emits = merge_end
        && blocks
            .last()
            .is_some_and(|block| block.children.iter().any(is_insertable_inline_fragment));
    let split_emits = has_left && has_right && !merge_destinations;
    let emits_change =
        !inserted_blocks.is_empty() || split_emits || join_start_emits || merge_end_emits;
    if !emits_change {
        return None;
    }

    let unjoined_ends_with_page_break = inserted_blocks
        .last()
        .is_some_and(|fragment| paragraph_ends_with_page_break(fragment));
    let final_caret_at_right_boundary =
        split_emits && inserted_blocks.is_empty() && !join_start_emits && !merge_end_emits;
    let insert_at = textblock_index + usize::from(has_left);
    let left = if has_left {
        Some(ExistingBlock {
            id: Some(textblock.id),
            node_type: textblock_type,
        })
    } else {
        textblock_index.checked_sub(1).and_then(|index| {
            Some(ExistingBlock {
                id: container.child_ids.get(index).copied().flatten(),
                node_type: *container.child_types.get(index)?,
            })
        })
    };
    let right = if has_right && !merge_destinations {
        Some(ExistingBlock {
            id: None,
            node_type: textblock_type,
        })
    } else {
        let index = textblock_index + 1;
        container
            .child_types
            .get(index)
            .map(|node_type| ExistingBlock {
                id: container.child_ids.get(index).copied().flatten(),
                node_type: *node_type,
            })
    };
    let list_merges = plan_adjacent_list_merges(left, &inserted_blocks, right)?;

    Some(TextblockSplicePlan {
        blocks: slice.content.clone(),
        inserted_blocks,
        textblock_type,
        container_type: container.node_type,
        textblock_index,
        child_count,
        split_index: target.position.offset,
        has_left,
        has_right,
        join_start,
        merge_destinations,
        merge_end,
        unjoined_ends_with_page_break,
        final_caret_at_right_boundary,
        insert_at,
        list_merges,
    })
}

fn insert_blocks_in_textblock(
    tr: &mut Transaction,
    plan: &TextblockSplicePlan,
    mode: &InlineMode,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    // In the projected model the caret already addresses the textblock + child
    // offset, so no inner text-node split is needed.
    let textblock_id = head.node;
    if head.offset != plan.split_index {
        return Err(CommandError::Corrupted(
            "textblock splice position changed after planning".into(),
        ));
    }
    let split_index_in_textblock = plan.split_index;

    let container_id = {
        let view = tr.state().view();
        let tb = view
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = tb.parent().ok_or(CommandError::NoParent(textblock_id))?;
        let textblock_index = tb
            .index()
            .ok_or_else(|| CommandError::orphan_child(textblock_id, parent.id()))?;
        let child_count = tb.children().count();
        if tb.node_type() != plan.textblock_type
            || parent.node_type() != plan.container_type
            || textblock_index != plan.textblock_index
            || child_count != plan.child_count
        {
            return Err(CommandError::Corrupted(
                "textblock splice destination changed after planning".into(),
            ));
        }
        parent.id()
    };

    let blocks: Vec<&Fragment> = plan.blocks.iter().collect();
    let unjoined_count = plan.inserted_blocks.len();

    let left_id = plan.has_left.then_some(textblock_id);
    let right_id = if plan.has_left && plan.has_right {
        tr.split_node(textblock_id, split_index_in_textblock)?;
        Some(
            block_child_id(tr, container_id, plan.textblock_index + 1)
                .ok_or_else(|| CommandError::Corrupted("split produced no right half".into()))?,
        )
    } else if plan.has_right {
        Some(textblock_id)
    } else {
        None
    };
    if !plan.has_left && !plan.has_right {
        tr.remove_subtree(textblock_id)?;
    }

    let mut last_caret: Option<Position> = None;
    let mut inserted_range = InsertedRange::default();
    let mut terminal_page_break_start: Option<Position> = None;

    if plan.join_start {
        let left_id = left_id.expect("merge start requires left destination content");
        let first = blocks[0];
        let inline = first.children.to_vec();
        let insertable_inline = inline
            .last()
            .is_some_and(|fragment| fragment.node.as_type() == NodeType::PageBreak)
            .then(|| &inline[..inline.len() - 1])
            .unwrap_or(&inline);
        tr.set_selection(Some(Selection::collapsed(position_at_end_of_block(
            tr, left_id,
        )?)))?;
        let start = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        let inserted = insert_inline_fragments(tr, insertable_inline, mode)?;
        let inserted_page_break = insert_terminal_page_break_from_edge(tr, left_id, &inline)?;
        let end = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        if start.offset < end.offset {
            inserted_range
                .units
                .extend(output_units_between(tr, start, end)?);
        }
        if inserted_page_break {
            let page_break = tr
                .view()
                .node(left_id)
                .and_then(|paragraph| paragraph.last_child())
                .and_then(|child| match child {
                    ChildView::Leaf(leaf) if leaf.node_type() == NodeType::PageBreak => {
                        Some(leaf.dot())
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    CommandError::Corrupted(
                        "planned terminal PageBreak output was not authored".into(),
                    )
                })?;
            inserted_range.units.push(page_break);
            terminal_page_break_start = Some(start);
            last_caret = Some(end);
        } else if inserted {
            inserted_range.include_position_range(start, end);
            last_caret = Some(end);
        }
    }

    if plan.merge_destinations {
        let left_id = left_id.expect("destination merge requires left content");
        tr.merge_node(left_id)?;
        last_caret = tr.selection().map(|s| s.head);
    }

    let mut inserted_blocks = Vec::with_capacity(unjoined_count);
    for fragment in &plan.inserted_blocks {
        let block_index = sequential_insert_index(
            tr,
            container_id,
            plan.insert_at,
            inserted_blocks.last().copied(),
        );
        if let Some(inserted_id) =
            tr.insert_subtree(container_id, block_index, fragment.clone().into_subtree())?
        {
            inserted_blocks.push(inserted_id);
            inserted_range.include_block(inserted_id);
            if let Some(paint) = mode.plain_paint() {
                paint_block_uniformly(tr, inserted_id, paint)?;
            }
            // Block-level atoms have no inner caret. Their planned end slot is
            // the boundary immediately after the authored atom.
            last_caret = Some(position_at_end_of_block(tr, inserted_id).or_else(|_| {
                resolve_inserted_range_endpoint(tr, InsertedRangeEndpoint::AfterBlock(inserted_id))
                    .ok_or(CommandError::NodeNotFound(inserted_id))
            })?);
        }
    }

    if plan.merge_end {
        let right_id = right_id.expect("merge end requires right destination content");
        let last = blocks.last().unwrap();
        let inline = last.children.to_vec();
        tr.set_selection(Some(Selection::collapsed(position_at_start_of_block(
            tr, right_id,
        )?)))?;
        let start = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        let inserted = insert_inline_fragments(tr, &inline, mode)?;
        let end = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        if inserted {
            inserted_range.include_position_range(start, end);
            inserted_range
                .units
                .extend(output_units_between(tr, start, end)?);
        }
        last_caret = Some(end);
        if let Some(paint) = mode.plain_paint() {
            tr.replace_carry(right_id, carry_from_paint(paint))?;
        }
    }

    let steps = {
        let view = tr.state().view();
        view.node(container_id)
            .map(|container| fulfill(&container))
            .unwrap_or_default()
    };
    tr.apply_steps(steps)?;

    if !plan.merge_end && plan.unjoined_ends_with_page_break {
        let following_id = block_child_id(tr, container_id, plan.insert_at + unjoined_count)
            .ok_or_else(|| CommandError::Corrupted("PageBreak has no following block".into()))?;
        last_caret = Some(position_at_start_of_block(tr, following_id)?);
    }

    if let Some(start) = terminal_page_break_start {
        if inserted_range.end.is_some() {
            inserted_range.prepend_position(start);
        } else {
            let following_id =
                block_child_id(tr, container_id, plan.insert_at).ok_or_else(|| {
                    CommandError::Corrupted("PageBreak has no following block".into())
                })?;
            let end = position_at_start_of_block(tr, following_id)?;
            inserted_range.include_position_range(start, end);
            last_caret = Some(end);
        }
    }

    let mut final_pos = match (last_caret, plan.final_caret_at_right_boundary) {
        (_, true) => position_at_start_of_block(
            tr,
            right_id.ok_or_else(|| {
                CommandError::Corrupted(
                    "planned textblock split lost its right caret boundary".into(),
                )
            })?,
        )?,
        (Some(position), false) => position,
        (None, false) => {
            return Err(CommandError::Corrupted(
                "textblock Slice plan produced a different final caret".into(),
            ));
        }
    };
    final_pos.affinity = Affinity::Downstream;
    let explicit_inserted_selection = inserted_range.selection(tr)?;
    let split_boundary_selection =
        if plan.has_left && plan.has_right && explicit_inserted_selection.is_none() {
            match (left_id, right_id) {
                (Some(left_id), Some(right_id)) => Some(Selection::new(
                    position_at_end_of_block(tr, left_id)?,
                    position_at_start_of_block(tr, right_id)?,
                )),
                _ => {
                    return Err(CommandError::Corrupted(
                        "planned textblock split lost a surviving boundary".into(),
                    ));
                }
            }
        } else {
            None
        };
    let inserted_selection = explicit_inserted_selection
        .or(split_boundary_selection)
        .ok_or_else(|| {
            CommandError::Corrupted("textblock Slice plan produced no inserted selection".into())
        })?;
    let output_units = expand_planned_list_output_units(
        tr,
        &inserted_range.units,
        &inserted_blocks,
        &plan.list_merges,
    )?;
    tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    let inserted_selection = apply_planned_list_merges(
        tr,
        container_id,
        plan.insert_at,
        &inserted_blocks,
        &plan.list_merges,
        inserted_selection,
    )?;

    Ok(Some(SliceInsertionExecution {
        inserted: inserted_selection,
        units: output_units,
        split_left: left_id,
        split_right: right_id,
    }))
}

fn position_at_end_of_block(tr: &Transaction, block_id: Dot) -> Result<Position, CommandError> {
    let view = tr.state().view();
    let block = view
        .node(block_id)
        .ok_or(CommandError::NodeNotFound(block_id))?;
    Ok(Position {
        node: block_id,
        offset: block.children().count(),
        affinity: Affinity::Upstream,
    })
}

fn position_at_start_of_block(tr: &Transaction, block_id: Dot) -> Result<Position, CommandError> {
    let view = tr.state().view();
    if let Some(block) = view.node(block_id) {
        return first_cursor_position(&block)
            .ok_or_else(|| CommandError::Corrupted("block has no position at its start".into()));
    }
    let (parent, index) =
        block_parent_and_index(&view, block_id).ok_or(CommandError::NodeNotFound(block_id))?;
    Ok(Position::new(parent, index))
}

pub(crate) fn insert_blocks_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    blocks: Vec<Fragment>,
    list_merges: Vec<PlannedListMerge>,
) -> Result<Option<SliceInsertionExecution>, CommandError> {
    let container_id = position.node;
    let base_index = position.offset;
    let block_count = blocks.len();
    let terminal_page_break = blocks.last().is_some_and(paragraph_ends_with_page_break);
    let mut inserted: Vec<Dot> = Vec::with_capacity(block_count);
    tr.batch(|tr| {
        // Normalization between the sequential inserts can synthesize scaffold
        // children that shift projected indices, so each follow-up insert
        // re-derives its slot from the sibling inserted just before it instead
        // of trusting `base_index + offset` arithmetic — that arithmetic is
        // what used to anchor an insert on a synthetic scaffold (no CRDT
        // identity) and fail with NodeNotFound.
        for block in blocks.iter() {
            let subtree = block.clone().into_subtree();
            let index =
                sequential_insert_index(tr, container_id, base_index, inserted.last().copied());
            let new_id = tr
                .insert_subtree(container_id, index, subtree)?
                .ok_or_else(|| {
                    CommandError::Corrupted(
                        "planned block Slice insertion produced no authored node".into(),
                    )
                })?;
            inserted.push(new_id);
        }
        let steps = {
            let view = tr.state().view();
            view.node(container_id)
                .map(|container| fulfill(&container))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        Ok::<(), CommandError>(())
    })?;

    if inserted.len() != block_count {
        return Err(CommandError::Corrupted(
            "planned block Slice insertion produced an incomplete output forest".into(),
        ));
    }
    let first_inserted_index = inserted
        .first()
        .copied()
        .and_then(|id| {
            let view = tr.state().view();
            let (parent, index) = block_parent_and_index(&view, id)?;
            (parent == container_id).then_some(index)
        })
        .ok_or_else(|| {
            CommandError::Corrupted("planned block Slice start output no longer resolves".into())
        })?;
    let last_inserted_index = inserted
        .last()
        .copied()
        .and_then(|id| {
            let view = tr.state().view();
            let (parent, index) = block_parent_and_index(&view, id)?;
            (parent == container_id).then_some(index)
        })
        .ok_or_else(|| {
            CommandError::Corrupted("planned block Slice end output no longer resolves".into())
        })?;
    let span = last_inserted_index
        .checked_sub(first_inserted_index)
        .and_then(|distance| distance.checked_add(1))
        .filter(|span| *span == inserted.len())
        .ok_or_else(|| {
            CommandError::Corrupted(
                "planned block Slice outputs are no longer one contiguous range".into(),
            )
        })?;
    let start_index = first_inserted_index;

    let mut final_pos = if terminal_page_break {
        let following =
            block_child_id(tr, container_id, last_inserted_index + 1).ok_or_else(|| {
                CommandError::Corrupted("planned PageBreak output has no following block".into())
            })?;
        position_at_start_of_block(tr, following)?
    } else {
        let last = *inserted
            .last()
            .ok_or_else(|| CommandError::Corrupted("planned block Slice output is empty".into()))?;
        position_at_end_of_block(tr, last).or_else(|_| {
            resolve_inserted_range_endpoint(tr, InsertedRangeEndpoint::AfterBlock(last))
                .ok_or(CommandError::NodeNotFound(last))
        })?
    };
    final_pos.affinity = Affinity::Downstream;
    tr.set_selection(Some(Selection::collapsed(final_pos)))?;

    let inserted_selection = selection_over_inserted_blocks(container_id, start_index, span);
    let output_units = expand_planned_list_output_units(tr, &inserted, &inserted, &list_merges)?;
    let inserted_selection = apply_planned_list_merges(
        tr,
        container_id,
        start_index,
        &inserted,
        &list_merges,
        inserted_selection,
    )?;
    Ok(Some(SliceInsertionExecution {
        inserted: inserted_selection,
        units: output_units,
        split_left: None,
        split_right: None,
    }))
}

fn apply_planned_list_merges(
    tr: &mut Transaction,
    container_id: Dot,
    start_index: usize,
    inserted: &[Dot],
    merges: &[PlannedListMerge],
    mut inserted_selection: Selection,
) -> Result<Selection, CommandError> {
    let left = start_index
        .checked_sub(1)
        .and_then(|index| block_child_id(tr, container_id, index));
    let right = block_child_id(tr, container_id, start_index + inserted.len());
    let resolve_member = |member: PlannedListMember| match member {
        PlannedListMember::Left(_) => left,
        PlannedListMember::Inserted { index, .. } => inserted.get(index).copied(),
        PlannedListMember::Right(_) => right,
    };

    for planned in merges {
        let pair = {
            let view = tr.state().view();
            let earlier_id = resolve_member(planned.earlier).ok_or_else(|| {
                CommandError::Corrupted("planned earlier list output is unavailable".into())
            })?;
            let later_id = resolve_member(planned.later).ok_or_else(|| {
                CommandError::Corrupted("planned later list output is unavailable".into())
            })?;
            let earlier = view
                .node(earlier_id)
                .ok_or(CommandError::NodeNotFound(earlier_id))?;
            let later = view
                .node(later_id)
                .ok_or(CommandError::NodeNotFound(later_id))?;
            if earlier.node_type() != planned.earlier.node_type()
                || later.node_type() != planned.later.node_type()
                || earlier.parent().map(|parent| parent.id()) != Some(container_id)
                || later.parent().map(|parent| parent.id()) != Some(container_id)
                || earlier.index().and_then(|index| index.checked_add(1)) != later.index()
            {
                return Err(CommandError::Corrupted(
                    "planned adjacent-list merge no longer matches its bound outputs".into(),
                ));
            }
            (earlier_id, later_id)
        };
        let selection = tr.selection();
        let stable = selection.map(|selection| StableSelection::capture(&selection, &tr.view()));
        let inserted_stable = StableSelection::capture(&inserted_selection, &tr.view());
        let merged_lists = merge_adjacent_list_pair(tr, pair.0, pair.1)?;
        if let Some((selection, stable)) = selection.zip(stable) {
            restore_selection_after_adjacent_list_merge(tr, selection, stable, &merged_lists)?;
        }
        inserted_selection = resolve_selection_after_adjacent_list_merge(
            tr,
            inserted_selection,
            inserted_stable,
            &merged_lists,
        )?;
    }
    Ok(inserted_selection)
}

fn expand_planned_list_output_units(
    tr: &Transaction,
    units: &[Dot],
    inserted: &[Dot],
    merges: &[PlannedListMerge],
) -> Result<Vec<Dot>, CommandError> {
    let mut merged_inserted = vec![false; inserted.len()];
    for planned in merges {
        for member in [planned.earlier, planned.later] {
            if let PlannedListMember::Inserted { index, .. } = member {
                let merged = merged_inserted.get_mut(index).ok_or_else(|| {
                    CommandError::Corrupted(
                        "planned list merge referenced an unavailable inserted block".into(),
                    )
                })?;
                *merged = true;
            }
        }
    }

    let view = tr.view();
    let mut output = Vec::new();
    for unit in units {
        let Some(index) = inserted.iter().position(|inserted| inserted == unit) else {
            output.push(*unit);
            continue;
        };
        if !merged_inserted[index] {
            output.push(*unit);
            continue;
        }
        let list = view.node(*unit).ok_or(CommandError::NodeNotFound(*unit))?;
        if !is_list_type(list.node_type()) {
            return Err(CommandError::Corrupted(
                "planned list merge output is not a list".into(),
            ));
        }
        let items = list
            .child_blocks()
            .map(|item| item.id())
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err(CommandError::Corrupted(
                "planned list merge output contains no list item".into(),
            ));
        }
        output.extend(items);
    }
    Ok(output)
}

fn sequential_insert_index(
    tr: &Transaction,
    container_id: Dot,
    base_index: usize,
    previous: Option<Dot>,
) -> usize {
    let view = tr.state().view();
    previous
        .and_then(|previous| block_parent_and_index(&view, previous))
        .and_then(|(parent, index)| (parent == container_id).then_some(index + 1))
        .unwrap_or(base_index)
}

pub(crate) fn block_boundary_fragments(
    slice: &Slice,
    container_type: NodeType,
) -> Option<Vec<Fragment>> {
    let top_level = top_level_fragments(slice);
    if fragments_are_inline(&top_level)
        && Schema::node_spec(container_type)
            .content
            .matches(NodeType::Paragraph)
    {
        return Some(vec![Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: top_level.into_iter().cloned().collect(),
        }]);
    }

    open_fragments_for_parent(top_level, slice.open_start, slice.open_end, container_type)
        .map(|(fragments, _, _)| fragments.into_iter().cloned().collect())
}

fn open_fragments_for_parent(
    mut candidates: Vec<&Fragment>,
    mut open_start: u32,
    mut open_end: u32,
    parent_type: NodeType,
) -> Option<(Vec<&Fragment>, u32, u32)> {
    let content = &Schema::node_spec(parent_type).content;
    loop {
        if candidates.is_empty() {
            return None;
        }
        if fragments_fit_parent(parent_type, &candidates) {
            return Some((candidates, open_start, open_end));
        }

        let first_rejected = !content.matches(candidates.first()?.node.as_type());
        let last_rejected = !content.matches(candidates.last()?.node.as_type());
        if !first_rejected && !last_rejected {
            return None;
        }

        if candidates.len() == 1 {
            let can_open_start = first_rejected && open_start > 0;
            let can_open_end = last_rejected && open_end > 0;
            if !can_open_start && !can_open_end {
                return None;
            }
            let only = candidates.pop()?;
            if only.children.is_empty() {
                return None;
            }
            candidates.extend(&only.children);
            if can_open_start {
                open_start -= 1;
            }
            if can_open_end {
                open_end -= 1;
            }
            continue;
        }

        if first_rejected {
            if open_start == 0 {
                return None;
            }
            let first = candidates.remove(0);
            if first.children.is_empty() {
                return None;
            }
            candidates.splice(0..0, &first.children);
            open_start -= 1;
        }
        if last_rejected {
            if open_end == 0 {
                return None;
            }
            let last = candidates.pop()?;
            if last.children.is_empty() {
                return None;
            }
            candidates.extend(&last.children);
            open_end -= 1;
        }
    }
}

fn selection_over_inserted_blocks(
    container_id: Dot,
    start_index: usize,
    block_count: usize,
) -> Selection {
    Selection::new(
        Position {
            node: container_id,
            offset: start_index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: container_id,
            offset: start_index + block_count,
            affinity: Affinity::Upstream,
        },
    )
}

fn resolve_inserted_range_endpoint(
    tr: &Transaction,
    endpoint: InsertedRangeEndpoint,
) -> Option<Position> {
    match endpoint {
        InsertedRangeEndpoint::Position(position) => Some(position),
        InsertedRangeEndpoint::BeforeBlock(ref id) | InsertedRangeEndpoint::AfterBlock(ref id) => {
            let view = tr.state().view();
            let (parent_id, index) = block_parent_and_index(&view, *id)?;
            let (offset, affinity) = match endpoint {
                InsertedRangeEndpoint::BeforeBlock(_) => (index, Affinity::Downstream),
                InsertedRangeEndpoint::AfterBlock(_) => (index + 1, Affinity::Upstream),
                InsertedRangeEndpoint::Position(_) => unreachable!(),
            };
            Some(Position {
                node: parent_id,
                offset,
                affinity,
            })
        }
    }
}

/// Parent id and full child-slot index of `id`, which may be a real block or a
/// block-level atom leaf (which projects as a `Child::Leaf`, not a node).
pub(crate) fn block_parent_and_index(view: &DocView, id: Dot) -> Option<(Dot, usize)> {
    if let Some(node) = view.node(id) {
        let parent = node.parent()?;
        let index = node.index()?;
        return Some((parent.id(), index));
    }
    if let Some(op) = id.as_op_dot() {
        let dot = op.dot();
        let leaf = view.leaf(dot)?;
        let parent = leaf.parent()?;
        let index = parent.children().position(|c| match c {
            ChildView::Leaf(l) => l.dot() == dot,
            ChildView::Block(_) => false,
        })?;
        return Some((parent.id(), index));
    }
    None
}

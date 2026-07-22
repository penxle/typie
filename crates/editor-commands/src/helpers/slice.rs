use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{
    ChildView, ContentExpr, DocView, Fragment, Modifier, NodeType, NodeView, PlainNode,
    PlainParagraphNode, Schema, context_allows, wrap_chain,
};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use super::{
    apply_inline_modifiers, child_node_type, consume_pending_modifiers, find_ancestor_textblock,
    insert_hard_break_at_caret, insert_tab_at_caret, insert_text_at_caret,
    resolve_effective_modifiers,
};
use crate::types::SliceProvenance;
use crate::{CommandError, CommandResult};

mod page_break;

pub(crate) use page_break::prepare_page_breaks_for_position;
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

pub(crate) fn position_in_textblock(view: &DocView, position: &Position) -> bool {
    position
        .resolve(view)
        .is_some_and(|resolved| resolved.is_inline_position())
}

pub(crate) fn top_level_fragments(slice: &Slice) -> Vec<&Fragment> {
    slice.content.iter().collect()
}

pub(crate) fn fragments_fit_parent(parent_type: NodeType, fragments: &[&Fragment]) -> bool {
    let content = &Schema::node_spec(parent_type).content;
    fragments
        .iter()
        .all(|fragment| content.matches(fragment.node.as_type()))
}

pub(crate) fn fragments_are_inline(fragments: &[&Fragment]) -> bool {
    !fragments.is_empty()
        && fragments
            .iter()
            .all(|fragment| Schema::node_spec(fragment.node.as_type()).inline)
}

fn can_split_textblock_for_structural_insert(view: &DocView, textblock_id: Dot) -> bool {
    let Some(textblock) = view.node(textblock_id) else {
        return false;
    };
    let Some(parent) = textblock.parent() else {
        return false;
    };
    let Some(index) = textblock.index() else {
        return false;
    };

    let mut child_types: Vec<NodeType> = parent.children().map(|c| child_node_type(&c)).collect();
    child_types.insert(index + 1, textblock.node_type());
    parent.spec().content.matches_sequence(&child_types)
}

pub(crate) fn fit_slice_for_textblock_parent(
    view: &DocView,
    position: &Position,
    slice: &Slice,
) -> Option<Slice> {
    let textblock_id = find_ancestor_textblock(view, position.node)?;
    let parent_type = view.node(textblock_id)?.parent()?.node_type();
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

pub(crate) fn open_inline_content_for_textblock_insert<'a>(
    view: &DocView,
    position: &Position,
    slice: &'a Slice,
) -> Option<Vec<&'a Fragment>> {
    let textblock_id = find_ancestor_textblock(view, position.node)?;
    let textblock = view.node(textblock_id)?;
    let textblock_type = textblock.node_type();

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
) -> Result<Option<Selection>, CommandError> {
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
    Ok(Some(Selection::new(start, end)))
}

pub(crate) fn insert_blocks_in_textblock_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
    mode: &InlineMode,
) -> Result<Option<Selection>, CommandError> {
    tr.set_selection(Some(Selection::collapsed(position)))?;
    insert_blocks_in_textblock(tr, slice, mode)
}

fn insert_inline_fragments(
    tr: &mut Transaction,
    fragments: &[Fragment],
    mode: &InlineMode,
) -> CommandResult {
    let mut any_change = false;
    for f in fragments {
        match &f.node {
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
            _ => {}
        }
    }
    Ok(any_change)
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

#[derive(Default)]
struct InsertedRange {
    start: Option<InsertedRangeEndpoint>,
    end: Option<InsertedRangeEndpoint>,
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
    }

    fn selection(&self, tr: &Transaction) -> Option<Selection> {
        let start = resolve_inserted_range_endpoint(tr, self.start.clone()?)?;
        let end = resolve_inserted_range_endpoint(tr, self.end.clone()?)?;
        Some(Selection::new(start, end))
    }
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

fn textblock_edge_joins(textblock: &NodeView, offset: usize, slice: &Slice) -> (bool, bool) {
    let destination_type = textblock.node_type();
    let destination_children: Vec<NodeType> = textblock
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    if offset > destination_children.len() {
        return (false, false);
    }
    let content = &textblock.spec().content;

    let join_start = offset > 0
        && slice.open_start > 0
        && slice.content.first().is_some_and(|source| {
            source.node.as_type() == destination_type
                && content.matches_sequence(
                    &destination_children[..offset]
                        .iter()
                        .copied()
                        .chain(source.children.iter().map(|child| child.node.as_type()))
                        .collect::<Vec<_>>(),
                )
        });
    let mut join_end = offset < destination_children.len()
        && slice.open_end > 0
        && slice.content.last().is_some_and(|source| {
            source.node.as_type() == destination_type
                && content.matches_sequence(
                    &source
                        .children
                        .iter()
                        .map(|child| child.node.as_type())
                        .chain(destination_children[offset..].iter().copied())
                        .collect::<Vec<_>>(),
                )
        });

    if join_start && join_end && slice.content.len() == 1 {
        // Pairwise-valid joins may still form an invalid three-part sequence.
        // Preserve the start join and leave the end in its destination wrapper.
        let source = &slice.content[0];
        join_end = content.matches_sequence(
            &destination_children[..offset]
                .iter()
                .copied()
                .chain(source.children.iter().map(|child| child.node.as_type()))
                .chain(destination_children[offset..].iter().copied())
                .collect::<Vec<_>>(),
        );
    }

    (join_start, join_end)
}

pub(crate) fn can_splice_textblock(view: &DocView, position: &Position, slice: &Slice) -> bool {
    let Some(textblock_id) = find_ancestor_textblock(view, position.node) else {
        return false;
    };
    let Some(textblock) = view.node(textblock_id) else {
        return false;
    };
    let Some(container) = textblock.parent() else {
        return false;
    };
    let Some(textblock_index) = textblock.index() else {
        return false;
    };
    let child_count = textblock.children().count();
    if position.offset > child_count {
        return false;
    }

    let blocks: Vec<&Fragment> = slice.content.iter().collect();
    if blocks.is_empty() {
        return false;
    }

    let textblock_type = textblock.node_type();
    let incompatible_textblock = |fragment: &&Fragment| {
        let source_type = fragment.node.as_type();
        Schema::node_spec(source_type).is_textblock() && source_type != textblock_type
    };
    if blocks.first().is_some_and(incompatible_textblock)
        || blocks.last().is_some_and(incompatible_textblock)
    {
        return false;
    }

    let has_left = position.offset > 0;
    let has_right = position.offset < child_count;
    if has_left && has_right && !can_split_textblock_for_structural_insert(view, textblock_id) {
        return false;
    }

    let (join_start, join_end) = textblock_edge_joins(&textblock, position.offset, slice);
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

    let mut final_types: Vec<NodeType> = container
        .children()
        .map(|child| child_node_type(&child))
        .collect();
    final_types.splice(textblock_index..=textblock_index, replacement);
    container
        .spec()
        .content
        .completion_insertions(&final_types)
        .is_some()
}

/// `can_splice_textblock`이 통과시킨 splice가 실제로 관측 가능한 op를 방출하는지 —
/// `insert_blocks_in_textblock`이 `Ok(Some)`을 돌리는 조건의 정확한 미러. 구조 유효성만
/// 보는 `can_splice_textblock`은 콘텐츠 없는 슬라이스(예: 빈 문단)가 edge join으로 소진돼
/// 아무 op도 내지 않는(`Ok(None)`) 경우를 통과시키므로, resolve가 그런 슬라이스에
/// SpliceBlocks Plan을 내지 않도록 이 술어로 걸러 `resolve_slice_insertion`의 계약
/// (`Some(plan)` ⇒ 삽입 op 방출)을 유지한다. 모든 입력은 (view, position, slice)의 순수
/// 함수다.
pub(crate) fn splice_emits_change(view: &DocView, position: &Position, slice: &Slice) -> bool {
    let Some(textblock_id) = find_ancestor_textblock(view, position.node) else {
        return false;
    };
    let Some(textblock) = view.node(textblock_id) else {
        return false;
    };
    let offset = position.offset;
    let child_count = textblock.children().count();
    let has_left = offset > 0;
    let has_right = offset < child_count;

    let blocks: Vec<&Fragment> = slice.content.iter().collect();
    if blocks.is_empty() {
        return false;
    }
    let (join_start, join_end) = textblock_edge_joins(&textblock, offset, slice);
    let merge_destinations = join_start && join_end && blocks.len() == 1;
    let unjoined_start = usize::from(join_start);
    let unjoined_end = if join_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };

    // 소진되지 않은 블록은 그대로 insert_subtree 된다. 빈 텍스트블록 제거 케이스도 포함:
    // !has_left && !has_right ⇒ join 불가 ⇒ unjoined_count == blocks.len() > 0.
    if unjoined_end > unjoined_start {
        return true;
    }
    // 인접 위치의 미병합 split은 두 반쪽을 남긴다(구조 변경).
    if has_left && has_right && !merge_destinations {
        return true;
    }
    // start join: blocks[0]의 인라인 병합, 또는 말미 페이지브레이크 삽입.
    // 페이지브레이크 절은 실행의 into_root_paragraph 성공 여부를 직접 보지 않는다 —
    // 상류 prepare_page_breaks_for_position이 비-root 문맥의 말미 페이지브레이크를
    // 이미 제거하므로, 여기 도달한 말미 PageBreak는 root-terminal 허용 문맥이 보장된다.
    if join_start
        && (blocks[0].children.iter().any(is_insertable_inline_fragment)
            || blocks[0]
                .children
                .last()
                .is_some_and(|child| child.node.as_type() == NodeType::PageBreak))
    {
        return true;
    }
    // end join: blocks.last()의 인라인 병합.
    let merge_end = join_end && !merge_destinations;
    if merge_end
        && blocks
            .last()
            .is_some_and(|block| block.children.iter().any(is_insertable_inline_fragment))
    {
        return true;
    }
    false
}

fn insert_blocks_in_textblock(
    tr: &mut Transaction,
    slice: &Slice,
    mode: &InlineMode,
) -> Result<Option<Selection>, CommandError> {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    // In the projected model the caret already addresses the textblock + child
    // offset, so no inner text-node split is needed.
    let textblock_id = head.node;
    let split_index_in_textblock = head.offset;

    let (container_id, textblock_index, child_count, join_start, join_end) = {
        let view = tr.state().view();
        let tb = view
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = tb.parent().ok_or(CommandError::NoParent(textblock_id))?;
        let textblock_index = tb
            .index()
            .ok_or_else(|| CommandError::orphan_child(textblock_id, parent.id()))?;
        let child_count = tb.children().count();
        let (join_start, join_end) = textblock_edge_joins(&tb, split_index_in_textblock, slice);
        (
            parent.id(),
            textblock_index,
            child_count,
            join_start,
            join_end,
        )
    };

    let blocks: Vec<&Fragment> = slice.content.iter().collect();
    let has_left = split_index_in_textblock > 0;
    let has_right = split_index_in_textblock < child_count;
    let merge_destinations = join_start && join_end && blocks.len() == 1;
    let unjoined_start = usize::from(join_start);
    let unjoined_end = if join_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };
    let merge_end = join_end && !merge_destinations && unjoined_end >= unjoined_start;
    let unjoined_count = unjoined_end.saturating_sub(unjoined_start);
    let unjoined_ends_with_page_break = unjoined_count > 0
        && blocks
            .get(unjoined_end - 1)
            .is_some_and(|fragment| paragraph_ends_with_page_break(fragment));
    let insert_at = textblock_index + usize::from(has_left);

    let left_id = has_left.then_some(textblock_id);
    let right_id = if has_left && has_right {
        tr.split_node(textblock_id, split_index_in_textblock)?;
        Some(
            block_child_id(tr, container_id, textblock_index + 1)
                .ok_or_else(|| CommandError::Corrupted("split produced no right half".into()))?,
        )
    } else if has_right {
        Some(textblock_id)
    } else {
        None
    };
    if !has_left && !has_right {
        tr.remove_subtree(textblock_id)?;
    }

    let mut last_caret: Option<Position> = None;
    let mut inserted_range = InsertedRange::default();
    let mut terminal_page_break_start: Option<Position> = None;

    if join_start {
        let left_id = left_id.expect("merge start requires left destination content");
        let first = blocks[0];
        let inline = first.children.to_vec();
        tr.set_selection(Some(Selection::collapsed(position_at_end_of_block(
            tr, left_id,
        )?)))?;
        let start = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        let inserted = insert_inline_fragments(tr, &inline, mode)?;
        let inserted_page_break = insert_terminal_page_break_from_edge(tr, left_id, &inline)?;
        let end = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        if inserted_page_break {
            terminal_page_break_start = Some(start);
            last_caret = Some(end);
        } else if inserted {
            inserted_range.include_position_range(start, end);
            last_caret = Some(end);
        }
    }

    if merge_destinations {
        let left_id = left_id.expect("destination merge requires left content");
        tr.merge_node(left_id)?;
        last_caret = tr.selection().map(|s| s.head);
    }

    for (offset, fragment) in blocks
        .iter()
        .take(unjoined_end)
        .skip(unjoined_start)
        .copied()
        .enumerate()
    {
        let block_index = insert_at + offset;
        tr.insert_subtree(container_id, block_index, fragment.clone().into_subtree())?;
        if let Some(inserted_id) = block_child_id(tr, container_id, block_index) {
            inserted_range.include_block(inserted_id);
            if let Some(paint) = mode.plain_paint() {
                paint_block_uniformly(tr, inserted_id, paint)?;
            }
            // Block-level atoms are selectable as a range but have no inner caret.
            if let Ok(position) = position_at_end_of_block(tr, inserted_id) {
                last_caret = Some(position);
            }
        }
    }

    if merge_end {
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

    if !merge_end && unjoined_ends_with_page_break {
        let following_id = block_child_id(tr, container_id, insert_at + unjoined_count)
            .ok_or_else(|| CommandError::Corrupted("PageBreak has no following block".into()))?;
        last_caret = Some(position_at_start_of_block(tr, following_id)?);
    }

    if let Some(start) = terminal_page_break_start {
        if inserted_range.end.is_some() {
            inserted_range.prepend_position(start);
        } else {
            let following_id = block_child_id(tr, container_id, insert_at).ok_or_else(|| {
                CommandError::Corrupted("PageBreak has no following block".into())
            })?;
            let end = position_at_start_of_block(tr, following_id)?;
            inserted_range.include_position_range(start, end);
            last_caret = Some(end);
        }
    }

    let live_right = right_id.filter(|id| tr.state().view().node(*id).is_some());
    let live_left = left_id.filter(|id| tr.state().view().node(*id).is_some());
    let mut final_pos = if let Some(position) = last_caret {
        position
    } else if let Some(right_id) = live_right {
        position_at_start_of_block(tr, right_id)?
    } else if let Some(left_id) = live_left {
        position_at_end_of_block(tr, left_id)?
    } else {
        Position {
            node: container_id,
            offset: insert_at + unjoined_count,
            affinity: Affinity::Upstream,
        }
    };
    final_pos.affinity = Affinity::Downstream;
    let explicit_inserted_selection = inserted_range.selection(tr);
    let split_boundary_selection = if has_left && has_right && explicit_inserted_selection.is_none()
    {
        match (live_left, live_right) {
            (Some(left_id), Some(right_id)) => Some(Selection::new(
                position_at_end_of_block(tr, left_id)?,
                position_at_start_of_block(tr, right_id)?,
            )),
            _ => None,
        }
    } else {
        None
    };
    let inserted_selection = explicit_inserted_selection.or(split_boundary_selection);
    tr.set_selection(Some(Selection::collapsed(final_pos)))?;

    Ok(inserted_selection)
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
    if view.node(block_id).is_none() {
        return Err(CommandError::NodeNotFound(block_id));
    }
    Ok(Position {
        node: block_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })
}

pub(crate) fn insert_blocks_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    blocks: Vec<Fragment>,
) -> Result<Option<Selection>, CommandError> {
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
        let mut known = {
            let view = tr.state().view();
            real_child_ids(&view, container_id)
        };
        for block in blocks.iter() {
            let subtree = block.clone().into_subtree();
            let index = {
                let view = tr.state().view();
                match inserted.last().and_then(|prev| view.node(*prev)) {
                    Some(prev) => prev.index().map(|i| i + 1),
                    None => None,
                }
                .unwrap_or(base_index)
            };
            tr.insert_subtree(container_id, index, subtree)?;
            let view = tr.state().view();
            if let Some(new_id) = real_child_ids(&view, container_id)
                .into_iter()
                .find(|id| !known.contains(id))
            {
                known.push(new_id);
                inserted.push(new_id);
            }
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

    let last_inserted_index = inserted.last().copied().and_then(|id| {
        let view = tr.state().view();
        view.node(id).and_then(|n| n.index())
    });
    let trailing_synthetic = terminal_page_break
        .then(|| block_child_id(tr, container_id, last_inserted_index? + 1))
        .flatten()
        .filter(|id| id.is_synthetic());
    let final_position = trailing_synthetic
        .and_then(|id| position_at_start_of_block(tr, id).ok())
        .or_else(|| {
            inserted
                .last()
                .and_then(|id| position_at_end_of_block(tr, *id).ok())
        });
    if let Some(mut final_pos) = final_position {
        final_pos.affinity = Affinity::Downstream;
        tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    }

    let first_inserted_index = inserted.first().copied().and_then(|id| {
        let view = tr.state().view();
        view.node(id).and_then(|n| n.index())
    });
    let (start_index, span) = match (first_inserted_index, last_inserted_index) {
        (Some(first), Some(last)) => (first, last - first + 1),
        _ => (base_index, block_count),
    };
    Ok(Some(selection_over_inserted_blocks(
        container_id,
        start_index,
        span,
    )))
}

fn real_child_ids(view: &DocView, container_id: Dot) -> Vec<Dot> {
    view.node(container_id)
        .map(|container| {
            container
                .child_blocks()
                .map(|b| b.id())
                .filter(|id| id.as_op_dot().is_some())
                .collect()
        })
        .unwrap_or_default()
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

fn open_fragments_for_parent<'a>(
    mut candidates: Vec<&'a Fragment>,
    mut open_start: u32,
    mut open_end: u32,
    parent_type: NodeType,
) -> Option<(Vec<&'a Fragment>, u32, u32)> {
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
fn block_parent_and_index(view: &DocView, id: Dot) -> Option<(Dot, usize)> {
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

/// Recursion-depth ceiling and WRAP/SPLIT operation budget for one repair
/// invocation. Clipboard HTML is external input, so — like the projection repair's
/// `RepairCtx` — the pass is bounded: past either limit it stops restructuring and
/// leaves the (partially repaired) fragments for the projection layer to absorb.
/// Both are far above any real paste; they exist only to cap pathological input.
const REPAIR_DEPTH_LIMIT: usize = 128;
const REPAIR_OP_BUDGET: usize = 4096;

/// Restructure each top-level slice fragment's subtree so every node satisfies its
/// schema content model before the insert-shape decision, applying the same
/// WRAP / SPLIT-HOIST algebra the projection repair uses — on the dot-less
/// `Fragment` tree. A misfit child is either wrapped in the minimal scaffold chain
/// (`wrap_chain`) that makes it legal in place, or split out: the container keeps
/// the fitting prefix and the residue is promoted to the container's parent level
/// (a trailing scaffold of the container's own type carries any following
/// children) to be re-examined and re-placed. Ancestors above the top-level
/// fragments are unknown at this point; `Root` is assumed so that Root-anchored
/// contexts (e.g. `PageBreak`) stay legal. One pass, proportional to slice size;
/// nothing is dropped.
pub(crate) fn repair_slice_fragments(fragments: &mut [Fragment]) {
    let mut budget = REPAIR_OP_BUDGET;
    for fragment in fragments.iter_mut() {
        repair_fragment_subtree(fragment, &[NodeType::Root], &mut budget);
    }
}

fn repair_fragment_subtree(fragment: &mut Fragment, ancestors: &[NodeType], budget: &mut usize) {
    let container = fragment.node.as_type();
    let mut path: Vec<NodeType> = ancestors.to_vec();
    path.push(container);
    let hoist = repair_fragment_children(container, &mut fragment.children, &mut path, budget);
    fragment.children.extend(hoist);
}

/// Repair `children` against `container`'s content model (`path` ends with
/// `container`), recursing into each fitting child. Returns the forest to hoist to
/// `container`'s parent level (empty when self-contained). Stops early once the
/// depth ceiling or op budget is reached.
fn repair_fragment_children(
    container: NodeType,
    children: &mut Vec<Fragment>,
    path: &mut Vec<NodeType>,
    budget: &mut usize,
) -> Vec<Fragment> {
    if path.len() > REPAIR_DEPTH_LIMIT {
        return Vec::new();
    }
    let mut i = 0;
    while i < children.len() {
        if *budget == 0 {
            break;
        }
        if fragment_is_misfit(container, children, i, path) {
            match scaffold_chain_for(path, children[i].node.as_type()) {
                Some(chain) => {
                    *budget -= 1;
                    wrap_fragment(children, i, &chain);
                    continue;
                }
                None => {
                    *budget -= 1;
                    return split_hoist_fragment(container, children, i);
                }
            }
        }
        let child = children[i].node.as_type();
        path.push(child);
        let hoist = repair_fragment_children(child, &mut children[i].children, path, budget);
        path.pop();
        if !hoist.is_empty() {
            children.splice(i + 1..i + 1, hoist);
        }
        i += 1;
    }
    Vec::new()
}

fn fragment_is_misfit(
    container: NodeType,
    children: &[Fragment],
    i: usize,
    path: &[NodeType],
) -> bool {
    if container == NodeType::Unknown {
        return false;
    }
    let child = children[i].node.as_type();
    if child != NodeType::Unknown && !context_allows(path, child) {
        return true;
    }
    fragment_content_residue(container, children) == Some(i)
}

/// Index of the first child `container`'s content model cannot admit in sequence
/// order (the first child past the greedily consumable prefix). `Unknown` children
/// are transparent. `None` when the children form a completable prefix — missing
/// required trailing slots are a completion concern, not a residue.
fn fragment_content_residue(container: NodeType, children: &[Fragment]) -> Option<usize> {
    if container == NodeType::Unknown {
        return None;
    }
    let reals: Vec<(usize, NodeType)> = children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let t = c.node.as_type();
            (t != NodeType::Unknown).then_some((i, t))
        })
        .collect();
    let types: Vec<NodeType> = reals.iter().map(|(_, t)| *t).collect();
    let consumed = greedy_consume(&Schema::node_spec(container).content, &types);
    (consumed < reals.len()).then(|| reals[consumed].0)
}

/// How many of `types` the content expr consumes from the front, greedily — the
/// dot-less mirror of the projection repair's consume walk.
fn greedy_consume(expr: &ContentExpr, types: &[NodeType]) -> usize {
    fn matches_at(e: &ContentExpr, types: &[NodeType], idx: usize) -> bool {
        types.get(idx).is_some_and(|t| e.matches(*t))
    }
    fn walk(expr: &ContentExpr, types: &[NodeType], idx: &mut usize) {
        match expr {
            ContentExpr::Empty => {}
            ContentExpr::Any => *idx = types.len(),
            ContentExpr::Single(t) => {
                if types.get(*idx) == Some(t) {
                    *idx += 1;
                }
            }
            ContentExpr::Optional(inner) => {
                if matches_at(inner, types, *idx) {
                    walk(inner, types, idx);
                }
            }
            ContentExpr::ZeroOrMore(inner) | ContentExpr::OneOrMore(inner) => {
                while matches_at(inner, types, *idx) {
                    walk(inner, types, idx);
                }
            }
            ContentExpr::Choice(cs) => {
                if let Some(c) = cs.iter().find(|c| matches_at(c, types, *idx)) {
                    walk(c, types, idx);
                }
            }
            ContentExpr::Seq(es) => {
                for e in es {
                    walk(e, types, idx);
                }
            }
        }
    }
    let mut idx = 0;
    walk(expr, types, &mut idx);
    idx
}

fn scaffold_chain_for(path: &[NodeType], child: NodeType) -> Option<Vec<NodeType>> {
    let chain = wrap_chain(path, child)?;
    (!chain.is_empty()).then_some(chain)
}

fn wrap_fragment(children: &mut Vec<Fragment>, i: usize, chain: &[NodeType]) {
    let mut current = children.remove(i);
    for &role in chain.iter().rev() {
        current = Fragment::leaf(role.into_node().to_plain()).with_children(vec![current]);
    }
    children.insert(i, current);
}

fn split_hoist_fragment(
    container: NodeType,
    children: &mut Vec<Fragment>,
    k: usize,
) -> Vec<Fragment> {
    let tail = children.split_off(k + 1);
    let promoted = children.pop().expect("k is a valid child index");
    let mut out = vec![promoted];
    if !tail.is_empty() {
        out.push(Fragment::leaf(container.into_node().to_plain()).with_children(tail));
    }
    out
}

#[cfg(test)]
mod repair_tests {
    use super::*;
    use editor_model::{PlainBulletListNode, PlainListItemNode, PlainPageBreakNode, PlainTextNode};

    fn text(t: &str) -> Fragment {
        Fragment::leaf(PlainNode::Text(PlainTextNode { text: t.into() }))
    }

    fn page_break() -> Fragment {
        Fragment::leaf(PlainNode::PageBreak(PlainPageBreakNode::default()))
    }

    fn para(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::Paragraph(PlainParagraphNode::default())).with_children(children)
    }

    fn list_item(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::ListItem(PlainListItemNode::default())).with_children(children)
    }

    fn bullet_list(children: Vec<Fragment>) -> Fragment {
        Fragment::leaf(PlainNode::BulletList(PlainBulletListNode::default()))
            .with_children(children)
    }

    fn types(fragments: &[Fragment]) -> Vec<NodeType> {
        fragments.iter().map(|f| f.node.as_type()).collect()
    }

    #[test]
    fn splits_list_item_with_two_paragraphs_into_sibling_items() {
        let mut content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            para(vec![text("b")]),
        ])])];

        repair_slice_fragments(&mut content);

        assert_eq!(types(&content), vec![NodeType::BulletList]);
        let items = &content[0].children;
        assert_eq!(types(items), vec![NodeType::ListItem, NodeType::ListItem]);
        assert_eq!(items[0].children, vec![para(vec![text("a")])]);
        assert_eq!(items[1].children, vec![para(vec![text("b")])]);
    }

    #[test]
    fn splits_list_item_with_three_paragraphs_into_three_items() {
        let mut content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            para(vec![text("b")]),
            para(vec![text("c")]),
        ])])];

        repair_slice_fragments(&mut content);

        let items = &content[0].children;
        assert_eq!(
            types(items),
            vec![NodeType::ListItem, NodeType::ListItem, NodeType::ListItem]
        );
        assert_eq!(items[0].children, vec![para(vec![text("a")])]);
        assert_eq!(items[1].children, vec![para(vec![text("b")])]);
        assert_eq!(items[2].children, vec![para(vec![text("c")])]);
    }

    #[test]
    fn keeps_trailing_paragraph_after_nested_list_in_a_sibling_item() {
        let mut content = vec![bullet_list(vec![list_item(vec![
            para(vec![text("a")]),
            bullet_list(vec![list_item(vec![para(vec![text("x")])])]),
            para(vec![text("b")]),
        ])])];

        repair_slice_fragments(&mut content);

        let items = &content[0].children;
        assert_eq!(types(items), vec![NodeType::ListItem, NodeType::ListItem]);
        assert_eq!(
            types(&items[0].children),
            vec![NodeType::Paragraph, NodeType::BulletList]
        );
        assert_eq!(items[1].children, vec![para(vec![text("b")])]);
    }

    #[test]
    fn leaves_valid_list_unchanged() {
        let mut content = vec![bullet_list(vec![
            list_item(vec![para(vec![text("a")])]),
            list_item(vec![para(vec![text("b")])]),
        ])];
        let before = content.clone();

        repair_slice_fragments(&mut content);

        assert_eq!(content, before);
    }

    #[test]
    fn leaves_bare_inline_fragments_unchanged() {
        let mut content = vec![text("a"), text("b")];
        let before = content.clone();

        repair_slice_fragments(&mut content);

        assert_eq!(content, before);
    }

    #[test]
    fn keeps_page_break_in_top_level_paragraph() {
        // PageBreak's context is `Root > Paragraph > &`; assuming `Root` above the
        // top-level fragments keeps it legal so the repair does not hoist it out.
        let mut content = vec![para(vec![text("a"), page_break()])];
        let before = content.clone();

        repair_slice_fragments(&mut content);

        assert_eq!(content, before);
    }

    #[test]
    fn deeply_nested_input_terminates() {
        // Clipboard HTML is external; nesting far past the depth ceiling must return
        // (the cap stops descent) rather than overflow the stack.
        let mut inner = para(vec![text("x")]);
        for _ in 0..(REPAIR_DEPTH_LIMIT * 4) {
            inner = bullet_list(vec![list_item(vec![inner])]);
        }
        let mut content = vec![inner];
        repair_slice_fragments(&mut content);
    }
}

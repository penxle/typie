use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Fragment, Modifier, NodeType, PlainNode, PlainParagraphNode, Schema,
};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use super::{
    apply_inline_modifiers, child_node_type, consume_pending_modifiers, find_ancestor_textblock,
    insert_hard_break_at_caret, insert_tab_at_caret, insert_text_at_caret,
    materialize_position_block, resolve_effective_modifiers,
};
use crate::types::SliceProvenance;
use crate::{CommandError, CommandResult};

mod page_break;

use page_break::{insert_terminal_page_break_from_edge, prepare_page_breaks_for_position};

enum InlineMode {
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

fn build_inline_mode(
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

pub(crate) fn insert_slice_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
    provenance: SliceProvenance,
) -> Result<Option<Selection>, CommandError> {
    if slice.is_empty() {
        return Ok(None);
    }
    let (slice, trailing_block_context) = prepare_page_breaks_for_position(tr, &position, slice);
    if slice.is_empty() {
        return Ok(None);
    }

    let in_textblock = position_in_textblock(tr, &position);
    if in_textblock {
        let opened = open_slice_for_textblock_parent(tr, &position, &slice);
        let candidate = opened.as_ref().unwrap_or(&slice);
        if let Some(fragments) =
            inline_content_fragments_for_textblock_insert(tr, &position, candidate)
        {
            let fragments: Vec<Fragment> = fragments.into_iter().cloned().collect();
            if !fragments.iter().any(is_insertable_inline_fragment) {
                return Ok(None);
            }
            let position = materialize_position_block(tr, position)?;
            let mode = build_inline_mode(tr, &position, provenance)?;
            return insert_content_as_inline_at_position(tr, position, fragments, &mode);
        }

        let Some(slice) = opened else {
            return Ok(None);
        };
        let position = materialize_position_block(tr, position)?;
        let mode = build_inline_mode(tr, &position, provenance)?;
        insert_blocks_in_textblock_at_position(tr, position, &slice, &mode)
    } else {
        let container_type = tr
            .state()
            .view()
            .node(position.node)
            .ok_or(CommandError::NodeNotFound(position.node))?
            .node_type();
        let Some(blocks) = block_boundary_fragments(&slice, container_type) else {
            return Ok(None);
        };
        let position = materialize_position_block(tr, position)?;
        insert_blocks_at_block_boundary(tr, position, blocks, trailing_block_context)
    }
}

fn position_in_textblock(tr: &Transaction, position: &Position) -> bool {
    let view = tr.state().view();
    position
        .resolve(&view)
        .is_some_and(|resolved| resolved.is_inline_position())
}

fn top_level_fragments(slice: &Slice) -> Vec<&Fragment> {
    slice.content.iter().collect()
}

fn fragments_fit_parent(parent_type: NodeType, fragments: &[&Fragment]) -> bool {
    let content = &Schema::node_spec(parent_type).content;
    fragments
        .iter()
        .all(|fragment| content.matches(fragment.node.as_type()))
}

fn fragments_are_inline(fragments: &[&Fragment]) -> bool {
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

fn open_slice_for_textblock_parent(
    tr: &Transaction,
    position: &Position,
    slice: &Slice,
) -> Option<Slice> {
    let view = tr.state().view();
    let textblock_id = find_ancestor_textblock(&view, position.node)?;
    if !can_split_textblock_for_structural_insert(&view, textblock_id) {
        return None;
    }
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

fn inline_content_fragments_for_textblock_insert<'a>(
    tr: &Transaction,
    position: &Position,
    slice: &'a Slice,
) -> Option<Vec<&'a Fragment>> {
    let view = tr.state().view();
    let textblock_id = find_ancestor_textblock(&view, position.node)?;
    let textblock = view.node(textblock_id)?;
    let parent = textblock.parent()?;
    let textblock_type = textblock.node_type();
    let parent_type = parent.node_type();

    let top_level = top_level_fragments(slice);
    if fragments_are_inline(&top_level) && fragments_fit_parent(textblock_type, &top_level) {
        return Some(top_level);
    }

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

    if !can_split_textblock_for_structural_insert(&view, textblock_id)
        || !fragments_fit_parent(parent_type, &top_level)
    {
        Some(open_content)
    } else {
        None
    }
}

fn insert_content_as_inline_at_position(
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

fn insert_blocks_in_textblock_at_position(
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

fn is_insertable_inline_fragment(fragment: &Fragment) -> bool {
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

    let (container_id, textblock_index) = {
        let view = tr.state().view();
        let tb = view
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = tb.parent().ok_or(CommandError::NoParent(textblock_id))?;
        let textblock_index = tb
            .index()
            .ok_or_else(|| CommandError::orphan_child(textblock_id, parent.id()))?;
        (parent.id(), textblock_index)
    };

    // Split the textblock at the resolved child index. The right half becomes
    // the next sibling block.
    tr.split_node(textblock_id, split_index_in_textblock)?;
    let p2_id = block_child_id(tr, container_id, textblock_index + 1)
        .ok_or_else(|| CommandError::Corrupted("split produced no right half".into()))?;

    let blocks: Vec<&Fragment> = slice.content.iter().collect();

    let merge_start = slice.open_start > 0
        && blocks
            .first()
            .is_some_and(|b| same_textblock_type(&b.node, textblock_id, tr));
    let merge_end = slice.open_end > 0
        && blocks
            .last()
            .is_some_and(|b| same_textblock_type(&b.node, p2_id, tr));
    let merge_end_into_start = merge_start && merge_end && blocks.len() == 1;

    let middle_start = if merge_start { 1 } else { 0 };
    let middle_end = if merge_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };
    let merge_end = merge_end && !merge_end_into_start && middle_end >= middle_start;

    let mut last_caret: Option<Position> = None;
    let mut inserted_range = InsertedRange::default();
    let mut terminal_page_break_start: Option<Position> = None;

    if merge_start {
        let first = blocks[0];
        let inline = first.children.to_vec();
        tr.set_selection(Some(Selection::collapsed(position_at_end_of_block(
            tr,
            textblock_id,
        )?)))?;
        let start = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        let inserted = insert_inline_fragments(tr, &inline, mode)?;
        let inserted_page_break = insert_terminal_page_break_from_edge(tr, textblock_id, &inline)?;
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

    if merge_end_into_start {
        // p2 is textblock's next sibling; fold it back in.
        tr.merge_node(textblock_id)?;
        last_caret = tr.selection().map(|s| s.head);
    }

    for (insert_at, block) in
        (textblock_index + 1..).zip(blocks.iter().take(middle_end).skip(middle_start))
    {
        let subtree = (*block).clone().into_subtree();
        tr.insert_subtree(container_id, insert_at, subtree)?;
        if let Some(inserted_id) = block_child_id(tr, container_id, insert_at) {
            inserted_range.include_block(inserted_id);
            if let Some(paint) = mode.plain_paint() {
                paint_block_uniformly(tr, inserted_id, paint)?;
            }
            // Block-level atom leaves (e.g. Image) have no inner caret; their
            // bracket selection comes from `inserted_range` instead.
            if let Ok(p) = position_at_end_of_block(tr, inserted_id) {
                last_caret = Some(p);
            }
        }
    }

    if merge_end {
        let last = blocks.last().unwrap();
        let inline = last.children.to_vec();
        tr.set_selection(Some(Selection::collapsed(position_at_start_of_block(
            tr, p2_id,
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
            tr.replace_carry(p2_id, carry_from_paint(paint))?;
        }
    }

    let keep_trailing_page_break_context = terminal_page_break_start.is_some();
    if let Some(start) = terminal_page_break_start {
        let end = position_at_start_of_block(tr, p2_id)?;
        inserted_range.include_position_range(start, end);
        last_caret = Some(end);
    }

    let safe_to_remove = |tr: &Transaction, target: Dot| -> bool {
        let view = tr.state().view();
        let Some(target_node) = view.node(target) else {
            return false;
        };
        if target_node.children().count() != 0 {
            return false;
        }
        let Some(container) = view.node(container_id) else {
            return false;
        };
        let remaining: Vec<NodeType> = container
            .children()
            .filter(|c| match c {
                ChildView::Block(b) => b.id() != target,
                ChildView::Leaf(_) => true,
            })
            .map(|c| child_node_type(&c))
            .collect();
        container.spec().content.matches_sequence(&remaining)
    };

    if !merge_start && safe_to_remove(tr, textblock_id) {
        tr.remove_subtree(textblock_id)?;
    }
    if !merge_end && !keep_trailing_page_break_context && safe_to_remove(tr, p2_id) {
        tr.remove_subtree(p2_id)?;
    }

    let steps = {
        let view = tr.state().view();
        view.node(container_id)
            .map(|container| fulfill(&container))
            .unwrap_or_default()
    };
    tr.apply_steps(steps)?;

    let mut final_pos = match last_caret {
        Some(p) => p,
        None => Position {
            node: p2_id,
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    final_pos.affinity = Affinity::Downstream;
    let explicit_inserted_selection = inserted_range.selection(tr);
    let split_boundary_selection = if explicit_inserted_selection.is_none()
        && tr.state().view().node(textblock_id).is_some()
        && tr.state().view().node(p2_id).is_some()
    {
        Some(Selection::new(
            position_at_end_of_block(tr, textblock_id)?,
            position_at_start_of_block(tr, p2_id)?,
        ))
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

fn insert_blocks_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    blocks: Vec<Fragment>,
    trailing_block_context: Option<Fragment>,
) -> Result<Option<Selection>, CommandError> {
    let container_id = position.node;
    let base_index = position.offset;
    let payload_block_count = blocks.len();
    let inserted_block_count = payload_block_count + usize::from(trailing_block_context.is_some());
    tr.batch(|tr| {
        for (offset, block) in blocks.iter().enumerate() {
            let subtree = block.clone().into_subtree();
            tr.insert_subtree(container_id, base_index + offset, subtree)?;
        }
        if let Some(context) = &trailing_block_context {
            tr.insert_subtree(
                container_id,
                base_index + payload_block_count,
                context.clone().into_subtree(),
            )?;
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

    if let Some(id) = block_child_id(tr, container_id, base_index + inserted_block_count - 1)
        && let Ok(mut final_pos) = position_at_end_of_block(tr, id)
    {
        final_pos.affinity = Affinity::Downstream;
        tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    }

    Ok(Some(selection_over_inserted_blocks(
        container_id,
        base_index,
        payload_block_count,
    )))
}

fn block_boundary_fragments(slice: &Slice, container_type: NodeType) -> Option<Vec<Fragment>> {
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

fn same_textblock_type(slice_node: &PlainNode, doc_node_id: Dot, tr: &Transaction) -> bool {
    let view = tr.state().view();
    let Some(doc_node) = view.node(doc_node_id) else {
        return false;
    };
    let slice_type = slice_node.as_type();
    Schema::node_spec(slice_type).is_textblock()
        && doc_node.spec().is_textblock()
        && slice_type == doc_node.node_type()
}

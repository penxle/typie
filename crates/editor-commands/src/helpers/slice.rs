use editor_clipboard::Slice;
use editor_model::{
    Fragment, Modifier, Node, NodeId, NodeType, PlainNode, PlainParagraphNode, PlainTextNode,
    Schema, Subtree,
};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use super::{
    compact_textblock_preserving_caret, find_ancestor_textblock, insert_hard_break_at_caret,
    insert_tab_at_caret, insert_text_at_caret,
};
use crate::{CommandError, CommandResult};

pub(crate) fn insert_slice_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
) -> Result<Option<Selection>, CommandError> {
    if slice.is_empty() {
        return Ok(None);
    }

    let in_textblock = position_in_textblock(tr, position);
    if in_textblock {
        if let Some(fragments) = inline_content_fragments_for_textblock_insert(tr, position, &slice)
        {
            insert_content_as_inline_at_position(tr, position, fragments)
        } else {
            insert_blocks_in_textblock_at_position(tr, position, &slice)
        }
    } else {
        insert_blocks_at_block_boundary(tr, position, &slice)
    }
}

fn position_in_textblock(tr: &Transaction, position: Position) -> bool {
    let doc = tr.doc();
    position
        .resolve(&doc)
        .is_some_and(|resolved| resolved.is_inline_position())
}

fn top_level_fragments(slice: &Slice) -> Vec<&Fragment> {
    match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    }
}

fn open_content_fragments(mut fragments: Vec<&Fragment>, open_depth: u32) -> Vec<&Fragment> {
    for _ in 0..open_depth {
        let mut next = Vec::new();
        for fragment in fragments {
            if Schema::node_spec(fragment.node.as_type()).is_leaf() || fragment.children.is_empty()
            {
                next.push(fragment);
            } else {
                next.extend(fragment.children.iter());
            }
        }
        fragments = next;
    }
    fragments
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

fn can_split_textblock_for_structural_insert(
    doc: &editor_model::Doc,
    textblock_id: NodeId,
) -> bool {
    let Some(textblock) = doc.node(textblock_id) else {
        return false;
    };
    let Some(parent) = textblock.parent() else {
        return false;
    };
    let Some(index) = textblock.index() else {
        return false;
    };

    let mut child_types: Vec<NodeType> = parent.children().map(|child| child.as_type()).collect();
    child_types.insert(index + 1, textblock.as_type());
    parent.spec().content.validate(&child_types).is_ok()
}

fn inline_content_fragments_for_textblock_insert<'a>(
    tr: &Transaction,
    position: Position,
    slice: &'a Slice,
) -> Option<Vec<&'a Fragment>> {
    let doc = tr.doc();
    let Some(textblock_id) = find_ancestor_textblock(&doc, position.node_id) else {
        return None;
    };
    let Some(textblock) = doc.node(textblock_id) else {
        return None;
    };
    let Some(parent) = textblock.parent() else {
        return None;
    };

    let top_level = top_level_fragments(slice);
    if fragments_are_inline(&top_level) && fragments_fit_parent(textblock.as_type(), &top_level) {
        return Some(top_level);
    }

    if slice.open_start == 0 {
        return None;
    }

    let open_content = open_content_fragments(top_level.clone(), slice.open_start);
    if !fragments_are_inline(&open_content)
        || !fragments_fit_parent(textblock.as_type(), &open_content)
    {
        return None;
    }

    if !can_split_textblock_for_structural_insert(&doc, textblock_id)
        || !fragments_fit_parent(parent.as_type(), &top_level)
    {
        Some(open_content)
    } else {
        None
    }
}

fn textblock_is_empty(tr: &Transaction, textblock_id: NodeId) -> bool {
    let doc = tr.doc();
    let Some(node) = doc.node(textblock_id) else {
        return false;
    };
    if !node.spec().is_textblock() {
        return false;
    }
    node.children().all(|c| match c.node() {
        Node::Text(t) => t.text.is_empty(),
        _ => false,
    })
}

fn insert_content_as_inline_at_position(
    tr: &mut Transaction,
    position: Position,
    fragments: Vec<&Fragment>,
) -> Result<Option<Selection>, CommandError> {
    let fragments: Vec<Fragment> = fragments.into_iter().cloned().collect();
    if fragments.is_empty() {
        return Ok(None);
    }

    tr.set_selection(Some(Selection::collapsed(position)))?;
    let start = tr
        .selection()
        .expect("selection preserved through mutations")
        .head;
    let inserted = insert_inline_fragments(tr, fragments)?;
    if !inserted {
        return Ok(None);
    }
    let end = tr
        .selection()
        .expect("selection preserved through mutations")
        .head;
    Ok(Some(normalized_selection(tr, start, end)))
}

fn insert_blocks_in_textblock_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    tr.set_selection(Some(Selection::collapsed(position)))?;
    insert_blocks_in_textblock(tr, slice)
}

fn insert_inline_fragments(tr: &mut Transaction, fragments: Vec<Fragment>) -> CommandResult {
    let mut any_change = false;
    for f in fragments {
        // Each clipboard run carries its own style ref. Route it through the
        // pending-style-ref channel that insert_text/insert_tab already consume
        // so the inserted run node receives the source run's style (and an
        // unstyled run forces None rather than inheriting the destination's).
        set_pending_style_for_run(tr, &f.style)?;
        match f.node {
            PlainNode::Text(t) if !t.text.is_empty() => {
                if f.modifiers.is_empty() {
                    insert_text_at_caret(tr, &t.text)?;
                } else {
                    insert_modifier_text(tr, &t.text, f.modifiers, f.style.clone())?;
                }
                any_change = true;
            }
            PlainNode::HardBreak(_) => {
                insert_hard_break_at_caret(tr)?;
                any_change = true;
            }
            PlainNode::Tab(_) => {
                insert_tab_at_caret(tr)?;
                any_change = true;
            }
            _ => {}
        }
    }
    // Leave no pending style lingering after the paste completes.
    if tr.pending_style().is_some() {
        tr.set_pending_style(None)?;
    }
    Ok(any_change)
}

fn set_pending_style_for_run(
    tr: &mut Transaction,
    style: &Option<String>,
) -> Result<(), CommandError> {
    let pending = match style {
        Some(style_id) => Some(editor_state::PendingStyle::Set {
            style_id: style_id.clone(),
        }),
        None => Some(editor_state::PendingStyle::Unset),
    };
    tr.set_pending_style(pending)?;
    Ok(())
}

fn insert_modifier_text(
    tr: &mut Transaction,
    text: &str,
    modifiers: Vec<Modifier>,
    style: Option<String>,
) -> CommandResult {
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let (parent_id, child_index) = textblock_insert_point(tr, pos)?;
    let id = NodeId::new();
    let subtree =
        Subtree::leaf(id, PlainNode::Text(PlainTextNode::default())).with_modifiers(modifiers);
    tr.insert_subtree(parent_id, child_index, subtree)?;
    tr.insert_text(id, 0, text)?;
    if let Some(style_id) = style {
        tr.set_node_style(id, Some(style_id))?;
    }
    let len = text.chars().count();
    tr.set_selection(Some(Selection::collapsed(Position {
        node_id: id,
        offset: len,
        affinity: Affinity::Upstream,
    })))?;
    Ok(true)
}

fn textblock_insert_point(
    tr: &mut Transaction,
    pos: Position,
) -> Result<(NodeId, usize), CommandError> {
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;
    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            let parent_id = parent.id();
            let text_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent_id))?;
            let text_len = text_node.text.len();
            let index = if pos.offset == 0 {
                text_index
            } else if pos.offset == text_len {
                text_index + 1
            } else {
                drop(doc);
                let split_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_id)?;
                text_index + 1
            };
            Ok((parent_id, index))
        }
        _ => Ok((pos.node_id, pos.offset)),
    }
}

#[derive(Clone, Copy)]
enum InsertedRangeEndpoint {
    // Open slice edges merge into split textblocks and are represented by
    // inline positions. Closed middle blocks are resolved after cleanup because
    // removing empty split halves can shift their parent indices.
    Position(Position),
    BeforeBlock(NodeId),
    AfterBlock(NodeId),
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

    fn include_block(&mut self, block_id: NodeId) {
        self.start
            .get_or_insert(InsertedRangeEndpoint::BeforeBlock(block_id));
        self.end = Some(InsertedRangeEndpoint::AfterBlock(block_id));
    }

    fn selection(&self, tr: &Transaction) -> Option<Selection> {
        let start = resolve_inserted_range_endpoint(tr, self.start?)?;
        let end = resolve_inserted_range_endpoint(tr, self.end?)?;
        Some(normalized_selection(tr, start, end))
    }
}

fn insert_blocks_in_textblock(
    tr: &mut Transaction,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    let head = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    // Resolve textblock id + split index, splitting any straddling text node first.
    let (textblock_id, split_index_in_textblock) = {
        let doc = tr.doc();
        let head_node = doc
            .node(head.node_id)
            .ok_or(CommandError::NodeNotFound(head.node_id))?;
        match head_node.node() {
            Node::Text(text_node) => {
                let parent = head_node
                    .parent()
                    .ok_or(CommandError::NoParent(head.node_id))?;
                let textblock_id = parent.id();
                let text_index = head_node
                    .index()
                    .ok_or(CommandError::orphan_child(head.node_id, textblock_id))?;
                let text_len = text_node.text.len();
                let index = if head.offset == 0 {
                    text_index
                } else if head.offset == text_len {
                    text_index + 1
                } else {
                    drop(doc);
                    let split_text_id = NodeId::new();
                    tr.split_node(head.node_id, head.offset, split_text_id)?;
                    text_index + 1
                };
                (textblock_id, index)
            }
            _ => (head.node_id, head.offset),
        }
    };

    let (container_id, textblock_index) = {
        let doc = tr.doc();
        let tb = doc
            .node(textblock_id)
            .ok_or(CommandError::NodeNotFound(textblock_id))?;
        let parent = tb.parent().ok_or(CommandError::NoParent(textblock_id))?;
        let textblock_index = tb
            .index()
            .ok_or(CommandError::orphan_child(textblock_id, parent.id()))?;
        (parent.id(), textblock_index)
    };

    let textblock_was_empty = textblock_is_empty(tr, textblock_id);

    // Split the textblock at the resolved child index. p2_id becomes the right half.
    let p2_id = NodeId::new();
    tr.split_node(textblock_id, split_index_in_textblock, p2_id)?;

    let blocks: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };

    let merge_start = slice.open_start > 0
        && blocks
            .first()
            .is_some_and(|b| same_textblock_type(&b.node, textblock_id, tr));
    let merge_end = slice.open_end > 0
        && blocks
            .last()
            .is_some_and(|b| same_textblock_type(&b.node, p2_id, tr));
    // If one open textblock is both the first and last block, it receives both
    // split halves instead of being applied once to each side.
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

    if merge_start {
        let first = blocks[0];
        if textblock_was_empty && let Some(style) = first.style.clone() {
            tr.set_node_style(textblock_id, Some(style))?;
        }
        let inline = first.children.to_vec();
        tr.set_selection(Some(Selection::collapsed(position_at_end_of_block(
            tr,
            textblock_id,
        )?)))?;
        let start = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        let inserted = insert_inline_fragments(tr, inline)?;
        let end = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        if inserted {
            inserted_range.include_position_range(start, end);
            last_caret = Some(end);
        }
    }

    if merge_end_into_start {
        tr.merge_node(p2_id, textblock_id)?;
        let caret = last_caret.unwrap_or(head);
        compact_textblock_preserving_caret(tr, caret)?;
        last_caret = tr.selection().map(|s| s.head);
    }

    for (insert_at, block) in
        (textblock_index + 1..).zip(blocks.iter().take(middle_end).skip(middle_start))
    {
        let subtree = (*block).clone().into_subtree();
        let inserted_id = subtree.id;
        tr.insert_subtree(container_id, insert_at, subtree)?;
        inserted_range.include_block(inserted_id);
        last_caret = Some(position_at_end_of_block(tr, inserted_id)?);
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
        let inserted = insert_inline_fragments(tr, inline)?;
        // After inserting at the start of p2, the caret naturally lands between
        // the merged-in inline and p2's original inline content.
        let end = tr
            .selection()
            .expect("selection preserved through mutations")
            .head;
        if inserted {
            inserted_range.include_position_range(start, end);
        }
        last_caret = Some(end);
    }

    // Drop the split halves when they're empty AND the container's schema
    // still accepts the remaining children without them — for example a Root
    // requiring a trailing Paragraph keeps p2 when no other textblock follows
    // the inserted blocks. fulfill below then patches any leftover gaps the
    // insertion or removal couldn't repair locally.
    let safe_to_remove = |tr: &Transaction, target: NodeId| -> bool {
        let doc = tr.doc();
        let Some(target_node) = doc.node(target) else {
            return false;
        };
        if target_node.children().count() != 0 {
            return false;
        }
        let Some(container) = doc.node(container_id) else {
            return false;
        };
        let remaining: Vec<NodeType> = container
            .children()
            .filter(|c| c.id() != target)
            .map(|c| c.as_type())
            .collect();
        container.spec().content.validate(&remaining).is_ok()
    };

    if !merge_start && safe_to_remove(tr, textblock_id) {
        tr.remove_subtree(textblock_id)?;
    }
    if !merge_end && safe_to_remove(tr, p2_id) {
        tr.remove_subtree(p2_id)?;
    }

    if let Some(container) = tr.doc().node(container_id) {
        let steps = fulfill(&container);
        tr.apply_steps(steps)?;
    }

    let final_pos = match last_caret {
        Some(p) => p,
        None => Position {
            node_id: p2_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };
    let explicit_inserted_selection = inserted_range.selection(tr);
    let split_boundary_selection = if explicit_inserted_selection.is_none()
        && tr.doc().node(textblock_id).is_some()
        && tr.doc().node(p2_id).is_some()
    {
        Some(normalized_selection(
            tr,
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

fn normalized_selection(tr: &Transaction, anchor: Position, head: Position) -> Selection {
    let selection = Selection::new(anchor, head);
    selection.normalize(&tr.doc()).unwrap_or(selection)
}

fn position_at_end_of_block(tr: &Transaction, block_id: NodeId) -> Result<Position, CommandError> {
    let doc = tr.doc();
    let block = doc
        .node(block_id)
        .ok_or(CommandError::NodeNotFound(block_id))?;
    let position = match block.last_child() {
        Some(c) => match c.node() {
            Node::Text(t) => Position {
                node_id: c.id(),
                offset: t.text.len(),
                affinity: Affinity::Upstream,
            },
            _ => {
                let child_count = block.children().count();
                Position {
                    node_id: block_id,
                    offset: child_count,
                    affinity: Affinity::Upstream,
                }
            }
        },
        None => Position {
            node_id: block_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };
    Ok(position)
}

fn position_at_start_of_block(
    tr: &Transaction,
    block_id: NodeId,
) -> Result<Position, CommandError> {
    let doc = tr.doc();
    let block = doc
        .node(block_id)
        .ok_or(CommandError::NodeNotFound(block_id))?;
    let position = match block.first_child() {
        Some(c) => match c.node() {
            Node::Text(_) => Position {
                node_id: c.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
            _ => Position {
                node_id: block_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        },
        None => Position {
            node_id: block_id,
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    Ok(position)
}

fn insert_blocks_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    let container_id = position.node_id;
    let base_index = position.offset;
    let container_type = tr
        .doc()
        .node(container_id)
        .ok_or(CommandError::NodeNotFound(container_id))?
        .as_type();
    let blocks = block_boundary_fragments(slice, container_type);
    if blocks.is_empty() {
        return Ok(None);
    }

    let mut last_inserted: Option<NodeId> = None;
    tr.batch(|tr| {
        for (offset, block) in blocks.iter().enumerate() {
            let subtree = block.clone().into_subtree();
            let inserted_id = subtree.id;
            tr.insert_subtree(container_id, base_index + offset, subtree)?;
            last_inserted = Some(inserted_id);
        }

        let steps = {
            let doc = tr.doc();
            doc.node(container_id)
                .map(|container| fulfill(&container))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        Ok::<(), CommandError>(())
    })?;

    if let Some(id) = last_inserted {
        let final_pos = position_at_end_of_block(tr, id)?;
        tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    }

    Ok(Some(selection_over_inserted_blocks(
        container_id,
        base_index,
        blocks.len(),
    )))
}

fn block_boundary_fragments(slice: &Slice, container_type: NodeType) -> Vec<Fragment> {
    let top_level = top_level_fragments(slice);
    if fragments_are_inline(&top_level)
        && Schema::node_spec(container_type)
            .content
            .matches(NodeType::Paragraph)
    {
        return vec![Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            style: None,
            children: top_level.into_iter().cloned().collect(),
        }];
    }

    top_level.into_iter().cloned().collect()
}

fn selection_over_inserted_blocks(
    container_id: NodeId,
    start_index: usize,
    block_count: usize,
) -> Selection {
    Selection::new(
        Position {
            node_id: container_id,
            offset: start_index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: container_id,
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
        InsertedRangeEndpoint::BeforeBlock(id) | InsertedRangeEndpoint::AfterBlock(id) => {
            let doc = tr.doc();
            let node = doc.node(id)?;
            let parent = node.parent()?;
            let index = node.index()?;
            let (offset, affinity) = match endpoint {
                InsertedRangeEndpoint::BeforeBlock(_) => (index, Affinity::Downstream),
                InsertedRangeEndpoint::AfterBlock(_) => (index + 1, Affinity::Upstream),
                InsertedRangeEndpoint::Position(_) => unreachable!(),
            };
            Some(Position {
                node_id: parent.id(),
                offset,
                affinity,
            })
        }
    }
}

fn same_textblock_type(slice_node: &PlainNode, doc_node_id: NodeId, tr: &Transaction) -> bool {
    let doc = tr.doc();
    let Some(doc_node) = doc.node(doc_node_id) else {
        return false;
    };
    let slice_type = slice_node.as_type();
    Schema::node_spec(slice_type).is_textblock()
        && doc_node.spec().is_textblock()
        && slice_type == doc_node.as_type()
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_model::{Fragment, PlainNode, PlainRootNode};

    #[test]
    fn empty_slice_helper_recognises_bare_container() {
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        assert!(empty.is_empty());
    }
}

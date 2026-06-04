use editor_clipboard::Slice;
use editor_model::{
    Fragment, Modifier, Node, NodeId, NodeType, PlainNode, PlainParagraphNode, PlainRootNode,
    PlainTextNode, Schema, Subtree,
};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use super::{insert_hard_break_at_caret, insert_tab_at_caret, insert_text_at_caret};
use crate::{CommandError, CommandResult};

pub(crate) fn insert_slice_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
) -> Result<Option<Selection>, CommandError> {
    if slice.is_empty() {
        return Ok(None);
    }

    let slice = coerce_slice_for_position(tr, position, slice);
    if slice.is_empty() {
        return Ok(None);
    }

    let inline_only = is_inline_only(&slice);
    let in_textblock = position_in_textblock(tr, position);
    match (inline_only, in_textblock) {
        (true, true) => insert_inline_at_position(tr, position, &slice),
        (false, true) => insert_blocks_in_textblock_at_position(tr, position, &slice),
        (true, false) => insert_inline_at_block_boundary(tr, position, &slice),
        (false, false) => insert_blocks_at_block_boundary(tr, position, &slice),
    }
}

// Coerce slice's top-level block children to types the caret container allows.
// Disallowed types are unwrapped recursively until either an allowed type or
// an inline leaf is reached.
fn coerce_slice_for_position(tr: &Transaction, position: Position, slice: Slice) -> Slice {
    let container_type = match container_type_for_position(tr, position) {
        Some(t) => t,
        None => return slice,
    };

    let Slice {
        fragment,
        open_start,
        open_end,
    } = slice;

    match fragment.node {
        PlainNode::Root(_) => {
            let coerced: Vec<Fragment> = fragment
                .children
                .into_iter()
                .flat_map(|c| coerce_fragment_for_parent(c, container_type))
                .collect();
            Slice {
                fragment: Fragment {
                    node: fragment.node,
                    modifiers: fragment.modifiers,
                    style: fragment.style,
                    children: coerced,
                },
                open_start,
                open_end,
            }
        }
        _ => {
            let coerced = coerce_fragment_for_parent(
                Fragment {
                    node: fragment.node,
                    modifiers: fragment.modifiers,
                    style: fragment.style,
                    children: fragment.children,
                },
                container_type,
            );
            let wrapped = Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: coerced,
            };
            Slice {
                fragment: wrapped,
                open_start,
                open_end,
            }
        }
    }
}

fn container_type_for_position(tr: &Transaction, position: Position) -> Option<NodeType> {
    let state = tr.state();
    let node = state.doc.node(position.node_id)?;
    // Coerce only against the textblock the caret sits inside — at block
    // boundaries (Case C/D candidates) we want the slice's blocks to land
    // as siblings of the existing blocks, not be unwrapped against the
    // boundary container's schema.
    match node.node() {
        Node::Text(_) => node.parent().map(|p| p.as_type()),
        _ => None,
    }
}

fn coerce_fragment_for_parent(f: Fragment, parent_type: NodeType) -> Vec<Fragment> {
    let f_type = f.node.as_type();
    let f_spec = Schema::node_spec(f_type);
    let parent_spec = Schema::node_spec(parent_type);

    // Block content inside a textblock parent (nested textblock, or a block
    // leaf like Image/HorizontalRule) keeps its boundary so the textblock
    // gets split around it instead of being recursively unwrapped to nothing.
    if parent_spec.is_textblock() && (f_spec.is_textblock() || f_spec.is_leaf()) && !f_spec.inline {
        return vec![f];
    }
    // Inline content reaches the inline insertion path as-is.
    if f_spec.inline {
        return vec![f];
    }
    if child_allowed(parent_type, f_type) {
        return vec![f];
    }
    let mut out = vec![];
    for child in f.children {
        out.extend(coerce_fragment_for_parent(child, parent_type));
    }
    out
}

fn child_allowed(parent_type: NodeType, child_type: NodeType) -> bool {
    let spec = Schema::node_spec(parent_type);
    spec.content.allowed_types().contains(&child_type)
}

// An inline-only slice represents pasteable content that fits inside a single
// textblock — either bare inline (Text/HardBreak) or a single textblock wrapper
// (Paragraph) around inline. A Root with multiple block children is a
// block-sequence even if every block happens to be inline-compatible.
fn is_inline_only(slice: &Slice) -> bool {
    fn is_textblock_wrapper(n: &PlainNode) -> bool {
        matches!(n, PlainNode::Paragraph(_))
    }
    fn is_inline_leaf(n: &PlainNode) -> bool {
        matches!(
            n,
            PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_)
        )
    }

    let frag = &slice.fragment;
    match &frag.node {
        n if is_inline_leaf(n) => true,
        n if is_textblock_wrapper(n) => frag.children.iter().all(|c| is_inline_leaf(&c.node)),
        PlainNode::Root(_) => {
            let block_kids: Vec<&Fragment> = frag
                .children
                .iter()
                .filter(|c| !is_inline_leaf(&c.node))
                .collect();
            match block_kids.len() {
                0 => true,
                1 if is_textblock_wrapper(&block_kids[0].node) => block_kids[0]
                    .children
                    .iter()
                    .all(|c| is_inline_leaf(&c.node)),
                _ => false,
            }
        }
        _ => false,
    }
}

fn position_in_textblock(tr: &Transaction, position: Position) -> bool {
    let doc = tr.doc();
    position
        .resolve(&doc)
        .is_some_and(|resolved| resolved.is_inline_position())
}

fn enclosing_textblock_id(tr: &Transaction, position: Position) -> Option<NodeId> {
    let doc = tr.doc();
    let node = doc.node(position.node_id)?;
    if matches!(node.node(), Node::Text(_)) {
        return node.parent().map(|p| p.id());
    }
    if node.spec().is_textblock() {
        return Some(node.id());
    }
    None
}

fn paragraph_is_empty(tr: &Transaction, para_id: NodeId) -> bool {
    let doc = tr.doc();
    let Some(node) = doc.node(para_id) else {
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

fn source_paragraph_wrapper_style(slice: &Slice) -> Option<String> {
    match &slice.fragment.node {
        PlainNode::Paragraph(_) => slice.fragment.style.clone(),
        PlainNode::Root(_) => {
            let paras: Vec<&Fragment> = slice
                .fragment
                .children
                .iter()
                .filter(|c| matches!(c.node, PlainNode::Paragraph(_)))
                .collect();
            if paras.len() == 1 {
                paras[0].style.clone()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn collect_inline(f: &Fragment) -> Vec<&Fragment> {
    fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
        match &f.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_) => out.push(f),
            _ => {
                for c in &f.children {
                    walk(c, out);
                }
            }
        }
    }
    let mut out = vec![];
    walk(f, &mut out);
    out
}

fn insert_inline_at_caret(tr: &mut Transaction, slice: &Slice) -> CommandResult {
    let fragments: Vec<Fragment> = collect_inline(&slice.fragment)
        .into_iter()
        .cloned()
        .collect();
    insert_inline_fragments(tr, fragments)
}

fn insert_inline_at_position(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    let dest_para = enclosing_textblock_id(tr, position);
    let dest_was_empty = dest_para.is_some_and(|id| paragraph_is_empty(tr, id));

    tr.set_selection(Some(Selection::collapsed(position)))?;
    let start = tr.selection().map(|s| s.head).unwrap_or(position);
    let inserted = insert_inline_at_caret(tr, slice)?;
    if !inserted {
        return Ok(None);
    }
    if dest_was_empty
        && let (Some(para_id), Some(style)) = (dest_para, source_paragraph_wrapper_style(slice))
    {
        tr.set_node_style(para_id, Some(style))?;
    }
    let Some(end) = tr.selection().map(|s| s.head) else {
        return Ok(None);
    };
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
        match f.node {
            PlainNode::Text(t) if !t.text.is_empty() => {
                if f.modifiers.is_empty() {
                    insert_text_at_caret(tr, &t.text)?;
                } else {
                    insert_modifier_text(tr, &t.text, f.modifiers)?;
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
    Ok(any_change)
}

fn insert_modifier_text(
    tr: &mut Transaction,
    text: &str,
    modifiers: Vec<Modifier>,
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

    let middle_start = if merge_start { 1 } else { 0 };
    let middle_end = if merge_end {
        blocks.len().saturating_sub(1)
    } else {
        blocks.len()
    };
    // When the same block participates as both first and last (single-block slice with
    // both ends open and matching textblocks), only merge into the start to avoid
    // double-applying its inline content.
    let merge_end = merge_end && middle_end >= middle_start;

    let mut last_caret: Option<Position> = None;
    let mut inserted_start: Option<InsertedRangeEndpoint> = None;
    let mut inserted_end: Option<InsertedRangeEndpoint> = None;

    if merge_start {
        let first = blocks[0];
        let inline = first.children.to_vec();
        position_caret_at_textblock_end(tr, textblock_id)?;
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
            inserted_start.get_or_insert(InsertedRangeEndpoint::Position(start));
            inserted_end = Some(InsertedRangeEndpoint::Position(end));
        }
        last_caret = Some(end);
    }

    for (insert_at, block) in
        (textblock_index + 1..).zip(blocks.iter().take(middle_end).skip(middle_start))
    {
        let subtree = (*block).clone().into_subtree();
        let inserted_id = subtree.id;
        tr.insert_subtree(container_id, insert_at, subtree)?;
        inserted_start.get_or_insert(InsertedRangeEndpoint::BeforeBlock(inserted_id));
        inserted_end = Some(InsertedRangeEndpoint::AfterBlock(inserted_id));
        last_caret = Some(position_at_end_of_block(tr, inserted_id));
    }

    if merge_end {
        let last = blocks.last().unwrap();
        let inline = last.children.to_vec();
        position_caret_at_textblock_start(tr, p2_id)?;
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
            inserted_start.get_or_insert(InsertedRangeEndpoint::Position(start));
            inserted_end = Some(InsertedRangeEndpoint::Position(end));
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
    let inserted_selection = inserted_start.zip(inserted_end).and_then(|(start, end)| {
        let start = resolve_inserted_range_endpoint(tr, start)?;
        let end = resolve_inserted_range_endpoint(tr, end)?;
        Some(normalized_selection(tr, start, end))
    });
    tr.set_selection(Some(Selection::collapsed(final_pos)))?;

    Ok(inserted_selection)
}

fn normalized_selection(tr: &Transaction, anchor: Position, head: Position) -> Selection {
    let selection = Selection::new(anchor, head);
    selection.normalize(&tr.doc()).unwrap_or(selection)
}

fn position_caret_at_textblock_end(
    tr: &mut Transaction,
    textblock_id: NodeId,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let tb = doc
        .node(textblock_id)
        .ok_or(CommandError::NodeNotFound(textblock_id))?;
    let pos = match tb.last_child() {
        Some(c) => match c.node() {
            Node::Text(t) => Position {
                node_id: c.id(),
                offset: t.text.len(),
                affinity: Affinity::Upstream,
            },
            _ => {
                let child_count = tb.children().count();
                Position {
                    node_id: textblock_id,
                    offset: child_count,
                    affinity: Affinity::Upstream,
                }
            }
        },
        None => Position {
            node_id: textblock_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };
    drop(doc);
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}

fn position_caret_at_textblock_start(
    tr: &mut Transaction,
    textblock_id: NodeId,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let tb = doc
        .node(textblock_id)
        .ok_or(CommandError::NodeNotFound(textblock_id))?;
    let pos = match tb.first_child() {
        Some(c) => match c.node() {
            Node::Text(_) => Position {
                node_id: c.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
            _ => Position {
                node_id: textblock_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        },
        None => Position {
            node_id: textblock_id,
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    drop(doc);
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}

fn position_at_end_of_block(tr: &Transaction, block_id: NodeId) -> Position {
    let doc = tr.doc();
    let block = doc.node(block_id).expect("inserted block exists");
    match block.last_child() {
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
    }
}

fn insert_inline_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    let inline_clones: Vec<Fragment> = collect_inline(&slice.fragment)
        .into_iter()
        .cloned()
        .collect();
    if inline_clones.is_empty() {
        return Ok(None);
    }

    let new_para_id = NodeId::new();
    let para_subtree = Subtree::leaf(
        new_para_id,
        PlainNode::Paragraph(PlainParagraphNode::default()),
    )
    .with_style(source_paragraph_wrapper_style(slice));
    tr.insert_subtree(position.node_id, position.offset, para_subtree)?;

    position_caret_at_textblock_start(tr, new_para_id)?;
    insert_inline_fragments(tr, inline_clones)?;
    Ok(Some(selection_over_inserted_blocks(
        position.node_id,
        position.offset,
        1,
    )))
}

fn insert_blocks_at_block_boundary(
    tr: &mut Transaction,
    position: Position,
    slice: &Slice,
) -> Result<Option<Selection>, CommandError> {
    let container_id = position.node_id;
    let base_index = position.offset;
    let blocks: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };
    if blocks.is_empty() {
        return Ok(None);
    }

    let mut last_inserted: Option<NodeId> = None;
    tr.batch(|tr| {
        for (offset, block) in blocks.iter().enumerate() {
            let subtree = (*block).clone().into_subtree();
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
        let final_pos = position_at_end_of_block(tr, id);
        tr.set_selection(Some(Selection::collapsed(final_pos)))?;
    }

    Ok(Some(selection_over_inserted_blocks(
        container_id,
        base_index,
        blocks.len(),
    )))
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
    matches!(
        (slice_node, doc_node.node()),
        (PlainNode::Paragraph(_), Node::Paragraph(_))
    )
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_model::{Fragment, PlainNode, PlainParagraphNode, PlainRootNode, PlainTextNode};

    use super::*;

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: text.into(),
                    }))],
                }],
            },
            open_start: 2,
            open_end: 2,
        }
    }

    fn paragraph_fragment(text: &str) -> Fragment {
        Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            style: None,
            children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
        }
    }

    #[test]
    fn empty_slice_helper_recognises_bare_container() {
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn is_inline_only_classifies_single_paragraph_slice() {
        let slice = root_with_paragraph("XY");
        assert!(is_inline_only(&slice));
    }

    #[test]
    fn is_inline_only_classifies_multi_paragraph_slice() {
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![paragraph_fragment("a"), paragraph_fragment("b")],
            },
            open_start: 2,
            open_end: 2,
        };
        assert!(!is_inline_only(&slice));
    }
}

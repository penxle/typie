use editor_crdt::sequence::Bias as SeqBias;
use editor_crdt::{Dot, ListOp, LwwRegOp};
use editor_model::{
    Anchor, AtomLeaf, Bias, Child, EditOp, Marker, Modifier, ModifierAttrOp, ModifierType,
    NodeAttrOp, NodeLwwOp, NodeType, PlainNode, PlainTextNode, SeqClass, SeqItem, SpanOp, Subtree,
    classify,
};
use editor_state::{BatchedState, ProjectedState};

use crate::{Step, StepError};

pub(crate) fn block_node_type(ps: &ProjectedState, block: Dot) -> Option<NodeType> {
    ps.block_node_type(block)
}

pub(crate) fn children_count(ps: &ProjectedState, block: Dot) -> Option<usize> {
    ps.child_count(block)
}

pub(crate) fn child_block_ids(ps: &ProjectedState, block: Dot) -> Vec<Dot> {
    ps.child_block_dots(block)
}

/// All children of `block` (chars, atom leaves, and nested blocks) as dots,
/// in order. Block-level atoms (Image/HR/…) project as Child::Leaf, so this is
/// the addressing the command layer uses (full child-slot index = offset),
/// distinct from `child_block_ids` which is blocks-only.
pub(crate) fn child_elem_ids(ps: &ProjectedState, block: Dot) -> Vec<Dot> {
    ps.child_elem_dots(block)
}

/// Seq position to insert at a full child-slot index of `block` (blocks + atom
/// leaves + chars). Inserts AFTER the full extent of the preceding child: after
/// a preceding block's whole subtree, or after a preceding leaf's single dot.
pub(crate) fn child_seq_insert_pos(
    ps: &ProjectedState,
    parent: Dot,
    index: usize,
) -> Result<usize, StepError> {
    let children = child_elem_ids(ps, parent);
    if index > children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent,
            index,
            len: children.len(),
        });
    }
    if index == 0 {
        return seq_insert_pos(ps, parent, 0).ok_or(StepError::NodeNotFound(parent));
    }
    let prev = children[index - 1];
    // Insert right after the previous sibling's last sequence element. For a block
    // sibling that's its subtree's max position — found in `O(depth)`, not by
    // resolving every dot in the subtree (the per-block-insert cost of a paste).
    let max = if ps.is_block(prev) {
        ps.subtree_max_seq_pos(prev)
            .ok_or(StepError::NodeNotFound(prev))?
    } else {
        match prev.as_op_dot() {
            Some(d) => ps
                .seq_flat_pos(d.dot())
                .ok_or(StepError::NodeNotFound(prev))?,
            None => return Err(StepError::NodeNotFound(prev)),
        }
    };
    Ok(max + 1)
}

pub(crate) fn parents_chain(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    ps.is_block(block)
        .then(|| ps.ancestor_real_dots(block, false))
}

pub(crate) fn self_inclusive_parents(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    ps.is_block(block)
        .then(|| ps.ancestor_real_dots(block, true))
}

pub(crate) fn seq_insert_pos(ps: &ProjectedState, block: Dot, offset: usize) -> Option<usize> {
    let block_dot = block.as_op_dot();
    if block_dot.is_none() && block != Dot::ROOT {
        if offset == 0 {
            let first_child = ps.child_dot_at(block, 0)?;
            return ps.seq_boundary_pos(first_child, SeqBias::Before);
        } else {
            let child = ps.child_dot_at(block, offset - 1)?;
            return ps.seq_boundary_pos(child, SeqBias::After);
        }
    }
    if offset == 0 {
        return match block_dot {
            Some(d) => ps.seq_boundary_pos(d.dot(), SeqBias::After),
            None => Some(0),
        };
    }
    let child = ps.child_dot_at(block, offset - 1)?;
    ps.seq_boundary_pos(child, SeqBias::After)
}

pub(crate) fn subtree_dots(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    ps.is_block(block).then(|| ps.subtree_real_dots(block))
}

/// Seq position at which to insert a new block as the `block_index`-th block
/// child of `parent`, accounting for nested subtrees (insert after the entire
/// preceding sibling subtree, not just its token).
pub(crate) fn block_seq_insert_pos(
    ps: &ProjectedState,
    parent: Dot,
    block_index: usize,
) -> Result<usize, StepError> {
    let children = child_block_ids(ps, parent);
    if block_index > children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent,
            index: block_index,
            len: children.len(),
        });
    }
    if block_index == 0 {
        return seq_insert_pos(ps, parent, 0).ok_or(StepError::NodeNotFound(parent));
    }
    let prev = children[block_index - 1];
    let max = ps
        .subtree_max_seq_pos(prev)
        .ok_or(StepError::NodeNotFound(prev))?;
    Ok(max + 1)
}

pub(crate) fn insert_text_ops(
    ps: &ProjectedState,
    block: Dot,
    offset: usize,
    text: &str,
) -> Result<Vec<EditOp>, StepError> {
    let len = children_count(ps, block).ok_or(StepError::NodeNotFound(block))?;
    let p = seq_insert_pos(ps, block, offset).ok_or(StepError::OffsetOutOfBounds {
        block,
        offset,
        len,
    })?;
    let mut ops = Vec::with_capacity(text.chars().count());
    for (i, ch) in text.chars().enumerate() {
        ops.push(EditOp::Seq(ListOp::Ins {
            pos: p + i,
            item: SeqItem::Char(ch),
        }));
    }
    Ok(ops)
}

pub(crate) fn char_leaf_dots_for_text(
    ps: &ProjectedState,
    block: Dot,
    offset: usize,
    text: &str,
) -> Result<Vec<Dot>, StepError> {
    let children = ps
        .block_children(block)
        .ok_or(StepError::NodeNotFound(block))?;
    let mut dots = Vec::with_capacity(text.chars().count());
    for (i, expected) in text.chars().enumerate() {
        let idx = offset + i;
        let child = children.get(idx).ok_or(StepError::OffsetOutOfBounds {
            block,
            offset: idx,
            len: children.len(),
        })?;
        match child {
            Child::Leaf {
                id,
                item: SeqItem::Char(actual),
            } if *actual == expected => dots.push(*id),
            Child::Leaf {
                item: SeqItem::Char(actual),
                ..
            } => {
                return Err(StepError::TextMismatch {
                    block,
                    offset: idx,
                    expected,
                    actual: *actual,
                });
            }
            _ => return Err(StepError::ExpectedText { block, offset: idx }),
        }
    }
    Ok(dots)
}

pub(crate) fn delete_dots_ops(ps: &ProjectedState, dots: &[Dot]) -> Vec<EditOp> {
    let mut positions: Vec<usize> = dots.iter().filter_map(|&d| ps.seq_flat_pos(d)).collect();
    // Descending, so deleting a run never shifts the positions of runs still to come.
    positions.sort_unstable_by(|a, b| b.cmp(a));
    positions.dedup();
    // Coalesce a maximal run of consecutive positions into ONE range `Del { pos, len }`.
    // Deleting a large selection is then `O(runs)` sequence ops (one for a contiguous
    // text run) instead of one op per character — each op otherwise pays the full
    // per-op projection + transaction overhead, which made select-all-delete `O(N²)`.
    let mut ops = Vec::new();
    let mut i = 0;
    while i < positions.len() {
        let mut j = i;
        while j + 1 < positions.len() && positions[j + 1] == positions[j] - 1 {
            j += 1;
        }
        let run_min = positions[j];
        ops.push(EditOp::Seq(ListOp::Del {
            pos: run_min,
            len: j - i + 1,
        }));
        i = j + 1;
    }
    ops
}

pub(crate) fn read_text(
    ps: &ProjectedState,
    block: Dot,
    offset: usize,
    len: usize,
) -> Result<String, StepError> {
    let children = ps
        .block_children(block)
        .ok_or(StepError::NodeNotFound(block))?;
    if offset > children.len() {
        return Err(StepError::OffsetOutOfBounds {
            block,
            offset,
            len: children.len(),
        });
    }
    let mut text = String::new();
    for i in 0..len {
        let idx = offset + i;
        let child = children.get(idx).ok_or(StepError::OffsetOutOfBounds {
            block,
            offset: idx,
            len: children.len(),
        })?;
        match child {
            Child::Leaf {
                item: SeqItem::Char(ch),
                ..
            } => text.push(*ch),
            _ => return Err(StepError::ExpectedText { block, offset: idx }),
        }
    }
    Ok(text)
}

pub(crate) fn remove_child_slots_steps(
    ps: &ProjectedState,
    parent: Dot,
    from: usize,
    to: usize,
) -> Result<Vec<Step>, StepError> {
    let children = ps
        .block_children(parent)
        .ok_or(StepError::NodeNotFound(parent))?;
    if from > to {
        return Err(StepError::InvalidChildRange { parent, from, to });
    }
    if from > children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent,
            index: from,
            len: children.len(),
        });
    }
    if to > children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent,
            index: to,
            len: children.len(),
        });
    }
    if to <= from {
        return Ok(Vec::new());
    }

    let mut steps = Vec::new();
    let mut text_offset = None;
    let mut text = String::new();

    for (idx, child) in children.iter().enumerate().skip(from).take(to - from) {
        match child {
            Child::Leaf {
                item: SeqItem::Char(ch),
                ..
            } => {
                if text_offset.is_none() {
                    text_offset = Some(idx);
                }
                text.push(*ch);
            }
            Child::Leaf { id, item } => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                let atom = match item {
                    SeqItem::Atom(atom) => atom,
                    SeqItem::BlockAtom { leaf, .. } => leaf,
                    SeqItem::Char(_) | SeqItem::Block { .. } => {
                        return Err(StepError::InvalidChildSlot { parent, index: idx });
                    }
                };
                steps.push(Step::RemoveSubtree {
                    parent,
                    index: idx,
                    subtree: atom_leaf_subtree(ps, *id, atom.clone()),
                });
            }
            Child::Block(block) => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                let subtree = capture_subtree(ps, *block).ok_or(StepError::NodeNotFound(*block))?;
                steps.push(Step::RemoveSubtree {
                    parent,
                    index: idx,
                    subtree,
                });
            }
        }
    }
    flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
    steps.reverse();
    Ok(steps)
}

fn flush_remove_text_step(
    steps: &mut Vec<Step>,
    block: Dot,
    offset: &mut Option<usize>,
    text: &mut String,
) {
    if let Some(offset) = offset.take()
        && !text.is_empty()
    {
        steps.push(Step::RemoveText {
            block,
            offset,
            text: std::mem::take(text),
        });
    }
}

pub(crate) fn span_add(first: Dot, last: Dot, modifier: Modifier) -> EditOp {
    EditOp::Span(SpanOp::AddSpan {
        start: Anchor {
            id: first,
            bias: Bias::Before,
        },
        end: Anchor {
            id: last,
            bias: Bias::After,
        },
        modifier,
    })
}

pub(crate) fn span_remove(first: Dot, last: Dot, modifier_type: ModifierType) -> EditOp {
    EditOp::Span(SpanOp::RemoveSpan {
        start: Anchor {
            id: first,
            bias: Bias::Before,
        },
        end: Anchor {
            id: last,
            bias: Bias::After,
        },
        modifier_type,
    })
}

pub(crate) fn block_modifier_set(target: Dot, modifier: Modifier) -> EditOp {
    EditOp::BlockModifier(ModifierAttrOp::SetModifier { target, modifier })
}

/// Ops that copy a source block's overlays (block modifiers, node marker) onto
/// `target`. Used when a structural step mints a new block dot that should
/// inherit the source's block-level formatting (e.g. SplitNode).
pub(crate) fn block_overlay_ops(ps: &ProjectedState, src: Dot, target: Dot) -> Vec<EditOp> {
    let mut ops = Vec::new();
    for (_ty, m) in ps.block_modifiers().modifiers_of(src) {
        ops.push(block_modifier_set(target, m));
    }
    if let Some(marker) = ps.node_markers().value_of(src) {
        ops.push(node_marker_set(target, Some(marker)));
    }
    ops
}

pub(crate) fn block_modifier_clear(target: Dot, key: ModifierType) -> EditOp {
    EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key })
}

pub(crate) fn node_marker_set(target: Dot, value: Option<Marker>) -> EditOp {
    EditOp::NodeMarker(NodeLwwOp {
        target,
        op: LwwRegOp::Set { value },
    })
}

/// Builds a `Subtree` snapshot of the projected block at `block`. Char leaves
/// are grouped into plain `Text` subtrees (per-char span styling is dropped —
/// the M1 move/subtree path is structural). Atom leaves keep their own
/// modifiers. Nested blocks recurse.
pub fn capture_subtree(ps: &ProjectedState, block: Dot) -> Option<Subtree> {
    let node = ps.block_node(block)?.to_plain();
    let dot = editor_model::anchor_dot(block);
    let modifiers: Vec<Modifier> = dot
        .map(|d| {
            ps.block_modifiers()
                .modifiers_of(d)
                .values()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let marker = dot.and_then(|d| ps.node_markers().value_of(d));

    let mut children: Vec<Subtree> = Vec::new();
    let mut pending_text = String::new();
    for c in ps.block_children(block)? {
        match c {
            Child::Leaf { id, item } => match item {
                SeqItem::Char(ch) => pending_text.push(ch),
                SeqItem::Atom(atom) => {
                    if !pending_text.is_empty() {
                        children.push(text_subtree(std::mem::take(&mut pending_text)));
                    }
                    children.push(atom_leaf_subtree(ps, id, atom));
                }
                SeqItem::BlockAtom { leaf, .. } => {
                    if !pending_text.is_empty() {
                        children.push(text_subtree(std::mem::take(&mut pending_text)));
                    }
                    children.push(atom_leaf_subtree(ps, id, leaf));
                }
                _ => {}
            },
            Child::Block(d) => {
                if !pending_text.is_empty() {
                    children.push(text_subtree(std::mem::take(&mut pending_text)));
                }
                if let Some(sub) = capture_subtree(ps, d) {
                    children.push(sub);
                }
            }
        }
    }
    if !pending_text.is_empty() {
        children.push(text_subtree(pending_text));
    }

    Some(Subtree {
        node,
        modifiers,
        marker,
        children,
    })
}

fn text_subtree(text: String) -> Subtree {
    Subtree::leaf(PlainNode::Text(PlainTextNode { text }))
}

fn atom_leaf_subtree(ps: &ProjectedState, dot: Dot, atom: AtomLeaf) -> Subtree {
    Subtree {
        node: atom.into_node().to_plain(),
        modifiers: ps.view().leaf_own_modifiers_by_dot_slow(dot),
        marker: None,
        children: Vec::new(),
    }
}

/// Emits a captured/described subtree into the working state starting at
/// `seq_pos`, beneath the chain `parents` (root-incl/self-excl for the subtree
/// root). Returns the dot of the subtree's root block, if any.
pub(crate) fn emit_subtree(
    batched: &mut BatchedState,
    subtree: &Subtree,
    parents: &[Dot],
    seq_pos: &mut usize,
) -> Result<Option<Dot>, StepError> {
    let node_type = subtree.node.as_type();
    match classify(node_type) {
        SeqClass::Block => {
            let dot = batched
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: *seq_pos,
                    item: SeqItem::Block {
                        node_type,
                        parents: parents.to_vec(),
                    },
                }))?
                .id;
            *seq_pos += 1;
            for modifier in &subtree.modifiers {
                batched.apply(block_modifier_set(dot, modifier.clone()))?;
            }
            if let Some(marker) = &subtree.marker {
                batched.apply(node_marker_set(dot, Some(marker.clone())))?;
            }
            for attr in subtree.node.to_attrs() {
                batched.apply(EditOp::NodeAttr(NodeAttrOp { target: dot, attr }))?;
            }
            let mut child_parents = parents.to_vec();
            child_parents.push(dot);
            for child in &subtree.children {
                emit_subtree(batched, child, &child_parents, seq_pos)?;
            }
            Ok(Some(dot))
        }
        SeqClass::Text => {
            if let PlainNode::Text(PlainTextNode { text }) = &subtree.node {
                let mut char_dots = Vec::with_capacity(text.chars().count());
                for ch in text.chars() {
                    let d = batched
                        .apply(EditOp::Seq(ListOp::Ins {
                            pos: *seq_pos,
                            item: SeqItem::Char(ch),
                        }))?
                        .id;
                    *seq_pos += 1;
                    char_dots.push(d);
                }
                if let (Some(&first), Some(&last)) = (char_dots.first(), char_dots.last()) {
                    for modifier in &subtree.modifiers {
                        batched.apply(span_add(first, last, modifier.clone()))?;
                    }
                }
            }
            Ok(None)
        }
        SeqClass::Atom => {
            let leaf = AtomLeaf::from_plain_node(&subtree.node)
                .ok_or(StepError::NodeNotFound(Dot::ROOT))?;
            let item = if leaf.is_block_level() {
                SeqItem::BlockAtom {
                    leaf,
                    parents: parents.to_vec(),
                }
            } else {
                SeqItem::Atom(leaf)
            };
            let dot = batched
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: *seq_pos,
                    item,
                }))?
                .id;
            *seq_pos += 1;
            for modifier in &subtree.modifiers {
                batched.apply(span_add(dot, dot, modifier.clone()))?;
            }
            Ok(Some(dot))
        }
    }
}

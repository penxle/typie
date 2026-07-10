use std::collections::BTreeMap;

use editor_crdt::sequence::Bias as SeqBias;
use editor_crdt::{Dot, ListOp};
use editor_model::{
    AliasRun, Anchor, AtomLeaf, Bias, Child, EditOp, Modifier, ModifierAttrOp, ModifierType,
    NodeType, PlainNode, PlainTextNode, SeqClass, SeqItem, SpanOp, Subtree, classify,
};
use editor_state::{BatchedState, ProjectedState};

use crate::{Step, StepError};

pub(crate) fn block_node_type(ps: &ProjectedState, block: Dot) -> Option<NodeType> {
    ps.block_node_type(block)
}

pub(crate) fn children_count(ps: &ProjectedState, block: Dot) -> Option<usize> {
    ps.child_count(block)
}

/// All children of `block` (chars, atom leaves, and nested blocks) as dots,
/// in order. Block-level atoms (Image/HR/…) project as Child::Leaf, so this is
/// the addressing the command layer uses (full child-slot index = offset).
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
    if ps.is_block(block) {
        return Some(ps.subtree_real_dots(block));
    }
    ps.view()
        .leaf(block)
        .and_then(|leaf| leaf.as_atom())
        .map(|_| vec![block])
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
    let mut opaque_dots: Vec<Dot> = Vec::new();

    for (idx, child) in children.iter().enumerate().skip(from).take(to - from) {
        match child {
            Child::Leaf {
                item: SeqItem::Char(ch),
                ..
            } => {
                flush_opaque_step(&mut steps, &mut opaque_dots);
                if text_offset.is_none() {
                    text_offset = Some(idx);
                }
                text.push(*ch);
            }
            // A slot that carries lossy/unrepresentable content cannot be
            // captured into a `Subtree` (no lossless Plain form, and
            // reinserting one via `emit_subtree` would synthesize a new
            // carrier dot) — route it to a position-based delete-only step
            // instead (`Step::DeleteOpaque`, inverse = dot-based `Undel`).
            Child::Leaf { id, item } if item.is_unknown_bearing() => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                opaque_dots.push(*id);
            }
            Child::Leaf { id, item } => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                flush_opaque_step(&mut steps, &mut opaque_dots);
                let atom = match item {
                    SeqItem::Atom(atom) => atom,
                    SeqItem::BlockAtom { leaf, .. } => leaf,
                    SeqItem::Char(_) | SeqItem::Block { .. } | SeqItem::Unknown { .. } => {
                        return Err(StepError::InvalidChildSlot { parent, index: idx });
                    }
                };
                steps.push(Step::RemoveSubtree {
                    parent,
                    index: idx,
                    subtree: atom_leaf_subtree(ps, *id, atom.clone()),
                });
            }
            Child::Block(block) if block_node_type(ps, *block) == Some(NodeType::Unknown) => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                opaque_dots.extend(subtree_dots(ps, *block).unwrap_or_default());
            }
            Child::Block(block) => {
                flush_remove_text_step(&mut steps, parent, &mut text_offset, &mut text);
                flush_opaque_step(&mut steps, &mut opaque_dots);
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
    flush_opaque_step(&mut steps, &mut opaque_dots);
    steps.reverse();
    Ok(steps)
}

fn flush_opaque_step(steps: &mut Vec<Step>, dots: &mut Vec<Dot>) {
    if !dots.is_empty() {
        steps.push(Step::DeleteOpaque {
            dots: std::mem::take(dots),
            emitted: Vec::new(),
        });
    }
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

/// Ops that copy a source block's block modifiers onto `target`. Used when a
/// structural step mints a new block dot that should inherit the source's
/// block-level formatting (e.g. SplitNode).
pub(crate) fn block_overlay_ops(ps: &ProjectedState, src: Dot, target: Dot) -> Vec<EditOp> {
    let mut ops = Vec::new();
    for (_ty, m) in ps.block_modifiers().modifiers_of(src) {
        ops.push(block_modifier_set(target, m));
    }
    ops
}

pub(crate) fn block_modifier_clear(target: Dot, key: ModifierType) -> EditOp {
    EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key })
}

pub(crate) fn node_carry_set(target: Dot, modifier: Modifier) -> EditOp {
    EditOp::NodeCarry(ModifierAttrOp::SetModifier { target, modifier })
}

pub(crate) fn node_carry_clear(target: Dot, key: ModifierType) -> EditOp {
    EditOp::NodeCarry(ModifierAttrOp::ClearModifier { target, key })
}

/// Builds a `Subtree` snapshot of the projected block at `block`. Consecutive
/// char leaves that share the same own paint collapse into one `Text` subtree
/// carrying that run's own modifiers; a paint change starts a new run. Atom
/// leaves keep their own modifiers. Nested blocks recurse.
pub fn capture_subtree(ps: &ProjectedState, block: Dot) -> Option<Subtree> {
    if let Some(leaf) = ps.view().leaf(block)
        && let Some(atom) = leaf.as_atom()
    {
        return Some(atom_leaf_subtree(ps, block, atom.clone()));
    }
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
    let carry: Vec<Modifier> = dot
        .map(|d| ps.node_carries().modifiers_of(d).into_values().collect())
        .unwrap_or_default();

    let leaf_own: Vec<Vec<Modifier>> = {
        let view = ps.view();
        match view.node(block) {
            Some(nv) => nv
                .inline()
                .iter()
                .map(|it| it.own_modifiers.values().map(|o| o.value.clone()).collect())
                .collect(),
            None => Vec::new(),
        }
    };

    let mut children: Vec<Subtree> = Vec::new();
    let mut pending_text = String::new();
    let mut pending_dots: Vec<Dot> = Vec::new();
    let mut pending_mods: Vec<Modifier> = Vec::new();
    let mut leaf_idx = 0usize;
    for c in ps.block_children(block)? {
        match c {
            Child::Leaf { id, item } => {
                let own = leaf_own.get(leaf_idx).cloned().unwrap_or_default();
                leaf_idx += 1;
                match item {
                    SeqItem::Char(ch) => {
                        if !pending_text.is_empty() && own != pending_mods {
                            children.push(text_subtree(
                                std::mem::take(&mut pending_text),
                                std::mem::take(&mut pending_dots),
                                std::mem::take(&mut pending_mods),
                            ));
                        }
                        pending_text.push(ch);
                        pending_dots.push(id);
                        pending_mods = own;
                    }
                    SeqItem::Atom(atom) => {
                        if !pending_text.is_empty() {
                            children.push(text_subtree(
                                std::mem::take(&mut pending_text),
                                std::mem::take(&mut pending_dots),
                                std::mem::take(&mut pending_mods),
                            ));
                        }
                        children.push(atom_leaf_subtree(ps, id, atom));
                    }
                    SeqItem::BlockAtom { leaf, .. } => {
                        if !pending_text.is_empty() {
                            children.push(text_subtree(
                                std::mem::take(&mut pending_text),
                                std::mem::take(&mut pending_dots),
                                std::mem::take(&mut pending_mods),
                            ));
                        }
                        children.push(atom_leaf_subtree(ps, id, leaf));
                    }
                    _ => {}
                }
            }
            Child::Block(d) => {
                if !pending_text.is_empty() {
                    children.push(text_subtree(
                        std::mem::take(&mut pending_text),
                        std::mem::take(&mut pending_dots),
                        std::mem::take(&mut pending_mods),
                    ));
                }
                if let Some(sub) = capture_subtree(ps, d) {
                    children.push(sub);
                }
            }
        }
    }
    if !pending_text.is_empty() {
        children.push(text_subtree(pending_text, pending_dots, pending_mods));
    }

    Some(Subtree {
        node,
        modifiers,
        carry,
        children,
        source_dots: if block.is_synthetic() {
            Vec::new()
        } else {
            vec![block]
        },
    })
}

fn text_subtree(text: String, dots: Vec<Dot>, modifiers: Vec<Modifier>) -> Subtree {
    Subtree {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers,
        carry: Vec::new(),
        children: Vec::new(),
        source_dots: dots,
    }
}

fn atom_leaf_subtree(ps: &ProjectedState, dot: Dot, atom: AtomLeaf) -> Subtree {
    let modifiers = if atom.is_block_level() {
        ps.block_modifiers()
            .modifiers_of(dot)
            .into_values()
            .collect()
    } else {
        ps.view().leaf_own_modifiers_by_dot_slow(dot)
    };
    Subtree {
        node: atom.into_node().to_plain(),
        modifiers,
        carry: Vec::new(),
        children: Vec::new(),
        source_dots: if dot.is_synthetic() {
            Vec::new()
        } else {
            vec![dot]
        },
    }
}

/// Recursively scans the *projected* subtree rooted at `block` for any of the
/// lossy shapes `remove_child_slots_steps` routes to `Step::DeleteOpaque`: an
/// unknown-bearing leaf, or a `NodeType::Unknown` placeholder block (its whole
/// subtree, since the block's children are only structurally attached). Must
/// run against the live projection, not a captured `Subtree` — capture already
/// drops unknown content, so by then it's undetectable.
pub(crate) fn subtree_has_unknown(ps: &ProjectedState, block: Dot) -> bool {
    if let Some(leaf) = ps.view().leaf(block) {
        return leaf.item().is_unknown_bearing();
    }
    if block_node_type(ps, block) == Some(NodeType::Unknown) {
        return true;
    }
    let Some(children) = ps.block_children(block) else {
        return false;
    };
    children.iter().any(|c| match c {
        Child::Leaf { item, .. } => item.is_unknown_bearing(),
        Child::Block(d) => subtree_has_unknown(ps, *d),
    })
}

/// Coalesces `(old, new)` dot pairs from an `emit_subtree` walk into maximal
/// `AliasRun`s: consecutive when both the old and new dots advance by one
/// clock tick on the same actor. A discontinuity on either side (an
/// interleaved overlay op consuming a clock tick, or a non-contiguous source)
/// starts a new run.
pub(crate) fn compress_alias_pairs(pairs: &[(Dot, Dot)]) -> Vec<AliasRun> {
    let mut runs: Vec<AliasRun> = Vec::new();
    for &(old, new) in pairs {
        if let Some(last) = runs.last_mut()
            && last.len < u32::MAX
            && old.actor == last.old_start.actor
            && new.actor == last.new_start.actor
            && old.clock == last.old_start.clock + last.len as u64
            && new.clock == last.new_start.clock + last.len as u64
        {
            last.len += 1;
            continue;
        }
        runs.push(AliasRun {
            old_start: old,
            len: 1,
            new_start: new,
        });
    }
    runs
}

/// Emits a captured/described subtree into the working state starting at
/// `seq_pos`, beneath the chain `parents` (root-incl/self-excl for the subtree
/// root). Returns the dot of the subtree's root block, if any. Every dot this
/// call mints is paired with the `source_dots` entry it replaces (if any) by
/// pushing onto `pairs` — recursive calls for nested children must be passed
/// the same `pairs` accumulator, or their pairs are lost.
pub(crate) fn emit_subtree(
    batched: &mut BatchedState,
    subtree: &Subtree,
    parents: &[Dot],
    seq_pos: &mut usize,
    pairs: &mut Vec<(Dot, Dot)>,
) -> Result<Option<Dot>, StepError> {
    if subtree.node == PlainNode::Unknown {
        return Err(StepError::UnknownSubtree);
    }
    let node_type = subtree.node.as_type();
    match classify(node_type) {
        SeqClass::Block => {
            if node_type == NodeType::Root {
                return Err(StepError::RootSubtree);
            }
            let dot = batched
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: *seq_pos,
                    item: SeqItem::Block {
                        node_type,
                        parents: parents.to_vec(),
                        attrs: subtree.node.to_attrs(),
                    },
                }))?
                .id;
            *seq_pos += 1;
            if let Some(&old) = subtree.source_dots.first() {
                pairs.push((old, dot));
            }
            for modifier in &subtree.modifiers {
                batched.apply(block_modifier_set(dot, modifier.clone()))?;
            }
            let mut carry_by_type: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
            for m in &subtree.carry {
                if m.as_type().is_carry_kind() {
                    carry_by_type.insert(m.as_type(), m.clone());
                }
            }
            for (_ty, m) in carry_by_type {
                batched.apply(node_carry_set(dot, m))?;
            }
            let mut child_parents = parents.to_vec();
            child_parents.push(dot);
            for child in &subtree.children {
                emit_subtree(batched, child, &child_parents, seq_pos, pairs)?;
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
                if subtree.source_dots.len() == char_dots.len() {
                    for (&old, &new) in subtree.source_dots.iter().zip(char_dots.iter()) {
                        pairs.push((old, new));
                    }
                }
                if let (Some(&first), Some(&last)) = (char_dots.first(), char_dots.last()) {
                    for modifier in &subtree.modifiers {
                        if modifier.as_type().is_text_applicable() {
                            batched.apply(span_add(first, last, modifier.clone()))?;
                        }
                    }
                }
            }
            Ok(None)
        }
        SeqClass::Atom => {
            let leaf = AtomLeaf::from_plain_node(&subtree.node)
                .ok_or(StepError::NodeNotFound(Dot::ROOT))?;
            let is_block_level = leaf.is_block_level();
            let item = if is_block_level {
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
            if let Some(&old) = subtree.source_dots.first() {
                pairs.push((old, dot));
            }
            for modifier in &subtree.modifiers {
                let ty = modifier.as_type();
                if is_block_level {
                    batched.apply(block_modifier_set(dot, modifier.clone()))?;
                } else if ty.is_text_applicable()
                    && !matches!(ty, ModifierType::Link | ModifierType::Ruby)
                {
                    batched.apply(span_add(dot, dot, modifier.clone()))?;
                }
            }
            Ok(Some(dot))
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    fn p(actor: u64, clock: u64) -> Dot {
        Dot::new(actor, clock)
    }

    #[test]
    fn compress_alias_pairs_splits_on_old_discontinuity() {
        let pairs = vec![
            (p(1, 0), p(9, 100)),
            (p(1, 1), p(9, 101)),
            (p(2, 7), p(9, 102)),
        ];
        let runs = compress_alias_pairs(&pairs);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].len, 2);
        assert_eq!(runs[1].old_start, p(2, 7));
    }

    #[test]
    fn compress_alias_pairs_splits_on_new_discontinuity() {
        let pairs = vec![(p(1, 0), p(9, 100)), (p(1, 1), p(9, 102))];
        let runs = compress_alias_pairs(&pairs);
        assert_eq!(runs.len(), 2, "new 불연속에서도 분할");
        assert_eq!(runs[1].new_start, p(9, 102));
    }

    /// A block-level atom leaf (e.g. a top-level image) is represented as a
    /// projected leaf rather than a block. Exercise its capture and emit
    /// pairing directly against `support`, including the block-modifier branch.
    #[test]
    fn emit_subtree_pairs_block_atom_source_dot() {
        let (state, root) = state! {
            doc { root: root {
                image
                paragraph { text("a") }
            } }
            selection: (root, 0)
        };
        let ps = &state.projected;
        let (image_dot, atom) = ps
            .block_children(root)
            .unwrap()
            .into_iter()
            .find_map(|c| match c {
                Child::Leaf {
                    id,
                    item: SeqItem::Atom(leaf),
                } if leaf.is_block_level() => Some((id, leaf)),
                _ => None,
            })
            .expect("image leaf present at root");

        let subtree = atom_leaf_subtree(ps, image_dot, atom);
        assert_eq!(subtree.source_dots, vec![image_dot]);

        let parents = self_inclusive_parents(ps, root).unwrap();
        let mut seq_pos = child_seq_insert_pos(ps, root, 2).unwrap();
        let mut pairs: Vec<(Dot, Dot)> = Vec::new();
        let mut new_dot = None;
        state
            .batch::<_, StepError>(|batched| {
                new_dot = emit_subtree(batched, &subtree, &parents, &mut seq_pos, &mut pairs)?;
                Ok(())
            })
            .unwrap();

        assert_eq!(pairs, vec![(image_dot, new_dot.unwrap())]);
    }
}

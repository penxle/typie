use std::collections::BTreeMap;

use editor_crdt::sequence::Bias as SeqBias;
use editor_crdt::{Dot, ListOp};
use editor_model::{
    AliasRun, Anchor, AtomLeaf, Bias, Child, EditOp, Modifier, ModifierAttrOp, ModifierType,
    NodeType, PlainNode, PlainTextNode, Schema, SeqClass, SeqItem, SpanOp, Subtree, anchor_dot,
    classify, is_fixed_slot,
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
/// `t` is the type of the node about to be inserted, validated against `parent`.
pub(crate) fn child_seq_insert_pos(
    ps: &ProjectedState,
    parent: Dot,
    index: usize,
    t: NodeType,
) -> Result<usize, StepError> {
    let children = child_elem_ids(ps, parent);
    if index > children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent,
            index,
            len: children.len(),
        });
    }
    let pos = if index == 0 {
        seq_insert_pos(ps, parent, 0).ok_or(StepError::NodeNotFound(parent))?
    } else {
        let prev = children[index - 1];
        // Insert right after the previous sibling's last sequence element. For a
        // block sibling that's its subtree's exact max position — a rightmost-path
        // walk under-reports it across a reversed fixed-slot (Fold), landing the
        // insert inside the container.
        let max = if ps.is_block(prev) {
            subtree_seq_max_exact(ps, prev).ok_or(StepError::NodeNotFound(prev))?
        } else {
            match prev.as_op_dot() {
                Some(d) => ps
                    .seq_flat_pos(d.dot())
                    .ok_or(StepError::NodeNotFound(prev))?,
                None => return Err(StepError::NodeNotFound(prev)),
            }
        };
        max + 1
    };
    validate_ins_slot(ps, parent, pos, index, t, None)?;
    Ok(pos)
}

/// The maximum sequence position spanned by `block`'s subtree. Mirrors
/// `ProjectedState::subtree_max_seq_pos`'s rightmost-path walk, but the instant
/// the walk meets a fixed-slot node (`Fold`) it switches to an exhaustive scan of
/// that node's subtree. A fixed-slot container reorders its children by role, so
/// its tree-last child is not its sequence-max — a reversed `Fold`
/// (`FoldContent` before `FoldTitle` in the sequence) hides its real max in the
/// tree-first slot, which the rightmost walk would miss. Every non-fixed parent
/// keeps its children in sequence order, so the max stays in the tree-last child
/// until such a container is reached — including one nested inside the rightmost
/// path of non-fixed ancestors.
pub(crate) fn subtree_seq_max_exact(ps: &ProjectedState, block: Dot) -> Option<usize> {
    let exhaustive = |b: Dot| {
        ps.subtree_real_dots(b)
            .iter()
            .filter_map(|&d| ps.seq_flat_pos(d))
            .max()
    };
    let mut node = block;
    loop {
        let t = ps.block_node_type(node)?;
        if is_fixed_slot(t) {
            return exhaustive(node);
        }
        let count = match ps.child_count(node) {
            Some(c) => c,
            None => break,
        };
        if count == 0 {
            return match anchor_dot(node).and_then(|d| ps.seq_flat_pos(d)) {
                Some(pos) => Some(pos),
                None => exhaustive(block),
            };
        }
        let last = ps.child_dot_at(node, count - 1)?;
        if ps.is_block(last) {
            node = last;
        } else {
            return match ps.seq_flat_pos(last) {
                Some(pos) => Some(pos),
                None => exhaustive(block),
            };
        }
    }
    exhaustive(block)
}

/// The single validation gate for a locally generated `Seq(Ins)` into `block` at
/// sequence position `pos`. Rejects with `IllegalInsertSlot` when the position
/// falls outside `block`'s own span, when `t` is not schema-legal content for
/// `block`, or — when `parents` is `Some` (a block-marker insert) — when the
/// supplied tree-parent chain is stale relative to `block`'s live ancestry.
/// `offset` is the child-slot offset the caller derived `pos` from; a run of
/// chars only needs its first slot checked, so it is carried for the contract
/// but not re-derived here (positional repair is a later concern).
pub(crate) fn validate_ins_slot(
    ps: &ProjectedState,
    block: Dot,
    pos: usize,
    offset: usize,
    t: NodeType,
    parents: Option<&[Dot]>,
) -> Result<(), StepError> {
    let _ = offset;
    let err = || StepError::IllegalInsertSlot { block, pos };

    let start = match block.as_op_dot() {
        Some(d) => ps
            .seq_boundary_pos(d.dot(), SeqBias::After)
            .ok_or_else(err)?,
        None if block == Dot::ROOT => 0,
        // synthetic block: fail-closed if its real anchor doesn't resolve. A legal
        // insert targets a real block, materialized out of any scaffold first.
        None => ps
            .child_dot_at(block, 0)
            .and_then(|c| c.as_op_dot().map(|d| d.dot()))
            .and_then(|c| ps.seq_boundary_pos(c, SeqBias::Before))
            .ok_or_else(err)?,
    };
    let end = subtree_seq_max_exact(ps, block).map_or(start, |m| m + 1);
    if !(start..=end).contains(&pos) {
        return Err(err());
    }

    let bt = ps.block_node_type(block).ok_or_else(err)?;
    if !Schema::node_spec(bt).content.matches(t) {
        return Err(err());
    }

    if let Some(parents) = parents {
        let chain: Vec<Dot> = ps.ancestor_real_dots(block, true);
        if parents != chain.as_slice() {
            return Err(err());
        }
    }
    Ok(())
}

/// Preorder type-check of an arbitrary `Subtree` about to be emitted beneath a
/// `host`-typed parent. Each node's type must be legal content for its parent
/// (the subtree root against `host`, every child against its own parent node).
/// A `Subtree` can be built freely, so `emit_subtree` runs this before mutating
/// any state — a rejection aborts before the first op, leaving the state
/// untouched.
fn validate_subtree_shape(sub: &Subtree, host: NodeType) -> Result<(), StepError> {
    let t = sub.node.as_type();
    if !Schema::node_spec(host).content.matches(t) {
        return Err(StepError::IllegalInsertSlot {
            block: Dot::ROOT,
            pos: 0,
        });
    }
    for child in &sub.children {
        validate_subtree_shape(child, t)?;
    }
    Ok(())
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
    validate_ins_slot(ps, block, p, offset, NodeType::Text, None)?;
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
    // The caller validated the entry slot; the shape below it is arbitrary, so
    // reject an illegal nesting before emitting anything. Each marker's tree
    // parents are woven from the tree structure here, so the parents-chain
    // criterion holds by construction and needs no per-marker chain check.
    if let Some(host) = parents
        .last()
        .and_then(|&p| batched.projected.block_node_type(p))
    {
        validate_subtree_shape(subtree, host)?;
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
    use editor_crdt::{Changeset, Op, OpGraph};
    use editor_macros::state;
    use editor_model::ChildView;
    use editor_state::State;
    use proptest::prelude::*;

    use super::*;
    use crate::Transaction;

    fn p(actor: u64, clock: u64) -> Dot {
        Dot::new(actor, clock)
    }

    #[test]
    fn insert_text_into_non_inline_block_rejects() {
        let (state, list, ..) = state! {
            doc {
                root {
                    list: bullet_list {
                        list_item { p: paragraph { text("ab") } }
                    }
                }
            }
            selection: (p, 0)
        };
        let mut tr = Transaction::new(&state);
        let err = tr.insert_text(list, 0, "x").unwrap_err();
        assert!(matches!(err, StepError::IllegalInsertSlot { .. }));
    }

    #[test]
    fn insert_text_into_paragraph_accepts_all_offsets() {
        let (state, p) = state! {
            doc {
                root {
                    p: paragraph { text("ab") page_break }
                }
            }
            selection: (p, 0)
        };
        for offset in 0..=3 {
            let mut tr = Transaction::new(&state);
            assert!(tr.insert_text(p, offset, "x").is_ok(), "offset {offset}");
        }
    }

    #[test]
    fn insert_pos_outside_block_span_is_illegal() {
        let (state, a) = state! {
            doc {
                root {
                    a: paragraph { text("ab") }
                    paragraph { text("cd") }
                }
            }
            selection: (a, 0)
        };
        let ps = &state.projected;
        let a_marker = ps.seq_flat_pos(a).unwrap();
        assert!(
            validate_ins_slot(ps, a, a_marker, 0, NodeType::Text, None).is_err(),
            "마커 슬롯"
        );
        assert!(validate_ins_slot(ps, a, a_marker + 1, 0, NodeType::Text, None).is_ok());
        assert!(
            validate_ins_slot(ps, a, a_marker + 3, 2, NodeType::Text, None).is_ok(),
            "말미"
        );
        assert!(
            validate_ins_slot(ps, a, a_marker + 4, 2, NodeType::Text, None).is_err(),
            "스팬 밖"
        );
    }

    #[test]
    fn marker_ins_with_stale_parents_chain_rejects() {
        let (state, _list, li, ..) = state! {
            doc {
                root {
                    list: bullet_list { li: list_item { p: paragraph { text("a") } } }
                }
            }
            selection: (p, 0)
        };
        let ps = &state.projected;
        let pos = child_seq_insert_pos(ps, li, 1, NodeType::Paragraph).unwrap();
        let good: Vec<Dot> = ps.ancestor_real_dots(li, true);
        assert!(validate_ins_slot(ps, li, pos, 1, NodeType::Paragraph, Some(&good)).is_ok());
        let stale = vec![Dot::ROOT, Dot::new(9, 9)];
        assert!(validate_ins_slot(ps, li, pos, 1, NodeType::Paragraph, Some(&stale)).is_err());
    }

    fn seq_block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block {
            node_type,
            parents,
            attrs: vec![],
        }
    }

    /// Weave raw changesets whose sequence order is exactly `items` (each appended
    /// in turn), on a deterministic actor so block markers can reference each
    /// other's dots by `(1, index)`. DAG parents are the linear chain `add_mut`
    /// derives from the running heads.
    fn weave_css(items: Vec<SeqItem>) -> Vec<Changeset<EditOp>> {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        for (pos, item) in items.into_iter().enumerate() {
            g.add_mut(EditOp::Seq(ListOp::Ins { pos, item })).unwrap();
        }
        g.commit_mut();
        g.changesets_as_vec()
    }

    /// A `Fold` whose sequence order is reversed relative to schema order:
    /// `[Fold, FoldContent(+Paragraph), FoldTitle(+char)]`. Projection reorders
    /// the children to `[FoldTitle, FoldContent]`, so the sequence-max hides in
    /// the tree-first slot.
    fn reversed_fold_css() -> Vec<Changeset<EditOp>> {
        let fold = Dot::new(1, 0);
        let content = Dot::new(1, 1);
        weave_css(vec![
            seq_block(NodeType::Fold, vec![Dot::ROOT]),
            seq_block(NodeType::FoldContent, vec![Dot::ROOT, fold]),
            seq_block(NodeType::Paragraph, vec![Dot::ROOT, fold, content]),
            seq_block(NodeType::FoldTitle, vec![Dot::ROOT, fold]),
            SeqItem::Char('t'),
        ])
    }

    /// The same reversed `Fold`, buried on the rightmost path of non-fixed
    /// ancestors (`Table > TableRow > TableCell > Fold`). A top-block-only
    /// fixed-slot check would miss it — the exhaustive switch must fire the moment
    /// the rightmost walk reaches the nested `Fold`.
    fn nested_reversed_fold_css() -> Vec<Changeset<EditOp>> {
        let table = Dot::new(1, 0);
        let row = Dot::new(1, 1);
        let cell = Dot::new(1, 2);
        let fold = Dot::new(1, 3);
        let content = Dot::new(1, 4);
        weave_css(vec![
            seq_block(NodeType::Table, vec![Dot::ROOT]),
            seq_block(NodeType::TableRow, vec![Dot::ROOT, table]),
            seq_block(NodeType::TableCell, vec![Dot::ROOT, table, row]),
            seq_block(NodeType::Fold, vec![Dot::ROOT, table, row, cell]),
            seq_block(
                NodeType::FoldContent,
                vec![Dot::ROOT, table, row, cell, fold],
            ),
            seq_block(
                NodeType::Paragraph,
                vec![Dot::ROOT, table, row, cell, fold, content],
            ),
            seq_block(NodeType::FoldTitle, vec![Dot::ROOT, table, row, cell, fold]),
            SeqItem::Char('t'),
        ])
    }

    #[test]
    fn insert_after_reversed_fold_preserves_all_dots() {
        let css = reversed_fold_css();
        let state = State::from_changesets(css, None).unwrap();
        let root_children = state.projected.child_elem_dots(Dot::ROOT);
        let fold = root_children[0];
        let before: Vec<Dot> = state.projected.subtree_real_dots(fold);

        let mut tr = Transaction::new(&state);
        tr.insert_subtree(
            Dot::ROOT,
            1,
            Subtree::leaf(NodeType::Paragraph.into_node().to_plain()),
        )
        .unwrap();

        let after_fold: Vec<Dot> = tr.state().projected.subtree_real_dots(fold);
        assert_eq!(
            before.len(),
            after_fold.len(),
            "Fold 서브트리가 침범당하지 않는다"
        );
        for d in &before {
            assert!(
                tr.state().view().node(*d).is_some() || tr.state().view().leaf(*d).is_some(),
                "{d:?} 보존"
            );
        }
    }

    #[test]
    fn insert_after_nested_reversed_fold_preserves_all_dots() {
        let state = State::from_changesets(nested_reversed_fold_css(), None).unwrap();
        let table = state.projected.child_elem_dots(Dot::ROOT)[0];
        let before: Vec<Dot> = state.projected.subtree_real_dots(table);

        let mut tr = Transaction::new(&state);
        tr.insert_subtree(
            Dot::ROOT,
            1,
            Subtree::leaf(NodeType::Paragraph.into_node().to_plain()),
        )
        .unwrap();

        let after: Vec<Dot> = tr.state().projected.subtree_real_dots(table);
        assert_eq!(
            before.len(),
            after.len(),
            "중첩 역순 Fold를 감싼 서브트리가 침범당하지 않는다"
        );
        for d in &before {
            assert!(
                tr.state().view().node(*d).is_some() || tr.state().view().leaf(*d).is_some(),
                "{d:?} 보존"
            );
        }
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
        let mut seq_pos = child_seq_insert_pos(ps, root, 2, NodeType::Image).unwrap();
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

    fn zombie_css() -> Vec<Changeset<EditOp>> {
        let d = |c| Dot::new(1, c);
        vec![Changeset {
            ops: vec![
                Op {
                    id: d(0),
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: d(1),
                    parents: vec![d(0)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
                Op {
                    id: d(2),
                    parents: vec![d(1)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 2,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT, Dot::new(9, 999)],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: d(3),
                    parents: vec![d(2)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 3,
                        item: SeqItem::Char('z'),
                    }),
                },
            ],
        }]
    }

    fn all_block_dots(state: &State) -> Vec<Dot> {
        let view = state.view();
        let mut out = vec![Dot::ROOT];
        if let Some(root) = view.root() {
            for d in root.descendants() {
                if let ChildView::Block(b) = d {
                    out.push(b.id());
                }
            }
        }
        out
    }

    fn build_fixture(pick: usize) -> State {
        match pick {
            0 => {
                let (state, ..) = state! {
                    doc {
                        root {
                            a: paragraph { text("ab") }
                            b: paragraph { text("cd") }
                        }
                    }
                    selection: (a, 0)
                };
                state
            }
            1 => {
                let (state, ..) = state! {
                    doc {
                        root {
                            list: bullet_list {
                                list_item { p: paragraph { text("ab") } }
                            }
                            tail: paragraph { text("z") }
                        }
                    }
                    selection: (p, 0)
                };
                state
            }
            2 => {
                let (state, ..) = state! {
                    doc {
                        root {
                            f: fold {
                                fold_title { text("t") }
                                fold_content {
                                    paragraph { text("a") }
                                    bullet_list { list_item { paragraph { text("b") } } }
                                }
                            }
                            tail: paragraph { text("z") }
                        }
                    }
                    selection: (f, 0)
                };
                state
            }
            _ => State::from_changesets(zombie_css(), None).unwrap(),
        }
    }

    proptest! {
        #[test]
        fn local_ins_realizes_requested_slot_or_rejects_atomically(
            fixture_pick in 0usize..4,
            block_pick in 0usize..64,
            offset_pick in 0usize..64,
            op_pick in 0usize..2,
            ch in proptest::char::range('a', 'z'),
        ) {
            let state = build_fixture(fixture_pick);
            let blocks = all_block_dots(&state);
            let block = blocks[block_pick % blocks.len()];
            let ins_type = if op_pick == 0 { NodeType::Text } else { NodeType::Paragraph };
            let content = &Schema::node_spec(state.projected.block_node_type(block).unwrap()).content;
            prop_assume!(!content.matches(ins_type) || content.is_repeatable(ins_type));

            let len = state.projected.child_count(block).unwrap_or(0);
            let offset = offset_pick % (len + 1);
            let before_doc = state.projected.projected().clone();
            let before_ops = state.graph().len();
            let before_drops = state.projected.projected().drops;
            let before_children: Vec<Dot> = state.projected.child_elem_dots(block);

            let mut tr = Transaction::new(&state);
            let result = if op_pick == 0 {
                tr.insert_text(block, offset, &ch.to_string())
            } else {
                tr.insert_subtree(
                    block,
                    offset,
                    Subtree::leaf(NodeType::Paragraph.into_node().to_plain()),
                )
            };
            match result {
                Ok(()) => {
                    let ps = &tr.state().projected;
                    let children = ps.block_children(block).unwrap();
                    let after_children: Vec<Dot> = ps.child_elem_dots(block);
                    prop_assert_eq!(after_children.len(), before_children.len() + 1);
                    prop_assert_eq!(&after_children[..offset], &before_children[..offset]);
                    prop_assert_eq!(&after_children[offset + 1..], &before_children[offset..]);
                    prop_assert!(
                        !before_children.contains(&after_children[offset]),
                        "신규 dot"
                    );
                    match children.get(offset) {
                        Some(Child::Leaf { item: SeqItem::Char(c), .. }) if op_pick == 0 => {
                            prop_assert_eq!(*c, ch);
                        }
                        Some(Child::Block(b)) if op_pick == 1 => {
                            prop_assert_eq!(ps.block_node_type(*b), Some(NodeType::Paragraph));
                        }
                        other => prop_assert!(false, "슬롯 {offset} 내용 불일치: {other:?}"),
                    }
                    prop_assert_eq!(
                        ps.projected().drops,
                        before_drops,
                        "성공 삽입이 드롭을 만들지 않는다"
                    );
                }
                Err(_) => {
                    prop_assert!(tr.state().projected.projected() == &before_doc);
                    prop_assert_eq!(
                        tr.state().graph().len(),
                        before_ops,
                        "거부는 op을 남기지 않는다"
                    );
                }
            }
        }
    }
}

use editor_crdt::sequence::{Bias as SeqBias, checkout_with_resolver};
use editor_crdt::{Dot, ListOp, LwwRegOp};
use editor_model::{
    Anchor, AtomLeaf, Bias, ChildView, EditOp, Marker, Modifier, ModifierAttrOp, ModifierType,
    NodeAttrOp, NodeLwwOp, NodeType, PlainNode, PlainTextNode, SeqClass, SeqItem, SpanOp, Subtree,
    classify,
};
use editor_state::{BatchedState, ProjectedState};

use crate::StepError;

pub(crate) fn block_node_type(ps: &ProjectedState, block: Dot) -> Option<NodeType> {
    Some(ps.view().node(block)?.node_type())
}

pub(crate) fn children_count(ps: &ProjectedState, block: Dot) -> Option<usize> {
    Some(ps.view().node(block)?.children().count())
}

pub(crate) fn child_block_ids(ps: &ProjectedState, block: Dot) -> Vec<Dot> {
    match ps.view().node(block) {
        Some(nv) => nv.child_blocks().map(|b| b.id()).collect(),
        None => Vec::new(),
    }
}

/// All children of `block` (chars, atom leaves, and nested blocks) as dots,
/// in order. Block-level atoms (Image/HR/…) project as Child::Leaf, so this is
/// the addressing the command layer uses (full child-slot index = offset),
/// distinct from `child_block_ids` which is blocks-only.
pub(crate) fn child_elem_ids(ps: &ProjectedState, block: Dot) -> Vec<Dot> {
    match ps.view().node(block) {
        Some(nv) => nv
            .children()
            .map(|c| match c {
                ChildView::Leaf(l) => l.dot(),
                ChildView::Block(b) => b.id(),
            })
            .collect(),
        None => Vec::new(),
    }
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
    let dots = match ps.view().node(prev) {
        Some(_) => subtree_dots(ps, prev).ok_or(StepError::NodeNotFound(prev))?,
        None => match prev.as_op_dot() {
            Some(d) => vec![d.dot()],
            None => return Err(StepError::NodeNotFound(prev)),
        },
    };
    let max = dots
        .iter()
        .filter_map(|&d| ps.seq_flat_pos(d))
        .max()
        .ok_or(StepError::NodeNotFound(prev))?;
    Ok(max + 1)
}

pub(crate) fn parents_chain(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    let view = ps.view();
    let chain: Vec<Dot> = view
        .node(block)?
        .ancestors()
        .skip(1)
        .filter_map(|n| n.dot())
        .collect();
    Some(chain.into_iter().rev().collect())
}

pub(crate) fn self_inclusive_parents(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    let view = ps.view();
    let chain: Vec<Dot> = view
        .node(block)?
        .ancestors()
        .filter_map(|n| n.dot())
        .collect();
    Some(chain.into_iter().rev().collect())
}

pub(crate) fn seq_insert_pos(ps: &ProjectedState, block: Dot, offset: usize) -> Option<usize> {
    let block_dot = block.as_op_dot();
    if block_dot.is_none() && block != Dot::ROOT {
        let (_, resolver) = checkout_with_resolver(ps.seq());
        let view = ps.view();
        let nv = view.node(block)?;
        if offset == 0 {
            let first_child = nv.child_at(0)?;
            let first_child_dot = match first_child {
                ChildView::Leaf(l) => l.dot(),
                ChildView::Block(b) => b.dot()?,
            };
            return resolver
                .resolve_boundary(first_child_dot, SeqBias::Before)
                .map(|b| b.position);
        } else {
            let child = nv.child_at(offset - 1)?;
            let child_dot = match child {
                ChildView::Leaf(l) => l.dot(),
                ChildView::Block(b) => b.dot()?,
            };
            return resolver
                .resolve_boundary(child_dot, SeqBias::After)
                .map(|b| b.position);
        }
    }
    let (_, resolver) = checkout_with_resolver(ps.seq());
    if offset == 0 {
        return match block_dot {
            Some(d) => resolver
                .resolve_boundary(d.dot(), SeqBias::After)
                .map(|b| b.position),
            None => Some(0),
        };
    }
    let view = ps.view();
    let child = view.node(block)?.child_at(offset - 1)?;
    let child_dot = match child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };
    resolver
        .resolve_boundary(child_dot, SeqBias::After)
        .map(|b| b.position)
}

pub(crate) fn subtree_dots(ps: &ProjectedState, block: Dot) -> Option<Vec<Dot>> {
    let view = ps.view();
    let nv = view.node(block)?;
    let mut dots = Vec::new();
    if let Some(d) = nv.dot() {
        dots.push(d);
    }
    for c in nv.descendants() {
        match c {
            ChildView::Leaf(l) => dots.push(l.dot()),
            ChildView::Block(b) => {
                if let Some(d) = b.dot() {
                    dots.push(d);
                }
            }
        }
    }
    Some(dots)
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
    let dots = subtree_dots(ps, prev).ok_or(StepError::NodeNotFound(prev))?;
    let max = dots
        .iter()
        .filter_map(|&d| ps.seq_flat_pos(d))
        .max()
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

pub(crate) fn leaf_dots_in_range(
    ps: &ProjectedState,
    block: Dot,
    offset: usize,
    len: usize,
) -> Result<Vec<Dot>, StepError> {
    let view = ps.view();
    let nv = view.node(block).ok_or(StepError::NodeNotFound(block))?;
    let children: Vec<ChildView> = nv.children().collect();
    let mut dots = Vec::with_capacity(len);
    for i in 0..len {
        let idx = offset + i;
        let child = children.get(idx).ok_or(StepError::OffsetOutOfBounds {
            block,
            offset: idx,
            len: children.len(),
        })?;
        let dot = match child {
            ChildView::Leaf(l) => l.dot(),
            ChildView::Block(b) => b.dot().ok_or(StepError::NodeNotFound(block))?,
        };
        dots.push(dot);
    }
    Ok(dots)
}

pub(crate) fn delete_dots_ops(ps: &ProjectedState, dots: &[Dot]) -> Vec<EditOp> {
    let mut positions: Vec<usize> = dots.iter().filter_map(|&d| ps.seq_flat_pos(d)).collect();
    positions.sort_unstable_by(|a, b| b.cmp(a));
    positions
        .into_iter()
        .map(|pos| EditOp::Seq(ListOp::Del { pos, len: 1 }))
        .collect()
}

pub(crate) fn read_text(ps: &ProjectedState, block: Dot, offset: usize, len: usize) -> String {
    match ps.view().node(block) {
        Some(nv) => nv.inline_text().chars().skip(offset).take(len).collect(),
        None => String::new(),
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

/// Ops that copy a source block's overlays (block modifiers, node style, node
/// marker) onto `target`. Used when a structural step mints a new block dot
/// that should inherit the source's block-level formatting (e.g. SplitNode).
pub(crate) fn block_overlay_ops(ps: &ProjectedState, src: Dot, target: Dot) -> Vec<EditOp> {
    let mut ops = Vec::new();
    for (_ty, m) in ps.block_modifiers().modifiers_of(src) {
        ops.push(block_modifier_set(target, m));
    }
    if let Some(style) = ps.node_styles().value_of(src) {
        ops.push(node_style_set(target, Some(style)));
    }
    if let Some(marker) = ps.node_markers().value_of(src) {
        ops.push(node_marker_set(target, Some(marker)));
    }
    ops
}

pub(crate) fn block_modifier_clear(target: Dot, key: ModifierType) -> EditOp {
    EditOp::BlockModifier(ModifierAttrOp::ClearModifier { target, key })
}

pub(crate) fn node_style_set(target: Dot, value: Option<String>) -> EditOp {
    EditOp::NodeStyle(NodeLwwOp {
        target,
        op: LwwRegOp::Set { value },
    })
}

pub(crate) fn node_marker_set(target: Dot, value: Option<Marker>) -> EditOp {
    EditOp::NodeMarker(NodeLwwOp {
        target,
        op: LwwRegOp::Set { value },
    })
}

/// Builds a `Subtree` snapshot of the projected block at `block`. Char/atom
/// leaves are grouped into `Text` subtrees (per-char span styling is dropped —
/// the M1 move/subtree path is structural). Nested blocks recurse.
pub fn capture_subtree(ps: &ProjectedState, block: Dot) -> Option<Subtree> {
    let view = ps.view();
    let nv = view.node(block)?;
    let node = nv.node().to_plain();
    let dot = nv.dot();
    let modifiers: Vec<Modifier> = dot
        .map(|d| {
            ps.block_modifiers()
                .modifiers_of(d)
                .values()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let style = dot.and_then(|d| ps.node_styles().value_of(d));
    let marker = dot.and_then(|d| ps.node_markers().value_of(d));

    let mut children: Vec<Subtree> = Vec::new();
    let mut pending_text = String::new();
    for c in nv.children() {
        match c {
            ChildView::Leaf(l) => {
                if let Some(ch) = l.as_char() {
                    pending_text.push(ch);
                } else if let Some(atom) = l.as_atom() {
                    if !pending_text.is_empty() {
                        children.push(text_subtree(std::mem::take(&mut pending_text)));
                    }
                    children.push(Subtree::leaf(atom.clone().into_node().to_plain()));
                }
            }
            ChildView::Block(b) => {
                if !pending_text.is_empty() {
                    children.push(text_subtree(std::mem::take(&mut pending_text)));
                }
                if let Some(sub) = capture_subtree(ps, b.id()) {
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
        style,
        marker,
        children,
    })
}

fn text_subtree(text: String) -> Subtree {
    Subtree::leaf(PlainNode::Text(PlainTextNode { text }))
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
            if let Some(style_id) = &subtree.style {
                batched.apply(node_style_set(dot, Some(style_id.clone())))?;
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
                    if let Some(style_id) = &subtree.style {
                        for &d in &char_dots {
                            batched.apply(node_style_set(d, Some(style_id.clone())))?;
                        }
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
            if let Some(style_id) = &subtree.style {
                batched.apply(node_style_set(dot, Some(style_id.clone())))?;
            }
            Ok(Some(dot))
        }
    }
}

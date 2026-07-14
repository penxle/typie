use std::collections::BTreeMap;

use editor_common::StrExt;
use editor_crdt::Dot;
use editor_model::{
    ChildView, Modifier, ModifierType, Node, NodeType, PlainHardBreakNode, PlainNode,
    PlainPageBreakNode, PlainTabNode, PlainTextNode, Subtree,
};
use editor_state::{Affinity, PendingModifiers, Position, Selection};
use editor_transaction::{Step, Transaction};

use crate::helpers::resolve_effective_modifiers;
use crate::{CommandError, CommandResult};

/// Remove the child at full child-slot `index` of `parent` (a block OR a
/// block-level/inline atom leaf). Unlike `Transaction::remove_subtree`, this
/// indexes by the full child list and can remove leaf atoms.
pub(crate) fn remove_child_at(
    tr: &mut Transaction,
    parent: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let subtree = {
        let state = tr.state();
        let view = state.view();
        let parent_node = view
            .node(parent)
            .ok_or(CommandError::NodeNotFound(parent))?;
        let child = parent_node
            .child_at(index)
            .ok_or_else(|| CommandError::Corrupted("child index out of range".into()))?;
        match child {
            ChildView::Block(b) => editor_transaction::capture_subtree(&state.projected, b.id())
                .ok_or(CommandError::NodeNotFound(b.id()))?,
            ChildView::Leaf(l) => {
                let node = if let Some(ch) = l.as_char() {
                    PlainNode::Text(PlainTextNode {
                        text: ch.to_string(),
                    })
                } else {
                    l.node().map(|n| n.to_plain()).unwrap_or_else(|| {
                        PlainNode::Text(PlainTextNode {
                            text: String::new(),
                        })
                    })
                };
                let modifiers = match l.as_atom() {
                    Some(atom) if atom.is_block_level() => state
                        .projected
                        .block_modifiers()
                        .modifiers_of(l.dot())
                        .into_values()
                        .collect(),
                    _ => parent_node.leaf_own_modifiers_at(index),
                };
                Subtree {
                    node,
                    modifiers,
                    carry: Vec::new(),
                    children: Vec::new(),
                    source_dots: Vec::new(),
                }
            }
        }
    };
    tr.apply_steps(vec![Step::RemoveSubtree {
        parent,
        index,
        subtree,
    }])?;
    Ok(())
}

/// Remove `block` if it has no real children (the projection may synthesize a
/// Derived placeholder, so emptiness is judged by real children), then cascade
/// to its parent if that becomes empty in turn. A no-op for blocks that still
/// hold real content, are structural, or for which empty is valid.
pub(crate) fn prune_empty_real(tr: &mut Transaction, block: Dot) -> Result<(), CommandError> {
    let mut current = block;
    loop {
        let parent_id = {
            let view = tr.state().view();
            let Some(node) = view.node(current) else {
                break;
            };
            let has_real_child = node
                .children()
                .any(|c| crate::helpers::child_elem_id(&c).as_op_dot().is_some());
            let spec = node.spec();
            let removable = !has_real_child
                && !spec.structural
                && spec.content.min_required() > 0
                && node.parent().is_some();
            if !removable {
                break;
            }
            node.parent().map(|p| p.id())
        };
        let Some(parent_id) = parent_id else {
            break;
        };
        tr.remove_subtree(current)?;
        current = parent_id;
    }
    Ok(())
}

/// Collect the leaf ids of the children in `[offset, offset + len)` of `block`.
pub(crate) fn child_leaf_dots(tr: &Transaction, block: Dot, offset: usize, len: usize) -> Vec<Dot> {
    let view = tr.state().view();
    let Some(node) = view.node(block) else {
        return Vec::new();
    };
    (offset..offset + len)
        .filter_map(|i| match node.child_at(i) {
            Some(ChildView::Leaf(l)) => Some(l.dot()),
            _ => None,
        })
        .collect()
}

/// Reconcile inline (span) formatting on `dots` so the non-style own modifiers
/// match `desired`, applied per leaf kind (char excludes nothing; atom excludes
/// link/ruby) over consecutive same-kind runs.
pub(crate) fn apply_inline_modifiers(
    tr: &mut Transaction,
    dots: &[Dot],
    desired: &[Modifier],
) -> Result<(), CommandError> {
    if dots.is_empty() {
        return Ok(());
    }
    let is_char: Vec<bool> = {
        let view = tr.state().view();
        dots.iter()
            .map(|d| view.leaf(*d).and_then(|l| l.as_char()).is_some())
            .collect()
    };
    let mut i = 0;
    while i < dots.len() {
        let kind = is_char[i];
        let mut j = i + 1;
        while j < dots.len() && is_char[j] == kind {
            j += 1;
        }
        apply_inline_modifiers_run(tr, dots[i], dots[j - 1], kind, desired)?;
        i = j;
    }
    Ok(())
}

fn apply_inline_modifiers_run(
    tr: &mut Transaction,
    first: Dot,
    last: Dot,
    is_char: bool,
    desired: &[Modifier],
) -> Result<(), CommandError> {
    let applicable = |ty: ModifierType| -> bool {
        ty.is_text_applicable()
            && (is_char || !matches!(ty, ModifierType::Link | ModifierType::Ruby))
    };

    let desired_map: BTreeMap<ModifierType, Modifier> = desired
        .iter()
        .filter(|m| applicable(m.as_type()))
        .map(|m| (m.as_type(), m.clone()))
        .collect();

    let actual: BTreeMap<ModifierType, Modifier> = {
        let view = tr.state().view();
        match view.leaf_state_by_dot_slow(first) {
            Some(st) => st.own.iter().map(|(t, o)| (*t, o.value.clone())).collect(),
            None => BTreeMap::new(),
        }
    };

    for (ty, m) in &desired_map {
        if actual.get(ty) != Some(m) {
            tr.add_span_modifier(first, last, m.clone())?;
        }
    }
    for (ty, m) in &actual {
        if !desired_map.contains_key(ty) && ty.is_text_applicable() {
            tr.remove_span_modifier(first, last, m.clone())?;
        }
    }
    Ok(())
}

/// If a position targets a projection-synthesized scaffold block (one with no
/// authored op — e.g. the mandatory trailing paragraph the Root schema derives
/// after a block-level unit like a horizontal rule), that block has no CRDT
/// identity to parent inserted content to, so an insert against it fails with
/// `OffsetOutOfBounds`. Materialize the synthetic block — and any synthetic
/// ancestors up to the nearest real (or root) container — into real blocks and
/// return the equivalent position in the new real block. No-op for a real block
/// or the root.
pub(crate) fn materialize_position_block(
    tr: &mut Transaction,
    position: Position,
) -> Result<Position, CommandError> {
    let block = position.node;
    if block.as_op_dot().is_some() || block == Dot::ROOT {
        return Ok(position);
    }

    // Walk up to the nearest real (or root) ancestor, recording the synthetic
    // chain (deepest-first) so it can be rebuilt as real nested blocks.
    let (anchor_id, anchor_slot, chain) = {
        let view = tr.state().view();
        let mut node = view.node(block).ok_or(CommandError::NodeNotFound(block))?;
        let mut chain = vec![node.node().to_plain()];
        loop {
            let parent = node.parent().ok_or(CommandError::NoParent(node.id()))?;
            let slot = node
                .index()
                .ok_or_else(|| CommandError::orphan_child(node.id(), parent.id()))?;
            if parent.id().as_op_dot().is_some() || parent.id() == Dot::ROOT {
                break (parent.id(), slot, chain);
            }
            chain.push(parent.node().to_plain());
            node = parent;
        }
    };

    let mut subtree: Option<Subtree> = None;
    for node in chain.iter().cloned() {
        let mut st = Subtree::leaf(node);
        if let Some(child) = subtree.take() {
            st = st.with_children(vec![child]);
        }
        subtree = Some(st);
    }
    let subtree = subtree.expect("synthetic chain is non-empty");

    materialize_preceding_synthetic_siblings(tr, anchor_id, anchor_slot)?;
    tr.insert_subtree(anchor_id, anchor_slot, subtree)?;

    let materialized_block = {
        let view = tr.state().view();
        let mut cur = match view.node(anchor_id).and_then(|p| p.child_at(anchor_slot)) {
            Some(ChildView::Block(b)) => b.id(),
            _ => return Err(CommandError::Corrupted("materialized block missing".into())),
        };
        for _ in 1..chain.len() {
            cur = match view.node(cur).and_then(|n| n.first_child()) {
                Some(ChildView::Block(b)) => b.id(),
                _ => return Err(CommandError::Corrupted("materialized child missing".into())),
            };
        }
        cur
    };

    Ok(Position {
        node: materialized_block,
        ..position
    })
}

fn materialize_preceding_synthetic_siblings(
    tr: &mut Transaction,
    parent_id: Dot,
    end_slot: usize,
) -> Result<(), CommandError> {
    let mut start_slot = end_slot;
    while start_slot > 0 {
        let is_synthetic = {
            let view = tr.state().view();
            matches!(
                view.node(parent_id).and_then(|p| p.child_at(start_slot - 1)),
                Some(ChildView::Block(child)) if child.id().as_op_dot().is_none()
            )
        };
        if !is_synthetic {
            break;
        }
        start_slot -= 1;
    }

    for slot in start_slot..end_slot {
        let subtree = {
            let view = tr.state().view();
            let Some(ChildView::Block(child)) = view.node(parent_id).and_then(|p| p.child_at(slot))
            else {
                continue;
            };
            if child.id().as_op_dot().is_some() {
                continue;
            }
            // InsertSubtree computes the insertion point from the previous sibling,
            // so the synthetic run immediately before the target must become addressable first.
            Subtree::leaf(child.node().to_plain())
        };
        tr.insert_subtree(parent_id, slot, subtree)?;
    }
    Ok(())
}

/// Materialize the synthetic block containing a collapsed caret and move the
/// caret into the new real block. No-op for a real caret block, the root, or a
/// non-collapsed selection.
pub(crate) fn materialize_caret_block(tr: &mut Transaction) -> Result<(), CommandError> {
    let Some(selection) = tr.selection() else {
        return Ok(());
    };
    if selection.anchor != selection.head {
        return Ok(());
    }
    let caret = selection.head;
    let materialized = materialize_position_block(tr, caret)?;
    if materialized.node == caret.node {
        return Ok(());
    }

    tr.set_selection(Some(Selection::collapsed(Position {
        node: materialized.node,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(())
}

pub(crate) fn insert_terminal_page_break_into_root_paragraph(
    tr: &mut Transaction,
    paragraph_id: Dot,
) -> CommandResult {
    let insert_index = {
        let view = tr.state().view();
        let Some(paragraph) = view.node(paragraph_id) else {
            return Ok(false);
        };
        if !matches!(paragraph.node(), Node::Paragraph(_)) {
            return Ok(false);
        }
        if paragraph
            .parent()
            .is_none_or(|parent| parent.id() != Dot::ROOT)
        {
            return Ok(false);
        }
        if paragraph.children().any(
            |child| matches!(child, ChildView::Leaf(leaf) if leaf.node_type() == NodeType::PageBreak),
        ) {
            return Ok(false);
        }
        paragraph.children().count()
    };

    tr.insert_subtree(
        paragraph_id,
        insert_index,
        Subtree::leaf(PlainNode::PageBreak(PlainPageBreakNode::default())),
    )?;
    Ok(true)
}

fn caret_paint(tr: &Transaction, block: Dot, offset: usize) -> Vec<Modifier> {
    let pending_modifiers = tr.pending_modifiers().clone();
    resolve_effective_modifiers(&tr.state().projected, block, offset, &pending_modifiers)
}

pub(crate) fn insert_text_at_caret(
    tr: &mut Transaction,
    text: &str,
    paint_override: Option<&[Modifier]>,
) -> CommandResult {
    if text.is_empty() {
        return Err(CommandError::InvalidArgument(
            "text must not be empty".into(),
        ));
    }
    if text.contains(['\n', '\r']) {
        return Err(CommandError::InvalidArgument(
            "text must not contain newlines".into(),
        ));
    }

    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;
    let block = pos.node;

    let mut paint = match paint_override {
        Some(paint) => paint.to_vec(),
        None => caret_paint(tr, block, pos.offset),
    };
    paint.retain(|m| m.as_type().is_text_applicable());

    let insert_len = text.char_count();
    tr.insert_text(block, pos.offset, text)?;

    let new_dots = child_leaf_dots(tr, block, pos.offset, insert_len);
    apply_inline_modifiers(tr, &new_dots, &paint)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: block,
        offset: pos.offset + insert_len,
        affinity: Affinity::Upstream,
    })))?;

    Ok(true)
}

fn insert_atom_at_caret(
    tr: &mut Transaction,
    plain: PlainNode,
    paint_override: Option<&[Modifier]>,
) -> CommandResult {
    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;
    let block = pos.node;

    let mut paint = match paint_override {
        Some(paint) => paint.to_vec(),
        None => caret_paint(tr, block, pos.offset),
    };
    paint.retain(|m| {
        m.as_type().is_text_applicable()
            && !matches!(m.as_type(), ModifierType::Link | ModifierType::Ruby)
    });

    tr.insert_subtree(block, pos.offset, Subtree::leaf(plain))?;

    let new_dots = child_leaf_dots(tr, block, pos.offset, 1);
    apply_inline_modifiers(tr, &new_dots, &paint)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: block,
        offset: pos.offset + 1,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

pub(crate) fn insert_hard_break_at_caret(
    tr: &mut Transaction,
    paint_override: Option<&[Modifier]>,
) -> CommandResult {
    insert_atom_at_caret(
        tr,
        PlainNode::HardBreak(PlainHardBreakNode::default()),
        paint_override,
    )
}

pub(crate) fn insert_tab_at_caret(
    tr: &mut Transaction,
    paint_override: Option<&[Modifier]>,
) -> CommandResult {
    insert_atom_at_caret(tr, PlainNode::Tab(PlainTabNode::default()), paint_override)
}

pub(crate) fn consume_pending_modifiers(tr: &mut Transaction) -> Result<(), CommandError> {
    if !tr.pending_modifiers().is_empty() {
        tr.set_pending_modifiers(PendingModifiers::new())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Alignment, ChildView, EditOp, SpanOp};
    use editor_state::State;
    use editor_transaction::Step;

    use super::*;

    fn root_id(state: &State) -> Dot {
        state.view().root().unwrap().id()
    }

    fn first_child_leaf_dot(state: &State, block: Dot, index: usize) -> Dot {
        match state.view().node(block).unwrap().child_at(index).unwrap() {
            ChildView::Leaf(l) => l.dot(),
            ChildView::Block(_) => panic!("expected leaf child"),
        }
    }

    #[test]
    fn remove_child_at_undo_restores_image_alignment() {
        let (base, _p1) = state! {
            doc { root { image p1: paragraph { text("x") } } }
            selection: (p1, 0)
        };
        let root = root_id(&base);
        let img = first_child_leaf_dot(&base, root, 0);

        let mut prep = Transaction::new(&base);
        prep.apply_steps(vec![Step::AddModifier {
            block: img,
            modifier: Modifier::Alignment {
                value: Alignment::Center,
            },
        }])
        .unwrap();
        let (initial, ..) = prep.commit();
        let center = Modifier::Alignment {
            value: Alignment::Center,
        };
        assert_eq!(
            initial
                .projected
                .block_modifiers()
                .modifiers_of(img)
                .get(&ModifierType::Alignment),
            Some(&center),
            "precondition: the image carries center alignment as its own block modifier"
        );

        let mut tr = Transaction::new(&initial);
        remove_child_at(&mut tr, root, 0).unwrap();
        let (after, records, ..) = tr.commit();
        assert!(after.view().leaf(img).is_none(), "the image is removed");

        let mut ops = Vec::new();
        let restored = records.iter().rev().fold(after, |s, r| {
            let out = r.step.inverse().apply(&s).unwrap();
            ops.extend(out.ops);
            out.state
        });

        let rimg = first_child_leaf_dot(&restored, root, 0);
        assert_eq!(
            restored
                .projected
                .block_modifiers()
                .modifiers_of(rimg)
                .get(&ModifierType::Alignment),
            Some(&center),
            "undo restores the image's center alignment as a block modifier"
        );

        for op in &ops {
            if let EditOp::Span(SpanOp::AddSpan { modifier, .. }) = &op.payload {
                assert!(
                    modifier.as_type().is_text_applicable(),
                    "undo recorded an invalid-target span carrying {modifier:?}"
                );
            }
        }
    }

    #[test]
    fn remove_child_at_undo_restores_inline_atom_paint() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("a") tab [font_size(2400)] text("b") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&initial);
        remove_child_at(&mut tr, p1, 1).unwrap();
        let (after, records, ..) = tr.commit();

        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);

        let view = restored.view();
        let para = view.node(p1).unwrap();
        assert!(
            para.leaf_own_modifiers_at(1)
                .contains(&Modifier::FontSize { value: 2400 }),
            "undo restores the tab's own font-size span"
        );
    }
}

use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, NodeType, NodeView, PlainNode, PlainParagraphNode, PlainTextNode, Subtree,
};
use editor_state::paragraph_break_at_end;
use editor_state::{Affinity, Position, Selection, State};
use editor_transaction::{Step, Transaction, fulfill};

use super::{
    apply_first_text_marker_lift, capture_first_text_marker, find_ancestor_textblock,
    find_enclosing_paragraph_id, find_lowest_common_ancestor, is_block_container,
    merge_element_cross_parent, next_sibling, path_from_ancestor,
};
use crate::{CommandError, CommandResult};

enum SlotKind {
    Char,
    Atom,
    Block(Dot),
}

fn slot_kind(view: &DocView, block: Dot, idx: usize) -> Option<SlotKind> {
    match view.node(block)?.child_at(idx)? {
        ChildView::Leaf(l) => {
            if l.as_char().is_some() {
                Some(SlotKind::Char)
            } else {
                Some(SlotKind::Atom)
            }
        }
        ChildView::Block(b) => Some(SlotKind::Block(b.id())),
    }
}

fn child_count(view: &DocView, block: Dot) -> usize {
    view.node(block).map(|n| n.children().count()).unwrap_or(0)
}

fn is_structural(view: &DocView, id: Dot) -> bool {
    view.node(id).is_some_and(|n| n.spec().structural)
}

/// Delete child slots `[from, to)` of `block`, high index first to avoid shifts.
fn delete_child_slots(
    tr: &mut Transaction,
    block: Dot,
    from: usize,
    to: usize,
) -> Result<(), CommandError> {
    if to <= from {
        return Ok(());
    }
    for idx in (from..to).rev() {
        let kind = {
            let view = tr.state().view();
            slot_kind(&view, block, idx)
        };
        match kind {
            Some(SlotKind::Char) => {
                tr.remove_text(block, idx, 1)?;
            }
            Some(SlotKind::Atom) => {
                remove_atom_leaf(tr, block, idx)?;
            }
            Some(SlotKind::Block(id)) => {
                remove_or_clear(tr, id)?;
            }
            None => {}
        }
    }
    Ok(())
}

fn elem_id_of(child: &ChildView) -> Dot {
    match child {
        ChildView::Block(b) => b.id(),
        ChildView::Leaf(l) => l.dot(),
    }
}

fn text_subtree(text: String) -> Subtree {
    Subtree::leaf(PlainNode::Text(PlainTextNode { text }))
}

/// Snapshot of a projected block's subtree (block overlays + char/atom/block
/// children), mirroring the substrate's capture so the removal step carries the
/// data needed for its inverse. Char runs collapse into `Text` subtrees.
fn capture_subtree(state: &State, block: Dot) -> Option<Subtree> {
    let view = state.view();
    let nv = view.node(block)?;
    let node = nv.node().to_plain();
    let dot = nv.dot();
    let modifiers: Vec<_> = dot
        .map(|d| {
            state
                .projected
                .block_modifiers()
                .modifiers_of(d)
                .into_values()
                .collect()
        })
        .unwrap_or_default();
    let style = dot.and_then(|d| state.projected.node_styles().value_of(d));
    let marker = dot.and_then(|d| state.projected.node_markers().value_of(d));

    let mut children: Vec<Subtree> = Vec::new();
    let mut pending = String::new();
    for c in nv.children() {
        match c {
            ChildView::Leaf(l) => {
                if let Some(ch) = l.as_char() {
                    pending.push(ch);
                } else if let Some(atom) = l.as_atom() {
                    if !pending.is_empty() {
                        children.push(text_subtree(std::mem::take(&mut pending)));
                    }
                    children.push(Subtree::leaf(atom.clone().into_node().to_plain()));
                }
            }
            ChildView::Block(b) => {
                if !pending.is_empty() {
                    children.push(text_subtree(std::mem::take(&mut pending)));
                }
                if let Some(sub) = capture_subtree(state, b.id()) {
                    children.push(sub);
                }
            }
        }
    }
    if !pending.is_empty() {
        children.push(text_subtree(pending));
    }

    Some(Subtree {
        node,
        modifiers,
        style,
        marker,
        children,
    })
}

/// Remove a leaf atom (image/HR/tab/break) child at full-child `index`.
/// The convenience `Transaction::remove_subtree` cannot address leaf atoms
/// (it resolves index via `child_blocks()` and parent via the node map), so
/// build the `RemoveSubtree` step directly with the full-child slot index.
pub(crate) fn remove_atom_leaf(
    tr: &mut Transaction,
    parent: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let subtree = {
        let view = tr.state().view();
        let node = view
            .node(parent)
            .ok_or(CommandError::NodeNotFound(parent))?;
        let atom = match node.child_at(index) {
            Some(ChildView::Leaf(l)) => l
                .as_atom()
                .ok_or_else(|| CommandError::Corrupted("expected atom leaf".into()))?
                .clone(),
            _ => return Err(CommandError::Corrupted("expected leaf at slot".into())),
        };
        Subtree::leaf(atom.into_node().to_plain())
    };
    tr.apply_steps(vec![Step::RemoveSubtree {
        parent,
        index,
        subtree,
    }])?;
    Ok(())
}

/// Remove a block (or leaf-atom) child by stable id, addressing it at its FULL
/// child-slot index. The convenience `Transaction::remove_subtree` resolves the
/// index via `child_blocks()`, which mismatches the step's full-child indexing
/// whenever leaf atoms precede the target — removing the wrong element. This
/// computes the true slot and captures the subtree for the inverse.
pub(crate) fn remove_subtree_full(tr: &mut Transaction, child_id: Dot) -> Result<(), CommandError> {
    let (parent_id, index, subtree) = {
        let state = tr.state();
        let view = state.view();
        match view.node(child_id) {
            Some(nv) => {
                let parent = nv.parent().ok_or(CommandError::NoParent(child_id))?;
                let parent_id = parent.id();
                let index = parent
                    .children()
                    .position(|c| elem_id_of(&c) == child_id)
                    .ok_or_else(|| CommandError::orphan_child(child_id, parent_id))?;
                let subtree =
                    capture_subtree(state, child_id).ok_or(CommandError::NodeNotFound(child_id))?;
                (parent_id, index, subtree)
            }
            None => {
                let Some(op) = child_id.as_op_dot() else {
                    return Err(CommandError::NodeNotFound(child_id));
                };
                let dot = op.dot();
                let leaf = view.leaf(dot).ok_or(CommandError::NodeNotFound(child_id))?;
                let parent = leaf.parent().ok_or(CommandError::NoParent(child_id))?;
                let parent_id = parent.id();
                let (index, subtree) = parent
                    .children()
                    .enumerate()
                    .find_map(|(i, c)| match &c {
                        ChildView::Leaf(l) if l.dot() == dot => {
                            let subtree = if let Some(ch) = l.as_char() {
                                text_subtree(ch.to_string())
                            } else {
                                Subtree::leaf(l.as_atom()?.clone().into_node().to_plain())
                            };
                            Some((i, subtree))
                        }
                        _ => None,
                    })
                    .ok_or_else(|| CommandError::orphan_child(child_id, parent_id))?;
                (parent_id, index, subtree)
            }
        }
    };
    tr.apply_steps(vec![Step::RemoveSubtree {
        parent: parent_id,
        index,
        subtree,
    }])?;
    Ok(())
}

fn is_real_child(child: &ChildView) -> bool {
    match child {
        ChildView::Block(b) => b.id().as_op_dot().is_some(),
        ChildView::Leaf(_) => true,
    }
}

/// A container is structurally empty when it holds no real children — only the
/// `Derived` placeholder paragraph the projection synthesizes for an otherwise
/// empty container. The projected `children()` is therefore never literally
/// empty, so emptiness must be tested against real ids.
pub(crate) fn is_structurally_empty(node: &NodeView) -> bool {
    !node.children().any(|c| is_real_child(&c))
}

/// Like `prune`, but removes the (structurally) empty node and any ancestor that
/// becomes empty as a result, using full-child-slot indexing. The substrate
/// `prune` resolves the slot via `child_blocks()` (wrong when leaf atoms precede
/// the target) and tests emptiness against projected children (which always show
/// the synthesized placeholder).
pub(crate) fn prune_empty_full(tr: &mut Transaction, node_id: Dot) -> Result<(), CommandError> {
    let mut current = node_id;
    loop {
        let next = {
            let view = tr.state().view();
            let Some(nv) = view.node(current) else {
                break;
            };
            if !is_structurally_empty(&nv) {
                break;
            }
            if nv.spec().content.min_required() == 0 {
                break;
            }
            if nv.spec().structural {
                break;
            }
            let Some(parent) = nv.parent() else {
                break;
            };
            let parent_id = parent.id();
            let parent_real_children = parent.children().filter(|c| is_real_child(c)).count();
            let parent_cascades = parent_real_children == 1
                && parent.spec().content.min_required() > 0
                && !parent.spec().structural;
            (parent_id, parent_cascades)
        };
        remove_subtree_full(tr, current)?;
        let (parent_id, parent_cascades) = next;
        if parent_cascades {
            current = parent_id;
        } else {
            break;
        }
    }
    Ok(())
}

pub(crate) fn selection_for_node(
    view: &DocView,
    node_id: Dot,
) -> Result<Option<Selection>, CommandError> {
    let (parent_id, index) = match view.node(node_id) {
        Some(target) => {
            let parent = match target.parent() {
                Some(parent) => parent,
                None => return Ok(None),
            };
            let parent_id = parent.id();
            let index = target
                .index()
                .ok_or_else(|| CommandError::orphan_child(node_id, parent_id))?;
            (parent_id, index)
        }
        None => {
            // Block-level atoms (image/HR/...) project as leaves, not nodes.
            let Some(op) = node_id.as_op_dot() else {
                return Err(CommandError::NodeNotFound(node_id));
            };
            let dot = op.dot();
            let leaf = view.leaf(dot).ok_or(CommandError::NodeNotFound(node_id))?;
            let parent = leaf.parent().ok_or(CommandError::NoParent(node_id))?;
            let parent_id = parent.id();
            let index = parent
                .children()
                .position(|c| matches!(&c, ChildView::Leaf(l) if l.dot() == dot))
                .ok_or_else(|| CommandError::orphan_child(node_id, parent_id))?;
            (parent_id, index)
        }
    };

    Ok(Some(Selection::new(
        Position {
            node: parent_id,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: parent_id,
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    )))
}

pub(crate) fn delete_selection_range(tr: &mut Transaction, selection: Selection) -> CommandResult {
    let selection = lower_exact_empty_paragraph_break_delete_range(tr, selection);
    if selection.anchor == selection.head {
        return Ok(false);
    }

    // Resolve the geometry under an immutable borrow, collecting only owned data.
    let plan = {
        let view = tr.state().view();
        let resolved = selection
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("cannot resolve selection".into()))?;

        if let Some(rect) = resolved.as_cell_rect() {
            let cell_ids: Vec<Dot> = rect.cells().iter().map(|c| c.id()).collect();
            let anchor_id = rect.anchor_cell.id();
            Plan::CellRect {
                cell_ids,
                anchor_id,
            }
        } else {
            let from = resolved.from().position();
            let to = resolved.to().position();
            Plan::Range { from, to }
        }
    };

    match plan {
        Plan::CellRect {
            cell_ids,
            anchor_id,
        } => {
            tr.batch::<_, CommandError>(|tr| {
                for cell_id in cell_ids {
                    clear_structural_subtree(tr, cell_id)?;
                }
                Ok(())
            })?;
            let cursor = {
                let view = tr.state().view();
                find_first_text_position(&view, anchor_id)
            }
            .ok_or_else(|| CommandError::Corrupted("anchor cell has no text position".into()))?;
            tr.set_selection(Some(Selection::collapsed(cursor)))?;
            Ok(true)
        }
        Plan::Range { from, to } => delete_resolved_range(tr, from, to),
    }
}

enum Plan {
    CellRect { cell_ids: Vec<Dot>, anchor_id: Dot },
    Range { from: Position, to: Position },
}

fn delete_resolved_range(tr: &mut Transaction, from: Position, to: Position) -> CommandResult {
    let captured = {
        let captured_paragraph_id = {
            let view = tr.state().view();
            find_enclosing_paragraph_id(&view, from.node)
        };
        captured_paragraph_id.and_then(|id| capture_first_text_marker(tr.state(), id))
    };

    if from.node == to.node {
        let is_container = {
            let view = tr.state().view();
            view.node(from.node).is_some_and(|n| is_block_container(&n))
        };
        if is_container {
            tr.batch::<_, CommandError>(|tr| {
                delete_child_slots(tr, from.node, from.offset, to.offset)?;
                let steps = {
                    let view = tr.state().view();
                    view.node(from.node)
                        .map(|n| fulfill(&n))
                        .unwrap_or_default()
                };
                tr.apply_steps(steps)?;
                Ok(())
            })?;
            let sel = ensure_selection_after_child_range_delete(tr, from.node, from.offset)?;
            tr.set_selection(Some(sel))?;
        } else {
            delete_child_slots(tr, from.node, from.offset, to.offset)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node: from.node,
                offset: from.offset,
                affinity: Affinity::Downstream,
            })))?;
        }
        if let Some(captured) = captured {
            apply_first_text_marker_lift(tr, &captured)?;
        }
        return Ok(true);
    }

    // Cross-node range.
    let (lca_id, from_tb, to_tb, from_path, to_path) = {
        let view = tr.state().view();
        let lca_id = find_lowest_common_ancestor(&view, from.node, to.node)
            .ok_or_else(|| CommandError::Corrupted("no common ancestor".into()))?;
        let from_tb = find_ancestor_textblock(&view, from.node);
        let to_tb = find_ancestor_textblock(&view, to.node);
        let mut from_path = path_from_ancestor(&view, from.node, lca_id)
            .ok_or_else(|| CommandError::Corrupted("from is not descendant of LCA".into()))?;
        from_path.push(from.offset);
        let mut to_path = path_from_ancestor(&view, to.node, lca_id)
            .ok_or_else(|| CommandError::Corrupted("to is not descendant of LCA".into()))?;
        to_path.push(to.offset);
        (lca_id, from_tb, to_tb, from_path, to_path)
    };

    let from_node_id = from.node;
    let to_node_id = to.node;
    tr.batch::<_, CommandError>(|tr| {
        delete_range(tr, &from_path, &to_path, lca_id)?;
        merge_after_delete(tr, from_tb, to_tb, lca_id)?;
        fulfill_ancestors(tr, from_node_id, lca_id)?;
        fulfill_ancestors(tr, to_node_id, lca_id)?;
        Ok(())
    })?;

    let from_still_exists = tr.state().view().node(from.node).is_some();
    let selection = if from_still_exists {
        let view = tr.state().view();
        resolve_selection_at(&view, from.node, from.offset)
    } else {
        let view = tr.state().view();
        let cursor = match find_first_text_position(&view, lca_id) {
            Some(p) => p,
            None => Position::new(lca_id, 0),
        };
        Selection::collapsed(cursor)
    };
    tr.set_selection(Some(selection))?;

    if let Some(captured) = captured {
        apply_first_text_marker_lift(tr, &captured)?;
    }
    Ok(true)
}

fn lower_exact_empty_paragraph_break_delete_range(
    tr: &Transaction,
    selection: Selection,
) -> Selection {
    let view = tr.state().view();
    let Some(resolved) = selection.resolve(&view) else {
        return selection;
    };
    let from = resolved.from().position();
    let to = resolved.to().position();
    let Some(paragraph_break) = paragraph_break_at_end(&from, &view) else {
        return selection;
    };
    if Selection::new(from, to) != paragraph_break {
        return selection;
    }
    let Some(start) = empty_paragraph_delete_start(&view, &from) else {
        return selection;
    };
    Selection::new(start, to)
}

fn empty_paragraph_delete_start(view: &DocView, position: &Position) -> Option<Position> {
    let paragraph = view.node(position.node)?;
    if paragraph.node_type() != NodeType::Paragraph || paragraph.children().next().is_some() {
        return None;
    }
    Some(Position {
        node: paragraph.parent()?.id(),
        offset: paragraph.index()?,
        affinity: Affinity::Downstream,
    })
}

fn ensure_selection_after_child_range_delete(
    tr: &mut Transaction,
    container_id: Dot,
    offset: usize,
) -> Result<Selection, CommandError> {
    let count = {
        let view = tr.state().view();
        if view.node(container_id).is_none() {
            return Ok(resolve_selection_at(&view, container_id, offset));
        }
        child_count(&view, container_id)
    };

    if offset < count {
        let view = tr.state().view();
        match slot_kind(&view, container_id, offset) {
            // A synthetic scaffold block (no real op) cannot host a caret or
            // receive inserts; fall through to materialize a real paragraph.
            Some(SlotKind::Block(child_id)) if child_id.as_op_dot().is_some() => {
                return Ok(selection_at_child(&view, container_id, offset, child_id)
                    .unwrap_or_else(|| resolve_selection_at(&view, container_id, offset)));
            }
            Some(SlotKind::Atom) => {
                // A block-level atom (image/HR) now sits at the deletion point;
                // node-select it rather than inserting a fresh paragraph.
                return Ok(Selection::new(
                    Position {
                        node: container_id,
                        offset,
                        affinity: Affinity::Downstream,
                    },
                    Position {
                        node: container_id,
                        offset: offset + 1,
                        affinity: Affinity::Downstream,
                    },
                ));
            }
            _ => {}
        }
    }

    tr.insert_subtree(
        container_id,
        offset,
        Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default())),
    )?;
    let new_elem = {
        let view = tr.state().view();
        view.node(container_id)
            .and_then(|c| match c.child_at(offset) {
                Some(ChildView::Block(b)) => Some(b.id()),
                _ => None,
            })
    };
    match new_elem {
        Some(id) => Ok(Selection::collapsed(Position::new(id, 0))),
        None => Ok(Selection::collapsed(Position::new(container_id, offset))),
    }
}

fn remove_or_clear(tr: &mut Transaction, child_id: Dot) -> Result<(), CommandError> {
    let structural = {
        let view = tr.state().view();
        is_structural(&view, child_id)
    };
    if structural {
        clear_structural_subtree(tr, child_id)?;
    } else {
        remove_subtree_full(tr, child_id)?;
    }
    Ok(())
}

fn clear_structural_subtree(tr: &mut Transaction, node_id: Dot) -> Result<(), CommandError> {
    let child_ids: Vec<Dot> = {
        let view = tr.state().view();
        match view.node(node_id) {
            Some(n) => n.children().map(|c| elem_id_of(&c)).collect(),
            None => return Ok(()),
        }
    };
    for child_id in child_ids.into_iter().rev() {
        let structural = {
            let view = tr.state().view();
            is_structural(&view, child_id)
        };
        if structural {
            clear_structural_subtree(tr, child_id)?;
        } else {
            remove_subtree_full(tr, child_id)?;
        }
    }
    let steps = {
        let view = tr.state().view();
        view.node(node_id).map(|n| fulfill(&n)).unwrap_or_default()
    };
    tr.apply_steps(steps)?;
    Ok(())
}

/// Recursively delete content from path position to end of subtree.
fn delete_from(tr: &mut Transaction, path: &[usize], node_id: Dot) -> Result<(), CommandError> {
    // A synthetic scaffold node (e.g. a mandatory trailing paragraph) has no real
    // op and is regenerated by projection with a slot-dependent id that may have
    // shifted after preceding slots were deleted; there is nothing to delete in
    // one, so descending into it is a no-op.
    if node_id.as_op_dot().is_none() && node_id != Dot::ROOT {
        return Ok(());
    }
    let count = {
        let view = tr.state().view();
        if view.node(node_id).is_none() {
            return Err(CommandError::NodeNotFound(node_id));
        }
        child_count(&view, node_id)
    };

    if path.len() == 1 {
        let offset = path[0];
        delete_child_slots(tr, node_id, offset, count)?;
    } else {
        let idx = path[0];
        let child_id = {
            let view = tr.state().view();
            match view.node(node_id).and_then(|n| n.child_at(idx)) {
                Some(ChildView::Block(b)) => b.id(),
                _ => return Ok(()),
            }
        };
        delete_child_slots(tr, node_id, idx + 1, count)?;
        delete_from(tr, &path[1..], child_id)?;
    }
    Ok(())
}

/// Recursively delete content from start of subtree to path position.
fn delete_to(tr: &mut Transaction, path: &[usize], node_id: Dot) -> Result<(), CommandError> {
    // See `delete_from`: a synthetic scaffold node has nothing to delete and its
    // id may be stale after sibling slots were removed, so no-op.
    if node_id.as_op_dot().is_none() && node_id != Dot::ROOT {
        return Ok(());
    }
    if tr.state().view().node(node_id).is_none() {
        return Err(CommandError::NodeNotFound(node_id));
    }

    if path.len() == 1 {
        let offset = path[0];
        delete_child_slots(tr, node_id, 0, offset)?;
    } else {
        let idx = path[0];
        // Resolve the descend target by stable id BEFORE deleting preceding
        // slots — that deletion shifts later indices, so `child_at(idx)`
        // afterwards would point at the wrong child.
        let child_id = {
            let view = tr.state().view();
            match view.node(node_id).and_then(|n| n.child_at(idx)) {
                Some(ChildView::Block(b)) => b.id(),
                _ => return Ok(()),
            }
        };
        delete_child_slots(tr, node_id, 0, idx)?;
        delete_to(tr, &path[1..], child_id)?;
    }
    Ok(())
}

/// Delete range [from, to) within subtree rooted at node_id.
fn delete_range(
    tr: &mut Transaction,
    from_path: &[usize],
    to_path: &[usize],
    node_id: Dot,
) -> Result<(), CommandError> {
    let from_idx = from_path[0];
    let to_idx = to_path[0];

    if from_idx == to_idx {
        let child_id = {
            let view = tr.state().view();
            match view.node(node_id).and_then(|n| n.child_at(from_idx)) {
                Some(ChildView::Block(b)) => Some(b.id()),
                _ => None,
            }
        };
        match (from_path.len(), to_path.len()) {
            (1, l) if l > 1 => {
                if let Some(child_id) = child_id {
                    delete_to(tr, &to_path[1..], child_id)?;
                }
            }
            (l, 1) if l > 1 => {
                if let Some(child_id) = child_id {
                    delete_from(tr, &from_path[1..], child_id)?;
                }
            }
            (fl, tl) if fl > 1 && tl > 1 => {
                if let Some(child_id) = child_id {
                    delete_range(tr, &from_path[1..], &to_path[1..], child_id)?;
                }
            }
            (1, 1) => {
                delete_child_slots(tr, node_id, from_idx, to_idx)?;
            }
            _ => {}
        }
    } else {
        let (from_child_id, to_child_id) = {
            let view = tr.state().view();
            let node = view.node(node_id);
            let from_child_id = if from_path.len() > 1 {
                node.as_ref()
                    .and_then(|n| n.child_at(from_idx))
                    .and_then(|c| match c {
                        ChildView::Block(b) => Some(b.id()),
                        _ => None,
                    })
            } else {
                None
            };
            let to_child_id = if to_path.len() > 1 {
                node.as_ref()
                    .and_then(|n| n.child_at(to_idx))
                    .and_then(|c| match c {
                        ChildView::Block(b) => Some(b.id()),
                        _ => None,
                    })
            } else {
                None
            };
            (from_child_id, to_child_id)
        };

        let fully_from = if from_path.len() == 1 {
            from_idx
        } else {
            from_idx + 1
        };

        if let Some(child_id) = from_child_id {
            delete_from(tr, &from_path[1..], child_id)?;
        }

        delete_child_slots(tr, node_id, fully_from, to_idx)?;

        if let Some(child_id) = to_child_id {
            delete_to(tr, &to_path[1..], child_id)?;
        }
    }

    Ok(())
}

fn resolve_selection_at(view: &DocView, container_id: Dot, offset: usize) -> Selection {
    let count = match view.node(container_id) {
        Some(_) => child_count(view, container_id),
        None => return Selection::collapsed(Position::new(container_id, offset)),
    };

    if offset < count {
        let child_id = match slot_kind(view, container_id, offset) {
            Some(SlotKind::Block(id)) => Some(id),
            _ => None,
        };
        if let Some(child_id) = child_id
            && let Some(selection) = selection_at_child(view, container_id, offset, child_id)
        {
            return selection;
        }
    }

    if offset > 0 {
        let child_id = match slot_kind(view, container_id, offset - 1) {
            Some(SlotKind::Block(id)) => Some(id),
            _ => None,
        };
        if let Some(child_id) = child_id
            && let Some(selection) = selection_at_child(view, container_id, offset - 1, child_id)
        {
            return selection;
        }
    }

    Selection::collapsed(Position::new(container_id, offset.min(count)))
}

fn selection_at_child(
    view: &DocView,
    container_id: Dot,
    index: usize,
    child_id: Dot,
) -> Option<Selection> {
    let child = view.node(child_id)?;
    let spec = child.spec();
    if spec.selectable && !spec.inline {
        return Some(Selection::new(
            Position {
                node: container_id,
                offset: index,
                affinity: Affinity::Downstream,
            },
            Position {
                node: container_id,
                offset: index + 1,
                affinity: Affinity::Upstream,
            },
        ));
    }
    find_first_text_position(view, child_id).map(Selection::collapsed)
}

/// Walk into a node to find the first valid text-level position.
pub(crate) fn find_first_text_position(view: &DocView, node_id: Dot) -> Option<Position> {
    let node = view.node(node_id)?;
    if node.spec().is_textblock() {
        return Some(Position {
            node: node_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
    }
    let first_child_id = node.child_blocks().next()?.id();
    find_first_text_position(view, first_child_id)
}

fn structural_region(view: &DocView, node_id: Dot) -> Option<Dot> {
    let node = view.node(node_id)?;
    if node.spec().structural {
        return Some(node_id);
    }
    let mut current = node.parent()?;
    loop {
        if current.spec().structural {
            return Some(current.id());
        }
        current = current.parent()?;
    }
}

/// Merges `source` (a block container) into `target` by re-parenting each of
/// source's child blocks to the end of `target`, then removing the emptied
/// source. Unlike `merge_node` (which only flows up loose char/atom leaves),
/// this correctly relocates block children whose parents chain would otherwise
/// dangle to the deleted container.
fn merge_containers(tr: &mut Transaction, target: Dot, source: Dot) -> Result<(), CommandError> {
    loop {
        // Only real children move; the projection synthesizes a Derived
        // placeholder for an empty required container, so stop when only
        // placeholders remain (otherwise this loops forever).
        let child = {
            let view = tr.state().view();
            match view.node(source) {
                Some(s) => s
                    .child_blocks()
                    .find(|c| c.id().as_op_dot().is_some())
                    .map(|c| c.id()),
                None => return Ok(()),
            }
        };
        let Some(child) = child else { break };
        let target_len = {
            let view = tr.state().view();
            view.node(target)
                .map(|n| {
                    n.child_blocks()
                        .filter(|c| c.id().as_op_dot().is_some())
                        .count()
                })
                .unwrap_or(0)
        };
        tr.move_node(child, target, target_len)?;
    }
    remove_subtree_full(tr, source)
}

/// Merges `block`'s next same-parent sibling into it via `merge_containers`
/// (re-resolving the sibling, whose dot may be fresh after a prior move).
fn merge_with_next_sibling(tr: &mut Transaction, block: Dot) -> Result<(), CommandError> {
    let next = {
        let view = tr.state().view();
        view.node(block)
            .and_then(|n| next_sibling(&n))
            .and_then(|c| match c {
                ChildView::Block(b) => Some(b.id()),
                ChildView::Leaf(_) => None,
            })
    };
    match next {
        Some(next_id) => merge_containers(tr, block, next_id),
        None => Ok(()),
    }
}

fn merge_after_delete(
    tr: &mut Transaction,
    from_tb: Option<Dot>,
    to_tb: Option<Dot>,
    lca_id: Dot,
) -> Result<(), CommandError> {
    let (from_tb, to_tb) = match (from_tb, to_tb) {
        (Some(a), Some(b)) if a != b => (a, b),
        _ => return Ok(()),
    };

    {
        let view = tr.state().view();
        if view.node(from_tb).is_none() || view.node(to_tb).is_none() {
            return Ok(());
        }
        if structural_region(&view, from_tb) != structural_region(&view, to_tb) {
            return Ok(());
        }
    }

    let to_tb_parent = {
        let view = tr.state().view();
        view.node(to_tb).and_then(|n| n.parent()).map(|p| p.id())
    };

    // Trailing PageBreak guard: drop it before merging so it does not end up mid-list.
    let trailing_page_break = {
        let view = tr.state().view();
        view.node(from_tb)
            .and_then(|target| match target.last_child() {
                Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak => Some(l.dot()),
                _ => None,
            })
    };
    if let Some(pb) = trailing_page_break {
        remove_subtree_full(tr, pb)?;
    }

    merge_element_cross_parent(tr, to_tb, from_tb)?;

    // The to-side container that held to_tb is now emptied (its content merged
    // into from_tb); drop it before the container walk so it is not carried into
    // the merged container as an empty item.
    if let Some(parent_id) = to_tb_parent {
        let empty = {
            let view = tr.state().view();
            view.node(parent_id)
                .map(|p| is_structurally_empty(&p))
                .unwrap_or(false)
        };
        if empty {
            prune_empty_full(tr, parent_id)?;
        }
    }

    // Container-level merge: walk up, merge adjacent same-type siblings.
    let mut from_current = {
        let view = tr.state().view();
        view.node(from_tb).and_then(|n| n.parent()).map(|p| p.id())
    };

    while let Some(from_id) = from_current {
        if from_id == lca_id {
            break;
        }

        let (next_id, next_same_type, parent_id, is_list_item) = {
            let view = tr.state().view();
            let Some(from_node) = view.node(from_id) else {
                break;
            };
            match next_sibling(&from_node) {
                Some(ChildView::Block(next)) => {
                    let same = next.node_type() == from_node.node_type();
                    (
                        Some(next.id()),
                        same,
                        from_node.parent().map(|p| p.id()),
                        from_node.node_type() == NodeType::ListItem,
                    )
                }
                Some(ChildView::Leaf(_)) => {
                    (None, false, from_node.parent().map(|p| p.id()), false)
                }
                None => (None, false, from_node.parent().map(|p| p.id()), false),
            }
        };

        match next_id {
            Some(next_id) if next_same_type => {
                if is_list_item {
                    let (target_sublist, moved_sublist) = {
                        let view = tr.state().view();
                        let target_sublist = view.node(from_id).and_then(|n| {
                            n.child_blocks()
                                .find(|c| {
                                    matches!(
                                        c.node_type(),
                                        NodeType::BulletList | NodeType::OrderedList
                                    )
                                })
                                .map(|c| c.id())
                        });
                        let moved_sublist = view.node(next_id).and_then(|n| {
                            n.child_blocks()
                                .find(|c| {
                                    matches!(
                                        c.node_type(),
                                        NodeType::BulletList | NodeType::OrderedList
                                    )
                                })
                                .map(|c| c.id())
                        });
                        (target_sublist, moved_sublist)
                    };

                    if let Some(moved_id) = moved_sublist {
                        match target_sublist {
                            // A list item cannot hold two sublists (normalization
                            // drops the second), so relocate the next item's
                            // sublist ITEMS into the existing sublist rather than
                            // moving the sublist whole.
                            Some(target_id) => merge_containers(tr, target_id, moved_id)?,
                            None => {
                                let from_len = {
                                    let view = tr.state().view();
                                    view.node(from_id)
                                        .map(|n| n.child_blocks().count())
                                        .unwrap_or(0)
                                };
                                tr.move_node(moved_id, from_id, from_len)?;
                            }
                        }
                    }
                }

                let _ = next_id;
                merge_with_next_sibling(tr, from_id)?;
                from_current = parent_id;
            }
            _ => {
                if next_id.is_none() {
                    from_current = parent_id;
                } else {
                    break;
                }
            }
        }
    }

    // Repair structural ancestors and prune empties.
    let ancestor_chain: Vec<Dot> = {
        let view = tr.state().view();
        let mut chain = Vec::new();
        if let Some(start_id) = to_tb_parent {
            let mut current = start_id;
            loop {
                chain.push(current);
                if current == lca_id {
                    break;
                }
                match view.node(current).and_then(|n| n.parent()).map(|p| p.id()) {
                    Some(parent_id) => current = parent_id,
                    None => break,
                }
            }
        }
        chain
    };

    if let Some(parent_id) = to_tb_parent {
        let (empty, structural) = {
            let view = tr.state().view();
            match view.node(parent_id) {
                Some(parent) => (is_structurally_empty(&parent), parent.spec().structural),
                None => (false, false),
            }
        };
        if empty {
            if structural {
                let steps = {
                    let view = tr.state().view();
                    view.node(parent_id)
                        .map(|parent| fulfill(&parent))
                        .unwrap_or_default()
                };
                tr.apply_steps(steps)?;
            } else {
                prune_empty_full(tr, parent_id)?;
            }
        }
    }

    for id in &ancestor_chain {
        let steps = {
            let view = tr.state().view();
            match view.node(*id) {
                Some(node) if node.spec().structural => fulfill(&node),
                _ => Vec::new(),
            }
        };
        tr.apply_steps(steps)?;
    }

    let lca_steps = {
        let view = tr.state().view();
        view.node(lca_id)
            .map(|lca| fulfill(&lca))
            .unwrap_or_default()
    };
    tr.apply_steps(lca_steps)?;

    Ok(())
}

fn fulfill_ancestors(tr: &mut Transaction, start_id: Dot, lca_id: Dot) -> Result<(), CommandError> {
    let mut current = start_id;
    loop {
        let steps = {
            let view = tr.state().view();
            view.node(current).map(|n| fulfill(&n)).unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        if current == lca_id {
            break;
        }
        let parent = {
            let view = tr.state().view();
            view.node(current).and_then(|n| n.parent()).map(|p| p.id())
        };
        match parent {
            Some(parent_id) => current = parent_id,
            None => break,
        }
    }
    Ok(())
}

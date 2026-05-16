use editor_model::{Doc, Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, compact, fulfill, prune};

use super::{
    find_ancestor_textblock, find_lowest_common_ancestor, is_block_container, path_from_ancestor,
};
use crate::{CommandError, CommandResult};

pub(crate) fn selection_for_node(
    doc: &Doc,
    node_id: NodeId,
) -> Result<Option<Selection>, CommandError> {
    let target = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;
    let parent = match target.parent() {
        Some(parent) => parent,
        None => return Ok(None),
    };
    let parent_id = parent.id();
    let index = target
        .index()
        .ok_or_else(|| CommandError::orphan_child(node_id, parent_id))?;

    Ok(Some(Selection::new(
        Position {
            node_id: parent_id,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: parent_id,
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    )))
}

pub(crate) fn delete_selection_range(tr: &mut Transaction, selection: Selection) -> CommandResult {
    if selection.is_collapsed() {
        return Ok(false);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;

    // A cell-rect is the corner-bracket encoding of a rectangular cell block —
    // not a linear range. Decode it via the selection layer, then apply the
    // same generic structural clear: every selected cell keeps its structure
    // and is emptied to one paragraph; no cell/row is removed.
    if let Some(rect) = resolved.as_cell_rect() {
        let cell_ids: Vec<NodeId> = rect.cells().map(|c| c.id()).collect();
        let anchor_id = rect.anchor_cell.id();
        tr.batch::<_, CommandError>(|tr| {
            for cell_id in cell_ids {
                clear_structural_subtree(tr, cell_id)?;
            }
            Ok(())
        })?;
        let cursor = find_first_text_position(&tr.doc(), anchor_id).ok_or(
            CommandError::Corrupted("anchor cell has no text position".into()),
        )?;
        tr.set_selection(Selection::collapsed(cursor))?;
        return Ok(true);
    }

    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    if from.node_id == to.node_id {
        if tr
            .doc()
            .node(from.node_id)
            .is_some_and(|n| is_block_container(n.node()))
        {
            tr.batch::<_, CommandError>(|tr| {
                delete_within_node(tr, from.node_id, from.offset, to.offset)?;
                let doc = tr.doc();
                if let Some(node) = doc.node(from.node_id) {
                    tr.apply_steps(fulfill(&node))?;
                }
                Ok(())
            })?;
            let sel = resolve_selection_at(&tr.doc(), from.node_id, from.offset);
            tr.set_selection(sel)?;
        } else {
            let cursor = delete_within_node(tr, from.node_id, from.offset, to.offset)?;
            tr.set_selection(Selection::collapsed(cursor))?;
        }
    } else {
        let lca_id = find_lowest_common_ancestor(&doc, from.node_id, to.node_id)
            .ok_or(CommandError::Corrupted("no common ancestor".into()))?;

        let from_tb = find_ancestor_textblock(&doc, from.node_id);
        let to_tb = find_ancestor_textblock(&doc, to.node_id);

        // Record cursor info for inline from that may be deleted
        let from_parent_id = doc
            .node(from.node_id)
            .and_then(|n| n.parent())
            .map(|p| p.id());
        let from_index = doc.node(from.node_id).and_then(|n| n.index());
        let from_is_text = doc
            .node(from.node_id)
            .is_some_and(|n| matches!(n.node(), Node::Text(_)));
        let from_will_be_deleted = from_is_text && from.offset == 0;

        let mut from_path = path_from_ancestor(&doc, from.node_id, lca_id).ok_or(
            CommandError::Corrupted("from is not descendant of LCA".into()),
        )?;
        from_path.push(from.offset);

        let mut to_path = path_from_ancestor(&doc, to.node_id, lca_id).ok_or(
            CommandError::Corrupted("to is not descendant of LCA".into()),
        )?;
        to_path.push(to.offset);

        let from_node_id = from.node_id;
        let to_node_id = to.node_id;
        tr.batch::<_, CommandError>(|tr| {
            delete_range(tr, &from_path, &to_path, lca_id)?;
            merge_after_delete(tr, from_tb, to_tb, lca_id)?;
            fulfill_ancestors(tr, from_node_id, lca_id)?;
            // The to-side endpoint may itself be a structural container whose
            // non-structural children were emptied by delete_to; fulfill it too.
            // fulfill_ancestors is idempotent and skips already-removed nodes.
            fulfill_ancestors(tr, to_node_id, lca_id)?;
            Ok(())
        })?;

        let cursor = if from_is_text && !from_will_be_deleted {
            from
        } else if from_is_text && from_will_be_deleted {
            if tr.doc().node(from.node_id).is_some() {
                from
            } else {
                // Re-lookup siblings from the post-merge doc, not pre-recorded ones.
                // Merge may have moved new children into the parent.
                let parent_id = from_parent_id.unwrap_or(lca_id);
                let idx = from_index.unwrap_or(0);
                let doc = tr.doc();
                let next_id = doc
                    .node(parent_id)
                    .and_then(|p| p.entry().children.iter().nth(idx).copied());
                let prev_id = if idx > 0 {
                    doc.node(parent_id)
                        .and_then(|p| p.entry().children.iter().nth(idx - 1).copied())
                } else {
                    None
                };

                resolve_cursor_after_removal(tr, prev_id, next_id, parent_id, idx)
            }
        } else {
            let sel = resolve_selection_at(&tr.doc(), from.node_id, from.offset);
            tr.set_selection(sel)?;
            return Ok(true);
        };

        tr.set_selection(Selection::collapsed(cursor))?;
    }

    Ok(true)
}

fn delete_within_node(
    tr: &mut Transaction,
    node_id: NodeId,
    from_offset: usize,
    to_offset: usize,
) -> Result<Position, CommandError> {
    let doc = tr.doc();
    let node = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;

    match node.node() {
        Node::Text(text_node) => {
            let text_len = text_node.text.len();
            if from_offset == 0 && to_offset >= text_len {
                let parent_id = node.parent().ok_or(CommandError::NoParent(node_id))?.id();
                let node_index = node
                    .index()
                    .ok_or(CommandError::orphan_child(node_id, parent_id))?;
                let prev_id = node.prev_sibling().map(|n| n.id());
                let next_id = node.next_sibling().map(|n| n.id());

                tr.remove_subtree(node_id)?;

                Ok(resolve_cursor_after_removal(
                    tr, prev_id, next_id, parent_id, node_index,
                ))
            } else {
                tr.remove_text(node_id, from_offset, to_offset - from_offset)?;
                Ok(Position {
                    node_id,
                    offset: from_offset,
                    affinity: Affinity::Upstream,
                })
            }
        }
        _ => {
            let children_to_remove: Vec<NodeId> = node
                .entry()
                .children
                .iter()
                .skip(from_offset)
                .take(to_offset - from_offset)
                .copied()
                .collect();

            for child_id in children_to_remove.into_iter().rev() {
                remove_or_clear(tr, child_id)?;
            }

            Ok(Position {
                node_id,
                offset: from_offset,
                affinity: Affinity::Downstream,
            })
        }
    }
}

/// Remove a fully-selected child, honoring the schema `structural` invariant:
/// a structural node is a fixed part of its parent and is never deleted — its
/// content is cleared recursively and the node is re-fulfilled instead.
fn remove_or_clear(tr: &mut Transaction, child_id: NodeId) -> Result<(), CommandError> {
    let is_structural = tr.doc().node(child_id).is_some_and(|n| n.spec().structural);
    if is_structural {
        clear_structural_subtree(tr, child_id)?;
    } else {
        tr.remove_subtree(child_id)?;
    }
    Ok(())
}

/// Recursively empties a structural node: structural descendants are preserved
/// (recursed into), non-structural descendants are removed, then every visited
/// structural node is re-fulfilled so it regains its minimal required content
/// (an emptied TableCell/FoldContent gets one empty paragraph; an emptied
/// FoldTitle stays empty since its content is `Text*`).
fn clear_structural_subtree(tr: &mut Transaction, node_id: NodeId) -> Result<(), CommandError> {
    let child_ids: Vec<NodeId> = match tr.doc().node(node_id) {
        Some(n) => n.entry().children.iter().copied().collect(),
        None => return Ok(()),
    };
    for child_id in child_ids.into_iter().rev() {
        let child_structural = tr.doc().node(child_id).is_some_and(|n| n.spec().structural);
        if child_structural {
            clear_structural_subtree(tr, child_id)?;
        } else {
            tr.remove_subtree(child_id)?;
        }
    }
    let doc = tr.doc();
    if let Some(node) = doc.node(node_id) {
        tr.apply_steps(fulfill(&node))?;
    }
    Ok(())
}

fn resolve_cursor_after_removal(
    tr: &Transaction,
    prev_id: Option<NodeId>,
    next_id: Option<NodeId>,
    parent_id: NodeId,
    removed_index: usize,
) -> Position {
    let doc = tr.doc();

    if let Some(next_id) = next_id
        && let Some(next) = doc.node(next_id)
        && matches!(next.node(), Node::Text(_))
    {
        return Position {
            node_id: next_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
    }

    if let Some(prev_id) = prev_id
        && let Some(prev) = doc.node(prev_id)
        && let Node::Text(t) = prev.node()
    {
        return Position {
            node_id: prev_id,
            offset: t.text.len(),
            affinity: Affinity::Upstream,
        };
    }

    Position {
        node_id: parent_id,
        offset: removed_index,
        affinity: Affinity::Downstream,
    }
}

/// Recursively delete content from path position to end of subtree.
fn delete_from(tr: &mut Transaction, path: &[usize], node_id: NodeId) -> Result<(), CommandError> {
    let doc = tr.doc();
    let node = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;

    if path.len() == 1 {
        let offset = path[0];
        match node.node() {
            Node::Text(t) => {
                let text_len = t.text.len();
                if offset == 0 {
                    tr.remove_subtree(node_id)?;
                } else if offset < text_len {
                    tr.remove_text(node_id, offset, text_len - offset)?;
                }
            }
            _ => {
                let children: Vec<NodeId> =
                    node.entry().children.iter().skip(offset).copied().collect();
                for child_id in children.into_iter().rev() {
                    remove_or_clear(tr, child_id)?;
                }
            }
        }
    } else {
        let idx = path[0];
        let children: Vec<NodeId> = node
            .entry()
            .children
            .iter()
            .skip(idx + 1)
            .copied()
            .collect();
        for child_id in children.into_iter().rev() {
            remove_or_clear(tr, child_id)?;
        }
        let child_id = node.entry().children.iter().nth(idx).copied().unwrap();
        delete_from(tr, &path[1..], child_id)?;
    }

    Ok(())
}

/// Recursively delete content from start of subtree to path position.
fn delete_to(tr: &mut Transaction, path: &[usize], node_id: NodeId) -> Result<(), CommandError> {
    let doc = tr.doc();
    let node = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;

    if path.len() == 1 {
        let offset = path[0];
        match node.node() {
            Node::Text(t) => {
                let text_len = t.text.len();
                if offset >= text_len {
                    tr.remove_subtree(node_id)?;
                } else if offset > 0 {
                    tr.remove_text(node_id, 0, offset)?;
                }
            }
            _ => {
                let children: Vec<NodeId> =
                    node.entry().children.iter().take(offset).copied().collect();
                for child_id in children.into_iter().rev() {
                    remove_or_clear(tr, child_id)?;
                }
            }
        }
    } else {
        let idx = path[0];
        let children: Vec<NodeId> = node.entry().children.iter().take(idx).copied().collect();
        for child_id in children.into_iter().rev() {
            remove_or_clear(tr, child_id)?;
        }
        let child_id = node.entry().children.iter().nth(idx).copied().unwrap();
        delete_to(tr, &path[1..], child_id)?;
    }

    Ok(())
}

/// Delete range [from, to) within subtree rooted at node_id.
fn delete_range(
    tr: &mut Transaction,
    from_path: &[usize],
    to_path: &[usize],
    node_id: NodeId,
) -> Result<(), CommandError> {
    let from_idx = from_path[0];
    let to_idx = to_path[0];

    if from_idx == to_idx {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        let child_id = node.entry().children.iter().nth(from_idx).copied().unwrap();

        match (from_path.len(), to_path.len()) {
            (1, l) if l > 1 => delete_to(tr, &to_path[1..], child_id)?,
            (l, 1) if l > 1 => delete_from(tr, &from_path[1..], child_id)?,
            (fl, tl) if fl > 1 && tl > 1 => {
                delete_range(tr, &from_path[1..], &to_path[1..], child_id)?
            }
            (1, 1) => {
                let _ = delete_within_node(tr, node_id, from_idx, to_idx)?;
            }
            _ => {}
        }
    } else {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        let children = &node.entry().children;

        let from_child_id = if from_path.len() > 1 {
            children.iter().nth(from_idx).copied()
        } else {
            None
        };
        let to_child_id = if to_path.len() > 1 {
            children.iter().nth(to_idx).copied()
        } else {
            None
        };

        let fully_from = if from_path.len() == 1 {
            from_idx
        } else {
            from_idx + 1
        };
        let fully_selected: Vec<NodeId> = children
            .iter()
            .skip(fully_from)
            .take(to_idx - fully_from)
            .copied()
            .collect();

        if let Some(child_id) = from_child_id {
            delete_from(tr, &from_path[1..], child_id)?;
        }

        for child_id in fully_selected.into_iter().rev() {
            remove_or_clear(tr, child_id)?;
        }

        if let Some(child_id) = to_child_id {
            delete_to(tr, &to_path[1..], child_id)?;
        }
    }

    Ok(())
}

/// Resolve a container position (container_id, offset) to the nearest valid selection.
fn resolve_selection_at(doc: &Doc, container_id: NodeId, offset: usize) -> Selection {
    let container = match doc.node(container_id) {
        Some(node) => node,
        None => return Selection::collapsed(Position::new(container_id, offset)),
    };
    let children = &container.entry().children;

    // After block-level deletions, cursor may be at a container position like (root, 0).
    // A collapsed selection at a container position in a block-children container is invalid.
    // Try forward child first: node selection for a block-level leaf, or collapsed at first text position.
    if let Some(&child_id) = children.iter().nth(offset)
        && let Some(selection) = selection_at_child(doc, container_id, offset, child_id)
    {
        return selection;
    }

    // Fall back to previous child.
    if offset > 0
        && let Some(&child_id) = children.iter().nth(offset - 1)
        && let Some(selection) = selection_at_child(doc, container_id, offset - 1, child_id)
    {
        return selection;
    }

    // Clamp to children.len() in case the offset became stale after child removals.
    Selection::collapsed(Position::new(container_id, offset.min(children.len())))
}

fn selection_at_child(
    doc: &Doc,
    container_id: NodeId,
    index: usize,
    child_id: NodeId,
) -> Option<Selection> {
    let child = doc.node(child_id)?;
    if is_block_level_leaf(child.node()) {
        return Some(Selection::new(
            Position {
                node_id: container_id,
                offset: index,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: container_id,
                offset: index + 1,
                affinity: Affinity::Upstream,
            },
        ));
    }
    find_first_text_position(doc, child_id).map(Selection::collapsed)
}

/// Check if a node is a "block-level leaf" for selection purposes.
/// These are selectable block nodes with no inline content (e.g., Image, File, HorizontalRule).
fn is_block_level_leaf(node: &Node) -> bool {
    let spec = node.spec();
    spec.selectable && !spec.inline
}

/// Walk into a node to find the first valid text-level position.
fn find_first_text_position(doc: &Doc, node_id: NodeId) -> Option<Position> {
    let node_ref = doc.node(node_id)?;
    let node = node_ref.node();

    // Text node -> position at offset 0
    if matches!(node, Node::Text(_)) {
        return Some(Position {
            node_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
    }

    if node.spec().is_textblock() {
        // Textblock with text children -> recurse into first child
        if let Some(&first_child_id) = node_ref.entry().children.iter().next()
            && let Some(pos) = find_first_text_position(doc, first_child_id)
        {
            return Some(pos);
        }
        // Empty textblock — cursor at (textblock, 0)
        return Some(Position {
            node_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
    }

    // Otherwise -> recurse into first child
    let first_child_id = *node_ref.entry().children.iter().next()?;
    find_first_text_position(doc, first_child_id)
}

/// The structural region a node belongs to: the node itself if it is
/// `structural` (e.g. FoldTitle, which is both a textblock and structural),
/// otherwise its nearest structural ancestor, otherwise `None` (outermost).
/// Two textblocks with different regions are separated by a structural
/// boundary and must not have content merged across it.
fn structural_region(doc: &Doc, node_id: NodeId) -> Option<NodeId> {
    let node = doc.node(node_id)?;
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

/// After deletion, merge boundary textblocks and clean up containers.
fn merge_after_delete(
    tr: &mut Transaction,
    from_tb: Option<NodeId>,
    to_tb: Option<NodeId>,
    lca_id: NodeId,
) -> Result<(), CommandError> {
    let (from_tb, to_tb) = match (from_tb, to_tb) {
        (Some(a), Some(b)) if a != b => (a, b),
        _ => return Ok(()),
    };

    let doc = tr.doc();
    if doc.node(from_tb).is_none() || doc.node(to_tb).is_none() {
        return Ok(());
    }

    // Never merge content across a structural boundary. If the two textblocks
    // live in different structural regions, leave both intact.
    if structural_region(&doc, from_tb) != structural_region(&doc, to_tb) {
        return Ok(());
    }

    let to_tb_parent = doc.node(to_tb).and_then(|n| n.parent()).map(|p| p.id());

    // PageBreak is schema-restricted to the trailing child of a paragraph.
    // Merging into a paragraph whose last child is a PageBreak would place
    // that PageBreak in the middle of the resulting child list.
    if let Some(target) = doc.node(from_tb)
        && let Some(last) = target.last_child()
        && matches!(last.node(), Node::PageBreak(_))
    {
        let last_id = last.id();
        tr.remove_subtree(last_id)?;
    }

    tr.merge_node(to_tb, from_tb)?;

    let doc = tr.doc();
    if let Some(p) = doc.node(from_tb) {
        tr.apply_steps(compact(&p))?;
    }

    // Container-level merge: walk up, merge adjacent same-type siblings
    let mut from_current = {
        let doc = tr.doc();
        doc.node(from_tb).and_then(|n| n.parent()).map(|p| p.id())
    };

    while let Some(from_id) = from_current {
        if from_id == lca_id {
            break;
        }

        let doc = tr.doc();
        let Some(from_node) = doc.node(from_id) else {
            break;
        };

        match from_node.next_sibling() {
            Some(next) if next.node().as_type() == from_node.node().as_type() => {
                // Same-type next sibling → merge and walk up
                let next_id = next.id();
                let parent_id = from_node.parent().map(|p| p.id());

                // When merging two adjacent list_items, combine their sublists into one
                // so the merged list_item still has at most one trailing sublist. Identify
                // sublists by node type rather than child index because the prior
                // textblock merge has already removed next_list_item's paragraph child.
                if matches!(from_node.node(), Node::ListItem(_)) {
                    let target_sublist_id = from_node
                        .children()
                        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
                        .map(|c| c.id());
                    let moved_sublist_id = next
                        .children()
                        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
                        .map(|c| c.id());

                    if let Some(moved_id) = moved_sublist_id {
                        let doc = tr.doc();
                        let from_li = doc
                            .node(from_id)
                            .ok_or(CommandError::NodeNotFound(from_id))?;
                        let from_len = from_li.entry().children.len();
                        tr.move_node(moved_id, from_id, from_len)?;

                        if let Some(target_id) = target_sublist_id {
                            tr.merge_node(moved_id, target_id)?;
                        }
                    }
                }

                tr.merge_node(next_id, from_id)?;
                from_current = parent_id;
            }
            None => {
                // No next sibling → walk up (to-branch may be at a higher level)
                from_current = from_node.parent().map(|p| p.id());
            }
            Some(_) => {
                // Different-type next sibling → stop
                break;
            }
        }
    }

    // Collect the ancestor chain from to_tb_parent up to lca_id before any prune
    // runs, because prune removes nodes and severs the parent link we need to walk.
    let ancestor_chain: Vec<NodeId> = {
        let doc = tr.doc();
        let mut chain = Vec::new();
        if let Some(start_id) = to_tb_parent {
            let mut current = start_id;
            loop {
                chain.push(current);
                if current == lca_id {
                    break;
                }
                match doc.node(current).and_then(|n| n.parent()).map(|p| p.id()) {
                    Some(parent_id) => current = parent_id,
                    None => break,
                }
            }
        }
        chain
    };

    let doc = tr.doc();
    if let Some(parent_id) = to_tb_parent
        && let Some(parent) = doc.node(parent_id)
        && parent.entry().children.is_empty()
    {
        if parent.spec().structural {
            tr.apply_steps(fulfill(&parent))?;
        } else {
            tr.apply_steps(prune(&parent))?;
        }
    }

    // The prune cascade above stops at the first structural ancestor but does not
    // repair it. Any structural node between to_tb_parent and the LCA may have been
    // emptied by the merge; fulfill them so no structural container is left
    // schema-invalid. fulfill is idempotent and skips already-removed nodes.
    for &id in &ancestor_chain {
        let doc = tr.doc();
        if let Some(node) = doc.node(id)
            && node.spec().structural
        {
            tr.apply_steps(fulfill(&node))?;
        }
    }

    let doc = tr.doc();
    if let Some(lca) = doc.node(lca_id) {
        tr.apply_steps(fulfill(&lca))?;
    }

    Ok(())
}

/// Walk from `start_id` up to (and including) `lca_id`, running fulfill on each node.
/// This ensures deeply nested containers that became empty after deletion are fixed.
fn fulfill_ancestors(
    tr: &mut Transaction,
    start_id: NodeId,
    lca_id: NodeId,
) -> Result<(), CommandError> {
    let mut current = start_id;
    loop {
        let doc = tr.doc();
        if let Some(node) = doc.node(current) {
            tr.apply_steps(fulfill(&node))?;
        }
        if current == lca_id {
            break;
        }
        let doc = tr.doc();
        match doc.node(current).and_then(|n| n.parent()).map(|p| p.id()) {
            Some(parent_id) => current = parent_id,
            None => break,
        }
    }
    Ok(())
}

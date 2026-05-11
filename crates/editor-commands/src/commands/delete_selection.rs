use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, compact, fulfill, prune};

use crate::helpers::{
    find_ancestor_textblock, find_lowest_common_ancestor, is_block_container, path_from_ancestor,
};
use crate::{CommandError, CommandResult};

pub fn delete_selection(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if selection.is_collapsed() {
        return Ok(false);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;

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
        tr.batch::<_, CommandError>(|tr| {
            delete_range(tr, &from_path, &to_path, lca_id)?;
            merge_after_delete(tr, from_tb, to_tb, lca_id)?;
            fulfill_ancestors(tr, from_node_id, lca_id)?;
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
                tr.remove_subtree(child_id)?;
            }

            Ok(Position {
                node_id,
                offset: from_offset,
                affinity: Affinity::Downstream,
            })
        }
    }
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
                    tr.remove_subtree(child_id)?;
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
            tr.remove_subtree(child_id)?;
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
                    tr.remove_subtree(child_id)?;
                }
            }
        }
    } else {
        let idx = path[0];
        let children: Vec<NodeId> = node.entry().children.iter().take(idx).copied().collect();
        for child_id in children.into_iter().rev() {
            tr.remove_subtree(child_id)?;
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
            tr.remove_subtree(child_id)?;
        }

        if let Some(child_id) = to_child_id {
            delete_to(tr, &to_path[1..], child_id)?;
        }
    }

    Ok(())
}

/// Check if a node is a "block-level leaf" for selection purposes.
/// These are selectable block nodes with no inline content (e.g., Image, File, HorizontalRule).
fn is_block_level_leaf(node: &Node) -> bool {
    let spec = node.spec();
    spec.selectable && !spec.inline
}

/// Walk into a node to find the first valid text-level position.
fn find_first_text_position(doc: &editor_model::Doc, node_id: NodeId) -> Option<Position> {
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

/// Resolve a container position (container_id, offset) to the nearest valid selection.
fn resolve_selection_at(doc: &editor_model::Doc, container_id: NodeId, offset: usize) -> Selection {
    let container = match doc.node(container_id) {
        Some(n) => n,
        None => return Selection::collapsed(Position::new(container_id, offset)),
    };
    let children = &container.entry().children;

    // After block-level deletions, cursor may be at a container position like (root, 0).
    // A collapsed selection at a container position in a block-children container is invalid.
    // Try forward child first: node selection for a block-level leaf, or collapsed at first text position.
    if let Some(&child_id) = children.iter().nth(offset)
        && let Some(child) = doc.node(child_id)
    {
        if is_block_level_leaf(child.node()) {
            return Selection::new(
                Position {
                    node_id: container_id,
                    offset,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: container_id,
                    offset: offset + 1,
                    affinity: Affinity::Upstream,
                },
            );
        }
        if let Some(pos) = find_first_text_position(doc, child_id) {
            return Selection::collapsed(pos);
        }
    }

    // Fall back to previous child.
    if offset > 0
        && let Some(&child_id) = children.iter().nth(offset - 1)
        && let Some(child) = doc.node(child_id)
    {
        if is_block_level_leaf(child.node()) {
            return Selection::new(
                Position {
                    node_id: container_id,
                    offset: offset - 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: container_id,
                    offset,
                    affinity: Affinity::Upstream,
                },
            );
        }
        if let Some(pos) = find_first_text_position(doc, child_id) {
            return Selection::collapsed(pos);
        }
    }

    Selection::collapsed(Position::new(container_id, offset))
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

    let to_tb_parent = doc.node(to_tb).and_then(|n| n.parent()).map(|p| p.id());

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

    let doc = tr.doc();
    if let Some(parent_id) = to_tb_parent
        && let Some(parent) = doc.node(parent_id)
        && parent.entry().children.is_empty()
    {
        tr.apply_steps(prune(&parent))?;
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

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| delete_selection(&mut tr));
    }

    #[test]
    fn delete_within_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Heorld") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_entire_text_node() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("A")
                t2: text("B")
                t3: text("C")
            } } }
            selection: (t2, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("A") t3: text("C") } } }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Held") }
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_with_middle_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("Middle") }
                paragraph { t3: text("World") }
            } }
            selection: (t1, 2) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Held") }
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_blockquotes_merges_containers() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("Hello") }
                }
                blockquote {
                    paragraph { t3: text("World") }
                    paragraph { t4: text("B") }
                }
            } }
            selection: (t2, 2) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("Held") }
                    paragraph { t4: text("B") }
                }
                paragraph {}
            } }
            selection: (t2, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_sole_content_leaves_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_inline_to() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { t1: text("Hello") } } }
            selection: (r, 0) -> (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_inline_from_block_to() {
        let (initial, ..) = state! {
            doc { r: root { paragraph { t1: text("Hello") } image } }
            selection: (t1, 2) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("He") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_block_to_same_parent() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                image
                horizontal_rule
                paragraph { t2: text("After") }
            } }
            selection: (r, 1) -> (r, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("After") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_inline_to_with_middle_nodes() {
        let (initial, ..) = state! {
            doc { r: root {
                image
                paragraph { t1: text("Middle") }
                paragraph { t2: text("Hello") }
            } }
            selection: (r, 0) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t2: text("lo") } } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_nodes_cursor_selects_adjacent_block() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
                p1: paragraph {}
            } }
            selection: (r, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_nodes_cursor_selects_remaining_hr() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("A") }
                image
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { t1: text("A") }
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1) -> (r, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_single_block_cursor_to_textblock() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { t1: text("Hello") } } }
            selection: (r, 0) -> (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_from_does_not_merge_adjacent_paragraphs() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                image
                paragraph { t2: text("Hello") }
            } }
            selection: (r, 1) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("lo") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfill_empty_container_after_deletion() {
        let (initial, ..) = state! {
            doc { r: root {
                fold {
                    fold_title { t1: text("Title") }
                    fc: fold_content {
                        image
                        paragraph { t2: text("Content") }
                    }
                }
                paragraph { t3: text("Hello") }
            } }
            selection: (fc, 0) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content {
                        fp: paragraph {}
                    }
                }
                paragraph { t3: text("lo") }
            } }
            selection: (fp, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_from_empty_paragraph_merges() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                paragraph { t2: text("asdf") }
            } }
            selection: (p1, 0) -> (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t2: text("asdf") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_both_texts_fully() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("asdf") }
                paragraph { t2: text("asdf") }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_to_empty_paragraph_merges() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("asdf") }
                p2: paragraph {}
                paragraph { t3: text("asdf") }
            } }
            selection: (t1, 4) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("asdf") }
                paragraph { t3: text("asdf") }
            } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_spanning_empty_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                paragraph { t1: text("asdf") }
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_hard_break() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("qwer")
                hard_break {}
                t2: text("zxcv")
            } } }
            selection: (t1, 2) -> (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("qw")
                t2: text("cv")
            } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_image_and_full_text() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { t1: text("hello") } } }
            selection: (r, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_start_and_image() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("hello") }
                image
                p2: paragraph {}
            } }
            selection: (t1, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_image_to_paragraph_start() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { t1: text("hello") } } }
            selection: (r, 0) -> (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_middle_image_cursor_to_prev_end() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("hello") }
                image
                paragraph { t2: text("world") }
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_to_first_hr_preserves_others() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("text1") }
                horizontal_rule
                horizontal_rule
                horizontal_rule
                paragraph { t2: text("text2") }
            } }
            selection: (t1, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
                horizontal_rule
                horizontal_rule
                paragraph { t2: text("text2") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_all_list_items_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { t1: text("A") } }
                    list_item { paragraph { t2: text("B") } }
                }
                p3: paragraph {}
            } }
            selection: (t1, 0) -> (p3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph {} }
                }
                p3: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_two_list_items() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { t1: text("asdf") } }
                    list_item { paragraph { t2: text("asdf") } }
                }
                paragraph {}
            } }
            selection: (t1, 2) -> (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { t1: text("asdf") } }
                }
                paragraph {}
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_merge_adjacent_lists() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { t1: text("1") } }
                    list_item { paragraph { t2: text("2") } }
                }
                ordered_list {
                    list_item { paragraph { t3: text("3") } }
                    list_item { paragraph { t4: text("4") } }
                }
            } }
            selection: (t2, 0) -> (t3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { t1: text("1") } }
                    list_item { paragraph { t3: text("3") } }
                    list_item { paragraph { t4: text("4") } }
                }
                paragraph {}
            } }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_fold_boundary() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1) -> (t3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("13") }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_title_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t2, 1) -> (t4, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("24") }
                    fold_content { paragraph {} }
                }
                paragraph {}
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_containing_whole_fold() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1) -> (t4, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("14") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_with_non_textblock_content() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("11") }
                    fold_content {
                        paragraph { t2: text("22") }
                        bullet_list {
                            list_item {
                                paragraph { t3: text("33") }
                            }
                        }
                    }
                }
            } }
            selection: (t1, 1) -> (t3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("13") }
                    fold_content { paragraph {} }
                }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_from_blockquote_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph {}
                    paragraph { t1: text("ㅁㄴㅇㅁㄴㅇ") }
                }
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph {}
                }
                p2: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_last_paragraph_fulfills_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}

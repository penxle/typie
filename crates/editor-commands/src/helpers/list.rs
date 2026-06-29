use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{DocView, NodeType, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

fn is_list_type(ty: NodeType) -> bool {
    matches!(ty, NodeType::BulletList | NodeType::OrderedList)
}

pub(crate) fn lift_list_item_inner(tr: &mut Transaction, list_item_id: Dot) -> CommandResult {
    let (
        list_id,
        list_type,
        owner_id,
        owner_is_list_item,
        list_index,
        list_item_index,
        after_items,
        lifted_paragraph_id,
        existing_sublist_id,
    ) = {
        let view = tr.state().view();
        let list_item = view
            .node(list_item_id)
            .ok_or(CommandError::NodeNotFound(list_item_id))?;
        if list_item.node_type() != NodeType::ListItem {
            return Ok(false);
        }

        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        let list_id = list.id();
        let list_type = list.node_type();
        if !is_list_type(list_type) {
            return Ok(false);
        }

        let owner = list.parent().ok_or(CommandError::NoParent(list_id))?;
        let owner_id = owner.id();
        let owner_is_list_item = owner.node_type() == NodeType::ListItem;

        let list_item_index = list_item
            .index()
            .ok_or_else(|| CommandError::orphan_child(list_item_id, list_id))?;
        let list_index = list
            .index()
            .ok_or_else(|| CommandError::orphan_child(list_id, owner_id))?;

        let after_items: Vec<Dot> = list
            .child_blocks()
            .skip(list_item_index + 1)
            .map(|c| c.id())
            .collect();

        let lifted_paragraph_id: Option<Dot> = list_item.child_blocks().next().map(|c| c.id());

        let existing_sublist_id: Option<Dot> = list_item
            .child_blocks()
            .find(|c| is_list_type(c.node_type()))
            .map(|c| c.id());

        (
            list_id,
            list_type,
            owner_id,
            owner_is_list_item,
            list_index,
            list_item_index,
            after_items,
            lifted_paragraph_id,
            existing_sublist_id,
        )
    };
    let _ = list_item_index;
    let _ = &existing_sublist_id;

    let (sel_offset, sel_affinity) = match (tr.selection(), &lifted_paragraph_id) {
        (Some(sel), Some(para)) if sel.head.node == *para => (sel.head.offset, sel.head.affinity),
        _ => (0, Affinity::Downstream),
    };
    let mut new_para_id: Option<Dot> = None;

    tr.batch::<_, CommandError>(|tr| {
        if owner_is_list_item {
            let (outer_list_id, outer_index) = {
                let view = tr.state().view();
                let outer_list_item = view
                    .node(owner_id)
                    .ok_or(CommandError::NodeNotFound(owner_id))?;
                let outer_list = outer_list_item
                    .parent()
                    .ok_or(CommandError::NoParent(owner_id))?;
                let outer_list_id = outer_list.id();
                let outer_index = outer_list_item
                    .index()
                    .ok_or_else(|| CommandError::orphan_child(owner_id, outer_list_id))?;
                (outer_list_id, outer_index)
            };
            tr.move_node(list_item_id, outer_list_id, outer_index + 1)?;

            // move_node re-emits the moved subtree with fresh dots; re-resolve
            // the lifted item (and its first paragraph) from the view.
            let moved_item_id = {
                let view = tr.state().view();
                view.node(outer_list_id)
                    .and_then(|l| l.child_blocks().nth(outer_index + 1))
                    .map(|b| b.id())
                    .ok_or(CommandError::NodeNotFound(outer_list_id))?
            };
            new_para_id = {
                let view = tr.state().view();
                view.node(moved_item_id)
                    .and_then(|li| li.child_blocks().next())
                    .map(|p| p.id())
            };

            if !after_items.is_empty() {
                let existing = {
                    let view = tr.state().view();
                    view.node(moved_item_id)
                        .and_then(|li| li.child_blocks().find(|c| is_list_type(c.node_type())))
                        .map(|c| c.id())
                };
                let target_sublist_id = match existing {
                    Some(id) => id,
                    None => {
                        let new_sublist_node = list_type.into_node().to_plain();
                        let insert_at = {
                            let view = tr.state().view();
                            view.node(moved_item_id)
                                .ok_or(CommandError::NodeNotFound(moved_item_id))?
                                .child_blocks()
                                .count()
                        };
                        tr.insert_subtree(
                            moved_item_id,
                            insert_at,
                            Subtree::leaf(new_sublist_node),
                        )?;
                        let view = tr.state().view();
                        view.node(moved_item_id)
                            .and_then(|li| li.child_blocks().last())
                            .map(|b| b.id())
                            .ok_or(CommandError::NodeNotFound(moved_item_id))?
                    }
                };
                let base = {
                    let view = tr.state().view();
                    view.node(target_sublist_id)
                        .ok_or(CommandError::NodeNotFound(target_sublist_id))?
                        .child_blocks()
                        .filter(|b| b.id().as_op_dot().is_some())
                        .count()
                };
                for (offset, item_id) in after_items.iter().enumerate() {
                    tr.move_node(*item_id, target_sublist_id, base + offset)?;
                }
            }
        } else {
            let (children, child_count) = {
                let view = tr.state().view();
                let list_item_ref = view
                    .node(list_item_id)
                    .ok_or(CommandError::NodeNotFound(list_item_id))?;
                let children: Vec<Dot> = list_item_ref.child_blocks().map(|c| c.id()).collect();
                let child_count = children.len();
                (children, child_count)
            };
            for (i, child_id) in children.iter().enumerate() {
                tr.move_node(*child_id, owner_id, list_index + 1 + i)?;
                if i == 0 {
                    new_para_id = {
                        let view = tr.state().view();
                        view.node(owner_id)
                            .and_then(|o| o.child_blocks().nth(list_index + 1))
                            .map(|p| p.id())
                    };
                }
            }
            tr.remove_subtree(list_item_id)?;

            if !after_items.is_empty() {
                let new_list_node = list_type.into_node().to_plain();
                tr.insert_subtree(
                    owner_id,
                    list_index + 1 + child_count,
                    Subtree::leaf(new_list_node),
                )?;
                let new_list_elem = {
                    let view = tr.state().view();
                    view.node(owner_id)
                        .and_then(|o| o.child_blocks().nth(list_index + 1 + child_count))
                        .map(|b| b.id())
                        .ok_or(CommandError::NodeNotFound(owner_id))?
                };
                for (i, item_id) in after_items.iter().enumerate() {
                    tr.move_node(*item_id, new_list_elem, i)?;
                }
            }
        }

        // The original list, once emptied of real items, projects a derived
        // scaffold list_item, so `prune` (which bails on any child) can't drop
        // it; remove the now-empty list directly.
        let remove_empty_list = {
            let view = tr.state().view();
            view.node(list_id)
                .map(|l| !l.child_blocks().any(|b| b.id().as_op_dot().is_some()))
                .unwrap_or(false)
        };
        if remove_empty_list {
            tr.remove_subtree(list_id)?;
        }

        let fulfill_steps = {
            let view = tr.state().view();
            view.node(owner_id).map(|o| fulfill(&o)).unwrap_or_default()
        };
        tr.apply_steps(fulfill_steps)?;
        Ok(())
    })?;

    if let Some(para_id) = new_para_id
        && tr.state().view().node(para_id).is_some()
    {
        tr.set_selection(Some(Selection::collapsed(Position {
            node: para_id,
            offset: sel_offset,
            affinity: sel_affinity,
        })))?;
    }

    Ok(true)
}

pub(crate) fn collect_top_level_list_items_in_selection(
    view: &DocView,
    from: Position,
    to: Position,
) -> Vec<Dot> {
    let from_list_item = find_enclosing_list_item_id(view, from.node);
    let to_list_item = find_enclosing_list_item_id(view, to.node);

    let (Some(from_li), Some(to_li)) = (from_list_item, to_list_item) else {
        return Vec::new();
    };

    if from_li == to_li {
        return vec![from_li];
    }

    let Some(common_list_id) = lowest_common_list_ancestor(view, from_li, to_li) else {
        return Vec::new();
    };

    let common_list = match view.node(common_list_id) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let children: Vec<Dot> = common_list.child_blocks().map(|c| c.id()).collect();
    let from_idx = ancestor_index_within(view, from_li, common_list_id);
    let to_idx = ancestor_index_within(view, to_li, common_list_id);
    let (Some(a), Some(b)) = (from_idx, to_idx) else {
        return Vec::new();
    };
    let lo = a.min(b);
    let hi = a.max(b);
    children[lo..=hi].to_vec()
}

pub(crate) fn find_enclosing_list_item_id(view: &DocView, node: Dot) -> Option<Dot> {
    let mut current = view.node(node)?;
    loop {
        if current.node_type() == NodeType::ListItem {
            return Some(current.id());
        }
        current = current.parent()?;
    }
}

fn lowest_common_list_ancestor(view: &DocView, a: Dot, b: Dot) -> Option<Dot> {
    let ancestors_a: Vec<Dot> = view.node(a)?.ancestors().map(|n| n.id()).collect();
    let ancestors_b: HashSet<Dot> = view.node(b)?.ancestors().map(|n| n.id()).collect();

    for la in ancestors_a.iter() {
        if view
            .node(*la)
            .map(|n| is_list_type(n.node_type()))
            .unwrap_or(false)
            && ancestors_b.contains(la)
        {
            return Some(*la);
        }
    }
    None
}

fn ancestor_index_within(view: &DocView, node: Dot, ancestor: Dot) -> Option<usize> {
    let mut current_id = node;
    loop {
        let current = view.node(current_id)?;
        let parent = current.parent()?;
        if parent.id() == ancestor {
            return current.index();
        }
        current_id = parent.id();
    }
}

pub(crate) fn is_at_list_item_content_start(view: &DocView, selection: &Selection) -> bool {
    if selection.anchor != selection.head {
        return false;
    }
    let pos = &selection.head;
    let Some(item_id) = find_enclosing_list_item_id(view, pos.node) else {
        return false;
    };
    let Some(item) = view.node(item_id) else {
        return false;
    };
    let Some(para) = item.child_blocks().next() else {
        return false;
    };
    pos.node == para.id() && pos.offset == 0
}

/// Sinks `list_item_id` into the preceding sibling's sublist. Returns the moved
/// item's fresh id (move_node re-emits the subtree), or `None` when the item
/// cannot sink (no previous sibling). Selection is preserved by the caller.
pub(crate) fn sink_list_item_inner(
    tr: &mut Transaction,
    list_item_id: Dot,
) -> Result<Option<Dot>, CommandError> {
    let (prev_id, list_type, target_sublist_id) = {
        let view = tr.state().view();
        let list_item = view
            .node(list_item_id)
            .ok_or(CommandError::NodeNotFound(list_item_id))?;
        if list_item.node_type() != NodeType::ListItem {
            return Ok(None);
        }

        let prev = match super::prev_sibling(&list_item) {
            Some(editor_model::ChildView::Block(p)) => p,
            _ => return Ok(None),
        };
        let prev_id = prev.id();

        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        let list_type = list.node_type();
        if !is_list_type(list_type) {
            return Ok(None);
        }

        let target_sublist_id = prev
            .child_blocks()
            .find(|c| is_list_type(c.node_type()))
            .map(|c| c.id());

        (prev_id, list_type, target_sublist_id)
    };

    let mut new_item_id: Option<Dot> = None;
    tr.batch::<_, CommandError>(|tr| {
        let target_id = match target_sublist_id {
            Some(id) => id,
            None => {
                let new_node = list_type.into_node().to_plain();
                let insert_at = {
                    let view = tr.state().view();
                    view.node(prev_id)
                        .ok_or(CommandError::NodeNotFound(prev_id))?
                        .child_blocks()
                        .count()
                };
                tr.insert_subtree(prev_id, insert_at, Subtree::leaf(new_node))?;
                let view = tr.state().view();
                view.node(prev_id)
                    .and_then(|p| p.child_blocks().last())
                    .map(|b| b.id())
                    .ok_or(CommandError::NodeNotFound(prev_id))?
            }
        };

        // A freshly created sublist projects a derived scaffold item; count only
        // real items so the move targets the true end slot.
        let target_len = {
            let view = tr.state().view();
            view.node(target_id)
                .ok_or(CommandError::NodeNotFound(target_id))?
                .child_blocks()
                .filter(|b| b.id().as_op_dot().is_some())
                .count()
        };
        tr.move_node(list_item_id, target_id, target_len)?;

        new_item_id = {
            let view = tr.state().view();
            view.node(target_id)
                .and_then(|t| t.child_blocks().nth(target_len))
                .map(|b| b.id())
        };

        let fulfill_steps = {
            let view = tr.state().view();
            view.node(prev_id).map(|p| fulfill(&p)).unwrap_or_default()
        };
        tr.apply_steps(fulfill_steps)?;
        Ok(())
    })?;

    Ok(new_item_id)
}

pub(crate) struct SelectionAnchor {
    item_index: usize,
    path: Vec<usize>,
    offset: usize,
    affinity: Affinity,
}

/// Captures the anchor/head positions relative to the top-level `items` being
/// restructured, so they can be re-resolved after `move_node` re-emits the
/// items with fresh dots.
pub(crate) fn capture_selection_anchors(
    view: &DocView,
    items: &[Dot],
    selection: &Selection,
) -> Option<(SelectionAnchor, SelectionAnchor)> {
    Some((
        capture_anchor(view, items, &selection.anchor)?,
        capture_anchor(view, items, &selection.head)?,
    ))
}

fn capture_anchor(view: &DocView, items: &[Dot], pos: &Position) -> Option<SelectionAnchor> {
    items.iter().enumerate().find_map(|(item_index, item)| {
        super::path_from_ancestor(view, pos.node, *item).map(|path| SelectionAnchor {
            item_index,
            path,
            offset: pos.offset,
            affinity: pos.affinity,
        })
    })
}

pub(crate) fn restore_selection_anchors(
    view: &DocView,
    new_items: &[Option<Dot>],
    anchor: &SelectionAnchor,
    head: &SelectionAnchor,
) -> Option<Selection> {
    Some(Selection::new(
        restore_anchor(view, new_items, anchor)?,
        restore_anchor(view, new_items, head)?,
    ))
}

fn restore_anchor(
    view: &DocView,
    new_items: &[Option<Dot>],
    cap: &SelectionAnchor,
) -> Option<Position> {
    let item = new_items.get(cap.item_index)?.as_ref()?;
    let mut node = *item;
    for &idx in &cap.path {
        match view.node(node)?.child_at(idx)? {
            editor_model::ChildView::Block(b) => node = b.id(),
            editor_model::ChildView::Leaf(_) => return None,
        }
    }
    Some(Position {
        node,
        offset: cap.offset,
        affinity: cap.affinity,
    })
}

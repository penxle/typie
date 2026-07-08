use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView, Subtree};
use editor_state::{
    Affinity, Position, ResolvedSelection, Selection, StableResolveCtx, StableSelection,
};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub(crate) fn is_list_type(ty: NodeType) -> bool {
    matches!(ty, NodeType::BulletList | NodeType::OrderedList)
}

#[derive(Clone)]
enum LiftedListItemTarget {
    ListItem(Dot),
    Children(Vec<Dot>),
}

pub(crate) fn lift_list_item_inner(tr: &mut Transaction, list_item_id: Dot) -> CommandResult {
    let captured_head = {
        let view = tr.view();
        tr.selection()
            .and_then(|selection| capture_anchor(&view, &[list_item_id], &selection.head))
    };
    let Some(target) = lift_list_item_to_parent(tr, list_item_id)? else {
        return Ok(false);
    };
    let position = {
        let view = tr.view();
        captured_head
            .as_ref()
            .and_then(|head| restore_lift_anchor_in_target(&view, &target, head))
            .or_else(|| first_position_in_lift_target(&view, &target))
    };
    if let Some(position) = position {
        tr.set_selection(Some(Selection::collapsed(position)))?;
    }
    Ok(true)
}

fn lift_list_item_to_parent(
    tr: &mut Transaction,
    list_item_id: Dot,
) -> Result<Option<LiftedListItemTarget>, CommandError> {
    let (list_id, list_type, owner_id, owner_is_list_item, list_index, after_items) = {
        let view = tr.state().view();
        let list_item = view
            .node(list_item_id)
            .ok_or(CommandError::NodeNotFound(list_item_id))?;
        if list_item.node_type() != NodeType::ListItem {
            return Ok(None);
        }

        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        let list_id = list.id();
        let list_type = list.node_type();
        if !is_list_type(list_type) {
            return Ok(None);
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

        (
            list_id,
            list_type,
            owner_id,
            owner_is_list_item,
            list_index,
            after_items,
        )
    };

    let mut lifted_target: Option<LiftedListItemTarget> = None;

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
            lifted_target = Some(LiftedListItemTarget::ListItem(moved_item_id));

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
            let mut moved_children = Vec::with_capacity(child_count);
            for (i, child_id) in children.iter().enumerate() {
                let target_index = list_index + 1 + i;
                tr.move_node(*child_id, owner_id, target_index)?;
                let moved_child_id = {
                    let view = tr.state().view();
                    view.node(owner_id)
                        .and_then(|o| match o.child_at(target_index) {
                            Some(ChildView::Block(p)) => Some(p.id()),
                            _ => None,
                        })
                        .ok_or(CommandError::NodeNotFound(owner_id))?
                };
                moved_children.push(moved_child_id);
            }
            if !moved_children.is_empty() {
                lifted_target = Some(LiftedListItemTarget::Children(moved_children));
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
                        .and_then(|o| match o.child_at(list_index + 1 + child_count) {
                            Some(ChildView::Block(b)) => Some(b.id()),
                            _ => None,
                        })
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

    Ok(lifted_target)
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

pub(crate) fn collect_list_items_in_selection(rs: &ResolvedSelection<'_>) -> Vec<Dot> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    if let Some(root) = rs.view().root() {
        collect_list_items_in_block(rs, &root, &mut items, &mut seen);
    }
    items
}

fn collect_list_items_in_block(
    rs: &ResolvedSelection<'_>,
    node: &NodeView<'_>,
    out: &mut Vec<Dot>,
    seen: &mut HashSet<Dot>,
) {
    if !rs.intersects_subtree(node) {
        return;
    }

    if node.node_type() == NodeType::ListItem
        && list_item_own_paragraph_intersects(rs, node)
        && seen.insert(node.id())
    {
        out.push(node.id());
    }

    for child in node.child_blocks() {
        collect_list_items_in_block(rs, &child, out, seen);
    }
}

pub(crate) fn list_item_own_paragraph_intersects(
    rs: &ResolvedSelection<'_>,
    item: &NodeView<'_>,
) -> bool {
    item.child_blocks()
        .next()
        .map(|paragraph| rs.intersects_subtree(&paragraph))
        .unwrap_or_else(|| rs.intersects_subtree(item))
}

pub(crate) fn sort_list_items_for_lift(view: &DocView, items: &mut [Dot]) {
    let order: Vec<Dot> = items.to_vec();
    items.sort_by(|a, b| {
        list_item_depth(view, *b)
            .cmp(&list_item_depth(view, *a))
            .then_with(|| item_order(&order, *b).cmp(&item_order(&order, *a)))
    });
}

fn item_order(items: &[Dot], item: Dot) -> usize {
    items
        .iter()
        .position(|id| *id == item)
        .unwrap_or(usize::MAX)
}

fn retain_topmost_list_items(view: &DocView, items: &mut Vec<Dot>) {
    let selected: HashSet<Dot> = items.iter().copied().collect();
    items.retain(|item_id| {
        let Some(item) = view.node(*item_id) else {
            return false;
        };
        item.ancestors().skip(1).all(|ancestor| {
            ancestor.node_type() != NodeType::ListItem || !selected.contains(&ancestor.id())
        })
    });
}

pub(crate) fn list_item_depth(view: &DocView, item_id: Dot) -> usize {
    view.node(item_id)
        .map(|item| {
            item.ancestors()
                .filter(|ancestor| ancestor.node_type() == NodeType::ListItem)
                .count()
        })
        .unwrap_or_default()
}

pub(crate) fn list_item_parent_list_id(view: &DocView, item_id: Dot) -> Option<Dot> {
    let item = view.node(item_id)?;
    let parent = item.parent()?;
    if is_list_type(parent.node_type()) {
        Some(parent.id())
    } else {
        None
    }
}

pub(crate) fn first_list_item_paragraph_id(view: &DocView, item_id: Dot) -> Option<Dot> {
    view.node(item_id)
        .and_then(|item| item.child_blocks().next())
        .map(|paragraph| paragraph.id())
}

pub(crate) fn lift_selected_list_items(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        let list_item_id = {
            let view = tr.view();
            find_enclosing_list_item_id(&view, selection.head.node)
        };
        let Some(list_item_id) = list_item_id else {
            return Ok(false);
        };
        return lift_list_item_inner(tr, list_item_id);
    }

    let mut items = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        collect_list_items_in_selection(&resolved)
    };
    if items.is_empty() {
        return Ok(false);
    }
    {
        let view = tr.view();
        retain_topmost_list_items(&view, &mut items);
    }
    if items.is_empty() {
        return Ok(false);
    }
    let stable_selection = StableSelection::capture(&selection, &tr.view());
    {
        let view = tr.view();
        sort_list_items_for_lift(&view, &mut items);
    }

    for item_id in items.iter() {
        let exists = {
            let view = tr.view();
            view.node(*item_id).is_some()
        };
        if !exists {
            continue;
        }
        lift_list_item_to_parent(tr, *item_id)?;
    }

    let sel = {
        let view = tr.view();
        let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
        stable_selection.resolve(&ctx)
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list selection".into()))?;
    tr.set_selection(Some(sel))?;

    Ok(true)
}

pub(crate) fn sink_selected_list_items(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let items = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        collect_list_items_in_selection(&resolved)
    };
    if items.is_empty() {
        return Ok(false);
    }

    let stable_selection = StableSelection::capture(&selection, &tr.view());
    let mut groups = {
        let view = tr.view();
        group_list_items_by_parent(&view, &items)
    };
    groups.sort_by(|a, b| {
        b.depth
            .cmp(&a.depth)
            .then_with(|| a.first_index.cmp(&b.first_index))
    });

    let mut any_sunk = false;
    for group in groups {
        let first_has_prev = {
            let view = tr.view();
            view.node(group.items[0])
                .and_then(|item| item.index())
                .map(|index| index > 0)
                .unwrap_or(false)
        };

        if !first_has_prev {
            continue;
        }

        for item_id in group.items {
            let exists = {
                let view = tr.view();
                view.node(item_id).is_some()
            };
            if !exists {
                continue;
            }
            let new_id = sink_list_item_inner(tr, item_id)?;
            if new_id.is_some() {
                any_sunk = true;
            }
        }
    }
    if !any_sunk {
        return Ok(!selection.is_collapsed());
    }

    let sel = {
        let view = tr.view();
        let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
        stable_selection.resolve(&ctx)
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list selection".into()))?;
    tr.set_selection(Some(sel))?;

    Ok(true)
}

struct ListItemGroup {
    depth: usize,
    first_index: usize,
    items: Vec<Dot>,
}

fn group_list_items_by_parent(view: &DocView, items: &[Dot]) -> Vec<ListItemGroup> {
    let mut groups: Vec<(Dot, ListItemGroup)> = Vec::new();
    for (item_index, item_id) in items.iter().copied().enumerate() {
        let Some(parent_id) = list_item_parent_list_id(view, item_id) else {
            continue;
        };
        if let Some((_, group)) = groups.iter_mut().find(|(id, _)| *id == parent_id) {
            group.items.push(item_id);
            continue;
        }
        groups.push((
            parent_id,
            ListItemGroup {
                depth: list_item_depth(view, item_id),
                first_index: item_index,
                items: vec![item_id],
            },
        ));
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn restore_lift_anchor_in_target(
    view: &DocView,
    target: &LiftedListItemTarget,
    cap: &SelectionAnchor,
) -> Option<Position> {
    match target {
        LiftedListItemTarget::ListItem(root) => {
            restore_anchor_from_root(view, *root, &cap.path, cap.offset, cap.affinity)
        }
        LiftedListItemTarget::Children(roots) => {
            let (&child_index, path) = cap.path.split_first()?;
            let root = *roots.get(child_index)?;
            restore_anchor_from_root(view, root, path, cap.offset, cap.affinity)
        }
    }
}

fn first_position_in_lift_target(
    view: &DocView,
    target: &LiftedListItemTarget,
) -> Option<Position> {
    match target {
        LiftedListItemTarget::ListItem(root) => first_position_in_lift_root(view, *root),
        LiftedListItemTarget::Children(roots) => roots
            .iter()
            .find_map(|root| first_position_in_lift_root(view, *root)),
    }
}

fn first_position_in_lift_root(view: &DocView, root: Dot) -> Option<Position> {
    let node = match view.node(root)?.node_type() {
        NodeType::ListItem => first_list_item_paragraph_id(view, root)?,
        _ => root,
    };
    Some(Position {
        node,
        offset: 0,
        affinity: Affinity::Downstream,
    })
}

fn restore_anchor_from_root(
    view: &DocView,
    root: Dot,
    path: &[usize],
    offset: usize,
    affinity: Affinity,
) -> Option<Position> {
    let mut node = root;
    for &idx in path {
        match view.node(node)?.child_at(idx)? {
            ChildView::Block(block) => node = block.id(),
            ChildView::Leaf(_) => return None,
        }
    }
    Some(Position {
        node,
        offset,
        affinity,
    })
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

struct SelectionAnchor {
    path: Vec<usize>,
    offset: usize,
    affinity: Affinity,
}

fn capture_anchor(view: &DocView, items: &[Dot], pos: &Position) -> Option<SelectionAnchor> {
    items.iter().find_map(|item| {
        super::path_from_ancestor(view, pos.node, *item).map(|path| SelectionAnchor {
            path,
            offset: pos.offset,
            affinity: pos.affinity,
        })
    })
}

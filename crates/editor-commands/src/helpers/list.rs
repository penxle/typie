use hashbrown::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView, Subtree};
use editor_state::{
    Affinity, Position, ResolvedSelection, Selection, StableResolveCtx, StableSelection,
};
use editor_transaction::{Step, StepError, Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub(crate) fn is_list_type(ty: NodeType) -> bool {
    matches!(ty, NodeType::BulletList | NodeType::OrderedList)
}

#[derive(Clone)]
enum LiftedListItemTarget {
    ListItem(Dot),
    Children(Vec<Dot>),
}

/// A child counts as "real" content (as opposed to a schema-driven synthetic
/// scaffold, or a dot already scheduled for removal by an earlier item in the
/// same batch — see [`finish_lift`]) for every emptiness/count computation a
/// lift plan needs.
fn is_real_child(scheduled: &HashSet<Dot>, id: Dot) -> bool {
    id.as_op_dot().is_some() && !scheduled.contains(&id)
}

fn real_child_count(scheduled: &HashSet<Dot>, node: &NodeView) -> usize {
    node.child_blocks()
        .filter(|b| is_real_child(scheduled, b.id()))
        .count()
}

fn is_empty_of_real_content(scheduled: &HashSet<Dot>, node: &NodeView) -> bool {
    real_child_count(scheduled, node) == 0
}

/// `list`'s block siblings after `list_item_index`, excluding any dot already
/// scheduled for removal — the plan-builder's single point for this
/// computation (a scheduled sibling was already lifted by an earlier item in
/// the same multi-item batch and must not be treated as still-live content).
fn after_siblings(list: &NodeView, scheduled: &HashSet<Dot>, list_item_index: usize) -> Vec<Dot> {
    list.child_blocks()
        .skip(list_item_index + 1)
        .map(|c| c.id())
        .filter(|id| !scheduled.contains(id))
        .collect()
}

enum ItemPlanKind {
    NestedUnderListItem {
        outer_list_id: Dot,
        outer_index: usize,
        own_child_count: usize,
        existing_sublist: Option<(Dot, usize)>,
    },
    TopLevel {
        children: Vec<Dot>,
    },
}

struct ItemPlan {
    list_id: Dot,
    list_type: NodeType,
    owner_id: Dot,
    list_index: usize,
    after_items: Vec<Dot>,
    kind: ItemPlanKind,
}

/// Builds the whole plan for lifting `list_item_id` from a single clean view —
/// every child/index/count read a lift needs (`list.index()`, owner child
/// count, children, after-items, emptiness) goes through this one function,
/// filtered by `scheduled` (see [`is_real_child`]/[`after_siblings`]).
fn build_item_plan(
    view: &DocView,
    list_item_id: Dot,
    scheduled: &HashSet<Dot>,
) -> Result<Option<ItemPlan>, CommandError> {
    let Some(list_item) = view.node(list_item_id) else {
        return Ok(None);
    };
    if list_item.node_type() != NodeType::ListItem {
        return Ok(None);
    }

    let Some(list) = list_item.parent() else {
        return Err(CommandError::NoParent(list_item_id));
    };
    let list_id = list.id();
    let list_type = list.node_type();
    if !is_list_type(list_type) {
        return Ok(None);
    }

    let Some(owner) = list.parent() else {
        return Err(CommandError::NoParent(list_id));
    };
    let owner_id = owner.id();
    let owner_is_list_item = owner.node_type() == NodeType::ListItem;

    let list_item_index = list_item
        .index()
        .ok_or_else(|| CommandError::orphan_child(list_item_id, list_id))?;
    let list_index = list
        .index()
        .ok_or_else(|| CommandError::orphan_child(list_id, owner_id))?;

    let after_items = after_siblings(&list, scheduled, list_item_index);

    let kind = if owner_is_list_item {
        let outer_list = owner.parent().ok_or(CommandError::NoParent(owner_id))?;
        let outer_list_id = outer_list.id();
        let outer_index = owner
            .index()
            .ok_or_else(|| CommandError::orphan_child(owner_id, outer_list_id))?;
        let own_child_count = list_item.child_blocks().count();
        let existing_sublist = list_item
            .child_blocks()
            .find(|c| is_list_type(c.node_type()))
            .map(|c| (c.id(), real_child_count(scheduled, &c)));
        ItemPlanKind::NestedUnderListItem {
            outer_list_id,
            outer_index,
            own_child_count,
            existing_sublist,
        }
    } else {
        let children: Vec<Dot> = list_item.child_blocks().map(|c| c.id()).collect();
        ItemPlanKind::TopLevel { children }
    };

    Ok(Some(ItemPlan {
        list_id,
        list_type,
        owner_id,
        list_index,
        after_items,
        kind,
    }))
}

pub(crate) fn lift_list_item_inner(tr: &mut Transaction, list_item_id: Dot) -> CommandResult {
    let captured_head = {
        let view = tr.view();
        tr.selection()
            .and_then(|selection| capture_anchor(&view, &[list_item_id], &selection.head))
    };
    let mut scheduled: HashSet<Dot> = HashSet::new();
    let mut owners: HashSet<Dot> = HashSet::new();
    let Some(target) = lift_list_item_to_parent(tr, list_item_id, &mut scheduled, &mut owners)?
    else {
        return Ok(false);
    };
    finish_lift(tr, scheduled, owners)?;
    let position = {
        let view = tr.view_clean().map_err(StepError::from)?;
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

/// Lifts a single list item toward its parent: reads the whole plan from one
/// clean view ([`build_item_plan`]), then emits the moves via the composite
/// steps (`move_nodes_consecutive`/`insert_subtree_with_moved`), scheduling
/// the emptied `list_item`/`list` for [`finish_lift`] instead of removing them
/// in place. Both the multi-item ([`lift_list_items`]) and single-item
/// ([`lift_list_item_inner`]) callers share `scheduled`/`owners` across every
/// item they process and call `finish_lift` once, after their whole loop.
fn lift_list_item_to_parent(
    tr: &mut Transaction,
    list_item_id: Dot,
    scheduled: &mut HashSet<Dot>,
    owners: &mut HashSet<Dot>,
) -> Result<Option<LiftedListItemTarget>, CommandError> {
    let plan = {
        let view = tr.view_clean().map_err(StepError::from)?;
        build_item_plan(&view, list_item_id, scheduled)?
    };
    let Some(ItemPlan {
        list_id,
        list_type,
        owner_id,
        list_index,
        after_items,
        kind,
    }) = plan
    else {
        return Ok(None);
    };

    owners.insert(owner_id);
    scheduled.insert(list_id);

    let lifted_target = match kind {
        ItemPlanKind::NestedUnderListItem {
            outer_list_id,
            outer_index,
            own_child_count,
            existing_sublist,
        } => {
            let moved = tr.move_node(list_item_id, outer_list_id, outer_index + 1)?;
            let moved_item_id = moved.root;
            let target = LiftedListItemTarget::ListItem(moved_item_id);

            if !after_items.is_empty() {
                match existing_sublist {
                    Some((old_sublist_id, base)) => {
                        let new_sublist_id = moved
                            .pairs
                            .iter()
                            .find(|(old, _)| *old == old_sublist_id)
                            .map(|(_, new)| *new)
                            .ok_or(CommandError::NodeNotFound(old_sublist_id))?;
                        tr.projected_clean().map_err(StepError::from)?;
                        tr.move_nodes_consecutive(&after_items, new_sublist_id, base)?;
                    }
                    None => {
                        let new_sublist_node = list_type.into_node().to_plain();
                        tr.projected_clean().map_err(StepError::from)?;
                        tr.insert_subtree_with_moved(
                            moved_item_id,
                            own_child_count,
                            Subtree::leaf(new_sublist_node),
                            &after_items,
                        )?;
                    }
                }
            }
            Some(target)
        }
        ItemPlanKind::TopLevel { children } => {
            let moved_children: Vec<Dot> = if children.is_empty() {
                Vec::new()
            } else {
                tr.move_nodes_consecutive(&children, owner_id, list_index + 1)?
                    .into_iter()
                    .map(|m| m.root)
                    .collect()
            };
            scheduled.insert(list_item_id);

            if !after_items.is_empty() {
                let new_list_node = list_type.into_node().to_plain();
                if !children.is_empty() {
                    tr.projected_clean().map_err(StepError::from)?;
                }
                tr.insert_subtree_with_moved(
                    owner_id,
                    list_index + 1 + children.len(),
                    Subtree::leaf(new_list_node),
                    &after_items,
                )?;
            }

            if moved_children.is_empty() {
                None
            } else {
                Some(LiftedListItemTarget::Children(moved_children))
            }
        }
    };

    Ok(lifted_target)
}

/// Shared cleanup for both lift paths: depth-descending rounds that
/// re-flush, re-judge every still-`scheduled` dot against a clean view (using
/// the same [`is_empty_of_real_content`] predicate the plan builder uses, so
/// a container refilled since scheduling is not removed), and bulk-remove
/// each round's confirmed layer in one pass — followed by one batched
/// `fulfill` pass over every touched owner. A confirmed dot whose ancestor is
/// also confirmed in the same round is dropped from the removal batch (kept
/// only as the topmost one): the ancestor's own `Step::RemoveSubtree` already
/// captures/removes that whole subtree, so keeping both would double-delete
/// the same dots. Selection restoration stays with the caller (the
/// multi-item and single-item paths use different recovery machinery).
fn finish_lift(
    tr: &mut Transaction,
    mut scheduled: HashSet<Dot>,
    owners: HashSet<Dot>,
) -> Result<(), CommandError> {
    loop {
        let (all_confirmed, topmost, steps): (Vec<Dot>, Vec<Dot>, Vec<Step>) = {
            let ps = tr.projected_clean().map_err(StepError::from)?;
            let view = ps.view();
            let all_confirmed: Vec<Dot> = scheduled
                .iter()
                .copied()
                .filter(|&dot| {
                    view.node(dot)
                        .is_some_and(|node| is_empty_of_real_content(&scheduled, &node))
                })
                .collect();
            let confirmed_set: HashSet<Dot> = all_confirmed.iter().copied().collect();
            let topmost: Vec<Dot> = all_confirmed
                .iter()
                .copied()
                .filter(|&dot| {
                    view.node(dot).is_some_and(|node| {
                        node.ancestors()
                            .skip(1)
                            .all(|ancestor| !confirmed_set.contains(&ancestor.id()))
                    })
                })
                .collect();
            let mut steps = Vec::with_capacity(topmost.len());
            for &dot in &topmost {
                let node = view.node(dot).expect("just confirmed present in this view");
                let parent = node.parent().ok_or(CommandError::NoParent(dot))?;
                let index = node
                    .index()
                    .ok_or_else(|| CommandError::orphan_child(dot, parent.id()))?;
                let subtree = editor_transaction::capture_subtree(ps, dot)
                    .ok_or(CommandError::NodeNotFound(dot))?;
                steps.push(Step::RemoveSubtree {
                    parent: parent.id(),
                    index,
                    subtree,
                });
            }
            (all_confirmed, topmost, steps)
        };
        if topmost.is_empty() {
            break;
        }
        for dot in &all_confirmed {
            scheduled.remove(dot);
        }
        tr.apply_steps_bulk_delete(steps)?;
    }

    let fulfill_steps: Vec<Step> = {
        let ps = tr.projected_clean().map_err(StepError::from)?;
        let view = ps.view();
        owners
            .iter()
            .filter_map(|&id| view.node(id))
            .flat_map(|node| fulfill(&node))
            .collect()
    };
    if !fulfill_steps.is_empty() {
        tr.apply_steps_flushed(fulfill_steps)?;
    }
    Ok(())
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

pub(crate) fn retain_topmost_list_items(view: &DocView, items: &mut Vec<Dot>) {
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

pub(crate) fn lift_list_items_planned(tr: &mut Transaction, mut items: Vec<Dot>) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let stable_selection = StableSelection::capture(&selection, &tr.view());
    {
        let view = tr.view();
        sort_list_items_for_lift(&view, &mut items);
    }

    let mut scheduled: HashSet<Dot> = HashSet::new();
    let mut owners: HashSet<Dot> = HashSet::new();
    for item_id in items.iter() {
        lift_list_item_to_parent(tr, *item_id, &mut scheduled, &mut owners)?;
    }
    finish_lift(tr, scheduled, owners)?;

    let sel = {
        let ps = tr.projected_clean().map_err(StepError::from)?;
        let view = ps.view();
        let ctx = StableResolveCtx::from_live(&view, ps.seq_checkout());
        stable_selection.resolve(&ctx)
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list selection".into()))?;
    tr.set_selection(Some(sel))?;

    Ok(true)
}

pub(crate) struct ListItemGroup {
    pub(crate) depth: usize,
    pub(crate) first_index: usize,
    pub(crate) items: Vec<Dot>,
}

pub(crate) fn group_list_items_by_parent(view: &DocView, items: &[Dot]) -> Vec<ListItemGroup> {
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

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView, PlainNodeEntry, Subtree};
use editor_state::{Affinity, Position, ResolvedSelection, Selection};
use editor_transaction::Transaction;

use crate::helpers::{is_list_type, list_item_own_paragraph_intersects, path_from_ancestor};
use crate::{CommandError, CommandResult};

pub fn set_list_kind(tr: &mut Transaction, target_list_type: NodeType) -> CommandResult {
    if !is_list_type(target_list_type) {
        return Ok(false);
    }

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.is_collapsed() {
        return set_collapsed_list_kind(tr, target_list_type, selection.head.node);
    }

    set_range_list_kind(tr, target_list_type, selection)
}

fn set_collapsed_list_kind(
    tr: &mut Transaction,
    target_list_type: NodeType,
    cursor_node: Dot,
) -> CommandResult {
    let list_id = {
        let view = tr.view();
        find_enclosing_list_id(&view, cursor_node)
    };
    let Some(list_id) = list_id else {
        return Ok(false);
    };

    set_existing_list_kind(tr, list_id, target_list_type)
}

fn set_range_list_kind(
    tr: &mut Transaction,
    target_list_type: NodeType,
    selection: Selection,
) -> CommandResult {
    let list_ids = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        collect_existing_lists_in_range(&resolved)
    };
    if list_ids.is_empty() {
        return Ok(false);
    }

    let mut changed = false;

    for list_id in list_ids {
        if tr.view().node(list_id).is_some() {
            changed |= set_existing_list_kind(tr, list_id, target_list_type)?;
        }
    }

    let runs = {
        let selection = tr
            .selection()
            .ok_or(CommandError::Corrupted("missing selection".into()))?;
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        collect_listable_wrap_runs(&resolved, target_list_type)
    };
    for run in runs {
        if run_still_exists(&tr.view(), &run) {
            changed |= wrap_run_into_list(tr, &run, target_list_type)?;
        }
    }

    Ok(changed)
}

fn find_enclosing_list_id(view: &DocView, node: Dot) -> Option<Dot> {
    let mut current = view.node(node)?;
    loop {
        if is_list_type(current.node_type()) {
            return Some(current.id());
        }
        current = current.parent()?;
    }
}

fn set_existing_list_kind(
    tr: &mut Transaction,
    list_id: Dot,
    target_list_type: NodeType,
) -> CommandResult {
    let (parent_id, current_type) = {
        let view = tr.view();
        let list = view
            .node(list_id)
            .ok_or(CommandError::NodeNotFound(list_id))?;
        let parent = list.parent().ok_or(CommandError::NoParent(list_id))?;
        (parent.id(), list.node_type())
    };
    if !is_list_type(current_type) || current_type == target_list_type {
        return Ok(false);
    }

    replace_list_with_kind(tr, parent_id, list_id, target_list_type)
}

fn collect_existing_lists_in_range(rs: &ResolvedSelection<'_>) -> Vec<Dot> {
    let mut ids = Vec::new();
    if let Some(root) = rs.view().root() {
        collect_existing_lists_in_block(rs, &root, &mut ids);
    }
    ids.sort_by_key(|id| std::cmp::Reverse(list_depth(rs.view(), *id)));
    ids
}

fn collect_existing_lists_in_block(
    rs: &ResolvedSelection<'_>,
    node: &NodeView<'_>,
    out: &mut Vec<Dot>,
) {
    if !rs.intersects_subtree(node) {
        return;
    }

    if is_list_type(node.node_type()) && list_has_selected_direct_item(rs, node) {
        out.push(node.id());
    }

    for child in node.child_blocks() {
        collect_existing_lists_in_block(rs, &child, out);
    }
}

fn list_has_selected_direct_item(rs: &ResolvedSelection<'_>, list: &NodeView<'_>) -> bool {
    list.child_blocks().any(|item| {
        item.node_type() == NodeType::ListItem && list_item_own_paragraph_intersects(rs, &item)
    })
}

fn list_depth(view: &DocView, list_id: Dot) -> usize {
    view.node(list_id)
        .map(|list| {
            list.ancestors()
                .filter(|ancestor| is_list_type(ancestor.node_type()))
                .count()
        })
        .unwrap_or_default()
}

#[derive(Clone)]
struct ListableWrapRun {
    parent_id: Dot,
    first_child_id: Dot,
    children: Vec<ListableRunChild>,
}

#[derive(Clone)]
struct ListableRunChild {
    id: Dot,
    node_type: NodeType,
}

fn collect_listable_wrap_runs(
    rs: &ResolvedSelection<'_>,
    target_list_type: NodeType,
) -> Vec<ListableWrapRun> {
    let mut runs = Vec::new();
    if let Some(root) = rs.view().root() {
        collect_listable_wrap_runs_in_block(rs, &root, target_list_type, &mut runs);
    }
    runs
}

fn collect_listable_wrap_runs_in_block(
    rs: &ResolvedSelection<'_>,
    parent: &NodeView<'_>,
    target_list_type: NodeType,
    out: &mut Vec<ListableWrapRun>,
) {
    if !rs.intersects_subtree(parent) {
        return;
    }

    if parent.node_type() != NodeType::ListItem && parent.spec().content.matches(target_list_type) {
        collect_direct_child_runs(rs, parent, out);
    }

    for child in parent.child_blocks() {
        collect_listable_wrap_runs_in_block(rs, &child, target_list_type, out);
    }
}

fn collect_direct_child_runs(
    rs: &ResolvedSelection<'_>,
    parent: &NodeView<'_>,
    out: &mut Vec<ListableWrapRun>,
) {
    let mut current = Vec::new();

    for (slot, child) in parent.children().enumerate() {
        let selected = child_intersects_selection(rs, parent, slot, &child);
        if !selected {
            flush_run(parent.id(), &mut current, out);
            continue;
        }

        match child {
            ChildView::Block(block) if is_run_child_type(block.node_type()) => {
                current.push(ListableRunChild {
                    id: block.id(),
                    node_type: block.node_type(),
                });
            }
            _ => flush_run(parent.id(), &mut current, out),
        }
    }

    flush_run(parent.id(), &mut current, out);
}

fn child_intersects_selection(
    rs: &ResolvedSelection<'_>,
    parent: &NodeView<'_>,
    slot: usize,
    child: &ChildView<'_>,
) -> bool {
    match child {
        ChildView::Block(block) if is_list_type(block.node_type()) => {
            list_has_selected_direct_item(rs, block)
        }
        ChildView::Block(block) => rs.intersects_subtree(block),
        ChildView::Leaf(_) => rs.contains_leaf_slot(parent, slot),
    }
}

fn flush_run(parent_id: Dot, current: &mut Vec<ListableRunChild>, out: &mut Vec<ListableWrapRun>) {
    if should_wrap_run(current) {
        out.push(ListableWrapRun {
            parent_id,
            first_child_id: current[0].id,
            children: std::mem::take(current),
        });
    } else {
        current.clear();
    }
}

fn should_wrap_run(children: &[ListableRunChild]) -> bool {
    if children.is_empty() {
        return false;
    }
    children.len() > 1
        || children
            .iter()
            .any(|child| child.node_type == NodeType::Paragraph)
}

fn is_run_child_type(node_type: NodeType) -> bool {
    node_type == NodeType::Paragraph || is_list_type(node_type)
}

fn run_still_exists(view: &DocView, run: &ListableWrapRun) -> bool {
    run.children.iter().all(|child| {
        view.node(child.id)
            .is_some_and(|node| node.parent().is_some_and(|p| p.id() == run.parent_id))
    })
}

fn wrap_run_into_list(
    tr: &mut Transaction,
    run: &ListableWrapRun,
    target_list_type: NodeType,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let move_roots = {
        let view = tr.view();
        collect_run_move_roots(&view, run)?
    };
    if move_roots.is_empty() {
        return Ok(false);
    }

    let (insert_index, child_root_counts) = {
        let view = tr.view();
        (
            run_insert_index(&view, run)?,
            collect_run_child_root_counts(&view, run)?,
        )
    };

    // TODO(move-node-alias): Replace this run-relative selection restore with
    // StableSelection once move_node can resolve old dots to re-emitted dots.
    // Parent-slot endpoints around wrapped direct children are translated here
    // manually because the run collapses to one list at the parent.
    let (captured_anchor, captured_head) = {
        let view = tr.view();
        (
            capture_wrapped_run_endpoint(
                &view,
                run,
                insert_index,
                &child_root_counts,
                &move_roots,
                &selection.anchor,
            ),
            capture_wrapped_run_endpoint(
                &view,
                run,
                insert_index,
                &child_root_counts,
                &move_roots,
                &selection.head,
            ),
        )
    };

    let mut new_roots = vec![None; move_roots.len()];
    let mut new_list_id = None;

    tr.batch::<_, CommandError>(|tr| {
        tr.insert_subtree(
            run.parent_id,
            insert_index,
            Subtree::leaf(target_list_type.into_node().to_plain()),
        )?;
        let inserted_list_id = block_child_id_at(tr, run.parent_id, insert_index)?;
        new_list_id = Some(inserted_list_id);

        let mut root_index = 0;
        for child in &run.children {
            match child.node_type {
                NodeType::Paragraph => {
                    let paragraph_id = child.id;
                    append_new_list_item_with_paragraph(
                        tr,
                        inserted_list_id,
                        paragraph_id,
                        root_index,
                        &mut new_roots,
                    )?;
                    root_index += 1;
                }
                ty if is_list_type(ty) => {
                    let item_ids = real_list_item_ids(&tr.view(), child.id)?;
                    for item_id in item_ids {
                        append_existing_list_item(
                            tr,
                            inserted_list_id,
                            item_id,
                            root_index,
                            &mut new_roots,
                        )?;
                        root_index += 1;
                    }
                    if tr.view().node(child.id).is_some() {
                        tr.remove_subtree(child.id)?;
                    }
                }
                _ => {}
            }
        }

        let fulfill_steps = {
            let view = tr.view();
            let mut steps = Vec::new();
            if let Some(list) = view.node(inserted_list_id) {
                steps.extend(editor_transaction::fulfill(&list));
            }
            if let Some(parent) = view.node(run.parent_id) {
                steps.extend(editor_transaction::fulfill(&parent));
            }
            steps
        };
        tr.apply_steps(fulfill_steps)?;

        Ok(())
    })?;

    let restored = {
        let view = tr.view();
        restore_run_selection_endpoints(
            &view,
            &new_roots,
            new_list_id,
            &captured_anchor,
            &captured_head,
        )
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list kind selection".into()))?;
    tr.set_selection(Some(restored))?;

    Ok(true)
}

fn replace_list_with_kind(
    tr: &mut Transaction,
    parent_id: Dot,
    list_id: Dot,
    target_list_type: NodeType,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let (index, subtree) = {
        let view = tr.view();
        let list = view
            .node(list_id)
            .ok_or(CommandError::NodeNotFound(list_id))?;
        let index = list
            .index()
            .ok_or_else(|| CommandError::orphan_child(list_id, parent_id))?;
        let subtree = capture_plain_subtree_as(&view, tr.state(), list_id, target_list_type)?;
        (index, subtree)
    };

    // TODO(move-node-alias): Replace this list-root-relative selection restore with
    // StableSelection once remove/insert replacement can resolve old dots to new dots.
    let (captured_anchor, captured_head) = {
        let view = tr.view();
        (
            capture_run_endpoint(&view, &[list_id], &selection.anchor),
            capture_run_endpoint(&view, &[list_id], &selection.head),
        )
    };

    let new_list_id = {
        tr.batch::<_, CommandError>(|tr| {
            tr.remove_subtree(list_id)?;
            tr.insert_subtree(parent_id, index, subtree)?;
            let fulfill_steps = {
                let view = tr.view();
                view.node(parent_id)
                    .map(|parent| editor_transaction::fulfill(&parent))
                    .unwrap_or_default()
            };
            tr.apply_steps(fulfill_steps)?;
            Ok(())
        })?;
        block_child_id_at(tr, parent_id, index)?
    };

    let restored = {
        let view = tr.view();
        restore_run_selection_endpoints(
            &view,
            &[Some(new_list_id)],
            Some(new_list_id),
            &captured_anchor,
            &captured_head,
        )
    }
    .ok_or_else(|| CommandError::Corrupted("cannot restore list kind selection".into()))?;
    tr.set_selection(Some(restored))?;

    Ok(true)
}

fn capture_plain_subtree_as(
    view: &DocView,
    state: &editor_state::State,
    block: Dot,
    node_type: NodeType,
) -> Result<Subtree, CommandError> {
    let path = node_path_from_root(view, block)
        .ok_or_else(|| CommandError::Corrupted("cannot locate list subtree".into()))?;
    let plain = state.to_plain();
    let entry = plain_entry_at_path(&plain.root, &path)
        .ok_or_else(|| CommandError::Corrupted("cannot capture list subtree".into()))?;
    let mut subtree = plain_entry_to_subtree(entry.clone());
    subtree.node = node_type.into_node().to_plain();
    Ok(subtree)
}

fn node_path_from_root(view: &DocView, node: Dot) -> Option<Vec<usize>> {
    let mut path: Vec<usize> = view
        .node(node)?
        .ancestors()
        .filter_map(|n| n.index())
        .collect();
    path.reverse();
    Some(path)
}

fn plain_entry_at_path<'a>(root: &'a PlainNodeEntry, path: &[usize]) -> Option<&'a PlainNodeEntry> {
    let mut entry = root;
    for &index in path {
        entry = entry.children.get(index)?;
    }
    Some(entry)
}

fn plain_entry_to_subtree(entry: PlainNodeEntry) -> Subtree {
    Subtree {
        node: entry.node,
        modifiers: entry.modifiers.into_values().collect(),
        carry: entry.carry,
        children: entry
            .children
            .into_iter()
            .map(plain_entry_to_subtree)
            .collect(),
        source_dots: Vec::new(),
    }
}

fn collect_run_move_roots(view: &DocView, run: &ListableWrapRun) -> Result<Vec<Dot>, CommandError> {
    let mut roots = Vec::new();
    for child in &run.children {
        match child.node_type {
            NodeType::Paragraph => roots.push(child.id),
            ty if is_list_type(ty) => roots.extend(real_list_item_ids(view, child.id)?),
            _ => {}
        }
    }
    Ok(roots)
}

fn collect_run_child_root_counts(
    view: &DocView,
    run: &ListableWrapRun,
) -> Result<Vec<usize>, CommandError> {
    run.children
        .iter()
        .map(|child| match child.node_type {
            NodeType::Paragraph => Ok(1),
            ty if is_list_type(ty) => real_list_item_ids(view, child.id).map(|items| items.len()),
            _ => Ok(0),
        })
        .collect()
}

fn run_insert_index(view: &DocView, run: &ListableWrapRun) -> Result<usize, CommandError> {
    view.node(run.first_child_id)
        .ok_or(CommandError::NodeNotFound(run.first_child_id))?
        .index()
        .ok_or_else(|| CommandError::orphan_child(run.first_child_id, run.parent_id))
}

fn real_list_item_ids(view: &DocView, list_id: Dot) -> Result<Vec<Dot>, CommandError> {
    let list = view
        .node(list_id)
        .ok_or(CommandError::NodeNotFound(list_id))?;
    Ok(list
        .child_blocks()
        .filter(|child| child.id().as_op_dot().is_some())
        .map(|child| child.id())
        .collect())
}

fn append_new_list_item_with_paragraph(
    tr: &mut Transaction,
    list_id: Dot,
    paragraph_id: Dot,
    root_index: usize,
    new_roots: &mut [Option<Dot>],
) -> Result<(), CommandError> {
    let target_len = real_list_len(&tr.view(), list_id)?;
    tr.insert_subtree(
        list_id,
        target_len,
        Subtree::leaf(NodeType::ListItem.into_node().to_plain()),
    )?;
    let list_item_id = block_child_id_at(tr, list_id, target_len)?;
    tr.move_node(paragraph_id, list_item_id, 0)?;
    new_roots[root_index] = first_real_child_id(&tr.view(), list_item_id);
    Ok(())
}

fn append_existing_list_item(
    tr: &mut Transaction,
    list_id: Dot,
    item_id: Dot,
    root_index: usize,
    new_roots: &mut [Option<Dot>],
) -> Result<(), CommandError> {
    let target_len = real_list_len(&tr.view(), list_id)?;
    tr.move_node(item_id, list_id, target_len)?;
    new_roots[root_index] = block_child_id_at(tr, list_id, target_len).ok();
    Ok(())
}

fn real_list_len(view: &DocView, list_id: Dot) -> Result<usize, CommandError> {
    Ok(view
        .node(list_id)
        .ok_or(CommandError::NodeNotFound(list_id))?
        .child_blocks()
        .filter(|child| child.id().as_op_dot().is_some())
        .count())
}

fn block_child_id_at(tr: &Transaction, parent_id: Dot, index: usize) -> Result<Dot, CommandError> {
    match tr
        .view()
        .node(parent_id)
        .and_then(|parent| parent.child_at(index))
    {
        Some(ChildView::Block(block)) => Ok(block.id()),
        _ => Err(CommandError::NodeNotFound(parent_id)),
    }
}

fn first_real_child_id(view: &DocView, parent_id: Dot) -> Option<Dot> {
    view.node(parent_id)?
        .child_blocks()
        .find(|child| child.id().as_op_dot().is_some())
        .map(|child| child.id())
}

enum RunEndpoint {
    Moved(RunAnchor),
    ParentSlot(RunParentSlotAnchor),
    Unchanged(Position),
}

struct RunAnchor {
    root_index: usize,
    path: Vec<usize>,
    offset: usize,
    affinity: Affinity,
}

struct RunParentSlotAnchor {
    parent_id: Dot,
    insert_index: usize,
    root_boundary: usize,
    affinity: Affinity,
}

fn capture_wrapped_run_endpoint(
    view: &DocView,
    run: &ListableWrapRun,
    insert_index: usize,
    child_root_counts: &[usize],
    roots: &[Dot],
    pos: &Position,
) -> RunEndpoint {
    if pos.node == run.parent_id {
        let run_end = insert_index + run.children.len();
        if (insert_index..=run_end).contains(&pos.offset) {
            let child_boundary = pos.offset - insert_index;
            let root_boundary = child_root_counts.iter().take(child_boundary).sum();
            return RunEndpoint::ParentSlot(RunParentSlotAnchor {
                parent_id: run.parent_id,
                insert_index,
                root_boundary,
                affinity: pos.affinity,
            });
        }
    }

    capture_run_endpoint(view, roots, pos)
}

fn capture_run_endpoint(view: &DocView, roots: &[Dot], pos: &Position) -> RunEndpoint {
    roots
        .iter()
        .enumerate()
        .find_map(|(root_index, root)| {
            path_from_ancestor(view, pos.node, *root).map(|path| RunAnchor {
                root_index,
                path,
                offset: pos.offset,
                affinity: pos.affinity,
            })
        })
        .map(RunEndpoint::Moved)
        .unwrap_or(RunEndpoint::Unchanged(*pos))
}

fn restore_run_selection_endpoints(
    view: &DocView,
    new_roots: &[Option<Dot>],
    new_list_id: Option<Dot>,
    anchor: &RunEndpoint,
    head: &RunEndpoint,
) -> Option<Selection> {
    Some(Selection::new(
        restore_run_endpoint(view, new_roots, new_list_id, anchor)?,
        restore_run_endpoint(view, new_roots, new_list_id, head)?,
    ))
}

fn restore_run_endpoint(
    view: &DocView,
    new_roots: &[Option<Dot>],
    new_list_id: Option<Dot>,
    endpoint: &RunEndpoint,
) -> Option<Position> {
    match endpoint {
        RunEndpoint::Moved(anchor) => {
            let root = *new_roots.get(anchor.root_index)?.as_ref()?;
            restore_position_from_root(view, root, &anchor.path, anchor.offset, anchor.affinity)
        }
        RunEndpoint::ParentSlot(anchor) => {
            restore_wrapped_parent_slot(view, new_roots, new_list_id?, anchor)
        }
        RunEndpoint::Unchanged(position) => position.resolve(view).map(|_| *position),
    }
}

fn restore_wrapped_parent_slot(
    view: &DocView,
    new_roots: &[Option<Dot>],
    new_list_id: Dot,
    anchor: &RunParentSlotAnchor,
) -> Option<Position> {
    let position = if anchor.root_boundary == 0 {
        Position {
            node: anchor.parent_id,
            offset: anchor.insert_index,
            affinity: anchor.affinity,
        }
    } else if anchor.root_boundary >= new_roots.len() {
        Position {
            node: anchor.parent_id,
            offset: anchor.insert_index + 1,
            affinity: anchor.affinity,
        }
    } else {
        Position {
            node: new_list_id,
            offset: anchor.root_boundary,
            affinity: anchor.affinity,
        }
    };
    position.resolve(view).map(|_| position)
}

fn restore_position_from_root(
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

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::NodeType;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_inside_other_kind_converts_closest_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_text_restores_selection_offset() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { p1: paragraph { text("ABC") } } }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("ABC") } } }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_same_kind_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
    }

    #[test]
    fn collapsed_outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
    }

    #[test]
    fn range_paragraph_and_list_become_one_compatible_run() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    ordered_list {
                        list_item { p2: paragraph { text("B") } }
                        list_item { p3: paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { p3: paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_paragraph_and_list_restores_parent_slot_endpoint() {
        let (mut initial, p1, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    ordered_list {
                        list_item { p2: paragraph { text("B") } }
                    }
                    p3: paragraph { text("C") }
                }
            }
            selection: (p1, 0)
        };
        initial.selection = Some(Selection::new(
            Position::new(p1, 0),
            Position {
                node: Dot::ROOT,
                offset: 3,
                affinity: Affinity::Upstream,
            },
        ));

        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (mut expected, p1, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { p3: paragraph { text("C") } }
                    }
                }
            }
            selection: (p1, 0)
        };
        expected.selection = Some(Selection::new(
            Position::new(p1, 0),
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ));
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn full_document_range_with_list_restores_root_slot_endpoints() {
        let (mut initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    ordered_list {
                        list_item { p2: paragraph { text("B") } }
                    }
                    p3: paragraph { text("C") }
                }
            }
            selection: (p1, 0)
        };
        initial.selection = Some(Selection::new(
            Position {
                node: Dot::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: Dot::ROOT,
                offset: 3,
                affinity: Affinity::Upstream,
            },
        ));

        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (mut expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { p3: paragraph { text("C") } }
                    }
                }
            }
            selection: (p1, 0)
        };
        expected.selection = Some(Selection::new(
            Position {
                node: Dot::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: Dot::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ));
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_unsupported_block_splits_compatible_runs() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    image {}
                    ordered_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    image {}
                    bullet_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn blockquote_internal_paragraph_and_list_convert_inside_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        ordered_list { list_item { p2: paragraph { text("B") } } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list {
                            list_item { p1: paragraph { text("A") } }
                            list_item { p2: paragraph { text("B") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn nested_list_selection_converts_each_intersecting_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item {
                            p1: paragraph { text("A") }
                            ordered_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn nested_child_list_selection_does_not_convert_parent_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::OrderedList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            ordered_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn nested_child_list_and_trailing_paragraph_do_not_wrap_parent_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    p3: paragraph { text("C") }
                }
            }
            selection: (p2, 0) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::OrderedList));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            ordered_list { list_item { p2: paragraph { text("B") } } }
                        }
                    }
                    ordered_list { list_item { p3: paragraph { text("C") } } }
                }
            }
            selection: (p2, 0) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn same_table_cell_paragraph_and_list_convert_inside_cell() {
        let (initial, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell {
                                p1: paragraph { text("A") }
                                ordered_list { list_item { p2: paragraph { text("B") } } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
        let (expected, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell {
                                bullet_list {
                                    list_item { p1: paragraph { text("A") } }
                                    list_item { p2: paragraph { text("B") } }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pure_paragraph_range_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| set_list_kind(&mut tr, NodeType::BulletList));
    }
}

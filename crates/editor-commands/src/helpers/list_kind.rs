use hashbrown::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeType, NodeView};
use editor_state::ResolvedSelection;

use super::{is_list_type, list_item_own_paragraph_intersects};

pub(crate) fn find_enclosing_list_id(view: &DocView, node: Dot) -> Option<Dot> {
    let mut current = view.node(node)?;
    loop {
        if is_list_type(current.node_type()) {
            return Some(current.id());
        }
        current = current.parent()?;
    }
}

pub(crate) fn collect_existing_lists_in_range(rs: &ResolvedSelection<'_>) -> Vec<Dot> {
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
        if should_descend_into_child(rs, &child) {
            collect_existing_lists_in_block(rs, &child, out);
        }
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
pub(crate) struct ListableWrapRun {
    pub(crate) parent_id: Dot,
    pub(crate) first_child_id: Dot,
    pub(crate) children: Vec<ListableRunChild>,
}

#[derive(Clone)]
pub(crate) struct ListableRunChild {
    pub(crate) id: Dot,
    pub(crate) node_type: NodeType,
}

pub(crate) fn collect_listable_wrap_runs(
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
        if should_descend_into_child(rs, &child) {
            collect_listable_wrap_runs_in_block(rs, &child, target_list_type, out);
        }
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

fn should_descend_into_child(rs: &ResolvedSelection<'_>, child: &NodeView<'_>) -> bool {
    if !rs.intersects_subtree(child) {
        return false;
    }
    child.node_type() == NodeType::ListItem
        || !rs.contains_subtree(child)
        || is_run_child_type(child.node_type())
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

pub(crate) fn collect_list_items_for_kind_toggle(
    selection: &ResolvedSelection<'_>,
    view: &DocView,
) -> Vec<Dot> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    if let Some(root) = view.root() {
        collect_list_items_for_kind_toggle_in(selection, &root, &mut items, &mut seen);
    }
    items
}

fn collect_list_items_for_kind_toggle_in(
    selection: &ResolvedSelection<'_>,
    node: &NodeView<'_>,
    out: &mut Vec<Dot>,
    seen: &mut HashSet<Dot>,
) {
    if !selection.intersects_subtree(node) {
        return;
    }
    if selection.contains_subtree(node) && is_unsupported_whole_container(node.node_type()) {
        return;
    }

    if node.node_type() == NodeType::ListItem
        && list_item_own_paragraph_intersects(selection, node)
        && seen.insert(node.id())
    {
        out.push(node.id());
    }

    for child in node.child_blocks() {
        collect_list_items_for_kind_toggle_in(selection, &child, out, seen);
    }
}

fn is_unsupported_whole_container(node_type: NodeType) -> bool {
    !matches!(
        node_type,
        NodeType::Root
            | NodeType::Paragraph
            | NodeType::ListItem
            | NodeType::BulletList
            | NodeType::OrderedList
    )
}

pub(crate) fn contains_selected_plain_paragraph(
    selection: &editor_state::ResolvedSelection<'_>,
    view: &DocView,
) -> bool {
    view.root()
        .is_some_and(|root| contains_plain_paragraph_in(selection, &root))
}

fn contains_plain_paragraph_in(
    selection: &editor_state::ResolvedSelection<'_>,
    node: &NodeView<'_>,
) -> bool {
    if !selection.intersects_subtree(node) {
        return false;
    }
    if node.node_type() == NodeType::Paragraph {
        return !node
            .ancestors()
            .skip(1)
            .any(|ancestor| ancestor.node_type() == NodeType::ListItem);
    }

    node.child_blocks().any(|child| {
        if selection.contains_subtree(&child) && is_unsupported_whole_container(child.node_type()) {
            return false;
        }
        contains_plain_paragraph_in(selection, &child)
    })
}

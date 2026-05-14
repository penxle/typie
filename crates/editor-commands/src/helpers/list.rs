use std::collections::HashSet;

use editor_model::{Doc, Node, NodeId, NodeType, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill, prune};

use crate::{CommandError, CommandResult};

pub(crate) fn lift_list_item_inner(tr: &mut Transaction, list_item_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let list_item = doc
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    if !matches!(list_item.node(), Node::ListItem(_)) {
        return Ok(false);
    }

    let list = list_item
        .parent()
        .ok_or(CommandError::NoParent(list_item_id))?;
    let list_id = list.id();
    let list_type = list.as_type();
    if !matches!(list_type, NodeType::BulletList | NodeType::OrderedList) {
        return Ok(false);
    }

    let owner = list.parent().ok_or(CommandError::NoParent(list_id))?;
    let owner_id = owner.id();
    let owner_is_list_item = matches!(owner.node(), Node::ListItem(_));

    let list_item_index = list_item
        .index()
        .ok_or(CommandError::orphan_child(list_item_id, list_id))?;
    let list_index = list
        .index()
        .ok_or(CommandError::orphan_child(list_id, owner_id))?;

    let after_items: Vec<NodeId> = list
        .children()
        .skip(list_item_index + 1)
        .map(|c| c.id())
        .collect();

    let lifted_paragraph_id: Option<NodeId> = list_item.first_child().map(|c| c.id());

    // A list_item's content is `Paragraph, (BulletList|OrderedList)?` — at most one
    // trailing sublist. Locate it by node type rather than fixed index.
    let existing_sublist_id: Option<NodeId> = list_item
        .children()
        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
        .map(|c| c.id());

    tr.batch::<_, CommandError>(|tr| {
        if owner_is_list_item {
            let doc = tr.doc();
            let outer_list_item = doc
                .node(owner_id)
                .ok_or(CommandError::NodeNotFound(owner_id))?;
            let outer_list = outer_list_item
                .parent()
                .ok_or(CommandError::NoParent(owner_id))?;
            let outer_list_id = outer_list.id();
            let outer_index = outer_list_item
                .index()
                .ok_or(CommandError::orphan_child(owner_id, outer_list_id))?;
            tr.move_node(list_item_id, outer_list_id, outer_index + 1)?;

            // A list_item allows at most one trailing sublist. If the lifted item
            // already carries one, append after_items to it instead of creating a
            // second sublist.
            if !after_items.is_empty() {
                let target_sublist_id = match existing_sublist_id {
                    Some(id) => id,
                    None => {
                        let new_sublist_id = NodeId::new();
                        let new_sublist_node = list_type.into_node().to_plain();
                        let doc = tr.doc();
                        let lifted = doc
                            .node(list_item_id)
                            .ok_or(CommandError::NodeNotFound(list_item_id))?;
                        let insert_at = lifted.entry().children.len();
                        tr.insert_subtree(
                            list_item_id,
                            insert_at,
                            Subtree::leaf(new_sublist_id, new_sublist_node),
                        )?;
                        new_sublist_id
                    }
                };
                let doc = tr.doc();
                let target_sublist = doc
                    .node(target_sublist_id)
                    .ok_or(CommandError::NodeNotFound(target_sublist_id))?;
                let base = target_sublist.entry().children.len();
                for (offset, item_id) in after_items.iter().enumerate() {
                    tr.move_node(*item_id, target_sublist_id, base + offset)?;
                }
            }
        } else {
            let doc = tr.doc();
            let list_item_ref = doc
                .node(list_item_id)
                .ok_or(CommandError::NodeNotFound(list_item_id))?;
            let children: Vec<NodeId> = list_item_ref.children().map(|c| c.id()).collect();
            let child_count = children.len();
            for (i, child_id) in children.iter().enumerate() {
                tr.move_node(*child_id, owner_id, list_index + 1 + i)?;
            }
            tr.remove_subtree(list_item_id)?;

            if !after_items.is_empty() {
                let new_list_id = NodeId::new();
                let new_list_node = list_type.into_node().to_plain();
                tr.insert_subtree(
                    owner_id,
                    list_index + 1 + child_count,
                    Subtree::leaf(new_list_id, new_list_node),
                )?;
                for (i, item_id) in after_items.iter().enumerate() {
                    tr.move_node(*item_id, new_list_id, i)?;
                }
            }
        }

        let doc = tr.doc();
        if let Some(original_list) = doc.node(list_id)
            && original_list.entry().children.is_empty()
        {
            tr.apply_steps(prune(&original_list))?;
        }

        let doc = tr.doc();
        if let Some(owner) = doc.node(owner_id) {
            tr.apply_steps(fulfill(&owner))?;
        }
        Ok(())
    })?;

    let doc = tr.doc();
    if let Some(para_id) = lifted_paragraph_id
        && let Some(para) = doc.node(para_id)
    {
        let cursor_pos = match para.first_child() {
            Some(child) if matches!(child.node(), Node::Text(_)) => Position {
                node_id: child.id(),
                offset: 0,
                affinity: Affinity::Downstream,
            },
            _ => Position {
                node_id: para_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        };
        tr.set_selection(Selection::collapsed(cursor_pos))?;
    }

    Ok(true)
}

pub(crate) fn collect_top_level_list_items_in_selection(
    doc: &Doc,
    from: Position,
    to: Position,
) -> Vec<NodeId> {
    let from_list_item = find_enclosing_list_item_id(doc, from.node_id);
    let to_list_item = find_enclosing_list_item_id(doc, to.node_id);

    let (Some(from_li), Some(to_li)) = (from_list_item, to_list_item) else {
        return Vec::new();
    };

    if from_li == to_li {
        return vec![from_li];
    }

    let Some(common_list_id) = lowest_common_list_ancestor(doc, from_li, to_li) else {
        return Vec::new();
    };

    let common_list = match doc.node(common_list_id) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let children: Vec<NodeId> = common_list.children().map(|c| c.id()).collect();
    let from_idx = ancestor_index_within(doc, from_li, common_list_id);
    let to_idx = ancestor_index_within(doc, to_li, common_list_id);
    let (Some(a), Some(b)) = (from_idx, to_idx) else {
        return Vec::new();
    };
    let lo = a.min(b);
    let hi = a.max(b);
    children[lo..=hi].to_vec()
}

pub(crate) fn find_enclosing_list_item_id(doc: &Doc, node_id: NodeId) -> Option<NodeId> {
    let mut current = doc.node(node_id)?;
    loop {
        if matches!(current.node(), Node::ListItem(_)) {
            return Some(current.id());
        }
        current = current.parent()?;
    }
}

fn lowest_common_list_ancestor(doc: &Doc, a: NodeId, b: NodeId) -> Option<NodeId> {
    let ancestors_a: Vec<NodeId> = doc.node(a)?.ancestors().map(|n| n.id()).collect();
    let ancestors_b: HashSet<NodeId> = doc.node(b)?.ancestors().map(|n| n.id()).collect();

    for la in ancestors_a.iter() {
        if matches!(
            doc.node(*la).map(|n| n.as_type()),
            Some(NodeType::BulletList | NodeType::OrderedList)
        ) && ancestors_b.contains(la)
        {
            return Some(*la);
        }
    }
    None
}

fn ancestor_index_within(doc: &Doc, node_id: NodeId, ancestor_id: NodeId) -> Option<usize> {
    let mut current_id = node_id;
    loop {
        let current = doc.node(current_id)?;
        let parent = current.parent()?;
        if parent.id() == ancestor_id {
            return current.index();
        }
        current_id = parent.id();
    }
}

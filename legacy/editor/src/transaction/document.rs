use crate::model::*;
use crate::runtime::Effect;
use crate::schema::{NodeSpec, Schema};
use crate::state::position_helpers::{find_child_at_offset, leaf_block_end, leaf_block_start};
use crate::state::{Position, Selection, block_content_len};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result};
use rustc_hash::FxHashSet;

#[derive(Debug, Clone)]
pub enum InsertResult {
    None,
    Inserted {
        anchor: Position,
        head: Position,
        is_selectable: bool,
    },
}

impl InsertResult {
    pub fn inserted(&self) -> bool {
        !matches!(self, InsertResult::None)
    }

    pub fn as_selection(&self) -> Option<Selection> {
        match self {
            InsertResult::None => None,
            InsertResult::Inserted {
                anchor,
                head,
                is_selectable,
            } => {
                if *is_selectable {
                    let anchor = Position::new(anchor.node_id, anchor.offset, Affinity::Downstream);
                    let head = Position::new(head.node_id, head.offset, Affinity::Upstream);
                    Some(Selection::new(anchor, head))
                } else {
                    Some(Selection::collapsed(*head))
                }
            }
        }
    }

    pub fn as_range_selection(&self) -> Option<Selection> {
        match self {
            InsertResult::None => None,
            InsertResult::Inserted { anchor, head, .. } => {
                let anchor = Position::new(anchor.node_id, anchor.offset, Affinity::Downstream);
                let head = Position::new(head.node_id, head.offset, Affinity::Upstream);
                Some(Selection::new(anchor, head))
            }
        }
    }

    pub fn as_inline_range_selection(&self, doc: &Doc) -> Option<Selection> {
        match self {
            InsertResult::None => None,
            InsertResult::Inserted { anchor, head, .. } => {
                let anchor = doc
                    .node(anchor.node_id)
                    .and_then(|parent| {
                        parent
                            .children()
                            .nth(anchor.offset)
                            .and_then(|child| leaf_block_start(&child))
                    })
                    .unwrap_or(*anchor);

                let head = if head.offset > 0 {
                    doc.node(head.node_id)
                        .and_then(|parent| {
                            parent
                                .children()
                                .nth(head.offset - 1)
                                .and_then(|child| leaf_block_end(&child))
                        })
                        .unwrap_or(*head)
                } else {
                    *head
                };

                Some(Selection::new(anchor, head))
            }
        }
    }
}

struct FragmentMergeResult {
    merged_start_node: Option<NodeId>,
    start_merge_offset: usize,
    last_top_level: Option<NodeId>,
    open_end_survivor: Option<NodeId>,
    had_open_start: bool,
    had_open_end: bool,
}

impl Transaction {
    pub fn delete_range(&mut self, from: Position, to: Position) -> Result<bool> {
        self.replace_range(from, to, Fragment::empty())
    }

    pub fn replace_range(
        &mut self,
        from: Position,
        to: Position,
        fragment: Fragment,
    ) -> Result<bool> {
        if from == to {
            let result = self.insert_fragment(from, fragment)?;
            if let InsertResult::Inserted { head, .. } = result {
                self.set_selection_position(head);
            }
            return Ok(true);
        }

        let from_node = self
            .doc()
            .node(from.node_id)
            .context("From node not found")?;
        let to_node = self.doc().node(to.node_id).context("To node not found")?;

        if from_node.node_id() == to_node.node_id() {
            if let Some((from_child_id, from_local_offset)) =
                find_child_at_offset(&from_node, from.offset)
            {
                if let Some((to_child_id, to_local_offset)) =
                    find_child_at_offset(&to_node, to.offset)
                {
                    if from_child_id == to_child_id {
                        let child = self.doc().node(from_child_id).context("Child not found")?;
                        if matches!(child.node(), Some(Node::Text(_))) {
                            self.replace_within_same_text_node(
                                from_child_id,
                                from_local_offset,
                                to_local_offset,
                                from,
                                fragment,
                            )?;
                            return Ok(true);
                        } else {
                            self.delete_node_recursive(from_child_id)?;
                            let offset = from.offset.saturating_sub(from_local_offset);
                            let parent =
                                self.doc().node(from.node_id).context("Parent not found")?;

                            let selection_pos = if let Some((child_id, _)) =
                                find_child_at_offset(&parent, offset)
                            {
                                let child = self.doc().node(child_id).context("Child not found")?;
                                if !child.spec().map_or(true, |s| s.content.is_leaf()) {
                                    let child_len = child.children().fold(0, |acc, c| {
                                        acc + match c.node() {
                                            Some(Node::Text(t)) => t.text.char_len(),
                                            _ => 1,
                                        }
                                    });
                                    Position::new(child_id, child_len, from.affinity)
                                } else {
                                    Position::new(from.node_id, offset, from.affinity)
                                }
                            } else {
                                Position::new(from.node_id, offset, from.affinity)
                            };

                            let result = self.insert_fragment(selection_pos, fragment)?;
                            if let InsertResult::Inserted { head, .. } = result {
                                self.set_selection_position(head);
                            } else {
                                self.set_selection_position(selection_pos);
                            }
                            return Ok(true);
                        }
                    } else {
                        self.replace_within_same_block(
                            from_child_id,
                            from_local_offset,
                            to_child_id,
                            to_local_offset,
                            from,
                            fragment,
                        )?;
                        return Ok(true);
                    }
                }
            }
        }

        self.replace_across_blocks(from, to, fragment)?;
        Ok(true)
    }

    pub fn move_node_range(
        &mut self,
        start_id: NodeId,
        end_id: NodeId,
        parent_id: Option<NodeId>,
        prev_id: Option<NodeId>,
        _next_id: Option<NodeId>,
    ) -> Result<()> {
        let target_parent_id = parent_id.context("Parent ID not found for move_node_range")?;

        let mut index = if let Some(prev_id) = prev_id {
            let prev_node = self
                .doc()
                .node(prev_id)
                .context("Previous node not found")?;
            prev_node.index().context("Previous node has no index")? + 1
        } else {
            0
        };

        let mut current_id = start_id;

        let old_parent_id = self
            .doc()
            .node(start_id)
            .and_then(|n| n.parent().map(|p| p.node_id()));

        loop {
            let node = self.node_mut(current_id).context("Node not found")?;
            let is_last = current_id == end_id;
            let next_id = node.next_sibling().map(|n| n.node_id());

            node.as_mut().move_to(target_parent_id, index)?;
            index += 1;

            if is_last {
                break;
            }

            current_id = match next_id {
                Some(id) => id,
                None => break,
            };
        }

        self.mark_attr_mutation(target_parent_id);
        if let Some(pid) = old_parent_id {
            if pid != target_parent_id {
                self.mark_attr_mutation(pid);
            }
        }
        self.mark_structure_mutation(target_parent_id);

        Ok(())
    }

    pub fn delete_node_recursive(&mut self, node_id: NodeId) -> Result<()> {
        let parent_id = self
            .doc()
            .node(node_id)
            .and_then(|n| n.parent().map(|p| p.node_id()));

        let node = self.node_mut(node_id).context("Node not found")?;
        node.as_mut().delete().context("Failed to delete node")?;

        if let Some(pid) = parent_id {
            self.mark_attr_mutation(pid);
            self.mark_structure_mutation(pid);
        }

        Ok(())
    }

    pub fn delete_text_range(
        &mut self,
        node_id: NodeId,
        from_byte: Option<usize>,
        to_byte: Option<usize>,
    ) -> Result<bool> {
        let text_len = {
            let node = self.doc().node(node_id).context("Node not found")?;
            if let Some(Node::Text(text_node)) = node.node() {
                text_node.text.len()
            } else {
                anyhow::bail!("Node is not a text node");
            }
        };

        let starts_at_beginning = from_byte.map_or(true, |f| f == 0);
        let ends_at_end = to_byte.map_or(true, |t| t == text_len);
        let should_remove = starts_at_beginning && ends_at_end;

        if should_remove {
            self.delete_node_recursive(node_id)?;
            Ok(true)
        } else {
            let node = self.node_mut(node_id).context("Node not found")?;
            node.as_mut().update(|node| {
                if let Node::Text(text_node) = node {
                    match (from_byte, to_byte) {
                        (Some(from), Some(to)) => {
                            let from_char = text_node.text.byte_to_char(from);
                            let to_char = text_node.text.byte_to_char(to);
                            text_node.text.delete(from_char, to_char);
                        }
                        (Some(from), None) => {
                            let from_char = text_node.text.byte_to_char(from);
                            let len = text_node.text.char_len();
                            text_node.text.delete(from_char, len);
                        }
                        (None, Some(to)) => {
                            let to_char = text_node.text.byte_to_char(to);
                            text_node.text.delete(0, to_char);
                        }
                        _ => {}
                    }
                }
            })?;

            // Notify that the parent has changed (layout update needed)
            if let Some(parent_id) = self
                .doc()
                .node(node_id)
                .and_then(|n| n.parent().map(|p| p.node_id()))
            {
                self.mark_attr_mutation(parent_id);
            }

            Ok(false)
        }
    }

    // Text 노드면 지정 방향으로 부분 삭제, Non-text 노드면 조건부 전체 삭제
    fn delete_child_at_offset(
        &mut self,
        node_id: NodeId,
        char_offset: usize,
        direction: DeleteDirection,
    ) -> Result<()> {
        let node = self.doc().node(node_id).context("Node not found")?;

        if let Some(Node::Text(text_node)) = node.node() {
            let byte_offset = text_node.text.char_to_byte(char_offset);
            match direction {
                DeleteDirection::FromOffset => {
                    self.delete_text_range(node_id, Some(byte_offset), None)?;
                }
                DeleteDirection::UntilOffset => {
                    if byte_offset > 0 {
                        self.delete_text_range(node_id, None, Some(byte_offset))?;
                    }
                }
            }
        } else {
            let should_delete = match direction {
                DeleteDirection::FromOffset => char_offset == 0,
                DeleteDirection::UntilOffset => char_offset > 0,
            };
            if should_delete {
                self.delete_node_recursive(node_id)?;
            }
        }

        Ok(())
    }

    // 블록 내 offset 위치의 child부터 (또는 까지) siblings 삭제
    fn delete_range_at_block(
        &mut self,
        block_id: NodeId,
        offset: usize,
        target_ancestor_id: NodeId,
        direction: SiblingDirection,
    ) -> Result<()> {
        let block = self.doc().node(block_id).context("Block not found")?;

        if let Some((child_id, local_offset)) = find_child_at_offset(&block, offset) {
            let siblings_to_delete =
                self.collect_siblings_until_ancestor(child_id, target_ancestor_id, direction)?;

            let delete_dir = match direction {
                SiblingDirection::Following => DeleteDirection::FromOffset,
                SiblingDirection::Preceding => DeleteDirection::UntilOffset,
            };
            self.delete_child_at_offset(child_id, local_offset, delete_dir)?;

            for id in siblings_to_delete {
                self.delete_node_recursive(id)?;
            }
        }

        Ok(())
    }

    fn delete_flat_range(&mut self, from_id: NodeId, to_id: NodeId) -> Result<()> {
        let from_node = self.doc().node(from_id).context("From node not found")?;
        let parent = from_node.parent().context("From node has no parent")?;
        let children = self.doc().get_children_ids(parent.node_id());

        let from_index = children
            .iter()
            .position(|&id| id == from_id)
            .context("From node not found in parent children")?;
        let to_index = children
            .iter()
            .position(|&id| id == to_id)
            .context("To node not found in parent children")?;

        if from_index > to_index {
            anyhow::bail!("From index > To index");
        }

        let node_ids = children[from_index..=to_index].to_vec();

        let parent_id = parent.node_id();
        if let Some(children_list) = self.doc().get_children_list(parent_id) {
            children_list.delete(from_index, node_ids.len())?;
        }
        self.doc().invalidate_children_cache_for(parent_id);
        for &node_id in &node_ids {
            self.doc().mark_unreachable_subtree(node_id);
        }
        self.mark_structure_mutation(parent_id);

        Ok(())
    }

    fn collect_sibling_range(&self, from_id: NodeId, to_id: NodeId) -> Result<Vec<NodeId>> {
        let mut node_ids = Vec::new();
        let from_node = self.doc().node(from_id).context("From node not found")?;
        let mut current_id = from_node.next_sibling().map(|n| n.node_id());

        while let Some(node_id) = current_id {
            if node_id == to_id {
                break;
            }
            node_ids.push(node_id);
            current_id = self
                .doc()
                .node(node_id)
                .and_then(|n| n.next_sibling().map(|s| s.node_id()));
        }

        Ok(node_ids)
    }

    pub fn merge_blocks_content(
        &mut self,
        from_block_id: NodeId,
        to_block_id: NodeId,
    ) -> Result<()> {
        let to_block = self.doc().node(to_block_id).context("To block not found")?;
        let to_block_parent = to_block.parent().map(|n| n.node_id());

        if let (Some(first_child), Some(last_child)) =
            (to_block.first_child(), to_block.last_child())
        {
            let from_block = self
                .doc()
                .node(from_block_id)
                .context("From block not found")?;
            let from_last_child = from_block.last_child().map(|n| n.node_id());

            self.move_node_range(
                first_child.node_id(),
                last_child.node_id(),
                Some(from_block_id),
                from_last_child,
                None,
            )?;
        }

        self.delete_node_recursive(to_block_id)?;

        if let Some(parent_id) = to_block_parent {
            self.clean_up_empty_ancestors(parent_id)?;
        }

        Ok(())
    }

    fn try_merge_subtree_roots(
        &mut self,
        from_subtree_root_id: NodeId,
        to_subtree_root_id: NodeId,
    ) -> Result<()> {
        if from_subtree_root_id != to_subtree_root_id {
            let from_root_exists = self.doc().node(from_subtree_root_id).is_some();
            let to_root_exists = self.doc().node(to_subtree_root_id).is_some();

            if from_root_exists
                && to_root_exists
                && self
                    .can_join_nodes(from_subtree_root_id, to_subtree_root_id)
                    .unwrap_or(false)
            {
                self.merge_nodes(from_subtree_root_id, to_subtree_root_id)?;
            }
        }
        Ok(())
    }

    pub fn clean_up_empty_ancestors(&mut self, mut node_id: NodeId) -> Result<()> {
        loop {
            let Some(node) = self.doc().node(node_id) else {
                break;
            };

            let parent_id = node.parent().map(|n| n.node_id());
            let has_children = node.first_child().is_some();

            if has_children || parent_id.is_none() {
                break;
            }

            let spec = node.spec().context("Node spec not found")?;
            if spec.content.allows_empty() {
                break;
            }

            self.delete_node_recursive(node_id)?;

            if let Some(pid) = parent_id {
                node_id = pid;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn replace_within_same_text_node(
        &mut self,
        text_node_id: NodeId,
        from_char_offset: usize,
        to_char_offset: usize,
        position: Position,
        fragment: Fragment,
    ) -> Result<()> {
        let text_node = self
            .doc()
            .node(text_node_id)
            .context("Text node not found")?;

        let Some(Node::Text(text)) = text_node.node() else {
            anyhow::bail!("Expected text node but got different node type");
        };

        let from_byte_offset = text.text.char_to_byte(from_char_offset);
        let to_byte_offset = text.text.char_to_byte(to_char_offset);

        self.delete_text_range(text_node_id, Some(from_byte_offset), Some(to_byte_offset))?;
        let result = self.insert_fragment(position, fragment)?;
        if let InsertResult::Inserted { head, .. } = result {
            self.set_selection_position(head);
        } else {
            self.set_selection_position(position);
        }
        Ok(())
    }

    fn replace_within_same_block(
        &mut self,
        from_child_id: NodeId,
        from_local_offset: usize,
        to_child_id: NodeId,
        to_local_offset: usize,
        from: Position,
        fragment: Fragment,
    ) -> Result<()> {
        let (from_is_text, to_is_text) = {
            let from_child = self
                .doc()
                .node(from_child_id)
                .context("replace_within_same_block: From child not found")?;
            let to_child = self.doc().node(to_child_id).context("To child not found")?;

            (
                matches!(from_child.node(), Some(Node::Text(_))),
                matches!(to_child.node(), Some(Node::Text(_))),
            )
        };

        let nodes_between = self.collect_sibling_range(from_child_id, to_child_id)?;

        let from_node_was_deleted = if from_is_text {
            let from_byte_offset = {
                let from_child = self
                    .doc()
                    .node(from_child_id)
                    .context("replace_within_same_block: From child not found")?;
                if let Some(Node::Text(text)) = from_child.node() {
                    text.text.char_to_byte(from_local_offset)
                } else {
                    0
                }
            };
            self.delete_text_range(from_child_id, Some(from_byte_offset), None)?
        } else {
            if from_local_offset == 0 {
                self.delete_node_recursive(from_child_id)?;
                true
            } else {
                false
            }
        };

        for node_id in nodes_between {
            self.delete_node_recursive(node_id)?;
        }

        if to_is_text {
            let to_byte_offset = {
                let to_child = self.doc().node(to_child_id).context("To child not found")?;
                if let Some(Node::Text(text)) = to_child.node() {
                    text.text.char_to_byte(to_local_offset)
                } else {
                    0
                }
            };
            if to_byte_offset > 0 {
                self.delete_text_range(to_child_id, None, Some(to_byte_offset))?;
            }
        } else {
            if to_local_offset > 0 {
                self.delete_node_recursive(to_child_id)?;
            }
        }

        let (selection_pos, node_selection) = if from_node_was_deleted {
            let mut offset = from.offset.saturating_sub(from_local_offset);

            if !from_is_text && from.node_id == NodeId::ROOT && offset > 0 {
                offset -= 1;
            }

            let parent_id = from.node_id;

            // Scope the borrow of parent
            let (child_id, child_is_leaf, child_is_selectable, child_len) = {
                let parent = self.doc().node(parent_id).context("Parent not found 2")?;
                if let Some((child_id, _)) = find_child_at_offset(&parent, offset) {
                    let child = self.doc().node(child_id).context("Child not found")?;
                    let is_leaf = child.spec().map_or(true, |s| s.content.is_leaf());
                    let is_selectable = child.spec().map_or(false, |s| s.selectable);
                    let len = if !is_leaf {
                        child.children().fold(0, |acc, c| {
                            acc + match c.node() {
                                Some(Node::Text(t)) => t.text.char_len(),
                                _ => 1,
                            }
                        })
                    } else {
                        0
                    };
                    (Some(child_id), is_leaf, is_selectable, len)
                } else {
                    (None, true, false, 0)
                }
            };

            if let Some(child_id) = child_id {
                if !child_is_leaf {
                    let pos = if from.offset == 0 {
                        Position::new(child_id, 0, from.affinity)
                    } else {
                        Position::new(child_id, child_len, from.affinity)
                    };
                    (pos, None)
                } else if child_is_selectable {
                    let anchor = Position::new(from.node_id, offset, Affinity::Downstream);
                    let head = Position::new(from.node_id, offset + 1, Affinity::Upstream);
                    (anchor, Some(Selection::new(anchor, head)))
                } else {
                    (Position::new(from.node_id, offset, from.affinity), None)
                }
            } else {
                (Position::new(from.node_id, offset, from.affinity), None)
            }
        } else {
            (from, None)
        };

        let result = self.insert_fragment(selection_pos, fragment)?;
        if let Some(sel) = node_selection {
            self.set_selection(sel);
        } else if let InsertResult::Inserted { head, .. } = result {
            self.set_selection_position(head);
        } else {
            self.set_selection_position(selection_pos);
        }
        Ok(())
    }

    fn replace_across_blocks(
        &mut self,
        from: Position,
        to: Position,
        fragment: Fragment,
    ) -> Result<()> {
        let is_delete = fragment.is_empty();
        let mut fragment = Some(fragment);

        let from_node = self
            .doc()
            .node(from.node_id)
            .context("From node not found")?;
        let to_node = self.doc().node(to.node_id).context("To node not found")?;
        let flatten_seed_ids = if is_delete {
            let mut seen = FxHashSet::default();
            let mut seed_ids = Vec::new();
            for node in from_node.ancestors().chain(to_node.ancestors()) {
                let node_id = node.node_id();
                if seen.insert(node_id) {
                    seed_ids.push(node_id);
                }
            }
            seed_ids
        } else {
            Vec::new()
        };

        let from_ancestors: Vec<NodeId> = from_node.ancestors().map(|n| n.node_id()).collect();
        let to_ancestors: Vec<NodeId> = to_node.ancestors().map(|n| n.node_id()).collect();

        let is_from_ancestor_of_to = to_ancestors.contains(&from.node_id);

        if is_from_ancestor_of_to {
            if let Some((child_id, child_local_offset)) =
                find_child_at_offset(&from_node, from.offset)
            {
                if child_id == to.node_id || to_ancestors.contains(&child_id) {
                    let fragment_to_use = fragment.take().unwrap();
                    self.replace_range(
                        Position::new(child_id, child_local_offset, from.affinity),
                        to,
                        fragment_to_use,
                    )?;

                    // Preserve the selection set by the recursive replace_range before any
                    // structural cleanup that could delete the node the selection points to.
                    let preserved_selection = *self.selection();

                    if let Some(child_node) = self.doc().node(child_id) {
                        if !child_node.spec().map_or(true, |s| s.content.allows_empty())
                            && child_node.first_child().is_none()
                        {
                            self.delete_node_recursive(child_id)?;
                            self.clean_up_empty_ancestors(from.node_id)?;
                        }
                    }

                    self.set_selection(preserved_selection);
                    if is_delete {
                        self.promote_container_items(&flatten_seed_ids)?;
                    }
                    return Ok(());
                }
            }
        }

        let is_to_ancestor_of_from = from_ancestors.contains(&to.node_id);
        let fragment = fragment.unwrap();

        let subtree_info = self.find_common_ancestor_and_subtrees(from, to)?;
        let from_subtree_root_id = subtree_info.from_subtree_root_id;
        let to_subtree_root_id = subtree_info.to_subtree_root_id;
        let siblings_between_roots = subtree_info.siblings_between_roots;
        let from_ancestor_siblings = self.collect_ancestor_siblings(
            from.node_id,
            from_subtree_root_id,
            SiblingDirection::Following,
        )?;
        let to_ancestor_siblings = self.collect_ancestor_siblings(
            to.node_id,
            to_subtree_root_id,
            SiblingDirection::Preceding,
        )?;

        if let Some((start, end)) = siblings_between_roots {
            self.delete_flat_range(start, end)?;
        }

        self.delete_range_at_block(
            from.node_id,
            from.offset,
            to.node_id,
            SiblingDirection::Following,
        )?;

        if subtree_info.to_at_ancestor_level {
            self.delete_child_at_offset(
                to_subtree_root_id,
                subtree_info.to_local_offset,
                DeleteDirection::UntilOffset,
            )?;
        } else {
            self.delete_range_at_block(
                to.node_id,
                to.offset,
                from.node_id,
                SiblingDirection::Preceding,
            )?;
        }

        for id in from_ancestor_siblings {
            self.delete_node_recursive(id)?;
        }

        for id in to_ancestor_siblings {
            self.delete_node_recursive(id)?;
        }

        if let Some(node) = self.doc().node(from.node_id) {
            self.clean_up_empty_ancestors(node.node_id())?;
        }

        if let Some(node) = self.doc().node(to.node_id) {
            self.clean_up_empty_ancestors(node.node_id())?;
        }

        self.try_lift_block(from.node_id, from_subtree_root_id, from.offset)?;

        let final_selection_pos = if from.node_id != to.node_id {
            if !is_from_ancestor_of_to && !is_to_ancestor_of_from {
                self.merge_blocks_content(from.node_id, to.node_id)?;
                self.merge_adjacent_text_nodes(from)?;
                self.try_merge_subtree_roots(from_subtree_root_id, to_subtree_root_id)?;

                from
            } else if is_from_ancestor_of_to {
                Position::new(to.node_id, 0, Affinity::Downstream)
            } else {
                from
            }
        } else {
            from
        };

        let result = self.insert_fragment(final_selection_pos, fragment)?;
        if let InsertResult::Inserted { head, .. } = result {
            self.set_selection_position(head);
        } else {
            self.set_selection_position(final_selection_pos);
        }

        if is_delete {
            self.promote_container_items(&flatten_seed_ids)?;
        }

        Ok(())
    }

    fn promote_item_type_on_delete(&self, node: &NodeRef<'_>) -> Option<NodeType> {
        node.spec()
            .and_then(|spec| spec.promote_item_type_on_delete)
    }

    fn collect_promotion_candidates(&self, node_ids: &[NodeId]) -> Vec<NodeId> {
        let mut seen = FxHashSet::default();
        let mut result = Vec::new();

        for &start_id in node_ids {
            let Some(start) = self.doc().node(start_id) else {
                continue;
            };

            for node in start.ancestors() {
                let Some(parent) = node.parent() else {
                    continue;
                };

                if node.node_type() == self.promote_item_type_on_delete(&parent) {
                    let node_id = node.node_id();
                    if seen.insert(node_id) {
                        result.push(node_id);
                    }
                }
            }
        }

        result
    }

    fn promote_container_item(&mut self, item_id: NodeId) -> Result<bool> {
        let (container_id, nested_container_ids, mut prev_id) = {
            let Some(item) = self.doc().node(item_id) else {
                return Ok(false);
            };
            let prev_id = item.prev_sibling().map(|n| n.node_id());

            let Some(container) = item.parent() else {
                return Ok(false);
            };
            let container_id = container.node_id();

            let Some(item_type) = self.promote_item_type_on_delete(&container) else {
                return Ok(false);
            };
            if item.node_type() != Some(item_type) {
                return Ok(false);
            };

            let Some(first_child) = item.first_child() else {
                return Ok(false);
            };

            let container_type = container.node_type();
            let mut nested_container_ids = Vec::new();

            match first_child.spec() {
                Some(spec) if spec.is_textblock() => {
                    if block_content_len(&first_child) != 0 {
                        return Ok(false);
                    }

                    for child in item.children().skip(1) {
                        let child_item_type = self.promote_item_type_on_delete(&child);
                        if child_item_type != Some(item_type) || child.node_type() != container_type
                        {
                            return Ok(false);
                        }
                        nested_container_ids.push(child.node_id());
                    }
                }
                _ if self.promote_item_type_on_delete(&first_child) == Some(item_type)
                    && first_child.node_type() == container_type =>
                {
                    for child in item.children() {
                        let child_item_type = self.promote_item_type_on_delete(&child);
                        if child_item_type != Some(item_type) || child.node_type() != container_type
                        {
                            return Ok(false);
                        }
                        nested_container_ids.push(child.node_id());
                    }
                }
                _ => {
                    return Ok(false);
                }
            }

            if nested_container_ids.is_empty() {
                return Ok(false);
            }

            (container_id, nested_container_ids, prev_id)
        };

        for nested_container_id in nested_container_ids {
            let nested_item_ids: Vec<NodeId> = self
                .doc()
                .node(nested_container_id)
                .map(|node| node.children().map(|c| c.node_id()).collect())
                .unwrap_or_default();

            for nested_item_id in nested_item_ids {
                self.move_node_range(
                    nested_item_id,
                    nested_item_id,
                    Some(container_id),
                    prev_id,
                    None,
                )?;
                prev_id = Some(nested_item_id);
            }
        }

        self.delete_node_with_selection_adjustment(item_id)?;
        self.clean_up_empty_ancestors(container_id)?;

        Ok(true)
    }

    fn promote_container_items(&mut self, seed_node_ids: &[NodeId]) -> Result<()> {
        let mut seed_node_ids = seed_node_ids.to_vec();
        seed_node_ids.push(self.selection().head.node_id);

        let mut pass = 0;
        loop {
            pass += 1;
            if pass > 32 {
                break;
            }

            let candidates = self.collect_promotion_candidates(&seed_node_ids);
            if candidates.is_empty() {
                break;
            }

            let mut changed = false;
            for candidate in candidates {
                if self.promote_container_item(candidate)? {
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            if let Some(last) = seed_node_ids.last_mut() {
                *last = self.selection().head.node_id;
            }
        }

        Ok(())
    }

    pub fn insert_fragment(
        &mut self,
        position: Position,
        fragment: Fragment,
    ) -> Result<InsertResult> {
        if fragment.is_empty() {
            return Ok(InsertResult::None);
        }

        self.detect_fragment_fonts(&fragment, position);

        let target_is_paragraph = self
            .doc()
            .node(position.node_id)
            .map_or(false, |n| matches!(n.node(), Some(Node::Paragraph(_))));

        let can_use_open_insert = fragment.is_open()
            && target_is_paragraph
            && self.first_top_level_valid_at_parent(&fragment, position.node_id);
        if can_use_open_insert {
            let head = self.insert_open_fragment(position, fragment.clone())?;

            let top_level_ids = fragment.top_level_node_ids();
            let anchor = top_level_ids
                .first()
                .and_then(|&first_id| self.doc().node(first_id))
                .and_then(|node| {
                    node.parent().map(|parent| {
                        let index = node.index().unwrap_or(0);
                        let offset = if matches!(parent.node(), Some(Node::Paragraph(_))) {
                            parent
                                .children()
                                .take(index)
                                .map(|c| match c.node() {
                                    Some(Node::Text(t)) => t.text.char_len(),
                                    _ => 1,
                                })
                                .sum()
                        } else {
                            index
                        };

                        Position::new(parent.node_id(), offset, Affinity::Downstream)
                    })
                })
                .unwrap_or(position);

            return Ok(InsertResult::Inserted {
                anchor,
                head,
                is_selectable: false,
            });
        }

        let position = self.prepare_insertion_position(position, &fragment)?;
        let anchor = position;

        let top_level_ids = fragment.top_level_node_ids();
        let is_selectable = top_level_ids.len() == 1
            && fragment.node(top_level_ids[0]).map_or(false, |n| {
                let spec = Schema::node_spec(n.data().as_type());
                spec.selectable
            });

        self.insert_fragment_nodes(position, &fragment)?;

        let top_level_nodes = fragment.top_level_node_ids();
        let merge_result = self.handle_fragment_merges(
            position,
            &top_level_nodes,
            fragment.open_start(),
            fragment.open_end(),
        )?;

        let anchor = top_level_nodes
            .first()
            .and_then(|&first_id| {
                self.doc().node(first_id).or_else(|| {
                    merge_result
                        .merged_start_node
                        .and_then(|id| self.doc().node(id))
                })
            })
            .and_then(|node| {
                node.parent().map(|parent| {
                    let index = node.index().unwrap_or(0);
                    let offset = if parent.spec().map_or(false, |s| s.is_textblock()) {
                        parent
                            .children()
                            .take(index)
                            .map(|c| c.node().map_or(1, |n| n.len()))
                            .sum()
                    } else {
                        index
                    };

                    Position::new(parent.node_id(), offset, Affinity::Downstream)
                })
            })
            .unwrap_or(anchor);

        let head =
            self.compute_head_position(position, &fragment, &top_level_nodes, &merge_result)?;

        Ok(InsertResult::Inserted {
            anchor,
            head,
            is_selectable,
        })
    }

    fn detect_fragment_fonts(&mut self, fragment: &Fragment, position: Position) {
        use crate::utils::collect_codepoints;
        use rustc_hash::FxHashMap;

        let cascade = self.resolve_style_cascade(position.node_id);
        let default_family = cascade
            .iter()
            .find_map(|s| match s {
                Style::FontFamily(f) => Some(f.family.clone()),
                _ => None,
            })
            .unwrap_or_else(|| DefaultAttrs::default().font_family().to_string());
        let default_weight = cascade
            .iter()
            .find_map(|s| match s {
                Style::FontWeight(w) => Some(w.weight),
                _ => None,
            })
            .unwrap_or_else(|| DefaultAttrs::default().font_weight());

        let dest_overrides = self
            .doc()
            .node(position.node_id)
            .and_then(|n| n.node().map(|node| node.style_overrides()))
            .unwrap_or_default();

        let dest_override_family = dest_overrides.iter().find_map(|s| match s {
            Style::FontFamily(f) => Some(f.family.clone()),
            _ => None,
        });
        let dest_override_weight = dest_overrides.iter().find_map(|s| match s {
            Style::FontWeight(w) => Some(w.weight),
            _ => None,
        });

        let mut font_codepoints: FxHashMap<(String, u16), Vec<u32>> = FxHashMap::default();

        for (_, node) in fragment.iter() {
            if let Node::Text(text_node) = node.data() {
                for segment in text_node.text.get_segments() {
                    let codepoints = collect_codepoints(&segment.text);
                    if codepoints.is_empty() {
                        continue;
                    }

                    let family = dest_override_family
                        .clone()
                        .or_else(|| {
                            segment.styles.iter().find_map(|s| match s {
                                Style::FontFamily(f) => Some(f.family.clone()),
                                _ => None,
                            })
                        })
                        .unwrap_or_else(|| default_family.clone());
                    let weight = dest_override_weight
                        .or_else(|| {
                            segment.styles.iter().find_map(|s| match s {
                                Style::FontWeight(w) => Some(w.weight),
                                _ => None,
                            })
                        })
                        .unwrap_or(default_weight);

                    font_codepoints
                        .entry((family, weight))
                        .or_default()
                        .extend_from_slice(&codepoints);
                }
            }
        }

        for ((family, weight), codepoints) in font_codepoints {
            self.push_effect(Effect::FontDetected {
                family,
                weight,
                codepoints,
            });
        }
    }

    fn handle_fragment_merges(
        &mut self,
        position: Position,
        top_level_nodes: &[NodeId],
        open_start: usize,
        open_end: usize,
    ) -> Result<FragmentMergeResult> {
        let mut result = FragmentMergeResult {
            merged_start_node: None,
            start_merge_offset: 0,
            last_top_level: top_level_nodes.last().copied(),
            open_end_survivor: None,
            had_open_start: open_start > 0,
            had_open_end: open_end > 0,
        };

        if open_start > 0 {
            if let Some(first_id) = top_level_nodes.first() {
                self.merge_fragment_start(*first_id, &mut result)?;

                if top_level_nodes.len() == 1 {
                    result.last_top_level = result.merged_start_node;
                }
            }
        }

        if open_end > 0 {
            result.open_end_survivor =
                self.handle_open_end_merge(position, open_end, result.last_top_level)?;
        } else {
            self.merge_adjacent_text_nodes(position)?;
        }

        Ok(result)
    }

    fn merge_fragment_start(
        &mut self,
        first_id: NodeId,
        result: &mut FragmentMergeResult,
    ) -> Result<()> {
        let Some(first_node) = self.doc().node(first_id) else {
            return Ok(());
        };

        let Some(prev) = first_node.prev_sibling() else {
            return Ok(());
        };

        let prev_id = prev.node_id();
        let prev_len = block_content_len(&prev);

        self.merge_blocks_content(prev_id, first_id)?;
        self.merge_adjacent_text_nodes(Position::new(prev_id, prev_len, Affinity::Downstream))?;

        result.start_merge_offset = prev_len;
        result.merged_start_node = Some(prev_id);

        Ok(())
    }

    fn compute_head_position(
        &self,
        position: Position,
        fragment: &Fragment,
        top_level_nodes: &[NodeId],
        merge_result: &FragmentMergeResult,
    ) -> Result<Position> {
        let FragmentMergeResult {
            merged_start_node,
            start_merge_offset,
            last_top_level,
            open_end_survivor,
            had_open_start,
            had_open_end,
        } = merge_result;

        match (*had_open_start, *had_open_end) {
            (_, true) => {
                let target = open_end_survivor.or(*last_top_level);
                if *had_open_start {
                    if let Some(target_id) = target {
                        let offset = fragment.last_top_level_inline_len();
                        return Ok(Position::new(target_id, offset, Affinity::Downstream));
                    }
                    Ok(position)
                } else {
                    self.compute_head_after_complex_insert(
                        position,
                        target,
                        top_level_nodes,
                        fragment,
                    )
                }
            }
            (true, false) => self.compute_head_after_open_start_insert(
                position,
                fragment,
                top_level_nodes,
                *merged_start_node,
                *start_merge_offset,
                *last_top_level,
            ),
            (false, false) => self.compute_head_for_simple_insert(position, fragment),
        }
    }

    fn compute_head_after_open_start_insert(
        &self,
        position: Position,
        fragment: &Fragment,
        top_level_nodes: &[NodeId],
        merged_start_node: Option<NodeId>,
        start_merge_offset: usize,
        last_top_level: Option<NodeId>,
    ) -> Result<Position> {
        if top_level_nodes.len() > 1 {
            self.compute_head_after_multi_block_insert(
                position,
                fragment,
                top_level_nodes,
                last_top_level,
            )
        } else {
            let target_id = merged_start_node.unwrap_or(position.node_id);
            let base_offset = merged_start_node.map_or(position.offset, |_| start_merge_offset);
            let end_offset = base_offset + fragment.inline_len();
            Ok(Position::new(target_id, end_offset, Affinity::Downstream))
        }
    }

    fn compute_head_after_multi_block_insert(
        &self,
        position: Position,
        fragment: &Fragment,
        top_level_nodes: &[NodeId],
        last_top_level: Option<NodeId>,
    ) -> Result<Position> {
        if let Some(last_id) = last_top_level {
            if let Some(last_node) = self.doc().node(last_id) {
                if let Some(next) = last_node.next_sibling() {
                    let content_len = block_content_len(&next);
                    return Ok(Position::new(
                        next.node_id(),
                        content_len,
                        Affinity::Downstream,
                    ));
                }
            }
        }
        self.compute_head_after_complex_insert(position, last_top_level, top_level_nodes, fragment)
    }

    fn prepare_insertion_position(
        &mut self,
        position: Position,
        fragment: &Fragment,
    ) -> Result<Position> {
        let has_open_start = fragment.has_open_start();

        let fragment_top_types: Vec<_> = if has_open_start {
            fragment
                .content_node_ids()
                .into_iter()
                .filter_map(|id| fragment.node(id))
                .map(|n| n.data().as_type())
                .collect()
        } else {
            fragment
                .top_level_node_ids()
                .into_iter()
                .filter_map(|id| fragment.node(id))
                .map(|n| n.data().as_type())
                .collect()
        };

        let has_block_nodes = fragment_top_types
            .iter()
            .any(|t| !Schema::node_spec(*t).inline);

        if !has_block_nodes {
            return Ok(position);
        }

        let target = self
            .doc()
            .node(position.node_id)
            .context("Node not found")?;
        let fragment_is_open = fragment.open_start() > 0 || fragment.open_end() > 0;

        match target.node().context("Node decode failed")? {
            Node::Text(_) => {
                let parent_id = target.parent().context("Text has no parent")?.node_id();
                let index = target.index().context("Text has no index")?;
                self.split_paragraph()?;
                let pos = Position::new(parent_id, index + 1, Affinity::Downstream);
                self.ascend_to_compatible_parent(pos, &fragment_top_types, has_open_start)
            }
            Node::Paragraph(_) => {
                let parent_id = target
                    .parent()
                    .context("Paragraph has no parent")?
                    .node_id();
                let index = target.index().context("Paragraph has no index")?;
                let is_empty = target.first_child().is_none();

                if position.offset == 0 && (fragment_is_open || is_empty) {
                    let pos = Position::new(parent_id, index, Affinity::Downstream);
                    return self.ascend_to_compatible_parent_no_split(pos, &fragment_top_types);
                }

                self.split_paragraph()?;
                let pos = Position::new(parent_id, index + 1, Affinity::Downstream);
                self.ascend_to_compatible_parent(pos, &fragment_top_types, has_open_start)
            }
            _ => Ok(position),
        }
    }

    fn ascend_to_compatible_parent(
        &mut self,
        mut pos: Position,
        fragment_types: &[NodeType],
        has_open_start: bool,
    ) -> Result<Position> {
        if has_open_start {
            return Ok(pos);
        }

        loop {
            let parent = self.node(pos.node_id).context("Parent not found")?;
            let spec = parent.spec().context("Parent spec not found")?;
            let content = &spec.content;

            if fragment_types.iter().all(|t| content.matches(*t)) {
                break;
            }

            if parent.parent().is_none() {
                break;
            }

            self.split_node_at_index(pos.node_id, pos.offset)?;
            let parent = self.node(pos.node_id).context("Parent not found")?;
            let grandparent = parent.parent().context("Grandparent not found")?;
            let idx = parent.index().context("No index")?;
            pos = Position::new(grandparent.node_id(), idx + 1, Affinity::Downstream);
        }
        Ok(pos)
    }

    fn ascend_to_compatible_parent_no_split(
        &self,
        mut pos: Position,
        fragment_types: &[NodeType],
    ) -> Result<Position> {
        loop {
            let parent = self.node(pos.node_id).context("Parent not found")?;
            let spec = parent.spec().context("Parent spec not found")?;
            let content = &spec.content;

            if fragment_types.iter().all(|t| content.matches(*t)) {
                break;
            }

            if parent.parent().is_none() {
                break;
            }

            let grandparent = parent.parent().context("Grandparent not found")?;
            let idx = parent.index().context("No index")?;
            pos = Position::new(grandparent.node_id(), idx, Affinity::Downstream);
        }
        Ok(pos)
    }

    pub fn delete_node_with_selection_adjustment(&mut self, node_id: NodeId) -> Result<()> {
        let selection_before = *self.selection();

        let (parent_id, node_index, anchor_in_deleted_subtree, head_in_deleted_subtree) =
            if let Some(node) = self.doc().node(node_id) {
                let anchor_in_deleted_subtree = selection_before.anchor.node_id == node_id
                    || self
                        .doc()
                        .is_ancestor(node_id, selection_before.anchor.node_id);
                let head_in_deleted_subtree = selection_before.head.node_id == node_id
                    || self
                        .doc()
                        .is_ancestor(node_id, selection_before.head.node_id);

                (
                    node.parent_id(),
                    node.path().last().copied(),
                    anchor_in_deleted_subtree,
                    head_in_deleted_subtree,
                )
            } else {
                (None, None, false, false)
            };

        self.delete_node_recursive(node_id)?;

        if let (Some(parent_id), Some(node_index)) = (parent_id, node_index) {
            let delete_start = Position::new(parent_id, node_index, Affinity::Downstream);
            let mut selection = selection_before;
            let mut changed = false;

            if anchor_in_deleted_subtree {
                selection.anchor = delete_start;
                changed = true;
            } else if selection.anchor.node_id == parent_id && selection.anchor.offset > node_index
            {
                selection.anchor.offset = selection.anchor.offset.saturating_sub(1);
                changed = true;
            }

            if head_in_deleted_subtree {
                selection.head = delete_start;
                changed = true;
            } else if selection.head.node_id == parent_id && selection.head.offset > node_index {
                selection.head.offset = selection.head.offset.saturating_sub(1);
                changed = true;
            }

            if changed {
                self.set_selection(selection);
            }
        }

        Ok(())
    }

    fn split_node_at_index(&mut self, node_id: NodeId, split_index: usize) -> Result<NodeId> {
        let node = self.node(node_id).context("Node not found")?;
        let parent = node.parent().context("Parent not found")?;
        let parent_id = parent.node_id();
        let node_index = node.index().context("Node has no index")?;

        let new_node_data = node.node().context("Node decode failed")?.clone();

        let new_node_id = self
            .node_mut(parent_id)
            .context("Parent not found")?
            .as_mut()
            .insert_child(node_index + 1, new_node_data)?;

        let node = self.node(node_id).context("Node not found")?;
        let children_count = node.children().count();
        if split_index < children_count {
            let first_child_to_move = node.children().nth(split_index).map(|n| n.node_id());
            let last_child = node.last_child().map(|n| n.node_id());

            if let (Some(first), Some(last)) = (first_child_to_move, last_child) {
                self.move_node_range(first, last, Some(new_node_id), None, None)?;
            }
        }

        Ok(new_node_id)
    }

    fn compute_head_for_simple_insert(
        &self,
        position: Position,
        remapped: &Fragment,
    ) -> Result<Position> {
        let target = self
            .doc()
            .node(position.node_id)
            .context("Target node not found")?;

        let top_level_ids = remapped.top_level_node_ids();

        // For selectable nodes inserted into non-textblock, return block position after
        if !target.spec().map_or(false, |s| s.is_textblock()) && top_level_ids.len() == 1 {
            if let Some(frag_node) = remapped.node(top_level_ids[0]) {
                let node_type = frag_node.data().as_type();
                let spec = Schema::node_spec(node_type);
                if spec.selectable {
                    return Ok(Position::new(
                        position.node_id,
                        position.offset + 1,
                        Affinity::Upstream,
                    ));
                }
            }
        }

        let inserted_count = if target.spec().map_or(false, |s| s.is_textblock()) {
            let top_level_set: std::collections::HashSet<_> = top_level_ids.iter().collect();
            remapped
                .iter()
                .filter(|(id, _)| top_level_set.contains(*id))
                .map(|(_, n)| n.data().len())
                .sum()
        } else {
            top_level_ids.len()
        };

        Ok(Position::new(
            position.node_id,
            position.offset + inserted_count,
            Affinity::Downstream,
        ))
    }

    fn handle_open_end_merge(
        &mut self,
        position: Position,
        open_end: usize,
        last_top_level: Option<NodeId>,
    ) -> Result<Option<NodeId>> {
        let Some(node_to_merge) = last_top_level else {
            return Ok(None);
        };

        let node = self
            .doc()
            .node(node_to_merge)
            .context("Last top level node not found")?;
        let node_index = node.index().context("Node has no index")?;

        let target_index = Some(node_index + 1);

        let mut last_survivor = None;

        for depth in (0..open_end).rev() {
            if let Some(survivor) =
                self.perform_merge(position, node_to_merge, depth, false, target_index)?
            {
                last_survivor = Some(survivor);
            }
        }

        if let Some(survivor) = last_survivor {
            let pos = Position::new(survivor, 0, Affinity::Downstream);
            if let Some(node) = self.doc().node(survivor) {
                let mut textblock_ids = Vec::new();
                let mut stack: Vec<NodeId> = node.children().map(|c| c.node_id()).collect();
                while let Some(id) = stack.pop() {
                    if let Some(child) = self.doc().node(id) {
                        if child.spec().map_or(false, |s| s.is_textblock()) {
                            textblock_ids.push(id);
                        }
                        for g in child.children() {
                            stack.push(g.node_id());
                        }
                    }
                }

                for textblock_id in textblock_ids {
                    let _ = self.merge_adjacent_text_nodes(Position::new(
                        textblock_id,
                        0,
                        Affinity::Downstream,
                    ));
                }
            }
            self.merge_adjacent_text_nodes(pos)?;

            return Ok(Some(survivor));
        }

        Ok(None)
    }

    fn compute_head_after_complex_insert(
        &self,
        position: Position,
        last_top_level: Option<NodeId>,
        top_level_nodes: &[NodeId],
        remapped: &Fragment,
    ) -> Result<Position> {
        if let Some(last_id) = last_top_level {
            let target_block_id = remapped.find_last_leaf_block(last_id).unwrap_or(last_id);
            let content_len: usize = remapped
                .iter()
                .filter(|(_, n)| n.parent() == Some(target_block_id))
                .map(|(_, n)| n.data().len())
                .sum();

            let open_start = remapped.open_start();
            let is_merged_at_start = open_start > 0 && top_level_nodes.len() == 1;
            let base_offset = if is_merged_at_start {
                position.offset
            } else {
                0
            };

            return Ok(Position::new(
                target_block_id,
                base_offset + content_len,
                Affinity::Downstream,
            ));
        }

        Ok(position)
    }

    pub fn merge_adjacent_text_nodes(&mut self, position: Position) -> Result<()> {
        let block = self
            .doc()
            .node(position.node_id)
            .context("Block not found")?;
        let children_ids: Vec<NodeId> = block.children().map(|n| n.node_id()).collect();
        let doc = self.doc();
        let children_refs: Vec<_> = children_ids.iter().filter_map(|id| doc.node(*id)).collect();
        let children = children_refs
            .iter()
            .filter_map(|n| n.node().map(|node| (n.node_id(), node)));
        let plans = Node::plan_consecutive_text_merges(children);

        for (keep_id, remove_ids, segments) in plans {
            let merged_text = Text::from_segments(&segments);

            let node = self.node_mut(keep_id).context("Node not found")?;
            node.as_mut().update(move |node| {
                if let Node::Text(text_node) = node {
                    text_node.text = merged_text;
                }
            })?;

            for node_id in remove_ids {
                self.delete_node_recursive(node_id)?;
            }

            self.mark_attr_mutation(keep_id);
        }

        Ok(())
    }

    fn can_join_nodes(&self, node1_id: NodeId, node2_id: NodeId) -> Result<bool> {
        let node1 = self.doc().node(node1_id).context("Node1 not found")?;
        let node2 = self.doc().node(node2_id).context("Node2 not found")?;

        let n1 = node1.node().context("Node1 decode failed")?;
        let n2 = node2.node().context("Node2 decode failed")?;
        let s1 = node1.spec().context("Node1 spec not found")?;
        let s2 = node2.spec().context("Node2 spec not found")?;
        Ok(Self::same_node_type(n1, n2) && Self::compatible_content(s1, s2))
    }

    fn same_node_type(node1: &Node, node2: &Node) -> bool {
        std::mem::discriminant(node1) == std::mem::discriminant(node2)
    }

    fn compatible_content(spec1: &NodeSpec, spec2: &NodeSpec) -> bool {
        spec1.content == spec2.content
    }

    fn merge_nodes(&mut self, target_id: NodeId, source_id: NodeId) -> Result<()> {
        if !self.can_join_nodes(target_id, source_id)? {
            anyhow::bail!("Cannot join incompatible nodes");
        }

        let (first_child, last_child, target_last) = {
            let source = self
                .doc()
                .node(source_id)
                .context("Source node not found")?;
            let target = self
                .doc()
                .node(target_id)
                .context("Target node not found")?;

            (
                source.first_child().map(|n| n.node_id()),
                source.last_child().map(|n| n.node_id()),
                target.last_child().map(|n| n.node_id()),
            )
        };

        if let (Some(first), Some(last)) = (first_child, last_child) {
            self.move_node_range(first, last, Some(target_id), target_last, None)?;
        }

        self.delete_node_recursive(source_id)?;

        Ok(())
    }

    fn perform_merge(
        &mut self,
        position: Position,
        fragment_node: NodeId,
        depth: usize,
        at_start: bool,
        target_index: Option<usize>,
    ) -> Result<Option<NodeId>> {
        let source_depth = depth;
        let target_depth = depth + 1;

        let open_node_id = self.find_open_node_in_doc(fragment_node, source_depth, at_start)?;
        let target_node_id =
            self.find_target_node_at_depth(position, target_depth, at_start, target_index)?;

        if let Some(open_node_id) = open_node_id {
            if let Some(open_node) = self.doc().node(open_node_id) {
                if matches!(open_node.node(), Some(Node::Text(_))) {
                    return Ok(None);
                }
            }
        }

        if let (Some(open_id), Some(target_id)) = (open_node_id, target_node_id) {
            let open_node = self.doc().node(open_id).context("Open node not found")?;
            let target_node = self
                .doc()
                .node(target_id)
                .context("Target node not found")?;

            if open_node.node_type() != target_node.node_type() {
                return Ok(None);
            }

            if matches!(open_node.node(), Some(Node::Text(_)))
                || matches!(target_node.node(), Some(Node::Text(_)))
            {
                return Ok(None);
            }

            if at_start {
                self.merge_nodes(target_id, open_id)?;
                return Ok(Some(target_id));
            } else {
                self.merge_nodes(open_id, target_id)?;
                return Ok(Some(open_id));
            }
        }

        Ok(None)
    }

    fn find_open_node_in_doc(
        &self,
        start_id: NodeId,
        depth: usize,
        use_first: bool,
    ) -> Result<Option<NodeId>> {
        let mut current_id = start_id;
        for _ in 0..depth {
            let node = self.doc().node(current_id).context("Node not found")?;
            let child = if use_first {
                node.first_child()
            } else {
                node.last_child()
            };

            if let Some(child_node) = child {
                current_id = child_node.node_id();
            } else {
                return Ok(None);
            }
        }
        Ok(Some(current_id))
    }

    fn find_target_node_at_depth(
        &self,
        position: Position,
        depth: usize,
        from_start: bool,
        target_index: Option<usize>,
    ) -> Result<Option<NodeId>> {
        let mut current_id = position.node_id;

        for i in 0..depth {
            let node = self.doc().node(current_id).context("Node not found")?;

            let child = if i == 0 {
                if let Some(index) = target_index {
                    if index < node.children().count() {
                        node.children().nth(index)
                    } else {
                        return Ok(None);
                    }
                } else {
                    if from_start {
                        node.last_child()
                    } else {
                        node.first_child()
                    }
                }
            } else {
                if from_start {
                    node.last_child()
                } else {
                    node.first_child()
                }
            };

            if let Some(child) = child {
                current_id = child.node_id();
            } else {
                return Ok(None);
            }
        }
        Ok(Some(current_id))
    }

    fn insert_fragment_nodes(&mut self, position: Position, fragment: &Fragment) -> Result<()> {
        let parent_node = self
            .node_mut(position.node_id)
            .context("Parent node not found")?;

        let mut changed_nodes = Vec::new();
        let mut structure_changed = false;

        let (insert_index, split_tail) = if let Some((child_id, local_offset)) =
            find_child_at_offset(&parent_node, position.offset)
        {
            let child = self.doc().node(child_id).context("Child not found")?;
            if let Some(Node::Text(text_node)) = child.node() {
                if local_offset > 0 && local_offset < text_node.text.char_len() {
                    let (head_text, tail_text) = self.split_text_node_at(child_id, local_offset)?;

                    let child = self
                        .node_mut(child_id)
                        .context("Child not found for update")?;
                    child.as_mut().update(|node| {
                        if let Node::Text(n) = node {
                            n.text = head_text;
                        }
                    })?;
                    changed_nodes.push(child_id);

                    (
                        child.index().context("Child has no index")? + 1,
                        Some(tail_text),
                    )
                } else if local_offset == 0 {
                    (child.index().context("Child has no index")?, None)
                } else {
                    (child.index().context("Child has no index")? + 1, None)
                }
            } else {
                if local_offset == 0 {
                    (child.index().context("Child has no index")?, None)
                } else {
                    (child.index().context("Child has no index")? + 1, None)
                }
            }
        } else {
            (parent_node.children().count(), None)
        };

        let nodes_to_insert = fragment.content_node_ids();

        let mut index = insert_index;
        for (node_id, fragment_node) in fragment.iter() {
            if nodes_to_insert.contains(node_id) {
                parent_node.as_mut().insert_child_with_id(
                    index,
                    *node_id,
                    fragment_node.data().clone(),
                )?;
                index += 1;
                structure_changed = true;
            }
        }

        if let Some(tail_text) = split_tail {
            parent_node
                .as_mut()
                .insert_child(index, Node::Text(TextNode { text: tail_text }))?;
            structure_changed = true;
        }

        for (node_id, fragment_node) in fragment.iter() {
            if !nodes_to_insert.contains(node_id) {
                if let Some(parent_id) = fragment_node.parent() {
                    let in_nodes_to_insert = nodes_to_insert.contains(&parent_id);
                    let parent_exists = self.doc().node(parent_id).is_some();

                    if in_nodes_to_insert || parent_exists {
                        let parent = self
                            .node_mut(parent_id)
                            .context("Fragment parent not found")?;
                        let child_index = parent
                            .children()
                            .position(|c| c.node_id() == *node_id)
                            .unwrap_or_else(|| parent.children().count());
                        parent.as_mut().insert_child_with_id(
                            child_index,
                            *node_id,
                            fragment_node.data().clone(),
                        )?;
                        structure_changed = true;
                    }
                }
            }
        }
        self.mark_attr_mutation(position.node_id);
        if structure_changed {
            self.mark_structure_mutation(position.node_id);
        }

        for node_id in changed_nodes {
            self.mark_attr_mutation(node_id);
        }

        Ok(())
    }

    fn insert_fragment_children_recursive(
        &mut self,
        node_id: NodeId,
        fragment: &Fragment,
    ) -> Result<()> {
        let children = fragment.children_of_node(node_id);
        for (idx, (child_id, child_node)) in children.iter().enumerate() {
            let parent = self.node_mut(node_id).context("Parent not found")?;
            parent
                .as_mut()
                .insert_child_with_id(idx, *child_id, child_node.data().clone())?;
            self.insert_fragment_children_recursive(*child_id, fragment)?;
        }
        Ok(())
    }

    fn insert_split_children(
        &mut self,
        para_id: NodeId,
        children: &[SplitChild],
        start_idx: usize,
    ) -> Result<(usize, usize)> {
        let para = self.doc().node(para_id).context("Paragraph not found")?;
        let mut idx = start_idx;
        let mut content_len = 0;
        for child in children {
            match child {
                SplitChild::Text(segments) => {
                    let text_obj = Text::from_segments(segments);
                    content_len += text_obj.char_len();
                    if text_obj.char_len() > 0 {
                        para.as_mut()
                            .insert_child(idx, Node::Text(TextNode { text: text_obj }))?;
                        idx += 1;
                    }
                }
                SplitChild::Node(node_data) => {
                    let new_id = NodeId::new();
                    para.as_mut()
                        .insert_child_with_id(idx, new_id, node_data.clone())?;
                    idx += 1;
                    content_len += 1;
                }
            }
        }
        Ok((idx - start_idx, content_len))
    }

    fn set_selection_position(&mut self, position: Position) {
        self.state.selection = Selection::collapsed(position);
        self.recompute_pending_styles();
    }

    // 블록을 타겟 부모 위치로 구조적 제약(structural/isolating)을 준수하며 lift하고 빈 조상을 정리
    pub fn try_lift_block(
        &mut self,
        block_id: NodeId,
        target_parent_id: NodeId,
        at_index: usize,
    ) -> Result<()> {
        if at_index != 0 || block_id == target_parent_id {
            return Ok(());
        }

        let (ancestor_under_target, source_parent_id) = {
            let _subtree_root = match self.doc().node(target_parent_id) {
                Some(n) => n,
                None => return Ok(()),
            };

            if !self.is_ancestor_of(target_parent_id, block_id) {
                return Ok(());
            }

            let mut current_id = block_id;
            while current_id != target_parent_id {
                let current_node = self
                    .doc()
                    .node(current_id)
                    .context("Node not found in lift")?;
                let parent_node = current_node.parent().context("Parent not found in lift")?;

                if current_node.prev_sibling().is_some() {
                    return Ok(());
                }

                if parent_node.node_id() == target_parent_id {
                    break;
                }
                current_id = parent_node.node_id();
            }

            let source_parent_id = self
                .doc()
                .node(block_id)
                .context("Block not found in lift")?
                .parent()
                .context("Block parent not found in lift")?
                .node_id();

            (current_id, source_parent_id)
        };

        let (destination_id, destination_prev_sibling, is_recursive_lift) = {
            let target_node = self
                .doc()
                .node(target_parent_id)
                .context("Target parent not found in lift")?;

            if target_node
                .spec()
                .map_or(false, |s| !s.isolating && !s.structural)
            {
                let container_prev_sibling = target_node.prev_sibling().map(|n| n.node_id());
                if let Some(parent) = target_node.parent() {
                    (parent.node_id(), container_prev_sibling, true)
                } else {
                    (target_parent_id, None, false)
                }
            } else {
                (target_parent_id, None, false)
            }
        };

        if !is_recursive_lift && block_id == ancestor_under_target {
            return Ok(());
        }

        if self
            .move_node_range(
                block_id,
                block_id,
                Some(destination_id),
                destination_prev_sibling,
                None,
            )
            .is_ok()
        {
            self.clean_up_empty_ancestors(source_parent_id)?;
            if destination_id != target_parent_id {
                self.clean_up_empty_ancestors(target_parent_id)?;
            }
        }

        Ok(())
    }

    fn first_top_level_valid_at_parent(&self, fragment: &Fragment, target_node_id: NodeId) -> bool {
        let first_top_id = match fragment.top_level_node_ids().first().copied() {
            Some(id) => id,
            None => return false,
        };
        let Some(first_top) = fragment.node(first_top_id) else {
            return false;
        };
        let Some(target) = self.doc().node(target_node_id) else {
            return false;
        };
        let Some(parent) = target.parent() else {
            return false;
        };

        let first_top_type = first_top.data().as_type();
        let Some(parent_spec) = parent.spec() else {
            return false;
        };
        parent_spec.content.matches(first_top_type)
    }

    fn insert_open_fragment(&mut self, position: Position, fragment: Fragment) -> Result<Position> {
        let block = self
            .doc()
            .node(position.node_id)
            .context("Block not found")?;

        let parent_id = block.parent().context("No parent")?.node_id();
        let block_index = block.index().context("No index")?;

        let top_level_ids = fragment.top_level_node_ids();
        if top_level_ids.is_empty() {
            return Ok(position);
        }

        let (left_children, right_children, split_offset) =
            self.split_paragraph_at_offset(position)?;

        // target 문단의 모든 자식 노드 삭제
        {
            let node = self.doc().node(position.node_id);
            if let Some(node) = node {
                let child_ids: Vec<_> = node.children().map(|c| c.node_id()).collect();
                for id in child_ids {
                    self.node_mut(id)
                        .context("Child not found")?
                        .as_mut()
                        .delete()?;
                }
            }
        }

        let (left_count, _) = self.insert_split_children(position.node_id, &left_children, 0)?;

        let mut last_para_id = position.node_id;
        let mut last_para_content_len = split_offset;
        let mut insert_at = block_index + 1;

        let first_is_paragraph = fragment
            .node(top_level_ids[0])
            .map(|n| matches!(n.data(), Node::Paragraph(_)))
            .unwrap_or(false);

        let delete_original_para = !first_is_paragraph && left_children.is_empty();
        if delete_original_para {
            self.delete_node_recursive(position.node_id)?;
            insert_at = block_index;
        }

        for (i, &node_id) in top_level_ids.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == top_level_ids.len() - 1;

            let Some(frag_node) = fragment.node(node_id) else {
                continue;
            };

            match frag_node.data() {
                Node::Paragraph(_) => {
                    let children = fragment.children_of_node(node_id);
                    let mut content_len = 0;

                    if is_first {
                        // When the current paragraph is empty, adopt the fragment's paragraph settings
                        if left_children.is_empty() && split_offset == 0 {
                            if let Node::Paragraph(frag_para) = frag_node.data() {
                                let frag_para = frag_para.clone();
                                let node_ref =
                                    self.node_mut(position.node_id).context("Block not found")?;
                                node_ref.as_mut().update(|node| {
                                    if let Node::Paragraph(p) = node {
                                        *p = frag_para;
                                    }
                                })?;
                            }
                        }

                        let mut idx = left_count;
                        let para = self
                            .doc()
                            .node(position.node_id)
                            .context("Block not found")?;
                        for (child_id, child_node) in &children {
                            para.as_mut().insert_child_with_id(
                                idx,
                                *child_id,
                                child_node.data().clone(),
                            )?;
                            idx += 1;
                            content_len += match child_node.data() {
                                Node::Text(t) => t.text.char_len(),
                                _ => 1,
                            };
                        }
                        if is_last {
                            self.insert_split_children(position.node_id, &right_children, idx)?;
                        }
                        last_para_content_len = split_offset + content_len;
                    } else {
                        let new_para_id = NodeId::new();
                        let parent = self.node_mut(parent_id).context("Parent not found")?;
                        parent.as_mut().insert_child_with_id(
                            insert_at,
                            new_para_id,
                            frag_node.data().clone(),
                        )?;

                        let new_para = self
                            .doc()
                            .node(new_para_id)
                            .context("New paragraph not found")?;
                        let mut child_idx = 0;
                        for (child_id, child_node) in &children {
                            new_para.as_mut().insert_child_with_id(
                                child_idx,
                                *child_id,
                                child_node.data().clone(),
                            )?;
                            child_idx += 1;
                            content_len += match child_node.data() {
                                Node::Text(t) => t.text.char_len(),
                                _ => 1,
                            };
                        }

                        if is_last {
                            self.insert_split_children(new_para_id, &right_children, child_idx)?;
                            self.merge_adjacent_text_nodes(Position::new(
                                new_para_id,
                                0,
                                Affinity::Downstream,
                            ))?;
                        }

                        last_para_id = new_para_id;
                        last_para_content_len = content_len;
                        insert_at += 1;
                    }
                }
                _ => {
                    let parent = self.node_mut(parent_id).context("Parent not found")?;
                    parent.as_mut().insert_child_with_id(
                        insert_at,
                        node_id,
                        frag_node.data().clone(),
                    )?;
                    self.insert_fragment_children_recursive(node_id, &fragment)?;
                    insert_at += 1;

                    let inserted_node = self
                        .doc()
                        .node(node_id)
                        .context("Inserted node not found")?;
                    let end_pos =
                        leaf_block_end(&inserted_node).context("Cannot find leaf block end")?;
                    last_para_id = end_pos.node_id;
                    last_para_content_len = end_pos.offset;

                    if is_last && !right_children.is_empty() {
                        let new_para_id = NodeId::new();
                        let parent = self.node_mut(parent_id).context("Parent not found")?;
                        parent.as_mut().insert_child_with_id(
                            insert_at,
                            new_para_id,
                            Node::Paragraph(ParagraphNode::default()),
                        )?;

                        let (_, content_len) =
                            self.insert_split_children(new_para_id, &right_children, 0)?;
                        self.merge_adjacent_text_nodes(Position::new(
                            new_para_id,
                            0,
                            Affinity::Downstream,
                        ))?;

                        last_para_id = new_para_id;
                        last_para_content_len = content_len;
                    }
                }
            }
        }

        if !delete_original_para {
            self.merge_adjacent_text_nodes(Position::new(
                position.node_id,
                0,
                Affinity::Downstream,
            ))?;
            self.mark_attr_mutation(position.node_id);
        }

        self.mark_structure_mutation(parent_id);
        let head = Position::new(last_para_id, last_para_content_len, Affinity::Downstream);
        Ok(head)
    }

    fn split_paragraph_at_offset(
        &self,
        position: Position,
    ) -> Result<(Vec<SplitChild>, Vec<SplitChild>, usize)> {
        let block = self
            .doc()
            .node(position.node_id)
            .context("Block not found")?;

        let mut left_children = Vec::new();
        let mut right_children = Vec::new();
        let mut current_offset = 0;
        let split_at = position.offset;

        for child in block.children() {
            let Some(child_data) = child.node() else {
                continue;
            };
            match child_data {
                Node::Text(text_node) => {
                    let char_count = text_node.text.char_len();
                    let child_start = current_offset;
                    let child_end = current_offset + char_count;

                    if child_end <= split_at {
                        left_children.push(SplitChild::Text(text_node.text.get_segments()));
                    } else if child_start >= split_at {
                        right_children.push(SplitChild::Text(text_node.text.get_segments()));
                    } else {
                        let local_split = split_at - child_start;
                        let (left_segs, right_segs) =
                            Fragment::split_segments_at(&text_node.text, local_split);
                        if !left_segs.is_empty() {
                            left_children.push(SplitChild::Text(left_segs));
                        }
                        if !right_segs.is_empty() {
                            right_children.push(SplitChild::Text(right_segs));
                        }
                    }
                    current_offset = child_end;
                }
                _ => {
                    if current_offset < split_at {
                        left_children.push(SplitChild::Node(child_data.clone()));
                    } else {
                        right_children.push(SplitChild::Node(child_data.clone()));
                    }
                    current_offset += 1;
                }
            }
        }

        Ok((left_children, right_children, split_at))
    }

    fn split_text_node_at(&self, node_id: NodeId, offset: usize) -> Result<(Text, Text)> {
        let node = self.doc().node(node_id).context("Node not found")?;
        let text_node = match node.node().context("Node decode failed")? {
            Node::Text(t) => t,
            _ => anyhow::bail!("Not a text node"),
        };

        let (head_segs, tail_segs) = Fragment::split_segments_at(&text_node.text, offset);
        let head = Text::from_segments(&head_segs);
        let tail = Text::from_segments(&tail_segs);

        Ok((head, tail))
    }

    fn find_common_ancestor_and_subtrees(
        &self,
        from: Position,
        to: Position,
    ) -> Result<SubtreeInfo> {
        let from_node = self
            .doc()
            .node(from.node_id)
            .context("From node not found")?;
        let to_node = self.doc().node(to.node_id).context("To node not found")?;

        let common_ancestor_depth = from_node
            .ancestors()
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .zip(to_node.ancestors().collect::<Vec<_>>().iter().rev())
            .take_while(|(a, b)| a.node_id() == b.node_id())
            .count()
            - 1;

        let from_subtree_root = if from_node.depth() == common_ancestor_depth {
            let (child_id, _) = find_child_at_offset(&from_node, from.offset)
                .context("find_common_ancestor_and_subtrees: From child not found")?;
            self.doc()
                .node(child_id)
                .context("From subtree root not found")?
        } else {
            from_node
                .ancestor(common_ancestor_depth + 1)
                .context("From subtree root not found")?
        };

        let to_at_ancestor_level = to_node.depth() == common_ancestor_depth;
        let (to_subtree_root, to_local_offset) = if to_at_ancestor_level {
            let (child_id, local_offset) =
                find_child_at_offset(&to_node, to.offset).context("To child not found")?;
            (
                self.doc()
                    .node(child_id)
                    .context("To subtree root not found")?,
                local_offset,
            )
        } else {
            (
                to_node
                    .ancestor(common_ancestor_depth + 1)
                    .context("To subtree root not found")?,
                0,
            )
        };

        let to_index = to_subtree_root
            .index()
            .context("To subtree root has no index")?;
        let from_index = from_subtree_root
            .index()
            .context("From subtree root has no index")?;

        let siblings = if to_index > from_index + 1 {
            Some((
                from_subtree_root
                    .next_sibling()
                    .map(|n| n.node_id())
                    .context("From subtree root has no next sibling")?,
                to_subtree_root
                    .prev_sibling()
                    .map(|n| n.node_id())
                    .context("To subtree root has no prev sibling")?,
            ))
        } else {
            None
        };

        Ok(SubtreeInfo {
            from_subtree_root_id: from_subtree_root.node_id(),
            to_subtree_root_id: to_subtree_root.node_id(),
            to_local_offset,
            to_at_ancestor_level,
            siblings_between_roots: siblings,
        })
    }

    fn collect_siblings_until_ancestor(
        &self,
        start_node_id: NodeId,
        target_node_id: NodeId,
        direction: SiblingDirection,
    ) -> Result<Vec<NodeId>> {
        let mut siblings = Vec::new();
        let start_node = self
            .doc()
            .node(start_node_id)
            .context("Start node not found")?;

        let mut current = match direction {
            SiblingDirection::Following => start_node.next_sibling().map(|n| n.node_id()),
            SiblingDirection::Preceding => start_node.prev_sibling().map(|n| n.node_id()),
        };

        while let Some(id) = current {
            let is_ancestor = self
                .doc()
                .node(target_node_id)
                .context("Target node not found")?
                .ancestors()
                .any(|a| a.node_id() == id);

            if is_ancestor {
                break;
            }

            siblings.push(id);
            current = self.doc().node(id).and_then(|n| match direction {
                SiblingDirection::Following => n.next_sibling().map(|s| s.node_id()),
                SiblingDirection::Preceding => n.prev_sibling().map(|s| s.node_id()),
            });
        }

        Ok(siblings)
    }

    fn collect_ancestor_siblings(
        &self,
        start_node_id: NodeId,
        ancestor_root_id: NodeId,
        direction: SiblingDirection,
    ) -> Result<Vec<NodeId>> {
        let mut current_id = start_node_id;
        let mut siblings_to_delete = Vec::new();

        let root_depth = self
            .doc()
            .node(ancestor_root_id)
            .map(|n| n.depth())
            .unwrap_or(0);
        let start_depth = self
            .doc()
            .node(start_node_id)
            .map(|n| n.depth())
            .unwrap_or(0);

        if start_depth <= root_depth {
            return Ok(siblings_to_delete);
        }

        while current_id != ancestor_root_id {
            let current_node = self
                .doc()
                .node(current_id)
                .context("Current node not found")?;
            let parent = current_node.parent().context("Parent not found")?;
            let parent_id = parent.node_id();

            let mut sibling = match direction {
                SiblingDirection::Following => current_node.next_sibling().map(|n| n.node_id()),
                SiblingDirection::Preceding => current_node.prev_sibling().map(|n| n.node_id()),
            };

            while let Some(id) = sibling {
                siblings_to_delete.push(id);
                sibling = self.doc().node(id).and_then(|n| match direction {
                    SiblingDirection::Following => n.next_sibling().map(|s| s.node_id()),
                    SiblingDirection::Preceding => n.prev_sibling().map(|s| s.node_id()),
                });
            }

            current_id = parent_id;
        }

        Ok(siblings_to_delete)
    }
}

struct SubtreeInfo {
    from_subtree_root_id: NodeId,
    to_subtree_root_id: NodeId,
    to_local_offset: usize,
    to_at_ancestor_level: bool,
    siblings_between_roots: Option<(NodeId, NodeId)>,
}

#[derive(Clone, Copy)]
enum DeleteDirection {
    /// 오프셋 이후 삭제 (from 쪽)
    FromOffset,
    /// 오프셋 이전 삭제 (to 쪽)
    UntilOffset,
}

#[derive(Clone, Copy)]
enum SiblingDirection {
    Following,
    Preceding,
}

enum SplitChild {
    Text(Vec<TextSegment>),
    Node(Node),
}

#[cfg(test)]
mod tests {
    use crate::model::{DefaultAttrs, Fragment, FragmentNode, Node, NodeId, Text, TextNode};
    use crate::state::Position;
    use crate::types::Affinity;

    #[test]
    fn test_nested_merge_traversal_open_start() {
        let mut list = id!();
        let mut item = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @list bullet_list {
                    @item list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                        @p2 paragraph {
                            text { "B" }
                        }
                    }
                }
            }
            selection { (p2, 1) }
        };

        let fragment = fragment! {
            open_start: 2, open_end: 0,
            list_item {
                paragraph {
                    text { "C" }
                }
            }
        };

        let actual = transact!(initial, |tr| {
            let result = tr.insert_fragment(tr.selection().head, fragment).unwrap();
            if let Some(selection) = result.as_selection() {
                tr.set_selection(selection);
            }
        });

        let expected = state! {
            doc {
                @list bullet_list {
                    @item list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                        @p2 paragraph {
                            text { "BC" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p2, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn test_nested_merge_traversal_open_end() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                    }
                    list_item {
                        paragraph {
                            text { "B" }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let fragment = fragment! {
            open_start: 0, open_end: 2,
            list_item {
                paragraph {
                    text { "C" }
                }
            }
        };

        let actual = transact!(initial, |tr| {
            let result = tr.insert_fragment(tr.selection().head, fragment).unwrap();
            if let Some(selection) = result.as_selection() {
                tr.set_selection(selection);
            }
        });

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "CA" }
                        }
                    }
                    list_item {
                        paragraph {
                            text { "B" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_open_fragment_merges_inline_and_preserves_styles() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph {
                text(styles: [italic()]) { "X" }
            }
        };

        let actual = transact!(initial, |tr| {
            let result = tr.insert_fragment(tr.selection().head, fragment).unwrap();
            if let Some(selection) = result.as_selection() {
                tr.set_selection(selection);
            }
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "A", "X" => [italic()], "B" }
                }
            }
            selection { (p, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_empty_fragment_is_noop() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = Fragment::empty();

        let actual = transact!(initial.clone(), |tr| {
            tr.insert_fragment(tr.selection().head, fragment).unwrap();
        });

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn insert_fragment_with_external_parent_updates_selection_correctly() {
        let mut p = id!();
        let external_parent = id!(); // Simulate an external parent ID

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                }
            }
            selection { (p, 6) }
        };

        let text_obj: Text = "World".into();
        let defaults = DefaultAttrs::default().to_styles();
        for style in &defaults {
            let _ = text_obj.apply_style(0..text_obj.char_len(), style);
        }
        let text_node = Node::Text(TextNode { text: text_obj });
        let fragment_node = FragmentNode::new(text_node, Some(external_parent));
        let node_id = NodeId::new();

        let fragment = Fragment {
            nodes: indexmap::IndexMap::from_iter([(node_id, fragment_node)]),
            open_start: 0,
            open_end: 0,
        };

        let actual = transact!(initial, |tr| {
            let result = tr.insert_fragment(tr.selection().head, fragment).unwrap();
            if let Some(selection) = result.as_selection() {
                tr.set_selection(selection);
            }
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello World" }
                }
            }
            selection { (p, 11) } // Selection should be at end (6 + 5)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_node_with_selection_adjustment_moves_selection_from_deleted_subtree_to_start() {
        let mut p_before = id!();
        let mut table = id!();
        let mut cell_p = id!();
        let mut p_after = id!();

        let initial = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @table table {
                    table_row {
                        table_cell { @cell_p paragraph { text { "cell" } } }
                    }
                }
                @p_after paragraph { text { "after" } }
            }
            selection { (cell_p, 2) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_node_with_selection_adjustment(table).unwrap();
        });

        let expected = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @p_after paragraph { text { "after" } }
            }
            selection { (NodeId::ROOT, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_node_with_selection_adjustment_updates_mixed_selection_endpoints() {
        let mut p_before = id!();
        let mut table = id!();
        let mut cell_p = id!();
        let mut p_mid = id!();
        let mut p_tail = id!();

        let initial = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @table table {
                    table_row {
                        table_cell { @cell_p paragraph { text { "cell" } } }
                    }
                }
                @p_mid paragraph { text { "mid" } }
                @p_tail paragraph { text { "tail" } }
            }
            selection { (cell_p, 1) -> (NodeId::ROOT, 3) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_node_with_selection_adjustment(table).unwrap();
        });

        let expected = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @p_mid paragraph { text { "mid" } }
                @p_tail paragraph { text { "tail" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn open_end_merge_merges_adjacent_text_in_fold_title() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "start" } }
                fold {
                    fold_title { text { "World" } }
                    fold_content {
                        paragraph { text { "inside" } }
                    }
                }
            }
            selection { (p, 5) }
        };

        // Fragment: open Fold with only FoldTitle("New"), open_end=2.
        // open_end merge at depth 1 merges FoldTitle("New") with FoldTitle("World"),
        // creating adjacent text nodes [Text("New"), Text("World")] in the surviving FoldTitle.
        // The is_textblock check at line 1426 ensures these get consolidated into "NewWorld".
        let fragment = fragment! {
            open_start: 0, open_end: 2,
            fold {
                fold_title { text { "New" } }
            }
        };

        let pos = Position::new(NodeId::ROOT, 1, Affinity::Downstream);

        let state = transact!(initial, |tr| {
            let result = tr.insert_fragment(pos, fragment).unwrap();
            if let Some(selection) = result.as_selection() {
                tr.set_selection(selection);
            }
        });

        let doc = &state.doc;

        // Navigate: Root → Fold → FoldTitle
        let root = doc.node(NodeId::ROOT).expect("Root should exist");
        let fold = root
            .children()
            .find(|c| matches!(c.node(), Some(Node::Fold(_))))
            .expect("Fold should exist");
        let fold_title = fold
            .children()
            .find(|c| matches!(c.node(), Some(Node::FoldTitle(_))))
            .expect("FoldTitle should exist");

        // Verify FoldTitle has exactly 1 text child (merged), not 2 separate ones
        let text_children: Vec<_> = fold_title
            .children()
            .filter(|c| matches!(c.node(), Some(Node::Text(_))))
            .collect();

        assert_eq!(
            text_children.len(),
            1,
            "FoldTitle should have 1 merged text node, not {} separate ones",
            text_children.len()
        );

        // Verify the merged text content
        if let Some(Node::Text(t)) = text_children[0].node() {
            assert_eq!(
                t.text.as_str(),
                "NewWorld",
                "Merged text should be 'NewWorld'"
            );
        } else {
            panic!("Expected text node");
        }
    }
}

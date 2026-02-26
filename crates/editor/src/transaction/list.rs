use crate::model::{
    BulletListNode, ListItemNode, Node, NodeId, NodeType, OrderedListNode, ParagraphNode,
};
use crate::runtime::Effect;
use crate::state::selection_helpers::{block_content_len, collect_top_level_blocks_in_range};
use crate::state::{Position, Selection};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result, anyhow};

impl Transaction {
    pub fn split_list_item(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let block_id = selection.head.node_id;
        let block = self.node(block_id).context("Block not found")?;
        let Some(parent) = block.parent() else {
            return Ok(false);
        };

        let Some(parent_type) = parent.node().map(|n| n.as_type()) else {
            return Ok(false);
        };
        if parent_type != NodeType::ListItem {
            return Ok(false);
        }

        let Some(grand_parent) = parent.parent() else {
            return Ok(false);
        };

        let Some(grand_parent_type) = grand_parent.node().map(|n| n.as_type()) else {
            return Ok(false);
        };
        if !matches!(
            grand_parent_type,
            NodeType::BulletList | NodeType::OrderedList
        ) {
            return Ok(false);
        }

        let list_item_id = parent.node_id();
        let grand_parent_id = grand_parent.node_id();

        let block_is_empty = block.children().count() == 0;
        if block_is_empty {
            return self.lift_list_item();
        }

        if !self.split_paragraph()? {
            return Ok(false);
        }

        let list_item = self.node(list_item_id).context("List item not found")?;

        let mut second_para_id: Option<NodeId> = None;
        let mut para_count = 0;
        for child in list_item.children() {
            if matches!(child.node(), Some(Node::Paragraph(_))) {
                para_count += 1;
                if para_count == 2 {
                    second_para_id = Some(child.node_id());
                    break;
                }
            }
        }

        // 두번째 paragraph를 새로운 list item으로 이동
        if let Some(second_para_id) = second_para_id {
            let list_node = self
                .node_mut(grand_parent_id)
                .context("List node not found")?;
            let list_item_index = list_item.index().context("List item index not found")?;

            let new_list_item_id = list_node
                .as_mut()
                .insert_child(list_item_index + 1, Node::ListItem(ListItemNode::default()))?;

            self.move_node(second_para_id, new_list_item_id, 0)?;

            let list_item = self.node(list_item_id).context("List item not found")?;
            let children_to_move: Vec<NodeId> =
                list_item.children().skip(1).map(|c| c.node_id()).collect();

            for (idx, child_id) in children_to_move.iter().enumerate() {
                self.move_node(*child_id, new_list_item_id, idx + 1)?;
            }

            let new_list_item = self
                .node(new_list_item_id)
                .context("New list item not found")?;
            let first_para = new_list_item
                .first_child()
                .context("First child not found")?;

            self.set_selection(Selection::collapsed(Position::new(
                first_para.node_id(),
                0,
                Affinity::default(),
            )));
        }

        Ok(true)
    }

    pub fn lift_list_item(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        self.lift_node(selection.head.node_id)
            .map(|opt| opt.is_some())
    }

    fn lift_node(&mut self, node_id: NodeId) -> Result<Option<NodeId>> {
        let Some(ctx) = self.build_lift_context(node_id)? else {
            return Ok(None);
        };

        let new_id = if ctx.owner_type == NodeType::ListItem {
            self.lift_nested_list_item(&ctx)?
        } else {
            self.lift_top_level_list_item(&ctx)?
        };

        self.push_effect(Effect::NodeChanged {
            node_id: ctx.list_id,
        });
        self.push_effect(Effect::NodeChanged {
            node_id: ctx.owner_id,
        });
        self.push_effect(Effect::StructureChanged);

        Ok(new_id)
    }

    pub fn sink_list_item(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;
        let parent = block.parent().context("Parent not found")?;

        if parent.node_type() != Some(NodeType::ListItem) {
            return Ok(false);
        }

        let list_item_id = parent.node_id();
        let list_item = self.node(list_item_id).context("List item not found")?;

        let prev_sibling = list_item.prev_sibling();
        if let Some(prev) = prev_sibling {
            let prev_id = prev.node_id();

            let grand_parent = list_item.parent().context("Grand parent not found")?;
            let list_type = grand_parent.node_type();

            let target_list_id = if let Some(last_child) = prev.last_child() {
                if last_child.node_type() == list_type {
                    Some(last_child.node_id())
                } else {
                    None
                }
            } else {
                None
            };

            let target_list_id = if let Some(id) = target_list_id {
                id
            } else {
                let new_list = match list_type {
                    Some(NodeType::BulletList) => Node::BulletList(BulletListNode::default()),
                    Some(NodeType::OrderedList) => Node::OrderedList(OrderedListNode::default()),
                    _ => return Ok(false),
                };

                let prev_mut = self
                    .node_mut(prev_id)
                    .context("Previous sibling not found")?;
                let count = prev_mut.children().count();
                prev_mut.as_mut().insert_child(count, new_list)?
            };

            let target_list = self.node(target_list_id).context("Target list not found")?;
            let count = target_list.children().count();
            self.move_node(list_item_id, target_list_id, count)?;

            self.push_effect(Effect::NodeChanged {
                node_id: list_item_id,
            });
            self.push_effect(Effect::NodeChanged {
                node_id: target_list_id,
            });
            self.push_effect(Effect::StructureChanged);
            return Ok(true);
        }

        Ok(false)
    }

    pub fn merge_list_item_forward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;

        if !matches!(block.node(), Some(Node::Paragraph(_))) {
            return Ok(false);
        }

        let at_end = selection.head.offset == block_content_len(&block);
        if !at_end || block.next_sibling().is_some() {
            return Ok(false);
        }

        let parent = block.parent().context("Parent not found")?;
        if parent.node_type() != Some(NodeType::ListItem) {
            return Ok(false);
        }

        let list_item_id = parent.node_id();
        let list_item = self.node(list_item_id).context("List item not found")?;
        let list = list_item.parent().context("List not found")?;
        let list_id = list.node_id();

        if let Some(next_sibling) = list_item.next_sibling() {
            let next_sibling_id = next_sibling.node_id();

            if next_sibling.node_type() == Some(NodeType::ListItem) {
                let children_ids: Vec<NodeId> =
                    next_sibling.children().map(|c| c.node_id()).collect();
                let count = list_item.children().count();

                for (i, child_id) in children_ids.into_iter().enumerate() {
                    self.move_node(child_id, list_item_id, count + i)?;
                }

                self.node_mut(next_sibling_id)
                    .context("Next sibling not found")?
                    .as_mut()
                    .delete()?;
                self.push_effect(Effect::NodeChanged { node_id: list_id });
                self.push_effect(Effect::StructureChanged);
                return self.join_forward();
            }
        } else {
            let grand_parent = list.parent().context("Grand parent not found")?;
            if let Some(next_block) = grand_parent.next_sibling() {
                let next_block_id = next_block.node_id();
                let count = list_item.children().count();
                self.move_node(next_block_id, list_item_id, count)?;
                self.push_effect(Effect::NodeChanged { node_id: list_id });
                self.push_effect(Effect::StructureChanged);
                return self.join_forward();
            }
        }

        Ok(false)
    }

    pub fn merge_list_item_backward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() || selection.head.offset != 0 {
            return Ok(false);
        }

        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;

        if block.index().unwrap_or(0) != 0 {
            return Ok(false);
        }

        let parent = block.parent().context("Parent not found")?;
        if parent.node_type() != Some(NodeType::ListItem) {
            return Ok(false);
        }

        let list_item_id = parent.node_id();
        let list_item = self.node(list_item_id).context("List item not found")?;

        let Some(prev_sibling) = list_item.prev_sibling() else {
            return Ok(false);
        };

        let prev_sibling_id = prev_sibling.node_id();

        if prev_sibling.node_type() == Some(NodeType::ListItem) {
            let children_ids: Vec<NodeId> = list_item.children().map(|c| c.node_id()).collect();
            let count = prev_sibling.children().count();

            for (i, child_id) in children_ids.into_iter().enumerate() {
                self.move_node(child_id, prev_sibling_id, count + i)?;
            }

            self.node_mut(list_item_id)
                .context("List item not found")?
                .as_mut()
                .delete()?;
            return self.join_backward();
        }

        Ok(false)
    }

    pub fn toggle_bullet_list(&mut self) -> Result<bool> {
        self.toggle_list(NodeType::BulletList)
    }

    pub fn toggle_ordered_list(&mut self) -> Result<bool> {
        self.toggle_list(NodeType::OrderedList)
    }

    fn toggle_list(&mut self, list_type: NodeType) -> Result<bool> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            self.toggle_list_collapsed(list_type)
        } else {
            self.toggle_list_range(list_type)
        }
    }

    fn toggle_list_range(&mut self, list_type: NodeType) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;
        let start_node_id = from.node_id;
        let end_node_id = to.node_id;
        let start_offset = from.offset;
        let end_offset = to.offset;
        let start_affinity = from.affinity;
        let end_affinity = to.affinity;

        let blocks = collect_top_level_blocks_in_range(self.doc(), from, to)?;

        if blocks.is_empty() {
            return Ok(false);
        }

        let should_lift = blocks
            .iter()
            .all(|&block_id| self.is_block_in_list_of_type(block_id, list_type));

        if should_lift {
            for block_id in blocks {
                let node_type = self.doc().node(block_id).and_then(|n| n.node_type());
                if node_type == Some(NodeType::ListItem) {
                    let children: Vec<NodeId> = self
                        .doc()
                        .node(block_id)
                        .map(|n| n.children().map(|c| c.node_id()).collect())
                        .unwrap_or_default();
                    for child_id in children {
                        let _ = self.lift_node(child_id);
                    }
                } else {
                    let _ = self.lift_node(block_id);
                }
            }
        } else {
            let mut current_list_id: Option<NodeId> = None;
            let mut converted_lists: std::collections::HashSet<NodeId> =
                std::collections::HashSet::new();

            for block_id in blocks {
                if self.node(block_id).is_none() {
                    continue;
                }

                current_list_id = self.process_block_for_list_toggle(
                    block_id,
                    list_type,
                    current_list_id,
                    &mut converted_lists,
                )?;
            }
        }

        self.set_selection(Selection::new(
            Position::new(start_node_id, start_offset, start_affinity),
            Position::new(end_node_id, end_offset, end_affinity),
        ));

        self.push_effect(Effect::StructureChanged);
        Ok(true)
    }

    fn process_block_for_list_toggle(
        &mut self,
        block_id: NodeId,
        list_type: NodeType,
        current_list_id: Option<NodeId>,
        converted_lists: &mut std::collections::HashSet<NodeId>,
    ) -> Result<Option<NodeId>> {
        let block = self.node(block_id).context("Block not found")?;
        let parent = block.parent().context("Parent not found")?;
        let parent_type = parent.node_type();

        let target_block_id = block_id;

        let list_info = if parent_type == Some(NodeType::ListItem) {
            let list = parent.parent().context("List not found")?;
            Some((list.node_id(), list.node_type()))
        } else if matches!(
            parent_type,
            Some(NodeType::BulletList) | Some(NodeType::OrderedList)
        ) {
            Some((parent.node_id(), parent_type))
        } else {
            None
        };

        if let Some((list_id, list_node_type)) = list_info {
            if list_node_type == Some(list_type) {
                return Ok(Some(list_id));
            } else if converted_lists.contains(&list_id) {
                return Ok(current_list_id);
            } else if current_list_id == Some(list_id) {
                return Ok(current_list_id);
            } else {
                let new_list_id = self.convert_list_type(list_id, list_type)?;
                converted_lists.insert(list_id);
                return Ok(Some(new_list_id));
            }
        }

        let (parent_id, prev_sibling_id) = {
            let block = self
                .node(target_block_id)
                .context("Target block not found")?;
            let parent = block.parent().context("Target parent not found")?;
            let prev = block.prev_sibling().map(|n| n.node_id());
            (parent.node_id(), prev)
        };

        if let Some(list_id) = current_list_id {
            if self.try_merge_block_into_list(
                target_block_id,
                list_id,
                parent_id,
                prev_sibling_id,
            )? {
                return Ok(current_list_id);
            }
        }

        let new_list_id = self.wrap_block_in_new_list(target_block_id, list_type)?;
        Ok(Some(new_list_id))
    }

    fn toggle_list_collapsed(&mut self, list_type: NodeType) -> Result<bool> {
        let selection = self.selection().clone();
        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;
        let parent = block.parent().context("Parent not found")?;

        if parent.node_type() == Some(NodeType::ListItem) {
            let list_item = parent;
            let list = list_item.parent().context("List not found")?;

            if list.node_type() == Some(list_type) {
                return self.lift_list_item();
            } else {
                let list_id = list.node_id();
                self.convert_list_type(list_id, list_type)?;
                return Ok(true);
            }
        } else {
            let block_id = block.node_id();

            let new_list_id = self.wrap_block_in_new_list(block_id, list_type)?;
            self.push_effect(Effect::NodeChanged {
                node_id: new_list_id,
            });
            self.push_effect(Effect::StructureChanged);
            return Ok(true);
        }
    }

    fn move_node(
        &mut self,
        node_id: NodeId,
        target_parent_id: NodeId,
        index: usize,
    ) -> Result<NodeId> {
        self.node_mut(node_id)
            .context("Node not found")?
            .as_mut()
            .move_to(target_parent_id, index)?;
        self.push_effect(Effect::NodeChanged {
            node_id: target_parent_id,
        });
        self.push_effect(Effect::NodeChanged { node_id });
        self.push_effect(Effect::StructureChanged);
        Ok(node_id)
    }

    fn try_merge_block_into_list(
        &mut self,
        block_id: NodeId,
        list_id: NodeId,
        expected_parent_id: NodeId,
        expected_prev_sibling: Option<NodeId>,
    ) -> Result<bool> {
        let list = self.node(list_id).context("List not found")?;
        let list_parent = match list.parent() {
            Some(p) => p,
            None => return Ok(false),
        };

        if list_parent.node_id() != expected_parent_id {
            return Ok(false);
        }

        if expected_prev_sibling != Some(list_id) {
            return Ok(false);
        }

        let block_type = self
            .node(block_id)
            .and_then(|n| n.node_type())
            .unwrap_or(NodeType::Paragraph);

        if matches!(block_type, NodeType::BulletList | NodeType::OrderedList) {
            if let Some(last_item) = list.last_child() {
                let last_item_id = last_item.node_id();
                let child_count = last_item.children().count();
                self.move_node(block_id, last_item_id, child_count)?;
                return Ok(true);
            }
        } else {
            let count = list.children().count();
            let new_list_item_id = self
                .node_mut(list_id)
                .context("List not found")?
                .as_mut()
                .insert_child(count, Node::ListItem(ListItemNode::default()))?;
            self.move_node(block_id, new_list_item_id, 0)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn is_block_in_list_of_type(&self, block_id: NodeId, list_type: NodeType) -> bool {
        let Some(block) = self.node(block_id) else {
            return false;
        };

        if let Some(parent) = block.parent() {
            if parent.node_type() == Some(NodeType::ListItem) {
                return parent
                    .parent()
                    .map(|gp| gp.node_type() == Some(list_type))
                    .unwrap_or(false);
            }
        }

        if block.node_type() == Some(NodeType::ListItem) {
            return block
                .parent()
                .map(|p| p.node_type() == Some(list_type))
                .unwrap_or(false);
        }

        false
    }

    fn wrap_block_in_new_list(&mut self, block_id: NodeId, list_type: NodeType) -> Result<NodeId> {
        let block = self.node(block_id).context("Block not found")?;
        let parent = block.parent().context("Parent not found")?;
        let parent_id = parent.node_id();
        let block_index = block.index().context("Block index not found")?;

        let new_list_node = self.new_list_node(list_type)?;
        let new_list_id = self
            .node_mut(parent_id)
            .context("wrap_block_in_new_list: parent node not found")?
            .as_mut()
            .insert_child(block_index, new_list_node)?;

        self.push_effect(Effect::NodeChanged { node_id: parent_id });

        let new_list_item_id = self
            .node_mut(new_list_id)
            .context("wrap_block_in_new_list: new list node not found")?
            .as_mut()
            .insert_child(0, Node::ListItem(ListItemNode::default()))?;

        self.move_node(block_id, new_list_item_id, 0)?;
        Ok(new_list_id)
    }

    fn convert_list_type(&mut self, list_id: NodeId, target_type: NodeType) -> Result<NodeId> {
        let list = self
            .node(list_id)
            .context("List not found for conversion")?;
        let current_type = list.node_type().context("List node type not found")?;

        if current_type == target_type {
            return Ok(list_id);
        }

        let parent = list.parent().context("List parent not found")?;
        let parent_id = parent.node_id();
        let list_index = list.index().context("List index not found")?;

        let new_list_node = self.new_list_node(target_type)?;
        let new_list_id = self
            .node_mut(parent_id)
            .context("Parent not found")?
            .as_mut()
            .insert_child(list_index, new_list_node)?;

        self.push_effect(Effect::NodeChanged { node_id: parent_id });

        let children_ids: Vec<NodeId> = self
            .node(list_id)
            .context("List not found")?
            .children()
            .map(|c| c.node_id())
            .collect();

        for (i, child_id) in children_ids.into_iter().enumerate() {
            self.move_node(child_id, new_list_id, i)?;
            self.push_effect(Effect::NodeChanged { node_id: child_id });
        }

        self.node_mut(list_id)
            .context("Old list not found")?
            .as_mut()
            .delete()?;

        self.push_effect(Effect::NodeChanged {
            node_id: new_list_id,
        });
        self.push_effect(Effect::StructureChanged);
        Ok(new_list_id)
    }

    fn build_lift_context(&self, node_id: NodeId) -> Result<Option<LiftContext>> {
        let block = match self.node(node_id) {
            Some(block) => block,
            None => return Ok(None),
        };

        let parent = match block.parent() {
            Some(parent) => parent,
            None => return Ok(None),
        };

        if parent.node_type() != Some(NodeType::ListItem) {
            return Ok(None);
        }

        let list_item_id = parent.node_id();
        let list_item = self.node(list_item_id).context("List item not found")?;
        let list = list_item.parent().context("List not found")?;
        let list_id = list.node_id();
        let list_type = list.node_type().context("List node type not found")?;

        if !matches!(list_type, NodeType::BulletList | NodeType::OrderedList) {
            return Ok(None);
        }

        let list_index_in_parent = list.index().context("List index not found")?;
        let owner = list.parent().context("List owner not found")?;
        let owner_id = owner.node_id();
        let owner_type = owner.node_type().context("Owner node type not found")?;

        let list_item_children: Vec<NodeId> = list_item.children().map(|c| c.node_id()).collect();
        let target_block_index_in_list_item = list_item_children
            .iter()
            .position(|&id| id == node_id)
            .context("Target block not found in list item children")?;

        Ok(Some(LiftContext {
            list_item_id,
            list_id,
            list_type,
            list_index_in_parent,
            owner_id,
            owner_type,
            list_item_children,
            target_block_index_in_list_item,
        }))
    }

    fn split_after_item(&self, list_id: NodeId, item_id: NodeId) -> Result<(Vec<NodeId>, usize)> {
        let children = self.list_children(list_id)?;
        let item_index = children
            .iter()
            .position(|id| *id == item_id)
            .context("List item not found in list")?;

        let after_items = children.iter().skip(item_index + 1).copied().collect();
        Ok((after_items, item_index))
    }

    fn list_children(&self, list_id: NodeId) -> Result<Vec<NodeId>> {
        Ok(self
            .node(list_id)
            .context("List not found")?
            .children()
            .map(|c| c.node_id())
            .collect())
    }

    fn new_list_node(&self, list_type: NodeType) -> Result<Node> {
        let node = match list_type {
            NodeType::BulletList => Node::BulletList(BulletListNode::default()),
            NodeType::OrderedList => Node::OrderedList(OrderedListNode::default()),
            _ => return Err(anyhow!("Unsupported list type")),
        };

        Ok(node)
    }

    fn delete_list_if_empty(&mut self, list_id: NodeId) -> Result<()> {
        let list = self.node(list_id).context("List not found")?;
        if list.children().next().is_none() {
            self.node_mut(list_id)
                .context("List not found")?
                .as_mut()
                .delete()?;
            self.push_effect(Effect::StructureChanged);
        }

        Ok(())
    }

    fn lift_nested_list_item(&mut self, ctx: &LiftContext) -> Result<Option<NodeId>> {
        let (after_items, _) = self.split_after_item(ctx.list_id, ctx.list_item_id)?;

        let parent_list_item_index = self
            .node(ctx.owner_id)
            .context("Parent list item not found")?
            .index()
            .context("Parent list item index not found")?;

        let target_parent_id = self
            .node(ctx.owner_id)
            .context("Parent list item not found")?
            .parent()
            .context("Target parent not found")?
            .node_id();

        let moved_list_item_id = self.move_node(
            ctx.list_item_id,
            target_parent_id,
            parent_list_item_index + 1,
        )?;

        let moved_list_item = self
            .node(moved_list_item_id)
            .context("Moved list item not found")?;
        let new_block_id = moved_list_item
            .children()
            .nth(ctx.target_block_index_in_list_item)
            .map(|c| c.node_id());

        if !after_items.is_empty() {
            let new_list_node = self.new_list_node(ctx.list_type)?;
            let moved_list_item = self
                .node_mut(moved_list_item_id)
                .context("Moved list item not found")?;
            let insert_at = moved_list_item.children().count();
            let new_list_id = moved_list_item
                .as_mut()
                .insert_child(insert_at, new_list_node)?;

            for (idx, child_id) in after_items.iter().enumerate() {
                self.move_node(*child_id, new_list_id, idx)?;
            }
        }

        self.delete_list_if_empty(ctx.list_id)?;
        Ok(new_block_id)
    }

    fn lift_top_level_list_item(&mut self, ctx: &LiftContext) -> Result<Option<NodeId>> {
        let (after_items, _) = self.split_after_item(ctx.list_id, ctx.list_item_id)?;
        let insert_at = ctx.list_index_in_parent + 1;
        let mut inserted_blocks = 0;
        let mut new_block_id = None;

        if ctx.list_item_children.is_empty() {
            let new_para_id = self
                .node_mut(ctx.owner_id)
                .context("Parent not found")?
                .as_mut()
                .insert_child(insert_at, Node::Paragraph(ParagraphNode::default()))?;

            self.set_selection(Selection::collapsed(Position::new(
                new_para_id,
                0,
                Affinity::default(),
            )));
            inserted_blocks += 1;
            new_block_id = Some(new_para_id);
        } else {
            for (i, child_id) in ctx.list_item_children.iter().enumerate() {
                let new_id = self.move_node(*child_id, ctx.owner_id, insert_at + i)?;
                if i == 0 {
                    self.set_selection(Selection::collapsed(Position::new(
                        new_id,
                        0,
                        Affinity::default(),
                    )));
                }
                if i == ctx.target_block_index_in_list_item {
                    new_block_id = Some(new_id);
                }
                inserted_blocks += 1;
            }
        }

        self.node_mut(ctx.list_item_id)
            .context("List item not found")?
            .as_mut()
            .delete()?;

        if !after_items.is_empty() {
            let new_list_node = self.new_list_node(ctx.list_type)?;
            let new_list_id = self
                .node_mut(ctx.owner_id)
                .context("Parent not found")?
                .as_mut()
                .insert_child(insert_at + inserted_blocks, new_list_node)?;

            for (i, item_id) in after_items.iter().enumerate() {
                self.move_node(*item_id, new_list_id, i)?;
            }
        }

        self.delete_list_if_empty(ctx.list_id)?;
        Ok(new_block_id)
    }
}

struct LiftContext {
    list_item_id: NodeId,
    list_id: NodeId,
    list_type: NodeType,
    list_index_in_parent: usize,
    owner_id: NodeId,
    owner_type: NodeType,
    list_item_children: Vec<NodeId>,
    target_block_index_in_list_item: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::State;

    #[test]
    fn split_list_item_middle() {
        let mut p = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p paragraph {
                            text { "helloworld" }
                        }
                    }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.split_list_item().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph {
                            text { "hello" }
                        }
                    }
                    list_item {
                        @p paragraph {
                            text { "world" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_list_item_with_nested_list() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "parent" }
                        }
                        bullet_list {
                            list_item {
                                paragraph {
                                    text { "nested" }
                                }
                            }
                        }
                    }
                }
            }
            selection { (p1, 6) }
        };

        let actual = transact!(initial, |tr| tr.split_list_item().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph {
                            text { "parent" }
                        }
                    }
                    list_item {
                        @p1 paragraph {}
                        bullet_list {
                            list_item {
                                paragraph {
                                    text { "nested" }
                                }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_middle_ordered_list_item_splits_list_and_inserts_paragraph() {
        let mut middle = id!();

        let initial = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph { text { "a" } }
                    }
                    list_item {
                        @middle paragraph { }
                    }
                    list_item {
                        paragraph { text { "c" } }
                    }
                }
                paragraph { }
            }
            selection { (middle, 0) }
        };

        let actual = transact!(initial, |tr| tr.lift_list_item().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph { text { "a" } }
                    }
                }
                @middle paragraph { }
                ordered_list {
                    list_item {
                        paragraph { text { "c" } }
                    }
                }
                paragraph { }
            }
            selection { (middle, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_nested_middle_ordered_list_item_splits_list() {
        let mut middle = id!();

        let initial = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph { text { "a" } }
                        bullet_list {
                            list_item {
                                paragraph { text { "1" } }
                            }
                            list_item {
                                @middle paragraph { }
                            }
                            list_item {
                                paragraph { text { "3" } }
                            }
                        }
                    }
                    list_item {
                        paragraph { text { "b" } }
                    }
                }
                paragraph { }
            }
            selection { (middle, 0) }
        };

        let actual = transact!(initial, |tr| tr.lift_list_item().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph { text { "a" } }
                        bullet_list {
                            list_item {
                                paragraph { text { "1" } }
                            }
                        }
                    }
                    list_item {
                        @middle paragraph { }
                        bullet_list {
                            list_item {
                                paragraph { text { "3" } }
                            }
                        }
                    }
                    list_item {
                        paragraph { text { "b" } }
                    }
                }
                paragraph { }
            }
            selection { (middle, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_empty_list_item() {
        let mut p = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.split_list_item().unwrap());

        let expected = state! {
            doc {
                @p paragraph {}
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn sink_list_item() {
        let mut p2 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph { text { "1" } }
                    }
                    list_item {
                        @p2 paragraph { text { "2" } }
                    }
                }
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.sink_list_item().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph { text { "1" } }
                        bullet_list {
                            list_item {
                                @p2 paragraph { text { "2" } }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn merge_list_item_forward() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "1" } }
                    }
                    list_item {
                        paragraph { text { "2" } }
                    }
                }
            }
            selection { (p1, 1) }
        };

        let actual = transact!(initial, |tr| tr.merge_list_item_forward().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "12" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn merge_list_item_backward() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        paragraph { text { "1" } }
                    }
                    list_item {
                        @p1 paragraph { text { "2" } }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.merge_list_item_backward().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "12" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn reproduce_list_in_blockquote_rerender_issue() {
        let mut p = id!();
        let mut bq = id!();

        let initial = state! {
            doc {
                @bq blockquote {
                    @p paragraph {
                        text { "item" }
                    }
                }
            }
            selection { (p, 0) }
        };

        let (final_state, effects) =
            transact_with_effect!(initial, |tr| tr.toggle_bullet_list().unwrap());

        let expected = state! {
            doc {
                @bq blockquote {
                    bullet_list {
                        list_item {
                            @p paragraph {
                                text { "item" }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(final_state, expected);

        let has_structure_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::StructureChanged));
        assert!(
            has_structure_changed,
            "Effect::StructureChanged should be emitted"
        );

        let root = final_state.doc.node(NodeId::ROOT).unwrap();
        let bq_id = root.first_child().unwrap().node_id();

        let has_bq_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == bq_id));
        assert!(
            has_bq_changed,
            "Effect::NodeChanged should be emitted for the blockquote"
        );
    }

    #[test]
    fn toggle_list_range_basic() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let doc = doc! {
            @p1 paragraph { text { "1" } }
            @p2 paragraph { text { "2" } }
            @p3 paragraph { text { "3" } }
        };
        let selection = Selection::new(
            Position::new(p1, 0, Affinity::default()),
            Position::new(p3, 1, Affinity::default()),
        );
        let initial = State::new(doc, selection);

        let actual = transact!(initial, |tr| tr.toggle_bullet_list().unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "1" } }
                    }
                    list_item {
                        @p2 paragraph { text { "2" } }
                    }
                    list_item {
                        @p3 paragraph { text { "3" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p3, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_list_range_mixed() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "1" } }
                bullet_list {
                    list_item {
                        @p2 paragraph { text { "2" } }
                    }
                }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr.toggle_ordered_list().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        @p1 paragraph { text { "1" } }
                        bullet_list {
                            list_item {
                                @p2 paragraph { text { "2" } }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_list_range_mixed_2() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "111" } }
                bullet_list {
                    list_item {
                        paragraph { text { "2" } }
                        bullet_list {
                            list_item {
                                paragraph { text { "3" } }
                            }
                        }
                    }
                    list_item {
                        @p2 paragraph { text { "444" } }
                    }
                }
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr.toggle_ordered_list().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        @p1 paragraph { text { "111" } }
                        bullet_list {
                            list_item {
                                paragraph { text { "2" } }
                                bullet_list {
                                    list_item {
                                        paragraph { text { "3" } }
                                    }
                                }
                            }
                            list_item {
                                @p2 paragraph { text { "444" } }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p1, 1) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_list_range_untoggle() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph { text { "1" } }
                    }
                    list_item {
                        @p2 paragraph { text { "2" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr.toggle_bullet_list().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { text { "1" } }
                @p2 paragraph { text { "2" } }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_bullet_list_to_ordered_list() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut list = id!();

        let initial = state! {
            doc {
                @list bullet_list {
                    list_item {
                        @p1 paragraph { text { "First" } }
                    }
                    list_item {
                        @p2 paragraph { text { "Second" } }
                    }
                }
            }
            selection { (p1, 2) -> (p2, 3) }
        };

        let actual = transact!(initial, |tr| tr.toggle_ordered_list().unwrap());

        let expected = state! {
            doc {
                @list ordered_list {
                    list_item {
                        @p1 paragraph { text { "First" } }
                    }
                    list_item {
                        @p2 paragraph { text { "Second" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 2) -> (p2, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_ordered_list_to_bullet_list() {
        let mut p1 = id!();
        let mut list = id!();

        let initial = state! {
            doc {
                @list ordered_list {
                    list_item {
                        @p1 paragraph { text { "Item" } }
                    }
                }
            }
            selection { (p1, 2) }
        };

        let actual = transact!(initial, |tr| tr.toggle_bullet_list().unwrap());

        let expected = state! {
            doc {
                @list bullet_list {
                    list_item {
                        @p1 paragraph { text { "Item" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn reproduce_list_toggle_rerender_issue() {
        let mut p1 = id!();
        let mut list_item_id = id!();

        let initial = state! {
            doc {
                bullet_list {
                    @list_item_id list_item {
                        @p1 paragraph { text { "Item" } }
                    }
                }
            }
            selection { (p1, 2) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| tr.toggle_ordered_list().unwrap());

        let has_structure_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::StructureChanged));
        assert!(
            has_structure_changed,
            "Effect::StructureChanged should be emitted"
        );

        let has_item_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == list_item_id));

        assert!(
            has_item_changed,
            "Effect::NodeChanged should be emitted for the list item"
        );
    }
}

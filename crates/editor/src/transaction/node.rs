use crate::model::*;
use crate::runtime::Effect;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{Position, Selection, block_content_len, collect_top_level_blocks_in_range};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result};

fn common_parent_if_same(tr: &Transaction, block_ids: &[NodeId]) -> Result<Option<NodeId>> {
    if block_ids.is_empty() {
        return Ok(None);
    }

    let first_block = tr.node(block_ids[0]).context("Block not found")?;
    let parent = first_block.parent().context("Block has no parent")?;
    let parent_id = parent.node_id();

    for id in block_ids.iter().skip(1) {
        let block = tr.node(*id).context("Block not found")?;
        let block_parent = block.parent().context("Block has no parent")?;
        if block_parent.node_id() != parent_id {
            return Ok(None);
        }
    }

    Ok(Some(parent_id))
}

impl Transaction {
    pub fn insert_node(&mut self, node: Node) -> Result<bool> {
        if node.is_external(self.doc().schema()) {
            self.push_effect(Effect::ExternalElementChanged);
        }

        let selection = self.selection().clone();

        let insert_pos = if selection.is_collapsed() {
            selection.head
        } else {
            selection.as_sorted(self.doc())?.1
        };

        let node_type = node.as_type();

        if self
            .doc()
            .is_type_forbidden_at(insert_pos.node_id, node_type)
        {
            return Ok(false);
        }

        let (parent_allows_node, parent_info) = {
            let parent_node = self
                .node(insert_pos.node_id)
                .context("Parent node not found")?;
            let parent_spec = parent_node.spec();

            let allows = parent_spec.content.matches(node_type);

            if !allows {
                let grandparent = match parent_node.parent() {
                    Some(g) => g,
                    None => return Ok(false),
                };
                let grandparent_id = grandparent.node_id();
                let grandparent_spec = grandparent.spec();

                if !grandparent_spec.content.matches(node_type) {
                    return Ok(false);
                }

                let parent_id = parent_node.node_id();
                let parent_size = block_content_len(&parent_node);
                let parent_index = parent_node.index();
                let prev_sibling = parent_node.prev_sibling().map(|n| n.node_id());
                let next_sibling = parent_node.next_sibling().map(|n| n.node_id());

                let child_info = if insert_pos.offset > 0 && insert_pos.offset < parent_size {
                    let (child_id, local_offset) =
                        find_child_at_offset(&parent_node, insert_pos.offset)
                            .context("No child at offset")?;

                    let child = self.node(child_id).context("Child not found")?;
                    let next_id = child.next_sibling().map(|n| n.node_id());
                    let parent_next_id = parent_node.next_sibling().map(|n| n.node_id());
                    let parent_last_child_id = parent_node.last_child().map(|n| n.node_id());

                    if let Node::Text(text_node) = child.node() {
                        let (head, tail) = text_node.text.split_at(local_offset);

                        Some((
                            child_id,
                            head,
                            tail,
                            next_id,
                            parent_next_id,
                            parent_last_child_id,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };

                (
                    allows,
                    Some((
                        grandparent_id,
                        parent_id,
                        parent_size,
                        parent_index,
                        prev_sibling,
                        next_sibling,
                        parent_node.node().clone(),
                        child_info,
                    )),
                )
            } else {
                (allows, None)
            }
        };

        if parent_allows_node {
            let new_selection = Selection::new(
                Position::new(insert_pos.node_id, insert_pos.offset, Affinity::Downstream),
                Position::new(
                    insert_pos.node_id,
                    insert_pos.offset + 1,
                    Affinity::Upstream,
                ),
            );

            let node_id = NodeId::new();
            let fragment_node = FragmentNode::new(node, None);
            let fragment = Fragment::builder().add((node_id, fragment_node)).build();

            self.replace_range(insert_pos, insert_pos, fragment)?;
            self.set_selection(new_selection);
        } else {
            let (grandparent_id, _, parent_size, parent_index, _, _, parent_node_data, child_info) =
                parent_info.context("Missing parent info")?;

            if insert_pos.offset == 0 {
                let parent_index = parent_index.context("Parent has no index")?;
                let grandparent = self
                    .node_mut(grandparent_id)
                    .context("Grandparent not found")?;
                grandparent.as_mut().insert_child(parent_index, node)?;

                self.set_selection(Selection::new(
                    Position::new(grandparent_id, parent_index, Affinity::Downstream),
                    Position::new(grandparent_id, parent_index + 1, Affinity::Upstream),
                ));
            } else if insert_pos.offset == parent_size {
                let parent_index = parent_index.context("Parent has no index")?;
                let grandparent = self
                    .node_mut(grandparent_id)
                    .context("Grandparent not found")?;
                grandparent.as_mut().insert_child(parent_index + 1, node)?;

                self.set_selection(Selection::new(
                    Position::new(grandparent_id, parent_index + 1, Affinity::Downstream),
                    Position::new(grandparent_id, parent_index + 2, Affinity::Upstream),
                ));
            } else if let Some((child_id, head, tail, next_id, _, parent_last_child_id)) =
                child_info
            {
                let parent_index = parent_index.context("Parent has no index")?;
                let new_block_id = NodeId::new();
                let grandparent = self
                    .node_mut(grandparent_id)
                    .context("Grandparent not found")?;
                grandparent.as_mut().insert_child_with_id(
                    parent_index + 1,
                    new_block_id,
                    parent_node_data,
                )?;

                if head.is_empty() {
                    self.delete_node_recursive(child_id)?;
                } else {
                    self.node_mut(child_id)
                        .context("Child not found")?
                        .as_mut()
                        .update(|n| {
                            if let Node::Text(t) = n {
                                t.text = head.clone();
                            }
                        })?;
                }

                if let Some(next) = next_id {
                    if let Some(last) = parent_last_child_id {
                        self.move_node_range(next, last, Some(new_block_id), None, None)?;
                    }
                }

                if !tail.is_empty() {
                    let new_block = self.node_mut(new_block_id).context("New block not found")?;
                    new_block.as_mut().insert_child(
                        0,
                        Node::Text(TextNode {
                            text: tail,
                            ..Default::default()
                        }),
                    )?;
                }

                let grandparent = self
                    .node_mut(grandparent_id)
                    .context("Grandparent not found")?;
                grandparent.as_mut().insert_child(parent_index + 1, node)?;

                self.set_selection(Selection::new(
                    Position::new(grandparent_id, parent_index + 1, Affinity::Downstream),
                    Position::new(grandparent_id, parent_index + 2, Affinity::Upstream),
                ));
            } else {
                return Ok(false);
            }
        }

        let new_selection = self.selection().clone();
        self.push_effect(Effect::NodeChanged {
            node_id: new_selection.head.node_id,
        });
        self.push_effect(Effect::StructureChanged);
        Ok(true)
    }

    pub fn expand_selection_until<F>(&self, predicate: F) -> Result<Option<Selection>>
    where
        F: Fn(&Node, &[NodeId]) -> bool,
    {
        let mut selection = self.selection().clone();

        loop {
            let (from, to) = selection.as_sorted(self.doc())?;
            let block_ids = collect_top_level_blocks_in_range(self.doc(), from, to)?;

            if block_ids.is_empty() {
                return Ok(None);
            }

            match common_parent_if_same(self, &block_ids)? {
                Some(parent_id) => {
                    let parent = self.node(parent_id).context("Parent not found")?;

                    if predicate(parent.node(), &block_ids) {
                        return Ok(Some(selection));
                    }

                    if parent_id == NodeId::ROOT {
                        return Ok(None);
                    }

                    selection = self.selection_covering_parent(parent_id)?;
                }
                None => {
                    if let Some(next_selection) = self.selection_for_common_ancestor(&block_ids)? {
                        selection = next_selection;
                    } else {
                        return Ok(None);
                    }
                }
            }
        }
    }

    fn selection_covering_parent(&self, parent_id: NodeId) -> Result<Selection> {
        let parent = self.node(parent_id).context("Parent not found")?;
        let grandparent = parent.parent().context("Grandparent not found")?;
        let parent_index = parent.index().context("Parent index not found")?;

        Ok(Selection::new(
            Position::new(grandparent.node_id(), parent_index, Affinity::Downstream),
            Position::new(grandparent.node_id(), parent_index + 1, Affinity::Upstream),
        ))
    }

    fn selection_for_common_ancestor(&self, block_ids: &[NodeId]) -> Result<Option<Selection>> {
        let first_id = block_ids[0];
        let last_id = *block_ids.last().unwrap();

        let first_ancestors = self.ancestor_chain(first_id)?;
        let last_ancestors = self.ancestor_chain(last_id)?;

        // lowest common ancestor
        let ancestor_id = first_ancestors
            .iter()
            .find(|id| last_ancestors.contains(id))
            .copied()
            .context("No ancestor")?;

        let start_child_id =
            self.find_child_under_ancestor(ancestor_id, first_id, &first_ancestors)?;
        let end_child_id = self.find_child_under_ancestor(ancestor_id, last_id, &last_ancestors)?;

        let start_index = self
            .node(start_child_id)
            .context("Start child node not found")?
            .index()
            .context("Start child index not found")?;
        let end_index = self
            .node(end_child_id)
            .context("End child node not found")?
            .index()
            .context("End child index not found")?;

        Ok(Some(Selection::new(
            Position::new(ancestor_id, start_index, Affinity::Downstream),
            Position::new(ancestor_id, end_index + 1, Affinity::Upstream),
        )))
    }

    fn ancestor_chain(&self, node_id: NodeId) -> Result<Vec<NodeId>> {
        let node = self.node(node_id).context("Node not found")?;
        Ok(node.ancestors().map(|n| n.node_id()).collect())
    }

    pub fn lift_from_ancestor<F>(&mut self, predicate: F) -> Result<bool>
    where
        F: Fn(&Node, &[NodeId]) -> bool,
    {
        let original_selection = self.selection().clone();
        let target = self.expand_selection_until(predicate)?;

        if let Some(target) = target {
            self.set_selection(target);
            let result = self.lift()?;
            if result {
                self.set_selection(original_selection);
            }
            return Ok(result);
        }

        Ok(false)
    }

    pub fn wrap_in_ancestor(&mut self, wrapper: Node) -> Result<bool> {
        let original_selection = self.selection().clone();
        let wrapper_type = wrapper.as_type();
        let wrapper_spec = self.doc().schema().node_spec(wrapper_type);

        let target = self.expand_selection_until(|parent, blocks| {
            let parent_type = parent.as_type();
            let parent_spec = self.doc().schema().node_spec(parent_type);

            let block_types: Vec<NodeType> = blocks
                .iter()
                .map(|id| self.node(*id).unwrap().node().as_type())
                .collect();

            let parent_allows_wrapper = parent_spec.content.matches(wrapper_type);
            let wrapper_allows_blocks =
                block_types.iter().all(|t| wrapper_spec.content.matches(*t));

            if !parent_allows_wrapper || !wrapper_allows_blocks {
                return false;
            }

            // blocks[0]의 parent가 parent_id이므로, parent_id 위치에서 wrapper_type이 금지되는지 확인
            if let Some(&first_block) = blocks.first() {
                if let Some(parent_node) = self.node(first_block) {
                    if let Some(parent_ref) = parent_node.parent() {
                        if self
                            .doc()
                            .is_type_forbidden_at(parent_ref.node_id(), wrapper_type)
                        {
                            return false;
                        }
                    }
                }
            }

            true
        })?;

        if let Some(target) = target {
            self.set_selection(target);
            let result = self.wrap_in(wrapper)?;
            if result {
                self.set_selection(original_selection);
            }
            return Ok(result);
        }

        Ok(false)
    }

    pub fn wrap_in(&mut self, wrapper: Node) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let block_ids = collect_top_level_blocks_in_range(self.doc(), from, to)?;
        let parent_id = match common_parent_if_same(self, &block_ids)? {
            Some(id) => id,
            None => return Ok(false),
        };

        if self
            .doc()
            .is_type_forbidden_at(parent_id, wrapper.as_type())
        {
            return Ok(false);
        }

        let wrapper_spec = self.doc().schema().node_spec(wrapper.as_type());
        let block_types: Vec<NodeType> = block_ids
            .iter()
            .map(|id| self.node(*id).unwrap().node().as_type())
            .collect();

        if !block_types.iter().all(|t| wrapper_spec.content.matches(*t)) {
            return Ok(false);
        }

        let parent = self.node_mut(parent_id).context("Parent not found")?;
        let first_block = self.node(block_ids[0]).context("First block not found")?;
        let wrapper_id = parent.as_mut().insert_child(
            first_block.index().context("First block has no index")?,
            wrapper,
        )?;

        self.move_node_range(
            block_ids[0],
            *block_ids.last().unwrap(),
            Some(wrapper_id),
            None,
            None,
        )?;

        for block_id in &block_ids {
            self.push_effect(Effect::NodeChanged { node_id: *block_id });
        }
        self.push_effect(Effect::NodeChanged {
            node_id: wrapper_id,
        });
        self.push_effect(Effect::StructureChanged);
        Ok(true)
    }

    pub fn lift(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let block_ids = collect_top_level_blocks_in_range(self.doc(), from, to)?;
        let parent_id = match common_parent_if_same(self, &block_ids)? {
            Some(id) => id,
            None => return Ok(false),
        };

        let parent = self.node(parent_id).context("Parent not found")?;

        if parent.spec().isolating {
            return Ok(false);
        }

        let grandparent = match parent.parent() {
            Some(g) => g,
            None => {
                return Ok(false);
            }
        };

        let grandparent_spec = grandparent.spec();
        let block_types: Vec<NodeType> = block_ids
            .iter()
            .map(|id| self.node(*id).unwrap().node().as_type())
            .collect();

        if !block_types
            .iter()
            .all(|t| grandparent_spec.content.matches(*t))
        {
            return Ok(false);
        }

        let remaining_child_types: Vec<NodeType> = parent
            .children()
            .filter(|child| !block_ids.contains(&child.node_id()))
            .map(|child| child.node().as_type())
            .collect();

        if !remaining_child_types.is_empty() {
            let parent_spec = parent.spec();
            if parent_spec
                .content
                .validate(&remaining_child_types)
                .is_err()
            {
                return Ok(false);
            }
        }

        let grandparent_id = grandparent.node_id();
        let parent_prev = parent.prev_sibling().map(|n| n.node_id());

        let first_block = self.node(block_ids[0]).context("First block not found")?;
        let has_prev_siblings = first_block.prev_sibling().is_some();

        let last_block_id = *block_ids.last().unwrap();
        let last_block = self.node(last_block_id).context("Last block not found")?;

        let remaining_after: Vec<NodeId> = {
            let mut siblings = Vec::new();
            let mut current_id = last_block.next_sibling().map(|n| n.node_id());
            while let Some(id) = current_id {
                siblings.push(id);
                current_id = self
                    .node(id)
                    .and_then(|n| n.next_sibling().map(|s| s.node_id()));
            }
            siblings
        };

        if has_prev_siblings && !remaining_after.is_empty() {
            let parent_node = self.node(parent_id).context("Parent not found")?;

            let all_children: Vec<NodeId> = {
                let mut children = Vec::new();
                let mut current_id = parent_node.first_child().map(|n| n.node_id());
                while let Some(id) = current_id {
                    children.push(id);
                    current_id = self
                        .node(id)
                        .and_then(|n| n.next_sibling().map(|s| s.node_id()));
                }
                children
            };

            let lift_start_idx = all_children
                .iter()
                .position(|&id| id == block_ids[0])
                .context("Lift block not found in parent")?;

            let parent_node_data = parent_node.node().clone();
            let parent_index = parent_node.index().context("Parent node has no index")?;
            let grandparent = self
                .node_mut(grandparent_id)
                .context("Grandparent not found")?;
            let new_parent_id = grandparent
                .as_mut()
                .insert_child(parent_index + 1, parent_node_data)?;

            self.move_node_range(
                all_children[0],
                *all_children.last().unwrap(),
                Some(new_parent_id),
                None,
                None,
            )?;

            if lift_start_idx > 0 {
                self.move_node_range(
                    all_children[0],
                    all_children[lift_start_idx - 1],
                    Some(parent_id),
                    None,
                    None,
                )?;
            }

            self.move_node_range(
                block_ids[0],
                last_block_id,
                Some(grandparent_id),
                Some(parent_id),
                Some(new_parent_id),
            )?;
        } else {
            if has_prev_siblings {
                let parent = self.node(parent_id).context("Parent not found")?;
                let parent_next = parent.next_sibling().map(|n| n.node_id());
                self.move_node_range(
                    block_ids[0],
                    last_block_id,
                    Some(grandparent_id),
                    Some(parent_id),
                    parent_next,
                )?;
            } else {
                self.move_node_range(
                    block_ids[0],
                    last_block_id,
                    Some(grandparent_id),
                    parent_prev,
                    Some(parent_id),
                )?;
            }
        }

        let parent_after = self.node(parent_id);
        if parent_after.is_some() && parent_after.unwrap().first_child().is_none() {
            self.delete_node_recursive(parent_id)?;
        }

        for block_id in &block_ids {
            self.push_effect(Effect::NodeChanged { node_id: *block_id });
        }
        self.push_effect(Effect::NodeChanged { node_id: parent_id });
        self.push_effect(Effect::NodeChanged {
            node_id: grandparent_id,
        });
        self.push_effect(Effect::StructureChanged);
        Ok(true)
    }

    pub fn lift_on_empty_paragraph(&mut self) -> Result<bool> {
        let selection = self.selection().clone();

        if !selection.is_collapsed() {
            return Ok(false);
        }

        let this = self
            .node(selection.head.node_id)
            .context("Node not found")?;

        if let Node::Paragraph(_) = this.node() {
            if this.children().count() == 0 {
                if let Some(parent) = this.parent() {
                    if parent.spec().isolating {
                        return Ok(false);
                    }
                }
                return self.lift();
            }
        }

        Ok(false)
    }

    fn find_child_under_ancestor(
        &self,
        ancestor_id: NodeId,
        descendant_id: NodeId,
        descendant_ancestors: &[NodeId],
    ) -> Result<NodeId> {
        let descendant = self.node(descendant_id).context("Descendant not found")?;
        if descendant.parent_id() == Some(ancestor_id) {
            return Ok(descendant_id);
        }

        let child_id = descendant_ancestors
            .iter()
            .find(|&id| self.node(*id).unwrap().parent_id() == Some(ancestor_id))
            .copied()
            .context("Child not found in ancestors")?;

        Ok(child_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_node_after() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                }
            }

            selection { (t, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                }
            }

            selection { (t, 5) -> (t, 6, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_middle_of_text() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                }
            }

            selection { (t, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hel" }
                    hard_break()
                    text { "lo" }
                }
            }

            selection { (t, 3) -> (t, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_start_of_text() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                }
            }

            selection { (t, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    hard_break()
                    text { "hello" }
                }
            }

            selection { (t, 0) -> (t, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_block_node_in_paragraph_start() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
            }

            selection { (n, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::Image(ImageNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                image()
                paragraph {
                    text { "hello" }
                }
            }

            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_block_node_in_paragraph_middle() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
            }

            selection { (n, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::Image(ImageNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hel" }
                }
                image()
                paragraph {
                    text { "lo" }
                }
            }

            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_block_node_in_paragraph_end() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
            }

            selection { (n, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::Image(ImageNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hello" }
                }
                image()
                paragraph {}
            }

            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_with_multiple_children() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    text { "world" }
                }
            }

            selection { (t, 6) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    hard_break()
                    text { "world" }
                }
            }

            selection { (t, 6) -> (t, 7, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_text_start_between_hard_breaks() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    hard_break()
                    text { "x" }
                    hard_break()
                }
            }

            selection { (t, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    hard_break()
                    hard_break()
                    text { "x" }
                    hard_break()
                }
            }

            selection { (t, 1) -> (t, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_multiple_nodes_consecutively() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                }
            }

            selection { (t, 5) }
        };

        let result1 = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let actual = transact!(result1, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    hard_break()
                }
            }

            selection { (t, 6) -> (t, 7, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_splits_long_text() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "abcdefghijklmnopqrstuvwxyz" }
                }
            }

            selection { (t, 13) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "abcdefghijklm" }
                    hard_break()
                    text { "nopqrstuvwxyz" }
                }
            }

            selection { (t, 13) -> (t, 14, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_hard_break_boundary() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    hard_break()
                }
            }

            selection { (t, 6) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    hard_break()
                    hard_break()
                }
            }

            selection { (t, 6) -> (t, 7, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_text_end_in_multiple_nodes() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    hard_break()
                    text { "a" }
                    hard_break()
                    text { "b" }
                }
            }

            selection { (t, 2) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    hard_break()
                    text { "a" }
                    hard_break()
                    hard_break()
                    text { "b" }
                }
            }

            selection { (t, 2) -> (t, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_text_boundary() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    hard_break()
                    text { "hello" }
                }
            }

            selection { (t, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    hard_break()
                    hard_break()
                    text { "hello" }
                }
            }

            selection { (t, 1) -> (t, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_after_text_in_sequence() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "a" }
                    hard_break()
                    text { "b" }
                    hard_break()
                    text { "c" }
                }
            }

            selection { (t, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "a" }
                    hard_break()
                    text { "b" }
                    hard_break()
                    hard_break()
                    text { "c" }
                }
            }

            selection { (t, 3) -> (t, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_mixed_inline_nodes() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    text { "world" }
                }
            }

            selection { (t, 7) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "hello" }
                    hard_break()
                    text { "w" }
                    hard_break()
                    text { "orld" }
                }
            }

            selection { (t, 7) -> (t, 8, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_node_at_last_offset() {
        let mut t = id!();

        let initial = state! {
            doc {
                @t paragraph {
                    text { "ab" }
                    hard_break()
                }
            }

            selection { (t, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_node(Node::HardBreak(HardBreakNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                @t paragraph {
                    text { "ab" }
                    hard_break()
                    hard_break()
                }
            }

            selection { (t, 3) -> (t, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn wrap_in_blockquote() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .wrap_in(Node::Blockquote(BlockquoteNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "hello" } }
                    @p2 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn wrap_in_blockquote_2() {
        let mut p2 = id!();

        let initial = state! {
            doc {
                paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .wrap_in(Node::Blockquote(BlockquoteNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                paragraph { text { "hello" } }
                blockquote {
                    @p2 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn wrap_in_blockquote_3() {
        let mut p2 = id!();

        let initial = state! {
            doc {
                paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p2, 1) -> (p2, 2)}
        };

        let actual = transact!(initial, |tr| tr
            .wrap_in(Node::Blockquote(BlockquoteNode::default()))
            .unwrap());

        let expected = state! {
            doc {
                paragraph { text { "hello" } }
                blockquote {
                    @p2 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p2, 1) -> (p2, 2)}
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn wrap_in_blockquote_4() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
            }
            selection { (p1, 0) -> (p2, 5)}
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
            tr.wrap_in(Node::Blockquote(BlockquoteNode::default()))
                .unwrap()
        });

        let expected = state! {
            doc {
                blockquote {
                    @p1 paragraph { }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_from_blockquote() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "hello" } }
                    @p2 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let actual = transact!(initial, |tr| tr.lift().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph { text { "world" } }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_from_blockquote_2() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                blockquote {
                    @p1 paragraph { text { "hello" } }
                    paragraph { text { "world" } }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.lift().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                blockquote {
                    paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_from_blockquote_3() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                blockquote {
                    paragraph { text { "hello" } }
                    @p1 paragraph { text { "world" } }
                    paragraph { text { "abc" } }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.lift().unwrap());

        let expected = state! {
            doc {
                blockquote {
                    paragraph { text { "hello" } }
                }
                @p1 paragraph { text { "world" } }
                blockquote {
                    paragraph { text { "abc" } }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_from_blockquote_4() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                blockquote {
                    paragraph { text { "hello" } }
                    @p1 paragraph { text { "world" } }
                }
                paragraph {}
            }
            selection { (p1, 1) -> (p1, 2) }
        };

        let actual = transact!(initial, |tr| tr.lift().unwrap());

        let expected = state! {
            doc {
                blockquote {
                    paragraph {
                        text { "hello" }
                    }
                }
                @p1 paragraph { text { "world" } }
                paragraph { }
            }
            selection { (p1, 1) -> (p1, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_on_empty_paragraph() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                blockquote {
                    paragraph { text { "hello" } }
                    @p1 paragraph { }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.lift_on_empty_paragraph().unwrap());

        let expected = state! {
            doc {
                blockquote {
                    paragraph {
                        text { "hello" }
                    }
                }
                @p1 paragraph { }
                paragraph { }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn lift_in_isolating_node_does_nothing() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        paragraph { text { "world" } }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial.clone(), |tr| tr.lift().unwrap());

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn lift_on_empty_paragraph_in_isolating_node_does_nothing() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        paragraph { text { "hello" } }
                        @p1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial.clone(), |tr| tr.lift_on_empty_paragraph().unwrap());

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn wrap_in_table_inside_table_is_rejected() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            @p1 paragraph { text { "cell" } }
                            paragraph { text { "cell2" } }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial.clone(), |tr| {
            let result = tr
                .wrap_in_ancestor(Node::Table(TableNode::default()))
                .unwrap();
            assert!(!result, "Wrapping in table inside table should be rejected");
        });

        assert_state_eq!(actual, initial);
    }
}

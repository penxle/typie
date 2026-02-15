use crate::model::*;
use crate::runtime::Effect;
use crate::schema::ContentExpr;
use crate::state::collect_top_level_blocks_in_range;
use crate::state::position_helpers::{calculate_offset_before_child, find_child_at_offset};
use crate::state::*;
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result};

pub(crate) fn clone_block_type(node: &Node) -> Option<Node> {
    match node {
        Node::Paragraph(p) => Some(Node::Paragraph(ParagraphNode {
            align: p.align,
            line_height: p.line_height,
        })),
        _ => None,
    }
}

fn can_join_content(node_a: &NodeRef, node_b: &NodeRef) -> bool {
    node_a.is_block() && node_b.is_block() && node_a.spec().content == node_b.spec().content
}

fn find_last_joinable_block(node: &NodeRef, target_content: &ContentExpr) -> Option<NodeId> {
    if node.is_block() && &node.spec().content == target_content {
        return Some(node.node_id());
    }

    if node.spec().isolating {
        return None;
    }

    if let Some(last_child) = node.last_child() {
        return find_last_joinable_block(&last_child, target_content);
    }

    None
}

fn find_first_joinable_block(node: &NodeRef, target_content: &ContentExpr) -> Option<NodeId> {
    if node.is_block() && &node.spec().content == target_content {
        return Some(node.node_id());
    }

    if node.spec().isolating {
        return None;
    }

    if let Some(first_child) = node.first_child() {
        return find_first_joinable_block(&first_child, target_content);
    }

    None
}

impl Transaction {
    fn try_select_adjacent_block(
        &mut self,
        current_block_id: NodeId,
        sibling_id: NodeId,
        select_next: bool,
    ) -> Result<bool> {
        let sibling = self.node(sibling_id).context("Sibling not found")?;
        if !sibling.spec().selectable {
            return Ok(false);
        }

        let current_block = self
            .node(current_block_id)
            .context("Current block not found")?;
        let parent = current_block.parent().context("Parent not found")?;
        let parent_id = parent.node_id();

        if current_block.children().count() == 0 {
            let index = current_block
                .index()
                .context("Current block has no index")?;
            self.node_mut(current_block_id)
                .context("Current block not found")?
                .as_mut()
                .delete()?;
            self.push_effect(Effect::StructureChanged);

            let selection_index = if select_next {
                index
            } else {
                index.checked_sub(1).context("Index underflow")?
            };

            self.set_selection(Selection::new(
                Position::new(parent_id, selection_index, Affinity::Downstream),
                Position::new(parent_id, selection_index + 1, Affinity::Upstream),
            ));
        } else {
            let sibling_index = sibling.index().context("Sibling has no index")?;
            self.set_selection(Selection::new(
                Position::new(parent_id, sibling_index, Affinity::Downstream),
                Position::new(parent_id, sibling_index + 1, Affinity::Upstream),
            ));
        }

        Ok(true)
    }

    pub fn split_block_at(&mut self, pos: Position) -> Result<Option<(NodeId, Position)>> {
        let paragraph = self.node(pos.node_id).context("Paragraph not found")?;

        if let Some((child_id, local_offset)) = find_child_at_offset(&paragraph, pos.offset) {
            let this = self.node(child_id).context("Child not found")?;
            let this_id = this.node_id();
            let next_id = this.next_sibling().map(|n| n.node_id());
            let parent = this.parent().context("Parent not found")?;
            let parent_last_child_id = parent.last_child().map(|n| n.node_id());

            match this.node() {
                Node::Text(text_node) => {
                    let grandparent = parent.parent().context("Grandparent not found")?;
                    let grandparent_id = grandparent.node_id();

                    let text_len = text_node.text.char_len();
                    let tail = text_node.text.slice(local_offset, text_len);

                    let parent_index = parent.index().context("Parent has no index")?;
                    let grandparent = self
                        .node_mut(grandparent_id)
                        .context("Grandparent not found")?;
                    let new_block_id = grandparent.as_mut().insert_child(
                        parent_index + 1,
                        clone_block_type(&parent.node()).context("Cannot clone block type")?,
                    )?;

                    if local_offset == 0 {
                        self.node_mut(this_id)
                            .context("Node not found")?
                            .as_mut()
                            .delete()?;
                    } else {
                        self.node_mut(this_id)
                            .context("Node not found")?
                            .as_mut()
                            .update(|node| {
                                if let Node::Text(n) = node {
                                    n.text.truncate(local_offset);
                                }
                            })?;
                    }

                    if let Some(next) = next_id {
                        if let Some(last) = parent_last_child_id {
                            self.move_node_range(next, last, Some(new_block_id), None, None)?;
                        }
                    }

                    self.push_effect(Effect::NodeChanged {
                        node_id: pos.node_id,
                    });
                    self.push_effect(Effect::NodeChanged {
                        node_id: new_block_id,
                    });
                    self.push_effect(Effect::StructureChanged);

                    if tail.is_empty() {
                        let new_pos = if let Some(next) = next_id {
                            let offset_before = calculate_offset_before_child(
                                &self.node(new_block_id).context("New block not found")?,
                                next,
                            );
                            Position::new(new_block_id, offset_before, Affinity::Downstream)
                        } else {
                            Position::new(new_block_id, 0, Affinity::default())
                        };
                        return Ok(Some((new_block_id, new_pos)));
                    } else {
                        let new_block =
                            self.node_mut(new_block_id).context("New block not found")?;
                        new_block.as_mut().insert_child(
                            0,
                            Node::Text(TextNode {
                                text: tail.clone(),
                                ..Default::default()
                            }),
                        )?;
                        return Ok(Some((
                            new_block_id,
                            Position::new(new_block_id, 0, Affinity::Downstream),
                        )));
                    }
                }
                Node::HardBreak(_) | Node::PageBreak(_) => {
                    let grandparent = parent.parent().context("Grandparent not found")?;
                    let grandparent_id = grandparent.node_id();

                    let parent_index = parent.index().context("Parent has no index")?;
                    let grandparent = self
                        .node_mut(grandparent_id)
                        .context("Grandparent not found")?;
                    let new_block_id = grandparent.as_mut().insert_child(
                        parent_index + 1,
                        clone_block_type(&parent.node()).context("Cannot clone block type")?,
                    )?;

                    let first_child_to_move = if local_offset == 0 {
                        Some(this_id)
                    } else {
                        next_id
                    };

                    if let Some(first_child_to_move) = first_child_to_move {
                        if let Some(last) = parent_last_child_id {
                            self.move_node_range(
                                first_child_to_move,
                                last,
                                Some(new_block_id),
                                None,
                                None,
                            )?;
                        }
                    }

                    self.push_effect(Effect::NodeChanged {
                        node_id: pos.node_id,
                    });
                    self.push_effect(Effect::NodeChanged {
                        node_id: new_block_id,
                    });
                    self.push_effect(Effect::StructureChanged);

                    let new_block_node = self.node(new_block_id).context("New block not found")?;
                    if let Some(first_child) = new_block_node.first_child() {
                        let first_id = first_child.node_id();
                        let offset_before =
                            calculate_offset_before_child(&new_block_node, first_id);
                        return Ok(Some((
                            new_block_id,
                            Position::new(new_block_id, offset_before, Affinity::default()),
                        )));
                    } else {
                        return Ok(Some((
                            new_block_id,
                            Position::new(new_block_id, 0, Affinity::default()),
                        )));
                    }
                }
                _ => {
                    return Ok(None);
                }
            }
        } else {
            let parent = paragraph.parent().context("Parent not found")?;
            let parent_id = parent.node_id();
            let paragraph_index = paragraph.index().context("Paragraph has no index")?;
            let parent = self.node_mut(parent_id).context("Parent not found")?;
            let new_block_id = parent.as_mut().insert_child(
                paragraph_index + 1,
                clone_block_type(&paragraph.node()).context("Cannot clone block type")?,
            )?;

            self.push_effect(Effect::NodeChanged {
                node_id: pos.node_id,
            });
            self.push_effect(Effect::NodeChanged {
                node_id: new_block_id,
            });
            self.push_effect(Effect::StructureChanged);

            return Ok(Some((
                new_block_id,
                Position::new(new_block_id, 0, Affinity::default()),
            )));
        }
    }

    pub fn split_paragraph(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let styles = self.state.pending_styles.clone();

        if let Some((_, new_pos)) = self.split_block_at(selection.head)? {
            self.set_selection(Selection::collapsed(new_pos));
            self.state.pending_styles = styles.clone();
            let _ = self.set_cascade_attrs(new_pos.node_id, &Attr::from_styles(&styles));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn insert_paragraph_on_nontextblock_selection(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            return Ok(false);
        }

        let (from, to) = selection.as_sorted(self.doc())?;

        let blocks = collect_top_level_blocks_in_range(self.doc(), from.clone(), to.clone())?;
        let Some(&block_id) = blocks.first() else {
            return Ok(false);
        };
        if blocks.len() != 1 {
            return Ok(false);
        }

        let block = self.node(block_id).context("Block not found")?;
        if block.spec().is_textblock(self.doc().schema()) {
            return Ok(false);
        }

        let parent = block.parent().context("Parent not found")?;
        let parent_id = parent.node_id();

        let block_offset = calculate_offset_before_child(&parent, block_id);
        let is_exact_block_selection = from.node_id == parent_id
            && to.node_id == parent_id
            && from.offset == block_offset
            && to.offset == block_offset + 1;

        if !is_exact_block_selection {
            return Ok(false);
        }

        let insert_before = matches!(parent.node(), Node::Root(_))
            && parent.first_child().map(|child| child.node_id()) == Some(block_id);
        let prev = if insert_before {
            block.prev_sibling().map(|n| n.node_id())
        } else {
            Some(block_id)
        };

        let attrs = self.resolve_attr_cascade(parent_id);
        let line_height = Attr::extract_paragraph_attr(&attrs)
            .map(|p| p.line_height)
            .unwrap_or(1.6);

        let parent = self.node_mut(parent_id).context("Parent not found")?;
        let insert_index = if let Some(prev_id) = prev {
            let prev_node = self.node(prev_id).context("Previous node not found")?;
            prev_node.index().context("Previous node has no index")? + 1
        } else {
            let block = self.node(block_id).context("Block not found")?;
            block.index().context("Block has no index")?
        };
        let new_block_id = parent.as_mut().insert_child(
            insert_index,
            Node::Paragraph(ParagraphNode {
                line_height,
                ..Default::default()
            }),
        )?;
        self.set_selection(Selection::collapsed(Position::new(
            new_block_id,
            0,
            Affinity::default(),
        )));
        self.push_effect(Effect::NodeChanged {
            node_id: new_block_id,
        });
        self.push_effect(Effect::StructureChanged);
        Ok(true)
    }

    pub fn join_backward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;
        let current_block_id = block.node_id();

        let at_start = selection.head.offset == 0;

        if !at_start {
            return Ok(false);
        }

        let prev_sibling_id = self
            .node(current_block_id)
            .context("Current block not found")?
            .prev_sibling()
            .map(|n| n.node_id());

        let Some(prev_sibling_id) = prev_sibling_id else {
            return Ok(false);
        };

        if self.try_select_adjacent_block(current_block_id, prev_sibling_id, false)? {
            self.push_effect(Effect::NodeChanged {
                node_id: prev_sibling_id,
            });
            return Ok(true);
        }

        let current_block = self
            .node(current_block_id)
            .context("Current block not found")?;
        let prev_sibling = self
            .node(prev_sibling_id)
            .context("Previous sibling not found")?;
        let current_content = current_block.spec().content.clone();

        let target_block_id = if can_join_content(&current_block, &prev_sibling) {
            prev_sibling.node_id()
        } else if let Some(joinable_id) = find_last_joinable_block(&prev_sibling, &current_content)
        {
            joinable_id
        } else {
            return Ok(false);
        };

        let prev_end_offset = {
            let target_block = self
                .node(target_block_id)
                .context("Target block not found")?;
            let mut offset = 0;
            for child in target_block.children() {
                match child.node() {
                    Node::Text(text) => {
                        offset += text.text.char_len();
                    }
                    Node::HardBreak(_) => {
                        offset += 1;
                    }
                    _ => {}
                }
            }
            offset
        };

        let pre_styles = self.state.pending_styles.clone();

        let from = Position::new(target_block_id, prev_end_offset, Affinity::Downstream);
        let to = Position::new(current_block_id, 0, Affinity::Downstream);

        self.delete_range(from, to)?;

        if !self.cursor_has_text_segment(target_block_id, prev_end_offset) {
            let _ = self.set_cascade_attrs(target_block_id, &Attr::from_styles(&pre_styles));
        }

        self.set_selection(Selection::collapsed(Position::new(
            target_block_id,
            prev_end_offset,
            Affinity::Downstream,
        )));

        self.push_effect(Effect::NodeChanged {
            node_id: target_block_id,
        });
        if target_block_id != prev_sibling_id {
            self.push_effect(Effect::NodeChanged {
                node_id: prev_sibling_id,
            });
        }
        Ok(true)
    }

    pub fn join_forward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let block = self
            .node(selection.head.node_id)
            .context("Block not found")?;
        let current_block_id = block.node_id();

        let at_end = if let Some((child_id, local_offset)) =
            find_child_at_offset(&block, selection.head.offset)
        {
            let child = self.node(child_id).context("Child not found")?;
            match child.node() {
                Node::Text(text) => {
                    let text_len = text.text.char_len();
                    local_offset == text_len
                }
                _ => return Ok(false),
            }
        } else {
            true
        };

        if !at_end {
            return Ok(false);
        }

        let next_sibling_id = self
            .node(current_block_id)
            .context("Current block not found")?
            .next_sibling()
            .map(|n| n.node_id());

        let Some(next_sibling_id) = next_sibling_id else {
            return Ok(false);
        };

        if self.try_select_adjacent_block(current_block_id, next_sibling_id, true)? {
            self.push_effect(Effect::NodeChanged {
                node_id: next_sibling_id,
            });
            return Ok(true);
        }

        let current_block = self
            .node(current_block_id)
            .context("Current block not found")?;
        let next_sibling = self
            .node(next_sibling_id)
            .context("Next sibling not found")?;
        let current_content = current_block.spec().content.clone();

        let target_block_id = if can_join_content(&current_block, &next_sibling) {
            next_sibling.node_id()
        } else if let Some(joinable_id) = find_first_joinable_block(&next_sibling, &current_content)
        {
            joinable_id
        } else {
            return Ok(false);
        };

        let current_end_offset = {
            let current_block = self
                .node(current_block_id)
                .context("Current block not found")?;
            let mut offset = 0;
            for child in current_block.children() {
                match child.node() {
                    Node::Text(text) => {
                        offset += text.text.char_len();
                    }
                    Node::HardBreak(_) => {
                        offset += 1;
                    }
                    _ => {}
                }
            }
            offset
        };

        let pre_styles = self.state.pending_styles.clone();

        let from = Position::new(current_block_id, current_end_offset, Affinity::Downstream);
        let to = Position::new(target_block_id, 0, Affinity::Downstream);

        self.delete_range(from, to)?;

        if !self.cursor_has_text_segment(current_block_id, current_end_offset) {
            let _ = self.set_cascade_attrs(current_block_id, &Attr::from_styles(&pre_styles));
        }

        self.set_selection(Selection::collapsed(Position::new(
            current_block_id,
            current_end_offset,
            Affinity::Downstream,
        )));

        self.push_effect(Effect::NodeChanged {
            node_id: current_block_id,
        });
        self.push_effect(Effect::NodeChanged {
            node_id: next_sibling_id,
        });
        if target_block_id != next_sibling_id {
            self.push_effect(Effect::NodeChanged {
                node_id: target_block_id,
            });
        }
        Ok(true)
    }

    pub fn set_text_align(&mut self, align: TextAlign) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let blocks = collect_blocks_in_range(self.doc(), from, to)?;

        for block_id in blocks {
            let block = self.node_mut(block_id).context("Block not found")?;
            if let Node::Paragraph(_) = block.node() {
                block.as_mut().update(|node| {
                    if let Node::Paragraph(p) = node {
                        p.align = align;
                    }
                })?;
                self.push_effect(Effect::NodeChanged { node_id: block_id });
            }
        }

        Ok(true)
    }

    pub fn set_line_height(&mut self, line_height: f32) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let blocks = collect_blocks_in_range(self.doc(), from, to)?;

        for block_id in blocks {
            let block = self.node_mut(block_id).context("Block not found")?;
            if let Node::Paragraph(_) = block.node() {
                block.as_mut().update(|node| {
                    if let Node::Paragraph(p) = node {
                        p.line_height = line_height;
                    }
                })?;
                self.push_effect(Effect::NodeChanged { node_id: block_id });
            }
        }

        Ok(true)
    }

    pub fn reset_fully_selected_paragraphs(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        let mut paragraph_ids = collect_nodes_in_selection(self.doc(), &selection, |node| {
            matches!(node, Node::Paragraph(_))
        })?;

        let (_, to) = selection.as_sorted(self.doc())?;
        if let Some(node) = self.node(to.node_id) {
            if matches!(node.node(), Node::Paragraph(_)) && !paragraph_ids.contains(&to.node_id) {
                paragraph_ids.push(to.node_id);
            }
        }

        let root_attrs = self.resolve_attr_cascade(NodeId::ROOT);
        let default_styles = Attr::extract_styles(&root_attrs);
        let default_para_attr = Attr::extract_paragraph_attr(&root_attrs);

        let mut changed = false;
        for para_id in paragraph_ids {
            if is_node_fully_selected(self.doc(), &selection, para_id)? {
                let default_line_height = default_para_attr
                    .as_ref()
                    .map(|p| p.line_height)
                    .unwrap_or(1.6);

                let para_node = self
                    .node_mut(para_id)
                    .context("reset_fully_selected_paragraphs: Paragraph not found")?;
                let mut para_changed = false;
                para_node.as_mut().update(|n| {
                    if let Node::Paragraph(p) = n {
                        if p.align != TextAlign::default() {
                            p.align = TextAlign::default();
                            para_changed = true;
                        }
                        if (p.line_height - default_line_height).abs() > f32::EPSILON {
                            p.line_height = default_line_height;
                            para_changed = true;
                        }
                    }
                })?;
                if para_changed {
                    self.push_effect(Effect::NodeChanged { node_id: para_id });
                    changed = true;
                }

                if self.node(para_id).and_then(|n| n.cascade_attrs()).is_some() {
                    self.set_cascade_attrs(para_id, &Attr::from_styles(&default_styles))?;
                    changed = true;
                }
            }
        }

        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_paragraph_not_collapsed() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 2) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.split_paragraph().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_backward_not_collapsed() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) -> (p, 2) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_backward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_backward_not_at_start() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 1) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_backward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_backward_no_prev_sibling() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_backward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_backward_slot_no_prev_sibling() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_backward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_forward_not_collapsed() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 0) -> (p, 2) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_forward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_forward_not_at_end() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
                paragraph { text { "world" } }
            }
            selection { (p, 2) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_forward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_forward_no_next_sibling() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph { text { "hello" } }
            }
            selection { (p, 5) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_forward().unwrap();
        assert!(!result);
    }

    #[test]
    fn join_forward_slot_no_next_sibling() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };
        let mut tr = Transaction::new(&state);
        let result = tr.join_forward().unwrap();
        assert!(!result);
    }

    #[test]
    fn split_paragraph_text_start() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                }
            }

            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {}
                @p paragraph {
                    text { "helloworld" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_text_middle() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                }
            }

            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hello" }
                }
                @p paragraph {
                    text { "world" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_text_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "helloworld" }
                }
            }

            selection { (p1, 10) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "helloworld" }
                }
                @p2 paragraph {}
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_text_multiple_1() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }

            selection { (p, 4) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hell" }
                }
                @p paragraph {
                    text { "o" }
                    text { "world" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_text_multiple_2() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }

            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hello" }
                }
                @p paragraph {
                    text { "world" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_text_multiple_3() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }

            selection { (p, 7) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "hello" }
                    text { "wo" }
                }
                @p paragraph {
                    text { "rld" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_empty_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {}
            }

            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {}
                @p2 paragraph {}
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_middle_node_multiple_children() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "abc" }
                    text { "def" }
                    text { "ghi" }
                }
            }

            selection { (p1, 4) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "abc" }
                    text { "d" }
                }
                @p2 paragraph {
                    text { "ef" }
                    text { "ghi" }
                }
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_paragraph_on_block_before_first_root_child() {
        let mut inserted = id!();

        let initial = state! {
            doc {
                image()
                paragraph {
                    text { "next" }
                }
            }

            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_paragraph_on_nontextblock_selection()
            .unwrap());

        let expected = state! {
            doc {
                @inserted paragraph {}
                image()
                paragraph {
                    text { "next" }
                }
            }

            selection { (inserted, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_paragraph_on_block_after_non_first_child() {
        let mut inserted = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "first" }
                }
                image()
                paragraph {}
            }

            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_paragraph_on_nontextblock_selection()
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "first" }
                }
                image()
                @inserted paragraph {}
                paragraph {}
            }

            selection { (inserted, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_last_node_tail_empty() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "abc" }
                    text { "def" }
                }
            }

            selection { (p1, 6) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "abc" }
                    text { "def" }
                }
                @p2 paragraph {}
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_after_hard_break() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "abc" }
                    hard_break {}
                    hard_break {}
                }
            }

            selection { (p1, 4) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "abc" }
                    hard_break {}
                }
                @p2 paragraph {
                    hard_break {}
                }
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_before_hard_break() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    hard_break {}
                    hard_break {}
                    text { "abc" }
                }
            }

            selection { (p1, 1) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    hard_break {}
                }
                @p2 paragraph {
                    hard_break {}
                    text { "abc" }
                }
            }

            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_with_style() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "asdf" }
                }
            }
            selection { (p, 2) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text(styles: [italic()]) { "as" }
                }
                @p paragraph {
                    text(styles: [italic()]) { "df" }
                }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_with_style_2() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "asdf" => [italic()], "gh" }
                }
            }
            selection { (p, 2) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text(styles: [italic()]) { "as" }
                }
                @p paragraph {
                    text { "df" => [italic()], "gh" }
                }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_simple() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "world" }
                }
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "helloworld" }
                }
            }
            selection { (p1, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_empty_next_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {}
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
            }
            selection { (p1, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_simple() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
                paragraph {
                    text { "world" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_empty_prev_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
                paragraph {
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_empty_paragraphs() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
                paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_empty_paragraph_before_selectable_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { }
                image()
                paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 0, Affinity::Downstream) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_before_selectable_node_with_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "hello" } }
                image()
                paragraph {}
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph { text { "hello" } }
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_empty_paragraph_after_selectable_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                image()
                @p paragraph { }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 0, Affinity::Downstream) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_after_selectable_node_with_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                image()
                @p paragraph { text { "world" } }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                image()
                @p paragraph { text { "world" } }
            }
            selection { (NodeId::ROOT, 0, Affinity::Downstream) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_empty_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_style_diff() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text(styles: [italic()]) { "asdf" } }
                @p2 paragraph { text { "qwer" } }
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { text { "asdf" => [italic()], "qwer" } }
            }
            selection { (p1, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn set_text_align() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.set_text_align(TextAlign::Center).unwrap());

        let expected = state! {
            doc {
                @p paragraph(align: TextAlign::Center,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_at_page_break() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                    page_break {}
                }
            }
            selection { (p1, 5) }
        };

        let actual = transact!(initial, |tr| tr.split_paragraph().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    page_break {}
                }
                paragraph {}
            }
            selection { (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_backward_does_not_enter_isolating_node() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        paragraph { text { "inside" } }
                    }
                }
                @p1 paragraph { text { "outside" } }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial.clone(), |tr| tr.join_backward().unwrap());

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn join_forward_does_not_enter_isolating_node() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "outside" } }
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        paragraph { text { "inside" } }
                    }
                }
            }
            selection { (p1, 7) }
        };

        let actual = transact!(initial.clone(), |tr| tr.join_forward().unwrap());

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn join_backward_works_inside_isolating_node() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        @p2 paragraph { text { "world" } }
                    }
                }
            }
            selection { (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "helloworld" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn join_forward_works_inside_isolating_node() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "hello" } }
                        @p2 paragraph { text { "world" } }
                    }
                }
            }
            selection { (p1, 5) }
        };

        let actual = transact!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "helloworld" } }
                    }
                }
                paragraph {}
            }
            selection { (p1, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn split_paragraph_styles_preserved_through_complete_preedit() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [font_weight(700)]) { "안녕하세요" }
                }
            }
            selection { (p, 5) }
        };

        let after_split = transact!(initial, |tr| tr.split_paragraph().unwrap());

        assert!(
            after_split
                .pending_styles
                .iter()
                .any(|m| matches!(m, Style::FontWeight(_)))
        );

        let after_preedit = transact!(after_split, |tr| {
            tr.set_preedit("ㅎ".to_string()).unwrap()
        });

        let after_complete = transact!(after_preedit, |tr| tr.complete_preedit().unwrap());

        assert!(
            after_complete
                .pending_styles
                .iter()
                .any(|m| matches!(m, Style::FontWeight(_)))
        );

        let after_insert = transact!(after_complete, |tr| tr.insert_text("ㅎ").unwrap());

        let root = after_insert.doc.node(NodeId::ROOT).unwrap();
        let second_para = root.children().nth(1).unwrap();
        let text_node = second_para.children().next().unwrap();

        if let Node::Text(t) = text_node.node() {
            let segments = t.text.get_segments();
            assert_eq!(segments.len(), 1);
            let seg = &segments[0];
            assert_eq!(seg.text, "ㅎ");
            assert!(
                seg.styles.iter().any(|m| matches!(m, Style::FontWeight(_))),
                "Text on new line should have FontWeight style"
            );
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn join_backward_preserves_pending_styles_when_empty_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p2, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.state
            .pending_styles
            .push(Style::FontWeight(FontWeightStyle { weight: 700 }));
        tr.join_backward().unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|m| matches!(m, Style::FontWeight(fw) if fw.weight == 700)),
            "join_backward should preserve pending_styles when merging empty paragraphs, got: {:?}",
            view.pending_styles
        );
    }

    #[test]
    fn join_forward_preserves_pending_styles_when_empty_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p1, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.state
            .pending_styles
            .push(Style::FontWeight(FontWeightStyle { weight: 700 }));
        tr.join_forward().unwrap();
        let (view, _) = tr.commit().unwrap();

        assert!(
            view.pending_styles
                .iter()
                .any(|m| matches!(m, Style::FontWeight(fw) if fw.weight == 700)),
            "join_forward should preserve pending_styles when merging empty paragraphs, got: {:?}",
            view.pending_styles
        );
    }

    #[test]
    fn insert_paragraph_on_nontextblock_uses_cascade_line_height() {
        let initial = state! {
            doc {
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .insert_paragraph_on_nontextblock_selection()
            .unwrap());

        let root = actual.doc.node(NodeId::ROOT).unwrap();
        let first_child = root.first_child().unwrap();
        if let Node::Paragraph(para) = first_child.node() {
            assert!(
                (para.line_height - 1.6).abs() < f32::EPSILON,
                "Inserted paragraph should inherit line_height from cascade, got: {}",
                para.line_height
            );
        } else {
            panic!("First child should be the inserted paragraph");
        }
    }
}

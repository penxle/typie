use crate::model::*;
use crate::runtime::Effect;
use crate::state::position_helpers::{calculate_offset_before_child, find_child_at_offset};
use crate::state::selection_helpers::{StructureSelectionInfo, compute_structure_selection};
use crate::state::{Position, Selection, block_content_len};
use crate::transaction::Transaction;
use crate::types::Affinity;
use crate::utils::{
    detect_writing_systems, find_next_grapheme_boundary, find_prev_grapheme_boundary,
    resolve_affinity_boundary,
};
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub enum DeleteResult {
    None,
    Local {
        node_id: NodeId,
        start: usize,
        len: usize,
    },
    Merged {
        from: NodeId,
        into: NodeId,
        merge_offset: usize,
        deleted_prefix: usize,
    },
}

impl DeleteResult {
    pub fn deleted(&self) -> bool {
        !matches!(self, DeleteResult::None)
    }

    pub fn remap_position(&self, pos: Position) -> Position {
        match self {
            DeleteResult::None => pos,
            DeleteResult::Local {
                node_id,
                start,
                len,
            } => {
                if pos.node_id == *node_id && pos.offset > *start {
                    let new_offset = pos.offset.saturating_sub(*len);
                    Position::new(pos.node_id, new_offset, pos.affinity)
                } else {
                    pos
                }
            }
            DeleteResult::Merged {
                from,
                into,
                merge_offset,
                deleted_prefix,
            } => {
                if pos.node_id == *from {
                    let adjusted_offset = pos.offset.saturating_sub(*deleted_prefix);
                    Position::new(*into, merge_offset + adjusted_offset, pos.affinity)
                } else {
                    pos
                }
            }
        }
    }
}

fn apply_pending_marks_to_text(text: &Text, range: std::ops::Range<usize>, marks: &[Mark]) {
    for mark_type in MarkType::all() {
        let _ = text.unmark(range.clone(), mark_type);
    }

    for mark in marks {
        let _ = text.mark(range.clone(), mark);
    }
}

fn resolve_affinity_after_edit(
    tr: &Transaction,
    block_id: NodeId,
    offset: usize,
    default_affinity: Affinity,
) -> Affinity {
    let is_hard_break = |node: &NodeRef| matches!(node.node(), Node::HardBreak(_));

    let block = match tr.node(block_id) {
        Some(b) => b,
        None => return default_affinity,
    };

    let Some((child_id, local_offset)) = find_child_at_offset(&block, offset) else {
        return default_affinity;
    };

    let child = match tr.node(child_id) {
        Some(c) => c,
        None => return default_affinity,
    };

    let prev_is_hard_break = || {
        child
            .prev_sibling()
            .and_then(|n| tr.node(n.node_id()))
            .map(|n| is_hard_break(&n))
            .unwrap_or(false)
    };

    let next_is_hard_break = || {
        child
            .next_sibling()
            .and_then(|n| tr.node(n.node_id()))
            .map(|n| is_hard_break(&n))
            .unwrap_or(false)
    };

    let boundary = match child.node() {
        Node::Text(text) => match local_offset {
            0 => Some((prev_is_hard_break(), false)),
            len if len == text.text.char_len() => Some((false, next_is_hard_break())),
            _ => None,
        },
        _ => match local_offset {
            0 => Some((prev_is_hard_break(), is_hard_break(&child))),
            1 => Some((is_hard_break(&child), next_is_hard_break())),
            _ => None,
        },
    };

    if let Some((left_hard_break, right_hard_break)) = boundary {
        resolve_affinity_boundary(left_hard_break, right_hard_break, default_affinity)
    } else {
        default_affinity
    }
}

impl Transaction {
    pub fn surround_selection(&mut self, left: &str, right: &str) -> Result<bool> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            return Ok(false);
        }

        let (from, to) = selection.as_sorted(self.doc())?;

        let left_len = bytecount::num_chars(left.as_bytes());
        let right_len = bytecount::num_chars(right.as_bytes());

        self.set_selection(Selection::collapsed(to));
        self.insert_text(right)?;

        self.set_selection(Selection::collapsed(from));
        self.insert_text(left)?;

        let new_to = if from.node_id == to.node_id {
            Position::new(to.node_id, to.offset + left_len + right_len, to.affinity)
        } else {
            Position::new(to.node_id, to.offset + right_len, to.affinity)
        };

        self.set_selection(Selection::new(from, new_to));

        self.push_effect(Effect::NodeChanged {
            node_id: from.node_id,
        });
        if from.node_id != to.node_id {
            self.push_effect(Effect::NodeChanged {
                node_id: to.node_id,
            });
        }

        Ok(true)
    }

    pub fn insert_text(&mut self, s: &str) -> Result<bool> {
        if s.is_empty() {
            return Ok(false);
        }

        let writing_systems = detect_writing_systems(s);
        if !writing_systems.is_empty() {
            self.push_effect(Effect::WritingSystemsUsageChanged {
                systems: writing_systems,
            });
        }

        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let pending_marks = self.state.pending_marks.clone();

        let paragraph = self
            .node(selection.head.node_id)
            .context("Paragraph not found")?;

        if let Some((child_id, local_offset)) =
            find_child_at_offset(&paragraph, selection.head.offset)
        {
            let child = self.node_mut(child_id).context("Child not found")?;

            if let Node::Text(text_node) = child.node() {
                let char_count = bytecount::num_chars(s.as_bytes());

                child.as_mut().update(|node| {
                    if let Node::Text(t) = node {
                        t.text.insert(local_offset, s);
                    }
                })?;

                if let Some(marks) = &pending_marks {
                    apply_pending_marks_to_text(
                        &text_node.text,
                        local_offset..(local_offset + char_count),
                        marks,
                    );
                }

                self.set_selection(Selection::collapsed(Position::new(
                    selection.head.node_id,
                    selection.head.offset + char_count,
                    selection.head.affinity,
                )));

                self.state.pending_marks = None;
                self.push_effect(Effect::NodeChanged { node_id: child_id });
                self.push_effect(Effect::NodeChanged {
                    node_id: selection.head.node_id,
                });
                return Ok(true);
            }
        }

        let new_selection = Selection::collapsed(Position::new(
            selection.head.node_id,
            selection.head.offset + bytecount::num_chars(s.as_bytes()),
            selection.head.affinity,
        ));

        let text = Text::from(s);
        if let Some(marks) = &pending_marks {
            for mark in marks {
                let _ = text.mark(0..text.char_len(), mark);
            }
        }

        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(
            Node::Text(TextNode {
                text,
                ..Default::default()
            }),
            None,
        );
        let fragment = Fragment::builder().add((node_id, fragment_node)).build();

        self.replace_range(selection.head, selection.head, fragment)?;
        self.set_selection(new_selection);

        self.state.pending_marks = None;
        self.push_effect(Effect::NodeChanged {
            node_id: selection.head.node_id,
        });
        Ok(true)
    }

    pub fn insert_hard_break(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let new_selection = Selection::collapsed(Position::new(
            selection.head.node_id,
            selection.head.offset + 1,
            Affinity::Downstream,
        ));

        let node_id = NodeId::new();
        let fragment_node = FragmentNode::new(Node::HardBreak(HardBreakNode::default()), None);
        let fragment = Fragment::builder().add((node_id, fragment_node)).build();

        self.replace_range(selection.head, selection.head, fragment)?;
        self.set_selection(new_selection);

        self.push_effect(Effect::NodeChanged {
            node_id: selection.head.node_id,
        });
        Ok(true)
    }

    pub fn insert_page_break(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let original_para_id = selection.head.node_id;
        let original_para = self
            .node(original_para_id)
            .context("insert_page_break: Original paragraph not found")?;

        let end_offset = block_content_len(&original_para);
        let is_at_end = selection.head.offset == end_offset;

        if !is_at_end {
            self.split_paragraph()?;
        }

        let selection = self.selection().clone();

        let target_para_id = if is_at_end {
            original_para_id
        } else {
            let current_para = self
                .node(selection.head.node_id)
                .context("insert_page_break: Current paragraph not found")?;
            current_para
                .prev_sibling()
                .map(|n| n.node_id())
                .context("insert_page_break: Previous sibling not found after split")?
        };

        let target_para = self
            .node(target_para_id)
            .context("insert_page_break: Target paragraph not found")?;
        let target_end_offset = block_content_len(&target_para);

        self.set_selection(Selection::collapsed(Position::new(
            target_para_id,
            target_end_offset,
            Affinity::Downstream,
        )));

        self.insert_node(Node::PageBreak(PageBreakNode::default()))?;

        if is_at_end {
            self.move_to_next_block(target_para_id)?;
        } else {
            self.set_selection(selection);
        }

        Ok(true)
    }

    pub fn delete_text_backward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let head = selection.head;

        let paragraph = self.node(head.node_id).context("Paragraph not found")?;

        let Some((child_id, local_offset)) = find_child_at_offset(&paragraph, head.offset) else {
            return Ok(false);
        };

        let this = self.node(child_id).context("Child not found")?;

        let (from_global_offset, to_global_offset) = if local_offset == 0 {
            let Some(prev_id) = this.prev_sibling().map(|n| n.node_id()) else {
                return Ok(false);
            };

            let prev = self.node(prev_id).context("Previous node not found")?;
            let prev_offset = calculate_offset_before_child(&paragraph, prev_id);

            match prev.node() {
                Node::Text(prev_text) => {
                    let text_content = prev_text.text.to_string();
                    let prev_grapheme_offset =
                        find_prev_grapheme_boundary(&text_content, prev_text.text.char_len());

                    (prev_offset + prev_grapheme_offset, head.offset)
                }
                Node::HardBreak(_) => (prev_offset, head.offset),
                _ => {
                    return Ok(false);
                }
            }
        } else {
            if let Node::Text(text_node) = this.node() {
                let text_content = text_node.text.to_string();
                let prev_grapheme_offset = find_prev_grapheme_boundary(&text_content, local_offset);
                let global_offset_before_child =
                    calculate_offset_before_child(&paragraph, child_id);

                (
                    global_offset_before_child + prev_grapheme_offset,
                    head.offset,
                )
            } else {
                (head.offset - 1, head.offset)
            }
        };

        let from = Position::new(head.node_id, from_global_offset, head.affinity);
        let to = Position::new(head.node_id, to_global_offset, head.affinity);

        self.delete_range(from, to)?;

        let new_affinity =
            resolve_affinity_after_edit(self, head.node_id, from_global_offset, head.affinity);

        self.set_selection(Selection::collapsed(Position::new(
            head.node_id,
            from_global_offset,
            new_affinity,
        )));

        self.push_effect(Effect::NodeChanged {
            node_id: head.node_id,
        });
        Ok(true)
    }

    pub fn delete_text_forward(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let head = selection.head;

        let paragraph = self.node(head.node_id).context("Paragraph not found")?;

        let Some((child_id, local_offset)) = find_child_at_offset(&paragraph, head.offset) else {
            return Ok(false);
        };

        let this = self.node(child_id).context("Child not found")?;

        let text_len = match this.node() {
            Node::Text(text) => text.text.char_len(),
            _ => 1,
        };

        let (from_global_offset, to_global_offset) = if local_offset == text_len {
            let Some(next_id) = this.next_sibling().map(|n| n.node_id()) else {
                return Ok(false);
            };

            let next = self.node(next_id).context("Next node not found")?;
            let next_offset = calculate_offset_before_child(&paragraph, next_id);

            match next.node() {
                Node::Text(next_text) => {
                    let text_content = next_text.text.to_string();
                    let next_grapheme_offset = find_next_grapheme_boundary(&text_content, 0);

                    (head.offset, next_offset + next_grapheme_offset)
                }
                Node::HardBreak(_) => (head.offset, next_offset + 1),
                _ => {
                    return Ok(false);
                }
            }
        } else {
            if let Node::Text(text_node) = this.node() {
                let text_content = text_node.text.to_string();
                let next_grapheme_offset = find_next_grapheme_boundary(&text_content, local_offset);
                let global_offset_before_child =
                    calculate_offset_before_child(&paragraph, child_id);

                (
                    head.offset,
                    global_offset_before_child + next_grapheme_offset,
                )
            } else {
                (head.offset, head.offset + 1)
            }
        };

        let from = Position::new(head.node_id, from_global_offset, head.affinity);
        let to = Position::new(head.node_id, to_global_offset, head.affinity);

        self.delete_range(from, to)?;

        let new_affinity =
            resolve_affinity_after_edit(self, head.node_id, from_global_offset, head.affinity);

        self.set_selection(Selection::collapsed(Position::new(
            head.node_id,
            from_global_offset,
            new_affinity,
        )));

        self.push_effect(Effect::NodeChanged {
            node_id: head.node_id,
        });
        Ok(true)
    }

    pub fn delete_selection(&mut self) -> Result<bool> {
        let structure_selection = compute_structure_selection(self.doc(), self.selection());

        if let StructureSelectionInfo::Rectangular { .. } = structure_selection {
            return self.delete_structure_selection(&structure_selection);
        }

        if let StructureSelectionInfo::Structural(block_ids) = structure_selection {
            let (mut from, mut to) = self.selection().as_sorted(self.doc())?;

            for block_id in block_ids {
                let Some(block) = self.node(block_id) else {
                    continue;
                };
                let Some(parent) = block.parent() else {
                    continue;
                };
                let index = block.index().unwrap_or(0);

                let start_pos = Position::new(parent.node_id(), index, Affinity::Downstream);
                let end_pos = Position::new(parent.node_id(), index + 1, Affinity::Downstream);

                if crate::state::position_helpers::compare_positions(self.doc(), start_pos, from)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .is_lt()
                {
                    from = start_pos;
                }
                if crate::state::position_helpers::compare_positions(self.doc(), end_pos, to)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .is_gt()
                {
                    to = end_pos;
                }
            }

            self.set_selection(Selection::new(from, to));
        }

        let deleted = self.delete_selection_with_merge()?.deleted();

        Ok(deleted)
    }

    pub fn delete_selection_with_merge(&mut self) -> Result<DeleteResult> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            return Ok(DeleteResult::None);
        }

        let (from, to) = selection.as_sorted(self.doc())?;

        if self.crosses_isolating_boundary(from, to)? {
            return self.delete_across_isolating_boundary(from, to);
        }

        self.push_effect(Effect::NodeChanged {
            node_id: from.node_id,
        });
        if from.node_id != to.node_id {
            self.push_effect(Effect::NodeChanged {
                node_id: to.node_id,
            });
        }

        self.delete_range(from, to)?;

        if from.node_id != to.node_id {
            Ok(DeleteResult::Merged {
                from: to.node_id,
                into: from.node_id,
                merge_offset: from.offset,
                deleted_prefix: to.offset,
            })
        } else {
            Ok(DeleteResult::Local {
                node_id: from.node_id,
                start: from.offset,
                len: to.offset.saturating_sub(from.offset),
            })
        }
    }

    fn crosses_isolating_boundary(&self, from: Position, to: Position) -> Result<bool> {
        let from_node = self
            .doc()
            .node(from.node_id)
            .context("From node not found")?;
        let to_node = self.doc().node(to.node_id).context("To node not found")?;

        let find_isolating = |node: &crate::model::NodeRef<'_>| {
            if node.spec().isolating {
                return Some(node.node_id());
            }
            node.ancestors()
                .find(|a| a.spec().isolating)
                .map(|n| n.node_id())
        };

        let from_iso_id = find_isolating(&from_node);
        let to_iso_id = find_isolating(&to_node);

        match (from_iso_id, to_iso_id) {
            (Some(f_id), Some(t_id)) => Ok(f_id != t_id),
            (Some(_), None) | (None, Some(_)) => Ok(true),
            (None, None) => Ok(false),
        }
    }

    fn delete_across_isolating_boundary(
        &mut self,
        from: Position,
        to: Position,
    ) -> Result<DeleteResult> {
        if from == to {
            self.set_selection(Selection::collapsed(from));
            return Ok(DeleteResult::Local {
                node_id: from.node_id,
                start: from.offset,
                len: 0,
            });
        }

        self.delete_structural_range(from, to)?;

        self.set_selection(Selection::collapsed(from));

        Ok(DeleteResult::Local {
            node_id: from.node_id,
            start: from.offset,
            len: 0,
        })
    }

    fn delete_structural_range(&mut self, from: Position, to: Position) -> Result<()> {
        let lca_id = self.find_lowest_common_ancestor(from.node_id, to.node_id);
        let lca = self.doc().node(lca_id).context("LCA not found")?;

        if lca.spec().is_textblock(self.doc().schema()) {
            self.push_effect(Effect::NodeChanged {
                node_id: from.node_id,
            });
            if from.node_id != to.node_id {
                self.push_effect(Effect::NodeChanged {
                    node_id: to.node_id,
                });
            }
            self.delete_range(from, to)?;
            return Ok(());
        }

        let children: Vec<NodeId> = lca.children().map(|c| c.node_id()).collect();
        if children.is_empty() {
            return Ok(());
        }

        let (start_idx, end_idx) = {
            let contains_pos = |node_id: NodeId, pos: Position| {
                node_id == pos.node_id || self.is_ancestor_of(node_id, pos.node_id)
            };

            let start = if from.node_id == lca_id {
                from.offset.min(children.len())
            } else {
                children
                    .iter()
                    .position(|&child| contains_pos(child, from))
                    .unwrap()
            };

            let end = if to.node_id == lca_id {
                to.offset
                    .saturating_sub(1)
                    .min(children.len().saturating_sub(1))
            } else {
                children
                    .iter()
                    .rposition(|&child| contains_pos(child, to))
                    .unwrap_or(children.len().saturating_sub(1))
            };

            (start, end)
        };

        let mut i = start_idx;
        while i <= end_idx {
            let child_id = children[i];

            if self.is_barrier_node(child_id) {
                self.delete_barrier_segment(
                    child_id,
                    i == start_idx,
                    i == end_idx,
                    from,
                    to,
                    lca_id,
                )?;
                i += 1;
            } else {
                i = self.delete_non_barrier_segment(
                    &children, i, end_idx, start_idx, from, to, lca_id,
                )?;
            }
        }
        Ok(())
    }

    fn find_lowest_common_ancestor(&self, node_a: NodeId, node_b: NodeId) -> NodeId {
        let ancestors_a = self.collect_ancestors_including_self(node_a);
        let ancestors_b = self.collect_ancestors_including_self(node_b);

        let mut lca = NodeId::ROOT;
        for (a, b) in ancestors_a.iter().rev().zip(ancestors_b.iter().rev()) {
            if a == b {
                lca = *a;
            } else {
                break;
            }
        }
        lca
    }

    fn collect_ancestors_including_self(&self, node_id: NodeId) -> Vec<NodeId> {
        std::iter::once(node_id)
            .chain(
                self.doc()
                    .node(node_id)
                    .unwrap()
                    .ancestors()
                    .map(|n| n.node_id()),
            )
            .collect()
    }

    fn is_barrier_node(&self, node_id: NodeId) -> bool {
        self.doc()
            .node(node_id)
            .map(|n| {
                let spec = n.spec();
                spec.isolating || spec.structural
            })
            .unwrap_or(false)
    }

    fn delete_barrier_segment(
        &mut self,
        child_id: NodeId,
        is_first_in_range: bool,
        is_last_in_range: bool,
        from: Position,
        to: Position,
        lca_id: NodeId,
    ) -> Result<()> {
        let seg_from = if is_first_in_range && from.node_id != lca_id {
            from
        } else {
            Position::new(child_id, 0, Affinity::Downstream)
        };

        let child = self.doc().node(child_id).unwrap();
        let child_len = crate::state::selection_helpers::block_content_len(&child);

        let seg_to = if is_last_in_range && to.node_id != lca_id {
            to
        } else {
            Position::new(child_id, child_len, Affinity::Downstream)
        };

        let covers_entire_node = seg_from.node_id == child_id
            && seg_from.offset == 0
            && seg_to.node_id == child_id
            && seg_to.offset == child_len;
        let is_not_structural = !child.spec().structural;

        if covers_entire_node && is_not_structural {
            self.delete_node_recursive(child_id)?;
        } else {
            self.delete_structural_range(seg_from, seg_to)?;
        }
        Ok(())
    }

    fn delete_non_barrier_segment(
        &mut self,
        children: &[NodeId],
        start_group: usize,
        end_idx: usize,
        start_idx: usize,
        from: Position,
        to: Position,
        lca_id: NodeId,
    ) -> Result<usize> {
        let mut end_group = start_group;
        while end_group < end_idx && !self.is_barrier_node(children[end_group + 1]) {
            end_group += 1;
        }

        let group_from_pos = if start_group == start_idx && from.node_id != lca_id {
            Some(from)
        } else {
            self.find_first_textblock_pos(children[start_group])
        };

        let group_to_pos = if end_group == end_idx && to.node_id != lca_id {
            Some(to)
        } else {
            self.find_last_textblock_pos(children[end_group])
        };

        match (group_from_pos, group_to_pos) {
            (Some(g_from), Some(g_to)) if g_from != g_to => {
                self.push_effect(Effect::NodeChanged {
                    node_id: g_from.node_id,
                });
                if g_from.node_id != g_to.node_id {
                    self.push_effect(Effect::NodeChanged {
                        node_id: g_to.node_id,
                    });
                }
                self.delete_range(g_from, g_to)?;
                if start_group == start_idx && from.node_id == lca_id {
                    self.lift_container_contents_recursive(lca_id, start_group)?;
                }
            }
            (Some(_), Some(_)) => {}
            _ => {
                for k in start_group..=end_group {
                    self.delete_node_recursive(children[k])?;
                }
            }
        }

        Ok(end_group + 1)
    }

    fn lift_container_contents_recursive(
        &mut self,
        lca_id: NodeId,
        start_group: usize,
    ) -> Result<()> {
        loop {
            let children_now: Vec<_> = self
                .doc()
                .node(lca_id)
                .unwrap()
                .children()
                .map(|c| c.node_id())
                .collect();
            if start_group >= children_now.len() {
                break;
            }

            let container_id = children_now[start_group];
            let container = self.doc().node(container_id).unwrap();
            if !container.spec().is_textblock(self.doc().schema()) {
                if let Some(first_child) = container.first_child() {
                    self.try_lift_block(first_child.node_id(), lca_id, start_group)?;
                    continue;
                }
            }
            break;
        }
        Ok(())
    }

    fn find_first_textblock_pos(&self, node_id: NodeId) -> Option<Position> {
        let node = self.doc().node(node_id)?;
        if node.spec().is_textblock(self.doc().schema()) {
            return Some(Position::new(node_id, 0, Affinity::Downstream));
        }
        for child in node.children() {
            if let Some(pos) = self.find_first_textblock_pos(child.node_id()) {
                return Some(pos);
            }
        }
        None
    }

    fn find_last_textblock_pos(&self, node_id: NodeId) -> Option<Position> {
        let node = self.doc().node(node_id)?;
        if node.spec().is_textblock(self.doc().schema()) {
            let len = crate::state::selection_helpers::block_content_len(&node);
            return Some(Position::new(node_id, len, Affinity::Downstream));
        }
        let children: Vec<_> = node.children().collect();
        for child in children.iter().rev() {
            if let Some(pos) = self.find_last_textblock_pos(child.node_id()) {
                return Some(pos);
            }
        }
        None
    }

    pub(crate) fn is_ancestor_of(&self, ancestor_id: NodeId, node_id: NodeId) -> bool {
        if ancestor_id == node_id {
            return false;
        }
        let Some(node) = self.doc().node(node_id) else {
            return false;
        };
        node.ancestors().any(|a| a.node_id() == ancestor_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use crate::state::{Position, Selection};
    use crate::types::Affinity;

    #[test]
    fn insert_text_at_middle() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                }
            }

            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_text(" ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }

            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_at_slot() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { }
            }

            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("h").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "h" }
                }
            }

            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_between_hard_breaks() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("h").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "h" }
                    hard_break { }
                }
            }
            selection { (p, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_at_text_node_beginning() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("hello ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_at_text_node_end() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_text(" world").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_inherits_marks_at_boundary() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "asdf" }
                }
            }
            selection { (p, 0) -> (p, 2) }
        };

        let actual = transact!(initial, |tr| {
            tr.add_mark(Mark::Italic(ItalicMark)).unwrap();
            tr.set_selection(Selection::collapsed(Position::new(
                p,
                2,
                Affinity::Downstream,
            )));
            tr.insert_text("z").unwrap()
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "asz" => [italic()], "df" }
                }
            }
            selection { (p, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_multiple_characters() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("xyz").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "axyzb" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_before_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("!").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello!" }
                    hard_break { }
                }
            }
            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_after_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("hello ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "hello world" }
                }
            }
            selection { (p, 7) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_between_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_text(" ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_korean_characters() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "안녕" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("하세요 ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "안하세요 녕" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_emoji() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("👋🌍").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello👋🌍" }
                }
            }
            selection { (p, 7) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_in_long_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "The quick brown fox jumps over the lazy dog" }
                }
            }
            selection { (p, 19) }
        };

        let actual = transact!(initial, |tr| tr.insert_text(" red").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "The quick brown fox red jumps over the lazy dog" }
                }
            }
            selection { (p, 23) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_in_second_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "first" }
                }
                @p paragraph {
                    text { "second" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("x").unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "first" }
                }
                @p paragraph {
                    text { "secxond" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_at_boundary_of_multiple_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "abc" }
                }
            }
            selection { (p, 2) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("X").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "abXc" }
                }
            }
            selection { (p, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_mixed_content_before_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "aabb" }
                    hard_break { }
                    text { "cc" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("ZZ").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "aabZZb" }
                    hard_break { }
                    text { "cc" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_at_the_beginning() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_in_the_middle() {
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

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hellowo" }
                    hard_break { }
                    text { "rld" }
                }
            }
            selection { (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_in_the_end() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 10) }
        };

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "helloworld" }
                    hard_break { }
                }
            }
            selection { (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_in_empty_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break { }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_after_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 6) }
        };

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    hard_break { }
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 7) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_hard_break_after_hard_break_2() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 7) }
        };

        let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_at_the_beginning() {
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

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hell" }
                    text { "world" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn text_node_removal_by_delete_text_backward() {
        let mut p = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                @p paragraph {
                    text { " " }
                    text { "world" }
                }
            }

            selection {
                (p, 1)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                @p paragraph {
                    text { "world" }
                }
            }

            selection {
                (p, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn text_node_removal_by_delete_text_backward_single_empty_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { " " }
                }
            }

            selection {
                (p, 1)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph { }
            }

            selection {
                (p, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn text_node_removal_by_delete_text_forward() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello," }
                    text { " " }
                    text { "world" }
                }
            }

            selection {
                (p, 6)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello," }
                    text { "world" }
                }
            }

            selection {
                (p, 6)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn text_node_removal_by_delete_text_forward_single_empty_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { " " }
                }
            }

            selection {
                (p, 0)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph { }
            }

            selection {
                (p, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_paragraph_last_1_char_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                    text { "l" }
                }
            }

            selection {
                (p, 3)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                }
            }

            selection {
                (p, 2)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_after_blockquote() {
        let mut p = id!();

        let initial = state! {
            doc {
                blockquote {
                    paragraph {
                        text { "hello" }
                    }
                }
                @p paragraph {
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.join_backward().unwrap());

        let expected = state! {
            doc {
                blockquote {
                    @p paragraph {
                        text { "helloworld" }
                    }
                }
                paragraph {}
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_forward_paragraph_last_1_char_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                    text { "l" }
                }
            }

            selection {
                (p, 2)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                }
            }

            selection {
                (p, 2)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_between_1_char_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                    text { "l" }
                }
            }

            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "e" }
                    text { "l" }
                }
            }

            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 6) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_hard_break_at_end() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                }
            }
            selection { (p, 6) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_forward_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_of_multiple_1_char_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    text { "e" }
                    text { "l" }
                    text { "l" }
                }
            }
            selection { (p, 1) -> (p, 4) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "h" }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_start_as_slot() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "asdf" }
                }
            }

            selection {
                (p1, 0) -> (p2, 0)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "asdf" }
                }
            }

            selection {
                (p1, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_of_two_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "asdf" }
                }
                @p2 paragraph {
                    text { "asdf" }
                }
            }

            selection {
                (p1, 0) -> (p2, 4)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
            }

            selection {
                (p1, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn selection_after_delete_selection() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "asdf" }
                }
                paragraph { }
                @p1 paragraph { }
                @p2 paragraph {
                    text { "asdf" }
                }
            }

            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "asdf" }
                }
                paragraph { }
                @p1 paragraph {
                    text { "asdf" }
                }
            }

            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn to_slot_position_node_after_delete_selection() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "asdf" }
                }
                @p2 paragraph { }
                paragraph {
                    text { "asdf" }
                }
            }

            selection { (p1, 4) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "asdf" }
                }
                @p2 paragraph {
                    text { "asdf" }
                }
            }

            selection { (p1, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_empty_paragraphs_at_ends() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                paragraph {
                    text { "asdf" }
                }
                @p2 paragraph { }
            }

            selection {
                (p1, 0) -> (p2, 0)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
            }

            selection {
                (p1, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_of_a_paragraph_with_multiple_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }

            selection {
                (p, 0) -> (p, 10)
            }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph { }
            }

            selection {
                (p, 0)
            }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_of_multiple_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "qwer" }
                    text { "asdf" }
                    text { "zxcv" }
                }
            }
            selection { (p, 2) -> (p, 10) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "qw" }
                    text { "cv" }
                }
            }
            selection { (p, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_including_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "qwer" }
                    hard_break {}
                    text { "zxcv" }
                }
            }
            selection { (p, 2) -> (p, 7) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "qw" }
                    text { "cv" }
                }
            }
            selection { (p, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_after_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "qwer" }
                    hard_break {}
                    hard_break {}
                    text { "zxcv" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "qwer" }
                    hard_break {}
                    text { "cv" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_before_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "qwer" }
                    hard_break {}
                    hard_break {}
                    text { "zxcv" }
                }
            }
            selection { (p, 2) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "qw" }
                    hard_break {}
                    text { "zxcv" }
                }
            }
            selection { (p, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_blockquote() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                blockquote {
                    @p2 paragraph {
                        text { "world" }
                    }
                }
                paragraph {}
            }
            selection { (p1, 5) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                paragraph { }
            }
            selection { (p1, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_image() {
        let mut n = id!();

        let initial = state! {
            doc {
                image()
                @n paragraph {
                    text { "hello" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (n, 5) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph { }
            }
            selection { (n, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_image_2() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                image()
                paragraph {}
            }
            selection { (n, 0) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph { }
                paragraph { }
            }
            selection { (n, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_image_3() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                image()
                paragraph {}
            }
            selection { (n, 5) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                paragraph { }
            }
            selection { (n, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_with_image_4() {
        let mut n = id!();

        let initial = state! {
            doc {
                image()
                @n paragraph {
                    text { "hello" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (n, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
            }
            selection { (n, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_image_selection_1() {
        let mut n = id!();

        let initial = state! {
            doc {
                image()
                @n paragraph {
                    text { "hello" }
                }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
            }
            selection { (n, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_image_selection_2() {
        let mut n = id!();

        let initial = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                paragraph { }
            }
            selection { (n, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_image_selection_3() {
        let mut n = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "hello" }
                }
                image()
                paragraph {
                    text { "world" }
                }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @n paragraph {
                    text { "hello" }
                }
                paragraph {
                    text { "world" }
                }
            }
            selection { (n, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_of_mark_and_paragraph() {
        let mut p = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "a" }
                    text(marks: [italic()]) { "b" }
                    text { "c" }
                }
                @p2 paragraph { }
            }
            selection { (p, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_forward_hard_break_at_start() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

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
    fn delete_selection_hard_break_at_start() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "world" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_all_list_items() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                    }
                    list_item {
                        @p2 paragraph {
                            text { "B" }
                        }
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_text_applies_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.state.pending_marks = Some(vec![Mark::Italic(ItalicMark)]);
        tr.insert_text("X").unwrap();
        let (new_state, _) = tr.commit().unwrap();

        let p_node = new_state.doc.node(p).unwrap();
        let text_child = p_node.first_child().unwrap();

        if let Node::Text(text_node) = text_child.node() {
            let segments = text_node.text.get_rich_text_segments();
            assert!(segments.len() >= 2);
            let last_segment = segments.last().unwrap();
            assert!(last_segment.1.iter().any(|m| matches!(m, Mark::Italic(_))));
        } else {
            panic!("Expected text node");
        }

        assert!(new_state.pending_marks.is_none());
    }

    #[test]
    fn insert_text_clears_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.state.pending_marks = Some(vec![Mark::Italic(ItalicMark)]);
        tr.insert_text("X").unwrap();
        let (new_state, _) = tr.commit().unwrap();

        assert!(new_state.pending_marks.is_none());
    }

    #[test]
    fn insert_text_at_slot_with_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.state.pending_marks = Some(vec![Mark::FontWeight(FontWeightMark { weight: 700 })]);
        tr.insert_text("Bold").unwrap();
        let (new_state, _) = tr.commit().unwrap();

        let p_node = new_state.doc.node(p).unwrap();
        let text_child = p_node.first_child().unwrap();

        if let Node::Text(text_node) = text_child.node() {
            let segments = text_node.text.get_rich_text_segments();
            assert_eq!(segments.len(), 1);
            assert!(
                segments[0]
                    .1
                    .iter()
                    .any(|m| matches!(m, Mark::FontWeight(fw) if fw.weight == 700))
            );
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn delete_text_backward_emoji() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "a👨‍👩‍👧‍👦b" }
                }
            }
            selection { (p, 8) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_forward_emoji() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "a👨‍👩‍👧‍👦b" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_across_hard_breaks_preserves_grapheme() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "❤️" }
                    hard_break {}
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 4, Affinity::Downstream) -> (p, 2, Affinity::Upstream) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "❤️" }
                    hard_break {}
                }
            }
            selection { (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_flag_emoji() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "a🇺🇸b" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 1) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_hard_break_after_emoji() {
        let mut p = id!();
        let initial = state! {
            doc {
                @p paragraph {
                    text { "❤️" }
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "❤️" }
                    hard_break {}
                }
            }
            selection { (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_between_hard_breaks_affinity_upstream() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "a" }
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "a" }
                    hard_break { }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_between_hard_breaks_affinity_downstream() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "a" }
                    hard_break { }
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "a" }
                    hard_break { }
                    hard_break { }
                }
            }
            selection { (p, 2, Affinity::Downstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_text_backward_after_hard_break_single_char_downstream() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break { }
                    text { "a" }
                }
            }
            selection { (p, 2, Affinity::Upstream) }
        };

        let actual = transact!(initial, |tr| tr.delete_text_backward().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break { }
                }
            }
            selection { (p, 1, Affinity::Downstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_from_empty_paragraph_after_blockquote() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                blockquote {
                    @p1 paragraph {
                    }
                    paragraph {
                        text { "ㅁㄴㅇㅁㄴㅇ" }
                    }
                }
                @p2 paragraph { }
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_merge_adjacent_list() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                    list_item {
                        @p1 paragraph {
                            text { "2" }
                        }
                    }
                }
                ordered_list {
                    list_item {
                        @p2 paragraph {
                            text { "3" }
                        }
                    }
                    list_item {
                        paragraph {
                            text { "4" }
                        }
                    }
                }
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                    list_item {
                        @p1 paragraph {
                            text { "3" }
                        }
                    }
                    list_item {
                        paragraph {
                            text { "4" }
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
    fn delete_selection_list_with_empty_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                ordered_list {
                    list_item {
                        @p1 paragraph {
                            text { "asd" }
                        }
                    }
                    list_item {
                        @p2 paragraph {}
                    }
                }
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        @p1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_selection_list_with_empty_paragraph_2() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                ordered_list {
                    list_item {
                        @p1 paragraph {
                            text { "asd" }
                        }
                    }
                    list_item {
                        paragraph {}
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {}
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_page_break_at_end_of_middle_paragraph() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "A" }
                }
                @p2 paragraph {
                    text { "B" }
                }
            }
            selection { (p1, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_page_break().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "A" }
                    page_break {}
                }
                @p2 paragraph {
                    text { "B" }
                }
            }
            selection { (p2, 0) } // Selection should be at the start of the next paragraph
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_page_break_at_end_of_last_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_page_break().unwrap());

        // Should create a new trailing paragraph because the last one has a page break
        // And cursor should be in that new paragraph
        let expected = state! {
            doc {
                paragraph {
                    text { "A" }
                    page_break {}
                }
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_page_break_in_middle_of_paragraph() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "AB" }
                }
            }
            selection { (p1, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_page_break().unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "A" }
                    page_break {}
                }
                @p1 paragraph {
                    text { "B" }
                }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn insert_page_break_before_image() {
        let mut p = id!();
        let mut img = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "A" }
                }
                @img image { }
                paragraph {}
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr.insert_page_break().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "A" }
                    page_break {}
                }
                @img image { }
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn delete_forward_into_list_invalidates_layout() {
        use crate::runtime::Effect;

        let mut p = id!();
        let mut list = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "Hello" } }
                @list bullet_list {
                    list_item {
                        paragraph { text { "World" } }
                    }
                }
                paragraph {}
            }
            selection { (p, 5) }
        };

        let (actual, effects) = transact_with_effect!(initial, |tr| tr.join_forward().unwrap());

        let expected = state! {
            doc {
                @p paragraph { text { "HelloWorld" } }
                paragraph {}
            }
            selection { (p, 5) }
        };

        assert_state_eq!(&actual, expected);

        let has_list_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == list));
        assert!(
            has_list_changed,
            "NodeChanged for list should be emitted for layout recalculation. Effects: {:?}",
            effects
        );
    }

    #[test]
    fn delete_forward_into_list_with_remaining_items() {
        use crate::runtime::Effect;

        let mut p = id!();
        let mut list = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "Hello" } }
                @list bullet_list {
                    list_item {
                        paragraph { text { "First" } }
                    }
                    list_item {
                        paragraph { text { "Second" } }
                    }
                }
            }
            selection { (p, 5) }
        };

        let list_id = list;

        let (_, effects) = transact_with_effect!(initial, |tr| tr.join_forward().unwrap());

        let has_list_changed = effects
            .iter()
            .any(|e| matches!(e, Effect::NodeChanged { node_id } if *node_id == list_id));
        assert!(
            has_list_changed,
            "NodeChanged for list {:?} should be emitted for layout recalculation. Effects: {:?}",
            list_id, effects
        );
    }

    #[test]
    fn delete_selection_from_text_to_first_hr_should_not_delete_other_hrs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "text1" } }
                horizontal_rule {}
                horizontal_rule {}
                horizontal_rule {}
                @p2 paragraph { text { "text2" } }
            }
            selection { (p1, 0) -> (NodeId::ROOT, 2) }
        };

        let actual = transact!(initial, |tr| tr.delete_selection().unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {}
                horizontal_rule {}
                horizontal_rule {}
                @p2 paragraph { text { "text2" } }
            }
            selection { (p1, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn test_delete_fold_selection_structural() {
        let mut fold_id = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { text { "before" } }
                @fold_id fold {
                    fold_title { text { "Title" } }
                    fold_content {
                        paragraph { text { "Inside" } }
                    }
                }
                @p2 paragraph { text { "after" } }
            }
            selection { (p1, 6) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
        });

        let expected = state! {
            doc {
                @p1 paragraph { text { "beforeafter" } }
            }
            selection { (p1, 6) -> (p1, 6) }
        };

        assert_state_eq!(actual, expected);
    }
}

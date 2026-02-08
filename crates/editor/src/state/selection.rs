use crate::model::{Doc, Fragment, Node, NodeId};
use crate::state::BlockTraverser;
use crate::state::position::Position;
use crate::state::position_helpers::{compare_positions, is_block_position};
use anyhow::{Context, Result};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn collapsed(position: Position) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn as_sorted(&self, doc: &Doc) -> Result<(Position, Position)> {
        match compare_positions(doc, self.anchor, self.head)? {
            Ordering::Greater => Ok((self.head, self.anchor)),
            _ => Ok((self.anchor, self.head)),
        }
    }

    pub fn validate(&self, doc: &Doc) -> Result<()> {
        let _ = doc
            .node(self.anchor.node_id)
            .context("Anchor node not found")?;
        let _ = doc.node(self.head.node_id).context("Head node not found")?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn extract_fragment(&self, doc: &Doc) -> Result<Fragment> {
        Fragment::new_from_selection(doc, self)
    }

    pub fn classify(&self, doc: &Doc) -> Result<SelectionKind> {
        let anchor_block = is_block_position(doc, self.anchor);
        let head_block = is_block_position(doc, self.head);

        if anchor_block && head_block {
            if self.anchor.node_id == self.head.node_id {
                return Ok(SelectionKind::BlockRange);
            }

            return Ok(SelectionKind::BlockAcrossContainers);
        }

        Ok(SelectionKind::InlineRange)
    }

    pub fn anchor_before_head(&self, doc: &Doc) -> bool {
        match compare_positions(doc, self.anchor, self.head) {
            Ok(Ordering::Greater) => false,
            _ => true,
        }
    }

    pub fn to_plain_text(&self, doc: &Doc) -> String {
        if self.is_collapsed() {
            return String::new();
        }

        let Ok((from, to)) = self.as_sorted(doc) else {
            return String::new();
        };

        if from.node_id == to.node_id {
            let is_textblock = doc
                .node(from.node_id)
                .map_or(false, |n| n.spec().is_textblock(doc.schema()));

            if is_textblock {
                return extract_block_text_range(doc, from.node_id, from.offset, to.offset);
            }

            let Ok(blocks) = crate::state::collect_blocks_in_range(doc, from, to) else {
                return String::new();
            };
            let mut result = String::new();
            for block_id in blocks {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&extract_block_text_full(doc, block_id));
            }
            return result;
        }

        let mut result = String::new();

        result.push_str(&extract_block_text_from(doc, from.node_id, from.offset));

        let Ok(mut traverser) = BlockTraverser::new_after_subtree(doc, from.node_id) else {
            return result;
        };

        while let Some(block_id) = traverser.next() {
            if block_id == to.node_id {
                break;
            }

            result.push('\n');
            result.push_str(&extract_block_text_full(doc, block_id));
        }

        result.push('\n');
        result.push_str(&extract_block_text_to(doc, to.node_id, to.offset));

        result
    }
}

fn extract_block_text_full(doc: &Doc, block_id: NodeId) -> String {
    let Some(block) = doc.node(block_id) else {
        return String::new();
    };

    let mut result = String::new();
    for child in block.children() {
        match child.node() {
            Node::Text(text_node) => {
                result.push_str(&text_node.text.as_str());
            }
            Node::HardBreak(_) => {
                result.push('\n');
            }
            _ => {}
        }
    }
    result
}

fn extract_block_text_range(
    doc: &Doc,
    block_id: NodeId,
    from_offset: usize,
    to_offset: usize,
) -> String {
    let full_text = extract_block_text_full(doc, block_id);
    let chars: Vec<char> = full_text.chars().collect();
    let from = from_offset.min(chars.len());
    let to = to_offset.min(chars.len());
    chars[from..to].iter().collect()
}

fn extract_block_text_from(doc: &Doc, block_id: NodeId, from_offset: usize) -> String {
    let full_text = extract_block_text_full(doc, block_id);
    let chars: Vec<char> = full_text.chars().collect();
    let from = from_offset.min(chars.len());
    chars[from..].iter().collect()
}

fn extract_block_text_to(doc: &Doc, block_id: NodeId, to_offset: usize) -> String {
    let full_text = extract_block_text_full(doc, block_id);
    let chars: Vec<char> = full_text.chars().collect();
    let to = to_offset.min(chars.len());
    chars[..to].iter().collect()
}

#[derive(Copy, Clone, Debug)]
pub enum SelectionKind {
    BlockRange,
    BlockAcrossContainers,
    InlineRange,
}

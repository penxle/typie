use crate::model::{Doc, Fragment};
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
}

#[derive(Copy, Clone, Debug)]
pub enum SelectionKind {
    BlockRange,
    BlockAcrossContainers,
    InlineRange,
}

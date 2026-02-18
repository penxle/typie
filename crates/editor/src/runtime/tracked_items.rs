use crate::layout::Page;
use crate::model::{Doc, NodeId};
use crate::runtime::Runtime;
use crate::state::position_helpers::{calculate_offset_before_child, find_text_at_offset};
use crate::types::TextBound;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
#[repr(u32)]
pub enum TrackedItemGroup {
    Spellcheck = 0,
    AiFeedback = 1,
    Search = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct RawTrackedItem {
    pub id: String,
    #[serde(
        serialize_with = "serialize_node_id",
        deserialize_with = "deserialize_node_id"
    )]
    pub node_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
}

fn serialize_node_id<S>(node_id: &NodeId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&node_id.to_string())
}

fn deserialize_node_id<'de, D>(deserializer: D) -> Result<NodeId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NodeId::from_string(&s).ok_or_else(|| serde::de::Error::custom("invalid node_id"))
}

#[derive(Clone)]
pub struct TrackedItem {
    pub id: String,
    pub group: TrackedItemGroup,
    pub node_id: NodeId,
    pub start_node_id: NodeId,
    pub start_cursor: loro::cursor::Cursor,
    pub end_node_id: NodeId,
    pub end_cursor: loro::cursor::Cursor,
}

impl TrackedItem {
    pub fn resolve_range(&self, doc: &Doc) -> Option<(NodeId, usize, usize)> {
        let loro = doc.loro_doc();

        let start_internal = loro
            .get_cursor_pos(&self.start_cursor)
            .ok()
            .map(|p| p.current.pos)?;
        let end_internal = loro
            .get_cursor_pos(&self.end_cursor)
            .ok()
            .map(|p| p.current.pos)?;

        let block_node = doc.node(self.node_id)?;

        let start_before = calculate_offset_before_child(&block_node, self.start_node_id);
        let end_before = calculate_offset_before_child(&block_node, self.end_node_id);

        Some((
            self.node_id,
            start_before + start_internal,
            end_before + end_internal,
        ))
    }
}

pub struct TrackedItemOverlay {
    pub page_idx: usize,
    pub group: u32,
    pub id: String,
    pub node_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
    pub bounds: Vec<TextBound>,
}

pub fn build_tracked_item_overlays(
    pages: &[Page],
    items: &[TrackedItem],
    doc: &Doc,
) -> Vec<TrackedItemOverlay> {
    let mut overlays = Vec::new();

    for item in items {
        let Some((node_id, start_flat, end_flat)) = item.resolve_range(doc) else {
            continue;
        };

        for (page_idx, page) in pages.iter().enumerate() {
            let bounds = page.get_text_range_bounds(node_id, start_flat, end_flat);
            if !bounds.is_empty() {
                overlays.push(TrackedItemOverlay {
                    page_idx,
                    group: item.group as u32,
                    id: item.id.clone(),
                    node_id,
                    start_offset: start_flat,
                    end_offset: end_flat,
                    bounds,
                });
                break;
            }
        }
    }

    overlays
}

impl Runtime {
    pub fn set_tracked_items(&mut self, group: u32, raw_items: Vec<RawTrackedItem>) {
        let target_group = match group {
            0 => TrackedItemGroup::Spellcheck,
            1 => TrackedItemGroup::AiFeedback,
            2 => TrackedItemGroup::Search,
            _ => return,
        };

        self.tracked_items.retain(|item| item.group != target_group);

        for raw in raw_items {
            let Some(block_node) = self.state.doc.node(raw.node_id) else {
                continue;
            };

            let Some((start_child_id, start_internal, start_text)) =
                find_text_at_offset(&self.state.doc, &block_node, raw.start_offset)
            else {
                continue;
            };

            let Some((end_child_id, end_internal, end_text)) =
                find_text_at_offset(&self.state.doc, &block_node, raw.end_offset)
            else {
                continue;
            };

            let Some(start_cursor) =
                start_text.get_cursor(start_internal, loro::cursor::Side::Right)
            else {
                continue;
            };

            let Some(end_cursor) = end_text.get_cursor(end_internal, loro::cursor::Side::Left)
            else {
                continue;
            };

            self.tracked_items.push(TrackedItem {
                id: raw.id,
                group: target_group,
                node_id: raw.node_id,
                start_node_id: start_child_id,
                start_cursor,
                end_node_id: end_child_id,
                end_cursor,
            });
        }

        self.pending.tracked_items = true;
    }

    pub fn remove_tracked_items(&mut self, group: u32, ids: &[String]) {
        if ids.is_empty() {
            return;
        }

        let target_group = match group {
            0 => TrackedItemGroup::Spellcheck,
            1 => TrackedItemGroup::AiFeedback,
            2 => TrackedItemGroup::Search,
            _ => return,
        };

        let id_set: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();

        self.tracked_items
            .retain(|item| item.group != target_group || !id_set.contains(item.id.as_str()));

        self.pending.tracked_items = true;
    }
}

use crate::layout::Page;
use crate::model::{Doc, NodeId};
use crate::runtime::Runtime;
use crate::runtime::cmd::AiFeedbackOverlay;
use crate::state::position_helpers::{calculate_offset_before_child, find_text_at_offset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct RawAiFeedbackItem {
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
pub struct AiFeedbackItem {
    pub id: String,
    pub node_id: NodeId,
    pub start_node_id: NodeId,
    pub start_cursor: loro::cursor::Cursor,
    pub end_node_id: NodeId,
    pub end_cursor: loro::cursor::Cursor,
    pub original_length: usize,
}

impl AiFeedbackItem {
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

pub fn build_ai_feedback_overlays(
    pages: &[Page],
    items: &[AiFeedbackItem],
    doc: &Doc,
    active_item_id: Option<&String>,
) -> Vec<AiFeedbackOverlay> {
    let mut overlays = Vec::new();

    for item in items {
        let Some((node_id, start_flat, end_flat)) = item.resolve_range(doc) else {
            continue;
        };

        for (page_idx, page) in pages.iter().enumerate() {
            let bounds = page.get_text_range_bounds(node_id, start_flat, end_flat);
            if !bounds.is_empty() {
                overlays.push(AiFeedbackOverlay {
                    page_idx,
                    id: item.id.clone(),
                    bounds,
                    is_active: Some(&item.id) == active_item_id,
                });
                break;
            }
        }
    }

    overlays
}

impl Runtime {
    pub fn get_ai_feedback_items(&mut self) -> Vec<RawAiFeedbackItem> {
        self.ai_feedback_items
            .iter()
            .filter_map(|item| {
                let (node_id, start_offset, end_offset) = item.resolve_range(&self.state.doc)?;
                Some(RawAiFeedbackItem {
                    id: item.id.clone(),
                    node_id,
                    start_offset,
                    end_offset,
                })
            })
            .collect()
    }

    pub(crate) fn update_active_ai_feedback_item(&mut self) -> bool {
        let head = self.state.selection.head;

        let new_active_id = self
            .ai_feedback_items
            .iter()
            .find(|item| {
                if item.node_id != head.node_id {
                    return false;
                }

                item.resolve_range(&self.state.doc)
                    .is_some_and(|(_, start, end)| head.offset >= start && head.offset <= end)
            })
            .map(|item| item.id.clone());

        if self.active_ai_feedback_item_id != new_active_id {
            self.active_ai_feedback_item_id = new_active_id;
            true
        } else {
            false
        }
    }

    pub(crate) fn clean_invalidated_ai_feedback_items(&mut self) -> bool {
        if self.ai_feedback_items.is_empty() {
            return false;
        }

        let old_count = self.ai_feedback_items.len();

        self.ai_feedback_items.retain(|item| {
            if let Some((_, start, end)) = item.resolve_range(&self.state.doc) {
                start < end && (end - start) == item.original_length
            } else {
                false
            }
        });

        self.ai_feedback_items.len() != old_count
    }

    pub fn set_ai_feedback_items(&mut self, raw_items: Vec<RawAiFeedbackItem>) {
        let mut items = Vec::new();

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

            items.push(AiFeedbackItem {
                id: raw.id,
                node_id: raw.node_id,
                start_node_id: start_child_id,
                start_cursor,
                end_node_id: end_child_id,
                end_cursor,
                original_length: raw.end_offset.saturating_sub(raw.start_offset),
            });
        }

        self.ai_feedback_items = items;
        self.update_active_ai_feedback_item();
        self.pending.ai_feedback_overlays = true;
    }

    pub fn clear_ai_feedback_items(&mut self) {
        if !self.ai_feedback_items.is_empty() {
            self.ai_feedback_items.clear();
            self.active_ai_feedback_item_id = None;
            self.pending.ai_feedback_overlays = true;
        }
    }

    pub fn has_ai_feedback_items(&self) -> bool {
        !self.ai_feedback_items.is_empty()
    }
}

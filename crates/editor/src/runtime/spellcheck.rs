use crate::layout::Page;
use crate::model::{Doc, NodeId};
use crate::runtime::Runtime;
use crate::runtime::cmd::SpellcheckOverlay;
use crate::state::position_helpers::{calculate_offset_before_child, find_text_at_offset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct RawSpellcheckError {
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

// Loro cursor를 이용해 포지션을 추적하는 spellcheck error
#[derive(Clone)]
pub struct SpellcheckError {
    pub id: String,
    pub node_id: NodeId,       // Block ID
    pub start_node_id: NodeId, // Start Leaf node ID
    pub start_cursor: loro::cursor::Cursor,
    pub end_node_id: NodeId, // End Leaf node ID
    pub end_cursor: loro::cursor::Cursor,
    pub original_length: usize,
}

impl SpellcheckError {
    // Loro cursor -> (node_id, start_offset, end_offset)
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

pub fn build_spellcheck_overlays(
    pages: &[Page],
    errors: &[SpellcheckError],
    doc: &Doc,
    active_error_id: Option<&String>,
) -> Vec<SpellcheckOverlay> {
    let mut overlays = Vec::new();

    for error in errors {
        let Some((node_id, start_flat, end_flat)) = error.resolve_range(doc) else {
            continue;
        };

        for (page_idx, page) in pages.iter().enumerate() {
            let bounds = page.get_text_range_bounds(node_id, start_flat, end_flat);
            if !bounds.is_empty() {
                overlays.push(SpellcheckOverlay {
                    page_idx,
                    id: error.id.clone(),
                    bounds,
                    is_active: Some(&error.id) == active_error_id,
                });
                break;
            }
        }
    }

    overlays
}

impl Runtime {
    pub fn get_spellcheck_errors(&mut self) -> Vec<RawSpellcheckError> {
        self.spellcheck_errors
            .iter()
            .filter_map(|e| {
                let (node_id, start_offset, end_offset) = e.resolve_range(&self.state.doc)?;
                Some(RawSpellcheckError {
                    id: e.id.clone(),
                    node_id,
                    start_offset,
                    end_offset,
                })
            })
            .collect()
    }

    pub(crate) fn update_active_spellcheck_error(&mut self) -> bool {
        let head = self.state.selection.head;

        let new_active_id = self
            .spellcheck_errors
            .iter()
            .find(|e| {
                if e.node_id != head.node_id {
                    return false;
                }

                e.resolve_range(&self.state.doc)
                    .is_some_and(|(_, start, end)| head.offset >= start && head.offset <= end)
            })
            .map(|e| e.id.clone());

        if self.active_spellcheck_error_id != new_active_id {
            self.active_spellcheck_error_id = new_active_id;
            true
        } else {
            false
        }
    }

    pub(crate) fn clean_invalidated_spellcheck_errors(&mut self) -> bool {
        if self.spellcheck_errors.is_empty() {
            return false;
        }

        let old_count = self.spellcheck_errors.len();

        self.spellcheck_errors.retain(|e| {
            if let Some((_, start, end)) = e.resolve_range(&self.state.doc) {
                start < end && (end - start) == e.original_length
            } else {
                false
            }
        });

        self.spellcheck_errors.len() != old_count
    }

    pub fn set_spellcheck_errors(&mut self, raw_errors: Vec<RawSpellcheckError>) {
        let mut errors = Vec::new();

        for raw in raw_errors {
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

            errors.push(SpellcheckError {
                id: raw.id,
                node_id: raw.node_id,
                start_node_id: start_child_id,
                start_cursor,
                end_node_id: end_child_id,
                end_cursor,
                original_length: raw.end_offset.saturating_sub(raw.start_offset),
            });
        }

        self.spellcheck_errors = errors;
        self.update_active_spellcheck_error();
        self.pending.spellcheck_overlays = true;
    }

    pub fn clear_spellcheck_errors(&mut self) {
        if !self.spellcheck_errors.is_empty() {
            self.spellcheck_errors.clear();
            self.active_spellcheck_error_id = None;
            self.pending.spellcheck_overlays = true;
        }
    }

    pub fn has_spellcheck_errors(&self) -> bool {
        !self.spellcheck_errors.is_empty()
    }

    pub fn apply_spellcheck_correction(
        &mut self,
        node_id: NodeId,
        start_offset: usize,
        end_offset: usize,
        correction: &str,
    ) -> bool {
        let mut to_remove = Vec::new();
        for e in &self.spellcheck_errors {
            if e.node_id != node_id {
                continue;
            }

            if let Some((_, e_start_flat, e_end_flat)) = e.resolve_range(&self.state.doc) {
                if e_start_flat == start_offset && e_end_flat == end_offset {
                    to_remove.push(e.id.clone());
                }
            } else {
                to_remove.push(e.id.clone());
            }
        }

        if self
            .replace_text_in_block(node_id, start_offset, end_offset, correction)
            .is_err()
        {
            return false;
        }

        self.spellcheck_errors
            .retain(|e| !to_remove.contains(&e.id));

        self.pending.spellcheck_overlays = true;

        true
    }
}

use crate::layout::Page;
use crate::model::{Doc, Node, NodeId};
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
    pub node_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
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
        let loro = doc.loro();

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

        let mut overlay = TrackedItemOverlay {
            page_idx: 0,
            group: item.group as u32,
            id: item.id.clone(),
            node_id,
            start_offset: start_flat,
            end_offset: end_flat,
            bounds: Vec::new(),
        };

        for (page_idx, page) in pages.iter().enumerate() {
            let bounds = page.get_text_range_bounds(node_id, start_flat, end_flat);
            if !bounds.is_empty() {
                overlay.page_idx = page_idx;
                overlay.bounds = bounds;
                break;
            }
        }

        overlays.push(overlay);
    }

    overlays
}

impl Runtime {
    fn tracked_item_group(group: u32) -> Option<TrackedItemGroup> {
        match group {
            0 => Some(TrackedItemGroup::Spellcheck),
            1 => Some(TrackedItemGroup::AiFeedback),
            2 => Some(TrackedItemGroup::Search),
            _ => None,
        }
    }

    fn reveal_node_in_folds(&mut self, node_id: NodeId) -> bool {
        let Some(node) = self.state.doc.node(node_id) else {
            return false;
        };

        let fold_ids: Vec<_> = node
            .ancestors()
            .filter_map(|ancestor| match ancestor.node() {
                Some(Node::Fold(_)) => Some(ancestor.node_id()),
                _ => None,
            })
            .collect();

        if fold_ids.is_empty() {
            return false;
        }

        let mut effects = Vec::new();
        let mut changed = false;

        for fold_id in fold_ids.into_iter().rev() {
            if self.layout_engine.fold_expanded(fold_id) {
                continue;
            }

            effects.extend(self.toggle_view_state(fold_id));
            changed = true;
        }

        if changed {
            self.process_effects(effects);
        }

        changed
    }

    pub fn reveal_tracked_item(&mut self, group: u32, id: &str) -> bool {
        let Some(target_group) = Self::tracked_item_group(group) else {
            return false;
        };

        let Some(node_id) = self
            .tracked_items
            .iter()
            .find(|item| item.group == target_group && item.id == id)
            .map(|item| item.node_id)
        else {
            return false;
        };

        self.reveal_node_in_folds(node_id)
    }

    pub fn set_tracked_items(&mut self, group: u32, raw_items: Vec<RawTrackedItem>) {
        let Some(target_group) = Self::tracked_item_group(group) else {
            return;
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

        let Some(target_group) = Self::tracked_item_group(group) else {
            return;
        };

        let id_set: HashSet<&str> = ids.iter().map(|id| id.as_str()).collect();

        self.tracked_items
            .retain(|item| item.group != target_group || !id_set.contains(item.id.as_str()));

        self.pending.tracked_items = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tracked_item_overlays_keeps_hidden_search_match_placeholder() {
        let mut fold = id!();
        let mut title = id!();
        let mut paragraph = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @fold fold {
                    @title fold_title {
                        text { "Title" }
                    }
                    fold_content {
                        @paragraph paragraph {
                            text { "needle" }
                        }
                    }
                }
            }
            selection { (title, 0) }
        };

        runtime.set_tracked_items(
            2,
            vec![RawTrackedItem {
                id: "search-0".to_string(),
                node_id: paragraph,
                start_offset: 0,
                end_offset: 6,
            }],
        );

        runtime.tick();

        let overlays =
            build_tracked_item_overlays(runtime.pages(), &runtime.tracked_items, runtime.doc());

        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].id, "search-0");
        assert_eq!(overlays[0].node_id, paragraph);
        assert!(overlays[0].bounds.is_empty());
    }

    #[test]
    fn reveal_tracked_item_opens_ancestor_folds_and_restores_bounds() {
        let mut outer_fold = id!();
        let mut outer_title = id!();
        let mut inner_fold = id!();
        let mut inner_title = id!();
        let mut paragraph = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @outer_fold fold {
                    @outer_title fold_title {
                        text { "Outer" }
                    }
                    fold_content {
                        @inner_fold fold {
                            @inner_title fold_title {
                                text { "Inner" }
                            }
                            fold_content {
                                @paragraph paragraph {
                                    text { "needle" }
                                }
                            }
                        }
                    }
                }
            }
            selection { (outer_title, 0) }
        };

        runtime.set_tracked_items(
            2,
            vec![RawTrackedItem {
                id: "search-0".to_string(),
                node_id: paragraph,
                start_offset: 0,
                end_offset: 6,
            }],
        );
        runtime.tick();

        assert!(!runtime.layout_engine.fold_expanded(outer_fold));
        assert!(!runtime.layout_engine.fold_expanded(inner_fold));

        assert!(runtime.reveal_tracked_item(2, "search-0"));

        runtime.tick();

        assert!(runtime.layout_engine.fold_expanded(outer_fold));
        assert!(runtime.layout_engine.fold_expanded(inner_fold));

        let overlays =
            build_tracked_item_overlays(runtime.pages(), &runtime.tracked_items, runtime.doc());

        assert_eq!(overlays.len(), 1);
        assert!(!overlays[0].bounds.is_empty());
    }
}

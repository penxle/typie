use super::geometry::LayoutRect;
use crate::layout::{Page, PositionedNode};
use crate::model::NodeId;
use crate::types::Point;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Default)]
pub(super) struct DebugFrame {
    pub(super) render_rects: Vec<LayoutRect>,
    pub(super) overflow_rects: Vec<LayoutRect>,
    pub(super) full_repaint: bool,
    pub(super) cache_reused: bool,
    pub(super) layout_rects: Vec<LayoutRect>,
    pub(super) full_relayout: bool,
    pub(super) layout_reused: bool,
}

#[derive(Default)]
pub(super) struct DiagnosticsState {
    layout_revision_by_page: FxHashMap<usize, u64>,
}

impl DiagnosticsState {
    pub(super) fn clear(&mut self) {
        self.layout_revision_by_page.clear();
    }

    pub(super) fn retain_pages(&mut self, valid_page_count: usize) {
        self.layout_revision_by_page
            .retain(|page_idx, _| *page_idx < valid_page_count);
    }

    pub(super) fn is_layout_revision_reused(&self, page_idx: usize, revision: u64) -> bool {
        self.layout_revision_by_page.get(&page_idx).copied() == Some(revision)
    }

    pub(super) fn mark_layout_revision(&mut self, page_idx: usize, revision: u64) {
        self.layout_revision_by_page.insert(page_idx, revision);
    }
}

pub(super) fn collect_layout_dirty_rects(
    page: &Page,
    recomputed_nodes: &FxHashSet<NodeId>,
) -> Vec<LayoutRect> {
    let mut collector = LayoutDirtyCollector::new(recomputed_nodes);
    collector.visit(&page.root, Point::zero(), false);
    collector.rects
}

struct LayoutDirtyCollector<'a> {
    recomputed_nodes: &'a FxHashSet<NodeId>,
    rects: Vec<LayoutRect>,
}

impl<'a> LayoutDirtyCollector<'a> {
    fn new(recomputed_nodes: &'a FxHashSet<NodeId>) -> Self {
        Self {
            recomputed_nodes,
            rects: Vec::new(),
        }
    }

    fn visit(&mut self, positioned: &PositionedNode, offset: Point, has_recomputed_ancestor: bool) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        let is_recomputed_self = positioned
            .node
            .element
            .as_ref()
            .and_then(|element| element.block_id())
            .is_some_and(|block_id| self.recomputed_nodes.contains(&block_id));

        if is_recomputed_self && !has_recomputed_ancestor {
            self.push_node_rect(positioned, pos);
        }

        let descendant_has_recomputed_ancestor = has_recomputed_ancestor || is_recomputed_self;
        if let Some(children) = &positioned.node.children {
            for child in children {
                self.visit(child, pos, descendant_has_recomputed_ancestor);
            }
        }
    }

    fn push_node_rect(&mut self, positioned: &PositionedNode, pos: Point) {
        if let Some(bounds) = LayoutRect::from_xywh(
            pos.x,
            pos.y,
            positioned.node.size.width,
            positioned.node.size.height,
        ) {
            self.rects.push(bounds);
        }
    }
}

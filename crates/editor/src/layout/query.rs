use std::collections::HashSet;

use crate::layout::{Page, PositionedNode};
use crate::model::{Doc, NodeId};
use crate::types::Point;

#[derive(Debug, Clone, Copy)]
pub struct NodeBounds {
    pub page_idx: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NodeBounds {
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn to_rect(&self) -> crate::types::Rect {
        crate::types::Rect::new(self.x, self.y, self.width, self.height)
    }
}

pub fn find_node_bounds(doc: &Doc, pages: &[Page], node_id: NodeId) -> Option<NodeBounds> {
    let targets = collect_leaf_ids(doc, node_id);
    if targets.is_empty() {
        return None;
    }

    scan_pages_for_bounds(pages, &targets)
}

pub fn find_node_bounds_on_page(
    doc: &Doc,
    page: &Page,
    node_id: NodeId,
    page_idx: usize,
) -> Option<NodeBounds> {
    let targets = collect_leaf_ids(doc, node_id);
    if targets.is_empty() {
        return None;
    }

    let mut acc = BoundsAccumulator::new();
    scan_layout_node(&page.root, &targets, Point::zero(), &mut acc);
    acc.to_bounds(page_idx)
}

fn collect_leaf_ids(doc: &Doc, root_id: NodeId) -> HashSet<NodeId> {
    let mut ids = HashSet::new();
    collect_recursive(doc, root_id, &mut ids);
    ids
}

fn collect_recursive(doc: &Doc, node_id: NodeId, ids: &mut HashSet<NodeId>) {
    ids.insert(node_id);

    if let Some(node) = doc.node(node_id) {
        for child in node.children() {
            collect_recursive(doc, child.node_id(), ids);
        }
    }
}

fn scan_pages_for_bounds(pages: &[Page], targets: &HashSet<NodeId>) -> Option<NodeBounds> {
    for (page_idx, page) in pages.iter().enumerate() {
        let mut acc = BoundsAccumulator::new();
        scan_layout_node(&page.root, targets, Point::zero(), &mut acc);

        if let Some(bounds) = acc.to_bounds(page_idx) {
            return Some(bounds);
        }
    }
    None
}

fn scan_layout_node(
    node: &PositionedNode,
    targets: &HashSet<NodeId>,
    offset: Point,
    acc: &mut BoundsAccumulator,
) {
    let abs_pos = Point::new(offset.x + node.position.x, offset.y + node.position.y);

    if let Some(element) = &node.node.element {
        if let Some(block_id) = element.block_id() {
            if targets.contains(&block_id) {
                acc.add_rect(
                    abs_pos.x,
                    abs_pos.y,
                    node.node.size.width,
                    node.node.size.height,
                );
            }
        }
    }

    if let Some(children) = &node.node.children {
        for child in children {
            scan_layout_node(child, targets, abs_pos, acc);
        }
    }
}

struct BoundsAccumulator {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    found: bool,
}

impl BoundsAccumulator {
    fn new() -> Self {
        Self {
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
            found: false,
        }
    }

    fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x + w);
        self.max_y = self.max_y.max(y + h);
        self.found = true;
    }

    fn to_bounds(&self, page_idx: usize) -> Option<NodeBounds> {
        if self.found {
            Some(NodeBounds {
                page_idx,
                x: self.min_x,
                y: self.min_y,
                width: self.max_x - self.min_x,
                height: self.max_y - self.min_y,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct DragImagePageBounds {
    pub page_idx: usize,
    pub bounds: crate::types::Rect,
    pub clip_rects: Vec<crate::types::Rect>,
}

#[derive(Debug, Clone)]
pub struct DragImageBounds {
    pub pages: Vec<DragImagePageBounds>,
}

pub fn find_drag_image_bounds(
    doc: &Doc,
    selection: &crate::state::Selection,
    pages: &[Page],
) -> Option<DragImageBounds> {
    let block_ids = collect_selected_block_ids(doc, selection)?;
    let all_ids = collect_all_descendant_ids(doc, &block_ids);
    let drag_pages = collect_page_bounds(doc, pages, &block_ids, &all_ids);

    if drag_pages.is_empty() {
        None
    } else {
        Some(DragImageBounds { pages: drag_pages })
    }
}

fn collect_selected_block_ids(
    doc: &Doc,
    selection: &crate::state::Selection,
) -> Option<Vec<NodeId>> {
    if selection.is_collapsed() {
        return None;
    }

    let (from, to) = selection.as_sorted(doc).ok()?;
    let block_ids = crate::state::collect_blocks_in_range(doc, from, to).ok()?;

    if block_ids.is_empty() {
        None
    } else {
        Some(block_ids)
    }
}

fn collect_all_descendant_ids(doc: &Doc, block_ids: &[NodeId]) -> HashSet<NodeId> {
    let mut all_ids = HashSet::new();
    for &block_id in block_ids {
        collect_recursive(doc, block_id, &mut all_ids);
    }
    all_ids
}

fn collect_page_bounds(
    doc: &Doc,
    pages: &[Page],
    block_ids: &[NodeId],
    all_ids: &HashSet<NodeId>,
) -> Vec<DragImagePageBounds> {
    let mut drag_pages = Vec::new();

    for (page_idx, page) in pages.iter().enumerate() {
        if let Some(page_bounds) =
            collect_single_page_bounds(doc, page, page_idx, block_ids, all_ids)
        {
            drag_pages.push(page_bounds);
        }
    }

    drag_pages
}

fn collect_single_page_bounds(
    doc: &Doc,
    page: &Page,
    page_idx: usize,
    block_ids: &[NodeId],
    all_ids: &HashSet<NodeId>,
) -> Option<DragImagePageBounds> {
    let mut acc = BoundsAccumulator::new();
    scan_layout_node(&page.root, all_ids, Point::zero(), &mut acc);

    let overall_bounds = acc.to_bounds(page_idx)?;

    let clip_rects: Vec<_> = block_ids
        .iter()
        .filter_map(|&block_id| {
            let block_ids_set = collect_leaf_ids(doc, block_id);
            let mut block_acc = BoundsAccumulator::new();
            scan_layout_node(&page.root, &block_ids_set, Point::zero(), &mut block_acc);
            block_acc.to_bounds(page_idx).map(|b| b.to_rect())
        })
        .collect();

    Some(DragImagePageBounds {
        page_idx,
        bounds: overall_bounds.to_rect(),
        clip_rects,
    })
}

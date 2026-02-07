use std::collections::HashSet;

use crate::layout::{Element, Page, PositionedNode};
use crate::model::{Doc, NodeId, SelectionDecor};
use crate::state::selection_helpers::build_selection_decorations;
use crate::types::{Point, Rect};

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
    if selection.is_collapsed() {
        return None;
    }

    let decorations = build_selection_decorations(doc, selection, None);
    if decorations.is_empty() {
        return None;
    }

    let drag_pages = collect_page_bounds_from_decorations(pages, &decorations);

    if drag_pages.is_empty() {
        None
    } else {
        Some(DragImageBounds { pages: drag_pages })
    }
}

fn collect_page_bounds_from_decorations(
    pages: &[Page],
    decorations: &[SelectionDecor],
) -> Vec<DragImagePageBounds> {
    let mut drag_pages = Vec::new();

    for (page_idx, page) in pages.iter().enumerate() {
        if let Some(page_bounds) = collect_single_page_selection_bounds(page, page_idx, decorations)
        {
            drag_pages.push(page_bounds);
        }
    }

    drag_pages
}

fn collect_single_page_selection_bounds(
    page: &Page,
    page_idx: usize,
    decorations: &[SelectionDecor],
) -> Option<DragImagePageBounds> {
    let mut clip_rects = Vec::new();
    scan_for_selection_bounds(&page.root, Point::zero(), decorations, &mut clip_rects);

    if clip_rects.is_empty() {
        return None;
    }

    let mut acc = BoundsAccumulator::new();
    for rect in &clip_rects {
        acc.add_rect(rect.x, rect.y, rect.width, rect.height);
    }

    let overall_bounds = acc.to_bounds(page_idx)?;

    Some(DragImagePageBounds {
        page_idx,
        bounds: overall_bounds.to_rect(),
        clip_rects,
    })
}

fn scan_for_selection_bounds(
    node: &PositionedNode,
    offset: Point,
    decorations: &[SelectionDecor],
    out: &mut Vec<Rect>,
) {
    let pos = Point::new(offset.x + node.position.x, offset.y + node.position.y);

    if let Some(ref element) = node.node.element {
        match element {
            Element::Line(line) => {
                let rects = line.compute_selection_rects(pos, decorations);
                out.extend(rects);
            }
            _ => {
                if let Some(block_id) = element.block_id() {
                    if decorations.iter().any(|d| d.node_id() == block_id) {
                        out.push(Rect::new(
                            pos.x,
                            pos.y,
                            node.node.size.width,
                            node.node.size.height,
                        ));
                    }
                }
            }
        }
    }

    if let Some(children) = &node.node.children {
        for child in children {
            scan_for_selection_bounds(child, pos, decorations, out);
        }
    }
}

use std::cmp::Ordering;
use std::collections::HashSet;

use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::{Element, Page, PositionedNode};
use crate::model::{Doc, NodeId, SelectionDecor};
use crate::state::selection_helpers::build_selection_decorations;
use crate::state::{compare_positions, position_in_selection};
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
    find_node_rect_on_page(doc, page, node_id).map(|rect| NodeBounds {
        page_idx,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    })
}

pub fn find_node_rect_on_page(doc: &Doc, page: &Page, node_id: NodeId) -> Option<Rect> {
    let targets = collect_leaf_ids(doc, node_id);
    if targets.is_empty() {
        return None;
    }

    let mut acc = BoundsAccumulator::new();
    scan_layout_node(&page.root, &targets, Point::zero(), &mut acc);

    if acc.found {
        Some(Rect::new(
            acc.min_x,
            acc.min_y,
            acc.max_x - acc.min_x,
            acc.max_y - acc.min_y,
        ))
    } else {
        None
    }
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
    let non_text_blocks = collect_selected_non_text_blocks(doc, selection);

    if decorations.is_empty() && non_text_blocks.is_empty() {
        return None;
    }

    let mut drag_pages = Vec::new();

    for (page_idx, page) in pages.iter().enumerate() {
        let rects =
            collect_page_selection_bounds(doc, page, selection, &decorations, &non_text_blocks);

        if !rects.is_empty() {
            let mut acc = BoundsAccumulator::new();
            for rect in &rects {
                acc.add_rect(rect.x, rect.y, rect.width, rect.height);
            }

            if let Some(overall_bounds) = acc.to_bounds(page_idx) {
                drag_pages.push(DragImagePageBounds {
                    page_idx,
                    bounds: overall_bounds.to_rect(),
                    clip_rects: rects,
                });
            }
        }
    }

    if drag_pages.is_empty() {
        None
    } else {
        Some(DragImageBounds { pages: drag_pages })
    }
}

fn collect_selected_non_text_blocks(doc: &Doc, selection: &crate::state::Selection) -> Vec<NodeId> {
    if let Ok((from, to)) = selection.as_sorted(doc) {
        if let Ok(blocks) = crate::state::selection_helpers::collect_blocks_in_range(doc, from, to)
        {
            return blocks
                .into_iter()
                .filter(|&id| {
                    doc.node(id)
                        .map(|n| !n.spec().is_textblock(doc.schema()))
                        .unwrap_or(false)
                })
                .collect();
        }
    }
    Vec::new()
}

fn collect_page_selection_bounds(
    doc: &Doc,
    page: &Page,
    _selection: &crate::state::Selection,
    decorations: &[SelectionDecor],
    non_text_blocks: &[NodeId],
) -> Vec<Rect> {
    let mut rects = Vec::new();

    if !decorations.is_empty() {
        scan_for_selection_bounds(&page.root, Point::zero(), decorations, &mut rects);
    }

    for &block_id in non_text_blocks {
        if let Some(rect) = find_node_rect_on_page(doc, page, block_id) {
            rects.push(rect);
        }
    }

    rects
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

pub fn is_point_in_selection_bounds(
    doc: &Doc,
    page: &Page,
    selection: &crate::state::Selection,
    point: Point,
) -> bool {
    if selection.is_collapsed() {
        return false;
    }

    let decorations = build_selection_decorations(doc, selection, None);
    let non_text_blocks = collect_selected_non_text_blocks(doc, selection);

    if decorations.is_empty() && non_text_blocks.is_empty() {
        return false;
    }

    let rects = collect_page_selection_bounds(doc, page, selection, &decorations, &non_text_blocks);

    for rect in rects {
        if point.x >= rect.x
            && point.x <= rect.x + rect.width
            && point.y >= rect.y
            && point.y <= rect.y + rect.height
        {
            return true;
        }
    }

    false
}

pub fn is_selection_hit(
    doc: &Doc,
    page: &Page,
    selection: &crate::state::Selection,
    x: f32,
    y: f32,
) -> bool {
    if selection.is_collapsed() {
        return false;
    }

    let ctx = NavigationContext::new(doc);
    let Some(hit_selection) = Cursor::hit_test(&ctx, page, x, y) else {
        return false;
    };

    let position = hit_selection.head;

    if is_selectable_block_hit(doc, &hit_selection) {
        if let (Ok((sel_from, sel_to)), Ok((hit_from, hit_to))) =
            (selection.as_sorted(doc), hit_selection.as_sorted(doc))
        {
            let start_ok = matches!(
                compare_positions(doc, sel_from, hit_from),
                Ok(Ordering::Less | Ordering::Equal)
            );
            let end_ok = matches!(
                compare_positions(doc, hit_to, sel_to),
                Ok(Ordering::Less | Ordering::Equal)
            );

            if start_ok && end_ok {
                return true;
            }
        }
    }

    if position_in_selection(doc, position, selection) {
        // position이 selection의 경계에 있는 경우 좌표가 selection bounds 안에 있는지 확인
        if let Ok((from, to)) = selection.as_sorted(doc) {
            let is_at_start = matches!(compare_positions(doc, from, position), Ok(Ordering::Equal));
            let is_at_end = matches!(compare_positions(doc, to, position), Ok(Ordering::Equal));

            if is_at_start || is_at_end {
                return is_point_in_selection_bounds(
                    doc,
                    page,
                    selection,
                    crate::types::Point::new(x, y),
                );
            }
        }
        return true;
    }

    false
}

pub fn is_selectable_block_hit(doc: &Doc, hit_selection: &crate::state::Selection) -> bool {
    use crate::state::position_helpers::find_child_at_offset;

    if hit_selection.is_collapsed() {
        return false;
    }

    let anchor = hit_selection.anchor;
    let Some(parent) = doc.node(anchor.node_id) else {
        return false;
    };

    let Some((child_id, _)) = find_child_at_offset(&parent, anchor.offset) else {
        return false;
    };

    doc.node(child_id)
        .map(|child| child.spec().selectable)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::elements::CalloutIconElement;
    use crate::layout::{Element, LayoutNode, Page, PageBreakPolicy, PositionedNode, RenderHints};
    use crate::model::{CalloutVariant, NodeId};
    use crate::types::{Point, Size};
    use std::rc::Rc;

    #[test]
    fn test_is_interactive_hit() {
        let icon_node_id = NodeId::new();
        let icon_element =
            CalloutIconElement::new(Size::new(20.0, 20.0), CalloutVariant::Info, icon_node_id);

        let icon_node = PositionedNode {
            position: Point::new(10.0, 10.0),
            node: Rc::new(LayoutNode {
                size: Size::new(20.0, 20.0),
                element: Some(Element::CalloutIcon(icon_element)),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        };

        let root_node = Rc::new(LayoutNode {
            size: Size::new(100.0, 100.0),
            element: None,
            children: Some(vec![icon_node]),
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        });

        let page_root = PositionedNode {
            position: Point::zero(),
            node: root_node,
        };

        let page = Page::from_root(page_root);

        assert!(
            page.find_interactive_at(15.0, 15.0).is_some(),
            "Should hit inside icon"
        );
        assert!(
            page.find_interactive_at(10.0, 10.0).is_some(),
            "Should hit top-left corner"
        );
        assert!(
            page.find_interactive_at(30.0, 30.0).is_some(),
            "Should hit bottom-right corner"
        );

        assert!(
            page.find_interactive_at(5.0, 5.0).is_none(),
            "Should not hit before icon"
        );
        assert!(
            page.find_interactive_at(31.0, 15.0).is_none(),
            "Should not hit right of icon"
        );
        assert!(
            page.find_interactive_at(15.0, 31.0).is_none(),
            "Should not hit below icon"
        );
    }
}

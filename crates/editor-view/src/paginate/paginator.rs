use editor_common::{EdgeInsets, Rect, Size};

use crate::style::Alignment;
use editor_model::NodeId;
use editor_state::Position;

use crate::measure::*;
use crate::page::LayoutPage;
use crate::page_fragment::build_page_fragment_trees;
use crate::style::*;

use super::types::*;

pub(crate) struct Paginator {
    paginated: bool,
    page_width: f32,
    page_height: f32,
    content_width: f32,
    content_height: f32,
    margins: EdgeInsets,
    accumulated_y: f32,
    current_x: f32,
    page_content_top: f32,
    page_content_bottom: f32,
    pages: Vec<LayoutPage>,
}

impl Paginator {
    pub fn paginated(page_width: f32, page_height: f32, margins: EdgeInsets) -> Self {
        let content_width = page_width - margins.left - margins.right;
        let content_height = page_height - margins.top - margins.bottom;
        Self {
            paginated: true,
            page_width,
            page_height,
            content_width,
            content_height,
            margins,
            accumulated_y: margins.top,
            current_x: margins.left,
            page_content_top: margins.top,
            page_content_bottom: margins.top + content_height,
            pages: vec![],
        }
    }

    pub fn continuous(page_width: f32, max_content_height: f32, margins: EdgeInsets) -> Self {
        let content_width = page_width - margins.left - margins.right;
        Self {
            paginated: false,
            page_width,
            page_height: 0.0,
            content_width,
            content_height: max_content_height,
            margins,
            accumulated_y: margins.top,
            current_x: margins.left,
            page_content_top: margins.top,
            page_content_bottom: margins.top + max_content_height,
            pages: vec![],
        }
    }

    pub fn paginate(mut self, tree: MeasuredTree) -> PaginatedLayout {
        let root = self.place_node(&tree.root, NodeId::ROOT, 0, 0.0);
        let pages = self.finish();
        let tree = LayoutTree { root };
        let page_fragments = build_page_fragment_trees(&tree, &pages);
        PaginatedLayout {
            tree,
            pages,
            page_fragments,
        }
    }

    fn place_node(
        &mut self,
        node: &MeasuredNode,
        parent_id: NodeId,
        child_index: usize,
        terminal_chrome_after: f32,
    ) -> LayoutNode {
        match &node.content {
            MeasuredContent::Box(b) => {
                let mut placed = match b.style.direction {
                    Direction::Vertical => {
                        self.place_vertical(b, node.width, terminal_chrome_after)
                    }
                    Direction::Horizontal => self.place_horizontal(b, node),
                };
                if b.style.monolithic
                    && let LayoutContent::Box(lb) = &mut placed.content
                {
                    lb.nav = Some(NavUnit {
                        parent_id,
                        index: child_index,
                    });
                }
                placed
            }
            MeasuredContent::Line(l) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += node.height;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, node.height),
                    content: LayoutContent::Line(LayoutLine {
                        node_id: l.node_id,
                        baseline: l.baseline,
                        ascent: l.ascent,
                        descent: l.descent,
                        cursor_ascent: l.cursor_ascent,
                        cursor_descent: l.cursor_descent,
                        glyph_runs: l.glyph_runs.clone(),
                        ruby_annotations: l.ruby_annotations.clone(),
                        empty_caret_x: l.empty_caret_x,
                        child_range: l.child_range.clone(),
                        tab_gaps: l.tab_gaps.clone(),
                        is_phantom: l.is_phantom,
                        content_edge_x: l.content_edge_x,
                    }),
                }
            }
            MeasuredContent::Atom(a) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += node.height;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, node.height),
                    content: LayoutContent::Atom(LayoutAtom {
                        node_id: a.node_id,
                        parent_id,
                        index: child_index,
                    }),
                }
            }
            MeasuredContent::Spacing(h) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += h;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, *h),
                    content: LayoutContent::Spacing(SpacingKind::Gap {
                        position: Position::new(parent_id, child_index),
                    }),
                }
            }
            MeasuredContent::PageBreak => LayoutNode {
                rect: Rect::from_xywh(0.0, self.accumulated_y, 0.0, 0.0),
                content: LayoutContent::Spacing(SpacingKind::Gap {
                    position: Position::new(parent_id, child_index),
                }),
            },
        }
    }

    fn place_vertical(
        &mut self,
        measured: &MeasuredBox,
        width: f32,
        terminal_chrome_after: f32,
    ) -> LayoutNode {
        let box_x = self.compute_box_x(measured, width);
        let box_y = self.accumulated_y;

        let collapse = measured.style.border_mode == BorderMode::Collapse;
        let lead_border_top = if collapse {
            0.0
        } else {
            measured.style.border.top
        };
        let lead_border_left = if collapse {
            0.0
        } else {
            measured.style.border.left
        };

        self.accumulated_y += lead_border_top + measured.style.padding.top;

        let old_x = self.current_x;
        self.current_x = box_x + lead_border_left + measured.style.padding.left;

        let mut children = Vec::new();
        let mut prev_border_bottom: Option<f32> = None;

        let mut child_index: usize = 0;
        let terminal_child_index = terminal_child_index(measured);

        for (raw_child_index, child) in measured.children.iter().enumerate() {
            let is_doc_child = !matches!(child.content, MeasuredContent::Spacing(_));

            // 1. Gap absorption at page start (paginated only)
            if self.is_paginated()
                && self.is_at_page_start()
                && matches!(child.content, MeasuredContent::Spacing(_))
            {
                continue;
            }

            // 2. Border collapse
            if measured.style.border_mode == BorderMode::Collapse
                && let Some(prev_bb) = prev_border_bottom
            {
                let child_bt = child_border_top(child);
                let overlap = prev_bb.min(child_bt);
                self.accumulated_y -= overlap;
            }

            // 3. PageBreak -> forced break
            if matches!(child.content, MeasuredContent::PageBreak) {
                if self.is_paginated() && !self.is_at_page_start() {
                    self.break_page(&mut children);
                }
                prev_border_bottom = None;
                child_index += 1;
                continue; // PageBreak consumed, not added to output
            }

            let child_terminal_chrome_after = if Some(raw_child_index) == terminal_child_index {
                terminal_chrome_after + trailing_chrome_height(measured)
            } else {
                0.0
            };

            // 4. Break check
            if self.should_break_before_child(child, child_terminal_chrome_after) {
                self.break_page(&mut children);
                // Absorb gap immediately after a forced page break
                if matches!(child.content, MeasuredContent::Spacing(_)) {
                    continue;
                }
            }

            // 5. Place child
            let layout_child = self.place_node(
                child,
                measured.node_id,
                child_index,
                child_terminal_chrome_after,
            );
            if is_doc_child {
                child_index += 1;
            }
            children.push(layout_child);

            prev_border_bottom = child_border_bottom(child);

            // 6. Oversized child: advance pages until the child fits
            let child_bottom = self.accumulated_y;
            if self.is_paginated() {
                while child_bottom > self.page_content_bottom() {
                    self.start_new_page();
                }
                if self.accumulated_y < child_bottom {
                    self.accumulated_y = child_bottom;
                }
            }
            if !self.is_paginated() && self.accumulated_y > self.page_content_bottom() {
                self.start_new_page();
            }
        }

        self.current_x = old_x;
        let trail_border_bottom = if collapse {
            0.0
        } else {
            measured.style.border.bottom
        };
        self.accumulated_y += measured.style.padding.bottom + trail_border_bottom;
        let box_height = self.accumulated_y - box_y;

        LayoutNode {
            rect: Rect::from_xywh(box_x, box_y, width, box_height),
            content: LayoutContent::Box(LayoutBox {
                node_id: measured.node_id,
                style: measured.style.clone(),
                children,
                nav: None,
            }),
        }
    }

    fn place_horizontal(&mut self, measured: &MeasuredBox, node: &MeasuredNode) -> LayoutNode {
        let box_x = self.compute_box_x(measured, node.width);
        let box_y = self.accumulated_y;

        let collapse = measured.style.border_mode == BorderMode::Collapse;
        let lead_border_left = if collapse {
            0.0
        } else {
            measured.style.border.left
        };
        let lead_border_top = if collapse {
            0.0
        } else {
            measured.style.border.top
        };

        let mut child_x = box_x + lead_border_left + measured.style.padding.left;
        let child_y = box_y + lead_border_top + measured.style.padding.top;

        let mut child_index: usize = 0;
        let children: Vec<LayoutNode> = measured
            .children
            .iter()
            .map(|child| {
                let is_doc_child = !matches!(child.content, MeasuredContent::Spacing(_));
                let layout_child =
                    place_node_at(child, child_x, child_y, measured.node_id, child_index);
                if is_doc_child {
                    child_index += 1;
                }
                child_x += child.width;
                if measured.style.border_mode == BorderMode::Collapse
                    && let MeasuredContent::Box(child_box) = &child.content
                {
                    child_x -= child_box.style.border.right;
                }
                layout_child
            })
            .collect();

        self.accumulated_y += node.height;

        LayoutNode {
            rect: Rect::from_xywh(box_x, box_y, node.width, node.height),
            content: LayoutContent::Box(LayoutBox {
                node_id: measured.node_id,
                style: measured.style.clone(),
                children,
                nav: None,
            }),
        }
    }

    fn break_page(&mut self, children: &mut Vec<LayoutNode>) {
        let fill_height = self.remaining();
        if fill_height > 0.0 {
            children.push(LayoutNode {
                rect: Rect::from_xywh(0.0, self.accumulated_y, 0.0, fill_height),
                content: LayoutContent::Spacing(SpacingKind::Fill),
            });
            self.accumulated_y += fill_height;
        }
        self.start_new_page();
    }

    fn compute_box_x(&self, measured: &MeasuredBox, width: f32) -> f32 {
        let base_x = self.current_x;
        match measured.style.alignment {
            Alignment::Start => base_x,
            Alignment::Center => base_x + (self.content_width - width) / 2.0,
            Alignment::End => base_x + self.content_width - width,
        }
    }

    fn remaining(&self) -> f32 {
        (self.page_content_bottom - self.accumulated_y).max(0.0)
    }

    fn is_at_page_start(&self) -> bool {
        (self.accumulated_y - self.page_content_top).abs() < f32::EPSILON
    }

    fn is_paginated(&self) -> bool {
        self.paginated
    }

    fn should_break_before_child(&self, child: &MeasuredNode, terminal_chrome_after: f32) -> bool {
        if !self.is_paginated() || self.is_at_page_start() {
            return false;
        }

        let remaining = self.remaining();

        if matches!(child.content, MeasuredContent::Spacing(_)) {
            return child.height > remaining;
        }

        initial_keep_height(child, terminal_chrome_after).is_some_and(|keep| keep > remaining)
    }

    fn page_content_bottom(&self) -> f32 {
        self.page_content_bottom
    }

    pub fn content_width(&self) -> f32 {
        self.content_width
    }

    fn page_width(&self) -> f32 {
        self.page_width
    }

    fn start_new_page(&mut self) {
        if self.paginated {
            let page_start = self.page_content_top - self.margins.top;
            let page_end = self.page_content_bottom + self.margins.bottom;
            self.pages.push(LayoutPage::with_content(
                page_start,
                page_end,
                self.page_content_top,
                self.page_content_bottom,
                Size::new(self.page_width(), self.page_height),
            ));
            self.page_content_top = page_end + self.margins.top;
            self.page_content_bottom = self.page_content_top + self.content_height;
            self.accumulated_y = self.page_content_top;
        } else {
            let is_first_page = self.pages.is_empty();
            let page_start = if is_first_page {
                self.page_content_top - self.margins.top
            } else {
                self.page_content_top
            };
            let page_end = self.accumulated_y;
            self.pages.push(LayoutPage::new(
                page_start,
                page_end,
                Size::new(self.page_width(), page_end - page_start),
            ));
            self.page_content_top = self.accumulated_y;
            self.page_content_bottom = self.page_content_top + self.content_height;
        }
    }

    fn finish(mut self) -> Vec<LayoutPage> {
        if self.accumulated_y > self.page_content_top {
            if self.paginated {
                let page_start = self.page_content_top - self.margins.top;
                let page_end = self.page_content_top + self.content_height + self.margins.bottom;
                self.pages.push(LayoutPage::with_content(
                    page_start,
                    page_end,
                    self.page_content_top,
                    self.page_content_top + self.content_height,
                    Size::new(self.page_width(), self.page_height),
                ));
            } else {
                let is_first_page = self.pages.is_empty();
                let page_start = if is_first_page {
                    self.page_content_top - self.margins.top
                } else {
                    self.page_content_top
                };
                let page_end = self.accumulated_y + self.margins.bottom;
                self.pages.push(LayoutPage::new(
                    page_start,
                    page_end,
                    Size::new(self.page_width(), page_end - page_start),
                ));
            }
        } else if self.pages.is_empty() {
            if self.paginated {
                self.pages.push(LayoutPage::with_content(
                    0.0,
                    self.page_height,
                    self.margins.top,
                    self.page_height - self.margins.bottom,
                    Size::new(self.page_width(), self.page_height),
                ));
            } else {
                self.pages.push(LayoutPage::new(
                    0.0,
                    self.margins.top + self.margins.bottom,
                    Size::new(self.page_width(), self.margins.top + self.margins.bottom),
                ));
            }
        } else if !self.paginated
            && let Some(page) = self.pages.last_mut()
        {
            page.y_end += self.margins.bottom;
            page.content_y_end += self.margins.bottom;
            page.size.height += self.margins.bottom;
        }
        self.pages
    }
}

fn child_border_top(node: &MeasuredNode) -> f32 {
    match &node.content {
        MeasuredContent::Box(b) => b.style.border.top,
        _ => 0.0,
    }
}

fn child_border_bottom(node: &MeasuredNode) -> Option<f32> {
    match &node.content {
        MeasuredContent::Box(b) => Some(b.style.border.bottom),
        _ => None,
    }
}

fn leading_chrome_height(b: &MeasuredBox) -> f32 {
    let border_top = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.top
    };
    border_top + b.style.padding.top
}

fn trailing_chrome_height(b: &MeasuredBox) -> f32 {
    let border_bottom = if b.style.border_mode == BorderMode::Collapse {
        0.0
    } else {
        b.style.border.bottom
    };
    b.style.padding.bottom + border_bottom
}

fn terminal_child_index(b: &MeasuredBox) -> Option<usize> {
    b.children.iter().rposition(|child| {
        !matches!(
            child.content,
            MeasuredContent::Spacing(_) | MeasuredContent::PageBreak
        )
    })
}

fn initial_child_index(b: &MeasuredBox) -> Option<usize> {
    b.children
        .iter()
        .position(|child| !matches!(child.content, MeasuredContent::Spacing(_)))
}

fn initial_keep_height(node: &MeasuredNode, terminal_chrome_after: f32) -> Option<f32> {
    if node.page_break_policy() == PageBreakPolicy::Avoid {
        return Some(node.height + terminal_chrome_after);
    }

    let MeasuredContent::Box(b) = &node.content else {
        return None;
    };
    if b.style.direction != Direction::Vertical {
        return None;
    }

    let first_child_index = initial_child_index(b)?;
    let child_terminal_chrome_after = if Some(first_child_index) == terminal_child_index(b) {
        terminal_chrome_after + trailing_chrome_height(b)
    } else {
        0.0
    };

    Some(
        leading_chrome_height(b)
            + initial_keep_height(&b.children[first_child_index], child_terminal_chrome_after)?,
    )
}

fn place_node_at(
    node: &MeasuredNode,
    x: f32,
    y: f32,
    parent_id: NodeId,
    child_index: usize,
) -> LayoutNode {
    match &node.content {
        MeasuredContent::Box(b) => {
            let mut offset_y = b.style.border.top + b.style.padding.top;
            let mut offset_x = b.style.border.left + b.style.padding.left;
            let children: Vec<LayoutNode> = match b.style.direction {
                Direction::Vertical => {
                    let mut idx: usize = 0;
                    b.children
                        .iter()
                        .map(|child| {
                            let is_doc_child =
                                !matches!(child.content, MeasuredContent::Spacing(_));
                            let c =
                                place_node_at(child, x + offset_x, y + offset_y, b.node_id, idx);
                            if is_doc_child {
                                idx += 1;
                            }
                            offset_y += child.height;
                            c
                        })
                        .collect()
                }
                Direction::Horizontal => {
                    let mut idx: usize = 0;
                    b.children
                        .iter()
                        .map(|child| {
                            let is_doc_child =
                                !matches!(child.content, MeasuredContent::Spacing(_));
                            let c =
                                place_node_at(child, x + offset_x, y + offset_y, b.node_id, idx);
                            if is_doc_child {
                                idx += 1;
                            }
                            offset_x += child.width;
                            c
                        })
                        .collect()
                }
            };
            LayoutNode {
                rect: Rect::from_xywh(x, y, node.width, node.height),
                content: LayoutContent::Box(LayoutBox {
                    node_id: b.node_id,
                    style: b.style.clone(),
                    children,
                    nav: if b.style.monolithic {
                        Some(NavUnit {
                            parent_id,
                            index: child_index,
                        })
                    } else {
                        None
                    },
                }),
            }
        }
        MeasuredContent::Line(l) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Line(LayoutLine {
                node_id: l.node_id,
                baseline: l.baseline,
                ascent: l.ascent,
                descent: l.descent,
                cursor_ascent: l.cursor_ascent,
                cursor_descent: l.cursor_descent,
                glyph_runs: l.glyph_runs.clone(),
                ruby_annotations: l.ruby_annotations.clone(),
                empty_caret_x: l.empty_caret_x,
                child_range: l.child_range.clone(),
                tab_gaps: l.tab_gaps.clone(),
                is_phantom: l.is_phantom,
                content_edge_x: l.content_edge_x,
            }),
        },
        MeasuredContent::Atom(a) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Atom(LayoutAtom {
                node_id: a.node_id,
                parent_id,
                index: child_index,
            }),
        },
        MeasuredContent::Spacing(h) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, *h),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent_id, child_index),
            }),
        },
        MeasuredContent::PageBreak => LayoutNode {
            rect: Rect::from_xywh(x, y, 0.0, 0.0),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(parent_id, child_index),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_macros::{doc, state};
    use editor_model::NodeId;
    use std::sync::Arc;

    use crate::measure::Measurer;
    use crate::query::layout_index::LayoutIndex;
    use crate::view::View;
    use crate::view_state::ViewState;

    fn make_line_with_id(node_id: NodeId, height: f32) -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 400.0,
            height,
            content: MeasuredContent::Line(MeasuredLine {
                node_id,
                baseline: height * 0.8,
                ascent: height * 0.7,
                descent: height * 0.1,
                cursor_ascent: height * 0.7,
                cursor_descent: height * 0.1,
                glyph_runs: vec![],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        })
    }

    fn make_line(height: f32) -> Arc<MeasuredNode> {
        make_line_with_id(NodeId::new(), height)
    }

    fn make_box(children: Vec<Arc<MeasuredNode>>) -> MeasuredNode {
        let height: f32 = children.iter().map(|c| c.height).sum();
        MeasuredNode {
            width: 400.0,
            height,
            content: MeasuredContent::Box(MeasuredBox {
                node_id: NodeId::ROOT,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                page_break_policy: PageBreakPolicy::Auto,
            }),
        }
    }

    fn make_spacing(height: f32) -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 0.0,
            height,
            content: MeasuredContent::Spacing(height),
        })
    }

    fn make_atom(height: f32) -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 400.0,
            height,
            content: MeasuredContent::Atom(MeasuredAtom {
                node_id: NodeId::new(),
            }),
        })
    }

    fn make_box_with_style(
        node_id: NodeId,
        children: Vec<Arc<MeasuredNode>>,
        padding: EdgeInsets,
        border: EdgeInsets,
        page_break_policy: PageBreakPolicy,
    ) -> Arc<MeasuredNode> {
        let children_height: f32 = children.iter().map(|c| c.height).sum();
        Arc::new(MeasuredNode {
            width: 400.0,
            height: children_height + padding.top + padding.bottom + border.top + border.bottom,
            content: MeasuredContent::Box(MeasuredBox {
                node_id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding,
                    border,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                page_break_policy,
            }),
        })
    }

    fn into_tree(root: MeasuredNode) -> MeasuredTree {
        MeasuredTree { root }
    }

    fn paginate_c(
        root: MeasuredNode,
        vw: f32,
        max_h: f32,
        margin: f32,
    ) -> (LayoutTree, Vec<LayoutPage>) {
        let paginated =
            Paginator::continuous(vw, max_h, EdgeInsets::all(margin)).paginate(into_tree(root));
        (paginated.tree, paginated.pages)
    }

    fn paginate_p(
        root: MeasuredNode,
        pw: f32,
        ph: f32,
        margins: EdgeInsets,
    ) -> (LayoutTree, Vec<LayoutPage>) {
        let paginated = Paginator::paginated(pw, ph, margins).paginate(into_tree(root));
        (paginated.tree, paginated.pages)
    }

    #[test]
    fn continuous_simple_single_page() {
        let root = make_box(vec![make_line(20.0), make_line(20.0)]);
        let (_, pages) = paginate_c(root, 440.0, 1024.0, 20.0);
        assert_eq!(pages.len(), 1);
        // y_start = 0 (first page includes margin_top)
        assert_eq!(pages[0].y_start, 0.0);
        // y_end = margin_top(20) + content(40) + margin_bottom(20) = 80
        assert_eq!(pages[0].y_end, 80.0);
    }

    #[test]
    fn continuous_final_overflow_page_keeps_bottom_margin() {
        let root = make_box(vec![make_line(1020.0), make_line(25.0)]);
        let (_, pages) = paginate_c(root, 440.0, 1024.0, 20.0);

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].y_start, 0.0);
        assert_eq!(pages[0].y_end, 1085.0);
        assert_eq!(pages[0].size.height, 1085.0);
    }

    #[test]
    fn root_box_positioned_at_margin() {
        let root = make_box(vec![make_line(20.0)]);
        let paginated = Paginator::continuous(
            1024.0,
            1024.0,
            EdgeInsets {
                top: 10.0,
                left: 15.0,
                bottom: 10.0,
                right: 15.0,
            },
        )
        .paginate(into_tree(root));
        let tree = paginated.tree;
        // Root box y should be at margin_top
        assert_eq!(tree.root.rect.y, 10.0);
        // Root box x should be at margin_left
        assert_eq!(tree.root.rect.x, 15.0);
    }

    #[test]
    fn line_inherits_current_x() {
        let root = make_box(vec![make_line(20.0)]);
        let paginated = Paginator::continuous(
            1024.0,
            1024.0,
            EdgeInsets {
                top: 10.0,
                left: 25.0,
                bottom: 10.0,
                right: 25.0,
            },
        )
        .paginate(into_tree(root));
        let tree = paginated.tree;
        // The line inside the box should have x = margin_left (box has no border/padding)
        if let LayoutContent::Box(b) = &tree.root.content {
            assert_eq!(b.children[0].rect.x, 25.0);
            assert_eq!(b.children[0].rect.y, 10.0);
        } else {
            panic!("expected box");
        }
    }

    #[test]
    fn paginated_single_page() {
        let root = make_box(vec![make_line(20.0)]);
        let (_, pages) = paginate_p(root, 440.0, 200.0, EdgeInsets::all(20.0));
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].y_start, 0.0);
        assert_eq!(pages[0].y_end, 200.0);
        assert_eq!(pages[0].size.height, 200.0);
        assert_eq!(pages[0].size.width, 440.0);
    }

    #[test]
    fn spacing_advances_y() {
        let root = make_box(vec![make_spacing(10.0), make_line(20.0)]);
        let paginated = Paginator::continuous(
            1024.0,
            1024.0,
            EdgeInsets {
                top: 10.0,
                left: 15.0,
                bottom: 10.0,
                right: 15.0,
            },
        )
        .paginate(into_tree(root));
        let tree = paginated.tree;
        if let LayoutContent::Box(b) = &tree.root.content {
            // spacing at y=10 (margin_top), height=10
            assert_eq!(b.children[0].rect.y, 10.0);
            assert_eq!(b.children[0].rect.height, 10.0);
            assert!(matches!(
                b.children[0].content,
                LayoutContent::Spacing(SpacingKind::Gap {
                    position
                }) if position == editor_state::Position::new(NodeId::ROOT, 0)
            ));
            // line at y=20 (after spacing)
            assert_eq!(b.children[1].rect.y, 20.0);
        } else {
            panic!("expected box");
        }
    }

    #[test]
    fn empty_document_produces_one_page() {
        let root = make_box(vec![]);
        let (_, pages) = paginate_c(root, 440.0, 1024.0, 20.0);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].y_start, 0.0);
        assert_eq!(pages[0].y_end, 40.0); // margin_top + margin_bottom
    }

    #[test]
    fn nested_box_with_padding() {
        let inner = Arc::new(MeasuredNode {
            width: 380.0,
            height: 30.0, // 5 (padding.top) + 20 (line) + 5 (padding.bottom)
            content: MeasuredContent::Box(MeasuredBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets {
                        top: 5.0,
                        left: 10.0,
                        bottom: 5.0,
                        right: 10.0,
                    },
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children: vec![make_line(20.0)],
                page_break_policy: PageBreakPolicy::Auto,
            }),
        });

        let root = MeasuredNode {
            width: 400.0,
            height: 30.0,
            content: MeasuredContent::Box(MeasuredBox {
                node_id: NodeId::ROOT,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children: vec![inner],
                page_break_policy: PageBreakPolicy::Auto,
            }),
        };

        let paginated = Paginator::continuous(
            1024.0,
            1024.0,
            EdgeInsets {
                top: 10.0,
                left: 15.0,
                bottom: 10.0,
                right: 15.0,
            },
        )
        .paginate(into_tree(root));
        let tree = paginated.tree;

        if let LayoutContent::Box(outer) = &tree.root.content {
            let inner_node = &outer.children[0];
            // inner box starts at y=10 (margin_top), x=15 (margin_left)
            assert_eq!(inner_node.rect.y, 10.0);
            assert_eq!(inner_node.rect.x, 15.0);

            if let LayoutContent::Box(inner_box) = &inner_node.content {
                let line = &inner_box.children[0];
                // line inside inner box: x = 15 (margin_left) + 10 (padding.left) = 25
                // y = 10 (margin_top) + 5 (padding.top) = 15
                assert_eq!(line.rect.x, 25.0);
                assert_eq!(line.rect.y, 15.0);
            } else {
                panic!("expected inner box");
            }
        } else {
            panic!("expected outer box");
        }
    }

    #[test]
    fn paginated_inserts_fill_on_break() {
        // Box containing atom(40) + atom(60), page content height = 90
        let root = make_box(vec![make_atom(40.0), make_atom(60.0)]);
        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);
        // Box should contain: atom(40), Fill, atom(60)
        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!()
        };
        let has_fill = root_box
            .children
            .iter()
            .any(|c| matches!(c.content, LayoutContent::Spacing(SpacingKind::Fill)));
        assert!(has_fill);
        // Box height should be inflated
        assert!(tree.root.rect.height > 100.0);
    }

    #[test]
    fn paginated_splits_box_children_before_moving_splittable_box() {
        // Page content height is 100. After the first line, 40px remain.
        // A paragraph-like box with 3 lines should use that remaining space
        // before splitting internally; it must not move wholesale to page 2.
        let paragraph = Arc::new(make_box(vec![
            make_line(20.0),
            make_line(20.0),
            make_line(20.0),
        ]));
        let root = make_box(vec![make_line(60.0), paragraph]);

        let (tree, pages) = paginate_p(root, 400.0, 120.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);

        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!("expected root box")
        };
        let paragraph_node = root_box
            .children
            .iter()
            .find(|child| matches!(child.content, LayoutContent::Box(_)))
            .expect("paragraph-like box");
        let LayoutContent::Box(paragraph_box) = &paragraph_node.content else {
            panic!("expected paragraph box")
        };
        let first_line = paragraph_box
            .children
            .iter()
            .find(|child| matches!(child.content, LayoutContent::Line(_)))
            .expect("first paragraph line");

        assert!(
            first_line.rect.y < pages[0].y_end,
            "first paragraph line should stay on page 1 when space remains; got y={} page_end={}",
            first_line.rect.y,
            pages[0].y_end
        );
    }

    #[test]
    fn paginated_keeps_leading_chrome_with_first_avoid_child() {
        // Page content height is 90. The first line leaves exactly 1px, enough
        // for the fold-like box's top border but not for its title. The box
        // should move as a group instead of leaving top chrome on page 1.
        let fold_id = NodeId::new();
        let title_id = NodeId::new();
        let title = make_box_with_style(
            title_id,
            vec![make_line(20.0)],
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            PageBreakPolicy::Avoid,
        );
        let fold = make_box_with_style(
            fold_id,
            vec![title],
            EdgeInsets::ZERO,
            EdgeInsets::all(1.0),
            PageBreakPolicy::Auto,
        );
        let root = make_box(vec![make_line(89.0), fold]);

        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);

        let fold_node = find_node(&tree.root, fold_id).expect("fold-like box");
        let title_node = find_node(&tree.root, title_id).expect("title box");
        assert!(
            fold_node.rect.y >= pages[1].content_y_start,
            "fold leading chrome must move with title; fold y={} page2_content_start={}",
            fold_node.rect.y,
            pages[1].content_y_start
        );
        assert!(
            title_node.rect.y >= pages[1].content_y_start,
            "title must start on the same page as its leading chrome; title y={} page2_content_start={}",
            title_node.rect.y,
            pages[1].content_y_start
        );
    }

    #[test]
    fn paginated_fold_keeps_top_border_with_title() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let fold = measurer.measure(&doc, f1, 400.0, &ViewState::new());
        let root = make_box(vec![make_line(89.0), fold]);

        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert!(pages.len() >= 2);

        let fold_node = find_node(&tree.root, f1).expect("fold box");
        assert!(
            fold_node.rect.y >= pages[1].content_y_start,
            "measured fold must not leave top border on the previous page; fold y={} page2_content_start={}",
            fold_node.rect.y,
            pages[1].content_y_start
        );
    }

    #[test]
    fn paginated_expanded_fold_does_not_reserve_own_bottom_for_title() {
        // The first line leaves exactly enough space for top border + title.
        // The fold bottom does not need to fit with the title because content
        // follows; reserving it for the title strands the top border.
        let fold_id = NodeId::new();
        let title_id = NodeId::new();
        let title = make_box_with_style(
            title_id,
            vec![make_line(20.0)],
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            PageBreakPolicy::Avoid,
        );
        let content = make_box_with_style(
            NodeId::new(),
            vec![make_line(20.0)],
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            PageBreakPolicy::Auto,
        );
        let fold = make_box_with_style(
            fold_id,
            vec![title, content],
            EdgeInsets::ZERO,
            EdgeInsets::all(1.0),
            PageBreakPolicy::Auto,
        );
        let root = make_box(vec![make_line(69.0), fold]);

        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert!(pages.len() >= 2);

        let fold_node = find_node(&tree.root, fold_id).expect("fold-like box");
        let title_node = find_node(&tree.root, title_id).expect("title box");
        assert!(
            title_node.rect.y < pages[0].content_y_end,
            "title should stay with the top border when content follows; title y={} page1_content_end={}",
            title_node.rect.y,
            pages[0].content_y_end
        );
        assert!(
            fold_node.rect.y < pages[0].content_y_end,
            "fold top border should stay on the same page as the title; fold y={} page1_content_end={}",
            fold_node.rect.y,
            pages[0].content_y_end
        );
    }

    #[test]
    fn paginated_does_not_reserve_outer_trailing_chrome_for_expanded_first_child() {
        // The wrapper has only one child, but that child is an expanded
        // fold-like box. The wrapper's trailing chrome belongs with the fold's
        // terminal content, not with the fold title.
        let fold_id = NodeId::new();
        let title_id = NodeId::new();
        let title = make_box_with_style(
            title_id,
            vec![make_line(20.0)],
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            PageBreakPolicy::Avoid,
        );
        let content = make_box_with_style(
            NodeId::new(),
            vec![make_line(20.0)],
            EdgeInsets::ZERO,
            EdgeInsets::ZERO,
            PageBreakPolicy::Auto,
        );
        let fold = make_box_with_style(
            fold_id,
            vec![title, content],
            EdgeInsets::ZERO,
            EdgeInsets::all(1.0),
            PageBreakPolicy::Auto,
        );
        let wrapper = make_box_with_style(
            NodeId::new(),
            vec![fold],
            EdgeInsets {
                bottom: 10.0,
                ..EdgeInsets::ZERO
            },
            EdgeInsets::ZERO,
            PageBreakPolicy::Auto,
        );
        let root = make_box(vec![make_line(69.0), wrapper]);

        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert!(pages.len() >= 2);

        let fold_node = find_node(&tree.root, fold_id).expect("fold-like box");
        let title_node = find_node(&tree.root, title_id).expect("title box");
        assert!(
            title_node.rect.y < pages[0].content_y_end,
            "title should not move just to reserve outer trailing chrome; title y={} page1_content_end={}",
            title_node.rect.y,
            pages[0].content_y_end
        );
        assert!(
            fold_node.rect.y < pages[0].content_y_end,
            "fold top border should stay with the title even when outer trailing chrome does not fit; fold y={} page1_content_end={}",
            fold_node.rect.y,
            pages[0].content_y_end
        );
    }

    #[test]
    fn paginated_keeps_trailing_chrome_with_last_avoid_child() {
        // Page content height is 90. Inside a splittable wrapper, the first
        // child leaves exactly 15px. The last line fits by itself, but not with
        // the wrapper's trailing padding+border, so the line should move.
        let wrapper_id = NodeId::new();
        let last_line_id = NodeId::new();
        let wrapper = make_box_with_style(
            wrapper_id,
            vec![make_line(30.0), make_line_with_id(last_line_id, 15.0)],
            EdgeInsets {
                bottom: 10.0,
                ..EdgeInsets::ZERO
            },
            EdgeInsets {
                bottom: 1.0,
                ..EdgeInsets::ZERO
            },
            PageBreakPolicy::Auto,
        );
        let root = make_box(vec![make_line(45.0), wrapper]);

        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);

        let last_line = find_line(&tree.root, last_line_id).expect("last avoid line");
        assert!(
            last_line.rect.y >= pages[1].content_y_start,
            "last avoid child must move with trailing chrome; line y={} page2_content_start={}",
            last_line.rect.y,
            pages[1].content_y_start
        );
    }

    #[test]
    fn paginated_splits_table_by_rows_instead_of_moving_table() {
        let (doc, t1) = doc! {
            root {
                paragraph { text("before") }
                t1: table {
                    table_row { table_cell { paragraph { text("A") } } }
                    table_row { table_cell { paragraph { text("B") } } }
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &ViewState::new());

        let paginated =
            Paginator::paginated(400.0, 130.0, EdgeInsets::all(10.0)).paginate(measured_tree(root));
        let tree = paginated.tree;
        let pages = paginated.pages;

        assert_eq!(pages.len(), 2);
        let table = find_node(&tree.root, t1).expect("table box in layout");
        let rows = box_children(table);
        assert_eq!(rows.len(), 2);
        assert!(
            rows[0].rect.y < pages[0].y_end,
            "first table row should stay on page 1 when space remains; got y={} page_end={}",
            rows[0].rect.y,
            pages[0].y_end
        );
        assert!(
            rows[1].rect.y >= pages[1].y_start,
            "last table row should move to page 2 after row-level split; got y={} page2_start={}",
            rows[1].rect.y,
            pages[1].y_start
        );
    }

    #[test]
    fn paginated_absorbs_gap_at_page_start() {
        // atom(80) + spacing(16) + atom(20), page content = 90
        let root = make_box(vec![make_atom(80.0), make_spacing(16.0), make_atom(20.0)]);
        let (tree, pages) = paginate_p(root, 400.0, 110.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);
        // After break, spacing should be absorbed -- page 2 starts with atom, not gap
        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!()
        };
        // Children should be: atom, Fill, atom (spacing absorbed)
        let non_fill: Vec<_> = root_box
            .children
            .iter()
            .filter(|c| !matches!(c.content, LayoutContent::Spacing(SpacingKind::Fill)))
            .collect();
        assert_eq!(non_fill.len(), 2); // just 2 atoms, spacing absorbed
    }

    #[test]
    fn oversized_atom_spans_multiple_pages() {
        let root = make_box(vec![make_atom(500.0)]);
        let (_, pages) = paginate_p(root, 400.0, 120.0, EdgeInsets::all(10.0));
        assert!(pages.len() >= 5); // 500 / 100 content = 5 pages
    }

    #[test]
    fn continuous_no_fill_inserted() {
        let root = make_box(vec![make_atom(800.0), make_spacing(16.0), make_atom(400.0)]);
        let (tree, _pages) = paginate_c(root, 400.0, 1024.0, 0.0);
        assert!(!has_fill(&tree.root));
        // All 3 children present (no absorption in continuous mode)
        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!()
        };
        assert_eq!(root_box.children.len(), 3);
    }

    #[test]
    fn pagebreak_forces_break() {
        let root = make_box(vec![
            make_atom(20.0),
            Arc::new(MeasuredNode {
                width: 0.0,
                height: 0.0,
                content: MeasuredContent::PageBreak,
            }),
            make_atom(20.0),
        ]);
        let (tree, pages) = paginate_p(root, 400.0, 200.0, EdgeInsets::all(10.0));
        assert_eq!(pages.len(), 2);
        // PageBreak should NOT appear in output, Fill should
        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!()
        };
        assert!(
            root_box
                .children
                .iter()
                .any(|c| matches!(c.content, LayoutContent::Spacing(SpacingKind::Fill)))
        );
    }

    // Match production: `view.rs::compute` unwraps the `Arc` before handing
    // the tree to the paginator.
    fn measured_tree(root: Arc<MeasuredNode>) -> MeasuredTree {
        MeasuredTree {
            root: Arc::unwrap_or_clone(root),
        }
    }

    // Fill lands in whichever container's local children fired `break_page`,
    // which is not necessarily the root box — recurse to decouple the test
    // from that nesting.
    fn has_fill(node: &LayoutNode) -> bool {
        match &node.content {
            LayoutContent::Spacing(SpacingKind::Fill) => true,
            LayoutContent::Box(b) => b.children.iter().any(has_fill),
            _ => false,
        }
    }

    #[test]
    fn trailing_page_break_forces_paginated_break_via_measure_pipeline() {
        let (doc, _p1, _p2) = doc! {
            root {
                p1: paragraph { text("a") page_break }
                p2: paragraph { text("b") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);

        let paginated = Paginator::paginated(400.0, 1000.0, EdgeInsets::all(10.0))
            .paginate(measured_tree(root));
        let tree = paginated.tree;
        let pages = paginated.pages;

        assert_eq!(
            pages.len(),
            2,
            "trailing page_break must force a page break in paginated mode",
        );
        assert!(
            has_fill(&tree.root),
            "paginated forced break should emit a Fill spacing somewhere in the layout tree",
        );
    }

    #[test]
    fn trailing_page_break_does_not_break_in_continuous_mode() {
        let (doc, _p1, _p2) = doc! {
            root {
                p1: paragraph { text("a") page_break }
                p2: paragraph { text("b") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);

        let paginated = Paginator::continuous(400.0, 1024.0, EdgeInsets::all(10.0))
            .paginate(measured_tree(root));
        let tree = paginated.tree;
        let pages = paginated.pages;

        assert_eq!(
            pages.len(),
            1,
            "continuous mode must ignore the page_break marker",
        );
        assert!(
            !has_fill(&tree.root),
            "continuous mode must not emit Fill spacing for a page_break marker",
        );
    }

    #[test]
    fn trailing_page_break_in_middle_paragraph_routes_following_paragraphs_to_next_page() {
        // The page is large enough to hold all three paragraphs vertically;
        // the only thing that splits the document into two pages is p2's
        // trailing `page_break`. This verifies multi-paragraph routing: the
        // marker pushes the *following* paragraph(s) to the next page, not
        // just one paragraph at a time.
        let (doc, _p1, _p2, _p3) = doc! {
            root {
                p1: paragraph { text("a") }
                p2: paragraph { text("b") page_break }
                p3: paragraph { text("c") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);

        let paginated = Paginator::paginated(400.0, 1000.0, EdgeInsets::all(10.0))
            .paginate(measured_tree(root));
        let tree = paginated.tree;
        let pages = paginated.pages;

        assert_eq!(
            pages.len(),
            2,
            "p1 and p2 share page 1; p2's trailing page_break sends p3 to page 2",
        );

        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!("expected root box")
        };
        // The default root modifiers inject `BlockGap` between paragraphs,
        // which materialises as `Spacing` children of the root box, so the
        // raw children count is not 3. Filter to paragraph LayoutBoxes.
        let paragraph_boxes: Vec<&LayoutNode> = root_box
            .children
            .iter()
            .filter(|c| matches!(c.content, LayoutContent::Box(_)))
            .collect();
        assert_eq!(
            paragraph_boxes.len(),
            3,
            "all three paragraph LayoutBoxes must be present in the root box",
        );
        let LayoutContent::Box(p2_box) = &paragraph_boxes[1].content else {
            panic!("expected p2 to be a Box")
        };
        let p2_has_fill = p2_box
            .children
            .iter()
            .any(|c| matches!(c.content, LayoutContent::Spacing(SpacingKind::Fill)));
        assert!(
            p2_has_fill,
            "p2's trailing page_break must emit a Fill inside p2's LayoutBox",
        );
    }

    #[test]
    fn page_break_only_paragraph_breaks_after_strut_line() {
        let (doc, _p1, _p2) = doc! {
            root {
                p1: paragraph { page_break }
                p2: paragraph { text("a") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &vs);

        let paginated = Paginator::paginated(400.0, 1000.0, EdgeInsets::all(10.0))
            .paginate(measured_tree(root));
        let tree = paginated.tree;
        let pages = paginated.pages;

        assert_eq!(
            pages.len(),
            2,
            "page_break-only paragraph must emit a strut-only line and then break",
        );

        // The first paragraph LayoutBox contains exactly one Line (strut-only)
        // before the page break — the PageBreak marker itself is consumed, not
        // emitted into the layout tree.
        let LayoutContent::Box(root_box) = &tree.root.content else {
            panic!("expected root box")
        };
        let LayoutContent::Box(p1_box) = &root_box.children[0].content else {
            panic!("expected p1 to be a Box")
        };
        let p1_line_count = p1_box
            .children
            .iter()
            .filter(|c| matches!(c.content, LayoutContent::Line(_)))
            .count();
        assert_eq!(
            p1_line_count, 1,
            "p1 must contribute exactly one strut-only line",
        );
    }

    fn find_node(node: &LayoutNode, id: NodeId) -> Option<&LayoutNode> {
        if let LayoutContent::Box(b) = &node.content {
            if b.node_id == id {
                return Some(node);
            }
            for c in &b.children {
                if let Some(f) = find_node(c, id) {
                    return Some(f);
                }
            }
        }
        None
    }

    fn find_line(node: &LayoutNode, id: NodeId) -> Option<&LayoutNode> {
        match &node.content {
            LayoutContent::Line(l) if l.node_id == id => Some(node),
            LayoutContent::Box(b) => b.children.iter().find_map(|c| find_line(c, id)),
            _ => None,
        }
    }

    fn box_children(node: &LayoutNode) -> Vec<&LayoutNode> {
        let LayoutContent::Box(b) = &node.content else {
            panic!("expected box");
        };
        b.children
            .iter()
            .filter(|c| matches!(c.content, LayoutContent::Box(_)))
            .collect()
    }

    #[test]
    fn collapsed_table_borders_do_not_stack() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row {
                        table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            }
        };

        let mut measurer = Measurer::new_test();
        let root = measurer.measure(&doc, NodeId::ROOT, 500.0, &ViewState::new());
        let tree = into_tree(Arc::unwrap_or_clone(root));
        let paginated = Paginator::continuous(540.0, 1024.0, EdgeInsets::all(20.0)).paginate(tree);
        let layout = paginated.tree;

        let table = find_node(&layout.root, t1).expect("table box in layout");
        let rows = box_children(table);
        assert_eq!(rows.len(), 2, "two rows");
        let row0 = rows[0];
        let last_row = rows[rows.len() - 1];
        let cells0 = box_children(row0);
        let cell0 = cells0[0];
        let last_cell0 = cells0[cells0.len() - 1];

        let eps = 1e-3;
        assert!(
            (row0.rect.y - table.rect.y).abs() < eps,
            "row top must coincide with table top (got row.y={}, table.y={})",
            row0.rect.y,
            table.rect.y
        );
        assert!(
            (row0.rect.x - table.rect.x).abs() < eps,
            "row left must coincide with table left (got row.x={}, table.x={})",
            row0.rect.x,
            table.rect.x
        );
        assert!(
            (cell0.rect.y - table.rect.y).abs() < eps,
            "cell top must coincide with table top (got cell.y={}, table.y={})",
            cell0.rect.y,
            table.rect.y
        );
        assert!(
            (cell0.rect.x - table.rect.x).abs() < eps,
            "cell left must coincide with table left (got cell.x={}, table.x={})",
            cell0.rect.x,
            table.rect.x
        );
        assert!(
            ((table.rect.y + table.rect.height) - (last_row.rect.y + last_row.rect.height)).abs()
                < eps,
            "table bottom must coincide with last row bottom (got table_bottom={}, row_bottom={})",
            table.rect.y + table.rect.height,
            last_row.rect.y + last_row.rect.height
        );
        assert!(
            ((last_cell0.rect.x + last_cell0.rect.width) - (table.rect.x + table.rect.width)).abs()
                < eps,
            "last cell right must coincide with table right (got cell_right={}, table_right={})",
            last_cell0.rect.x + last_cell0.rect.width,
            table.rect.x + table.rect.width
        );
    }

    fn nav_of(tree: &LayoutTree, pages: &[LayoutPage], id: NodeId) -> Option<NavUnit> {
        let layout_index = LayoutIndex::new(tree.clone(), pages);
        match &layout_index.box_entry(id)?.content(&layout_index)? {
            LayoutContent::Box(b) => b.nav,
            _ => None,
        }
    }

    #[test]
    fn monolithic_box_carries_nav_linkage() {
        let (st, r, f, ..) = state! {
            doc { r: root {
                f: fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph { t: text("after") }
            } }
            selection: (t, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let tree = view.layout_tree_for_test().expect("laid out");
        let pages = view.pages();
        assert_eq!(
            nav_of(tree, pages, f),
            Some(NavUnit {
                parent_id: r,
                index: 0
            })
        );
    }

    #[test]
    fn monolithic_box_in_table_cell_carries_nav_linkage() {
        let (st, tc, f, ..) = state! {
            doc { root {
                table { table_row { tc: table_cell {
                    f: fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                } } }
                paragraph { t: text("after") }
            } }
            selection: (t, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let tree = view.layout_tree_for_test().expect("laid out");
        let pages = view.pages();
        assert_eq!(
            nav_of(tree, pages, f),
            Some(NavUnit {
                parent_id: tc,
                index: 0
            }),
            "fold nested in a table_cell must link to its cell parent at index 0 (place_node_at path)"
        );
    }
}

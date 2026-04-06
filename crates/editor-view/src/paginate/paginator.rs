use editor_common::{Alignment, EdgeInsets, Rect, Size};

use crate::measure::*;
use crate::page::LayoutPage;
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

    pub fn paginate(mut self, tree: MeasuredTree) -> (LayoutTree, Vec<LayoutPage>) {
        let root = self.place_node(&tree.root);
        let pages = self.finish();
        (LayoutTree { root }, pages)
    }

    fn place_node(&mut self, node: &MeasuredNode) -> LayoutNode {
        match &node.content {
            MeasuredContent::Box(b) => match b.style.direction {
                Direction::Vertical => self.place_vertical(b, node.width),
                Direction::Horizontal => self.place_horizontal(b, node),
            },
            MeasuredContent::Line(l) => {
                let y = self.accumulated_y;
                let x = self.current_x;
                self.accumulated_y += node.height;
                LayoutNode {
                    rect: Rect::from_xywh(x, y, node.width, node.height),
                    content: LayoutContent::Line(LayoutLine {
                        node_id: l.node_id,
                        baseline: l.baseline,
                        glyph_runs: l.glyph_runs.clone(),
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
                        parent_id: a.parent_id,
                        index: a.index,
                    }),
                }
            }
            MeasuredContent::Spacing(h) => {
                let y = self.accumulated_y;
                self.accumulated_y += h;
                LayoutNode {
                    rect: Rect::from_xywh(0.0, y, 0.0, *h),
                    content: LayoutContent::Spacing(SpacingKind::Gap),
                }
            }
            MeasuredContent::PageBreak => LayoutNode {
                rect: Rect::from_xywh(0.0, self.accumulated_y, 0.0, 0.0),
                content: LayoutContent::Spacing(SpacingKind::Gap),
            },
        }
    }

    fn place_vertical(&mut self, measured: &MeasuredBox, width: f32) -> LayoutNode {
        let box_x = self.compute_box_x(measured, width);
        let box_y = self.accumulated_y;

        self.accumulated_y += measured.style.border.top + measured.style.padding.top;

        let old_x = self.current_x;
        self.current_x = box_x + measured.style.border.left + measured.style.padding.left;

        let mut children = Vec::new();
        let mut prev_border_bottom: Option<f32> = None;

        for child in &measured.children {
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
                continue; // PageBreak consumed, not added to output
            }

            // 4. Break check
            if self.is_paginated()
                && child.height > self.remaining()
                && child.height <= self.page_content_height()
            {
                self.break_page(&mut children);
                // Absorb gap immediately after a forced page break
                if matches!(child.content, MeasuredContent::Spacing(_)) {
                    continue;
                }
            }

            // 5. Place child
            let layout_child = self.place_node(child);
            children.push(layout_child);

            prev_border_bottom = child_border_bottom(child);

            // 6. Oversized child: advance pages until the child fits, without resetting accumulated_y
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
        self.accumulated_y += measured.style.padding.bottom + measured.style.border.bottom;
        let box_height = self.accumulated_y - box_y;

        LayoutNode {
            rect: Rect::from_xywh(box_x, box_y, width, box_height),
            content: LayoutContent::Box(LayoutBox {
                node_id: measured.node_id,
                style: measured.style.clone(),
                children,
            }),
        }
    }

    fn place_horizontal(&mut self, measured: &MeasuredBox, node: &MeasuredNode) -> LayoutNode {
        let box_x = self.compute_box_x(measured, node.width);
        let box_y = self.accumulated_y;

        let mut child_x = box_x + measured.style.border.left + measured.style.padding.left;
        let child_y = box_y + measured.style.border.top + measured.style.padding.top;

        let children: Vec<LayoutNode> = measured
            .children
            .iter()
            .map(|child| {
                let layout_child = place_node_at(child, child_x, child_y);
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

    fn page_content_height(&self) -> f32 {
        self.content_height
    }

    fn is_paginated(&self) -> bool {
        self.paginated
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
            self.pages.push(LayoutPage {
                y_start: page_start,
                y_end: page_end,
                size: Size::new(self.page_width(), self.page_height),
            });
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
            self.pages.push(LayoutPage {
                y_start: page_start,
                y_end: page_end,
                size: Size::new(self.page_width(), page_end - page_start),
            });
            self.page_content_top = self.accumulated_y;
            self.page_content_bottom = self.page_content_top + self.content_height;
        }
    }

    fn finish(mut self) -> Vec<LayoutPage> {
        if self.accumulated_y > self.page_content_top {
            if self.paginated {
                let page_start = self.page_content_top - self.margins.top;
                let page_end = self.page_content_top + self.content_height + self.margins.bottom;
                self.pages.push(LayoutPage {
                    y_start: page_start,
                    y_end: page_end,
                    size: Size::new(self.page_width(), self.page_height),
                });
            } else {
                let is_first_page = self.pages.is_empty();
                let page_start = if is_first_page {
                    self.page_content_top - self.margins.top
                } else {
                    self.page_content_top
                };
                let page_end = self.accumulated_y + self.margins.bottom;
                self.pages.push(LayoutPage {
                    y_start: page_start,
                    y_end: page_end,
                    size: Size::new(self.page_width(), page_end - page_start),
                });
            }
        } else if self.pages.is_empty() {
            if self.paginated {
                self.pages.push(LayoutPage {
                    y_start: 0.0,
                    y_end: self.page_height,
                    size: Size::new(self.page_width(), self.page_height),
                });
            } else {
                self.pages.push(LayoutPage {
                    y_start: 0.0,
                    y_end: self.margins.top + self.margins.bottom,
                    size: Size::new(self.page_width(), self.margins.top + self.margins.bottom),
                });
            }
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

fn place_node_at(node: &MeasuredNode, x: f32, y: f32) -> LayoutNode {
    match &node.content {
        MeasuredContent::Box(b) => {
            let mut offset_y = b.style.border.top + b.style.padding.top;
            let mut offset_x = b.style.border.left + b.style.padding.left;
            let children: Vec<LayoutNode> = match b.style.direction {
                Direction::Vertical => b
                    .children
                    .iter()
                    .map(|child| {
                        let c = place_node_at(child, x + offset_x, y + offset_y);
                        offset_y += child.height;
                        c
                    })
                    .collect(),
                Direction::Horizontal => b
                    .children
                    .iter()
                    .map(|child| {
                        let c = place_node_at(child, x + offset_x, y + offset_y);
                        offset_x += child.width;
                        c
                    })
                    .collect(),
            };
            LayoutNode {
                rect: Rect::from_xywh(x, y, node.width, node.height),
                content: LayoutContent::Box(LayoutBox {
                    node_id: b.node_id,
                    style: b.style.clone(),
                    children,
                }),
            }
        }
        MeasuredContent::Line(l) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Line(LayoutLine {
                node_id: l.node_id,
                baseline: l.baseline,
                glyph_runs: l.glyph_runs.clone(),
            }),
        },
        MeasuredContent::Atom(a) => LayoutNode {
            rect: Rect::from_xywh(x, y, node.width, node.height),
            content: LayoutContent::Atom(LayoutAtom {
                node_id: a.node_id,
                parent_id: a.parent_id,
                index: a.index,
            }),
        },
        MeasuredContent::Spacing(h) => LayoutNode {
            rect: Rect::from_xywh(x, y, 0.0, *h),
            content: LayoutContent::Spacing(SpacingKind::Gap),
        },
        MeasuredContent::PageBreak => LayoutNode {
            rect: Rect::from_xywh(x, y, 0.0, 0.0),
            content: LayoutContent::Spacing(SpacingKind::Gap),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_model::NodeId;
    use std::sync::Arc;

    fn make_line(height: f32) -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 400.0,
            height,
            content: MeasuredContent::Line(MeasuredLine {
                node_id: NodeId::new(),
                baseline: height * 0.8,
                glyph_runs: vec![],
            }),
        })
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
                    scope: false,
                    decorations: vec![],
                },
                children,
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
                parent_id: NodeId::ROOT,
                index: 0,
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
        Paginator::continuous(vw, max_h, EdgeInsets::all(margin)).paginate(into_tree(root))
    }

    fn paginate_p(
        root: MeasuredNode,
        pw: f32,
        ph: f32,
        margins: EdgeInsets,
    ) -> (LayoutTree, Vec<LayoutPage>) {
        Paginator::paginated(pw, ph, margins).paginate(into_tree(root))
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
    fn root_box_positioned_at_margin() {
        let root = make_box(vec![make_line(20.0)]);
        let (tree, _) = Paginator::continuous(
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
        // Root box y should be at margin_top
        assert_eq!(tree.root.rect.y, 10.0);
        // Root box x should be at margin_left
        assert_eq!(tree.root.rect.x, 15.0);
    }

    #[test]
    fn line_inherits_current_x() {
        let root = make_box(vec![make_line(20.0)]);
        let (tree, _) = Paginator::continuous(
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
        let (tree, _) = Paginator::continuous(
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
        if let LayoutContent::Box(b) = &tree.root.content {
            // spacing at y=10 (margin_top), height=10
            assert_eq!(b.children[0].rect.y, 10.0);
            assert_eq!(b.children[0].rect.height, 10.0);
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
                    scope: false,
                    decorations: vec![],
                },
                children: vec![make_line(20.0)],
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
                    scope: false,
                    decorations: vec![],
                },
                children: vec![inner],
            }),
        };

        let (tree, _) = Paginator::continuous(
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
        fn has_fill(node: &LayoutNode) -> bool {
            match &node.content {
                LayoutContent::Spacing(SpacingKind::Fill) => true,
                LayoutContent::Box(b) => b.children.iter().any(has_fill),
                _ => false,
            }
        }
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
}

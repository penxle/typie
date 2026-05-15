use editor_common::Rect;
use editor_model::NodeId;

use crate::glyph_run::GlyphRun;
use crate::page::LayoutPage;
use crate::paginate::*;
use crate::style::{BoxStyle, DecorationData};

#[derive(Debug, Clone, Copy)]
pub struct Edges<T> {
    pub top: T,
    pub bottom: T,
    pub left: T,
    pub right: T,
}

#[derive(Debug, Clone, Copy)]
pub struct LineMetrics {
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
}

pub trait PageVisitor {
    fn box_enter(
        &mut self,
        node_id: NodeId,
        local_rect: Rect,
        style: &BoxStyle,
        edges: Edges<bool>,
    );
    fn box_exit(&mut self);
    fn line(
        &mut self,
        node_id: NodeId,
        local_rect: Rect,
        metrics: LineMetrics,
        glyph_runs: &[GlyphRun],
    );
    fn atom(&mut self, node_id: NodeId, local_rect: Rect);
    fn decoration(&mut self, local_rect: Rect, data: &DecorationData);
}

pub fn visit_page(tree: &LayoutTree, page: &LayoutPage, visitor: &mut impl PageVisitor) {
    visit_node(&tree.root, page, visitor);
}

fn visit_node(node: &LayoutNode, page: &LayoutPage, visitor: &mut impl PageVisitor) {
    let node_top = node.rect.y;
    let node_bottom = node.rect.y + node.rect.height;

    if node_bottom <= page.y_start || node_top >= page.y_end {
        return;
    }

    let local_rect = Rect::from_xywh(
        node.rect.x,
        node.rect.y - page.y_start,
        node.rect.width,
        node.rect.height,
    );

    match &node.content {
        LayoutContent::Box(b) => {
            // A border edge is visible only if it falls within the current page's y-range
            let edges = Edges {
                top: node_top >= page.y_start,
                bottom: node_bottom <= page.y_end,
                left: true,
                right: true,
            };

            visitor.box_enter(b.node_id, local_rect, &b.style, edges);

            for dec in &b.style.decorations {
                let dec_abs_y = node_top + dec.rect.y;
                let dec_abs_bottom = dec_abs_y + dec.rect.height;
                if dec_abs_bottom > page.y_start && dec_abs_y < page.y_end {
                    let dec_local = Rect::from_xywh(
                        node.rect.x + dec.rect.x,
                        dec_abs_y - page.y_start,
                        dec.rect.width,
                        dec.rect.height,
                    );
                    visitor.decoration(dec_local, &dec.data);
                }
            }

            for child in &b.children {
                visit_node(child, page, visitor);
            }

            visitor.box_exit();
        }
        LayoutContent::Line(l) => {
            let metrics = LineMetrics {
                baseline: l.baseline,
                ascent: l.ascent,
                descent: l.descent,
            };
            visitor.line(l.node_id, local_rect, metrics, &l.glyph_runs);
        }
        LayoutContent::Atom(a) => {
            visitor.atom(a.node_id, local_rect);
        }
        LayoutContent::Spacing(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Alignment;
    use editor_common::{EdgeInsets, Size};
    use editor_model::NodeId;

    use crate::style::*;

    struct MockVisitor {
        box_enter_count: usize,
        box_exit_count: usize,
        line_count: usize,
        atom_count: usize,
        decoration_count: usize,
        last_edges: Option<Edges<bool>>,
        line_local_rects: Vec<Rect>,
        last_line_metrics: Option<LineMetrics>,
    }

    impl MockVisitor {
        fn new() -> Self {
            Self {
                box_enter_count: 0,
                box_exit_count: 0,
                line_count: 0,
                atom_count: 0,
                decoration_count: 0,
                last_edges: None,
                line_local_rects: vec![],
                last_line_metrics: None,
            }
        }
    }

    impl PageVisitor for MockVisitor {
        fn box_enter(&mut self, _: NodeId, _: Rect, _: &BoxStyle, edges: Edges<bool>) {
            self.box_enter_count += 1;
            self.last_edges = Some(edges);
        }

        fn box_exit(&mut self) {
            self.box_exit_count += 1;
        }

        fn line(&mut self, _: NodeId, local_rect: Rect, metrics: LineMetrics, _: &[GlyphRun]) {
            self.line_count += 1;
            self.line_local_rects.push(local_rect);
            self.last_line_metrics = Some(metrics);
        }

        fn atom(&mut self, _: NodeId, _: Rect) {
            self.atom_count += 1;
        }

        fn decoration(&mut self, _: Rect, _: &DecorationData) {
            self.decoration_count += 1;
        }
    }

    fn make_layout_line(node_id: NodeId, y: f32, height: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(20.0, y, 400.0, height),
            content: LayoutContent::Line(LayoutLine {
                node_id,
                baseline: height * 0.8,
                ascent: height * 0.7,
                descent: height * 0.1,
                cursor_ascent: height * 0.7,
                cursor_descent: height * 0.1,
                glyph_runs: vec![],
                text_indent: 0.0,
                child_range: None,
            }),
        }
    }

    fn make_layout_box(
        node_id: NodeId,
        y: f32,
        height: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(20.0, y, 400.0, height),
            content: LayoutContent::Box(LayoutBox {
                node_id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
            }),
        }
    }

    #[test]
    fn visits_visible_nodes_only() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let id3 = NodeId::new();
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                0.0,
                300.0,
                vec![
                    make_layout_line(id1, 10.0, 20.0),  // 10-30: outside page [50, 150]
                    make_layout_line(id2, 80.0, 20.0),  // 80-100: visible
                    make_layout_line(id3, 120.0, 20.0), // 120-140: visible
                ],
            ),
        };
        let page = LayoutPage {
            y_start: 50.0,
            y_end: 150.0,
            size: Size::new(440.0, 100.0),
        };
        let mut visitor = MockVisitor::new();
        visit_page(&tree, &page, &mut visitor);
        assert_eq!(visitor.line_count, 2); // id2 and id3
    }

    #[test]
    fn translates_to_page_local_coords() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                0.0,
                200.0,
                vec![make_layout_line(id, 100.0, 20.0)],
            ),
        };
        let page = LayoutPage {
            y_start: 80.0,
            y_end: 180.0,
            size: Size::new(440.0, 100.0),
        };
        let mut visitor = MockVisitor::new();
        visit_page(&tree, &page, &mut visitor);
        assert_eq!(visitor.line_count, 1);
        // line absolute y=100, page y_start=80 -> local y = 20
        assert!((visitor.line_local_rects[0].y - 20.0).abs() < 0.01);
    }

    #[test]
    fn border_visibility_for_split_box() {
        // Box from y=50 to y=250, page [100, 200]
        // top: hidden (50 < 100), bottom: hidden (250 > 200)
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                50.0,
                200.0,
                vec![make_layout_line(NodeId::new(), 120.0, 20.0)],
            ),
        };
        let page = LayoutPage {
            y_start: 100.0,
            y_end: 200.0,
            size: Size::new(440.0, 100.0),
        };
        let mut visitor = MockVisitor::new();
        visit_page(&tree, &page, &mut visitor);
        let edges = visitor.last_edges.unwrap();
        assert!(!edges.top); // box starts before page
        assert!(!edges.bottom); // box ends after page
    }

    #[test]
    fn skips_spacing_nodes() {
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                0.0,
                100.0,
                vec![
                    make_layout_line(NodeId::new(), 0.0, 20.0),
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 20.0, 0.0, 16.0),
                        content: LayoutContent::Spacing(SpacingKind::Gap),
                    },
                    make_layout_line(NodeId::new(), 36.0, 20.0),
                ],
            ),
        };
        let page = LayoutPage {
            y_start: 0.0,
            y_end: 100.0,
            size: Size::new(440.0, 100.0),
        };
        let mut visitor = MockVisitor::new();
        visit_page(&tree, &page, &mut visitor);
        assert_eq!(visitor.line_count, 2); // spacing skipped
    }

    #[test]
    fn line_receives_metrics() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                0.0,
                200.0,
                vec![make_layout_line(id, 100.0, 20.0)],
            ),
        };
        let page = LayoutPage {
            y_start: 80.0,
            y_end: 180.0,
            size: Size::new(440.0, 100.0),
        };
        let mut visitor = MockVisitor::new();
        visit_page(&tree, &page, &mut visitor);
        let m = visitor.last_line_metrics.unwrap();
        assert!((m.baseline - 16.0).abs() < 0.01); // 20 * 0.8
        assert!((m.ascent - 14.0).abs() < 0.01); // 20 * 0.7
        assert!((m.descent - 2.0).abs() < 0.01); // 20 * 0.1
    }
}

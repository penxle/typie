use crate::page_fragment::{
    PageFragmentAtom, PageFragmentBox, PageFragmentContent, PageFragmentDecoration,
    PageFragmentLine, PageFragmentNode, PageFragmentTree,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn box_enter(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox);
    fn box_exit(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox);
    fn line(&mut self, node: &PageFragmentNode, fragment: &PageFragmentLine);
    fn atom(&mut self, node: &PageFragmentNode, fragment: &PageFragmentAtom);
    fn decoration(&mut self, decoration: &PageFragmentDecoration);
}

pub(crate) fn visit_page(tree: &PageFragmentTree, visitor: &mut impl PageVisitor) {
    if let Some(root) = &tree.root {
        visit_node(root, visitor);
    }
}

fn visit_node(node: &PageFragmentNode, visitor: &mut impl PageVisitor) {
    match &node.content {
        PageFragmentContent::Box(b) => {
            visitor.box_enter(node, b);

            for dec in &b.decorations {
                visitor.decoration(dec);
            }

            for child in &b.children {
                visit_node(child, visitor);
            }

            visitor.box_exit(node, b);
        }
        PageFragmentContent::Line(l) => {
            visitor.line(node, l);
        }
        PageFragmentContent::Atom(a) => {
            visitor.atom(node, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::LayoutPage;
    use crate::page_fragment::build_page_fragment_tree;
    use crate::paginate::{
        LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree, SpacingKind,
    };
    use crate::style::Alignment;
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_model::NodeId;

    use crate::style::*;

    struct MockVisitor {
        box_enter_count: usize,
        box_exit_count: usize,
        line_count: usize,
        atom_count: usize,
        decoration_count: usize,
        last_edges: Option<Edges<bool>>,
        box_local_rects: Vec<Rect>,
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
                box_local_rects: vec![],
                line_local_rects: vec![],
                last_line_metrics: None,
            }
        }
    }

    impl PageVisitor for MockVisitor {
        fn box_enter(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox) {
            self.box_enter_count += 1;
            self.last_edges = Some(fragment.edges);
            self.box_local_rects.push(node.rect);
        }

        fn box_exit(&mut self, _: &PageFragmentNode, _: &PageFragmentBox) {
            self.box_exit_count += 1;
        }

        fn line(&mut self, node: &PageFragmentNode, fragment: &PageFragmentLine) {
            self.line_count += 1;
            self.line_local_rects.push(node.rect);
            self.last_line_metrics = Some(LineMetrics {
                baseline: fragment.baseline,
                ascent: fragment.ascent,
                descent: fragment.descent,
            });
        }

        fn atom(&mut self, _: &PageFragmentNode, _: &PageFragmentAtom) {
            self.atom_count += 1;
        }

        fn decoration(&mut self, _: &PageFragmentDecoration) {
            self.decoration_count += 1;
        }
    }

    fn visit_layout_page(tree: &LayoutTree, page: &LayoutPage, visitor: &mut impl PageVisitor) {
        let fragment_tree = build_page_fragment_tree(tree, 0, page);
        visit_page(&fragment_tree, visitor);
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
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
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
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                attachment: None,
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
        let page = LayoutPage::new(50.0, 150.0, Size::new(440.0, 100.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);
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
        let page = LayoutPage::new(80.0, 180.0, Size::new(440.0, 100.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);
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
        let page = LayoutPage::new(100.0, 200.0, Size::new(440.0, 100.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);
        let edges = visitor.last_edges.unwrap();
        assert!(!edges.top); // box starts before page
        assert!(!edges.bottom); // box ends after page
    }

    #[test]
    fn box_rects_are_visible_content_fragments() {
        // Physical page is [0, 120], but only [10, 110] is content.
        let tree = LayoutTree {
            root: make_layout_box(
                NodeId::ROOT,
                5.0,
                110.0,
                vec![make_layout_line(NodeId::new(), 20.0, 20.0)],
            ),
        };
        let page = LayoutPage::with_content(0.0, 120.0, 10.0, 110.0, Size::new(440.0, 120.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);

        let rect = visitor.box_local_rects[0];
        assert_eq!(rect.y, 10.0);
        assert_eq!(rect.height, 100.0);
        let edges = visitor.last_edges.unwrap();
        assert!(!edges.top);
        assert!(!edges.bottom);
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
                        content: LayoutContent::Spacing(SpacingKind::Gap {
                            position: editor_state::Position::new(NodeId::ROOT, 0),
                        }),
                    },
                    make_layout_line(NodeId::new(), 36.0, 20.0),
                ],
            ),
        };
        let page = LayoutPage::new(0.0, 100.0, Size::new(440.0, 100.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);
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
        let page = LayoutPage::new(80.0, 180.0, Size::new(440.0, 100.0));
        let mut visitor = MockVisitor::new();
        visit_layout_page(&tree, &page, &mut visitor);
        let m = visitor.last_line_metrics.unwrap();
        assert!((m.baseline - 16.0).abs() < 0.01); // 20 * 0.8
        assert!((m.ascent - 14.0).abs() < 0.01); // 20 * 0.7
        assert!((m.descent - 2.0).abs() < 0.01); // 20 * 0.1
    }
}

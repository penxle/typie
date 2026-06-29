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

use crate::page_fragment::{
    PageFragmentAtom, PageFragmentBox, PageFragmentContent, PageFragmentDecoration,
    PageFragmentLine, PageFragmentNode, PageFragmentTree,
};

pub trait PageVisitor {
    fn box_enter(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox);
    fn box_exit(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox);
    fn line(&mut self, node: &PageFragmentNode, fragment: &PageFragmentLine);
    fn atom(&mut self, node: &PageFragmentNode, fragment: &PageFragmentAtom);
    fn decoration(&mut self, decoration: &PageFragmentDecoration);
}

pub fn visit_page(tree: &PageFragmentTree, visitor: &mut impl PageVisitor) {
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
    use super::super::{Edges, LineMetrics};
    use super::*;
    use crate::page::LayoutPage;
    use crate::page_fragment::build_page_fragment_tree;
    use crate::paginate::types::{
        ChildAttachment, LayoutAtom, LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree,
    };
    use crate::style::{Alignment, BorderMode, BoxStyle, Direction};
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_crdt::Dot;

    fn elem(peer: u64, clock: u64) -> Dot {
        Dot::new(peer, clock)
    }

    fn page(y_start: f32, height: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + height, Size::new(800.0, height))
    }

    fn empty_box_style() -> BoxStyle {
        BoxStyle {
            direction: Direction::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: Alignment::Start,
            decorations: vec![],
            monolithic: false,
        }
    }

    fn line_node(node: Dot, y: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Line(LayoutLine {
                node,
                baseline: h * 0.8,
                ascent: h * 0.8,
                descent: h * 0.2,
                cursor_ascent: h * 0.8,
                cursor_descent: h * 0.2,
                glyph_runs: vec![],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                offset_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn box_node(node: Dot, y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node,
                style: empty_box_style(),
                children,
                attachment: None,
            }),
        }
    }

    fn atom_node(node: Dot, parent: Dot, index: usize, y: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 50.0, 50.0),
            content: LayoutContent::Atom(LayoutAtom {
                node,
                attachment: ChildAttachment { parent, index },
            }),
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    enum VisitEvent {
        BoxEnter(Dot),
        BoxExit(Dot),
        Line(Dot),
        Atom(Dot),
        Decoration,
    }

    struct RecordingVisitor {
        events: Vec<VisitEvent>,
        line_metrics: Vec<LineMetrics>,
        box_edges: Vec<Edges<bool>>,
    }

    impl RecordingVisitor {
        fn new() -> Self {
            Self {
                events: vec![],
                line_metrics: vec![],
                box_edges: vec![],
            }
        }
    }

    impl PageVisitor for RecordingVisitor {
        fn box_enter(&mut self, _node: &PageFragmentNode, fragment: &PageFragmentBox) {
            self.events.push(VisitEvent::BoxEnter(fragment.node));
            self.box_edges.push(fragment.edges);
        }

        fn box_exit(&mut self, _node: &PageFragmentNode, fragment: &PageFragmentBox) {
            self.events.push(VisitEvent::BoxExit(fragment.node));
        }

        fn line(&mut self, _node: &PageFragmentNode, fragment: &PageFragmentLine) {
            self.events.push(VisitEvent::Line(fragment.node));
            self.line_metrics.push(LineMetrics {
                baseline: fragment.baseline,
                ascent: fragment.ascent,
                descent: fragment.descent,
            });
        }

        fn atom(&mut self, _node: &PageFragmentNode, fragment: &PageFragmentAtom) {
            self.events.push(VisitEvent::Atom(fragment.node));
        }

        fn decoration(&mut self, _decoration: &PageFragmentDecoration) {
            self.events.push(VisitEvent::Decoration);
        }
    }

    fn visit_layout(tree: &LayoutTree, pg: &LayoutPage, visitor: &mut impl PageVisitor) {
        let fragment = build_page_fragment_tree(tree, 0, pg);
        visit_page(&fragment, visitor);
    }

    #[test]
    fn two_level_nesting_dfs_order() {
        let outer_id = elem(1, 0);
        let inner_id = elem(1, 1);
        let line_id = elem(1, 2);

        let inner = box_node(inner_id, 10.0, 30.0, vec![line_node(line_id, 15.0, 20.0)]);
        let root = box_node(outer_id, 0.0, 100.0, vec![inner]);
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let mut visitor = RecordingVisitor::new();
        visit_layout(&tree, &pg, &mut visitor);

        assert_eq!(
            visitor.events,
            vec![
                VisitEvent::BoxEnter(outer_id),
                VisitEvent::BoxEnter(inner_id),
                VisitEvent::Line(line_id),
                VisitEvent::BoxExit(inner_id),
                VisitEvent::BoxExit(outer_id),
            ],
            "DFS order: enter(outer) → enter(inner) → line → exit(inner) → exit(outer)"
        );
    }

    #[test]
    fn atom_callback_fires_with_correct_id() {
        let root_id = elem(2, 0);
        let parent_id = elem(2, 1);
        let atom_id = elem(2, 2);

        let at = atom_node(atom_id, parent_id, 0, 10.0);
        let root = box_node(root_id, 0.0, 100.0, vec![at]);
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let mut visitor = RecordingVisitor::new();
        visit_layout(&tree, &pg, &mut visitor);

        assert!(
            visitor.events.contains(&VisitEvent::Atom(atom_id)),
            "atom event must fire with the atom's Dot"
        );
    }

    #[test]
    fn line_callback_fires_with_correct_id_and_metrics() {
        let root_id = elem(3, 0);
        let line_id = elem(3, 1);

        let root = box_node(root_id, 0.0, 100.0, vec![line_node(line_id, 10.0, 20.0)]);
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let mut visitor = RecordingVisitor::new();
        visit_layout(&tree, &pg, &mut visitor);

        assert!(
            visitor.events.contains(&VisitEvent::Line(line_id)),
            "line event must fire with the line's Dot"
        );
        assert_eq!(visitor.line_metrics.len(), 1);
        assert!((visitor.line_metrics[0].baseline - 16.0).abs() < 0.01);
    }

    #[test]
    fn decoration_fires_before_children() {
        use crate::style::{Decoration, DecorationData};

        let root_id = elem(4, 0);
        let line_id = elem(4, 1);

        let dec = Decoration {
            id: 1,
            rect: Rect::from_xywh(0.0, 0.0, 10.0, 5.0),
            data: DecorationData::Bullet,
        };
        let style = BoxStyle {
            direction: Direction::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: Alignment::Start,
            decorations: vec![dec],
            monolithic: false,
        };
        let root = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 100.0),
            content: LayoutContent::Box(LayoutBox {
                node: root_id,
                style,
                children: vec![line_node(line_id, 10.0, 20.0)],
                attachment: None,
            }),
        };
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let mut visitor = RecordingVisitor::new();
        visit_layout(&tree, &pg, &mut visitor);

        let enter_pos = visitor
            .events
            .iter()
            .position(|e| e == &VisitEvent::BoxEnter(root_id))
            .unwrap();
        let dec_pos = visitor
            .events
            .iter()
            .position(|e| e == &VisitEvent::Decoration)
            .unwrap();
        let line_pos = visitor
            .events
            .iter()
            .position(|e| e == &VisitEvent::Line(line_id))
            .unwrap();
        let exit_pos = visitor
            .events
            .iter()
            .position(|e| e == &VisitEvent::BoxExit(root_id))
            .unwrap();

        assert!(enter_pos < dec_pos, "decoration fires after box_enter");
        assert!(dec_pos < line_pos, "decoration fires before children");
        assert!(line_pos < exit_pos, "children fire before box_exit");
    }
}

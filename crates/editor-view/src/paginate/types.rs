use std::ops::Range;

use editor_common::Rect;
use editor_crdt::Dot;
use editor_state::Position;

use crate::glyph_run::GlyphRun;
use crate::glyph_run::RubyAnnotation;
use crate::measure::text::measure::TabGap;
use crate::page::LayoutPage;
use crate::style::BoxStyle;

#[derive(Debug)]
pub(crate) struct PaginatedLayout {
    pub tree: LayoutTree,
    pub pages: Vec<LayoutPage>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutTree {
    pub root: LayoutNode,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutNode {
    pub rect: Rect,
    pub content: LayoutContent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LayoutContent {
    Box(LayoutBox),
    Line(LayoutLine),
    Atom(LayoutAtom),
    Spacing(SpacingKind),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SpacingKind {
    Gap { position: Position },
    Fill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildAttachment {
    pub parent: Dot,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutBox {
    pub node: Dot,
    pub style: BoxStyle,
    pub children: Vec<LayoutNode>,
    pub attachment: Option<ChildAttachment>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutLine {
    pub node: Dot,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    pub offset_range: Option<Range<usize>>,
    pub tab_gaps: Vec<TabGap>,
    pub is_phantom: bool,
    pub content_edge_x: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutAtom {
    pub node: Dot,
    pub attachment: ChildAttachment,
}

#[cfg(test)]
mod tests {
    use editor_common::Rect;
    use editor_crdt::Dot;
    use editor_state::Position;

    use super::*;

    fn rect() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }
    }

    #[test]
    fn constructs_box_with_line_child() {
        let line = LayoutLine {
            node: Dot::new(1, 1),
            baseline: 8.0,
            ascent: 8.0,
            descent: 2.0,
            cursor_ascent: 8.0,
            cursor_descent: 2.0,
            glyph_runs: vec![],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            offset_range: Some(0..1),
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
        };
        let tree = LayoutTree {
            root: LayoutNode {
                rect: rect(),
                content: LayoutContent::Box(LayoutBox {
                    node: Dot::ROOT,
                    style: BoxStyle::default(),
                    children: vec![LayoutNode {
                        rect: rect(),
                        content: LayoutContent::Line(line),
                    }],
                    attachment: None,
                }),
            },
        };
        let LayoutContent::Box(b) = &tree.root.content else {
            panic!()
        };
        assert_eq!(b.node, Dot::ROOT);
        assert_eq!(b.children.len(), 1);
        let LayoutContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.node, Dot::new(1, 1));
        assert_eq!(l.offset_range, Some(0..1));
    }

    #[test]
    fn attachment_and_spacing() {
        let a = ChildAttachment {
            parent: Dot::ROOT,
            index: 2,
        };
        let b = ChildAttachment {
            parent: Dot::ROOT,
            index: 2,
        };
        assert_eq!(a, b);

        let fill = SpacingKind::Fill;
        assert!(matches!(fill, SpacingKind::Fill));
        let gap = SpacingKind::Gap {
            position: Position::new(Dot::ROOT, 0),
        };
        assert!(matches!(gap, SpacingKind::Gap { .. }));
    }

    #[test]
    fn clone_preserves_atom() {
        let atom = LayoutNode {
            rect: rect(),
            content: LayoutContent::Atom(LayoutAtom {
                node: Dot::new(1, 3),
                attachment: ChildAttachment {
                    parent: Dot::ROOT,
                    index: 0,
                },
            }),
        };
        let cloned = atom.clone();
        let LayoutContent::Atom(at) = &cloned.content else {
            panic!()
        };
        assert_eq!(at.node, Dot::new(1, 3));
    }
}

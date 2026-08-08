use std::sync::Arc;

use editor_common::Rect;
use editor_crdt::Dot;
use editor_state::Position;

use crate::measure::text::measure::MeasuredLine;
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
    pub children: LayoutChildren,
    pub attachment: Option<ChildAttachment>,
    pub scope: bool,
}

/// Copy-on-write child storage keeps displayed presentation snapshots cheap
/// while content-only edits replace just the affected ancestor paths.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LayoutChildren(Arc<Vec<LayoutNode>>);

impl LayoutChildren {
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut LayoutNode> {
        Arc::make_mut(&mut self.0).get_mut(index)
    }
}

impl From<Vec<LayoutNode>> for LayoutChildren {
    fn from(children: Vec<LayoutNode>) -> Self {
        Self(Arc::new(children))
    }
}

impl std::ops::Deref for LayoutChildren {
    type Target = [LayoutNode];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a LayoutChildren {
    type Item = &'a LayoutNode;
    type IntoIter = std::slice::Iter<'a, LayoutNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A positioned line. Shares the measured payload (glyph runs, ruby, tab
/// gaps) by `Arc` instead of deep-cloning it — pagination re-walks the whole
/// document per edit, and copying every line's glyphs dominated the
/// per-keystroke cost on large documents.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutLine {
    pub measured: Arc<MeasuredLine>,
}

impl std::ops::Deref for LayoutLine {
    type Target = MeasuredLine;

    fn deref(&self) -> &MeasuredLine {
        &self.measured
    }
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
            measured: std::sync::Arc::new(crate::measure::text::measure::MeasuredLine {
                height: 0.0,
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
            }),
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
                    }]
                    .into(),
                    attachment: None,
                    scope: false,
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
    fn cloned_layout_children_share_storage_until_mutated() {
        let original = LayoutChildren::from(vec![LayoutNode {
            rect: rect(),
            content: LayoutContent::Spacing(SpacingKind::Fill),
        }]);
        let mut cloned = original.clone();

        assert!(Arc::ptr_eq(&original.0, &cloned.0));
        cloned.get_mut(0).unwrap().rect.x = 5.0;
        assert_eq!(original[0].rect.x, 0.0);
        assert_eq!(cloned[0].rect.x, 5.0);
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

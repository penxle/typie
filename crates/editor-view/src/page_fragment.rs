use std::ops::Range;

use editor_common::Rect;
use editor_crdt::Dot;

use crate::glyph_run::GlyphRun;
use crate::glyph_run::RubyAnnotation;
use crate::measure::text::measure::TabGap;
use crate::page::LayoutPage;
use crate::paginate::types::{
    ChildAttachment, LayoutAtom, LayoutContent, LayoutLine, LayoutNode, LayoutTree,
};
use crate::query::Edges;
use crate::style::{BoxStyle, DecorationData};

#[derive(Debug, Clone)]
pub struct PageFragmentTree {
    pub page_idx: usize,
    pub root: Option<PageFragmentNode>,
}

#[derive(Debug, Clone)]
pub struct PageFragmentNode {
    pub rect: Rect,
    pub content: PageFragmentContent,
}

#[derive(Debug, Clone)]
pub enum PageFragmentContent {
    Box(PageFragmentBox),
    Line(PageFragmentLine),
    Atom(PageFragmentAtom),
}

#[derive(Debug, Clone)]
pub struct PageFragmentBox {
    pub node: Dot,
    pub style: BoxStyle,
    pub edges: Edges<bool>,
    pub decorations: Vec<PageFragmentDecoration>,
    pub children: Vec<PageFragmentNode>,
    pub attachment: Option<ChildAttachment>,
}

#[derive(Debug, Clone)]
pub struct PageFragmentDecoration {
    pub rect: Rect,
    pub data: DecorationData,
}

#[derive(Debug, Clone)]
pub struct PageFragmentLine {
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
}

#[derive(Debug, Clone)]
pub struct PageFragmentAtom {
    pub node: Dot,
    pub attachment: ChildAttachment,
}

impl PageFragmentNode {
    pub fn as_box(&self) -> Option<&PageFragmentBox> {
        match &self.content {
            PageFragmentContent::Box(b) => Some(b),
            _ => None,
        }
    }
}

pub(crate) fn build_page_fragment_tree(
    tree: &LayoutTree,
    page_idx: usize,
    page: &LayoutPage,
) -> PageFragmentTree {
    PageFragmentTree {
        page_idx,
        root: fragment_node(&tree.root, page),
    }
}

fn fragment_node(node: &LayoutNode, page: &LayoutPage) -> Option<PageFragmentNode> {
    let node_top = node.rect.y;
    let node_bottom = node.rect.bottom();
    let visible_top = page.content_y_start;
    let visible_bottom = page.content_y_end;

    if node_bottom <= visible_top || node_top >= visible_bottom {
        return None;
    }

    let content = match &node.content {
        LayoutContent::Box(b) => {
            let fragment_top = node_top.max(visible_top);
            let fragment_bottom = node_bottom.min(visible_bottom);
            let rect = Rect::from_xywh(
                node.rect.x,
                fragment_top - page.y_start,
                node.rect.width,
                fragment_bottom - fragment_top,
            );
            let content = PageFragmentContent::Box(PageFragmentBox {
                node: b.node,
                style: b.style.clone(),
                edges: Edges {
                    top: node_top >= visible_top,
                    bottom: node_bottom <= visible_bottom,
                    left: true,
                    right: true,
                },
                decorations: b
                    .style
                    .decorations
                    .iter()
                    .filter_map(|dec| fragment_decoration(node, dec, page))
                    .collect(),
                children: b
                    .children
                    .iter()
                    .filter_map(|child| fragment_node(child, page))
                    .collect(),
                attachment: b.attachment.clone(),
            });
            return Some(PageFragmentNode { rect, content });
        }
        LayoutContent::Line(l) => {
            debug_assert!(
                node_top >= visible_top && node_bottom <= visible_bottom,
                "line layout node should be contained by its page content window"
            );
            PageFragmentContent::Line(fragment_line(l))
        }
        LayoutContent::Atom(a) => {
            let fragment_top = node_top.max(visible_top);
            let fragment_bottom = node_bottom.min(visible_bottom);
            let rect = Rect::from_xywh(
                node.rect.x,
                fragment_top - page.y_start,
                node.rect.width,
                fragment_bottom - fragment_top,
            );
            let content = PageFragmentContent::Atom(fragment_atom(a));
            return Some(PageFragmentNode { rect, content });
        }
        LayoutContent::Spacing(kind) => {
            let _ = kind;
            return None;
        }
    };

    let rect = Rect::from_xywh(
        node.rect.x,
        node.rect.y - page.y_start,
        node.rect.width,
        node.rect.height,
    );
    Some(PageFragmentNode { rect, content })
}

fn fragment_decoration(
    node: &LayoutNode,
    dec: &crate::style::Decoration,
    page: &LayoutPage,
) -> Option<PageFragmentDecoration> {
    let dec_abs_y = node.rect.y + dec.rect.y;
    let dec_abs_bottom = dec_abs_y + dec.rect.height;

    if dec_abs_y < page.content_y_start || dec_abs_bottom > page.content_y_end {
        return None;
    }

    Some(PageFragmentDecoration {
        rect: Rect::from_xywh(
            node.rect.x + dec.rect.x,
            dec_abs_y - page.y_start,
            dec.rect.width,
            dec.rect.height,
        ),
        data: dec.data.clone(),
    })
}

fn fragment_line(line: &LayoutLine) -> PageFragmentLine {
    PageFragmentLine {
        node: line.node,
        baseline: line.baseline,
        ascent: line.ascent,
        descent: line.descent,
        cursor_ascent: line.cursor_ascent,
        cursor_descent: line.cursor_descent,
        glyph_runs: line.glyph_runs.clone(),
        ruby_annotations: line.ruby_annotations.clone(),
        empty_caret_x: line.empty_caret_x,
        offset_range: line.offset_range.clone(),
        tab_gaps: line.tab_gaps.clone(),
    }
}

fn fragment_atom(atom: &LayoutAtom) -> PageFragmentAtom {
    PageFragmentAtom {
        node: atom.node,
        attachment: atom.attachment.clone(),
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_crdt::Dot;

    use crate::page::LayoutPage;
    use crate::paginate::types::{
        ChildAttachment, LayoutAtom, LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree,
    };
    use crate::style::{Alignment, BorderMode, BoxStyle, Direction};

    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

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

    fn line_node(
        node: Dot,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        offset_range: Option<std::ops::Range<usize>>,
    ) -> LayoutNode {
        LayoutNode {
            rect: rect(x, y, w, h),
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
                offset_range,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn box_node(
        node: Dot,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        children: Vec<LayoutNode>,
        attachment: Option<ChildAttachment>,
    ) -> LayoutNode {
        LayoutNode {
            rect: rect(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node,
                style: empty_box_style(),
                children,
                attachment,
            }),
        }
    }

    fn atom_node(
        node: Dot,
        parent: Dot,
        index: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> LayoutNode {
        LayoutNode {
            rect: rect(x, y, w, h),
            content: LayoutContent::Atom(LayoutAtom {
                node,
                attachment: ChildAttachment { parent, index },
            }),
        }
    }

    #[test]
    fn clip_excludes_node_outside_page() {
        let root_id = elem(1, 0);
        let inside_id = elem(1, 1);
        let outside_id = elem(1, 2);

        let inside_line = line_node(inside_id, 0.0, 10.0, 200.0, 20.0, Some(0..5));
        let outside_line = line_node(outside_id, 0.0, 200.0, 200.0, 20.0, Some(5..10));

        let root = box_node(
            root_id,
            0.0,
            0.0,
            200.0,
            220.0,
            vec![inside_line, outside_line],
            None,
        );
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        assert_eq!(fragment.page_idx, 0);
        let root_frag = fragment.root.expect("root must be present");
        let root_box = root_frag.as_box().expect("root must be a box");

        assert_eq!(
            root_box.children.len(),
            1,
            "only the inside line should be kept"
        );
        let kept_child = &root_box.children[0];
        match &kept_child.content {
            PageFragmentContent::Line(l) => {
                assert_eq!(l.node, inside_id, "kept line must carry its Dot");
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn offset_range_carried_onto_fragment_line() {
        let root_id = elem(2, 0);
        let line_id = elem(2, 1);

        let ln = line_node(line_id, 0.0, 10.0, 200.0, 20.0, Some(3..9));
        let root = box_node(root_id, 0.0, 0.0, 200.0, 100.0, vec![ln], None);
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let fragment = build_page_fragment_tree(&tree, 0, &pg);
        let root_frag = fragment.root.unwrap();
        let root_box = root_frag.as_box().unwrap();

        assert_eq!(root_box.children.len(), 1);
        match &root_box.children[0].content {
            PageFragmentContent::Line(l) => {
                assert_eq!(l.offset_range, Some(3..9), "offset_range must be carried");
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn node_elem_id_carried_onto_box_and_atom() {
        let root_id = elem(3, 0);
        let parent_id = elem(3, 1);
        let atom_id = elem(3, 2);

        let at = atom_node(atom_id, parent_id, 0, 0.0, 10.0, 50.0, 50.0);
        let root = box_node(root_id, 0.0, 0.0, 200.0, 200.0, vec![at], None);
        let tree = LayoutTree { root };
        let pg = page(0.0, 200.0);

        let fragment = build_page_fragment_tree(&tree, 0, &pg);
        let root_frag = fragment.root.unwrap();
        let root_box = root_frag.as_box().unwrap();
        assert_eq!(root_box.node, root_id, "box node Dot must be carried");

        assert_eq!(root_box.children.len(), 1);
        match &root_box.children[0].content {
            PageFragmentContent::Atom(a) => {
                assert_eq!(a.node, atom_id, "atom node Dot must be carried");
                assert_eq!(
                    a.attachment.parent, parent_id,
                    "atom attachment parent must be carried"
                );
                assert_eq!(
                    a.attachment.index, 0,
                    "atom attachment index must be carried"
                );
            }
            _ => panic!("expected Atom"),
        }
    }

    #[test]
    fn decoration_geometry_preserved() {
        use crate::style::{Decoration, DecorationData};

        let root_id = elem(4, 0);
        let dec = Decoration {
            id: 1,
            rect: Rect::from_xywh(5.0, 2.0, 100.0, 3.0),
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
                children: vec![],
                attachment: None,
            }),
        };
        let tree = LayoutTree { root };
        let pg = page(0.0, 100.0);

        let fragment = build_page_fragment_tree(&tree, 0, &pg);
        let root_frag = fragment.root.unwrap();
        let root_box = root_frag.as_box().unwrap();

        assert_eq!(
            root_box.decorations.len(),
            1,
            "decoration must be preserved"
        );
        let frag_dec = &root_box.decorations[0];
        assert_eq!(
            frag_dec.rect,
            Rect::from_xywh(5.0, 2.0, 100.0, 3.0),
            "decoration rect must match"
        );
        assert!(matches!(frag_dec.data, DecorationData::Bullet));
    }
}

use editor_common::Rect;
use editor_model::NodeId;

use crate::glyph_run::{GlyphRun, RubyAnnotation};
use crate::measure::TabGap;
use crate::page::LayoutPage;
use crate::paginate::{
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
    pub node_id: NodeId,
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
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    pub child_range: Option<std::ops::Range<usize>>,
    pub tab_gaps: Vec<TabGap>,
}

#[derive(Debug, Clone)]
pub struct PageFragmentAtom {
    pub node_id: NodeId,
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

pub(crate) fn build_page_fragment_trees(
    tree: &LayoutTree,
    pages: &[LayoutPage],
) -> Vec<PageFragmentTree> {
    pages
        .iter()
        .enumerate()
        .map(|(page_idx, page)| build_page_fragment_tree(tree, page_idx, page))
        .collect()
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
                node_id: b.node_id,
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
                attachment: b.attachment,
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
        node_id: line.node_id,
        baseline: line.baseline,
        ascent: line.ascent,
        descent: line.descent,
        cursor_ascent: line.cursor_ascent,
        cursor_descent: line.cursor_descent,
        glyph_runs: line.glyph_runs.clone(),
        ruby_annotations: line.ruby_annotations.clone(),
        empty_caret_x: line.empty_caret_x,
        child_range: line.child_range.clone(),
        tab_gaps: line.tab_gaps.clone(),
    }
}

fn fragment_atom(atom: &LayoutAtom) -> PageFragmentAtom {
    PageFragmentAtom {
        node_id: atom.node_id,
        attachment: atom.attachment,
    }
}

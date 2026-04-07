use editor_common::Rect;
use editor_model::NodeId;

use crate::glyph_run::GlyphRun;
use crate::style::BoxStyle;

#[derive(Debug)]
pub struct LayoutTree {
    pub root: LayoutNode,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub rect: Rect,
    pub content: LayoutContent,
}

#[derive(Debug, Clone)]
pub enum LayoutContent {
    Box(LayoutBox),
    Line(LayoutLine),
    Atom(LayoutAtom),
    Spacing(SpacingKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingKind {
    Gap,
    Fill,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node_id: NodeId,
    pub style: BoxStyle,
    pub children: Vec<LayoutNode>,
}

#[derive(Debug, Clone)]
pub struct LayoutLine {
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub text_indent: f32,
}

#[derive(Debug, Clone)]
pub struct LayoutAtom {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
}

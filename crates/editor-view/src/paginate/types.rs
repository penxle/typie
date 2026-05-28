use editor_common::Rect;
use editor_model::NodeId;
use std::ops::Range;

use crate::TableLayoutInfo;
use crate::glyph_run::{GlyphRun, RubyAnnotation};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavUnit {
    pub parent_id: NodeId,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node_id: NodeId,
    pub style: BoxStyle,
    pub table_info: Option<Box<TableLayoutInfo>>,
    pub children: Vec<LayoutNode>,
    pub nav: Option<NavUnit>,
}

#[derive(Debug, Clone)]
pub struct LayoutLine {
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    /// Mirror of [`crate::measure::MeasuredLine::child_range`]; see that
    /// type for the inclusive-both matching contract and `None` semantics.
    pub child_range: Option<Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct LayoutAtom {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
}

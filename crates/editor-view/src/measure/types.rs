use editor_model::NodeId;
use std::ops::Range;
use std::sync::Arc;

use crate::TableLayoutInfo;
use crate::glyph_run::{GlyphRun, RubyAnnotation};
use crate::style::BoxStyle;

#[derive(Debug)]
pub struct MeasuredTree {
    pub root: MeasuredNode,
}

#[derive(Debug, Clone)]
pub struct MeasuredNode {
    pub width: f32,
    pub height: f32,
    pub content: MeasuredContent,
}

#[derive(Debug, Clone)]
pub enum MeasuredContent {
    Box(MeasuredBox),
    Line(MeasuredLine),
    Atom(MeasuredAtom),
    Spacing(f32),
    PageBreak,
}

#[derive(Debug, Clone)]
pub struct MeasuredBox {
    pub node_id: NodeId,
    pub style: BoxStyle,
    pub table_info: Option<Box<TableLayoutInfo>>,
    pub children: Vec<Arc<MeasuredNode>>,
}

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    /// Paragraph child-offset interval this visual line owns for matching
    /// container-anchored cursor positions. Matching is inclusive of both
    /// endpoints (`start <= offset && offset <= end`, not `Range::contains`).
    /// `None` for soft-wrap interior lines of a multi-line text segment —
    /// those lines own no paragraph boundary.
    pub child_range: Option<Range<usize>>,
}

#[derive(Debug, Clone)]
pub struct MeasuredAtom {
    pub node_id: NodeId,
}

use editor_model::NodeId;
use std::ops::Range;
use std::sync::Arc;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageBreakPolicy {
    #[default]
    Auto,
    Avoid,
}

impl MeasuredNode {
    pub(crate) fn page_break_policy(&self) -> PageBreakPolicy {
        match &self.content {
            MeasuredContent::Box(b) => b.page_break_policy,
            MeasuredContent::Line(_) | MeasuredContent::Atom(_) => PageBreakPolicy::Avoid,
            MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => PageBreakPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeasuredBox {
    pub node_id: NodeId,
    pub style: BoxStyle,
    pub children: Vec<Arc<MeasuredNode>>,
    pub page_break_policy: PageBreakPolicy,
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
    pub tab_gaps: Vec<TabGap>,
    pub is_phantom: bool,
    pub content_edge_x: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TabGap {
    pub node_id: NodeId,
    pub child_index: usize,
    pub x: f32,
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct MeasuredAtom {
    pub node_id: NodeId,
}

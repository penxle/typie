use editor_model::NodeId;
use std::sync::Arc;

use crate::glyph_run::GlyphRun;
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
    pub children: Vec<Arc<MeasuredNode>>,
}

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub node_id: NodeId,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub text_indent: f32,
}

#[derive(Debug, Clone)]
pub struct MeasuredAtom {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub index: usize,
}

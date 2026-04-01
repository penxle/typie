use editor_common::Rect;
use editor_model::NodeId;

use crate::fragment::glyph_run::GlyphRun;

#[derive(Debug, Clone)]
pub struct LineFragment {
    pub node_id: NodeId,
    pub rect: Rect,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,
}

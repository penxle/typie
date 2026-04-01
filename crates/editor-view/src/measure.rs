use editor_common::{Alignment, EdgeInsets, Rect, Size};
use editor_model::NodeId;
use std::sync::Arc;

use crate::fragment::{GlyphRun, PlaceholderData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub size: Size,
    pub gap_after: f32,
    pub alignment: Alignment,
    pub content: MeasuredContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderMode {
    #[default]
    Separate,
    Collapse,
}

/// 측정 단계에서 결정된 placeholder 위치.
#[derive(Debug, Clone)]
pub struct MeasuredPlaceholder {
    pub id: u32,
    pub rect: Rect,
    pub data: PlaceholderData,
}

#[derive(Debug, Clone)]
pub struct ContainerContent {
    pub children: Vec<ChildMeasurement>,
    pub scope: bool,
    pub direction: LayoutDirection,
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub border_mode: BorderMode,
    pub placeholders: Vec<MeasuredPlaceholder>,
}

#[cfg(test)]
impl Default for ContainerContent {
    fn default() -> Self {
        Self {
            children: vec![],
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            placeholders: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum MeasuredContent {
    Container(ContainerContent),
    TextBlock { lines: Vec<MeasuredLine> },
    Atom { parent_id: NodeId, index: usize },
    PageBreak,
}

#[derive(Debug, Clone)]
pub struct ChildMeasurement {
    pub node_id: NodeId,
    pub measurement: Arc<Measurement>,
}

#[derive(Debug, Clone)]
pub struct MeasuredLine {
    pub height: f32,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,
}

use crate::glyph_run::GlyphRun;
use editor_common::{Alignment, EdgeInsets, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderMode {
    #[default]
    Separate,
    Collapse,
}

#[derive(Debug, Clone)]
pub struct BoxStyle {
    pub direction: Direction,
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub border_mode: BorderMode,
    pub alignment: Alignment,
    pub scope: bool,
    pub decorations: Vec<Decoration>,
}

#[derive(Debug, Clone)]
pub struct Decoration {
    pub id: u32,
    pub rect: Rect,
    pub data: DecorationData,
}

#[derive(Debug, Clone)]
pub enum DecorationData {
    None,
    Bool(bool),
    Number(f32),
    Text(String),
    Glyphs(Vec<GlyphRun>),
}

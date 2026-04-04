use editor_model::NodeId;

/// Interned font identifier from FontRegistry.
pub type FontId = u16;

/// Position of an individual glyph within a run.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// Faux bold/italic synthesis flags for fonts that lack native variants.
#[derive(Debug, Clone, Copy, Default)]
pub struct Synthesis {
    pub embolden: bool,
    pub skew: Option<f32>,
}

/// A single glyph run combining render data (glyphs) and cursor navigation (char_advances).
#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub font_id: FontId,
    pub font_weight: u16,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,
    pub background_color: Option<String>,
    pub glyphs: Vec<Glyph>,

    pub node_id: NodeId,
    pub offset: usize,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub char_advances: Vec<f32>,
}

#[cfg(test)]
impl GlyphRun {
    pub fn make_test_run(
        node_id: NodeId,
        offset: usize,
        text: &str,
        x: f32,
        advances: Vec<f32>,
    ) -> Self {
        let width = advances.iter().sum();
        Self {
            font_id: 0,
            font_weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            node_id,
            offset,
            text: text.to_string(),
            x,
            width,
            char_advances: advances,
        }
    }
}

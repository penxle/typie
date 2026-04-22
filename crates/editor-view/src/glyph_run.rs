use editor_model::NodeId;

pub type FontId = u16;

#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Synthesis {
    pub embolden: bool,
    pub skew: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct GraphemeSpan {
    pub advance: f32,
    pub codepoints: u8,
}

#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub family_id: FontId,
    pub weight: u16,
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
    pub graphemes: Vec<GraphemeSpan>,
}

#[cfg(test)]
impl GlyphRun {
    pub fn make_test_run(
        node_id: NodeId,
        offset: usize,
        text: &str,
        x: f32,
        graphemes: Vec<GraphemeSpan>,
    ) -> Self {
        let width = graphemes.iter().map(|g| g.advance).sum();
        Self {
            family_id: 0,
            weight: 400,
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
            graphemes,
        }
    }
}

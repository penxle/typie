pub type FontId = u16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Synthesis {
    pub embolden: bool,
    pub skew: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphemeSpan {
    pub advance: f32,
    pub codepoints: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextDecoration {
    pub underline: bool,
    pub strikethrough: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    pub family_id: FontId,
    pub weight: u16,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,
    pub background_color: Option<String>,
    pub glyphs: Vec<Glyph>,
    pub decoration: TextDecoration,

    pub offset_range: std::ops::Range<usize>,
    pub link: Option<String>,
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub graphemes: Vec<GraphemeSpan>,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RubyAnnotation {
    pub family_id: FontId,
    pub weight: u16,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,
    pub ascent: f32,
    pub descent: f32,
    pub glyphs: Vec<Glyph>,
    pub x: f32,
    pub baseline_y: f32,
    pub width: f32,
}

#[cfg(test)]
impl GlyphRun {
    pub fn make_test_run(
        offset_range: std::ops::Range<usize>,
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
            decoration: TextDecoration::default(),
            offset_range,
            link: None,
            text: text.to_string(),
            x,
            width,
            graphemes,
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_test_run_defaults_decoration_to_false() {
        let run = GlyphRun::make_test_run(0..0, "hi", 0.0, vec![]);
        assert!(!run.decoration.underline);
        assert!(!run.decoration.strikethrough);
    }
}

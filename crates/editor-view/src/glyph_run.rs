use editor_model::NodeId;

/// 폰트 식별자. FontRegistry에서 interning된 값.
pub type FontId = u16;

/// 개별 glyph의 위치 정보.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// Faux bold/italic 합성 정보.
#[derive(Debug, Clone, Copy, Default)]
pub struct Synthesis {
    pub embolden: bool,
    pub skew: Option<f32>,
}

/// 하나의 glyph run. 렌더링(glyph 데이터)과 커서 내비게이션(char_advances)을 통합.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    // 렌더용
    pub font_id: FontId,
    pub font_weight: u16,
    pub font_size: f32,
    pub synthesis: Synthesis,
    pub color: String,
    pub background_color: Option<String>,
    pub glyphs: Vec<Glyph>,

    // 커서용
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

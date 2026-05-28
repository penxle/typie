use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen;
use skrifa::{FontRef, MetadataProvider};
use std::fmt::Write;

use crate::ServerError;

const SVG_PPEM: f32 = 100.0;

struct SvgPathPen {
    d: String,
    x_offset: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl SvgPathPen {
    fn new() -> Self {
        Self {
            d: String::new(),
            x_offset: 0.0,
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    fn track(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn fmt(v: f32) -> f32 {
        (v * 10.0).round() / 10.0
    }

    fn sx(&self, x: f32) -> f32 {
        Self::fmt(x + self.x_offset)
    }

    fn sy(y: f32) -> f32 {
        Self::fmt(-y)
    }
}

impl OutlinePen for SvgPathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (sx, sy) = (self.sx(x), Self::sy(y));
        self.track(sx, sy);
        let _ = write!(self.d, "M{sx} {sy}");
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (sx, sy) = (self.sx(x), Self::sy(y));
        self.track(sx, sy);
        let _ = write!(self.d, "L{sx} {sy}");
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (scx, scy) = (self.sx(cx), Self::sy(cy));
        let (sx, sy) = (self.sx(x), Self::sy(y));
        self.track(scx, scy);
        self.track(sx, sy);
        let _ = write!(self.d, "Q{scx} {scy} {sx} {sy}");
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        let (scx1, scy1) = (self.sx(cx1), Self::sy(cy1));
        let (scx2, scy2) = (self.sx(cx2), Self::sy(cy2));
        let (sx, sy) = (self.sx(x), Self::sy(y));
        self.track(scx1, scy1);
        self.track(scx2, scy2);
        self.track(sx, sy);
        let _ = write!(self.d, "C{scx1} {scy1} {scx2} {scy2} {sx} {sy}");
    }

    fn close(&mut self) {
        self.d.push('Z');
    }
}

pub fn outline_text_to_svg(font_data: &[u8], text: &str) -> Result<String, ServerError> {
    let font = FontRef::new(font_data).map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let size = Size::new(SVG_PPEM);
    let loc = LocationRef::default();
    let glyph_metrics = font.glyph_metrics(size, loc);
    let charmap = font.charmap();
    let outlines = font.outline_glyphs();

    let mut pen = SvgPathPen::new();
    let mut cursor_x = 0.0_f32;

    for ch in text.chars() {
        let gid = charmap
            .map(ch)
            .ok_or_else(|| ServerError::InvalidFont(format!("missing glyph for '{ch}'")))?;
        pen.x_offset = cursor_x;
        if let Some(glyph) = outlines.get(gid) {
            let _ = glyph.draw(size, &mut pen);
        }
        cursor_x += glyph_metrics.advance_width(gid).unwrap_or(0.0);
    }

    if pen.d.is_empty() {
        return Err(ServerError::InvalidFont("no glyphs to render".into()));
    }

    let min_x = pen.min_x;
    let width = SvgPathPen::fmt(pen.max_x - pen.min_x);
    let glyph_center_y = (pen.min_y + pen.max_y) / 2.0;
    let min_y = SvgPathPen::fmt(glyph_center_y - SVG_PPEM / 2.0);
    let height = SVG_PPEM;

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{min_x} {min_y} {width} {height}" fill="currentColor"><path d="{}"/></svg>"#,
        pen.d
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use skrifa::MetadataProvider;

    fn load_test_font() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/Pretendard-Regular.ttf"
        ))
        .expect("test font not found")
    }

    fn first_outline_char(data: &[u8]) -> char {
        let font = FontRef::new(data).expect("valid test font");
        let outlines = font.outline_glyphs();
        font.charmap()
            .mappings()
            .find_map(|(cp, gid)| {
                let glyph = outlines.get(gid)?;
                let mut pen = SvgPathPen::new();
                glyph.draw(Size::new(SVG_PPEM), &mut pen).ok()?;
                (!pen.d.is_empty()).then(|| char::from_u32(cp)).flatten()
            })
            .expect("test font must contain at least one outline glyph")
    }

    fn svg_width(svg: &str) -> f32 {
        let key = "viewBox=\"";
        let start = svg.find(key).expect("svg must contain viewBox") + key.len();
        let end = svg[start..].find('"').expect("viewBox must be closed") + start;
        let values: Vec<f32> = svg[start..end]
            .split_whitespace()
            .map(|v| v.parse::<f32>().expect("viewBox values must be numbers"))
            .collect();
        *values.get(2).expect("viewBox must contain width")
    }

    #[test]
    fn outline_text_to_svg_returns_svg_document() {
        let data = load_test_font();
        let text = first_outline_char(&data).to_string();
        let svg = outline_text_to_svg(&data, &text).unwrap();
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
        assert!(svg.contains("<path d=\""));
        assert!(svg.contains("fill=\"currentColor\""));
    }

    #[test]
    fn outline_text_to_svg_rejects_invalid_font() {
        let result = outline_text_to_svg(&[0, 1, 2, 3], "Test");
        assert!(result.is_err());
    }

    #[test]
    fn outline_text_to_svg_applies_advance_width_between_glyphs() {
        // 여러 글자를 그릴 때 glyph advance width 가 반영되어 SVG viewBox 폭이 커지는지 확인한다.
        let data = load_test_font();
        let ch = first_outline_char(&data);
        let single = outline_text_to_svg(&data, &ch.to_string()).unwrap();
        let doubled = outline_text_to_svg(&data, &format!("{ch}{ch}")).unwrap();

        assert!(svg_width(&doubled) > svg_width(&single));
    }
}

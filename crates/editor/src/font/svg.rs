use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen;
use skrifa::{FontRef, MetadataProvider};
use std::fmt::Write;

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

pub(crate) fn outline_text_to_svg(font_data: &[u8], text: &str) -> Result<String, String> {
    let font = FontRef::new(font_data).map_err(|e| e.to_string())?;
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
            .ok_or_else(|| format!("missing glyph for '{ch}'"))?;
        pen.x_offset = cursor_x;
        if let Some(glyph) = outlines.get(gid) {
            let _ = glyph.draw(size, &mut pen);
        }
        cursor_x += glyph_metrics.advance_width(gid).unwrap_or(0.0);
    }

    if pen.d.is_empty() {
        return Err("no glyphs to render".into());
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

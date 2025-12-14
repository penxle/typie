use crate::layout::elements::blockquote::BlockquoteLineElement;
use crate::render::{GlyphRenderer, Render, RenderContext};
use tiny_skia::{Color, Paint, PixmapMut, Rect, Transform};

impl Render for BlockquoteLineElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        _ctx: &RenderContext,
    ) {
        let Some(rect) = Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height) else {
            return;
        };

        let color = Color::from_rgba8(200, 200, 200, 255);
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        pixmap.fill_rect(rect, &paint, transform, None);
    }
}

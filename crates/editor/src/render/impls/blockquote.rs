use crate::layout::elements::SplitEdges;
use crate::layout::elements::blockquote::{
    BlockquoteLineElement, BlockquoteMessageElement, BlockquoteQuoteElement,
};
use crate::model::BlockquoteVariant;
use crate::render::{GlyphRenderer, Render, RenderContext};
use macros::svg_icon_path;
use tiny_skia::{Paint, PathBuilder, PixmapMut, Rect, Transform};

const QUOTE_ICON_SIZE: f32 = 16.0;
const MESSAGE_BORDER_RADIUS: f32 = 18.0;
const MESSAGE_TAIL_SIZE: f32 = 10.0;

impl Render for BlockquoteLineElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let Some(rect) = Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height) else {
            return;
        };

        let color = ctx.theme.color("ui.border.default");
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        pixmap.fill_rect(rect, &paint, transform, None);
    }
}

impl Render for BlockquoteQuoteElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let color = ctx.theme.color("ui.text.muted");
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        let cx = self.size.width / 2.0;
        let cy = self.size.height / 2.0;

        let path = svg_icon_path!("typie/blockquote-quote", QUOTE_ICON_SIZE, cx, cy);

        if let Some(path) = path {
            pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        }
    }
}

impl Render for BlockquoteMessageElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let is_sent = matches!(self.variant, BlockquoteVariant::MessageSent);
        let has_tail = !self.split_edges.bottom;

        let bg_color = if is_sent {
            ctx.theme.color("ui.blockquote.message-sent")
        } else {
            ctx.theme.color("ui.blockquote.message-received")
        };

        let mut paint = Paint::default();
        paint.set_color(bg_color);
        paint.anti_alias = true;

        let (tl, tr, mut br, mut bl) = corner_radii(MESSAGE_BORDER_RADIUS, &self.split_edges);

        if has_tail {
            if is_sent {
                br = 0.0;
            } else {
                bl = 0.0;
            }
        }

        if let Some(path) =
            build_rounded_rect(0.0, 0.0, self.size.width, self.size.height, tl, tr, br, bl)
        {
            pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        }

        if has_tail {
            if let Some(tail_path) = build_message_tail(self.size.width, self.size.height, is_sent)
            {
                pixmap.fill_path(
                    &tail_path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    transform,
                    None,
                );
            }
        }
    }
}

fn corner_radii(radius: f32, split: &SplitEdges) -> (f32, f32, f32, f32) {
    let top_radius = if split.top { 0.0 } else { radius };
    let bottom_radius = if split.bottom { 0.0 } else { radius };
    (top_radius, top_radius, bottom_radius, bottom_radius)
}

fn build_rounded_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    tl: f32,
    tr: f32,
    br: f32,
    bl: f32,
) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();

    pb.move_to(x + tl, y);
    pb.line_to(x + width - tr, y);
    if tr > 0.0 {
        pb.quad_to(x + width, y, x + width, y + tr);
    }
    pb.line_to(x + width, y + height - br);
    if br > 0.0 {
        pb.quad_to(x + width, y + height, x + width - br, y + height);
    }
    pb.line_to(x + bl, y + height);
    if bl > 0.0 {
        pb.quad_to(x, y + height, x, y + height - bl);
    }
    pb.line_to(x, y + tl);
    if tl > 0.0 {
        pb.quad_to(x, y, x + tl, y);
    }
    pb.close();

    pb.finish()
}

fn build_message_tail(width: f32, height: f32, is_sent: bool) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let s = MESSAGE_TAIL_SIZE;

    if is_sent {
        pb.move_to(width - s * 0.8, height);
        pb.quad_to(width, height, width, height - s * 0.5);
        pb.quad_to(width, height, width + s * 0.4, height + s * 0.15);
        pb.quad_to(width - s * 0.2, height + s * 0.05, width - s * 0.8, height);
        pb.close();
    } else {
        pb.move_to(s * 0.8, height);
        pb.quad_to(0.0, height, 0.0, height - s * 0.5);
        pb.quad_to(0.0, height, -s * 0.4, height + s * 0.15);
        pb.quad_to(s * 0.2, height + s * 0.05, s * 0.8, height);
        pb.close();
    }

    pb.finish()
}

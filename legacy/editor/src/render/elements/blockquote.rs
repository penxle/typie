use crate::layout::elements::SplitEdges;
use crate::layout::elements::blockquote::{
    BlockquoteLineElement, BlockquoteMessageElement, BlockquoteQuoteElement, MESSAGE_TAIL_SIZE,
};
use crate::model::BlockquoteVariant;
use crate::render::sink::RenderSink;
use crate::render::{Render, RenderParams, RenderPhase};
use kurbo::{Affine, BezPath, Rect};
use macros::svg_icon_path;
use peniko::{Brush, Fill};

const QUOTE_ICON_SIZE: f32 = 16.0;
const MESSAGE_BORDER_RADIUS: f32 = 18.0;
const BLOCKQUOTE_DECORATION_GAP: f32 = 16.0;

impl Render for BlockquoteLineElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl BlockquoteLineElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let is_selected = ctx.is_block_selected(self.block_id);

        match ctx.phase {
            RenderPhase::Content => {
                let line_width = self.line_width.min(self.size.width).max(0.0);
                let rect = Rect::new(0.0, 0.0, line_width as f64, self.size.height as f64);

                let color = ctx.theme.color("ui.border.default");
                let brush = Brush::Solid(color);

                sink.fill_rect(rect, &brush, transform);
            }
            RenderPhase::Selection => {
                if !is_selected {
                    return;
                }

                let selection_width = if ctx.has_descendant_text_selection(self.block_id) {
                    (self.line_width + BLOCKQUOTE_DECORATION_GAP).min(self.size.width)
                } else {
                    self.size.width
                };

                let rect = Rect::new(0.0, 0.0, selection_width as f64, self.size.height as f64);

                let brush = ctx.selection_paint();
                sink.fill_rect(rect, &brush, transform);
            }
            _ => {}
        }
    }
}

impl Render for BlockquoteQuoteElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl BlockquoteQuoteElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let is_selected = ctx.is_block_selected(self.block_id);

        match ctx.phase {
            RenderPhase::Content => {
                let color = ctx.theme.color("ui.text.muted");
                let brush = Brush::Solid(color);

                let cx = QUOTE_ICON_SIZE / 2.0;
                let cy = QUOTE_ICON_SIZE / 2.0;

                let path = svg_icon_path!("typie/blockquote-quote", QUOTE_ICON_SIZE, cx, cy);

                if let Some(path) = path {
                    sink.fill_path(&path, &brush, Fill::NonZero, transform);
                }
            }
            RenderPhase::Selection => {
                if !is_selected {
                    return;
                }

                let selection_width = if ctx.has_descendant_text_selection(self.block_id) {
                    (QUOTE_ICON_SIZE + BLOCKQUOTE_DECORATION_GAP).min(self.size.width)
                } else {
                    self.size.width
                };

                let rect = Rect::new(0.0, 0.0, selection_width as f64, self.size.height as f64);

                let brush = ctx.selection_paint();
                sink.fill_rect(rect, &brush, transform);
            }
            _ => {}
        }
    }
}

impl Render for BlockquoteMessageElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl BlockquoteMessageElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let is_selected = ctx.is_block_selected(self.block_id);

        match ctx.phase {
            RenderPhase::Background => {
                let is_sent = matches!(self.variant, BlockquoteVariant::MessageSent);
                let has_tail = !self.split_edges.bottom;

                let bg_color = if is_sent {
                    ctx.theme.color("ui.blockquote.message-sent")
                } else {
                    ctx.theme.color("ui.blockquote.message-received")
                };

                let brush = Brush::Solid(bg_color);

                let (tl, tr, mut br, mut bl) =
                    corner_radii(MESSAGE_BORDER_RADIUS, &self.split_edges);

                if has_tail {
                    if is_sent {
                        br = 0.0;
                    } else {
                        bl = 0.0;
                    }
                }

                let path =
                    build_rounded_rect(0.0, 0.0, self.size.width, self.size.height, tl, tr, br, bl);
                sink.fill_path(&path, &brush, Fill::NonZero, transform);

                if has_tail {
                    let tail_path = build_message_tail(self.size.width, self.size.height, is_sent);
                    sink.fill_path(&tail_path, &brush, Fill::NonZero, transform);
                }
            }
            RenderPhase::Selection => {
                if !is_selected {
                    return;
                }

                let is_sent = matches!(self.variant, BlockquoteVariant::MessageSent);
                let has_tail = !self.split_edges.bottom;

                let brush = ctx.selection_paint();

                let (tl, tr, mut br, mut bl) =
                    corner_radii(MESSAGE_BORDER_RADIUS, &self.split_edges);

                if has_tail {
                    if is_sent {
                        br = 0.0;
                    } else {
                        bl = 0.0;
                    }
                }

                let path =
                    build_rounded_rect(0.0, 0.0, self.size.width, self.size.height, tl, tr, br, bl);
                sink.fill_path(&path, &brush, Fill::NonZero, transform);

                if has_tail {
                    let tail_path = build_message_tail(self.size.width, self.size.height, is_sent);
                    sink.fill_path(&tail_path, &brush, Fill::NonZero, transform);
                }
            }
            _ => {}
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
) -> BezPath {
    let mut bp = BezPath::new();

    bp.move_to(((x + tl) as f64, y as f64));
    bp.line_to(((x + width - tr) as f64, y as f64));
    if tr > 0.0 {
        bp.quad_to(
            ((x + width) as f64, y as f64),
            ((x + width) as f64, (y + tr) as f64),
        );
    }
    bp.line_to(((x + width) as f64, (y + height - br) as f64));
    if br > 0.0 {
        bp.quad_to(
            ((x + width) as f64, (y + height) as f64),
            ((x + width - br) as f64, (y + height) as f64),
        );
    }
    bp.line_to(((x + bl) as f64, (y + height) as f64));
    if bl > 0.0 {
        bp.quad_to(
            (x as f64, (y + height) as f64),
            (x as f64, (y + height - bl) as f64),
        );
    }
    bp.line_to((x as f64, (y + tl) as f64));
    if tl > 0.0 {
        bp.quad_to((x as f64, y as f64), ((x + tl) as f64, y as f64));
    }
    bp.close_path();

    bp
}

fn build_message_tail(width: f32, height: f32, is_sent: bool) -> BezPath {
    let mut bp = BezPath::new();
    let s = MESSAGE_TAIL_SIZE;

    if is_sent {
        bp.move_to(((width - s * 0.8) as f64, height as f64));
        bp.quad_to(
            (width as f64, height as f64),
            (width as f64, (height - s * 0.5) as f64),
        );
        bp.quad_to(
            (width as f64, height as f64),
            ((width + s * 0.4) as f64, (height + s * 0.15) as f64),
        );
        bp.quad_to(
            ((width - s * 0.2) as f64, (height + s * 0.05) as f64),
            ((width - s * 0.8) as f64, height as f64),
        );
        bp.close_path();
    } else {
        bp.move_to(((s * 0.8) as f64, height as f64));
        bp.quad_to((0.0, height as f64), (0.0, (height - s * 0.5) as f64));
        bp.quad_to(
            (0.0, height as f64),
            ((-s * 0.4) as f64, (height + s * 0.15) as f64),
        );
        bp.quad_to(
            ((s * 0.2) as f64, (height + s * 0.05) as f64),
            ((s * 0.8) as f64, height as f64),
        );
        bp.close_path();
    }

    bp
}

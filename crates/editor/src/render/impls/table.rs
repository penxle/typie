use crate::layout::elements::{TableBorderElement, TableCellElement};
use crate::model::{TABLE_BORDER_WIDTH, TableBorderStyle};
use crate::render::{GlyphRenderer, Render, RenderContext, RenderPhase};
use tiny_skia::{Paint, PathBuilder, PixmapMut, Stroke, Transform};

impl Render for TableBorderElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        if matches!(self.border_style, TableBorderStyle::None) {
            return;
        }

        let color = ctx.theme.color("ui.border.default");
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        let stroke = Stroke {
            width: TABLE_BORDER_WIDTH,
            dash: match self.border_style {
                TableBorderStyle::Dashed => {
                    Some(tiny_skia::StrokeDash::new(vec![4.0, 2.0], 0.0).unwrap())
                }
                TableBorderStyle::Dotted => {
                    Some(tiny_skia::StrokeDash::new(vec![1.0, 2.0], 0.0).unwrap())
                }
                _ => None,
            },
            ..Default::default()
        };

        let mut pb = PathBuilder::new();

        let half = TABLE_BORDER_WIDTH / 2.0;
        pb.move_to(half, half);
        pb.line_to(self.size.width - half, half);
        pb.line_to(self.size.width - half, self.size.height - half);
        pb.line_to(half, self.size.height - half);
        pb.close();

        let mut y = TABLE_BORDER_WIDTH;
        for (idx, row_height) in self.row_heights.iter().enumerate() {
            y += *row_height;
            if idx < self.row_heights.len() - 1 {
                pb.move_to(0.0, y);
                pb.line_to(self.size.width, y);
            }
        }

        let mut x = TABLE_BORDER_WIDTH;
        for (idx, col_width) in self.col_widths.iter().enumerate() {
            x += *col_width;
            if idx < self.col_widths.len() - 1 {
                x += TABLE_BORDER_WIDTH;
                pb.move_to(x - TABLE_BORDER_WIDTH / 2.0, 0.0);
                pb.line_to(x - TABLE_BORDER_WIDTH / 2.0, self.size.height);
            }
        }

        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
    }
}

impl Render for TableCellElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let is_selected = ctx
            .selections
            .iter()
            .any(|s| s.is_cell() && s.node_id() == self.node_id);
        match ctx.phase {
            RenderPhase::Background => {
                let mut paint = Paint::default();
                paint.set_color(ctx.theme.color("ui.surface.default"));
                if let Some(rect) =
                    tiny_skia::Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height)
                {
                    pixmap.fill_rect(rect, &paint, transform, None);
                }
            }
            RenderPhase::Selection => {
                if is_selected {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("ui.surface.dark", 32)
                    };
                    let mut paint = Paint::default();
                    paint.set_color(color);

                    if let Some(rect) =
                        tiny_skia::Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height)
                    {
                        pixmap.fill_rect(rect, &paint, transform, None);
                    }
                }
            }
            RenderPhase::Content => {}
        }
    }
}

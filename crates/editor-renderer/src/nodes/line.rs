use editor_common::Rect;
use editor_view::fragment::LineFragment;
use std::sync::Arc;

use crate::glyph;
use crate::renderer::Renderer;
use crate::sink::RenderSink;
use crate::types::Transform;

pub fn draw(
    renderer: &mut Renderer,
    sink: &mut dyn RenderSink,
    lf: &LineFragment,
    transform: Transform,
) {
    let t = transform.translate(lf.rect.x, lf.rect.y);
    let scale_factor = transform.m[0];
    let inv_scale = 1.0 / scale_factor;

    for run in &lf.glyph_runs {
        if let Some(ref bg_token) = run.background_color {
            let bg_color = renderer.theme.color(bg_token);
            let run_rect = Rect {
                x: run.x,
                y: 0.0,
                width: run.width,
                height: lf.rect.height,
            };
            sink.fill_rect(run_rect, bg_color, t);
        }

        let color = renderer.theme.color(&run.color);

        let resource = Arc::clone(&renderer.resource);
        let resource_guard = resource.lock().unwrap();
        let positioned = crate::glyph::rasterize(
            run,
            &resource_guard.font_registry,
            &mut renderer.scale_ctx,
            &mut renderer.glyph_cache,
            scale_factor,
        );
        drop(resource_guard);

        for pg in &positioned {
            // Glyph path/bitmap is rasterized at device pixel size
            // (font_size * scale_factor). The root transform already scales
            // CSS positions to device pixels, so post_scale(1/sf) cancels
            // the root scale for glyph geometry while preserving the
            // device-pixel translation.
            let gt = t.translate(pg.x, pg.y).post_scale(inv_scale);
            match &pg.raster {
                glyph::RasterizedGlyph::Path(path) => {
                    sink.fill_path(path, color, gt);
                }
                glyph::RasterizedGlyph::Bitmap(image) => {
                    let rect = Rect {
                        x: 0.0,
                        y: 0.0,
                        width: image.width as f32,
                        height: image.height as f32,
                    };
                    sink.draw_image(image, rect, gt);
                }
            }
        }
    }
}

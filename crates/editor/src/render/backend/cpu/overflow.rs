//! CPU next-page overflow rendering and caching.

use crate::layout::Page;
use crate::model::Doc;
use crate::render::Renderer;
use crate::render::backend::cpu::pixel_buf::{PixelBuf, PixelBufMut};
use crate::render::backend::cpu::sink::CpuSink;
use crate::render::cache::same_scale_factor;
use crate::render::geometry::{LayoutRect, PixelRect};
use crate::render::renderer::{OverflowRenderCacheEntry, next_page_overflow_cull_clip};
use crate::types::Theme;
use rustc_hash::FxHashMap;
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub(in crate::render) fn render_next_page_overflow(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    theme: &Theme,
    page_idx: usize,
    next_page: &Page,
    doc: &Doc,
    overflow_cache: &mut FxHashMap<usize, OverflowRenderCacheEntry>,
    debug_rects: Option<&mut Vec<LayoutRect>>,
) {
    let scale = scale_factor as f32;
    let page_height = buf.height() as f32 / scale;
    let page_width = buf.width() as f32 / scale;
    let Some(cull_clip) = next_page_overflow_cull_clip(page_width, page_height) else {
        overflow_cache.remove(&page_idx);
        return;
    };
    let Some(pixel_rect) = PixelRect::from_layout_rect(cull_clip, scale, buf.width(), buf.height())
    else {
        overflow_cache.remove(&page_idx);
        return;
    };
    if !Renderer::has_visible_next_page_overflow(next_page, page_height, cull_clip) {
        overflow_cache.remove(&page_idx);
        return;
    }
    let next_root_ptr = Rc::as_ptr(&next_page.root.node) as usize;
    if let Some(cache_entry) = overflow_cache.get(&page_idx)
        && same_scale_factor(cache_entry.scale_factor, scale_factor)
        && cache_entry.canvas_width == buf.width()
        && cache_entry.canvas_height == buf.height()
        && cache_entry.pixel_rect == pixel_rect
        && cache_entry.next_root_ptr == next_root_ptr
    {
        super::compose::composite_src_over(
            buf,
            &cache_entry.tile,
            pixel_rect.x as i32,
            pixel_rect.y as i32,
        );

        if let Some(debug_rects) = debug_rects {
            debug_rects.extend(cache_entry.debug_rects.iter().copied());
        }
        return;
    }
    let next_snapshot = Renderer::next_page_overflow_snapshot(next_page, page_height, cull_clip);
    if let Some(cache_entry) = overflow_cache.get(&page_idx)
        && same_scale_factor(cache_entry.scale_factor, scale_factor)
        && cache_entry.canvas_width == buf.width()
        && cache_entry.canvas_height == buf.height()
        && cache_entry.pixel_rect == pixel_rect
        && cache_entry.next_snapshot == next_snapshot
    {
        super::compose::composite_src_over(
            buf,
            &cache_entry.tile,
            pixel_rect.x as i32,
            pixel_rect.y as i32,
        );

        if let Some(debug_rects) = debug_rects {
            debug_rects.extend(cache_entry.debug_rects.iter().copied());
        }
        return;
    }

    let hard_clip_layout_rect = pixel_rect.to_layout_rect(scale);
    let Some(mut tile_buf) = PixelBuf::new(pixel_rect.width, pixel_rect.height) else {
        overflow_cache.remove(&page_idx);
        return;
    };

    let w = tile_buf.width() as u16;
    let h = tile_buf.height() as u16;
    let mut sink = CpuSink::new(w, h);
    Renderer::render_next_page_overflow_to_sink(
        &mut sink,
        scale_factor,
        theme,
        next_page,
        doc,
        page_height,
        hard_clip_layout_rect,
        cull_clip,
    );
    sink.flush_to(tile_buf.data_mut(), w, h);
    super::compose::composite_src_over(buf, &tile_buf, pixel_rect.x as i32, pixel_rect.y as i32);

    let cache_debug_rects = Renderer::collect_next_page_overflow_debug_rects(
        next_page,
        page_width,
        page_height,
        cull_clip,
    );
    if let Some(debug_rects) = debug_rects {
        debug_rects.extend(cache_debug_rects.iter().copied());
    }

    overflow_cache.insert(
        page_idx,
        OverflowRenderCacheEntry {
            scale_factor,
            canvas_width: buf.width(),
            canvas_height: buf.height(),
            pixel_rect,
            next_root_ptr,
            next_snapshot,
            tile: tile_buf,
            debug_rects: cache_debug_rects,
        },
    );
}

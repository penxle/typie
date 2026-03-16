mod blend;
mod cache;
mod debug_overlay;
mod drag_image;
mod geometry;
pub mod glyph;
mod impls;
pub mod outline;
mod paint_diagnostics;
mod pipeline;
mod renderer;
mod selection;
mod vector_codec;

#[cfg(test)]
mod tests;

pub use glyph::GlyphRenderer;
pub use outline::{ElementSink, RasterSink, VectorPage, VectorSink};
#[cfg(feature = "native")]
pub use renderer::RenderInfo;
pub use renderer::{
    DragImageResult, Outline, Render, RenderContext, RenderPhase, RenderResult, Renderer,
};
pub(crate) use selection::{
    fill_rect_src_over_fast, selection_overlay_color, selection_overlay_paint,
};
pub use vector_codec::encode_vector_page;

use blend::blend_row_src_over;
use cache::{PageRenderCache, PageRenderSnapshot, node_paint_bounds, same_scale_factor};
use debug_overlay::render_debug_overlay;
use geometry::{
    CacheRect, PixelRect, clear_layout_rect, collect_non_overlapping_pixel_rects,
    merge_and_clamp_rects,
};
use paint_diagnostics::{PaintDebugFrame, PaintDiagnosticsState, collect_layout_dirty_rects};
#[cfg(test)]
use renderer::{FULL_REPAINT_RECT_THRESHOLD, PAGE_EDGE_OVERFLOW_BAND};
use renderer::{
    OverflowRenderCacheEntry, OverflowRenderSnapshot, OverflowSnapshotItem, SelectionOverlayData,
    next_page_overflow_cull_clip, normalize_dirty_rects, should_promote_full_repaint,
};

use crate::diagnostics::FrameDiagnostics;
use crate::layout::elements::LineElement;
use crate::layout::query::{DragImageBounds, DragImagePageBounds};
use crate::layout::{Element, Page, PositionedNode, RenderHints};
use crate::model::{Doc, LayoutMode, NodeId, SelectionDecor};
use crate::runtime::DropIndicator;
use crate::types::{Point, Theme};
use rstar::AABB;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use tiny_skia::{Color, Paint, Pixmap, PixmapMut, PixmapPaint, Rect, Transform};

pub mod backend;
mod cache;
mod debug_overlay;
mod diagnostics;
mod drag_image;
mod elements;
mod geometry;
pub mod glyph;
mod pipeline;
mod renderer;
pub mod sink;
mod surface;

#[cfg(test)]
mod tests;

pub(crate) use backend::cpu::sink::CpuSink;
pub use backend::export::{ExportPage, ExportSink, encode_export_page};
pub(crate) use pipeline::selection::{selection_overlay_brush, selection_overlay_color};
#[cfg(any(feature = "native", feature = "uniffi", test))]
pub use renderer::RenderInfo;
pub use renderer::{DragImageResult, Render, RenderParams, RenderPhase, Renderer};
pub use sink::RenderSink;
#[cfg(any(feature = "native", feature = "uniffi"))]
pub use surface::PlatformBuffer;
pub use surface::SurfaceSize;

use cache::{PageCache, node_paint_bounds, same_scale_factor};
#[cfg(test)]
use diagnostics::collect_layout_dirty_rects;
use diagnostics::{DebugFrame, DiagnosticsState};
use geometry::{LayoutRect, PixelRect, merge_and_clamp_rects};
#[cfg(test)]
use renderer::{FULL_REPAINT_RECT_THRESHOLD, PAGE_EDGE_OVERFLOW_BAND};
use renderer::{
    OverflowRenderSnapshot, OverflowSnapshotItem, SelectionOverlayData, normalize_dirty_rects,
};
#[cfg(test)]
use renderer::{next_page_overflow_cull_clip, should_promote_full_repaint};

use crate::diagnostics::FrameDiagnostics;
use crate::layout::elements::LineElement;
use crate::layout::query::{DragImageBounds, DragImagePageBounds};
use crate::layout::{Element, Page, PositionedNode, RenderHints};
use crate::model::{Doc, LayoutMode, NodeId, SelectionDecor};
use crate::runtime::DropIndicator;
use crate::types::theme::Color;
use crate::types::{Point, Theme};
use backend::cpu::pixel_buf::PixelBuf;
use kurbo::Affine;
use peniko::Brush;
use rstar::AABB;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};

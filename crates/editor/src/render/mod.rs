pub mod glyph;
mod impls;

pub use glyph::GlyphRenderer;

use crate::layout::{Element, Page, PositionedNode, RenderHints};
use crate::model::{Doc, SelectionDecor};
use crate::runtime::DropIndicator;
use crate::types::{Point, Theme};
use tiny_skia::{Color, Pixmap, PixmapMut, Rect, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    Background,
    Content,
    Selection,
}

pub struct RenderContext<'a> {
    pub scale_factor: f64,
    pub selections: &'a [SelectionDecor],
    pub theme: &'a Theme,
    pub doc: &'a Doc,
    pub default_text_color: Option<Color>,
    pub is_focused: bool,
    pub phase: RenderPhase,
}

pub trait Render {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext<'_>,
    );

    fn ignores_clip(&self) -> bool {
        false
    }
}

#[allow(dead_code)]
pub struct RenderResult {
    pub ptr: *const u8,
    pub len: usize,
    pub width: u16,
    pub height: u16,
}

pub struct RenderInfo {
    pub width: u16,
    pub height: u16,
    pub buffer_size: usize,
}

pub struct DragImageResult {
    pixmap: Pixmap,
    pub width: u16,
    pub height: u16,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
}

impl DragImageResult {
    pub fn ptr(&self) -> *const u8 {
        self.pixmap.data().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.pixmap.data().len()
    }
}

pub struct Renderer {
    scale_factor: f64,
    pixmap: Pixmap,
    glyph_renderer: GlyphRenderer,
    theme: Theme,
    is_focused: bool,
}

impl Renderer {
    pub fn new(scale_factor: f64) -> Self {
        let pixmap = Pixmap::new(1, 1).unwrap();

        Self {
            scale_factor,
            pixmap,
            glyph_renderer: GlyphRenderer::new(),
            theme: Theme::default(),
            is_focused: true,
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32, scale_factor: f64) {
        let new_width = (width as f64 * scale_factor).round() as u32;
        let new_height = (height as f64 * scale_factor).round() as u32;

        if self.pixmap.width() != new_width || self.pixmap.height() != new_height {
            if let Some(new_pixmap) = Pixmap::new(new_width.max(1), new_height.max(1)) {
                self.pixmap = new_pixmap;
            }
        }
        self.scale_factor = scale_factor;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn width(&self) -> u16 {
        self.pixmap.width() as u16
    }

    pub fn height(&self) -> u16 {
        self.pixmap.height() as u16
    }

    pub fn render(
        &mut self,
        page: &Page,
        page_idx: usize,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
    ) -> RenderResult {
        self.pixmap.data_mut().fill(0);

        let scale = self.scale_factor as f32;
        let transform = Transform::from_scale(scale, scale);

        let stages = [
            RenderPhase::Background,
            RenderPhase::Selection,
            RenderPhase::Content,
        ];

        let mut pixmap = self.pixmap.as_mut();

        for phase in stages {
            let ctx = RenderContext {
                scale_factor: self.scale_factor,
                selections,
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
            };

            Self::render_node(
                &mut pixmap,
                &mut self.glyph_renderer,
                &page.root,
                Point::zero(),
                transform,
                &ctx,
                &RenderHints::default(),
            );

            if phase == RenderPhase::Content {
                if let Some(indicator) = drop_indicator {
                    Self::render_drop_indicator(&mut pixmap, indicator, page_idx, transform, &ctx);
                }
            }
        }

        let data = self.pixmap.data();
        RenderResult {
            ptr: data.as_ptr(),
            len: data.len(),
            width: self.width(),
            height: self.height(),
        }
    }

    pub fn render_to(
        &mut self,
        page: &Page,
        page_idx: usize,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
        dst: &mut [u8],
    ) -> bool {
        let expected_size = self.pixmap.width() as usize * self.pixmap.height() as usize * 4;
        if dst.len() < expected_size {
            return false;
        }

        let Some(mut pixmap) =
            PixmapMut::from_bytes(dst, self.pixmap.width(), self.pixmap.height())
        else {
            return false;
        };

        pixmap.data_mut().fill(0);

        let scale = self.scale_factor as f32;
        let transform = Transform::from_scale(scale, scale);

        let stages = [
            RenderPhase::Background,
            RenderPhase::Selection,
            RenderPhase::Content,
        ];

        for phase in stages {
            let ctx = RenderContext {
                scale_factor: self.scale_factor,
                selections,
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
            };

            Self::render_node(
                &mut pixmap,
                &mut self.glyph_renderer,
                &page.root,
                Point::zero(),
                transform,
                &ctx,
                &RenderHints::default(),
            );

            if phase == RenderPhase::Content {
                if let Some(indicator) = drop_indicator {
                    Self::render_drop_indicator(&mut pixmap, indicator, page_idx, transform, &ctx);
                }
            }
        }

        true
    }

    fn render_drop_indicator(
        pixmap: &mut PixmapMut,
        indicator: &DropIndicator,
        current_page_idx: usize,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let indicator_color = ctx.theme.color("ui.accent.brand.default");
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(indicator_color);
        paint.anti_alias = true;

        match indicator {
            DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
            } => {
                if *page_idx != current_page_idx {
                    return;
                }
                if let Some(rect) = Rect::from_xywh(*x, *y, 2.0, *height) {
                    pixmap.fill_rect(rect, &paint, transform, None);
                }
            }
            DropIndicator::Block {
                page_idx,
                x,
                y,
                width,
            } => {
                if *page_idx != current_page_idx {
                    return;
                }
                if let Some(rect) = Rect::from_xywh(*x, *y - 1.0, *width, 2.0) {
                    pixmap.fill_rect(rect, &paint, transform, None);
                }
            }
        }
    }

    fn render_node(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        let merged_hints = positioned.node.render_hints.merge(inherited_hints);

        let child_ctx_data = RenderContext {
            default_text_color: merged_hints
                .default_text_color
                .as_ref()
                .map(|color_key| ctx.theme.color(color_key))
                .or(ctx.default_text_color),
            ..*ctx
        };
        let render_ctx = &child_ctx_data;

        if let Some(ref element) = positioned.node.element {
            if let Some(render) = element.as_render() {
                let element_transform = transform.pre_translate(pos.x, pos.y);
                render.render(pixmap, glyph_renderer, element_transform, ctx);
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::render_node(
                    pixmap,
                    glyph_renderer,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                );
            }
        }
    }

    pub fn render_drag_image(
        &mut self,
        pages: &[Page],
        bounds: &crate::layout::query::DragImageBounds,
        selections: &[crate::model::SelectionDecor],
        doc: &Doc,
        visible_pages: &[usize],
        drag_page_idx: usize,
    ) -> Option<DragImageResult> {
        let visible_bounds: Vec<_> = bounds
            .pages
            .iter()
            .filter(|pb| visible_pages.contains(&pb.page_idx))
            .collect();

        if visible_bounds.is_empty() {
            return None;
        }

        let scale = self.scale_factor as f32;
        let page_y_offsets = Self::compute_page_y_offsets(pages, doc);
        let (min_x, min_y, total_width, total_height) =
            Self::compute_global_bounds(&visible_bounds, &page_y_offsets);

        let pixel_width = ((total_width * scale).ceil() as u32).max(1);
        let pixel_height = ((total_height * scale).ceil() as u32).max(1);
        let mut drag_pixmap = Pixmap::new(pixel_width, pixel_height)?;

        for pb in &visible_bounds {
            let page = pages.get(pb.page_idx)?;
            let page_y = page_y_offsets[pb.page_idx];

            let ctx = RenderContext {
                scale_factor: self.scale_factor,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: true,
                phase: RenderPhase::Content,
            };

            Self::render_page_part_inner(
                &mut self.glyph_renderer,
                page,
                pb,
                selections,
                page_y,
                min_x,
                min_y,
                scale,
                pixel_width,
                pixel_height,
                &ctx,
                &mut drag_pixmap,
            )?;
        }

        let drag_page_y = page_y_offsets.get(drag_page_idx).copied().unwrap_or(0.0);

        Some(DragImageResult {
            pixmap: drag_pixmap,
            width: pixel_width as u16,
            height: pixel_height as u16,
            offset_x: min_x,
            offset_y: min_y - drag_page_y,
            scale_factor: scale,
        })
    }

    fn compute_page_y_offsets(pages: &[Page], doc: &Doc) -> Vec<f32> {
        let settings = doc.settings();
        let gap = 24.0;
        let mut offsets = Vec::with_capacity(pages.len());
        let mut current_y = 0.0f32;

        for page in pages {
            offsets.push(current_y);
            let h = match settings.layout_mode {
                crate::model::LayoutMode::Paginated { page_height, .. } => page_height,
                crate::model::LayoutMode::Continuous { .. } => page.root.node.size.height,
            };
            current_y += h + gap;
        }
        offsets
    }

    fn compute_global_bounds(
        visible_bounds: &[&crate::layout::query::DragImagePageBounds],
        page_y_offsets: &[f32],
    ) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for pb in visible_bounds {
            let page_y = page_y_offsets.get(pb.page_idx).copied().unwrap_or(0.0);
            let global_x = pb.bounds.x;
            let global_y = page_y + pb.bounds.y;

            min_x = min_x.min(global_x);
            min_y = min_y.min(global_y);
            max_x = max_x.max(global_x + pb.bounds.width);
            max_y = max_y.max(global_y + pb.bounds.height);
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_page_part_inner(
        glyph_renderer: &mut GlyphRenderer,
        page: &Page,
        pb: &crate::layout::query::DragImagePageBounds,
        selections: &[crate::model::SelectionDecor],
        page_y: f32,
        min_x: f32,
        min_y: f32,
        scale: f32,
        pixel_width: u32,
        pixel_height: u32,
        ctx: &RenderContext<'_>,
        drag_pixmap: &mut Pixmap,
    ) -> Option<()> {
        let dest_x = pb.bounds.x - min_x;
        let dest_y = (page_y + pb.bounds.y) - min_y;

        let part_pixel_w = ((pb.bounds.width * scale).ceil() as u32).max(1);
        let part_pixel_h = ((pb.bounds.height * scale).ceil() as u32).max(1);

        let mut temp_pixmap = Pixmap::new(part_pixel_w, part_pixel_h)?;
        let transform =
            Transform::from_scale(scale, scale).pre_translate(-pb.bounds.x, -pb.bounds.y);

        Self::render_node(
            &mut temp_pixmap.as_mut(),
            glyph_renderer,
            &page.root,
            Point::zero(),
            transform,
            ctx,
            &RenderHints::default(),
        );

        let mut clip_rects = Vec::new();
        Self::collect_clip_rects(
            &page.root,
            Point::zero(),
            selections,
            Point::new(pb.bounds.x, pb.bounds.y),
            scale,
            &mut clip_rects,
        );

        if clip_rects.is_empty() {
            for cr in &pb.clip_rects {
                if let Some(rect) = Rect::from_xywh(
                    (cr.x - pb.bounds.x) * scale,
                    (cr.y - pb.bounds.y) * scale,
                    cr.width * scale,
                    cr.height * scale,
                ) {
                    clip_rects.push(rect);
                }
            }
        }

        Self::copy_clipped_pixels(
            &temp_pixmap,
            drag_pixmap,
            &clip_rects,
            (dest_x * scale).round() as i32,
            (dest_y * scale).round() as i32,
            part_pixel_w,
            part_pixel_h,
            pixel_width,
            pixel_height,
        );

        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_clipped_pixels(
        src: &Pixmap,
        dest: &mut Pixmap,
        clip_rects: &[Rect],
        dest_base_x: i32,
        dest_base_y: i32,
        src_width: u32,
        src_height: u32,
        dest_width: u32,
        dest_height: u32,
    ) {
        let src_data = src.data();
        let dest_data = dest.data_mut();

        for rect in clip_rects {
            let x_start = rect.x().floor() as i32;
            let y_start = rect.y().floor() as i32;
            let x_end = rect.right().ceil() as i32;
            let y_end = rect.bottom().ceil() as i32;

            for y in y_start..y_end {
                for x in x_start..x_end {
                    if x >= 0 && y >= 0 && (x as u32) < src_width && (y as u32) < src_height {
                        let src_idx = (y as u32 * src_width + x as u32) as usize * 4;
                        let dest_px = dest_base_x + x;
                        let dest_py = dest_base_y + y;

                        if dest_px >= 0
                            && dest_py >= 0
                            && (dest_px as u32) < dest_width
                            && (dest_py as u32) < dest_height
                        {
                            let dest_idx =
                                (dest_py as u32 * dest_width + dest_px as u32) as usize * 4;
                            if src_idx + 3 < src_data.len() && dest_idx + 3 < dest_data.len() {
                                dest_data[dest_idx..dest_idx + 4]
                                    .copy_from_slice(&src_data[src_idx..src_idx + 4]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_clip_rects(
        positioned: &PositionedNode,
        offset: Point,
        selections: &[crate::model::SelectionDecor],
        bounds_origin: Point,
        scale: f32,
        out: &mut Vec<Rect>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            match element {
                Element::Line(line) => {
                    let line_rects = line.compute_selection_rects(pos, selections);
                    for rect in line_rects {
                        if let Some(translated) = Rect::from_xywh(
                            (rect.x() - bounds_origin.x) * scale,
                            (rect.y() - bounds_origin.y) * scale,
                            rect.width() * scale,
                            rect.height() * scale,
                        ) {
                            out.push(translated);
                        }
                    }
                }
                _ => {
                    if let Some(block_id) = element.block_id() {
                        if selections.iter().any(|s| s.node_id() == block_id) {
                            let node_size = &positioned.node.size;
                            if let Some(translated) = Rect::from_xywh(
                                (pos.x - bounds_origin.x) * scale,
                                (pos.y - bounds_origin.y) * scale,
                                node_size.width * scale,
                                node_size.height * scale,
                            ) {
                                out.push(translated);
                            }
                        }
                    }
                }
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_clip_rects(child, pos, selections, bounds_origin, scale, out);
            }
        }
    }
}

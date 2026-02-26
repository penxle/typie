use super::*;
impl Renderer {
    pub(in super::super) fn render_node(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        clip: Option<CacheRect>,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        if let Some(clip_rect) = clip {
            if let Some(node_rect) = node_paint_bounds(positioned, pos)
                && !node_rect.intersects(clip_rect)
            {
                return;
            }
        }

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

        if let Some(ref element) = positioned.node.element
            && let Some(render) = element.as_render()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            render.render(pixmap, glyph_renderer, element_transform, render_ctx);
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
                    clip,
                );
            }
        }
    }

    pub(in super::super) fn outline_node(
        sink: &mut dyn ElementSink,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        clip: Option<CacheRect>,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        if let Some(clip_rect) = clip {
            if let Some(node_rect) = node_paint_bounds(positioned, pos)
                && !node_rect.intersects(clip_rect)
            {
                return;
            }
        }

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

        if let Some(ref element) = positioned.node.element
            && let Some(outline) = element.as_outline()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            outline.outline(sink, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::outline_node(sink, child, pos, transform, render_ctx, &merged_hints, clip);
            }
        }
    }

    pub(in super::super) fn should_render_next_page_overflow(positioned: &PositionedNode) -> bool {
        positioned
            .node
            .element
            .as_ref()
            .filter(|element| element.as_render().is_some())
            .map(|element| element.paint_overflow().top > 0.0)
            .unwrap_or(false)
    }

    pub(in super::super) fn render_node_for_next_page_overflow(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        cull_clip: CacheRect,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        let node_rect = node_paint_bounds(positioned, pos);
        if let Some(node_rect) = node_rect
            && !node_rect.intersects(cull_clip)
        {
            return;
        }

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

        if Self::should_render_next_page_overflow(positioned)
            && let Some(element) = positioned.node.element.as_ref()
            && let Some(render) = element.as_render()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            render.render(pixmap, glyph_renderer, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::render_node_for_next_page_overflow(
                    pixmap,
                    glyph_renderer,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                    cull_clip,
                );
            }
        }
    }

    pub(in super::super) fn outline_node_for_next_page_overflow(
        sink: &mut dyn ElementSink,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        cull_clip: CacheRect,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        let node_rect = node_paint_bounds(positioned, pos);
        if let Some(node_rect) = node_rect
            && !node_rect.intersects(cull_clip)
        {
            return;
        }

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

        if Self::should_render_next_page_overflow(positioned)
            && let Some(element) = positioned.node.element.as_ref()
            && let Some(outline) = element.as_outline()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            outline.outline(sink, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::outline_node_for_next_page_overflow(
                    sink,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                    cull_clip,
                );
            }
        }
    }
}

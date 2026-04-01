use super::*;

impl Renderer {
    /// Background phase: sink를 통해 배경 레이어 노드를 렌더링한다.
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_background_phase_to_sink(
        sink: &mut dyn RenderSink,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        doc: &Doc,
        clip: Option<LayoutRect>,
        origin: Point,
    ) {
        let scale = scale_factor as f32;
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64)
            * Affine::translate((-origin.x as f64, -origin.y as f64));
        let ctx = RenderParams {
            scale_factor,
            selections: &[],
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Background,
            render_origin: origin,
        };

        Self::render_node(
            sink,
            &page.root,
            Point::zero(),
            transform,
            &ctx,
            &RenderHints::default(),
            clip,
        );
    }

    /// Content phase: sink를 통해 콘텐츠 레이어 노드를 렌더링한다.
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_content_phase_to_sink(
        sink: &mut dyn RenderSink,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        doc: &Doc,
        clip: Option<LayoutRect>,
        origin: Point,
    ) {
        let scale = scale_factor as f32;
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64)
            * Affine::translate((-origin.x as f64, -origin.y as f64));
        let ctx = RenderParams {
            scale_factor,
            selections: &[],
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Content,
            render_origin: origin,
        };

        Self::render_node(
            sink,
            &page.root,
            Point::zero(),
            transform,
            &ctx,
            &RenderHints::default(),
            clip,
        );
    }

    /// Selection phase: sink를 통해 선택 오버레이 노드를 렌더링한다.
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_selection_phase_to_sink(
        sink: &mut dyn RenderSink,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        selections: &[SelectionDecor],
        doc: &Doc,
        clip: Option<LayoutRect>,
        origin: Point,
    ) {
        let scale = scale_factor as f32;
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64)
            * Affine::translate((-origin.x as f64, -origin.y as f64));
        let ctx = RenderParams {
            scale_factor,
            selections,
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Selection,
            render_origin: origin,
        };

        Self::render_node(
            sink,
            &page.root,
            Point::zero(),
            transform,
            &ctx,
            &RenderHints::default(),
            clip,
        );
    }
}

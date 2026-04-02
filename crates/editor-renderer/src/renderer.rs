use editor_model::Doc;
use editor_resource::Resource;
use editor_view::Page;
use std::sync::{Arc, Mutex};

use crate::glyph::{GlyphCache, ScaleContext};
use crate::sink::RenderSink;
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;
use crate::types::Transform;

pub struct Renderer {
    pub(crate) theme: Theme,
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) scale_ctx: ScaleContext,
    pub(crate) glyph_cache: GlyphCache,
}

impl Renderer {
    pub fn new(variant: ThemeVariant, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            theme: Theme::new(variant),
            resource,
            scale_ctx: ScaleContext::new(),
            glyph_cache: GlyphCache::new(),
        }
    }

    pub fn set_theme_variant(&mut self, variant: ThemeVariant) {
        self.theme.set_variant(variant);
    }

    pub fn render_page(
        &mut self,
        sink: &mut dyn RenderSink,
        page: &Page,
        doc: &Doc,
        scale_factor: f32,
    ) {
        let root = Transform::scale(scale_factor);
        for fragment in &page.fragments {
            crate::nodes::render_fragment(self, sink, fragment, doc, None, root);
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_macros::doc;
    use editor_model::NodeId;
    use editor_view::fragment::*;

    use super::*;
    use crate::types::Color;
    use crate::types::{Image, Path, Stroke};

    struct MockSink {
        fill_rect_count: usize,
        fill_path_count: usize,
        stroke_path_count: usize,
        draw_image_count: usize,
        fill_rect_transforms: Vec<Transform>,
    }

    impl MockSink {
        fn new() -> Self {
            Self {
                fill_rect_count: 0,
                fill_path_count: 0,
                stroke_path_count: 0,
                draw_image_count: 0,
                fill_rect_transforms: Vec::new(),
            }
        }
    }

    impl RenderSink for MockSink {
        fn fill_rect(&mut self, _rect: Rect, _color: Color, transform: Transform) {
            self.fill_rect_count += 1;
            self.fill_rect_transforms.push(transform);
        }
        fn fill_path(&mut self, _path: &Path, _color: Color, _transform: Transform) {
            self.fill_path_count += 1;
        }
        fn stroke_path(
            &mut self,
            _path: &Path,
            _color: Color,
            _stroke: &Stroke,
            _transform: Transform,
        ) {
            self.stroke_path_count += 1;
        }
        fn draw_image(&mut self, _image: &Image, _rect: Rect, _transform: Transform) {
            self.draw_image_count += 1;
        }
    }

    fn make_renderer() -> Renderer {
        use editor_resource::Resource;
        Renderer::new(
            ThemeVariant::LightWhite,
            Arc::new(Mutex::new(Resource::new())),
        )
    }

    fn make_line(node_id: NodeId, x: f32, y: f32) -> LineFragment {
        LineFragment {
            node_id,
            rect: Rect {
                x,
                y,
                width: 100.0,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![GlyphRun {
                font_id: 0,
                font_weight: 400,
                font_size: 14.0,
                synthesis: Synthesis::default(),
                color: "ui.text".into(),
                background_color: None,
                glyphs: vec![],
                node_id,
                offset: 0,
                text: "hi".into(),
                x: 0.0,
                width: 20.0,
                char_advances: vec![10.0, 10.0],
            }],
        }
    }

    #[test]
    fn render_empty_page() {
        let mut renderer = make_renderer();
        let doc = Doc::new_test();
        let page = Page::new(vec![], 0.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        assert_eq!(sink.fill_rect_count, 0);
        assert_eq!(sink.fill_path_count, 0);
        assert_eq!(sink.stroke_path_count, 0);
        assert_eq!(sink.draw_image_count, 0);
    }

    #[test]
    fn render_page_with_line() {
        let mut renderer = make_renderer();
        let doc = Doc::new_test();
        let node_id = NodeId::new();

        let line = make_line(node_id, 0.0, 0.0);
        let page = Page::new(vec![Fragment::Line(line)], 20.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        assert_eq!(sink.fill_rect_count, 0);
    }

    #[test]
    fn render_line_with_highlight() {
        let mut renderer = make_renderer();
        let doc = Doc::new_test();
        let node_id = NodeId::new();

        let line = LineFragment {
            node_id,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            baseline: 16.0,
            glyph_runs: vec![GlyphRun {
                font_id: 0,
                font_weight: 400,
                font_size: 14.0,
                synthesis: Synthesis::default(),
                color: "ui.text".into(),
                background_color: Some("ui.highlight".into()),
                glyphs: vec![],
                node_id,
                offset: 0,
                text: "hi".into(),
                x: 0.0,
                width: 20.0,
                char_advances: vec![10.0, 10.0],
            }],
        };

        let page = Page::new(vec![Fragment::Line(line)], 20.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        assert_eq!(sink.fill_rect_count, 1);
    }

    #[test]
    fn container_with_callout_background() {
        let mut renderer = make_renderer();
        let (doc, co1, ..) = doc! { root { co1: callout(variant: CalloutVariant::Info) } };

        let container = ContainerFragment {
            node_id: co1,
            rect: Rect {
                x: 10.0,
                y: 20.0,
                width: 200.0,
                height: 100.0,
            },
            children: vec![],
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets::default(),
        };

        let page = Page::new(vec![Fragment::Container(container)], 120.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        // Background fill_rect for callout
        assert_eq!(sink.fill_rect_count, 1);
    }

    #[test]
    fn container_with_border() {
        let mut renderer = make_renderer();
        let (doc, bq1, ..) =
            doc! { root { bq1: blockquote(variant: BlockquoteVariant::LeftLine) } };

        let container = ContainerFragment {
            node_id: bq1,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
            },
            children: vec![],
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets {
                left: 3.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
        };

        let page = Page::new(vec![Fragment::Container(container)], 100.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        // Left border drawn via fill_path
        assert_eq!(sink.fill_path_count, 1);
        // No background for blockquote
        assert_eq!(sink.fill_rect_count, 0);
    }

    #[test]
    fn nested_container_no_double_translation() {
        let mut renderer = make_renderer();
        let (doc, co1, p1, ..) = doc! {
            root {
                co1: callout(variant: CalloutVariant::Info) {
                    p1: paragraph
                }
            }
        };

        // Container at y=80, child line at absolute y=120
        let child_line = make_line(p1, 10.0, 120.0);

        let container = ContainerFragment {
            node_id: co1,
            rect: Rect {
                x: 0.0,
                y: 80.0,
                width: 200.0,
                height: 60.0,
            },
            children: vec![Fragment::Line(child_line)],
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets::default(),
        };

        let page = Page::new(vec![Fragment::Container(container)], 200.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        // fill_rect: 1 for callout background
        assert_eq!(sink.fill_rect_count, 1);

        // The callout background transform should be at (0, 80)
        let bg_transform = &sink.fill_rect_transforms[0];
        assert_eq!(bg_transform.m[4], 0.0); // x = 0
        assert_eq!(bg_transform.m[5], 80.0); // y = 80

        // Children receive the original page-level transform (IDENTITY),
        // so the line at absolute y=120 translates from IDENTITY,
        // not from the container's translated transform.
    }

    #[test]
    fn placeholder_callout_icon() {
        let mut renderer = make_renderer();
        let (doc, co1, ..) = doc! { root { co1: callout(variant: CalloutVariant::Warning) } };

        let placeholder = PlaceholderFragment {
            id: 0,
            rect: Rect {
                x: 5.0,
                y: 10.0,
                width: 16.0,
                height: 16.0,
            },
            data: PlaceholderData::None,
        };

        let container = ContainerFragment {
            node_id: co1,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 40.0,
            },
            children: vec![Fragment::Placeholder(placeholder)],
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets::default(),
        };

        let page = Page::new(vec![Fragment::Container(container)], 40.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        // fill_rect: 1 for callout background
        assert_eq!(sink.fill_rect_count, 1);
        // fill_path: 1 for placeholder icon
        assert_eq!(sink.fill_path_count, 1);
    }

    #[test]
    fn placeholder_fold_chevron() {
        let mut renderer = make_renderer();
        let (doc, f1, ..) = doc! { root { f1: fold } };

        let placeholder = PlaceholderFragment {
            id: 0,
            rect: Rect {
                x: 5.0,
                y: 5.0,
                width: 16.0,
                height: 16.0,
            },
            data: PlaceholderData::Bool(true),
        };

        let container = ContainerFragment {
            node_id: f1,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 40.0,
            },
            children: vec![Fragment::Placeholder(placeholder)],
            scope: false,
            breaks: Breaks::default(),
            border: EdgeInsets::default(),
        };

        let page = Page::new(vec![Fragment::Container(container)], 40.0);
        let mut sink = MockSink::new();
        renderer.render_page(&mut sink, &page, &doc, 1.0);

        // fill_rect: 1 for fold background
        assert_eq!(sink.fill_rect_count, 1);
        // fill_path: 1 for chevron icon
        assert_eq!(sink.fill_path_count, 1);
    }
}

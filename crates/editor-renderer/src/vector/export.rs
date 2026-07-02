use editor_common::Rect;
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::sink::RenderSink;
use crate::types::{Color, Image, Path, PathElement, Stroke, StrokeCap, StrokeJoin, Transform};
use crate::vector::types::{
    TextOp, VectorFillRule, VectorLineCap, VectorLineJoin, VectorOp, VectorPage, VectorPathCommand,
};

pub struct VectorSink {
    ops: Vec<VectorOp>,
    text_ops: Vec<TextOp>,
}

impl Default for VectorSink {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorSink {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            text_ops: Vec::new(),
        }
    }

    pub fn into_page(self, width: f32, height: f32) -> VectorPage {
        VectorPage {
            width,
            height,
            ops: self.ops,
            text_ops: self.text_ops,
        }
    }
}

impl RenderSink for VectorSink {
    fn pixel_size(&self) -> (u32, u32) {
        (0, 0)
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        self.fill_path(&Path::rect(rect), color, transform);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let cmds = path_to_commands(path, transform);
        if cmds.is_empty() {
            return;
        }
        self.ops.push(VectorOp::FillPath {
            path: cmds,
            color: color_to_rgba(color),
            fill_rule: VectorFillRule::Winding,
        });
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let cmds = path_to_commands(path, transform);
        if cmds.is_empty() {
            return;
        }
        self.ops.push(VectorOp::StrokePath {
            path: cmds,
            color: color_to_rgba(color),
            width: stroke.width,
            line_cap: match stroke.cap {
                StrokeCap::Butt => VectorLineCap::Butt,
                StrokeCap::Round => VectorLineCap::Round,
                StrokeCap::Square => VectorLineCap::Square,
            },
            line_join: match stroke.join {
                StrokeJoin::Miter => VectorLineJoin::Miter,
                StrokeJoin::Round => VectorLineJoin::Round,
                StrokeJoin::Bevel => VectorLineJoin::Bevel,
            },
        });
    }

    fn draw_image(&mut self, image: &Image, rect: Rect, transform: Transform) {
        let (x, y, render_width, render_height) = map_rect_bounds(transform, rect);
        self.ops.push(VectorOp::Image {
            data: image.data.clone(),
            width: image.width,
            height: image.height,
            x,
            y,
            render_width,
            render_height,
        });
    }

    fn draw_glyph_run(
        &mut self,
        run: &editor_view::glyph_run::GlyphRun,
        color: Color,
        base_transform: Transform,
        fonts: &editor_resource::FontRegistry,
    ) {
        let Some(font_data) = fonts.font_data(run.family_id, run.weight) else {
            return;
        };
        let Ok(font) = FontRef::from_index(font_data, 0) else {
            return;
        };
        let outlines = font.outline_glyphs();
        let coords: &[NormalizedCoord] = &[];
        let size = Size::new(run.font_size);
        let rgba = color_to_rgba(color);

        for g in &run.glyphs {
            if g.id == 0 {
                continue;
            }
            let Some(og) = outlines.get(GlyphId::new(g.id)) else {
                continue;
            };
            let glyph_t = base_transform.translate(g.x, g.y);
            let mut writer = GlyphOutlineWriter {
                cmds: Vec::new(),
                transform: glyph_t,
            };
            let settings = DrawSettings::unhinted(size, LocationRef::new(coords));
            if og.draw(settings, &mut writer).is_err() || writer.cmds.is_empty() {
                continue;
            }
            self.ops.push(VectorOp::FillPath {
                path: writer.cmds,
                color: rgba,
                fill_rule: VectorFillRule::Winding,
            });
        }

        if !run.text.is_empty() {
            let baseline_x = run.glyphs.first().map(|g| g.x).unwrap_or(run.x);
            let baseline_y = run.glyphs.first().map(|g| g.y).unwrap_or(0.0);
            let (tx, ty) = map_point(base_transform, baseline_x, baseline_y);
            self.text_ops.push(TextOp {
                text: run.text.clone(),
                x: tx,
                y: ty,
                size: run.font_size * base_transform.m[0],
            });
        }
    }
}

struct GlyphOutlineWriter {
    cmds: Vec<VectorPathCommand>,
    transform: Transform,
}

impl OutlinePen for GlyphOutlineWriter {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = map_glyph_point(self.transform, x, y);
        self.cmds.push(VectorPathCommand::MoveTo { x, y });
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = map_glyph_point(self.transform, x, y);
        self.cmds.push(VectorPathCommand::LineTo { x, y });
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx, cy) = map_glyph_point(self.transform, cx0, cy0);
        let (x, y) = map_glyph_point(self.transform, x, y);
        self.cmds.push(VectorPathCommand::QuadTo { cx, cy, x, y });
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (c1x, c1y) = map_glyph_point(self.transform, cx0, cy0);
        let (c2x, c2y) = map_glyph_point(self.transform, cx1, cy1);
        let (x, y) = map_glyph_point(self.transform, x, y);
        self.cmds.push(VectorPathCommand::CubicTo {
            c1x,
            c1y,
            c2x,
            c2y,
            x,
            y,
        });
    }
    fn close(&mut self) {
        self.cmds.push(VectorPathCommand::ClosePath);
    }
}

fn map_point(t: Transform, x: f32, y: f32) -> (f32, f32) {
    let [a, b, c, d, e, f] = t.m;
    (a * x + c * y + e, b * x + d * y + f)
}

fn map_rect_bounds(t: Transform, rect: Rect) -> (f32, f32, f32, f32) {
    let corners = [
        map_point(t, rect.x, rect.y),
        map_point(t, rect.x + rect.width, rect.y),
        map_point(t, rect.x, rect.y + rect.height),
        map_point(t, rect.x + rect.width, rect.y + rect.height),
    ];

    let min_x = corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f32::INFINITY, f32::min);
    let max_y = corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f32::NEG_INFINITY, f32::max);

    (min_x, min_y, max_x - min_x, max_y - min_y)
}

fn map_glyph_point(t: Transform, x: f32, y: f32) -> (f32, f32) {
    map_point(t, x, -y)
}

fn path_to_commands(path: &Path, transform: Transform) -> Vec<VectorPathCommand> {
    let mut cmds = Vec::new();
    for el in &path.elements {
        match *el {
            PathElement::MoveTo { x, y } => {
                let (x, y) = map_point(transform, x, y);
                cmds.push(VectorPathCommand::MoveTo { x, y });
            }
            PathElement::LineTo { x, y } => {
                let (x, y) = map_point(transform, x, y);
                cmds.push(VectorPathCommand::LineTo { x, y });
            }
            PathElement::QuadTo { x1, y1, x, y } => {
                let (cx, cy) = map_point(transform, x1, y1);
                let (x, y) = map_point(transform, x, y);
                cmds.push(VectorPathCommand::QuadTo { cx, cy, x, y });
            }
            PathElement::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let (c1x, c1y) = map_point(transform, x1, y1);
                let (c2x, c2y) = map_point(transform, x2, y2);
                let (x, y) = map_point(transform, x, y);
                cmds.push(VectorPathCommand::CubicTo {
                    c1x,
                    c1y,
                    c2x,
                    c2y,
                    x,
                    y,
                });
            }
            PathElement::Close => {
                cmds.push(VectorPathCommand::ClosePath);
            }
        }
    }
    cmds
}

fn color_to_rgba(c: Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Stroke;
    use crate::vector::types::VectorOp;
    use editor_common::Rect;

    fn red() -> Color {
        Color {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[test]
    fn fill_rect_produces_fill_path_op() {
        // 사각형 채우기 명령이 VectorPage에서 FillPath op로 기록되는지 확인한다.
        let mut sink = VectorSink::new();
        sink.fill_rect(
            Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            red(),
            Transform::IDENTITY,
        );
        let page = sink.into_page(100.0, 100.0);
        assert!(matches!(page.ops[0], VectorOp::FillPath { .. }));
    }

    #[test]
    fn fill_path_produces_fill_path_op() {
        // 일반 path 채우기 명령이 FillPath op로 보존되는지 확인한다.
        let mut sink = VectorSink::new();
        sink.fill_path(
            &Path::rect(Rect::from_xywh(0.0, 0.0, 10.0, 10.0)),
            red(),
            Transform::IDENTITY,
        );
        let page = sink.into_page(100.0, 100.0);
        assert!(matches!(page.ops[0], VectorOp::FillPath { .. }));
    }

    #[test]
    fn stroke_path_produces_stroke_path_op() {
        // stroke 정보가 포함된 path가 StrokePath op로 기록되는지 확인한다.
        let mut sink = VectorSink::new();
        sink.stroke_path(
            &Path::rect(Rect::from_xywh(0.0, 0.0, 10.0, 10.0)),
            red(),
            &Stroke::new(1.0),
            Transform::IDENTITY,
        );
        let page = sink.into_page(100.0, 100.0);
        assert!(matches!(page.ops[0], VectorOp::StrokePath { .. }));
    }

    #[test]
    fn draw_image_produces_image_op() {
        // 이미지 그리기 명령이 VectorPage에서 Image op로 바뀌는지 확인한다.
        let image = crate::types::Image {
            data: vec![0u8; 4].into(),
            width: 1,
            height: 1,
            glyph: None,
        };
        let mut sink = VectorSink::new();
        sink.draw_image(
            &image,
            Rect::from_xywh(5.0, 10.0, 1.0, 1.0),
            Transform::IDENTITY,
        );
        let page = sink.into_page(100.0, 100.0);
        assert!(matches!(page.ops[0], VectorOp::Image { .. }));
    }

    #[test]
    fn image_is_vectorized_as_image_op() {
        // 이미지 리소스가 표시 위치와 표시 크기를 포함한 Image op로 보존되는지 확인한다.
        let image = crate::types::Image {
            data: vec![1, 2, 3, 4].into(),
            width: 1,
            height: 1,
            glyph: None,
        };
        let mut sink = VectorSink::new();
        sink.draw_image(
            &image,
            Rect::from_xywh(5.0, 10.0, 3.0, 4.0),
            Transform::scale(2.0),
        );

        let page = sink.into_page(100.0, 100.0);
        match &page.ops[0] {
            VectorOp::Image {
                data,
                width,
                height,
                x,
                y,
                render_width,
                render_height,
            } => {
                assert_eq!(&data[..], &[1, 2, 3, 4]);
                assert_eq!(*width, 1);
                assert_eq!(*height, 1);
                assert!((*x - 10.0).abs() < 0.001);
                assert!((*y - 20.0).abs() < 0.001);
                assert!((*render_width - 6.0).abs() < 0.001);
                assert!((*render_height - 8.0).abs() < 0.001);
            }
            other => panic!("expected Image op, got {other:?}"),
        }
    }

    #[test]
    fn into_page_sets_dimensions() {
        // sink가 수집한 op를 페이지로 바꿀 때 width/height가 그대로 보존되는지 확인한다.
        let sink = VectorSink::new();
        let page = sink.into_page(123.0, 456.0);
        assert_eq!(page.width, 123.0);
        assert_eq!(page.height, 456.0);
    }

    #[test]
    fn draw_glyph_run_produces_fill_path_ops() {
        // 글리프 run을 그리면 텍스트가 래스터가 아니라 FillPath op들로 수집되는지 확인한다.
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};

        const TEST_FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
        let mut resource = editor_resource::Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource.font_registry.intern_id("test").unwrap();

        let run = GlyphRun {
            family_id,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![Glyph {
                id: 3,
                x: 0.0,
                y: 0.0,
            }],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "A".to_string(),
            x: 0.0,
            width: 10.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };

        let mut sink = VectorSink::new();
        sink.draw_glyph_run(&run, red(), Transform::IDENTITY, &resource.font_registry);
        let page = sink.into_page(100.0, 100.0);
        assert!(!page.ops.is_empty());
        assert!(
            page.ops
                .iter()
                .all(|op| matches!(op, VectorOp::FillPath { .. }))
        );
    }

    #[test]
    fn text_glyph_outline_is_vectorized_as_fill_paths() {
        // 텍스트는 래스터 이미지가 아니라 글리프 outline 기반 FillPath들로 수집되어야 한다.
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};

        const TEST_FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
        let mut resource = editor_resource::Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource.font_registry.intern_id("test").unwrap();

        let run = GlyphRun {
            family_id,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![Glyph {
                id: 3,
                x: 0.0,
                y: 0.0,
            }],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "A".to_string(),
            x: 0.0,
            width: 10.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };

        let mut sink = VectorSink::new();
        sink.draw_glyph_run(&run, red(), Transform::IDENTITY, &resource.font_registry);
        let page = sink.into_page(100.0, 100.0);

        assert!(!page.ops.is_empty());
        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
        assert!(
            page.ops
                .iter()
                .all(|op| matches!(op, VectorOp::FillPath { .. }))
        );
    }

    #[test]
    fn draw_glyph_run_emits_text_op() {
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};
        const TEST_FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
        let mut resource = editor_resource::Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource.font_registry.intern_id("test").unwrap();
        let run = GlyphRun {
            family_id,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![Glyph {
                id: 3,
                x: 0.0,
                y: 0.0,
            }],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "A".to_string(),
            x: 0.0,
            width: 10.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };
        let mut sink = VectorSink::new();
        sink.draw_glyph_run(&run, red(), Transform::IDENTITY, &resource.font_registry);
        let page = sink.into_page(100.0, 100.0);
        assert_eq!(page.text_ops.len(), 1);
        assert_eq!(page.text_ops[0].text, "A");
        assert_eq!(page.text_ops[0].size, 16.0);
    }

    #[test]
    fn draw_glyph_run_text_op_uses_first_glyph_origin_and_transform() {
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};
        const TEST_FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
        let mut resource = editor_resource::Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource.font_registry.intern_id("test").unwrap();
        let run = GlyphRun {
            family_id,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![Glyph {
                id: 3,
                x: 5.0,
                y: 8.0,
            }],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "A".to_string(),
            x: 99.0,
            width: 10.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };
        let mut sink = VectorSink::new();
        sink.draw_glyph_run(
            &run,
            red(),
            Transform::IDENTITY.translate(10.0, 20.0),
            &resource.font_registry,
        );
        let page = sink.into_page(100.0, 100.0);
        assert_eq!(page.text_ops.len(), 1);
        assert_eq!(page.text_ops[0].x, 15.0);
        assert_eq!(page.text_ops[0].y, 28.0);
        assert_eq!(page.text_ops[0].size, 16.0);

        let mut sink2 = VectorSink::new();
        sink2.draw_glyph_run(&run, red(), Transform::scale(2.0), &resource.font_registry);
        let page2 = sink2.into_page(100.0, 100.0);
        assert_eq!(page2.text_ops[0].x, 10.0);
        assert_eq!(page2.text_ops[0].y, 16.0);
        assert_eq!(page2.text_ops[0].size, 32.0);
    }

    #[test]
    fn glyph_outline_y_coordinates_are_flipped_into_screen_space() {
        // 글리프 outline은 폰트 좌표계(y-up)를 화면 좌표계(y-down)로 뒤집어 baseline 위쪽에 배치해야 한다.
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};

        const TEST_FONT: &[u8] = include_bytes!("../../../../assets/Pretendard-Regular.ttf");
        let mut resource = editor_resource::Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource.font_registry.intern_id("test").unwrap();
        let baseline_y = 20.0;

        let run = GlyphRun {
            family_id,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![Glyph {
                id: 3,
                x: 0.0,
                y: baseline_y,
            }],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "A".to_string(),
            x: 0.0,
            width: 10.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };

        let mut sink = VectorSink::new();
        sink.draw_glyph_run(&run, red(), Transform::IDENTITY, &resource.font_registry);
        let page = sink.into_page(100.0, 100.0);

        let mut ys = Vec::new();
        for op in &page.ops {
            if let VectorOp::FillPath { path, .. } = op {
                for cmd in path {
                    match cmd {
                        VectorPathCommand::MoveTo { y, .. }
                        | VectorPathCommand::LineTo { y, .. }
                        | VectorPathCommand::QuadTo { y, .. }
                        | VectorPathCommand::CubicTo { y, .. } => ys.push(*y),
                        VectorPathCommand::ClosePath => {}
                    }
                }
            }
        }

        assert!(!ys.is_empty());
        assert!(
            ys.iter().all(|y| *y <= baseline_y + 0.01),
            "glyph outline points must stay at or above the baseline in screen space"
        );
        assert!(
            ys.iter().any(|y| *y < baseline_y - 1.0),
            "glyph outline should extend upward from the baseline"
        );
    }
}

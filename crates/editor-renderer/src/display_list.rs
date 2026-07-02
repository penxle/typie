use std::hash::{Hash, Hasher};
use std::sync::Arc;

use editor_common::{Color, Rect};
use editor_resource::FontRegistry;
use editor_view::glyph_run::GlyphRun;

use crate::backend::cpu::raster::{MIN_STROKE_WIDTH, MITER_LIMIT};
use crate::damage::IRect;
use crate::glyph::GlyphKey;
use crate::sink::RenderSink;
use crate::types::{Image, Path, PathElement, Stroke, StrokeCap, StrokeJoin, Transform};

pub enum PrimPayload {
    FillRect {
        rect: Rect,
        color: Color,
        transform: Transform,
    },
    FillPath {
        path: Path,
        color: Color,
        transform: Transform,
    },
    StrokePath {
        path: Path,
        color: Color,
        stroke: Stroke,
        transform: Transform,
    },
    Glyph {
        image: Arc<Image>,
        key: GlyphKey,
        dst_x: i32,
        dst_y: i32,
    },
}

pub struct Primitive {
    pub bounds: IRect,
    pub key: u64,
    pub payload: PrimPayload,
}

#[derive(Default)]
pub struct DisplayList {
    pub primitives: Vec<Primitive>,
}

pub struct DisplayListRecorder {
    page_bounds: IRect,
    list: DisplayList,
}

impl DisplayListRecorder {
    pub fn new(page_bounds: IRect) -> Self {
        Self {
            page_bounds,
            list: DisplayList::default(),
        }
    }

    pub fn into_list(self) -> DisplayList {
        self.list
    }

    fn push(&mut self, device_bounds: Option<Rect>, key: u64, payload: PrimPayload) {
        let Some(b) = device_bounds else {
            return;
        };
        let Some(ib) = IRect::from_rect_padded(b, 1, self.page_bounds) else {
            return;
        };
        self.list.primitives.push(Primitive {
            bounds: ib,
            key,
            payload,
        });
    }
}

fn device_bounds(local: Rect, t: Transform) -> Rect {
    let [a, b, c, d, e, f] = t.m;
    let map = |x: f32, y: f32| (a * x + c * y + e, b * x + d * y + f);
    let corners = [
        map(local.x, local.y),
        map(local.x + local.width, local.y),
        map(local.x, local.y + local.height),
        map(local.x + local.width, local.y + local.height),
    ];
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in corners {
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    Rect::from_xywh(minx, miny, maxx - minx, maxy - miny)
}

fn hash_bits(items: &[u64]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &i in items {
        i.hash(&mut h);
    }
    h.finish()
}

fn color_bits(c: Color) -> u64 {
    (c.r as u64) | (c.g as u64) << 8 | (c.b as u64) << 16 | (c.a as u64) << 24
}

fn tf_bits(t: Transform) -> u64 {
    hash_bits(&t.m.iter().map(|f| f.to_bits() as u64).collect::<Vec<_>>())
}

fn rect_bits(r: Rect) -> u64 {
    hash_bits(&[
        r.x.to_bits() as u64,
        r.y.to_bits() as u64,
        r.width.to_bits() as u64,
        r.height.to_bits() as u64,
    ])
}

fn path_bits(path: &Path) -> u64 {
    let mut v: Vec<u64> = Vec::with_capacity(path.elements.len() * 2);
    for el in &path.elements {
        match *el {
            PathElement::MoveTo { x, y } => {
                v.push(0);
                v.push(x.to_bits() as u64);
                v.push(y.to_bits() as u64);
            }
            PathElement::LineTo { x, y } => {
                v.push(1);
                v.push(x.to_bits() as u64);
                v.push(y.to_bits() as u64);
            }
            PathElement::QuadTo { x1, y1, x, y } => {
                v.push(2);
                v.extend([x1, y1, x, y].map(|f| f.to_bits() as u64));
            }
            PathElement::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                v.push(3);
                v.extend([x1, y1, x2, y2, x, y].map(|f| f.to_bits() as u64));
            }
            PathElement::Close => v.push(4),
        }
    }
    hash_bits(&v)
}

fn cap_join_bits(s: &Stroke) -> u64 {
    let cap = match s.cap {
        StrokeCap::Butt => 0u64,
        StrokeCap::Round => 1,
        StrokeCap::Square => 2,
    };
    let join = match s.join {
        StrokeJoin::Miter => 0u64,
        StrokeJoin::Round => 1,
        StrokeJoin::Bevel => 2,
    };
    cap | (join << 8)
}

fn glyph_key_bits(g: &GlyphKey) -> u64 {
    let ck = &g.cache_key;
    hash_bits(&[
        ck.family_id as u64,
        ck.weight as u64,
        ck.glyph_id as u64,
        ck.size_q4 as u64,
        ck.has_skew as u64,
        ck.embolden as u64,
        ck.subpixel_x as u64,
        color_bits(g.color),
        g.font_generation,
    ])
}

impl RenderSink for DisplayListRecorder {
    fn pixel_size(&self) -> (u32, u32) {
        (
            self.page_bounds.width() as u32,
            self.page_bounds.height() as u32,
        )
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        let key = hash_bits(&[1, rect_bits(rect), color_bits(color), tf_bits(transform)]);
        self.push(
            Some(device_bounds(rect, transform)),
            key,
            PrimPayload::FillRect {
                rect,
                color,
                transform,
            },
        );
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let key = hash_bits(&[2, path_bits(path), color_bits(color), tf_bits(transform)]);
        let bounds = path.bounds().map(|b| device_bounds(b, transform));
        self.push(
            bounds,
            key,
            PrimPayload::FillPath {
                path: path.clone(),
                color,
                transform,
            },
        );
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let key = hash_bits(&[
            3,
            path_bits(path),
            color_bits(color),
            stroke.width.to_bits() as u64,
            cap_join_bits(stroke),
            tf_bits(transform),
        ]);
        let eff_w = stroke.width.max(MIN_STROKE_WIDTH);
        let pad = eff_w * 0.5 * MITER_LIMIT;
        let bounds = path.bounds().map(|b| {
            device_bounds(
                Rect::from_xywh(
                    b.x - pad,
                    b.y - pad,
                    b.width + pad * 2.0,
                    b.height + pad * 2.0,
                ),
                transform,
            )
        });
        self.push(
            bounds,
            key,
            PrimPayload::StrokePath {
                path: path.clone(),
                color,
                stroke: *stroke,
                transform,
            },
        );
    }

    fn draw_image(&mut self, _image: &Image, _rect: Rect, _transform: Transform) {}

    fn draw_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        let Some(gk) = image.glyph else {
            return;
        };
        let key = hash_bits(&[5, glyph_key_bits(&gk), dst_x as u64, dst_y as u64]);
        let bounds = Some(Rect::from_xywh(
            dst_x as f32,
            dst_y as f32,
            image.width as f32,
            image.height as f32,
        ));
        self.push(
            bounds,
            key,
            PrimPayload::Glyph {
                image: Arc::new(image.clone()),
                key: gk,
                dst_x,
                dst_y,
            },
        );
    }

    fn draw_glyph_run(
        &mut self,
        _run: &GlyphRun,
        _color: Color,
        _transform: Transform,
        _fonts: &FontRegistry,
    ) {
        debug_assert!(
            false,
            "DisplayListRecorder는 Raster 텍스트 모드로만 구동되어야 한다; draw_glyph_run 호출은 텍스트가 display list에서 누락(under-damage)됨을 의미"
        );
    }
}

impl Primitive {
    pub fn same_content(&self, other: &Primitive) -> bool {
        use PrimPayload::*;
        match (&self.payload, &other.payload) {
            (
                Glyph {
                    key: ka,
                    dst_x: ax,
                    dst_y: ay,
                    ..
                },
                Glyph {
                    key: kb,
                    dst_x: bx,
                    dst_y: by,
                    ..
                },
            ) => ka == kb && ax == bx && ay == by,
            (
                FillRect {
                    rect: a,
                    color: ca,
                    transform: ta,
                },
                FillRect {
                    rect: b,
                    color: cb,
                    transform: tb,
                },
            ) => a == b && ca == cb && ta == tb,
            (
                FillPath {
                    path: a,
                    color: ca,
                    transform: ta,
                },
                FillPath {
                    path: b,
                    color: cb,
                    transform: tb,
                },
            ) => a == b && ca == cb && ta == tb,
            (
                StrokePath {
                    path: a,
                    color: ca,
                    stroke: sa,
                    transform: ta,
                },
                StrokePath {
                    path: b,
                    color: cb,
                    stroke: sb,
                    transform: tb,
                },
            ) => a == b && ca == cb && sa == sb && ta == tb,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::IRect;
    use crate::sink::RenderSink;
    use crate::types::{Color, Transform};

    fn full() -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 100,
        }
    }

    #[test]
    fn records_fill_rect_with_bounds_and_key() {
        let mut rec = DisplayListRecorder::new(full());
        rec.fill_rect(
            editor_common::Rect::from_xywh(10.0, 10.0, 20.0, 20.0),
            Color::new(1, 2, 3, 255),
            Transform::IDENTITY,
        );
        let dl = rec.into_list();
        assert_eq!(dl.primitives.len(), 1);
        let b = dl.primitives[0].bounds;
        assert!(b.x0 <= 10 && b.y0 <= 10 && b.x1 >= 30 && b.y1 >= 30);
    }

    #[test]
    fn same_input_same_key() {
        let mut a = DisplayListRecorder::new(full());
        let mut b = DisplayListRecorder::new(full());
        let r = editor_common::Rect::from_xywh(5.0, 5.0, 4.0, 4.0);
        a.fill_rect(r, Color::new(9, 9, 9, 255), Transform::IDENTITY);
        b.fill_rect(r, Color::new(9, 9, 9, 255), Transform::IDENTITY);
        let (da, db) = (a.into_list(), b.into_list());
        assert_eq!(da.primitives[0].key, db.primitives[0].key);
        assert!(da.primitives[0].same_content(&db.primitives[0]));
    }

    #[test]
    fn different_color_different_key() {
        let mut a = DisplayListRecorder::new(full());
        let mut b = DisplayListRecorder::new(full());
        let r = editor_common::Rect::from_xywh(5.0, 5.0, 4.0, 4.0);
        a.fill_rect(r, Color::new(9, 9, 9, 255), Transform::IDENTITY);
        b.fill_rect(r, Color::new(8, 9, 9, 255), Transform::IDENTITY);
        assert_ne!(
            a.into_list().primitives[0].key,
            b.into_list().primitives[0].key
        );
    }

    #[test]
    fn same_content_is_structural_and_resolves_key_collision() {
        let b = full();
        let mk = |rect| Primitive {
            bounds: b,
            key: 42,
            payload: PrimPayload::FillRect {
                rect,
                color: Color::new(1, 2, 3, 255),
                transform: Transform::IDENTITY,
            },
        };
        let a = mk(editor_common::Rect::from_xywh(0.0, 0.0, 2.0, 2.0));
        let c = mk(editor_common::Rect::from_xywh(1.0, 1.0, 2.0, 2.0));
        assert!(a.same_content(&a));
        assert!(
            !a.same_content(&c),
            "forced key collision with different content must not match"
        );
    }

    #[test]
    fn stroke_cap_and_join_change_break_match_and_key() {
        use crate::types::{Path, Stroke, StrokeCap, StrokeJoin};
        let path = Path::rect(editor_common::Rect::from_xywh(2.0, 2.0, 10.0, 10.0));
        let mk = |cap, join| {
            let mut rec = DisplayListRecorder::new(full());
            rec.stroke_path(
                &path,
                Color::new(0, 0, 0, 255),
                &Stroke {
                    width: 2.0,
                    cap,
                    join,
                },
                Transform::IDENTITY,
            );
            rec.into_list()
        };
        let base = mk(StrokeCap::Butt, StrokeJoin::Miter);
        let cap_changed = mk(StrokeCap::Round, StrokeJoin::Miter);
        let join_changed = mk(StrokeCap::Butt, StrokeJoin::Round);
        assert!(!base.primitives[0].same_content(&cap_changed.primitives[0]));
        assert_ne!(base.primitives[0].key, cap_changed.primitives[0].key);
        assert!(!base.primitives[0].same_content(&join_changed.primitives[0]));
        assert_ne!(base.primitives[0].key, join_changed.primitives[0].key);
    }

    #[test]
    fn recorded_bounds_are_device_space_under_scale_translate() {
        let mut rec = DisplayListRecorder::new(IRect {
            x0: 0,
            y0: 0,
            x1: 200,
            y1: 200,
        });
        let t = Transform::scale(2.0).translate(50.0, 60.0);
        rec.fill_rect(
            editor_common::Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            Color::new(1, 1, 1, 255),
            t,
        );
        let bd = rec.into_list().primitives[0].bounds;
        assert_eq!(
            (bd.x0, bd.y0, bd.x1, bd.y1),
            (99, 119, 121, 141),
            "bounds must be device-space (transform applied), AA-padded by 1"
        );
    }

    #[test]
    fn device_bounds_conservative_under_rotation_and_skew() {
        let c = std::f32::consts::FRAC_1_SQRT_2;
        let rot = Transform {
            m: [c, c, -c, c, 100.0, 100.0],
        };
        let mut rec = DisplayListRecorder::new(IRect {
            x0: 0,
            y0: 0,
            x1: 400,
            y1: 400,
        });
        rec.fill_rect(
            editor_common::Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
            Color::new(1, 1, 1, 255),
            rot,
        );
        let bd = rec.into_list().primitives[0].bounds;
        assert!(
            bd.x1 - bd.x0 >= 28 && bd.y1 - bd.y0 >= 28,
            "rotated square's device bbox must cover the diamond (only 2-corner mapping would under-cover)"
        );

        let skew = Transform {
            m: [1.0, 0.0, 0.5, 1.0, 50.0, 50.0],
        };
        let mut rec2 = DisplayListRecorder::new(IRect {
            x0: 0,
            y0: 0,
            x1: 400,
            y1: 400,
        });
        rec2.fill_rect(
            editor_common::Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
            Color::new(1, 1, 1, 255),
            skew,
        );
        let bd2 = rec2.into_list().primitives[0].bounds;
        assert!(
            bd2.x1 - bd2.x0 >= 30,
            "skew must widen device bbox (20 + 0.5*20)"
        );
    }
}

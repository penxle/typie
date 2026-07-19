use std::sync::Arc;

use editor_common::Rect;
use editor_view::Edges;

pub use editor_common::Color;

pub(crate) use crate::glyph::GlyphKey;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub m: [f32; 6],
}

impl Transform {
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    pub fn scale(s: f32) -> Self {
        Self {
            m: [s, 0.0, 0.0, s, 0.0, 0.0],
        }
    }

    pub fn translate(self, tx: f32, ty: f32) -> Self {
        let [a, b, c, d, e, f] = self.m;
        Self {
            m: [a, b, c, d, a * tx + c * ty + e, b * tx + d * ty + f],
        }
    }

    /// Post-multiply by a uniform scale, scaling path coordinates without affecting the accumulated translation.
    pub fn post_scale(self, s: f32) -> Self {
        let [a, b, c, d, e, f] = self.m;
        Self {
            m: [a * s, b * s, c * s, d * s, e, f],
        }
    }

    /// Output(device)-space offset: shifts where the transform lands without
    /// reinterpreting the offset in the transform's local basis (unlike `translate`).
    pub fn translate_device(self, tx: f32, ty: f32) -> Self {
        let [a, b, c, d, e, f] = self.m;
        Self {
            m: [a, b, c, d, e + tx, f + ty],
        }
    }
}

impl From<Transform> for kurbo::Affine {
    fn from(t: Transform) -> Self {
        let [a, b, c, d, e, f] = t.m;
        Self::new([a as f64, b as f64, c as f64, d as f64, e as f64, f as f64])
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathElement {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    QuadTo {
        x1: f32,
        y1: f32,
        x: f32,
        y: f32,
    },
    CurveTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x: f32,
        y: f32,
    },
    Close,
}

#[derive(Debug, Clone, Copy)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub fn from_edges(radius: f32, edges: &Edges<bool>) -> Self {
        let top = if edges.top { radius } else { 0.0 };
        let bottom = if edges.bottom { radius } else { 0.0 };
        Self {
            top_left: top,
            top_right: top,
            bottom_right: bottom,
            bottom_left: bottom,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Path {
    pub elements: Vec<PathElement>,
}

impl Path {
    pub fn rect(r: Rect) -> Self {
        Self {
            elements: vec![
                PathElement::MoveTo { x: r.x, y: r.y },
                PathElement::LineTo {
                    x: r.x + r.width,
                    y: r.y,
                },
                PathElement::LineTo {
                    x: r.x + r.width,
                    y: r.y + r.height,
                },
                PathElement::LineTo {
                    x: r.x,
                    y: r.y + r.height,
                },
                PathElement::Close,
            ],
        }
    }

    pub fn rrect(r: Rect, radii: CornerRadii) -> Self {
        let CornerRadii {
            top_left: tl,
            top_right: tr,
            bottom_right: br,
            bottom_left: bl,
        } = radii;
        let mut elements = Vec::new();

        elements.push(PathElement::MoveTo {
            x: r.x + tl,
            y: r.y,
        });
        elements.push(PathElement::LineTo {
            x: r.x + r.width - tr,
            y: r.y,
        });
        if tr > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x + r.width,
                y1: r.y,
                x: r.x + r.width,
                y: r.y + tr,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + r.width,
            y: r.y + r.height - br,
        });
        if br > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x + r.width,
                y1: r.y + r.height,
                x: r.x + r.width - br,
                y: r.y + r.height,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + bl,
            y: r.y + r.height,
        });
        if bl > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x,
                y1: r.y + r.height,
                x: r.x,
                y: r.y + r.height - bl,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x,
            y: r.y + tl,
        });
        if tl > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x,
                y1: r.y,
                x: r.x + tl,
                y: r.y,
            });
        }
        elements.push(PathElement::Close);

        Self { elements }
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut acc = |x: f32, y: f32| {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        };
        for el in &self.elements {
            match *el {
                PathElement::MoveTo { x, y } | PathElement::LineTo { x, y } => acc(x, y),
                PathElement::QuadTo { x1, y1, x, y } => {
                    acc(x1, y1);
                    acc(x, y);
                }
                PathElement::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    acc(x1, y1);
                    acc(x2, y2);
                    acc(x, y);
                }
                PathElement::Close => {}
            }
        }
        if min_x > max_x {
            return None;
        }
        Some(Rect::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y))
    }
}

impl From<&Path> for kurbo::BezPath {
    fn from(path: &Path) -> Self {
        let mut bp = Self::new();
        for el in &path.elements {
            match *el {
                PathElement::MoveTo { x, y } => bp.move_to((x as f64, y as f64)),
                PathElement::LineTo { x, y } => bp.line_to((x as f64, y as f64)),
                PathElement::QuadTo { x1, y1, x, y } => {
                    bp.quad_to((x1 as f64, y1 as f64), (x as f64, y as f64))
                }
                PathElement::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => bp.curve_to(
                    (x1 as f64, y1 as f64),
                    (x2 as f64, y2 as f64),
                    (x as f64, y as f64),
                ),
                PathElement::Close => bp.close_path(),
            }
        }
        bp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy)]
pub enum IconElement {
    Fill {
        path: &'static [PathElement],
        fill_rule: FillRule,
    },
    Stroke {
        path: &'static [PathElement],
        stroke_cap: StrokeCap,
        stroke_join: StrokeJoin,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct IconData {
    pub viewport: (f32, f32),
    pub elements: &'static [IconElement],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
}

impl Stroke {
    pub fn new(width: f32) -> Self {
        Self {
            width,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
        }
    }
}

impl From<StrokeCap> for kurbo::Cap {
    fn from(cap: StrokeCap) -> Self {
        match cap {
            StrokeCap::Butt => Self::Butt,
            StrokeCap::Round => Self::Round,
            StrokeCap::Square => Self::Square,
        }
    }
}

impl From<StrokeJoin> for kurbo::Join {
    fn from(join: StrokeJoin) -> Self {
        match join {
            StrokeJoin::Miter => Self::Miter,
            StrokeJoin::Round => Self::Round,
            StrokeJoin::Bevel => Self::Bevel,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub data: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    pub glyph: Option<GlyphKey>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrect_all_corners_rounded() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let radii = CornerRadii {
            top_left: 8.0,
            top_right: 8.0,
            bottom_right: 8.0,
            bottom_left: 8.0,
        };
        let path = Path::rrect(r, radii);
        let quad_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 4);
        assert!(matches!(path.elements.last(), Some(PathElement::Close)));
    }

    #[test]
    fn rrect_zero_radius_no_quads() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let radii = CornerRadii {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        };
        let path = Path::rrect(r, radii);
        let quad_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 0);
    }

    #[test]
    fn rrect_partial_corners() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let radii = CornerRadii {
            top_left: 8.0,
            top_right: 8.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        };
        let path = Path::rrect(r, radii);
        let quad_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::QuadTo { .. }))
            .count();
        assert_eq!(quad_count, 2);
    }

    #[test]
    fn corner_radii_from_edges_all_visible() {
        let edges = Edges {
            top: true,
            bottom: true,
            left: true,
            right: true,
        };
        let radii = CornerRadii::from_edges(8.0, &edges);
        assert_eq!(radii.top_left, 8.0);
        assert_eq!(radii.bottom_right, 8.0);
    }

    #[test]
    fn corner_radii_from_edges_top_split() {
        let edges = Edges {
            top: false,
            bottom: true,
            left: true,
            right: true,
        };
        let radii = CornerRadii::from_edges(8.0, &edges);
        assert_eq!(radii.top_left, 0.0);
        assert_eq!(radii.top_right, 0.0);
        assert_eq!(radii.bottom_left, 8.0);
        assert_eq!(radii.bottom_right, 8.0);
    }

    #[test]
    fn path_bounds_covers_all_points() {
        let p = Path::rect(editor_common::Rect::from_xywh(10.0, 20.0, 30.0, 40.0));
        let b = p.bounds().unwrap();
        assert_eq!((b.x, b.y, b.width, b.height), (10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn path_bounds_includes_quad_control_point() {
        let p = Path {
            elements: vec![
                PathElement::MoveTo { x: 0.0, y: 0.0 },
                PathElement::QuadTo {
                    x1: 50.0,
                    y1: 100.0,
                    x: 10.0,
                    y: 0.0,
                },
            ],
        };
        let b = p.bounds().unwrap();
        assert!(b.bottom() >= 100.0);
        assert!(b.right() >= 50.0);
    }

    #[test]
    fn path_bounds_empty_is_none() {
        assert!(Path::default().bounds().is_none());
    }
}

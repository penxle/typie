use editor_common::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
}

impl From<Color> for peniko::color::AlphaColor<peniko::color::Srgb> {
    fn from(c: Color) -> Self {
        Self::from_rgba8(c.r, c.g, c.b, c.a)
    }
}

#[derive(Debug, Clone, Copy)]
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

    /// Post-multiply by a uniform scale. Scales path coordinates without
    /// affecting the accumulated translation.
    pub fn post_scale(self, s: f32) -> Self {
        let [a, b, c, d, e, f] = self.m;
        Self {
            m: [a * s, b * s, c * s, d * s, e, f],
        }
    }
}

impl From<Transform> for kurbo::Affine {
    fn from(t: Transform) -> Self {
        let [a, b, c, d, e, f] = t.m;
        Self::new([a as f64, b as f64, c as f64, d as f64, e as f64, f as f64])
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Copy)]
pub struct Stroke {
    pub width: f32,
}

impl Stroke {
    pub fn new(width: f32) -> Self {
        Self { width }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[allow(dead_code)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[allow(dead_code)]
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[allow(dead_code)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[allow(dead_code)]
impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn center(&self) -> Point {
        Point::new(self.width / 2.0, self.height / 2.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[allow(dead_code)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[allow(dead_code)]
pub struct TextBound {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
}

#[allow(dead_code)]
impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + (self.width / 2.0), self.y + (self.height / 2.0))
    }

    pub fn with_padding(&self, padding: EdgeInsets) -> Self {
        Self {
            x: self.x + padding.left,
            y: self.y + padding.top,
            width: self.width - padding.left - padding.right,
            height: self.height - padding.top - padding.bottom,
        }
    }
}

#[allow(dead_code)]
impl TextBound {
    pub fn new(x: f32, y: f32, width: f32, height: f32, ascent: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            ascent,
        }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[allow(dead_code)]
impl EdgeInsets {
    pub fn all(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    pub fn zero() -> Self {
        Self::all(0.0)
    }
}

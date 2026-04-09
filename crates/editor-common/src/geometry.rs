use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        top: 0.0,
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
    };

    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            left: value,
            bottom: value,
            right: value,
        }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            left: horizontal,
            bottom: vertical,
            right: horizontal,
        }
    }
}

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[ffi]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_bottom_and_right() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 30.0,
        };
        assert_eq!(r.bottom(), 50.0);
        assert_eq!(r.right(), 110.0);
    }

    #[test]
    fn rect_center_x() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 30.0,
        };
        assert_eq!(r.center_x(), 60.0);
    }

    #[test]
    fn rect_contains() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 30.0,
        };
        assert!(r.contains(50.0, 35.0));
        assert!(!r.contains(5.0, 35.0));
        assert!(!r.contains(50.0, 55.0));
    }

    #[test]
    fn size_default_is_zero() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }
}

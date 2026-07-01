use editor_common::{Rect, Size};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

/// A y-range window into the LayoutTree produced by the two-pass layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPage {
    /// Physical page top in document coordinates.
    pub y_start: f32,
    /// Physical page bottom in document coordinates.
    pub y_end: f32,
    /// Top of the drawable content window in document coordinates.
    pub content_y_start: f32,
    /// Bottom of the drawable content window in document coordinates.
    pub content_y_end: f32,
    pub size: Size,
}

impl LayoutPage {
    pub fn new(y_start: f32, y_end: f32, size: Size) -> Self {
        Self::with_content(y_start, y_end, y_start, y_end, size)
    }

    pub fn with_content(
        y_start: f32,
        y_end: f32,
        content_y_start: f32,
        content_y_end: f32,
        size: Size,
    ) -> Self {
        debug_assert!(y_start.is_finite() && y_end.is_finite());
        Self {
            y_start,
            y_end,
            content_y_start,
            content_y_end,
            size,
        }
    }
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PageRect<T = ()> {
    pub page_idx: usize,
    pub rect: Rect,
    #[serde(skip)]
    pub meta: T,
}

impl PageRect {
    pub fn new(page_idx: usize, rect: Rect) -> Self {
        Self {
            page_idx,
            rect,
            meta: (),
        }
    }
}

impl<T> PageRect<T> {
    pub fn with_meta(page_idx: usize, rect: Rect, meta: T) -> Self {
        Self {
            page_idx,
            rect,
            meta,
        }
    }

    pub fn without_meta(&self) -> PageRect<()> {
        PageRect {
            page_idx: self.page_idx,
            rect: self.rect,
            meta: (),
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_common::Rect;

    use super::*;

    #[test]
    fn without_meta_strips_meta() {
        let rect = PageRect::with_meta(2, Rect::from_xywh(10.0, 20.0, 100.0, 50.0), 42u32);
        let stripped = rect.without_meta();
        assert_eq!(stripped.page_idx, 2);
        assert_eq!(stripped.rect, Rect::from_xywh(10.0, 20.0, 100.0, 50.0));
        assert_eq!(stripped.meta, ());
    }
}

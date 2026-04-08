use editor_common::{Rect, Size};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

/// A y-range window into the LayoutTree produced by the two-pass layout.
#[derive(Debug, Clone)]
pub struct LayoutPage {
    pub y_start: f32,
    pub y_end: f32,
    pub size: Size,
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
}

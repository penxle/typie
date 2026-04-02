use editor_common::{Rect, Size};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::fragment::Fragment;

#[derive(Debug, Clone)]
pub struct Page {
    pub size: Size,
    pub fragments: Vec<Fragment>,
}

impl Page {
    pub fn new(size: Size, fragments: Vec<Fragment>) -> Self {
        Self { size, fragments }
    }
}

#[ffi]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PageRect {
    pub page_idx: usize,
    pub rect: Rect,
}

impl PageRect {
    pub fn new(page_idx: usize, rect: Rect) -> Self {
        Self { page_idx, rect }
    }
}

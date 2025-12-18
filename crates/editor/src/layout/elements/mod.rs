pub mod blockquote;
pub mod callout;
pub mod external;
pub mod fold;
pub mod horizontal_rule;
pub mod line;
pub mod list_marker;

pub use blockquote::*;
pub use callout::*;
pub use external::*;
pub use fold::*;
pub use horizontal_rule::*;
pub use line::*;
pub use list_marker::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct WrapperPadding {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl WrapperPadding {
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SplitEdges {
    pub top: bool,
    pub bottom: bool,
}

pub trait Wrapper {
    fn padding(&self) -> WrapperPadding;

    // 페이지 끝에 빈 wrapper만 남기고 children은 다음 페이지로 넘어가는 것을 방지할 것인지 여부
    fn prevent_empty_on_page_break(&self) -> bool {
        false
    }
}

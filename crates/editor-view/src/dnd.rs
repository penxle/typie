use editor_common::Rect;
use editor_state::Position;

use crate::PageRect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropIndicator {
    Inline {
        page_idx: usize,
        x: f32,
        y: f32,
        height: f32,
    },
    Block {
        page_idx: usize,
        x: f32,
        y: f32,
        width: f32,
    },
}

impl DropIndicator {
    pub fn rect(&self) -> PageRect {
        match *self {
            Self::Inline {
                page_idx,
                x,
                y,
                height,
            } => PageRect::new(page_idx, Rect::from_xywh(x, y, 2.0, height)),
            Self::Block {
                page_idx,
                x,
                y,
                width,
            } => PageRect::new(page_idx, Rect::from_xywh(x, y - 1.0, width, 2.0)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropTarget {
    pub position: Position,
    pub indicator: DropIndicator,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_indicator_rect_matches_legacy_render_thickness() {
        let rect = DropIndicator::Inline {
            page_idx: 0,
            x: 10.0,
            y: 20.0,
            height: 30.0,
        }
        .rect();

        assert_eq!(rect.page_idx, 0);
        assert_eq!(rect.rect, Rect::from_xywh(10.0, 20.0, 2.0, 30.0));
    }

    #[test]
    fn block_indicator_rect_is_centered_on_target_y_with_legacy_thickness() {
        let rect = DropIndicator::Block {
            page_idx: 1,
            x: 10.0,
            y: 20.0,
            width: 200.0,
        }
        .rect();

        assert_eq!(rect.page_idx, 1);
        assert_eq!(rect.rect, Rect::from_xywh(10.0, 19.0, 200.0, 2.0));
    }
}

use crate::layout::cursor::{CursorNavigable, CursorNavigation, NavigationContext};
use crate::model::NodeId;
use crate::state::position_helpers::calculate_offset_before_child;
use crate::state::{Position, Selection};
use crate::types::{Affinity, Rect, Size};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExternalElementData {
    #[serde(rename_all = "camelCase")]
    Image {
        src: Option<String>,
        original_width: Option<f32>,
        original_height: Option<f32>,
        proportion: f32,
        upload_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    File {
        name: Option<String>,
        size: Option<u64>,
        src: Option<String>,
        upload_id: Option<String>,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ExternalElement {
    pub id: NodeId,
    pub parent_block_id: NodeId,
    pub size: Size,
    pub data: ExternalElementData,
}

impl ExternalElement {
    pub fn new(id: NodeId, parent_block_id: NodeId, size: Size, data: ExternalElementData) -> Self {
        Self {
            id,
            parent_block_id,
            size,
            data,
        }
    }

    fn offset_in_block(&self, ctx: &NavigationContext) -> usize {
        ctx.doc
            .node(self.parent_block_id)
            .map(|parent| calculate_offset_before_child(&parent, self.id))
            .unwrap_or(0)
    }

    pub fn node_selection(&self, offset_in_block: usize) -> Selection {
        let anchor = Position::new(self.parent_block_id, offset_in_block, Affinity::Downstream);
        let head = Position::new(
            self.parent_block_id,
            offset_in_block + 1,
            Affinity::Downstream,
        );
        Selection::new(anchor, head)
    }
}

impl CursorNavigable for ExternalElement {
    fn cursor_bounds(&self, ctx: &NavigationContext, position: &Position) -> Option<Rect> {
        if position.node_id != self.parent_block_id {
            return None;
        }

        let offset = self.offset_in_block(ctx);

        if position.offset == offset {
            return Some(Rect::new(0.0, 0.0, 0.0, self.size.height));
        }

        if position.offset == offset + 1 {
            return Some(Rect::new(self.size.width, 0.0, 0.0, self.size.height));
        }

        None
    }

    fn navigate_left(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x: 0.0,
            preferred_y,
        })
    }

    fn navigate_right(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x: self.size.width,
            preferred_y,
        })
    }

    fn navigate_word_left(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        self.navigate_left(ctx, position, preferred_y)
    }

    fn navigate_word_right(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        self.navigate_right(ctx, position, preferred_y)
    }

    fn navigate_up(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x,
            preferred_y: 0.0,
        })
    }

    fn navigate_down(
        &self,
        _ctx: &NavigationContext,
        _position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation> {
        Some(CursorNavigation::Exit {
            preferred_x,
            preferred_y: self.size.height,
        })
    }

    fn navigate_to_start(
        &self,
        ctx: &NavigationContext,
        _position: Position,
    ) -> Option<CursorNavigation> {
        let offset = self.offset_in_block(ctx);
        Some(CursorNavigation::Moved {
            selection: self.node_selection(offset),
        })
    }

    fn navigate_to_end(
        &self,
        ctx: &NavigationContext,
        _position: Position,
    ) -> Option<CursorNavigation> {
        let offset = self.offset_in_block(ctx);
        Some(CursorNavigation::Moved {
            selection: self.node_selection(offset),
        })
    }

    fn find_selection_at_point(
        &self,
        ctx: &NavigationContext,
        _x: f32,
        _y: f32,
    ) -> Option<Selection> {
        let offset = self.offset_in_block(ctx);
        Some(self.node_selection(offset))
    }
}

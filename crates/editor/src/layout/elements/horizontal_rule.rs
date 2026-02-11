use crate::layout::cursor::{CursorNavigable, CursorNavigation, NavigationContext};
use crate::model::HorizontalRuleVariant;
use crate::model::NodeId;
use crate::state::position_helpers::calculate_offset_before_child;
use crate::state::{Position, Selection};
use crate::types::{Affinity, Rect, Size};

#[derive(Debug, Clone, PartialEq)]
pub struct HorizontalRuleElement {
    pub node_id: NodeId,
    pub parent_id: NodeId,
    pub size: Size,
    pub variant: HorizontalRuleVariant,
}

impl HorizontalRuleElement {
    pub fn new(
        node_id: NodeId,
        parent_id: NodeId,
        size: Size,
        variant: HorizontalRuleVariant,
    ) -> Self {
        Self {
            node_id,
            parent_id,
            size,
            variant,
        }
    }

    fn offset_in_parent(&self, ctx: &NavigationContext) -> usize {
        ctx.doc
            .node(self.parent_id)
            .map(|parent| calculate_offset_before_child(&parent, self.node_id))
            .unwrap_or(0)
    }

    pub fn node_selection(&self, offset_in_parent: usize) -> Selection {
        let anchor = Position::new(self.parent_id, offset_in_parent, Affinity::Downstream);
        let head = Position::new(self.parent_id, offset_in_parent + 1, Affinity::Upstream);
        Selection::new(anchor, head)
    }
}

impl CursorNavigable for HorizontalRuleElement {
    fn cursor_bounds(&self, ctx: &NavigationContext, position: &Position) -> Option<Rect> {
        if position.node_id != self.parent_id {
            return None;
        }

        let offset = self.offset_in_parent(ctx);

        if position.offset == offset && position.affinity == Affinity::Downstream {
            return Some(Rect::new(0.0, 0.0, 0.0, self.size.height));
        }

        if position.offset == offset + 1 && position.affinity == Affinity::Upstream {
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

    fn navigate_sentence_up(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        self.navigate_left(ctx, position, preferred_y)
    }

    fn navigate_sentence_down(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation> {
        self.navigate_right(ctx, position, preferred_y)
    }

    fn navigate_to_start(
        &self,
        ctx: &NavigationContext,
        _position: Position,
    ) -> Option<CursorNavigation> {
        let offset = self.offset_in_parent(ctx);
        Some(CursorNavigation::Moved {
            selection: self.node_selection(offset),
        })
    }

    fn navigate_to_end(
        &self,
        ctx: &NavigationContext,
        _position: Position,
    ) -> Option<CursorNavigation> {
        let offset = self.offset_in_parent(ctx);
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
        let offset = self.offset_in_parent(ctx);
        Some(self.node_selection(offset))
    }
}

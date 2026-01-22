use crate::layout::Page;
use crate::model::{Doc, NodeId, NodeType};
use crate::state::{Position, Selection};
use crate::types::Rect;

pub mod hit_test;
pub mod navigation;

#[cfg(test)]
mod tests;

pub struct NavigationContext<'a> {
    pub doc: &'a Doc,
}

impl<'a> NavigationContext<'a> {
    pub fn new(doc: &'a Doc) -> Self {
        Self { doc }
    }
}

#[derive(Debug, Clone)]
pub enum Scope {
    Document,
    TableCell { node_id: NodeId },
}

impl Scope {
    pub fn scope_id(&self) -> NodeId {
        match self {
            Scope::Document => NodeId::ROOT,
            Scope::TableCell { node_id } => *node_id,
        }
    }
}

pub fn find_scope(ctx: &NavigationContext, node_id: NodeId) -> Scope {
    let doc = ctx.doc;
    let mut current = node_id;

    loop {
        let Some(parent_id) = doc.get_parent_id(current) else {
            return Scope::Document;
        };
        let Some(parent_type) = doc.get_node_type(parent_id) else {
            return Scope::Document;
        };

        if parent_type == NodeType::TableCell {
            let node_id = parent_id;
            return Scope::TableCell { node_id };
        }

        if parent_type == NodeType::Root {
            return Scope::Document;
        }

        current = parent_id;
    }
}

#[derive(Debug)]
pub enum CursorNavigation {
    Moved {
        selection: Selection,
    },
    Exit {
        #[allow(dead_code)]
        preferred_x: f32,
        preferred_y: f32,
    },
    SoftWrap {
        offset: usize,
    },
}

pub trait CursorNavigable {
    fn cursor_bounds(&self, ctx: &NavigationContext, position: &Position) -> Option<Rect>;

    fn navigate_left(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation>;
    fn navigate_right(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation>;
    fn navigate_word_left(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation>;
    fn navigate_word_right(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_y: f32,
    ) -> Option<CursorNavigation>;
    fn navigate_up(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation>;
    fn navigate_down(
        &self,
        ctx: &NavigationContext,
        position: Position,
        preferred_x: f32,
    ) -> Option<CursorNavigation>;

    fn navigate_to_start(
        &self,
        ctx: &NavigationContext,
        position: Position,
    ) -> Option<CursorNavigation>;
    fn navigate_to_end(
        &self,
        ctx: &NavigationContext,
        position: Position,
    ) -> Option<CursorNavigation>;

    fn find_selection_at_point(&self, ctx: &NavigationContext, x: f32, y: f32)
    -> Option<Selection>;
    fn find_drag_target(&self, ctx: &NavigationContext, x: f32, y: f32) -> Option<Selection> {
        self.find_selection_at_point(ctx, x, y)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum VerticalDirection {
    Up,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WordDirection {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub(crate) enum GapBehavior {
    SnapToClosestX,
    BlockPosition,
    ClosestNode,
}

pub struct Cursor;

impl Cursor {
    pub fn bounds(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<(usize, Rect)> {
        navigation::bounds(ctx, pages, position)
    }

    pub fn hit_test(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
        hit_test::hit_test(ctx, page, x, y)
    }

    pub fn hit_test_drag(
        ctx: &NavigationContext,
        page: &Page,
        x: f32,
        y: f32,
    ) -> Option<Selection> {
        hit_test::hit_test_drag(ctx, page, x, y)
    }

    pub fn hit_test_dnd(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
        hit_test::hit_test_dnd(ctx, page, x, y)
    }

    pub fn move_left(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_y: f32,
    ) -> Option<Selection> {
        navigation::move_left(ctx, pages, position, preferred_y)
    }

    pub fn move_right(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_y: f32,
    ) -> Option<Selection> {
        navigation::move_right(ctx, pages, position, preferred_y)
    }

    pub fn move_up(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_x: f32,
    ) -> Option<Selection> {
        navigation::move_up(ctx, pages, position, preferred_x)
    }

    pub fn move_down(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_x: f32,
    ) -> Option<Selection> {
        navigation::move_down(ctx, pages, position, preferred_x)
    }

    pub fn move_to_line_start(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<Selection> {
        navigation::move_to_line_start(ctx, pages, position)
    }

    pub fn move_to_line_end(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<Selection> {
        navigation::move_to_line_end(ctx, pages, position)
    }

    pub fn move_word_left(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_y: f32,
    ) -> Option<Selection> {
        navigation::move_word_left(ctx, pages, position, preferred_y)
    }

    pub fn move_word_right(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_y: f32,
    ) -> Option<Selection> {
        navigation::move_word_right(ctx, pages, position, preferred_y)
    }

    pub fn move_to_document_start(ctx: &NavigationContext, pages: &[Page]) -> Option<Selection> {
        navigation::move_to_document_start(ctx, pages)
    }

    pub fn move_to_document_end(ctx: &NavigationContext, pages: &[Page]) -> Option<Selection> {
        navigation::move_to_document_end(ctx, pages)
    }

    pub fn move_page_up(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_x: f32,
        viewport_height: f32,
    ) -> Option<Selection> {
        navigation::move_page_up(ctx, pages, position, preferred_x, viewport_height)
    }

    pub fn move_page_down(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
        preferred_x: f32,
        viewport_height: f32,
    ) -> Option<Selection> {
        navigation::move_page_down(ctx, pages, position, preferred_x, viewport_height)
    }
}

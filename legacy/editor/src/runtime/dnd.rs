use crate::layout::Page;
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::query::{find_node_bounds, find_node_bounds_on_page};
use crate::model::NodeRef;
use crate::state::Position;
use crate::state::position_helpers::is_inline_position;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DropIndicator {
    #[serde(rename_all = "camelCase")]
    Inline {
        page_idx: usize,
        x: f32,
        y: f32,
        height: f32,
    },

    #[serde(rename_all = "camelCase")]
    Block {
        page_idx: usize,
        x: f32,
        y: f32,
        width: f32,
    },
}

impl DropIndicator {
    pub fn from_position(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<Self> {
        if is_inline_position(ctx.doc, position) {
            Self::inline_indicator_from_position(ctx, pages, position)
        } else {
            Self::block_indicator_from_position(ctx, pages, position)
        }
    }

    fn inline_indicator_from_position(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<Self> {
        let (page_idx, rect) = Cursor::bounds(ctx, pages, position)?;

        Some(DropIndicator::Inline {
            page_idx,
            x: rect.x,
            y: rect.y,
            height: rect.height,
        })
    }

    fn block_indicator_from_position(
        ctx: &NavigationContext,
        pages: &[Page],
        position: Position,
    ) -> Option<Self> {
        let node = ctx.doc.node(position.node_id)?;
        let children: Vec<_> = node.children().collect();

        let (x, width) = Self::calc_indicator_horizontal(ctx, pages, &children)?;

        let (page_idx, y) = Self::calc_indicator_vertical(ctx, pages, &children, position.offset)?;

        Some(DropIndicator::Block {
            page_idx,
            x,
            y,
            width,
        })
    }

    fn calc_indicator_horizontal(
        ctx: &NavigationContext,
        pages: &[Page],
        children: &[NodeRef],
    ) -> Option<(f32, f32)> {
        children
            .first()
            .and_then(|child| find_node_bounds(ctx.doc, pages, child.node_id()))
            .map(|bounds| (bounds.x, bounds.width))
            .or_else(|| {
                let page = pages.first()?;
                Some((0.0, page.root.node.size.width))
            })
    }

    fn calc_indicator_vertical(
        ctx: &NavigationContext,
        pages: &[Page],
        children: &[NodeRef],
        offset: usize,
    ) -> Option<(usize, f32)> {
        if offset == 0 {
            let first_child = children.first()?;
            let bounds = find_node_bounds(ctx.doc, pages, first_child.node_id())?;
            Some((bounds.page_idx, bounds.y))
        } else if offset >= children.len() {
            let last_child = children.last()?;
            let bounds = find_node_bounds(ctx.doc, pages, last_child.node_id())?;
            Some((bounds.page_idx, bounds.bottom()))
        } else {
            let prev_child = children.get(offset - 1)?;
            let next_child = children.get(offset)?;

            let next_bounds = find_node_bounds(ctx.doc, pages, next_child.node_id())?;

            let prev_bounds = pages
                .get(next_bounds.page_idx)
                .and_then(|page| {
                    find_node_bounds_on_page(
                        ctx.doc,
                        page,
                        prev_child.node_id(),
                        next_bounds.page_idx,
                    )
                })
                .or_else(|| find_node_bounds(ctx.doc, pages, prev_child.node_id()))?;

            if prev_bounds.page_idx == next_bounds.page_idx {
                Some((
                    next_bounds.page_idx,
                    (prev_bounds.bottom() + next_bounds.y) / 2.0,
                ))
            } else {
                Some((next_bounds.page_idx, next_bounds.y))
            }
        }
    }
}

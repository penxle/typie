use super::{
    CursorNavigable, CursorNavigation, HorizontalDirection, NavigationContext, VerticalDirection,
    WordDirection, find_scope,
};
use crate::layout::Element;
use crate::layout::page::{ElementEntry, Page};
use crate::model::NodeId;
use crate::state::{Position, Selection};
use crate::types::{Point, Rect};
use crate::utils::resolve_affinity_boundary;
use rstar::AABB;

pub fn bounds(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
) -> Option<(usize, Rect)> {
    let (page_idx, pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;
    navigable
        .cursor_bounds(ctx, &position)
        .map(|relative_bounds| {
            let absolute_bounds = Rect::new(
                pos.x + relative_bounds.x,
                pos.y + relative_bounds.y,
                relative_bounds.width,
                relative_bounds.height,
            );
            (page_idx, absolute_bounds)
        })
}

fn navigate_horizontal<F>(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
    navigate_fn: F,
    direction: HorizontalDirection,
) -> Option<Selection>
where
    F: FnOnce(&dyn CursorNavigable, &NavigationContext, Position, f32) -> Option<CursorNavigation>,
{
    let (page_idx, pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;

    match navigate_fn(navigable, ctx, position, preferred_y)? {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::Exit { preferred_y, .. } => {
            let current_scope = find_scope(ctx, position.node_id);
            let scope_id = current_scope.scope_id();

            if let Some((entry, _)) = find_horizontal_target(
                pages,
                page_idx,
                pos,
                preferred_y,
                scope_id,
                element,
                direction,
            ) {
                if let Some(nav) = entry.element().as_cursor_navigable() {
                    let rel_x = match direction {
                        HorizontalDirection::Left => entry.size.width,
                        HorizontalDirection::Right => 0.0,
                    };
                    let rel_y = match direction {
                        HorizontalDirection::Left => entry.size.height - 1.0,
                        HorizontalDirection::Right => 1.0,
                    };
                    if let Some(selection) = nav.find_selection_at_point(ctx, rel_x, rel_y) {
                        return Some(selection);
                    }
                }
            }

            None
        }
        CursorNavigation::SoftWrap { offset } => {
            let next_pos = match direction {
                HorizontalDirection::Left => {
                    Position::new(position.node_id, offset, crate::types::Affinity::Upstream)
                }
                HorizontalDirection::Right => {
                    Position::new(position.node_id, offset, crate::types::Affinity::Downstream)
                }
            };
            match direction {
                HorizontalDirection::Left => move_left(ctx, pages, next_pos, preferred_y),
                HorizontalDirection::Right => move_right(ctx, pages, next_pos, preferred_y),
            }
        }
    }
}

pub fn move_left(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_horizontal(
        ctx,
        pages,
        position,
        preferred_y,
        |nav, ctx, pos, y| nav.navigate_left(ctx, pos, y),
        HorizontalDirection::Left,
    )
}

pub fn move_right(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_horizontal(
        ctx,
        pages,
        position,
        preferred_y,
        |nav, ctx, pos, y| nav.navigate_right(ctx, pos, y),
        HorizontalDirection::Right,
    )
}

fn navigate_vertical<F, G>(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_x: f32,
    navigate_fn: F,
    direction: VerticalDirection,
    fallback: G,
) -> Option<Selection>
where
    F: FnOnce(&dyn CursorNavigable, &NavigationContext, Position, f32) -> Option<CursorNavigation>,
    G: FnOnce(&NavigationContext, &[Page], Position) -> Option<Selection>,
{
    let (page_idx, pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;
    let current_scope = find_scope(ctx, position.node_id);
    let scope_id = current_scope.scope_id();

    let relative_x = preferred_x - pos.x;
    match navigate_fn(navigable, ctx, position, relative_x)? {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::Exit { preferred_y, .. } => {
            let absolute_y = pos.y + preferred_y;

            if let Some((entry, _)) = find_vertical_target(
                pages,
                page_idx,
                preferred_x,
                absolute_y,
                scope_id,
                element,
                direction,
            ) {
                return resolve_and_find_selection(
                    ctx,
                    entry,
                    Some(preferred_x),
                    match direction {
                        VerticalDirection::Up => Some(entry.size.height - 1.0),
                        VerticalDirection::Down => Some(0.0),
                    },
                );
            }

            fallback(ctx, pages, position)
        }
        CursorNavigation::SoftWrap { .. } => None,
    }
}

pub fn move_up(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_x: f32,
) -> Option<Selection> {
    navigate_vertical(
        ctx,
        pages,
        position,
        preferred_x,
        |nav, ctx, pos, x| nav.navigate_up(ctx, pos, x),
        VerticalDirection::Up,
        move_to_line_start,
    )
}

pub fn move_down(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_x: f32,
) -> Option<Selection> {
    navigate_vertical(
        ctx,
        pages,
        position,
        preferred_x,
        |nav, ctx, pos, x| nav.navigate_down(ctx, pos, x),
        VerticalDirection::Down,
        move_to_line_end,
    )
}

pub fn move_to_line_start(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
) -> Option<Selection> {
    let position = adjust_affinity_after_explicit_break(pages, position);
    let (_page_idx, _pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;

    match navigable.navigate_to_start(ctx, position)? {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::Exit { .. } => None,
        CursorNavigation::SoftWrap { .. } => None,
    }
}

pub fn move_to_line_end(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
) -> Option<Selection> {
    let position = adjust_affinity_after_explicit_break(pages, position);
    let (_page_idx, _pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;

    match navigable.navigate_to_end(ctx, position)? {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::Exit { .. } => None,
        CursorNavigation::SoftWrap { .. } => None,
    }
}

pub fn move_word_left(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_word(ctx, pages, position, preferred_y, WordDirection::Left)
}

pub fn move_word_right(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_word(ctx, pages, position, preferred_y, WordDirection::Right)
}

fn navigate_word(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
    direction: WordDirection,
) -> Option<Selection> {
    let (_page_idx, _pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;

    let nav_result = match direction {
        WordDirection::Left => navigable.navigate_word_left(ctx, position, preferred_y)?,
        WordDirection::Right => navigable.navigate_word_right(ctx, position, preferred_y)?,
    };

    match nav_result {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::SoftWrap { offset } => {
            let next_pos = match direction {
                WordDirection::Left => {
                    Position::new(position.node_id, offset, crate::types::Affinity::Upstream)
                }
                WordDirection::Right => {
                    Position::new(position.node_id, offset, crate::types::Affinity::Downstream)
                }
            };

            navigate_word(ctx, pages, next_pos, preferred_y, direction)
        }
        CursorNavigation::Exit { preferred_y, .. } => {
            let step = match direction {
                WordDirection::Left => move_left(ctx, pages, position, preferred_y),
                WordDirection::Right => move_right(ctx, pages, position, preferred_y),
            };

            if let Some(sel) = step {
                Some(sel)
            } else {
                Some(Selection::collapsed(position))
            }
        }
    }
}

pub fn move_sentence_up(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_sentence(ctx, pages, position, preferred_y, SentenceDirection::Up)
}

pub fn move_sentence_down(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
) -> Option<Selection> {
    navigate_sentence(ctx, pages, position, preferred_y, SentenceDirection::Down)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SentenceDirection {
    Up,
    Down,
}

fn navigate_sentence(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_y: f32,
    direction: SentenceDirection,
) -> Option<Selection> {
    let (_page_idx, _pos, element) = find_element_at_position(ctx, pages, &position)?;
    let navigable = element.as_cursor_navigable()?;

    let nav_result = match direction {
        SentenceDirection::Up => navigable.navigate_sentence_up(ctx, position, preferred_y)?,
        SentenceDirection::Down => navigable.navigate_sentence_down(ctx, position, preferred_y)?,
    };

    match nav_result {
        CursorNavigation::Moved { selection } => Some(selection),
        CursorNavigation::SoftWrap { .. } => None,
        CursorNavigation::Exit { .. } => {
            let step = match direction {
                SentenceDirection::Up => move_up(ctx, pages, position, 1_000_000.0),
                SentenceDirection::Down => move_down(ctx, pages, position, 0.0),
            };

            if let Some(sel) = step {
                Some(sel)
            } else {
                Some(Selection::collapsed(position))
            }
        }
    }
}

pub fn move_to_document_start(ctx: &NavigationContext, pages: &[Page]) -> Option<Selection> {
    for page in pages {
        if let Some((_, element)) = page.first_element() {
            let navigable = element.as_cursor_navigable()?;
            if let Some(selection) = navigable.find_selection_at_point(ctx, 0.0, 0.0) {
                return Some(selection);
            }
        }
    }
    None
}

pub fn move_to_document_end(ctx: &NavigationContext, pages: &[Page]) -> Option<Selection> {
    pages.iter().rev().find_map(|page| {
        find_last_navigable_element(page).and_then(|(_, element)| {
            let navigable = element.as_cursor_navigable()?;
            let size = element.size();
            navigable.find_selection_at_point(ctx, size.width, size.height)
        })
    })
}

fn find_selection_vertical(
    ctx: &NavigationContext,
    pages: &[Page],
    current_page_idx: usize,
    current_y: f32,
    preferred_x: f32,
    direction: VerticalDirection,
) -> Option<Selection> {
    if let Some(selection) = find_selection_in_page_vertical(
        ctx,
        &pages[current_page_idx],
        current_y,
        preferred_x,
        direction,
    ) {
        return Some(selection);
    }

    let (page_range, boundary_y): (Box<dyn Iterator<Item = usize>>, f32) = match direction {
        VerticalDirection::Up => (Box::new((0..current_page_idx).rev()), f32::INFINITY),
        VerticalDirection::Down => (
            Box::new((current_page_idx + 1)..pages.len()),
            f32::NEG_INFINITY,
        ),
    };

    for page_idx in page_range {
        if let Some(selection) = find_selection_in_page_vertical(
            ctx,
            &pages[page_idx],
            boundary_y,
            preferred_x,
            direction,
        ) {
            return Some(selection);
        }
    }

    None
}

fn find_selection_in_page_vertical(
    ctx: &NavigationContext,
    page: &Page,
    current_y: f32,
    preferred_x: f32,
    direction: VerticalDirection,
) -> Option<Selection> {
    let search_area = match direction {
        VerticalDirection::Up => {
            AABB::from_corners([f32::MIN, f32::MIN], [f32::MAX, current_y + 1.0])
        }
        VerticalDirection::Down => {
            AABB::from_corners([f32::MIN, current_y - 1.0], [f32::MAX, f32::MAX])
        }
    };

    let candidates = page.spatial_index().locate_in_envelope(&search_area);

    let mut best_selection = None;
    let mut closest_y = match direction {
        VerticalDirection::Up => f32::NEG_INFINITY,
        VerticalDirection::Down => f32::INFINITY,
    };

    for entry in candidates {
        let is_valid = match direction {
            VerticalDirection::Up => entry.pos.y < current_y - 0.1,
            VerticalDirection::Down => entry.pos.y + entry.size.height > current_y + 0.1,
        };

        if !is_valid {
            continue;
        }

        let target_y = match direction {
            VerticalDirection::Up => entry.pos.y + entry.size.height,
            VerticalDirection::Down => entry.pos.y,
        };

        let is_closer = match direction {
            VerticalDirection::Up => target_y > closest_y,
            VerticalDirection::Down => target_y < closest_y,
        };

        if is_closer {
            if let Some(navigable) = entry.element().as_cursor_navigable() {
                let relative_x = preferred_x - entry.pos.x;
                let relative_y = match direction {
                    VerticalDirection::Up => entry.size.height,
                    VerticalDirection::Down => 0.0,
                };

                if let Some(selection) =
                    navigable.find_selection_at_point(ctx, relative_x, relative_y)
                {
                    best_selection = Some(selection);
                    closest_y = target_y;
                }
            }
        }
    }

    best_selection
}

fn find_element_at_position<'a>(
    ctx: &NavigationContext,
    pages: &'a [Page],
    position: &Position,
) -> Option<(usize, Point, &'a Element)> {
    for (idx, page) in pages.iter().enumerate() {
        if let Some((pos, element)) = page.find_element_at_position(ctx, position) {
            return Some((idx, pos, element));
        }
    }

    None
}

fn adjust_affinity_after_explicit_break(pages: &[Page], position: Position) -> Position {
    if position.affinity == crate::types::Affinity::Downstream {
        return position;
    }

    if is_after_explicit_break(pages, position.node_id, position.offset) {
        let affinity = resolve_affinity_boundary(true, false, position.affinity);
        return Position::new(position.node_id, position.offset, affinity);
    }

    position
}

fn is_after_explicit_break(pages: &[Page], node_id: NodeId, offset: usize) -> bool {
    for page in pages {
        for entry in page.spatial_index().iter() {
            if let Element::Line(line) = entry.element() {
                if line.block_id == node_id
                    && line.metric.end_offset == offset
                    && line.metric.break_reason == parley::layout::BreakReason::Explicit
                {
                    return true;
                }
            }
        }
    }

    false
}

fn find_last_navigable_element<'a>(page: &'a Page) -> Option<(Point, &'a Element)> {
    let mut best: Option<(Point, &'a Element, f32)> = None;

    for entry in page.spatial_index().iter() {
        let Some(navigable) = entry.element().as_cursor_navigable() else {
            continue;
        };
        let bottom = entry.pos.y + entry.size.height;
        let is_better = best
            .as_ref()
            .map(|(_, _, best_bottom)| bottom > *best_bottom)
            .unwrap_or(true);
        if is_better {
            let element = entry.element();
            let _ = navigable;
            best = Some((entry.pos, element, bottom));
        }
    }

    best.map(|(pos, element, _)| (pos, element))
}

pub fn move_page_up(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_x: f32,
    viewport_height: f32,
) -> Option<Selection> {
    let (mut page_idx, rect) = bounds(ctx, pages, position.clone())?;
    let mut target_y = rect.y - viewport_height;

    while target_y < 0.0 && page_idx > 0 {
        page_idx -= 1;
        let page_height = pages[page_idx].root.node.size.height;
        target_y += page_height;
    }

    if target_y < 0.0 {
        return move_to_document_start(ctx, pages);
    }

    if let Some(selection) = find_selection_vertical(
        ctx,
        pages,
        page_idx,
        target_y,
        preferred_x,
        VerticalDirection::Down,
    ) {
        return Some(selection);
    }

    move_to_document_start(ctx, pages)
}

pub fn move_page_down(
    ctx: &NavigationContext,
    pages: &[Page],
    position: Position,
    preferred_x: f32,
    viewport_height: f32,
) -> Option<Selection> {
    let (mut page_idx, rect) = bounds(ctx, pages, position.clone())?;
    let mut target_y = rect.y + viewport_height;

    while page_idx < pages.len() {
        let page_height = pages[page_idx].root.node.size.height;
        if target_y < page_height {
            break;
        }
        target_y -= page_height;
        page_idx += 1;
        if page_idx >= pages.len() {
            return move_to_document_end(ctx, pages);
        }
    }

    if let Some(selection) = find_selection_vertical(
        ctx,
        pages,
        page_idx,
        target_y,
        preferred_x,
        VerticalDirection::Up,
    ) {
        return Some(selection);
    }

    move_to_document_end(ctx, pages)
}

fn find_vertical_target<'a>(
    pages: &'a [Page],
    current_page_idx: usize,
    preferred_x: f32,
    absolute_y: f32,
    scope_id: NodeId,
    current_element: &Element,
    direction: VerticalDirection,
) -> Option<(&'a ElementEntry, usize)> {
    let page = &pages[current_page_idx];

    let next_entry = if scope_id == NodeId::ROOT {
        match direction {
            VerticalDirection::Up => page.find_above(preferred_x, absolute_y, None),
            VerticalDirection::Down => page.find_below(preferred_x, absolute_y, None),
        }
    } else {
        let internal = match direction {
            VerticalDirection::Up => page.find_above_in_scope(
                preferred_x,
                absolute_y,
                scope_id,
                Some(current_element as *const _),
            ),
            VerticalDirection::Down => page.find_below_in_scope(
                preferred_x,
                absolute_y,
                scope_id,
                Some(current_element as *const _),
            ),
        };

        if internal.is_some() {
            internal
        } else {
            let mut boundary_y = absolute_y;
            if let Some(scope_entry) = page.scope_entry(scope_id) {
                boundary_y = match direction {
                    VerticalDirection::Up => scope_entry.pos.y,
                    VerticalDirection::Down => scope_entry.pos.y + scope_entry.size.height,
                };
            }
            match direction {
                VerticalDirection::Up => {
                    page.find_target_above(preferred_x, boundary_y, Some(scope_id))
                }
                VerticalDirection::Down => {
                    page.find_target_below(preferred_x, boundary_y, Some(scope_id))
                }
            }
        }
    };

    if let Some(entry) = next_entry {
        return Some((entry, current_page_idx));
    }

    let next_page_idx = match direction {
        VerticalDirection::Up => (current_page_idx > 0).then(|| current_page_idx - 1),
        VerticalDirection::Down => {
            (current_page_idx + 1 < pages.len()).then(|| current_page_idx + 1)
        }
    }?;

    let next_page = &pages[next_page_idx];
    let entry = match direction {
        VerticalDirection::Up => {
            next_page.find_target_above(preferred_x, next_page.root.node.size.height, None)
        }
        VerticalDirection::Down => next_page.find_target_below(preferred_x, 0.0, None),
    }?;

    Some((entry, next_page_idx))
}

fn find_horizontal_target<'a>(
    pages: &'a [Page],
    current_page_idx: usize,
    pos: Point,
    preferred_y: f32,
    scope_id: NodeId,
    current_element: &Element,
    direction: HorizontalDirection,
) -> Option<(&'a ElementEntry, usize)> {
    let page = &pages[current_page_idx];

    let vert_direction = match direction {
        HorizontalDirection::Left => VerticalDirection::Up,
        HorizontalDirection::Right => VerticalDirection::Down,
    };

    let search_x = match direction {
        HorizontalDirection::Left => 1_000_000.0,
        HorizontalDirection::Right => -1_000_000.0,
    };

    let next_entry = if scope_id == NodeId::ROOT {
        match vert_direction {
            VerticalDirection::Up => page.find_above(search_x, preferred_y, None),
            VerticalDirection::Down => page.find_below(search_x, preferred_y, None),
        }
    } else {
        match vert_direction {
            VerticalDirection::Up => page.find_above_in_scope(
                search_x,
                preferred_y,
                scope_id,
                Some(current_element as *const _),
            ),
            VerticalDirection::Down => page.find_below_in_scope(
                search_x,
                preferred_y,
                scope_id,
                Some(current_element as *const _),
            ),
        }
    };

    if next_entry.is_some() {
        return next_entry.map(|e| (e, current_page_idx));
    }

    if scope_id != NodeId::ROOT {
        let (boundary_x, boundary_y) = if let Some(scope_entry) = page.scope_entry(scope_id) {
            match direction {
                HorizontalDirection::Left => (scope_entry.pos.x, pos.y),
                HorizontalDirection::Right => (scope_entry.pos.x + scope_entry.size.width, pos.y),
            }
        } else {
            (pos.x, preferred_y)
        };

        let boundary_entry = match direction {
            HorizontalDirection::Left => {
                page.find_target_left(boundary_x, boundary_y, Some(scope_id))
            }
            HorizontalDirection::Right => {
                page.find_target_right(boundary_x, boundary_y, Some(scope_id))
            }
        };

        if boundary_entry.is_some() {
            return boundary_entry.map(|e| (e, current_page_idx));
        }

        let boundary_y = if let Some(scope_entry) = page.scope_entry(scope_id) {
            match vert_direction {
                VerticalDirection::Up => scope_entry.pos.y,
                VerticalDirection::Down => scope_entry.pos.y + scope_entry.size.height,
            }
        } else {
            preferred_y
        };

        let wrap_entry = match vert_direction {
            VerticalDirection::Up => page.find_target_above(search_x, boundary_y, Some(scope_id)),
            VerticalDirection::Down => page.find_target_below(search_x, boundary_y, Some(scope_id)),
        };

        if wrap_entry.is_some() {
            return wrap_entry.map(|e| (e, current_page_idx));
        }
    }

    let next_page_idx = match direction {
        HorizontalDirection::Left => (current_page_idx > 0).then(|| current_page_idx - 1),
        HorizontalDirection::Right => {
            (current_page_idx + 1 < pages.len()).then(|| current_page_idx + 1)
        }
    }?;

    let next_page = &pages[next_page_idx];
    let (search_x_page, search_y_page, vert_dir_page) = match direction {
        HorizontalDirection::Left => (
            1_000_000.0,
            next_page.root.node.size.height,
            VerticalDirection::Up,
        ),
        HorizontalDirection::Right => (-1_000_000.0, 0.0, VerticalDirection::Down),
    };

    let page_entry = match vert_dir_page {
        VerticalDirection::Up => next_page.find_target_above(search_x_page, search_y_page, None),
        VerticalDirection::Down => next_page.find_target_below(search_x_page, search_y_page, None),
    }?;

    Some((page_entry, next_page_idx))
}

fn resolve_and_find_selection(
    ctx: &NavigationContext,
    entry: &ElementEntry,
    target_absolute_x: Option<f32>,
    target_relative_y: Option<f32>,
) -> Option<Selection> {
    let nav = entry.element().as_cursor_navigable()?;

    let rel_x = if let Some(abs_x) = target_absolute_x {
        (abs_x - entry.pos.x).max(0.0)
    } else {
        0.0
    };

    let rel_y = target_relative_y.unwrap_or(0.0);

    nav.find_selection_at_point(ctx, rel_x, rel_y)
}

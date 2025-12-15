use super::{CursorNavigable, GapBehavior, NavigationContext};
use crate::layout::{Element, Page};
use crate::state::Selection;
use crate::types::Point;
use rstar::AABB;

pub fn hit_test(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
    hit_test_with_gap_behavior(
        ctx,
        page,
        x,
        y,
        GapBehavior::SnapToClosestX,
        |nav, ctx, x, y| nav.find_selection_at_point(ctx, x, y),
    )
}

pub fn hit_test_drag(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
    hit_test_with_gap_behavior(
        ctx,
        page,
        x,
        y,
        GapBehavior::ClosestNode,
        |nav, ctx, x, y| nav.find_drag_target(ctx, x, y),
    )
}

pub fn hit_test_dnd(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
    hit_test_with_gap_behavior(
        ctx,
        page,
        x,
        y,
        GapBehavior::BlockPosition,
        |nav, ctx, x, y| nav.find_selection_at_point(ctx, x, y),
    )
}

fn hit_test_with_gap_behavior<F>(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
    gap_behavior: GapBehavior,
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection> + Copy,
{
    let (closest_node, first_node, last_node) = find_closest_navigable_node(page, x, y)?;

    if matches!(gap_behavior, GapBehavior::BlockPosition) {
        if let Some(selection) = find_block_gap_position(ctx, page, x, y) {
            return Some(selection);
        }
    }

    if let Some(selection) = try_exact_hit(ctx, closest_node, x, y, selector) {
        return Some(selection);
    }

    if let Some(selection) = try_document_boundary_hit(ctx, y, first_node, last_node, selector) {
        return Some(selection);
    }

    if matches!(gap_behavior, GapBehavior::SnapToClosestX) {
        if let Some(selection) = find_closest_cursor_in_gap(ctx, page, x, y, closest_node) {
            return Some(selection);
        }
    }

    find_selection_in_closest_node(ctx, closest_node, x, y, selector)
}

fn find_block_gap_position(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
) -> Option<Selection> {
    if let Some((first_pos, first_elem)) = page.first_element() {
        if y < first_pos.y {
            let first_block = ctx.doc.node(first_elem.block_id()?)?;
            let root_child_id = find_root_child(ctx, &first_block)?;
            let root_child = ctx.doc.node(root_child_id)?;
            return Some(Selection::collapsed(crate::state::Position::new(
                crate::model::NodeId::ROOT,
                root_child.index()?,
                crate::types::Affinity::Downstream,
            )));
        }
    }

    if let Some((last_pos, last_elem)) = page.last_element() {
        if y > last_pos.y + last_elem.size().height {
            let last_block = ctx.doc.node(last_elem.block_id()?)?;
            let root_child_id = find_root_child(ctx, &last_block)?;
            let root_child = ctx.doc.node(root_child_id)?;
            return Some(Selection::collapsed(crate::state::Position::new(
                crate::model::NodeId::ROOT,
                root_child.index()? + 1,
                crate::types::Affinity::Downstream,
            )));
        }
    }

    if page
        .find_element_at_point(crate::types::Point::new(x, y))
        .is_some()
    {
        return None;
    }

    let element_above = find_element_above(page, x, y);
    let element_below = find_element_below(page, x, y);

    if let (Some(above), Some(below)) = (element_above, element_below) {
        let above_block = ctx.doc.node(above.element().block_id()?)?;
        let below_block = ctx.doc.node(below.element().block_id()?)?;

        if let Some(selection) = handle_container_boundary(ctx, &above_block, &below_block) {
            return Some(selection);
        }
        let below_parent = below_block.parent()?;
        let index = below_block.index()?;

        let position = crate::state::Position::new(
            below_parent.node_id(),
            index,
            crate::types::Affinity::Downstream,
        );
        return Some(Selection::collapsed(position));
    }

    None
}

fn handle_container_boundary(
    ctx: &NavigationContext,
    above_block: &crate::model::NodeRef,
    below_block: &crate::model::NodeRef,
) -> Option<Selection> {
    let above_parent = above_block.parent()?;
    let below_parent = below_block.parent()?;

    if above_parent.node_id() == below_parent.node_id() {
        return None;
    }

    let above_root_id = find_root_child(ctx, above_block)?;
    let below_root_id = find_root_child(ctx, below_block)?;

    if above_root_id == below_root_id {
        return None;
    }

    let below_root = ctx.doc.node(below_root_id)?;

    let position = crate::state::Position::new(
        crate::model::NodeId::ROOT,
        below_root.index()?,
        crate::types::Affinity::Downstream,
    );
    Some(Selection::collapsed(position))
}

fn find_root_child(
    ctx: &NavigationContext,
    node: &crate::model::NodeRef,
) -> Option<crate::model::NodeId> {
    let mut current_id = node.node_id();
    loop {
        let current = ctx.doc.node(current_id)?;
        if let Some(parent) = current.parent() {
            if parent.node_id() == crate::model::NodeId::ROOT {
                return Some(current_id);
            }
            current_id = parent.node_id();
        } else {
            return None;
        }
    }
}

fn find_element_above(page: &Page, x: f32, y: f32) -> Option<&crate::layout::page::ElementEntry> {
    let search_above = AABB::from_corners([f32::MIN, f32::MIN], [f32::MAX, y]);

    page.spatial_index()
        .locate_in_envelope(&search_above)
        .filter(|entry| entry.pos.y + entry.size.height <= y)
        .max_by(|a, b| {
            let a_bottom = a.pos.y + a.size.height;
            let b_bottom = b.pos.y + b.size.height;

            match a_bottom.partial_cmp(&b_bottom).unwrap() {
                std::cmp::Ordering::Equal => {
                    let a_contains_x = x >= a.pos.x && x < a.pos.x + a.size.width;
                    let b_contains_x = x >= b.pos.x && x < b.pos.x + b.size.width;

                    match (a_contains_x, b_contains_x) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => a.pos.x.partial_cmp(&b.pos.x).unwrap(),
                    }
                }
                ordering => ordering,
            }
        })
}

fn find_element_below(page: &Page, x: f32, y: f32) -> Option<&crate::layout::page::ElementEntry> {
    let search_below = AABB::from_corners([f32::MIN, y], [f32::MAX, f32::MAX]);

    page.spatial_index()
        .locate_in_envelope(&search_below)
        .filter(|entry| entry.pos.y >= y)
        .min_by(|a, b| match a.pos.y.partial_cmp(&b.pos.y).unwrap() {
            std::cmp::Ordering::Equal => {
                let a_contains_x = x >= a.pos.x && x < a.pos.x + a.size.width;
                let b_contains_x = x >= b.pos.x && x < b.pos.x + b.size.width;

                match (a_contains_x, b_contains_x) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.pos.x.partial_cmp(&b.pos.x).unwrap(),
                }
            }
            ordering => ordering,
        })
}

fn find_closest_cursor_in_gap(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
    closest_node: (Point, &Element),
) -> Option<Selection> {
    let (pos, element) = closest_node;
    if y >= pos.y && y < pos.y + element.size().height {
        // gap이 아님. 좌우 여백일 수 있음
        return None;
    }

    let element_above = find_element_above(page, x, y);
    let element_below = find_element_below(page, x, y);

    let y_dist_above = element_above
        .map(|e| y - (e.pos.y + e.size.height))
        .unwrap_or(f32::INFINITY);
    let y_dist_below = element_below.map(|e| e.pos.y - y).unwrap_or(f32::INFINITY);

    let (target_entry, relative_y) = if y_dist_above <= y_dist_below {
        (element_above?, element_above?.size.height)
    } else {
        (element_below?, 0.0)
    };

    let navigable = target_entry.element().as_cursor_navigable()?;
    let relative_x = x - target_entry.pos.x;
    navigable.find_selection_at_point(ctx, relative_x, relative_y)
}

fn find_closest_navigable_node<'a>(
    page: &'a Page,
    x: f32,
    y: f32,
) -> Option<(
    (Point, &'a Element),
    (Point, &'a Element),
    (Point, &'a Element),
)> {
    let closest_entry = page.spatial_index().nearest_neighbor(&[x, y])?;
    let closest = (closest_entry.pos, closest_entry.element());

    let first = page.first_element()?;
    let last = page.last_element()?;

    Some((closest, first, last))
}

fn try_exact_hit<F>(
    ctx: &NavigationContext,
    node: (Point, &Element),
    x: f32,
    y: f32,
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection>,
{
    let (pos, element) = node;
    let size = element.size();
    if x < pos.x || x > pos.x + size.width {
        return None;
    }
    if y < pos.y || y > pos.y + size.height {
        return None;
    }

    let navigable = element.as_cursor_navigable()?;
    let relative_x = x - pos.x;
    let relative_y = y - pos.y;
    selector(navigable, ctx, relative_x, relative_y)
}

fn try_document_boundary_hit<F>(
    ctx: &NavigationContext,
    y: f32,
    first: (Point, &Element),
    last: (Point, &Element),
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection>,
{
    let (first_pos, first_elem) = first;
    if y < first_pos.y {
        let navigable = first_elem.as_cursor_navigable()?;
        return selector(navigable, ctx, 0.0, 0.0);
    }

    let (last_pos, last_elem) = last;
    let last_size = last_elem.size();
    let last_bottom = last_pos.y + last_size.height;
    if y > last_bottom {
        let navigable = last_elem.as_cursor_navigable()?;
        return selector(navigable, ctx, last_size.width, last_size.height);
    }

    None
}

fn find_selection_in_closest_node<F>(
    ctx: &NavigationContext,
    node: (Point, &Element),
    x: f32,
    y: f32,
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection>,
{
    let (pos, element) = node;
    let size = element.size();
    let navigable = element.as_cursor_navigable()?;

    if y < pos.y {
        return selector(navigable, ctx, 0.0, 0.0);
    }

    if y >= pos.y + size.height {
        return selector(navigable, ctx, size.width, size.height);
    }

    let clamped_x = x.clamp(pos.x, pos.x + size.width);
    let relative_x = clamped_x - pos.x;
    let relative_y = y - pos.y;

    selector(navigable, ctx, relative_x, relative_y)
}

use crate::layout::cursor::{CursorNavigable, GapBehavior, NavigationContext};
use crate::layout::page::ElementEntry;
use crate::layout::{Element, Page};
use crate::model::{NodeId, NodeRef};
use crate::state::{Position, Selection};
use crate::types::{Affinity, Point};
use rstar::AABB;

pub fn hit_test(ctx: &NavigationContext, page: &Page, x: f32, y: f32) -> Option<Selection> {
    hit_test_with_gap_behavior(
        ctx,
        page,
        x,
        y,
        GapBehavior::SnapToClosestX,
        None,
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
        None,
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
        None,
        |nav, ctx, x, y| nav.find_selection_at_point(ctx, x, y),
    )
}

fn hit_test_with_gap_behavior<F>(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
    gap_behavior: GapBehavior,
    scope_node_id: Option<NodeId>,
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection> + Copy,
{
    match scope_node_id {
        None => {
            if let Some(found_scope_id) = find_container_scope(page, x, y) {
                return hit_test_with_gap_behavior(
                    ctx,
                    page,
                    x,
                    y,
                    gap_behavior,
                    Some(found_scope_id),
                    selector,
                );
            }

            hit_test_in_scope(ctx, page, x, y, gap_behavior, None, selector)
        }
        Some(scope_id) => {
            hit_test_in_scope(ctx, page, x, y, gap_behavior, Some(scope_id), selector)
        }
    }
}

fn hit_test_in_scope<F>(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
    gap_behavior: GapBehavior,
    scope_node_id: Option<NodeId>,
    selector: F,
) -> Option<Selection>
where
    F: Fn(&dyn CursorNavigable, &NavigationContext, f32, f32) -> Option<Selection> + Copy,
{
    let (closest_node, first_node, last_node) =
        find_closest_navigable_node_in_scope(page, x, y, ctx, scope_node_id)?;

    if matches!(gap_behavior, GapBehavior::BlockPosition) && scope_node_id.is_none() {
        if let Some(selection) = find_block_gap_position(ctx, page, x, y) {
            return Some(selection);
        }
    }

    if let Some(selection) = try_exact_hit(ctx, closest_node, x, y, selector) {
        return Some(selection);
    }

    if let Some(selection) = try_scope_boundary_hit(ctx, y, first_node, last_node, selector) {
        return Some(selection);
    }

    if matches!(gap_behavior, GapBehavior::SnapToClosestX) {
        if let Some(selection) =
            find_closest_cursor_in_gap_scoped(ctx, page, x, y, closest_node, scope_node_id)
        {
            return Some(selection);
        }
    }

    find_selection_in_closest_node(ctx, closest_node, x, y, selector)
}

fn find_container_scope(page: &Page, x: f32, y: f32) -> Option<NodeId> {
    if let Some(scope) = page.scope_at(x, y) {
        if scope.scope_id != NodeId::ROOT {
            return Some(scope.scope_id);
        }
    }

    page.nearest_scope_in_row(x, y)
        .and_then(|scope| (scope.scope_id != NodeId::ROOT).then_some(scope.scope_id))
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
            return Some(Selection::collapsed(Position::new(
                NodeId::ROOT,
                root_child.index()?,
                Affinity::Downstream,
            )));
        }
    }

    if let Some((last_pos, last_elem)) = page.last_element() {
        if y > last_pos.y + last_elem.size().height {
            let last_block = ctx.doc.node(last_elem.block_id()?)?;
            let root_child_id = find_root_child(ctx, &last_block)?;
            let root_child = ctx.doc.node(root_child_id)?;
            return Some(Selection::collapsed(Position::new(
                NodeId::ROOT,
                root_child.index()? + 1,
                Affinity::Downstream,
            )));
        }
    }

    if page.find_element_at_point(Point::new(x, y)).is_some() {
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

        let position = Position::new(below_parent.node_id(), index, Affinity::Downstream);
        return Some(Selection::collapsed(position));
    }

    None
}

fn handle_container_boundary(
    ctx: &NavigationContext,
    above_block: &NodeRef,
    below_block: &NodeRef,
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

    let position = Position::new(NodeId::ROOT, below_root.index()?, Affinity::Downstream);
    Some(Selection::collapsed(position))
}

fn find_root_child(ctx: &NavigationContext, node: &NodeRef) -> Option<NodeId> {
    let mut current_id = node.node_id();
    loop {
        let current = ctx.doc.node(current_id)?;
        if let Some(parent) = current.parent() {
            if parent.node_id() == NodeId::ROOT {
                return Some(current_id);
            }
            current_id = parent.node_id();
        } else {
            return None;
        }
    }
}

fn find_element_above(page: &Page, x: f32, y: f32) -> Option<&ElementEntry> {
    let search_above = AABB::from_corners([f32::MIN, f32::MIN], [f32::MAX, y]);

    page.spatial_index()
        .locate_in_envelope(&search_above)
        .filter(|entry| entry.pos.y + entry.size.height <= y)
        .max_by(|a, b| {
            let a_bottom = a.pos.y + a.size.height;
            let b_bottom = b.pos.y + b.size.height;

            match a_bottom.total_cmp(&b_bottom) {
                std::cmp::Ordering::Equal => {
                    let a_contains_x = x >= a.pos.x && x < a.pos.x + a.size.width;
                    let b_contains_x = x >= b.pos.x && x < b.pos.x + b.size.width;

                    match (a_contains_x, b_contains_x) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => a.pos.x.total_cmp(&b.pos.x),
                    }
                }
                ordering => ordering,
            }
        })
}

fn find_element_below(page: &Page, x: f32, y: f32) -> Option<&ElementEntry> {
    let search_below = AABB::from_corners([f32::MIN, y], [f32::MAX, f32::MAX]);

    page.spatial_index()
        .locate_in_envelope(&search_below)
        .filter(|entry| entry.pos.y >= y)
        .min_by(|a, b| match a.pos.y.total_cmp(&b.pos.y) {
            std::cmp::Ordering::Equal => {
                let a_contains_x = x >= a.pos.x && x < a.pos.x + a.size.width;
                let b_contains_x = x >= b.pos.x && x < b.pos.x + b.size.width;

                match (a_contains_x, b_contains_x) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.pos.x.total_cmp(&b.pos.x),
                }
            }
            ordering => ordering,
        })
}

fn find_closest_cursor_in_gap_scoped(
    ctx: &NavigationContext,
    page: &Page,
    x: f32,
    y: f32,
    closest_node: (Point, &Element),
    scope_node_id: Option<NodeId>,
) -> Option<Selection> {
    let (pos, element) = closest_node;
    if y >= pos.y && y < pos.y + element.size().height {
        // gap이 아님. 좌우 여백일 수 있음
        return None;
    }

    let mut element_above: Option<&ElementEntry> = None;
    let mut element_below: Option<&ElementEntry> = None;

    for entry in page.spatial_index().iter() {
        if entry.element().block_id().is_some() {
            let in_scope = match scope_node_id {
                None => true,
                Some(scope_id) => entry.scope_id == scope_id,
            };

            if in_scope {
                if entry.pos.y + entry.size.height <= y {
                    if element_above.is_none()
                        || entry.pos.y + entry.size.height
                            > element_above.unwrap().pos.y + element_above.unwrap().size.height
                    {
                        element_above = Some(entry);
                    }
                } else if entry.pos.y >= y {
                    if element_below.is_none() || entry.pos.y < element_below.unwrap().pos.y {
                        element_below = Some(entry);
                    }
                }
            }
        }
    }

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

fn find_closest_navigable_node_in_scope<'a>(
    page: &'a Page,
    x: f32,
    y: f32,
    _ctx: &NavigationContext,
    scope_node_id: Option<NodeId>,
) -> Option<(
    (Point, &'a Element),
    (Point, &'a Element),
    (Point, &'a Element),
)> {
    match scope_node_id {
        None => find_closest_navigable_node(page, x, y),
        Some(scope_id) => {
            let mut closest: Option<(&ElementEntry, f32)> = None;
            let mut first: Option<&ElementEntry> = None;
            let mut last: Option<&ElementEntry> = None;

            for entry in page.spatial_index().iter() {
                if entry.element().block_id().is_some() {
                    if entry.scope_id == scope_id {
                        if first.is_none() || entry.pos.y < first.unwrap().pos.y {
                            first = Some(entry);
                        }
                        if last.is_none() || entry.pos.y > last.unwrap().pos.y {
                            last = Some(entry);
                        }

                        let dx = if x < entry.pos.x {
                            entry.pos.x - x
                        } else if x > entry.pos.x + entry.size.width {
                            x - (entry.pos.x + entry.size.width)
                        } else {
                            0.0
                        };
                        let dy = if y < entry.pos.y {
                            entry.pos.y - y
                        } else if y > entry.pos.y + entry.size.height {
                            y - (entry.pos.y + entry.size.height)
                        } else {
                            0.0
                        };
                        let dist = dx * dx + dy * dy;

                        if closest.is_none() || dist < closest.unwrap().1 {
                            closest = Some((entry, dist));
                        }
                    }
                }
            }

            let closest_entry = closest?.0;
            let first_entry = first?;
            let last_entry = last?;

            Some((
                (closest_entry.pos, closest_entry.element()),
                (first_entry.pos, first_entry.element()),
                (last_entry.pos, last_entry.element()),
            ))
        }
    }
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

fn try_scope_boundary_hit<F>(
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

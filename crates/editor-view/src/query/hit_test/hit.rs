use editor_common::Rect;
use editor_model::{Doc, NodeId};
use editor_state::{Affinity, Position, Selection};

use crate::page::LayoutPage;
use crate::paginate::*;
use crate::style::Direction;

#[derive(Debug, Clone, Copy)]
pub(crate) enum HitTarget<'a> {
    TextLine {
        node: &'a LayoutNode,
        line: &'a LayoutLine,
    },
    Atom {
        node: &'a LayoutNode,
        atom: &'a LayoutAtom,
    },
}

/// Page-local hit-test adapter. It converts page-local coordinates to document
/// coordinates once and answers geometric hit questions; callers keep policy.
pub(crate) struct HitTester<'a> {
    tree: &'a LayoutTree,
    page: &'a LayoutPage,
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct StructuralBlock {
    node_id: NodeId,
    offset: usize,
    rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockGapSearch {
    Hit(Position),
    Blocked,
    Miss,
}

impl<'a> HitTester<'a> {
    pub(crate) fn for_page(
        tree: &'a LayoutTree,
        page: &'a LayoutPage,
        x: f32,
        page_y: f32,
    ) -> Self {
        Self {
            tree,
            page,
            x,
            y: page_y + page.y_start,
        }
    }

    /// Exact target classification used by click/pointer policies before each
    /// caller maps the target to a selection or cursor style.
    pub(crate) fn exact_target(&self) -> Option<HitTarget<'a>> {
        let (root, x) = self.target_root_and_x();
        exact_hit_target(root, x, self.y)
    }

    /// Exact hit: returns a selection only at a precise coordinate on a navigable node.
    pub(crate) fn exact_selection(&self) -> Option<Selection> {
        let (root, x) = self.target_root_and_x();
        exact_hit_target(root, x, self.y).map(|target| target.selection(x))
    }

    /// Closest hit: returns the nearest navigable by euclidean edge distance,
    /// restricted to navigables owned by the current page by `rect.y` range.
    pub(crate) fn closest_selection(&self) -> Option<Selection> {
        let (root, x) = self.target_root_and_x();
        closest_target(root, x, self.y, self.page.y_start, self.page.y_end)
            .map(|target| target.selection(x))
    }

    /// Extending variant: when the click fully escapes a monolithic ancestor's
    /// vertical range, promote the head to that container's slot boundary so the
    /// drag/shift-extend can select the container as a unit. Plain
    /// non-extending hit testing must use the exact/closest path instead.
    pub(crate) fn closest_extending_selection(&self) -> Option<Selection> {
        let (root, x) = self.target_root_and_x();
        let target = closest_target(root, x, self.y, self.page.y_start, self.page.y_end)?;
        if let Some(promoted) = promote_outside_y(&self.tree.root, target.node(), self.y) {
            return Some(Selection::collapsed(promoted));
        }
        Some(target.selection(x))
    }

    /// Exact hit first, then the extending closest fallback used by drag-style
    /// hit tests.
    pub(crate) fn hit_extending_selection(&self) -> Option<Selection> {
        self.exact_selection()
            .or_else(|| self.closest_extending_selection())
    }

    /// Returns a block insertion position only when the point falls in a
    /// vertical gap between structural block children of the active scope.
    pub(crate) fn block_gap_position(&self, doc: &Doc) -> Option<Position> {
        let (root, x) = self.target_root_and_x();
        match block_gap_in_node(root, doc, x, self.y) {
            BlockGapSearch::Hit(position) => Some(position),
            BlockGapSearch::Blocked | BlockGapSearch::Miss => None,
        }
    }

    /// Confine row-like scopes, such as table cells, before hit policy runs.
    /// Side gutters on the same row clamp horizontally into the nearest scope.
    fn target_root_and_x(&self) -> (&'a LayoutNode, f32) {
        let Some(scope) = scope_for_point_or_row(&self.tree.root, self.x, self.y) else {
            return (&self.tree.root, self.x);
        };
        let x = self.x.clamp(scope.rect.x, scope.rect.right());
        (scope, x)
    }
}

impl<'a> HitTarget<'a> {
    pub(crate) fn node(self) -> &'a LayoutNode {
        match self {
            HitTarget::TextLine { node, .. } | HitTarget::Atom { node, .. } => node,
        }
    }

    pub(crate) fn selection(self, x: f32) -> Selection {
        match self {
            HitTarget::TextLine { node, line } => navigate_to_line(line, &node.rect, x),
            HitTarget::Atom { atom, .. } => select_atom(atom),
        }
    }
}

/// Boxes containing the point, root to innermost. Consumers decide what each
/// box means for their own policy.
pub(crate) fn box_path_at<'a>(
    tree: &'a LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
) -> Vec<&'a LayoutNode> {
    let y = page_y + page.y_start;
    let mut path = Vec::new();
    collect_box_path_at(&tree.root, x, y, &mut path);
    path
}

/// Squared euclidean distance from point `(x, y)` to the nearest edge of `rect`.
/// Returns 0 if the point is inside the rect.
pub(crate) fn rect_distance_sq(rect: &Rect, x: f32, y: f32) -> f32 {
    let dx = if x < rect.x {
        rect.x - x
    } else if x > rect.x + rect.width {
        x - (rect.x + rect.width)
    } else {
        0.0
    };
    let dy = if y < rect.y {
        rect.y - y
    } else if y > rect.y + rect.height {
        y - (rect.y + rect.height)
    } else {
        0.0
    };
    dx * dx + dy * dy
}

fn exact_hit_target<'a>(node: &'a LayoutNode, x: f32, y: f32) -> Option<HitTarget<'a>> {
    match &node.content {
        LayoutContent::Box(b) => {
            if !node.rect.contains(x, y) {
                return None;
            }
            for child in &b.children {
                if let Some(target) = exact_hit_target(child, x, y) {
                    return Some(target);
                }
            }
            None
        }
        LayoutContent::Line(line) => (y >= node.rect.y && y < node.rect.y + node.rect.height)
            .then_some(HitTarget::TextLine { node, line }),
        LayoutContent::Atom(atom) => node
            .rect
            .contains(x, y)
            .then_some(HitTarget::Atom { node, atom }),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the closest navigable target by squared euclidean rect-edge distance.
/// Descends into the innermost containing box first, then falls back to all children.
/// Leaves are only considered if their `rect.y` lies within `[y_start, y_end)`.
fn closest_target<'a>(
    node: &'a LayoutNode,
    x: f32,
    y: f32,
    y_start: f32,
    y_end: f32,
) -> Option<HitTarget<'a>> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                if child.rect.contains(x, y)
                    && let Some(found) = closest_target(child, x, y, y_start, y_end)
                {
                    return Some(found);
                }
            }
            closest_target_in_range(node, x, y, y_start, y_end).map(|(_, target)| target)
        }
        LayoutContent::Line(line) => (node.rect.y >= y_start && node.rect.y < y_end)
            .then_some(HitTarget::TextLine { node, line }),
        LayoutContent::Atom(atom) => (node.rect.y >= y_start && node.rect.y < y_end)
            .then_some(HitTarget::Atom { node, atom }),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the navigable descendant of `node` whose `rect.y` lies within
/// `[y_start, y_end)` and is closest to `(x, y)` by squared rect-edge distance.
/// Returns `(dist_sq, target)`.
fn closest_target_in_range<'a>(
    node: &'a LayoutNode,
    x: f32,
    y: f32,
    y_start: f32,
    y_end: f32,
) -> Option<(f32, HitTarget<'a>)> {
    match &node.content {
        LayoutContent::Box(b) => b
            .children
            .iter()
            .filter_map(|child| closest_target_in_range(child, x, y, y_start, y_end))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)),
        LayoutContent::Line(line) => {
            if node.rect.y >= y_start && node.rect.y < y_end {
                Some((
                    rect_distance_sq(&node.rect, x, y),
                    HitTarget::TextLine { node, line },
                ))
            } else {
                None
            }
        }
        LayoutContent::Atom(atom) => {
            if node.rect.y >= y_start && node.rect.y < y_end {
                Some((
                    rect_distance_sq(&node.rect, x, y),
                    HitTarget::Atom { node, atom },
                ))
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

fn collect_box_path_at<'a>(
    node: &'a LayoutNode,
    x: f32,
    y: f32,
    path: &mut Vec<&'a LayoutNode>,
) -> bool {
    if !node.rect.contains(x, y) {
        return false;
    }
    let LayoutContent::Box(b) = &node.content else {
        return false;
    };
    path.push(node);
    for child in &b.children {
        if collect_box_path_at(child, x, y, path) {
            break;
        }
    }
    true
}

/// Search for a block gap inside the containing branch. Text and atom hits
/// block gap placement; scope boxes confine the search.
fn block_gap_in_node(node: &LayoutNode, doc: &Doc, x: f32, y: f32) -> BlockGapSearch {
    if !node.rect.contains(x, y) {
        return BlockGapSearch::Miss;
    }

    let LayoutContent::Box(b) = &node.content else {
        return BlockGapSearch::Blocked;
    };

    for child in &b.children {
        if !child.rect.contains(x, y) {
            continue;
        }

        match &child.content {
            LayoutContent::Spacing(_) => continue,
            LayoutContent::Box(child_box) => {
                let result = block_gap_in_node(child, doc, x, y);
                if matches!(result, BlockGapSearch::Hit(_)) {
                    return result;
                }
                if child_box.style.scope || matches!(result, BlockGapSearch::Blocked) {
                    return BlockGapSearch::Blocked;
                }
                return BlockGapSearch::Blocked;
            }
            LayoutContent::Line(_) | LayoutContent::Atom(_) => return BlockGapSearch::Blocked,
        }
    }

    block_gap_between_direct_children(b, doc, y).map_or(BlockGapSearch::Miss, BlockGapSearch::Hit)
}

/// Compute a gap position from direct structural block children only; layout
/// artifacts like lines and spacing are not insertion siblings.
fn block_gap_between_direct_children(b: &LayoutBox, doc: &Doc, y: f32) -> Option<Position> {
    if b.style.direction != Direction::Vertical {
        return None;
    }

    let children = structural_block_children(b, doc);
    if children.is_empty() {
        return None;
    }

    let first = children.first()?;
    if y < first.rect.y {
        return Some(Position::new(b.node_id, first.offset));
    }

    let last = children.last()?;
    if y > last.rect.bottom() {
        return Some(Position::new(b.node_id, last.offset + 1));
    }

    let above = children
        .iter()
        .filter(|child| child.rect.bottom() <= y)
        .max_by(|a, b| a.rect.bottom().total_cmp(&b.rect.bottom()))?;
    let below = children
        .iter()
        .filter(|child| child.rect.y >= y)
        .min_by(|a, b| a.rect.y.total_cmp(&b.rect.y))?;

    if above.node_id == below.node_id {
        return None;
    }

    Some(Position::new(b.node_id, below.offset))
}

/// Direct document children represented as layout block or atom items. This is
/// the insertion sibling set for block gap hit testing.
fn structural_block_children(b: &LayoutBox, doc: &Doc) -> Vec<StructuralBlock> {
    b.children
        .iter()
        .filter_map(|child| match &child.content {
            LayoutContent::Box(child_box) => {
                let child_ref = doc.node(child_box.node_id)?;
                (child_ref.parent()?.id() == b.node_id).then(|| StructuralBlock {
                    node_id: child_box.node_id,
                    offset: child_ref.index().unwrap_or(0),
                    rect: child.rect,
                })
            }
            LayoutContent::Atom(atom) => (atom.parent_id == b.node_id).then(|| StructuralBlock {
                node_id: atom.node_id,
                offset: atom.index,
                rect: child.rect,
            }),
            LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
        })
        .collect()
}

/// Exact scope if inside one, or nearest same-row scope for table side gutters.
fn scope_for_point_or_row<'a>(root: &'a LayoutNode, x: f32, y: f32) -> Option<&'a LayoutNode> {
    scope_at(root, x, y).or_else(|| nearest_scope_in_row(root, x, y))
}

fn scope_at<'a>(node: &'a LayoutNode, x: f32, y: f32) -> Option<&'a LayoutNode> {
    let LayoutContent::Box(b) = &node.content else {
        return None;
    };
    let mut best = if b.style.scope && node.rect.contains(x, y) {
        Some(node)
    } else {
        None
    };
    for child in &b.children {
        let Some(found) = scope_at(child, x, y) else {
            continue;
        };
        best = Some(match best {
            Some(prev) if rect_area(&prev.rect) <= rect_area(&found.rect) => prev,
            _ => found,
        });
    }
    best
}

fn nearest_scope_in_row<'a>(node: &'a LayoutNode, x: f32, y: f32) -> Option<&'a LayoutNode> {
    fn walk<'a>(
        node: &'a LayoutNode,
        x: f32,
        y: f32,
        best: &mut Option<(f32, f32, &'a LayoutNode)>,
    ) {
        let LayoutContent::Box(b) = &node.content else {
            return;
        };
        if b.style.scope && y >= node.rect.y && y <= node.rect.bottom() {
            let dx = if x < node.rect.x {
                node.rect.x - x
            } else if x > node.rect.right() {
                x - node.rect.right()
            } else {
                0.0
            };
            let area = rect_area(&node.rect);
            let replace = best.as_ref().is_none_or(|(best_dx, best_area, _)| {
                dx < *best_dx || (dx == *best_dx && area < *best_area)
            });
            if replace {
                *best = Some((dx, area, node));
            }
        }
        for child in &b.children {
            walk(child, x, y, best);
        }
    }

    let mut best = None;
    walk(node, x, y, &mut best);
    best.map(|(_, _, node)| node)
}

fn rect_area(rect: &Rect) -> f32 {
    rect.width * rect.height
}

/// When the click sits outside the vertical range of `leaf`'s monolithic
/// ancestor boxes, snap the head up the structural ancestry to the slot
/// boundary of the outermost monolithic box the click fully escaped. Above
/// the box -> its Front slot `(parent, idx)`; below -> its Back slot
/// `(parent, idx + 1)`. Without this, dragging the selection past a
/// monolithic container stalls at the container's innermost text position,
/// making it impossible to select the container as a unit.
///
/// The returned affinity points at the box the user is interacting with:
/// `above` returns Downstream (the slot points forward at the box being
/// approached); `below` returns Upstream (the slot points back at the box just
/// crossed). When surrounding normalize/unit logic resolves the slot to an
/// adjacent child via affinity, this consistently picks the box involved in
/// the gesture instead of the unrelated next sibling.
///
/// Only a true escape into the gutter promotes when escaping above the box.
/// The inter-block gap directly above the box belongs to the approach toward
/// the box, not an escape past it: `closest_target` can pick the box's own
/// leading text there, and that text caret, not a unit promotion, is the
/// intended drag-extend target. So above promotion requires the click to be
/// past the previous sibling too, or there to be no previous sibling at all.
/// The below direction needs no such guard: once the click is past the box's
/// bottom, `closest_target` resolves to the next sibling's text rather than
/// back into the box.
fn promote_outside_y(root: &LayoutNode, leaf: &LayoutNode, click_y: f32) -> Option<Position> {
    let mut path: Vec<(&LayoutNode, usize)> = Vec::new();
    if !build_path(root, leaf, &mut path) {
        return None;
    }
    for k in 1..path.len() {
        let ancestor = path[k].0;
        let LayoutContent::Box(ancestor_box) = &ancestor.content else {
            continue;
        };
        if !ancestor_box.style.monolithic {
            continue;
        }
        let above = click_y < ancestor.rect.y;
        let below = click_y >= ancestor.rect.y + ancestor.rect.height;
        if above || below {
            let (parent_box_node, idx) = path[k - 1];
            if above
                && idx > 0
                && let Some(prev) = nth_content_child(parent_box_node, idx - 1)
                && click_y >= prev.rect.y + prev.rect.height
            {
                continue;
            }
            if let LayoutContent::Box(parent_box) = &parent_box_node.content {
                let (slot, affinity) = if below {
                    (idx + 1, Affinity::Upstream)
                } else {
                    (idx, Affinity::Downstream)
                };
                return Some(Position {
                    node_id: parent_box.node_id,
                    offset: slot,
                    affinity,
                });
            }
        }
    }
    None
}

/// The `n`-th non-spacing child of `parent`, matching the content indexing
/// that `build_path` uses (`Spacing` children are skipped there too).
fn nth_content_child(parent: &LayoutNode, n: usize) -> Option<&LayoutNode> {
    let LayoutContent::Box(b) = &parent.content else {
        return None;
    };
    b.children
        .iter()
        .filter(|c| !matches!(c.content, LayoutContent::Spacing(_)))
        .nth(n)
}

fn build_path<'a>(
    node: &'a LayoutNode,
    target: &LayoutNode,
    path: &mut Vec<(&'a LayoutNode, usize)>,
) -> bool {
    if std::ptr::eq(node, target) {
        return true;
    }
    let LayoutContent::Box(b) = &node.content else {
        return false;
    };
    let mut content_idx = 0usize;
    for child in &b.children {
        if matches!(child.content, LayoutContent::Spacing(_)) {
            continue;
        }
        path.push((node, content_idx));
        if build_path(child, target, path) {
            return true;
        }
        path.pop();
        content_idx += 1;
    }
    false
}

fn navigate_to_line(line: &LayoutLine, rect: &Rect, x: f32) -> Selection {
    Selection::collapsed(position_in_line(line, rect, x))
}

fn position_in_line(line: &LayoutLine, rect: &Rect, x: f32) -> Position {
    let local_x = x - rect.x;
    super::super::grapheme::position_at_x(line, local_x)
}

fn select_atom(atom: &LayoutAtom) -> Selection {
    Selection::new(
        Position {
            node_id: atom.parent_id,
            offset: atom.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: atom.parent_id,
            offset: atom.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

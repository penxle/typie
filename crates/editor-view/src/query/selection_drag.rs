use editor_state::{Affinity, Position, Selection};

use crate::paginate::{LayoutContent, LayoutNode};

use super::hit_test::HitTarget;

pub(crate) fn selection_from_closest_target(
    root: &LayoutNode,
    target: HitTarget<'_>,
    x: f32,
    y: f32,
) -> Selection {
    if let Some(promoted) = promote_outside_monolithic_y(root, target.node(), y) {
        return Selection::collapsed(promoted);
    }
    target.selection(x)
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
/// the box, not an escape past it: closest hit can pick the box's own leading
/// text there, and that text caret, not a unit promotion, is the intended
/// drag-extend target. So above promotion requires the click to be past the
/// previous sibling too, or there to be no previous sibling at all. The below
/// direction needs no such guard: once the click is past the box's bottom,
/// closest hit resolves to the next sibling's text rather than back into the box.
fn promote_outside_monolithic_y(
    root: &LayoutNode,
    leaf: &LayoutNode,
    click_y: f32,
) -> Option<Position> {
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

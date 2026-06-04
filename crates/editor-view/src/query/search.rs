use editor_common::Rect;
use editor_model::NodeId;
use editor_state::{Affinity, Position};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

use super::selection::{SelectionRect, SelectionRectKind};

/// Find the LayoutNode (Line or Atom) containing a given Position.
///
/// At soft-wrap boundaries the same `(node_id, offset)` lies at the end of one
/// LayoutLine and at the start of the next. `pos.affinity` disambiguates:
/// `Upstream` → preceding (upper) line, `Downstream` → following (lower) line.
pub fn find_line_at<'a>(tree: &'a LayoutTree, pos: &Position) -> Option<&'a LayoutNode> {
    let mut candidates: Vec<&'a LayoutNode> = Vec::new();
    collect_lines(&tree.root, pos, &mut candidates);
    match (candidates.len(), pos.affinity) {
        (0, _) => None,
        (1, _) => Some(candidates[0]),
        (_, Affinity::Upstream) => Some(candidates[0]),
        (_, Affinity::Downstream) => candidates.last().copied(),
    }
}

fn collect_lines<'a>(node: &'a LayoutNode, pos: &Position, out: &mut Vec<&'a LayoutNode>) {
    match &node.content {
        LayoutContent::Box(b) => {
            // A monolithic box is terminal (atom-like) ONLY at its own bracket,
            // so a node-selection of the block resolves to the block itself.
            // For any interior position it stays transparent and recurses, so
            // editing/navigation inside the block is unaffected.
            if let Some(nv) = b.nav
                && position_attaches_to_child(pos, nv.parent_id, nv.index)
            {
                out.push(node);
                return;
            }
            for child in &b.children {
                collect_lines(child, pos, out);
            }
        }
        LayoutContent::Line(l) => {
            // Container-paragraph cursor: matches inclusive-both on child_range
            // when present. None means the line is a soft-wrap interior that
            // owns no paragraph boundary. This branch also subsumes the legacy
            // "empty line at offset 0" special case (empty paragraph yields
            // child_range = Some(0..0)).
            if let Some(range) = &l.child_range
                && l.node_id == pos.node_id
                && pos.offset >= range.start
                && pos.offset <= range.end
            {
                out.push(node);
                return;
            }
            // Text-node cursor: existing glyph_run code path.
            for run in &l.glyph_runs {
                if run.node_id == pos.node_id
                    && pos.offset >= run.offset
                    && pos.offset <= run.offset + super::grapheme::run_codepoint_count(run)
                {
                    out.push(node);
                    return;
                }
            }
        }
        LayoutContent::Atom(a) => {
            if position_attaches_to_child(pos, a.parent_id, a.index) {
                out.push(node);
            }
        }
        LayoutContent::Spacing(_) => {}
    }
}

fn position_attaches_to_child(pos: &Position, parent_id: NodeId, index: usize) -> bool {
    if pos.node_id != parent_id {
        return false;
    }
    match pos.affinity {
        Affinity::Downstream => pos.offset == index,
        Affinity::Upstream => index.checked_add(1) == Some(pos.offset),
    }
}

/// Find the first navigable (Line or Atom) node in a subtree.
pub fn find_first_navigable(node: &LayoutNode) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().find_map(find_first_navigable),
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the last navigable (Line or Atom) node in a subtree.
pub fn find_last_navigable(node: &LayoutNode) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().rev().find_map(find_last_navigable),
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
        LayoutContent::Spacing(_) => None,
    }
}

pub fn find_box_by_node_id(node: &LayoutNode, target: NodeId) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            if b.node_id == target {
                return Some(node);
            }
            b.children
                .iter()
                .find_map(|child| find_box_by_node_id(child, target))
        }
        _ => None,
    }
}

pub fn nearest_node_box(
    tree: &LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
    ids: &[NodeId],
) -> Option<NodeId> {
    let abs_y = page_y + page.y_start;
    let mut best: Option<(f32, NodeId)> = None;
    for &id in ids {
        let Some(node) = find_box_by_node_id(&tree.root, id) else {
            continue;
        };
        let d = super::hit_test::rect_distance_sq(&node.rect, x, abs_y);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, id));
        }
    }
    best.map(|(_, id)| id)
}

pub fn node_box_rects(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    ids: &[NodeId],
) -> Vec<SelectionRect> {
    let mut rects = Vec::new();
    for &id in ids {
        let Some(node) = find_box_by_node_id(&tree.root, id) else {
            continue;
        };
        let node_top = node.rect.y;
        let node_bottom = node_top + node.rect.height;
        for (page_idx, page) in pages.iter().enumerate() {
            if node_bottom <= page.y_start || node_top >= page.y_end {
                continue;
            }
            let top = node_top.max(page.y_start);
            let bottom = node_bottom.min(page.y_end);
            rects.push(PageRect::with_meta(
                page_idx,
                Rect::from_xywh(
                    node.rect.x,
                    top - page.y_start,
                    node.rect.width,
                    bottom - top,
                ),
                SelectionRectKind::Block,
            ));
        }
    }
    rects
}

/// Find the first navigable node whose bottom edge is below `y`.
pub fn find_navigable_below(node: &LayoutNode, y: f32) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                if let Some(nav) = find_navigable_below(child, y) {
                    return Some(nav);
                }
            }
            None
        }
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.y >= y {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the last navigable node whose top edge is above `y`.
pub fn find_navigable_above(node: &LayoutNode, y: f32) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in b.children.iter().rev() {
                if let Some(nav) = find_navigable_above(child, y) {
                    return Some(nav);
                }
            }
            None
        }
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.bottom() <= y {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the navigable (Line or Atom) node that immediately follows `target`
/// in document order (pre-order DFS), skipping Spacing.
///
/// Unlike [`find_navigable_below`], this follows logical document order rather
/// than vertical geometry, so it crosses into the next table cell in the same
/// row instead of the cell directly below.
pub fn find_navigable_after<'a>(
    root: &'a LayoutNode,
    target: &LayoutNode,
) -> Option<&'a LayoutNode> {
    let mut seen = false;
    find_navigable_after_inner(root, target, &mut seen)
}

fn find_navigable_after_inner<'a>(
    node: &'a LayoutNode,
    target: &LayoutNode,
    seen: &mut bool,
) -> Option<&'a LayoutNode> {
    if std::ptr::eq(node, target) {
        *seen = true;
        return None;
    }
    match &node.content {
        LayoutContent::Box(b) => b
            .children
            .iter()
            .find_map(|child| find_navigable_after_inner(child, target, seen)),
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => seen.then_some(node),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the navigable (Line or Atom) node that immediately precedes `target`
/// in document order (pre-order DFS), skipping Spacing.
///
/// The logical-order counterpart of [`find_navigable_above`].
pub fn find_navigable_before<'a>(
    root: &'a LayoutNode,
    target: &LayoutNode,
) -> Option<&'a LayoutNode> {
    let mut seen = false;
    find_navigable_before_inner(root, target, &mut seen)
}

fn find_navigable_before_inner<'a>(
    node: &'a LayoutNode,
    target: &LayoutNode,
    seen: &mut bool,
) -> Option<&'a LayoutNode> {
    if std::ptr::eq(node, target) {
        *seen = true;
        return None;
    }
    match &node.content {
        LayoutContent::Box(b) => b
            .children
            .iter()
            .rev()
            .find_map(|child| find_navigable_before_inner(child, target, seen)),
        LayoutContent::Line(l) if l.is_phantom => None,
        LayoutContent::Line(_) | LayoutContent::Atom(_) => seen.then_some(node),
        LayoutContent::Spacing(_) => None,
    }
}

fn collect_navigable<'a>(node: &'a LayoutNode, out: &mut Vec<&'a LayoutNode>) {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                collect_navigable(child, out);
            }
        }
        LayoutContent::Line(l) if l.is_phantom => {}
        LayoutContent::Line(_) | LayoutContent::Atom(_) => out.push(node),
        LayoutContent::Spacing(_) => {}
    }
}

fn contains_x(node: &LayoutNode, x: f32) -> bool {
    x >= node.rect.x && x <= node.rect.x + node.rect.width
}

fn horizontal_distance(node: &LayoutNode, x: f32) -> f32 {
    let clamped = x.clamp(node.rect.x, node.rect.x + node.rect.width);
    (x - clamped).abs()
}

/// Find the navigable node visually below `y_threshold` that best matches the
/// horizontal position `x`.
///
/// The nearest line below defines a band: only candidates that vertically
/// overlap it (the same visual row) compete on `x`, where a node whose extent
/// contains `x` is preferred and otherwise the horizontally nearest one wins so
/// the caret never gets stuck. Restricting the `x` contest to that band keeps
/// table-column movement intact while stopping a far full-width node from
/// leapfrogging a closer x-inset one — without the band an `x`-containing
/// paragraph after a fold would be chosen over the fold's padded title.
pub fn find_navigable_below_at_x(
    root: &LayoutNode,
    y_threshold: f32,
    x: f32,
) -> Option<&LayoutNode> {
    let mut nodes = Vec::new();
    collect_navigable(root, &mut nodes);
    let candidates: Vec<&LayoutNode> = nodes
        .into_iter()
        .filter(|n| n.rect.y >= y_threshold)
        .collect();
    let top = candidates.iter().copied().min_by(|p, q| {
        p.rect
            .y
            .total_cmp(&q.rect.y)
            .then(p.rect.x.total_cmp(&q.rect.x))
    })?;
    let band_end = top.rect.bottom();
    candidates
        .into_iter()
        .filter(|n| n.rect.y < band_end)
        .min_by(|p, q| {
            let key = |n: &LayoutNode| {
                let group = if contains_x(n, x) { 0u8 } else { 1u8 };
                (group, horizontal_distance(n, x), n.rect.y)
            };
            let (pg, pa, pb) = key(p);
            let (qg, qa, qb) = key(q);
            pg.cmp(&qg).then(pa.total_cmp(&qa)).then(pb.total_cmp(&qb))
        })
}

/// The upward counterpart of [`find_navigable_below_at_x`]: the nearest line
/// above defines the band, and only candidates that vertically overlap it
/// compete on `x`.
pub fn find_navigable_above_at_x(
    root: &LayoutNode,
    y_threshold: f32,
    x: f32,
) -> Option<&LayoutNode> {
    let mut nodes = Vec::new();
    collect_navigable(root, &mut nodes);
    let candidates: Vec<&LayoutNode> = nodes
        .into_iter()
        .filter(|n| n.rect.bottom() <= y_threshold)
        .collect();
    let bottom = candidates.iter().copied().min_by(|p, q| {
        q.rect
            .bottom()
            .total_cmp(&p.rect.bottom())
            .then(p.rect.x.total_cmp(&q.rect.x))
    })?;
    let band_start = bottom.rect.y;
    candidates
        .into_iter()
        .filter(|n| n.rect.bottom() > band_start)
        .min_by(|p, q| {
            let key = |n: &LayoutNode| {
                let group = if contains_x(n, x) { 0u8 } else { 1u8 };
                (group, horizontal_distance(n, x), -n.rect.bottom())
            };
            let (pg, pa, pb) = key(p);
            let (qg, qa, qb) = key(q);
            pg.cmp(&qg).then(pa.total_cmp(&qa)).then(pb.total_cmp(&qb))
        })
}

/// Find the innermost scope container (style.scope == true) that contains a given position.
pub fn find_scope_container_at<'a>(node: &'a LayoutNode, pos: &Position) -> Option<&'a LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            // Try children first so the innermost scope wins
            for child in &b.children {
                if let Some(scope) = find_scope_container_at(child, pos) {
                    return Some(scope);
                }
            }
            if b.style.scope && contains_position(node, pos) {
                Some(node)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn contains_position(node: &LayoutNode, pos: &Position) -> bool {
    let mut hits = Vec::new();
    collect_lines(node, pos, &mut hits);
    !hits.is_empty()
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use crate::style::Alignment;
    use editor_common::{EdgeInsets, Rect};
    use editor_model::NodeId;

    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::*;

    fn gs(n: usize) -> Vec<GraphemeSpan> {
        vec![
            GraphemeSpan {
                advance: 10.0,
                codepoints: 1
            };
            n
        ]
    }

    fn make_line_node(id: NodeId, y: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, "test", 0.0, gs(4))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn make_line_node_at(id: NodeId, x: f32, y: f32, w: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, "test", 0.0, gs(4))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn make_box_node(y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                nav: None,
            }),
        }
    }

    fn make_scope_box(y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: true,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                nav: None,
            }),
        }
    }

    #[test]
    fn find_first_navigable_skips_spacing() {
        let id = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                LayoutNode {
                    rect: Rect::from_xywh(0.0, 0.0, 0.0, 10.0),
                    content: LayoutContent::Spacing(SpacingKind::Gap),
                },
                make_line_node(id, 10.0),
            ],
        );
        let nav = find_first_navigable(&root).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_last_navigable_returns_bottom() {
        let id = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![make_line_node(NodeId::new(), 0.0), make_line_node(id, 20.0)],
        );
        let nav = find_last_navigable(&root).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_line_at_locates_position() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                40.0,
                vec![make_line_node(id, 0.0), make_line_node(NodeId::new(), 20.0)],
            ),
        };
        let pos = Position::new(id, 2);
        let node = find_line_at(&tree, &pos).unwrap();
        match &node.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_below_finds_next() {
        let id2 = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                make_line_node(NodeId::new(), 0.0),
                make_line_node(id2, 20.0),
            ],
        );
        let nav = find_navigable_below(&root, 10.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id2),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_above_finds_prev() {
        let id1 = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                make_line_node(id1, 0.0),
                make_line_node(NodeId::new(), 20.0),
            ],
        );
        let nav = find_navigable_above(&root, 20.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id1),
            _ => panic!("expected Line"),
        }
    }

    /// Two rows of two columns (table-like). The y-only search would return
    /// the document-order-first cell of the next row; the x-aware search must
    /// instead stay in the column whose horizontal extent contains `x`.
    fn two_by_two() -> (NodeId, NodeId, NodeId, NodeId, LayoutNode) {
        let (r0c0, r0c1, r1c0, r1c1) = (NodeId::new(), NodeId::new(), NodeId::new(), NodeId::new());
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                make_box_node(
                    0.0,
                    20.0,
                    vec![
                        make_line_node_at(r0c0, 0.0, 0.0, 100.0),
                        make_line_node_at(r0c1, 100.0, 0.0, 100.0),
                    ],
                ),
                make_box_node(
                    20.0,
                    20.0,
                    vec![
                        make_line_node_at(r1c0, 0.0, 20.0, 100.0),
                        make_line_node_at(r1c1, 100.0, 20.0, 100.0),
                    ],
                ),
            ],
        );
        (r0c0, r0c1, r1c0, r1c1, root)
    }

    #[test]
    fn find_navigable_below_at_x_stays_in_column() {
        let (_, _, _, r1c1, root) = two_by_two();
        // Below row 0, with x inside the second column (>= 100).
        let nav = find_navigable_below_at_x(&root, 20.0, 150.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, r1c1),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_above_at_x_stays_in_column() {
        let (_, r0c1, _, _, root) = two_by_two();
        // Above row 1, with x inside the second column (>= 100).
        let nav = find_navigable_above_at_x(&root, 20.0, 150.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, r0c1),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_below_at_x_falls_back_to_nearest_when_no_overlap() {
        let (_, _, r1c0, _, root) = two_by_two();
        // x past the right edge of every node: nearest column wins, not None.
        let nav = find_navigable_below_at_x(&root, 20.0, -50.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, r1c0),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_below_at_x_prefers_closer_inset_line_over_far_wide_line() {
        // Mimics ArrowDown into a fold: the fold title line is x-inset by the
        // fold's padding (so it does not contain the caret x) while the
        // full-width paragraph after the fold does. The closer inset line must
        // still win — the x contest is confined to the nearest band.
        let inset = NodeId::new();
        let far_wide = NodeId::new();
        let root = make_box_node(
            0.0,
            220.0,
            vec![
                make_line_node_at(inset, 40.0, 20.0, 60.0),
                make_line_node_at(far_wide, 0.0, 200.0, 200.0),
            ],
        );
        let nav = find_navigable_below_at_x(&root, 20.0, 0.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, inset),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_above_at_x_prefers_closer_inset_line_over_far_wide_line() {
        // ArrowUp counterpart: the fold content line is x-inset while the
        // paragraph above the fold is full-width; the closer inset line wins.
        let inset = NodeId::new();
        let far_wide = NodeId::new();
        let root = make_box_node(
            0.0,
            220.0,
            vec![
                make_line_node_at(far_wide, 0.0, 0.0, 200.0),
                make_line_node_at(inset, 40.0, 180.0, 60.0),
            ],
        );
        let nav = find_navigable_above_at_x(&root, 200.0, 0.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, inset),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_returns_none_at_boundary() {
        let root = make_box_node(0.0, 20.0, vec![make_line_node(NodeId::new(), 0.0)]);
        assert!(find_navigable_above(&root, 0.0).is_none());
    }

    #[test]
    fn find_line_at_matches_empty_line() {
        let para_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                20.0,
                vec![LayoutNode {
                    rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                    content: LayoutContent::Line(LayoutLine {
                        node_id: para_id,
                        baseline: 16.0,
                        ascent: 14.0,
                        descent: 4.0,
                        cursor_ascent: 14.0,
                        cursor_descent: 4.0,
                        glyph_runs: vec![], // empty line
                        ruby_annotations: vec![],
                        empty_caret_x: 0.0,
                        child_range: Some(0..0),
                        tab_gaps: vec![],
                        is_phantom: false,
                        content_edge_x: None,
                    }),
                }],
            ),
        };
        let pos = Position::new(para_id, 0);
        let node = find_line_at(&tree, &pos);
        assert!(node.is_some());
    }

    #[test]
    fn find_line_at_soft_wrap_boundary_downstream_picks_lower() {
        // Single text node `t` wrapped onto two visual lines:
        //   line A: glyph_run(t, offset=0, "abcde")
        //   line B: glyph_run(t, offset=5, "fghij")
        // At offset 5 both lines match; affinity disambiguates.
        let t = NodeId::new();
        let line_a = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 0, "abcde", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let line_b = LayoutNode {
            rect: Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 5, "fghij", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let tree = LayoutTree {
            root: make_box_node(0.0, 40.0, vec![line_a, line_b]),
        };

        let pos = editor_state::Position {
            node_id: t,
            offset: 5,
            affinity: editor_state::Affinity::Downstream,
        };
        let node = find_line_at(&tree, &pos).unwrap();
        // Downstream → lower line (rect.y == 20).
        assert_eq!(node.rect.y, 20.0);
    }

    #[test]
    fn find_line_at_soft_wrap_boundary_upstream_picks_upper() {
        let t = NodeId::new();
        let line_a = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 0, "abcde", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let line_b = LayoutNode {
            rect: Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 5, "fghij", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let tree = LayoutTree {
            root: make_box_node(0.0, 40.0, vec![line_a, line_b]),
        };

        let pos = editor_state::Position {
            node_id: t,
            offset: 5,
            affinity: editor_state::Affinity::Upstream,
        };
        let node = find_line_at(&tree, &pos).unwrap();
        // Upstream → upper line (rect.y == 0).
        assert_eq!(node.rect.y, 0.0);
    }

    #[test]
    fn find_scope_container_finds_innermost() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_scope_box(
                0.0,
                40.0,
                vec![make_scope_box(0.0, 20.0, vec![make_line_node(id, 0.0)])],
            ),
        };
        let pos = Position::new(id, 0);
        let scope = find_scope_container_at(&tree.root, &pos).unwrap();
        // Should be the inner scope (rect height 20), not the outer (40)
        assert_eq!(scope.rect.height, 20.0);
    }

    fn make_empty_line_node(para_id: NodeId, y: f32, child_range: Range<usize>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: para_id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: Some(child_range),
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    #[test]
    fn find_line_at_hard_break_boundary_upstream_picks_upper() {
        let p1 = NodeId::new();
        // Mimics paragraph { hard_break } → two empty lines at offsets 0..1 and 1..1.
        let line_a = make_empty_line_node(p1, 0.0, 0..1);
        let line_b = make_empty_line_node(p1, 20.0, 1..1);
        let tree = LayoutTree {
            root: make_box_node(0.0, 40.0, vec![line_a, line_b]),
        };
        let pos = editor_state::Position {
            node_id: p1,
            offset: 1,
            affinity: editor_state::Affinity::Upstream,
        };
        let node = find_line_at(&tree, &pos).unwrap();
        assert_eq!(node.rect.y, 0.0);
    }

    #[test]
    fn find_line_at_hard_break_boundary_downstream_picks_lower() {
        let p1 = NodeId::new();
        let line_a = make_empty_line_node(p1, 0.0, 0..1);
        let line_b = make_empty_line_node(p1, 20.0, 1..1);
        let tree = LayoutTree {
            root: make_box_node(0.0, 40.0, vec![line_a, line_b]),
        };
        let pos = editor_state::Position {
            node_id: p1,
            offset: 1,
            affinity: editor_state::Affinity::Downstream,
        };
        let node = find_line_at(&tree, &pos).unwrap();
        assert_eq!(node.rect.y, 20.0);
    }

    fn cell_box(id: NodeId, x: f32, y: f32, w: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: editor_common::Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: editor_common::EdgeInsets::ZERO,
                    border: editor_common::EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: false,
                    ..Default::default()
                },
                children: vec![],
                nav: None,
            }),
        }
    }

    fn wrap(id: NodeId, x: f32, y: f32, w: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: editor_common::Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: editor_common::EdgeInsets::ZERO,
                    border: editor_common::EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                nav: None,
            }),
        }
    }

    fn cell_page() -> LayoutPage {
        LayoutPage::new(0.0, 1000.0, editor_common::Size::new(200.0, 1000.0))
    }

    fn cell_tree() -> (LayoutTree, NodeId, NodeId, NodeId, NodeId) {
        let c00 = NodeId::new();
        let c01 = NodeId::new();
        let c10 = NodeId::new();
        let c11 = NodeId::new();
        let row0 = wrap(
            NodeId::new(),
            0.0,
            0.0,
            200.0,
            20.0,
            vec![
                cell_box(c00, 0.0, 0.0, 100.0, 20.0),
                cell_box(c01, 100.0, 0.0, 100.0, 20.0),
            ],
        );
        let row1 = wrap(
            NodeId::new(),
            0.0,
            20.0,
            200.0,
            20.0,
            vec![
                cell_box(c10, 0.0, 20.0, 100.0, 20.0),
                cell_box(c11, 100.0, 20.0, 100.0, 20.0),
            ],
        );
        let table = wrap(NodeId::new(), 0.0, 0.0, 200.0, 40.0, vec![row0, row1]);
        let root = wrap(NodeId::ROOT, 0.0, 0.0, 200.0, 40.0, vec![table]);
        (LayoutTree { root }, c00, c01, c10, c11)
    }

    #[test]
    fn nearest_node_box_inside_returns_that_cell() {
        let (tree, c00, c01, c10, c11) = cell_tree();
        let ids = [c00, c01, c10, c11];
        let page = cell_page();
        assert_eq!(
            super::nearest_node_box(&tree, &page, 150.0, 30.0, &ids),
            Some(c11)
        );
    }

    #[test]
    fn nearest_node_box_outside_clamps_to_edge_cell() {
        let (tree, c00, c01, c10, c11) = cell_tree();
        let ids = [c00, c01, c10, c11];
        let page = cell_page();
        assert_eq!(
            super::nearest_node_box(&tree, &page, 500.0, 500.0, &ids),
            Some(c11)
        );
        assert_eq!(
            super::nearest_node_box(&tree, &page, -50.0, 10.0, &ids),
            Some(c00)
        );
    }

    #[test]
    fn nearest_node_box_between_cells_picks_nearest_not_none() {
        let (tree, c00, c01, c10, c11) = cell_tree();
        let ids = [c00, c01, c10, c11];
        let page = cell_page();
        assert_eq!(
            super::nearest_node_box(&tree, &page, 100.0, 10.0, &ids),
            Some(c00)
        );
    }

    #[test]
    fn nearest_node_box_empty_ids_is_none() {
        let (tree, ..) = cell_tree();
        let page = cell_page();
        assert_eq!(super::nearest_node_box(&tree, &page, 10.0, 10.0, &[]), None);
    }

    #[test]
    fn node_box_rects_one_rect_per_cell_single_page() {
        let (tree, c00, _c01, _c10, c11) = cell_tree();
        let pages = vec![cell_page()];
        let rects = super::node_box_rects(&tree, &pages, &[c00, c11]);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].page_idx, 0);
        assert_eq!(
            rects[0].rect,
            editor_common::Rect::from_xywh(0.0, 0.0, 100.0, 20.0)
        );
        assert_eq!(
            rects[1].rect,
            editor_common::Rect::from_xywh(100.0, 20.0, 100.0, 20.0)
        );
    }

    #[test]
    fn node_box_rects_splits_a_cell_spanning_two_pages() {
        let (tree, c00, ..) = cell_tree();
        let pages = vec![
            LayoutPage::new(0.0, 10.0, editor_common::Size::new(200.0, 10.0)),
            LayoutPage::new(10.0, 1000.0, editor_common::Size::new(200.0, 990.0)),
        ];
        let rects = super::node_box_rects(&tree, &pages, &[c00]);
        assert_eq!(
            rects.len(),
            2,
            "cell crossing a page boundary must emit a rect per page"
        );
        assert_eq!(rects[0].page_idx, 0);
        assert_eq!(
            rects[0].rect,
            editor_common::Rect::from_xywh(0.0, 0.0, 100.0, 10.0)
        );
        assert_eq!(rects[1].page_idx, 1);
        assert_eq!(
            rects[1].rect,
            editor_common::Rect::from_xywh(0.0, 0.0, 100.0, 10.0)
        );
    }

    #[test]
    fn node_box_rects_skips_missing_ids() {
        let (tree, c00, ..) = cell_tree();
        let pages = vec![cell_page()];
        let missing = NodeId::new();
        let rects = super::node_box_rects(&tree, &pages, &[c00, missing]);
        assert_eq!(rects.len(), 1);
    }

    #[test]
    fn find_line_at_resolves_tab_boundary_positions() {
        use crate::view::View;
        use editor_macros::doc;

        // Paragraph: tab(child 0) text("x")(child 1). The tab boundary
        // positions (para, 0) [before] and (para, 1) [after] must both resolve
        // to the line owning the paragraph child_range.
        let (doc, p1) = doc! { root { p1: paragraph { tab {} text("x") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();

        for offset in [0usize, 1usize] {
            let pos = Position::new(p1, offset);
            let node = find_line_at(tree, &pos)
                .unwrap_or_else(|| panic!("tab boundary offset {offset} must resolve to a line"));
            match &node.content {
                LayoutContent::Line(l) => assert_eq!(l.node_id, p1),
                _ => panic!("expected Line for offset {offset}"),
            }
        }
    }
}

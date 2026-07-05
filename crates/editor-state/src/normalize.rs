use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, NodeView};

use crate::affinity::Affinity;

use crate::Position;
use crate::cell_selection::as_cell_rect;
use crate::classify;
use crate::gap_cursor::gap_cursor_at;
use crate::paragraph_break::selection_matches_trailing_break;
use crate::selection::{ResolvedSelection, Selection};
use crate::traversal::{first_cursor_position, last_cursor_position};

// ── top-level gate ────────────────────────────────────────────────────────────

pub(crate) fn normalize(sel: &Selection, view: &DocView) -> Option<Selection> {
    sel.resolve(view).map(|r| normalize_resolved(&r))
}

pub fn doc_start_selection<'a>(view: &'a DocView<'a>) -> Option<Selection> {
    let root = view.root()?;
    let pos = first_cursor_position(&root)?;
    normalize(&Selection::collapsed(pos), view)
}

// ── 7-stage skeleton ──────────────────────────────────────────────────────────

pub(crate) fn normalize_resolved(rs: &ResolvedSelection) -> Selection {
    let view = rs.view();
    let (a_in, h_in) = (rs.anchor().position(), rs.head().position());

    // Stage 0 — SEAM (b): promote_full_table_cell_rect
    if let Some(promoted) = promote_full_table_cell_rect(rs) {
        return promoted;
    }

    // Stage 1: boundary_identity (affinity-independent path)
    let a_bd = boundary_identity(&a_in, view).expect("resolved anchor");
    let h_bd = boundary_identity(&h_in, view).expect("resolved head");

    if a_bd == h_bd {
        // Stage 2 body — SEAM (b): collapsed_or_unit
        return collapsed_or_unit(&normalize_position(&h_in, view), view);
    }

    // Stage 3: affinity rewrite + text-interior preservation
    let (a_aff, h_aff) = affinity_for_order(&a_bd, &h_bd);
    let a_aff = preserve_text_interior_affinity(&a_in, view, a_aff);
    let h_aff = preserve_text_interior_affinity(&h_in, view, h_aff);

    // Stage 4: normalize_position (complete identity in the projection)
    let a = normalize_position(
        &Position {
            affinity: a_aff,
            ..a_in
        },
        view,
    );
    let h = normalize_position(
        &Position {
            affinity: h_aff,
            ..h_in
        },
        view,
    );

    let ap = a.resolve(view).expect("norm anchor").path().to_vec();
    let hp = h.resolve(view).expect("norm head").path().to_vec();

    // Stage 5 — SEAM (c): subtree_violation
    if subtree_violation(&ap, &hp) {
        return recover_subtree_violation(&a, &h, &a_in, &h_in, view);
    }

    // Stage 6 — SEAM (c): promote_cross_isolating
    if let Some(p) = promote_cross_isolating(&a_in, &h_in, view) {
        return normalize(&p, view).unwrap_or(p);
    }

    // Stage 7
    Selection::new(a, h)
}

// ── Stage 0: full-table promotion (d-2-4-1-b) ────────────────────────────────

fn promote_full_table_cell_rect(rs: &ResolvedSelection) -> Option<Selection> {
    let rect = as_cell_rect(rs)?;
    if !rect.is_full_table() {
        return None;
    }
    let parent = rect.table.parent()?;
    let index = rect.table.index()?;
    Some(Selection::new(
        Position {
            node: parent.id(),
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: parent.id(),
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    ))
}

// ── Stage 2: collapsed_or_unit (d-2-4-1-b) ───────────────────────────────────

fn collapsed_or_unit<'a>(pos: &Position, view: &'a DocView<'a>) -> Selection {
    if gap_cursor_at(pos, view).is_some() {
        return Selection::collapsed(*pos);
    }
    if let Some(sel) = expand_unit_at(pos, view) {
        return sel;
    }
    if let Some(cursor) = inline_cursor_near_block_boundary(pos, view) {
        return Selection::collapsed(cursor);
    }
    Selection::collapsed(*pos)
}

// ── helpers (d-2-4-1-b) ───────────────────────────────────────────────────────

fn expand_unit_at<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<Selection> {
    let host = view.node(pos.node)?;
    match pos.affinity {
        Affinity::Downstream => {
            let child = host.child_at(pos.offset)?;
            classify::child_is_unit(&child).then(|| {
                Selection::new(
                    Position {
                        node: pos.node,
                        offset: pos.offset,
                        affinity: Affinity::Downstream,
                    },
                    Position {
                        node: pos.node,
                        offset: pos.offset + 1,
                        affinity: Affinity::Upstream,
                    },
                )
            })
        }
        Affinity::Upstream => {
            let prev = pos.offset.checked_sub(1)?;
            let child = host.child_at(prev)?;
            classify::child_is_unit(&child).then(|| {
                Selection::new(
                    Position {
                        node: pos.node,
                        offset: pos.offset,
                        affinity: Affinity::Upstream,
                    },
                    Position {
                        node: pos.node,
                        offset: prev,
                        affinity: Affinity::Downstream,
                    },
                )
            })
        }
    }
}

pub(crate) fn unit_or_collapsed<'a>(pos: &Position, view: &'a DocView<'a>) -> Selection {
    expand_unit_at(pos, view).unwrap_or_else(|| Selection::collapsed(*pos))
}

fn cursor_in_child<'a>(
    child: &ChildView<'a>,
    at_end: bool,
    affinity: Affinity,
    view: &'a DocView<'a>,
) -> Option<Position> {
    let ChildView::Block(b) = child else {
        return None;
    };
    let base = if at_end {
        last_cursor_position(b)
    } else {
        first_cursor_position(b)
    }?;
    let cursor = Position { affinity, ..base };
    classify::is_inline_position(&cursor.resolve(view)?).then_some(cursor)
}

fn inline_cursor_near_block_boundary<'a>(
    pos: &Position,
    view: &'a DocView<'a>,
) -> Option<Position> {
    let host = view.node(pos.node)?;
    if host.spec().is_textblock() {
        return None;
    }
    let previous = || {
        cursor_in_child(
            &host.child_at(pos.offset.checked_sub(1)?)?,
            true,
            pos.affinity,
            view,
        )
    };
    let next = || cursor_in_child(&host.child_at(pos.offset)?, false, pos.affinity, view);
    match pos.affinity {
        Affinity::Upstream => previous().or_else(next),
        Affinity::Downstream => next(),
    }
}

fn subtree_violation(a_path: &[usize], h_path: &[usize]) -> bool {
    let a_node = &a_path[..a_path.len() - 1];
    let h_node = &h_path[..h_path.len() - 1];
    if a_node == h_node {
        return false;
    }
    let (anc_node, anc_full, desc_node) = if a_node.starts_with(h_node) {
        (h_node, h_path, a_node)
    } else if h_node.starts_with(a_node) {
        (a_node, a_path, h_node)
    } else {
        return false;
    };
    let anc_slot = anc_full[anc_full.len() - 1];
    let desc_child_idx = desc_node[anc_node.len()];
    anc_slot == desc_child_idx || anc_slot == desc_child_idx + 1
}

#[cfg(test)]
pub(crate) fn subtree_violation_pub(a_path: &[usize], h_path: &[usize]) -> bool {
    subtree_violation(a_path, h_path)
}

fn recover_subtree_violation(
    a: &Position,
    h: &Position,
    a_in: &Position,
    h_in: &Position,
    view: &DocView,
) -> Selection {
    if selection_matches_trailing_break(&Selection::new(*a, *h), view) {
        return Selection::new(*a, *h);
    }
    if let Some(sel) = enclosing_unit_at_subtree_overlap(a_in, h_in, view) {
        return sel;
    }
    unit_or_collapsed(&normalize_position(h_in, view), view)
}

fn enclosing_unit_at_subtree_overlap<'a>(
    a_in: &Position,
    h_in: &Position,
    view: &'a DocView<'a>,
) -> Option<Selection> {
    let a = a_in.resolve(view)?;
    let h = h_in.resolve(view)?;
    let ap = a.path();
    let hp = h.path();
    let a_node = &ap[..ap.len() - 1];
    let h_node = &hp[..hp.len() - 1];
    let (parent_id, child_idx) = if a_node.starts_with(h_node) && a_node != h_node {
        (h_in.node, a_node[h_node.len()])
    } else if h_node.starts_with(a_node) && h_node != a_node {
        (a_in.node, h_node[a_node.len()])
    } else {
        return None;
    };
    let child = view.node(parent_id)?.child_at(child_idx)?;
    if !classify::child_is_unit(&child) {
        return None;
    }
    expand_unit_at(
        &Position {
            node: parent_id,
            offset: child_idx + 1,
            affinity: Affinity::Upstream,
        },
        view,
    )
}

fn is_isolating_container(node: &NodeView) -> bool {
    let s = node.spec();
    s.isolating && s.monolithic
}

fn outermost_crossing_unit<'a>(
    inside: &Position,
    outside: &Position,
    view: &'a DocView<'a>,
) -> Option<Dot> {
    let inside_node = view.node(inside.node)?;
    let outside_set: HashSet<Dot> = view
        .node(outside.node)?
        .ancestors()
        .map(|n| n.id())
        .chain([outside.node])
        .collect();
    let mut result = None;
    for anc in inside_node.ancestors() {
        if outside_set.contains(&anc.id()) {
            break;
        }
        if is_isolating_container(&anc) {
            result = Some(anc.id());
        }
    }
    result
}

fn promote_cross_isolating<'a>(
    a: &Position,
    h: &Position,
    view: &'a DocView<'a>,
) -> Option<Selection> {
    if let Some(unit) = outermost_crossing_unit(a, h, view) {
        let u = view.node(unit)?;
        let (parent, idx) = (u.parent()?, u.index()?);
        let off = if h.resolve(view)? > a.resolve(view)? {
            idx
        } else {
            idx + 1
        };
        return Some(Selection::new(Position::new(parent.id(), off), *h));
    }
    if let Some(unit) = outermost_crossing_unit(h, a, view) {
        let u = view.node(unit)?;
        let (parent, idx) = (u.parent()?, u.index()?);
        let off = if h.resolve(view)? > a.resolve(view)? {
            idx + 1
        } else {
            idx
        };
        return Some(Selection::new(*a, Position::new(parent.id(), off)));
    }
    None
}

// ── §2.2 boundary_identity ────────────────────────────────────────────────────

pub(crate) fn boundary_identity(pos: &Position, view: &DocView) -> Option<Vec<usize>> {
    pos.resolve(view).map(|rp| rp.path().to_vec())
}

// ── §2.3 affinity rewrite ─────────────────────────────────────────────────────

pub(crate) fn affinity_for_order(a: &[usize], h: &[usize]) -> (Affinity, Affinity) {
    if a < h {
        (Affinity::Downstream, Affinity::Upstream)
    } else {
        (Affinity::Upstream, Affinity::Downstream)
    }
}

fn is_char_leaf(host: &editor_model::NodeView, i: usize) -> bool {
    matches!(host.child_at(i), Some(ChildView::Leaf(l)) if l.as_char().is_some())
}

pub(crate) fn preserve_text_interior_affinity(
    pos: &Position,
    view: &DocView,
    fallback: Affinity,
) -> Affinity {
    match view.node(pos.node) {
        Some(host)
            if host.spec().is_textblock()
                && pos.offset > 0
                && is_char_leaf(&host, pos.offset - 1)
                && is_char_leaf(&host, pos.offset) =>
        {
            pos.affinity
        }
        _ => fallback,
    }
}

// ── §2.4 normalize_position (complete identity) ───────────────────────────────

pub(crate) fn normalize_position(pos: &Position, _view: &DocView) -> Position {
    // Complete identity in the projection (§0.7, round-1 #3, verified):
    //  - text-node climb (old normalize.rs :481-502) has no analogue — no Node::Text.
    //  - descend_or_stay_at_textblock (:18-62) descends only into a Node::Text child;
    //    a textblock's children are char/atom leaves, so it always "stays" → identity.
    //  - a structural-container position was already returned unchanged (:510).
    *pos
}

// ── §2.5 small public helpers ─────────────────────────────────────────────────

pub fn is_unit_node_selection(sel: &Selection, view: &DocView) -> bool {
    if sel.anchor.node != sel.head.node {
        return false;
    }
    let (lo, hi) = (
        sel.anchor.offset.min(sel.head.offset),
        sel.anchor.offset.max(sel.head.offset),
    );
    if lo.checked_add(1) != Some(hi) {
        return false;
    }
    match view.node(sel.anchor.node).and_then(|n| n.child_at(lo)) {
        Some(child) => classify::child_is_unit(&child),
        None => false,
    }
}

pub fn farther_endpoint(
    view: &DocView,
    reference: &Position,
    e1: &Position,
    e2: &Position,
) -> Position {
    if e1 == e2 {
        return *e2;
    }
    let Some(r_ref) = reference.resolve(view) else {
        return *e2;
    };
    let Some(r1) = e1.resolve(view) else {
        return *e2;
    };
    let Some(r2) = e2.resolve(view) else {
        return *e1;
    };
    use std::cmp::Ordering;
    match (r1.cmp(&r_ref), r2.cmp(&r_ref)) {
        (Ordering::Less | Ordering::Equal, Ordering::Less | Ordering::Equal) => {
            if r1 <= r2 {
                *e1
            } else {
                *e2
            }
        }
        (Ordering::Greater | Ordering::Equal, Ordering::Greater | Ordering::Equal) => {
            if r1 >= r2 {
                *e1
            } else {
                *e2
            }
        }
        _ => *e2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeType,
        ProjectedDoc, SeqItem, SpanLog, project_document,
    };

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_markers: NodeMarkerLog::new(),
        }
    }

    // ── doc fixtures ──────────────────────────────────────────────────────────

    fn two_paras() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 5);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
        ];
        (project_document(&logs(&items)).unwrap(), p1, p2)
    }

    // para with: 'a' HardBreak 'b'
    fn para_with_atom() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(1, 4), SeqItem::Char('b')),
        ];
        (project_document(&logs(&items)).unwrap(), para)
    }

    // root → blockquote (unit) as the only child
    fn doc_with_block_atom() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let items = vec![(
            hr,
            SeqItem::BlockAtom {
                leaf: AtomLeaf::HorizontalRule {
                    variant: editor_model::HorizontalRuleVariant::default(),
                },
                parents: vec![root],
            },
        )];
        (project_document(&logs(&items)).unwrap(), root, hr)
    }

    fn pos(node: Dot, offset: usize) -> Position {
        Position::new(node, offset)
    }

    fn pos_aff(node: Dot, offset: usize, aff: Affinity) -> Position {
        Position {
            node,
            offset,
            affinity: aff,
        }
    }

    // ── §4.1 validation gate ──────────────────────────────────────────────────

    #[test]
    fn test_1_dead_endpoint_returns_none() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let dead = Position::new(Dot::new(9, 9), 0);
        let good = pos(p1, 0);
        assert!(normalize(&Selection::new(dead, good), &view).is_none());
        assert!(normalize(&Selection::new(good, dead), &view).is_none());
    }

    #[test]
    fn test_1_out_of_range_endpoint_returns_none() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let oor = pos(p1, 99);
        let good = pos(p1, 0);
        assert!(normalize(&Selection::new(oor, good), &view).is_none());
        assert!(normalize(&Selection::new(good, oor), &view).is_none());
    }

    // ── §4.2 boundary_identity = path ────────────────────────────────────────

    #[test]
    fn test_2_same_boundary_differing_affinity() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let bd_up = boundary_identity(&pos_aff(p1, 1, Affinity::Upstream), &view).unwrap();
        let bd_down = boundary_identity(&pos_aff(p1, 1, Affinity::Downstream), &view).unwrap();
        assert_eq!(bd_up, bd_down);
    }

    #[test]
    fn test_2_different_boundaries_ordered() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let bd1 = boundary_identity(&pos(p1, 0), &view).unwrap();
        let bd2 = boundary_identity(&pos(p2, 0), &view).unwrap();
        assert_ne!(bd1, bd2);
        assert!(bd1 < bd2);
    }

    // ── §4.3 collapsed branch ────────────────────────────────────────────────

    #[test]
    fn test_3_same_boundary_takes_collapsed_branch() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        // Same node, same offset, different affinity → same boundary → collapsed branch
        let a = pos_aff(p1, 1, Affinity::Upstream);
        let h = pos_aff(p1, 1, Affinity::Downstream);
        let sel = Selection::new(a, h);
        let result = normalize(&sel, &view).unwrap();
        assert!(result.is_collapsed());
    }

    // ── §4.4 affinity rewrite ────────────────────────────────────────────────

    #[test]
    fn test_4_forward_range_affinity() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        // p1, offset 0 < p2, offset 0 in doc order
        let a = pos_aff(p1, 0, Affinity::Upstream);
        let h = pos_aff(p2, 0, Affinity::Upstream);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        assert_eq!(
            result.anchor.affinity,
            Affinity::Downstream,
            "anchor in forward range"
        );
        assert_eq!(
            result.head.affinity,
            Affinity::Upstream,
            "head in forward range"
        );
    }

    #[test]
    fn test_4_reversed_range_affinity() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        // anchor = p2 (later), head = p1 (earlier) → reversed
        let a = pos_aff(p2, 0, Affinity::Downstream);
        let h = pos_aff(p1, 0, Affinity::Downstream);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        assert_eq!(
            result.anchor.affinity,
            Affinity::Upstream,
            "anchor in reversed range"
        );
        assert_eq!(
            result.head.affinity,
            Affinity::Downstream,
            "head in reversed range"
        );
    }

    // ── §4.5 text-interior affinity ──────────────────────────────────────────

    #[test]
    fn test_5_between_two_chars_preserved() {
        // Para: 'a' HardBreak 'b'   → offsets 0,1,2,3
        // offset 0 = before 'a', offset 1 = between 'a' and HardBreak,
        // offset 2 = between HardBreak and 'b', offset 3 = after 'b'
        // For the interior to be preserved, BOTH neighbors must be chars.
        // In this doc: children[0]='a', children[1]=HardBreak, children[2]='b'
        // There's no position where BOTH neighbors are chars (only 3 children, atom is in the middle).
        //
        // So we need a doc with two consecutive chars to test preservation.
        // Use two_paras: p1 has children 'H'(0), 'i'(1)  → offset 1 is between two chars.
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        // Place anchor at p1/offset=1 (between 'H' and 'i') with Upstream
        // head at p2/0 (different boundary)
        // forward range: a_bd < h_bd → anchor gets Downstream, but preserve_text_interior
        // will keep Upstream since offset 1 is strictly between two chars in p1
        let a = pos_aff(p1, 1, Affinity::Upstream);
        let h = pos_aff(p2, 0, Affinity::Upstream);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // Anchor is at text interior (between 'H' and 'i') → preserved as Upstream
        assert_eq!(
            result.anchor.affinity,
            Affinity::Upstream,
            "text interior affinity preserved"
        );
        // Head is at p2/0 where offset=0 → offset > 0 is false, so fallback
        assert_eq!(
            result.head.affinity,
            Affinity::Upstream,
            "head at non-interior gets computed"
        );
    }

    #[test]
    fn test_5_adjacent_to_inline_atom_gets_fallback() {
        // Para: 'a'(0) HardBreak(1) 'b'(2)
        // offset 1 = between 'a' and HardBreak → child_at(0)='a' (char), child_at(1)=HardBreak (atom)
        // Not both chars → fallback
        let (pd, para) = para_with_atom();
        let (_pd2, _p2) = {
            let root = Dot::ROOT;
            let p = Dot::new(2, 1);
            let items = vec![
                (
                    p,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                    },
                ),
                (Dot::new(2, 2), SeqItem::Char('z')),
            ];
            (project_document(&logs(&items)).unwrap(), p)
        };
        // We need two positions in the SAME doc for normalize. Use para_with_atom's root + para.
        // Place anchor at para/offset=1 (between 'a' and HardBreak), head at para/offset=3 (after 'b').
        // Both are in the same textblock but different boundary → range.
        let view = DocView::new(&pd);
        let a = pos_aff(para, 1, Affinity::Upstream);
        let h = pos_aff(para, 3, Affinity::Upstream);
        // a_bd < h_bd (offset 1 < offset 3 in same block) → a gets Downstream, h gets Upstream
        // but preserve_text_interior: at para/1, child_at(0)='a' (char), child_at(1)=HardBreak (atom)
        // → NOT both chars → fallback (Downstream) for anchor
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        assert_eq!(
            result.anchor.affinity,
            Affinity::Downstream,
            "atom-adjacent gets computed/fallback"
        );
    }

    #[test]
    fn test_5_run_edge_offset_zero_gets_fallback() {
        // offset=0: pos.offset > 0 is false → always fallback
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let a = pos_aff(p1, 0, Affinity::Upstream);
        let h = pos_aff(p2, 0, Affinity::Upstream);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // p1/0 has offset=0 → not interior → fallback=Downstream
        assert_eq!(result.anchor.affinity, Affinity::Downstream);
    }

    // ── §4.6 normalize_position complete identity ─────────────────────────────

    #[test]
    fn test_6_textblock_char_position_unchanged() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let p = pos(p1, 1);
        let result = normalize_position(&p, &view);
        assert_eq!(result, p);
    }

    #[test]
    fn test_6_textblock_adjacent_to_atom_unchanged() {
        let (pd, para) = para_with_atom();
        let view = DocView::new(&pd);
        // offset 1 = between 'a' and HardBreak
        let p = pos(para, 1);
        let result = normalize_position(&p, &view);
        assert_eq!(result, p);
    }

    #[test]
    fn test_6_structural_between_blocks_unchanged() {
        let (pd, _p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        // root/offset=1 is a structural between-blocks position
        let p = Position::new(root_id, 1);
        let result = normalize_position(&p, &view);
        assert_eq!(result, p);
    }

    // ── §4.7 is_unit_node_selection ──────────────────────────────────────────

    #[test]
    fn test_7_block_atom_is_unit() {
        let (pd, root, _hr) = doc_with_block_atom();
        let view = DocView::new(&pd);
        // root/0..root/1 brackets the HorizontalRule (a block-level atom leaf)
        let sel = Selection::new(pos(root, 0), pos(root, 1));
        assert!(is_unit_node_selection(&sel, &view));
    }

    #[test]
    fn test_7_monolithic_block_is_unit() {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let bq_para = Dot::new(1, 2);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                bq_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        // root/0..root/1 brackets the Blockquote (monolithic)
        let sel = Selection::new(pos(root, 0), pos(root, 1));
        assert!(is_unit_node_selection(&sel, &view));
    }

    #[test]
    fn test_7_char_leaf_is_not_unit() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        // p1/0..p1/1 brackets 'H' (a char leaf) → not a unit
        let sel = Selection::new(pos(p1, 0), pos(p1, 1));
        assert!(!is_unit_node_selection(&sel, &view));
    }

    #[test]
    fn test_7_non_adjacent_offsets_false() {
        let (_pd, _root, _hr) = doc_with_block_atom();
        // root/0..root/2 spans two children (non-adjacent in single-child doc it won't resolve, but use two_paras)
        let (pd2, p1, _p2) = two_paras();
        let view2 = DocView::new(&pd2);
        let sel = Selection::new(pos(p1, 0), pos(p1, 2));
        assert!(!is_unit_node_selection(&sel, &view2));
    }

    #[test]
    fn test_7_cross_container_false() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let sel = Selection::new(pos(p1, 0), pos(p2, 0));
        assert!(!is_unit_node_selection(&sel, &view));
    }

    // ── §4.8 farther_endpoint ────────────────────────────────────────────────

    #[test]
    fn test_8_farther_endpoint_from_anchor() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        // reference = p1/0, e1 = p1/1 (close), e2 = p2/0 (farther)
        let reference = pos(p1, 0);
        let e1 = pos(p1, 1);
        let e2 = pos(p2, 0);
        let result = farther_endpoint(&view, &reference, &e1, &e2);
        // e2 is farther from reference in doc order
        assert_eq!(result.node, p2);
    }

    #[test]
    fn test_8_equal_endpoints_returns_e2() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let reference = pos(p1, 0);
        let e1 = pos(p1, 1);
        let e2 = pos(p1, 1);
        let result = farther_endpoint(&view, &reference, &e1, &e2);
        // Equal distance → returns e2
        assert_eq!(result, e2);
    }

    // ── doc fixtures for Task 8 tests ─────────────────────────────────────────

    // root > [image_atom, para('x')]  — image at root index 0
    fn image_then_para() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let img_dot = Dot::new(2, 1);
        let para = Dot::new(2, 2);
        let img_node = match editor_model::NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items = vec![
            (
                img_dot,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(2, 3), SeqItem::Char('x')),
        ];
        (project_document(&logs(&items)).unwrap(), root, img_dot)
    }

    // 2×2 table for promote_full_table tests
    fn two_by_two_table_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let table = Dot::new(3, 1);
        let row0 = Dot::new(3, 2);
        let row1 = Dot::new(3, 5);
        let mut counter = 8u64;
        let mut next = || {
            let d = Dot::new(3, counter);
            counter += 1;
            d
        };
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                row0,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                next(), // cell00 = Dot(3,8)
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                },
            ),
            (
                next(), // para in cell00 = Dot(3,9)
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, Dot::new(3, 8)],
                },
            ),
            (
                row1,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                next(), // cell10 = Dot(3,10)
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row1],
                },
            ),
            (
                next(), // para in cell10 = Dot(3,11)
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row1, Dot::new(3, 10)],
                },
            ),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            table,
            row0,
            row1,
        )
    }

    // ── §4 Task 8 tests ───────────────────────────────────────────────────────

    // §4.1 gap cursor preserved: collapsed caret before a leading image stays collapsed
    #[test]
    fn test_b1_gap_cursor_preserved_collapsed() {
        let (pd, root, _img_dot) = image_then_para();
        let view = DocView::new(&pd);
        // (root, 0, Upstream) is a gap cursor (leading unit)
        let pos = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            result.is_collapsed(),
            "gap cursor must stay collapsed, not expand to unit"
        );
    }

    // §4.2 unit expansion downstream: caret just before image (Downstream) → node-selection
    #[test]
    fn test_b2_unit_expansion_downstream() {
        let (pd, root, _img_dot) = image_then_para();
        let view = DocView::new(&pd);
        // (root, 0, Downstream) — image is at index 0; Downstream looks at child[0]
        let pos = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            !result.is_collapsed(),
            "should expand to unit node-selection"
        );
        assert_eq!(result.anchor.node, root);
        assert_eq!(result.anchor.offset, 0);
        assert_eq!(result.anchor.affinity, Affinity::Downstream);
        assert_eq!(result.head.node, root);
        assert_eq!(result.head.offset, 1);
        assert_eq!(result.head.affinity, Affinity::Upstream);
    }

    // §4.3 unit expansion upstream: caret just after image (Upstream) → node-selection
    #[test]
    fn test_b3_unit_expansion_upstream() {
        let (pd, root, _img_dot) = image_then_para();
        let view = DocView::new(&pd);
        // (root, 1, Upstream) — image is at index 0; Upstream checks child[offset-1=0]
        let pos = Position {
            node: root,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            !result.is_collapsed(),
            "should expand upstream to unit node-selection"
        );
        // anchor = (root,1,Upstream), head = (root,0,Downstream)
        assert_eq!(result.anchor.node, root);
        assert_eq!(result.anchor.offset, 1);
        assert_eq!(result.anchor.affinity, Affinity::Upstream);
        assert_eq!(result.head.node, root);
        assert_eq!(result.head.offset, 0);
        assert_eq!(result.head.affinity, Affinity::Downstream);
    }

    // §4.4 no expansion in text: caret inside paragraph chars → plain collapsed
    #[test]
    fn test_b4_no_expansion_in_text() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        // p1 has 'H'(0) 'i'(1); caret at offset 1 (between chars)
        let pos = pos_aff(p1, 1, Affinity::Downstream);
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            result.is_collapsed(),
            "caret amid chars must stay collapsed"
        );
        assert_eq!(result.anchor.node, pos.node);
    }

    // §4.5 inline snap: caret at (root, idx) between two paragraphs
    // Downstream → snaps to next paragraph's first text cursor
    // Upstream → snaps to previous paragraph's last text cursor
    #[test]
    fn test_b5_inline_snap_downstream() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        // root has p1 at index 0, p2 at index 1
        // (root, 1, Downstream) → should snap into p2's first cursor
        let pos = Position {
            node: root_id,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            result.is_collapsed(),
            "inline snap produces collapsed result"
        );
        // Should land inside p2 (next paragraph)
        assert_eq!(result.anchor.node, p2, "snapped into p2");
        let _ = p1;
    }

    #[test]
    fn test_b5_inline_snap_upstream() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        // (root, 1, Upstream) → should snap into p1's last cursor
        let pos = Position {
            node: root_id,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let result = normalize(&Selection::collapsed(pos), &view).unwrap();
        assert!(
            result.is_collapsed(),
            "inline snap produces collapsed result"
        );
        // Should land inside p1 (previous paragraph)
        assert_eq!(result.anchor.node, p1, "snapped into p1");
        let _ = p2;
    }

    // §4.5 leaf-atom neighbor: no snap on the atom side
    #[test]
    fn test_b5_no_snap_adjacent_to_block_atom() {
        let (pd, root, _img_dot) = image_then_para();
        let _view = DocView::new(&pd);
        // (root, 1, Downstream) — image (block atom=unit) at index 0, para at index 1
        // Downstream at offset 1: next child is para → can snap into para
        // But offset 0, Downstream over the image is a unit expansion (handled by b2)
        // Here test that (root, 1, Upstream) where prev child is block-atom → no cursor_in_child
        // Because cursor_in_child returns None for ChildView::Leaf
        // So inline_cursor_near_block_boundary tries previous (image=Leaf→None) then next (para=Block→cursor)
        // For Upstream: previous().or_else(next) → None.or_else(→ para's last cursor) = para's last cursor
        // (para has 1 char 'x', last cursor = offset 1)
        let pos = Position {
            node: root,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        // But wait: (root, 1, Upstream) first hits expand_unit_at which finds image at index 0 (a unit)
        // → expands! That test is b3. So for b5 we need a position where no unit neighbor exists.
        // Use two_paras where root has [p1, p2] — both paragraphs (not units).
        // Already tested in test_b5_inline_snap_upstream above.
        // This test verifies cursor_in_child returns None for Leaf (image side) in image_then_para.
        // (root, 1, Upstream) → expand_unit_at: child[0]=image (unit) → expands (covered by b3).
        // We want to test the Leaf→None path in cursor_in_child directly.
        // Use two_paras, test at root/0 Downstream (no unit neighbor, p1 IS next child).
        let (pd2, _p1_2, _p2_2) = two_paras();
        let view2 = DocView::new(&pd2);
        let root2 = view2.root().unwrap().id();
        // (root, 0, Downstream): p1 is at index 0, not a unit, and is a Block → cursor_in_child works
        let pos2 = Position {
            node: root2,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let result2 = normalize(&Selection::collapsed(pos2), &view2).unwrap();
        // Should snap into p1 (the next block = paragraph)
        assert!(result2.is_collapsed());
        let _ = (pd, pos, root);
    }

    // §4.6 full-table promotion: full 2×2 cell-rect → table node-selection
    #[test]
    fn test_b6_full_table_promotion() {
        let (pd, root, table, row0, row1) = two_by_two_table_doc();
        let view = DocView::new(&pd);
        // Select full 2×2 table: anchor=(row0,0,Downstream), head=(row1,1,Downstream)
        // row0 has 1 cell (offset 0..1), row1 has 1 cell (offset 0..1)
        let a = Position {
            node: row0,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: row1,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // Should promote to table node-selection: (root, table_idx, Down)..(root, table_idx+1, Up)
        let table_node = view.node(table).unwrap();
        let table_idx = table_node.index().unwrap();
        assert_eq!(result.anchor.node, root);
        assert_eq!(result.anchor.offset, table_idx);
        assert_eq!(result.anchor.affinity, Affinity::Downstream);
        assert_eq!(result.head.node, root);
        assert_eq!(result.head.offset, table_idx + 1);
        assert_eq!(result.head.affinity, Affinity::Upstream);
    }

    // §4.6 partial cell-rect NOT promoted
    #[test]
    fn test_b6_partial_cell_rect_not_promoted() {
        let (pd, _root, _table, row0, _row1) = two_by_two_table_doc();
        let view = DocView::new(&pd);
        // Single cell selection: NOT full table → Stage 0 returns None, passes through
        let a = Position {
            node: row0,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: row0,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // Should NOT be promoted to table node-selection; endpoints should remain in row0
        assert_eq!(result.anchor.node, row0);
        assert_eq!(result.head.node, row0);
    }

    // §4.7 unit_or_collapsed: at a unit boundary → expands; elsewhere → plain collapse
    #[test]
    fn test_b7_unit_or_collapsed_expands() {
        let (pd, root, _img_dot) = image_then_para();
        let view = DocView::new(&pd);
        // (root, 0, Downstream) adjacent to image (a unit) → expand
        let pos = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let sel = unit_or_collapsed(&pos, &view);
        assert!(
            !sel.is_collapsed(),
            "unit_or_collapsed should expand at unit boundary"
        );
    }

    #[test]
    fn test_b7_unit_or_collapsed_stays_collapsed() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        // Inside paragraph chars → no unit neighbor → collapsed
        let pos = pos_aff(p1, 1, Affinity::Downstream);
        let sel = unit_or_collapsed(&pos, &view);
        assert!(
            sel.is_collapsed(),
            "unit_or_collapsed stays collapsed among chars"
        );
    }

    #[test]
    fn test_b7_unit_or_collapsed_no_gap_branch() {
        // unit_or_collapsed must never surface a gap cursor as expanded (it has no gap branch)
        // A gap cursor position → unit_or_collapsed collapses it (expand_unit_at finds a unit
        // only if child_is_unit, but at a gap position the neighbors are monolithic blocks which
        // ARE units, so expand_unit_at WILL expand there; that's correct for unit_or_collapsed
        // since it has no gap preservation branch — only collapsed_or_unit preserves gaps).
        // So this test verifies that collapsed_or_unit at a gap stays collapsed,
        // while unit_or_collapsed at a gap expands (correct per spec §2.2 and H4).
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        // Inside text: both must be collapsed
        let pos = pos_aff(p1, 0, Affinity::Downstream);
        let c_or_u = collapsed_or_unit(&pos, &view);
        let u_or_c = unit_or_collapsed(&pos, &view);
        // p1 has 'H' at index 0 which is a char, not a unit → both collapse
        assert!(c_or_u.is_collapsed());
        assert!(u_or_c.is_collapsed());
    }

    // ── §4.9 proptest ─────────────────────────────────────────────────────────

    proptest::proptest! {
        #[test]
        fn test_proptest_normalize_never_panics(
            a_off in 0usize..=3,
            h_off in 0usize..=3,
        ) {
            let (pd, p1, p2) = two_paras();
            let view = DocView::new(&pd);
            // p1 has 2 children ('H','i'), p2 has 1 child ('y')
            let a = Position::new(p1, a_off.min(2));
            let h = Position::new(p2, h_off.min(1));
            let sel = Selection::new(a, h);
            if let Some(result) = normalize(&sel, &view) {
                // endpoints must resolve
                proptest::prop_assert!(result.anchor.resolve(&view).is_some());
                proptest::prop_assert!(result.head.resolve(&view).is_some());
                // for a range, affinity_for_order must match path order
                let ra = result.anchor.resolve(&view).unwrap();
                let rh = result.head.resolve(&view).unwrap();
                if !result.is_collapsed() {
                    let (exp_a_aff, _) = affinity_for_order(ra.path(), rh.path());
                    proptest::prop_assert_eq!(ra.affinity(), exp_a_aff);
                }
            }
        }

        #[test]
        fn test_proptest_collapsed_selection_roundtrips(
            off in 0usize..=2,
        ) {
            let (pd, p1, _p2) = two_paras();
            let view = DocView::new(&pd);
            let p = Position::new(p1, off);
            if let Some(result) = normalize(&Selection::collapsed(p), &view) {
                proptest::prop_assert!(result.is_collapsed());
                proptest::prop_assert!(result.anchor.resolve(&view).is_some());
            }
        }

        // §4.8 (Task 8): collapsed_or_unit never panics; output is collapsed or unit bracket;
        // gap-cursor position always stays collapsed.
        #[test]
        fn test_proptest_b8_collapsed_or_unit_invariants(
            a_off in 0usize..=3,
            use_root in proptest::bool::ANY,
            use_upstream in proptest::bool::ANY,
        ) {
            let (pd, p1, p2) = two_paras();
            let view = DocView::new(&pd);
            let root_id = view.root().unwrap().id();

            let (node, max_off) = if use_root {
                // root has 2 children (p1, p2)
                (root_id, 2usize)
            } else {
                // p1 has 2 children ('H','i')
                (p1, 2usize)
            };
            let offset = a_off.min(max_off);
            let affinity = if use_upstream { Affinity::Upstream } else { Affinity::Downstream };
            let pos = Position { node, offset, affinity };

            // Must not panic
            let result = collapsed_or_unit(&pos, &view);

            // Result must resolve
            proptest::prop_assert!(result.anchor.resolve(&view).is_some(), "anchor resolves");
            proptest::prop_assert!(result.head.resolve(&view).is_some(), "head resolves");

            if result.is_collapsed() {
                // collapsed: trivially valid
            } else {
                // must be an adjacent-unit bracket: same node, offsets differ by 1
                proptest::prop_assert_eq!(&result.anchor.node, &result.head.node, "same node");
                let lo = result.anchor.offset.min(result.head.offset);
                let hi = result.anchor.offset.max(result.head.offset);
                proptest::prop_assert_eq!(hi - lo, 1, "unit bracket spans exactly 1 child");
                // the child at lo must be a unit
                if let Some(host) = view.node(result.anchor.node)
                    && let Some(child) = host.child_at(lo) {
                        proptest::prop_assert!(classify::child_is_unit(&child), "bracketed child is a unit");
                    }
            }

            // gap-cursor position stays collapsed
            if gap_cursor_at(&pos, &view).is_some() {
                proptest::prop_assert!(result.is_collapsed(), "gap cursor position stays collapsed");
            }

            let _ = p2;
        }
    }

    // ── Task 9 fixtures ───────────────────────────────────────────────────────

    // root > fold(fold_title fold_content(p_inside('x'))) p_outside('y')
    // Fold: isolating=true, monolithic=true → is_isolating_container
    fn fold_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(10, 1);
        let fold_title = Dot::new(10, 2);
        let fold_content = Dot::new(10, 3);
        let p_inside = Dot::new(10, 4);
        let p_outside = Dot::new(10, 6);
        let items = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                },
            ),
            (
                fold_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                },
            ),
            (
                fold_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                },
            ),
            (
                p_inside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, fold_content],
                },
            ),
            (Dot::new(10, 5), SeqItem::Char('x')),
            (
                p_outside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(10, 7), SeqItem::Char('y')),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            fold,
            fold_title,
            fold_content,
            p_outside,
        )
    }

    // ── §4.4 enclosing-unit recovery ──────────────────────────────────────────

    // A subtree-violating range where the deeper endpoint is the image unit
    // → collapses to the unit's node-selection
    #[test]
    fn test_c4_enclosing_unit_recovery() {
        // Use image_then_para fixture with a subtree-violating range:
        // anchor = (para, 0) inside the paragraph, head = (root, 0) at root level
        // path(anchor) = [1, 0], path(head) = [0]
        // head_node = [], anchor_node = [1]
        // anchor starts_with head(= [] → trivially) → that means head is ancestor
        // Let's check what subtree_violation gives:
        // a_node=[1], h_node=[], h_node.starts_with(a_node)? [] starts_with [1]? No.
        // a_node.starts_with(h_node)? [1] starts_with []? Yes (empty prefix).
        // anc=h: anc_node=[], anc_full=h_path=[0], desc_node=[1]
        // anc_slot = h_path.last() = 0, desc_child_idx = desc_node[0] = 1
        // 0 == 1? no. 0 == 1+1=2? no → false.
        // So this doesn't trigger subtree_violation with image_then_para.
        //
        // We need a case where anchor is inside a subtree and head is the node's boundary.
        // e.g. root > p1('Hi') > with anchor=(p1,1) and head=(root, 0)
        // a_node=[0], h_node=[], starts_with: yes (a starts with h=[])
        // anc_slot = h_path.last() = h_path=[0] last=0; desc_child_idx=a_node[0]=0
        // 0==0? YES → true
        // So anchor=(p1,1) head=(root,0) → subtree_violation → recover
        // recover: selection_matches_trailing_break? No (root has no paragraph context)
        // enclosing_unit: p1 inside root, root at level 0
        // a_in.resolve→path=[0,1], h_in.resolve→path=[0]
        // a_node=[0], h_node=[]
        // a starts_with h: parent_id=h.node=root, child_idx=a_node[0]=0
        // child=root.child_at(0)=p1 (Paragraph, not a unit)
        // → None → falls to unit_or_collapsed(h_in) → collapsed at (root,0)
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        let a = pos(p1, 1);
        let h = Position::new(root_id, 0);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // p1 is not a unit → falls back to collapsed at root/0
        assert!(result.is_collapsed(), "non-unit subtree overlap collapses");
    }

    // image unit as the bracketing node → enclosing_unit returns unit selection
    #[test]
    fn test_c4_enclosing_unit_image() {
        // root > image p_para('x')
        // anchor=(para,1) inside para, head=(root,0) → subtree_violation
        // enclosing_unit: parent=root, child_idx=0 (from a_node[0]=0, the para is at index 1 not 0)
        // Wait: root has [image, para], para is at index 1
        // a.resolve → para resolves to path [1, offset]
        // h.resolve → (root, 0) → path [0]
        // a_node=[1], h_node=[]
        // a starts_with h: parent=root, child_idx=a_node[0]=1
        // root.child_at(1)=para → is para a unit? No → None → collapsed
        //
        // For image test: we need anchor inside image's subtree (but image is a leaf)
        // Let's do: anchor=(root, 1, Upstream) and head=(root, 0, Downstream)
        // That's same node different offsets → not subtree_violation (same a_node=h_node=[])
        //
        // Actually for the image unit test, the subtree_violation needs anchor to be
        // "inside" (below) the image's slot. Since image is a leaf, positions inside
        // the image don't exist in the projection. Let's use blockquote instead which IS a unit:
        // root > blockquote(para('x')) p2('y')
        // anchor=(para_inside_bq, 0) → path=[0, 0, 0] (bq at idx 0, para at idx 0, offset 0)
        // head=(root, 1) → path=[1]
        // a_node=[0,0], h_node=[]
        // a starts_with h (empty): parent=root, child_idx=a_node[0]=0
        // root.child_at(0)=bq → is bq a unit? YES (monolithic) → expand_unit_at (root, 1, Up)
        let root = Dot::ROOT;
        let bq = Dot::new(11, 1);
        let bq_para = Dot::new(11, 2);
        let p_after = Dot::new(11, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                bq_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
            (Dot::new(11, 3), SeqItem::Char('x')),
            (
                p_after,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(11, 5), SeqItem::Char('y')),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        // anchor inside bq_para, head at (root, 1) — beyond bq's slot
        let a = pos(bq_para, 0); // path=[0,0,0]
        let h = Position::new(root_id, 1); // path=[1]
        // subtree_violation: a_node=[0,0], h_node=[]
        // a starts_with h: parent=root, child_idx=a_node[0]=0
        // root.child_at(0)=bq → IS unit → expand_unit_at(root, 1, Up)
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // Should be the blockquote's node-selection: (root,0,Down)..(root,1,Up) or reversed
        let lo = result.anchor.offset.min(result.head.offset);
        let hi = result.anchor.offset.max(result.head.offset);
        assert_eq!(
            result.anchor.node, root_id,
            "enclosing unit: same node as root"
        );
        assert_eq!(result.head.node, root_id);
        assert_eq!(lo, 0, "unit spans from 0");
        assert_eq!(hi, 1, "unit spans to 1");
    }

    // ── §4.5 cross-isolating promotion (Fold and Table only) ─────────────────

    // Range from inside a Fold to outside → inside endpoint promoted to fold boundary
    #[test]
    fn test_c5_cross_isolating_fold_anchor_inside() {
        let (pd, root, fold, _title, _content, p_outside) = fold_doc();
        let view = DocView::new(&pd);
        let fold_node = view.node(fold).unwrap();
        let fold_idx = fold_node.index().unwrap();
        // anchor inside fold_title (which is inside fold), head outside (p_outside)
        let a = Position::new(_title, 0);
        let h = pos(p_outside, 0);
        let result = normalize(&Selection::new(a, h), &view).unwrap();
        // Should promote anchor to fold boundary — both endpoints resolve
        assert!(
            result.anchor.resolve(&view).is_some(),
            "anchor resolves after cross-isolating"
        );
        assert!(
            result.head.resolve(&view).is_some(),
            "head resolves after cross-isolating"
        );
        // The result should not cross the fold's isolating boundary
        let ra = result.anchor.resolve(&view).unwrap();
        let rh = result.head.resolve(&view).unwrap();
        assert!(
            !subtree_violation(ra.path(), rh.path()),
            "result has no subtree_violation"
        );
        let _ = (fold_idx, root);
    }

    // ── §4.7 proptest: normalize never panics, terminates, output not subtree_violation ──

    proptest::proptest! {
        #[test]
        fn test_proptest_c7_normalize_no_subtree_violation(
            a_off in 0usize..=3,
            h_off in 0usize..=3,
            use_a_inside in proptest::bool::ANY,
        ) {
            let (pd, root, fold, _title, _content, p_outside) = fold_doc();
            let view = DocView::new(&pd);
            let root_id = view.root().unwrap().id();

            // Pick endpoints: sometimes inside fold, sometimes outside
            let a_node = if use_a_inside {
                p_outside // outside
            } else {
                root_id // at root
            };
            let a_max = view.node(a_node).map(|n| n.children().count()).unwrap_or(2);
            let h_node = p_outside;
            let h_max = view.node(h_node).map(|n| n.children().count()).unwrap_or(1);

            let a = Position::new(a_node, a_off.min(a_max));
            let h = Position::new(h_node, h_off.min(h_max));

            if let Some(result) = normalize(&Selection::new(a, h), &view) {
                let ra = result.anchor.resolve(&view);
                let rh = result.head.resolve(&view);
                proptest::prop_assert!(ra.is_some(), "anchor resolves");
                proptest::prop_assert!(rh.is_some(), "head resolves");
                if let (Some(ra), Some(rh)) = (ra, rh) {
                    // Output must not have a subtree violation
                    let out_violates = subtree_violation(ra.path(), rh.path());
                    proptest::prop_assert!(!out_violates, "output has no subtree_violation");
                }
            }
            let _ = (root, fold, _title, _content);
        }
    }
}

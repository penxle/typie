use editor_model::{AtomLeaf, ChildView, DocView, NodeType, NodeView};

use crate::affinity::Affinity;

use crate::Position;
use crate::selection::{ResolvedSelection, Selection};
use crate::traversal::{first_cursor_position, intersects_subtree, last_cursor_position};

fn paragraph_owner<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<NodeView<'a>> {
    view.node(pos.node)?
        .ancestors()
        .find(|n| n.node_type() == NodeType::Paragraph)
}

fn paragraph_start_boundary(p: &NodeView) -> Option<Position> {
    Some(Position {
        affinity: Affinity::Upstream,
        ..first_cursor_position(p)?
    })
}

fn paragraph_end_boundary(p: &NodeView) -> Option<Position> {
    Some(Position {
        affinity: Affinity::Downstream,
        ..last_cursor_position(p)?
    })
}

fn paragraph_is_empty(p: &NodeView) -> bool {
    p.node_type() == NodeType::Paragraph && p.children().count() == 0
}

fn has_trailing_page_break(p: &NodeView) -> bool {
    matches!(p.last_child(), Some(ChildView::Leaf(l)) if matches!(l.as_atom(), Some(AtomLeaf::PageBreak)))
}

fn child_node_type(c: &ChildView) -> NodeType {
    match c {
        ChildView::Block(b) => b.node_type(),
        ChildView::Leaf(l) => l.node_type(),
    }
}

fn empty_paragraph_is_removable(p: &NodeView) -> bool {
    let (Some(parent), Some(idx)) = (p.parent(), p.index()) else {
        return false;
    };
    let remaining: Vec<NodeType> = parent
        .children()
        .enumerate()
        .filter_map(|(i, c)| (i != idx).then(|| child_node_type(&c)))
        .collect();
    parent.spec().content.matches_sequence(&remaining)
}

fn after_node_position(node: &NodeView) -> Option<Position> {
    Some(Position {
        node: node.parent()?.id(),
        offset: node.index()? + 1,
        affinity: Affinity::Upstream,
    })
}

fn trailing_break_for_paragraph(p: &NodeView, _view: &DocView) -> Option<Selection> {
    if p.node_type() != NodeType::Paragraph || has_trailing_page_break(p) {
        return None;
    }
    let next = p.parent()?.child_at(p.index()? + 1)?;
    if let ChildView::Block(b) = &next
        && b.node_type() == NodeType::Paragraph
    {
        return Some(Selection::new(
            paragraph_end_boundary(p)?,
            paragraph_start_boundary(b)?,
        ));
    }
    if !paragraph_is_empty(p) || !empty_paragraph_is_removable(p) {
        return None;
    }
    Some(Selection::new(
        Position {
            node: p.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        after_node_position(p)?,
    ))
}

pub fn paragraph_break_at_end(pos: &Position, view: &DocView) -> Option<Selection> {
    let para = paragraph_owner(pos, view)?;
    let end = paragraph_end_boundary(&para)?;
    if pos.resolve(view)?.path() != end.resolve(view)?.path() {
        return None;
    }
    trailing_break_for_paragraph(&para, view)
}

fn same_boundary(a: &Position, b: &Position, view: &DocView) -> bool {
    matches!((a.resolve(view), b.resolve(view)), (Some(ra), Some(rb)) if ra.path() == rb.path())
}

pub fn before_or_same(a: &Position, b: &Position, view: &DocView) -> bool {
    matches!((a.resolve(view), b.resolve(view)), (Some(ra), Some(rb)) if ra.path() <= rb.path())
}

fn collect_closest_break<'a>(
    rs: &ResolvedSelection<'a>,
    node: &NodeView<'a>,
    from: &Position,
    to: &Position,
    forward: bool,
    view: &'a DocView<'a>,
    best: &mut Option<Position>,
) {
    if !intersects_subtree(rs, node) {
        return;
    }
    if node.node_type() == NodeType::Paragraph {
        let candidate_pos = Position {
            node: node.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let candidate_sel = paragraph_break_at_end(&candidate_pos, view);
        if let Some(break_sel) = candidate_sel {
            let head = &break_sel.head;
            let head_crosses = if forward {
                before_or_same(from, head, view)
                    && before_or_same(head, to, view)
                    && !same_boundary(head, from, view)
            } else {
                before_or_same(to, head, view)
                    && before_or_same(head, from, view)
                    && !same_boundary(head, from, view)
            };
            if head_crosses {
                let is_closer = if let Some(current_best) = best.as_ref() {
                    if forward {
                        before_or_same(head, current_best, view)
                            && !same_boundary(head, current_best, view)
                    } else {
                        before_or_same(current_best, head, view)
                            && !same_boundary(head, current_best, view)
                    }
                } else {
                    true
                };
                if is_closer {
                    *best = Some(*head);
                }
            }
        }
    }
    for child in node.child_blocks() {
        collect_closest_break(rs, &child, from, to, forward, view, best);
    }
}

pub fn closest_empty_paragraph_break_end_between<'a>(
    from: &Position,
    to: &Position,
    view: &'a DocView<'a>,
) -> Option<Position> {
    if same_boundary(from, to, view) {
        return None;
    }
    let rs = Selection::new(*from, *to).resolve(view)?;
    let forward = rs.anchor() < rs.head();
    let mut best = None;
    let root = view.node(rs.common_ancestor())?;
    collect_closest_break(&rs, &root, from, to, forward, view, &mut best);
    best
}

pub(crate) fn selection_matches_trailing_break(sel: &Selection, view: &DocView) -> bool {
    let Some(rs) = sel.resolve(view) else {
        return false;
    };
    let (from, to) = (rs.from().position(), rs.to().position());
    paragraph_break_at_end(&from, view).is_some_and(|pb| Selection::new(from, to) == pb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc, SeqItem,
        SpanLog, project_document,
    };

    use crate::Position;
    use crate::normalize::normalize;

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
            node_carries: ModifierAttrLog::new(),
        }
    }

    fn pos_aff(node: Dot, offset: usize, aff: Affinity) -> Position {
        Position {
            node,
            offset,
            affinity: aff,
        }
    }

    // root > p1('Hi') p2('y')
    fn two_paras() -> (ProjectedDoc, Dot, Dot, Dot) {
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
        (project_document(&logs(&items)).unwrap(), root, p1, p2)
    }

    // root > p1('Hi') callout(p_inside('x'))
    // Callout is monolithic=true, isolating=false → NOT isolating_container
    fn para_then_callout() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(2, 1);
        let callout = Dot::new(2, 4);
        let p_inside = Dot::new(2, 5);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(2, 2), SeqItem::Char('H')),
            (Dot::new(2, 3), SeqItem::Char('i')),
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root],
                },
            ),
            (
                p_inside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                },
            ),
            (Dot::new(2, 6), SeqItem::Char('x')),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            p1,
            callout,
            p_inside,
        )
    }

    // root > p1('Hi') + trailing PageBreak  p2('y')
    fn para_with_page_break() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let p1 = Dot::new(3, 1);
        let p2 = Dot::new(3, 10);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(3, 2), SeqItem::Char('H')),
            (Dot::new(3, 3), SeqItem::Char('i')),
            (Dot::new(3, 4), SeqItem::Atom(AtomLeaf::PageBreak)),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(3, 11), SeqItem::Char('y')),
        ];
        (project_document(&logs(&items)).unwrap(), root, p1, p2)
    }

    // root > empty_p  callout(p_inside)
    // empty_p is removable (root content allows it to be gone since root takes ZeroOrMore)
    fn empty_para_before_callout() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let empty_p = Dot::new(4, 1);
        let callout = Dot::new(4, 2);
        let p_inside = Dot::new(4, 3);
        let items = vec![
            (
                empty_p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root],
                },
            ),
            (
                p_inside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                },
            ),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            empty_p,
            callout,
        )
    }

    // root > callout(p1('a'), empty_p)
    // Only Root gets a derived trailing Paragraph (its schema ends with ", Paragraph").
    // Callout schema is (Paragraph | BulletList | OrderedList)+, so empty_p is the genuine
    // last child of callout with NO next sibling in the projected tree.
    // Removing empty_p leaves [Paragraph('a')] which still satisfies (…)+, so it IS removable.
    // trailing_break_for_paragraph must return None because child_at(index+1) is None.
    fn empty_para_last_in_callout() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let callout = Dot::new(5, 1);
        let p1 = Dot::new(5, 2);
        let empty_p = Dot::new(5, 4);
        let items = vec![
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root],
                },
            ),
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                },
            ),
            (Dot::new(5, 3), SeqItem::Char('a')),
            (
                empty_p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                },
            ),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            callout,
            p1,
            empty_p,
        )
    }

    // ── §4.1 subtree_violation true/false (path arithmetic) ───────────────────

    #[test]
    fn test_subtree_violation_true_adjacent_slot() {
        use crate::normalize::subtree_violation_pub as subtree_violation;
        // a_path = [0, 2, 3] (offset 3 in node at depth [0,2])
        // h_path = [0, 2, 1, 0] (offset 0 in node at depth [0,2,1])
        // a_node = [0,2], h_node = [0,2,1] → h_node starts_with a_node (h is descendant)
        // anc = a, desc = h: anc_node=[0,2], anc_full=[0,2,3], desc_node=[0,2,1]
        // anc_slot = 3, desc_child_idx = desc_node[2] = 1
        // 3 == 1? no. 3 == 1+1=2? no. → false?? Let me try a case that IS true:
        // anc_slot = desc_child_idx: anc_slot=1, desc_child_idx=1 → true
        // a_path = [0,2,1] (offset 1 in node [0,2]), h_path = [0,2,1,0] (offset 0 in node [0,2,1])
        // a_node=[0,2], h_node=[0,2,1] → h starts_with a → h is desc
        // anc=a: anc_node=[0,2], anc_full=[0,2,1], anc_slot=1; desc_child_idx=h_node[2]=1
        // 1==1 → true
        assert!(subtree_violation(&[0, 2, 1], &[0, 2, 1, 0]));
    }

    #[test]
    fn test_subtree_violation_true_slot_plus_one() {
        use crate::normalize::subtree_violation_pub as subtree_violation;
        // anc_slot = desc_child_idx + 1:
        // a_path=[0,2], h_path=[0,2,1,0,5]  (but h_node=[0,2,1,0])
        // Wait, for desc_child_idx = 0 and anc_slot=1: anc_full last = 1, desc_node[anc_node.len()] = 0
        // e.g. a_path=[0,1], h_path=[0,0,5]
        // a_node=[0], h_node=[0,0] → h starts_with a
        // anc=a: anc_node=[0], anc_full=[0,1], anc_slot=1; desc_child_idx=h_node[1]=0
        // 1==0? no. 1==0+1=1? yes → true
        assert!(subtree_violation(&[0, 1], &[0, 0, 5]));
    }

    #[test]
    fn test_subtree_violation_false_same_node() {
        use crate::normalize::subtree_violation_pub as subtree_violation;
        // Same node (both in p1): a_node == h_node → false
        assert!(!subtree_violation(&[0, 2], &[0, 5]));
    }

    #[test]
    fn test_subtree_violation_false_unrelated() {
        use crate::normalize::subtree_violation_pub as subtree_violation;
        // Unrelated: neither starts_with the other → false
        assert!(!subtree_violation(&[0, 2, 3], &[1, 0, 1]));
    }

    // ── §4.2 paragraph-break exception (forward + reversed direction) ─────────

    #[test]
    fn test_paragraph_break_forward() {
        let (pd, _root, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let p1_node = view.node(p1).unwrap();
        let p2_node = view.node(p2).unwrap();
        let p1_end = p1_node.children().count();
        // forward: anchor=p1 end, head=p2 start
        let sel = Selection::new(
            pos_aff(p1, p1_end, Affinity::Downstream),
            pos_aff(p2, 0, Affinity::Upstream),
        );
        let result = normalize(&sel, &view).unwrap();
        // Should be returned unchanged (paragraph-break exception)
        assert_eq!(result, sel, "paragraph-break span preserved (forward)");
        let _ = p2_node;
    }

    #[test]
    fn test_paragraph_break_reversed() {
        let (pd, _root, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let p1_node = view.node(p1).unwrap();
        let p1_end = p1_node.children().count();
        // reversed: anchor=p2 start, head=p1 end
        // from=p1_end (smaller path), to=p2_start (larger path)
        // selection_matches_trailing_break checks from→to so it should still match
        let sel = Selection::new(
            pos_aff(p2, 0, Affinity::Upstream),
            pos_aff(p1, p1_end, Affinity::Downstream),
        );
        let result = normalize(&sel, &view).unwrap();
        assert_eq!(result, sel, "paragraph-break span preserved (reversed)");
    }

    // ── §4.2b paragraph→non-paragraph (callout) — NOT a break ────────────────

    #[test]
    fn test_para_to_callout_not_a_break() {
        let (pd, _root, p1, _callout, _p_inside) = para_then_callout();
        let view = DocView::new(&pd);
        let p1_node = view.node(p1).unwrap();
        let p1_end = p1_node.children().count();
        // anchor = p1 end (Downstream), head = p1 end (Upstream) → same boundary → collapsed
        // Instead test: selection from inside p1 to outside that would cross subtree
        // Actually, selection_matches_trailing_break only applies when there IS a break
        // Here the "next sibling" of p1 is Callout (not Paragraph) → no break
        let result = paragraph_break_at_end(&pos_aff(p1, p1_end, Affinity::Downstream), &view);
        assert!(
            result.is_none(),
            "p1 followed by callout should not produce a paragraph break"
        );
    }

    // ── §4.2c trailing PageBreak → no break ───────────────────────────────────

    #[test]
    fn test_trailing_page_break_no_break() {
        let (pd, _root, p1, _p2) = para_with_page_break();
        let view = DocView::new(&pd);
        let p1_node = view.node(p1).unwrap();
        let p1_end = p1_node.children().count();
        let result = paragraph_break_at_end(&pos_aff(p1, p1_end, Affinity::Downstream), &view);
        assert!(
            result.is_none(),
            "paragraph with trailing PageBreak must not produce a break"
        );
    }

    // ── §4.3 removable empty paragraph before callout → break ────────────────

    #[test]
    fn test_removable_empty_before_callout() {
        let (pd, _root, empty_p, _callout) = empty_para_before_callout();
        let view = DocView::new(&pd);
        // empty_p is at offset 0 (= paragraph_end since it's empty)
        let result = paragraph_break_at_end(&pos_aff(empty_p, 0, Affinity::Downstream), &view);
        assert!(
            result.is_some(),
            "removable empty paragraph before callout should produce a break"
        );
        let pb = result.unwrap();
        // Should be (empty_p,0,Downstream)..(parent, idx+1, Upstream)
        let anchor = &pb.anchor;
        let head = &pb.head;
        assert_eq!(anchor.node, empty_p);
        assert_eq!(anchor.offset, 0);
        assert_eq!(anchor.affinity, Affinity::Downstream);
        assert_eq!(head.affinity, Affinity::Upstream);
    }

    // ── §4.3 removable empty paragraph with NO next sibling → NO break ────────
    //
    // Only Root gets a derived trailing Paragraph (its schema ends with ", Paragraph").
    // Callout has (Paragraph | BulletList | OrderedList)+, so a paragraph that is the
    // last child of a Callout has a genuine None next sibling in the projected tree.
    // The `?` on line 70 of trailing_break_for_paragraph guards exactly this case.

    #[test]
    fn test_removable_empty_no_next_sibling_no_break() {
        let (pd, _root, _callout, _p1, empty_p) = empty_para_last_in_callout();
        let view = DocView::new(&pd);
        let empty_p_nv = view.node(empty_p).unwrap();
        let idx = empty_p_nv.index().unwrap();
        let parent_nv = empty_p_nv.parent().unwrap();

        // Precondition: empty_p genuinely has no next sibling in the projected tree.
        // Callout does not synthesize a derived trailing paragraph, so child_at(idx+1) is None.
        assert!(
            parent_nv.child_at(idx + 1).is_none(),
            "empty_p must be the last child of callout with no next sibling"
        );

        // Guard assertion: no next sibling → trailing_break_for_paragraph returns None.
        let result = trailing_break_for_paragraph(&empty_p_nv, &view);
        assert!(
            result.is_none(),
            "removable empty paragraph with no next sibling must not produce a break"
        );
    }

    // ── §4.3 direct: trailing_break_for_paragraph returns None when no next sibling ──

    #[test]
    fn test_trailing_break_none_when_no_next_sibling() {
        // Reuse the same callout fixture to confirm paragraph_break_at_end also returns None.
        let (pd, _root, _callout, _p1, empty_p) = empty_para_last_in_callout();
        let view = DocView::new(&pd);
        let empty_p_nv = view.node(empty_p).unwrap();
        let idx = empty_p_nv.index().unwrap();
        let parent_nv = empty_p_nv.parent().unwrap();

        // Confirm the no-next-sibling precondition.
        assert!(
            parent_nv.child_at(idx + 1).is_none(),
            "precondition: empty_p is the last child of callout"
        );

        // paragraph_break_at_end must return None: the §4.3 guard fires at the `?` on
        // `child_at(index+1)`, before reaching the removable-empty-paragraph path.
        let result = paragraph_break_at_end(&pos_aff(empty_p, 0, Affinity::Downstream), &view);
        assert!(
            result.is_none(),
            "paragraph_break_at_end must return None when the paragraph has no next sibling"
        );
    }

    // ── §4.8 closest_empty_paragraph_break_end_between ────────────────────────

    // root > empty_p > p2('y')  — crossing empty_p gives its break head
    #[test]
    fn test_closest_empty_break_forward() {
        let (pd, _root, p1, p2) = two_paras();
        let _view = DocView::new(&pd);

        // Build a doc where p1 is empty (two_paras has p1 with 'Hi', so use a different fixture)
        // Instead, use empty_para_before_callout which has an empty paragraph first.
        let (pd2, _root2, empty_p, _callout) = empty_para_before_callout();
        let view2 = DocView::new(&pd2);

        // from = before empty_p, to = after the callout-related position
        // Actually: from = (empty_p, 0, Down), to = (callout-child, 0, Down)
        // This range crosses empty_p, so closest break should be found.
        let pb = paragraph_break_at_end(&pos_aff(empty_p, 0, Affinity::Downstream), &view2);
        let pb = pb.expect("empty_p must have a break for this test");

        let from = pos_aff(empty_p, 0, Affinity::Downstream);
        let to = pb.head;

        // same_boundary(from, to) = false (different paths), so should search
        let result = closest_empty_paragraph_break_end_between(&from, &to, &view2);
        assert!(
            result.is_some(),
            "range crossing empty_p should find its break head"
        );
        let _ = (p1, p2);
    }

    // No empty paragraph in range → None
    #[test]
    fn test_closest_empty_break_no_empty_paragraph() {
        let (pd, _root, p1, p2) = two_paras();
        let view = DocView::new(&pd);

        // Selection within p1 (non-empty) → no empty paragraph break
        let from = pos_aff(p1, 0, Affinity::Downstream);
        let to = pos_aff(p1, 2, Affinity::Upstream);
        let result = closest_empty_paragraph_break_end_between(&from, &to, &view);
        assert_eq!(result, None, "no empty paragraph in range → None");
        let _ = p2;
    }

    // §4.8 PATH-ONLY ordering trap test:
    // from and to at SAME path but DIFFERENT affinity → same_boundary → None (NOT split by raw Ord)
    #[test]
    fn test_closest_empty_break_same_path_different_affinity_is_same_boundary() {
        let (pd, _root, p1, _p2) = two_paras();
        let view = DocView::new(&pd);

        // from = (p1, 1, Upstream), to = (p1, 1, Downstream)
        // These have different raw Ord (Upstream < Downstream in our Ord impl) but same PATH [0, 1, 1]
        // same_boundary must treat them as same boundary → return None
        let from = pos_aff(p1, 1, Affinity::Upstream);
        let to = pos_aff(p1, 1, Affinity::Downstream);
        let result = closest_empty_paragraph_break_end_between(&from, &to, &view);
        assert_eq!(
            result, None,
            "same path with different affinity must be treated as same boundary (path-only, not raw Ord)"
        );
    }
}

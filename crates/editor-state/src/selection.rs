use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{ChildView, DocView, NodeView};
use serde::{Deserialize, Serialize};

use crate::{Position, ResolvedPosition};

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn collapsed(pos: Position) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn normalize(&self, view: &DocView) -> Option<Selection> {
        crate::normalize::normalize(self, view)
    }

    pub fn resolve<'a>(&self, view: &'a DocView<'a>) -> Option<ResolvedSelection<'a>> {
        let anchor = self.anchor.resolve(view)?;
        let head = self.head.resolve(view)?;
        Some(ResolvedSelection { view, anchor, head })
    }
}

pub struct ResolvedSelection<'a> {
    view: &'a DocView<'a>,
    anchor: ResolvedPosition<'a>,
    head: ResolvedPosition<'a>,
}

impl<'a> ResolvedSelection<'a> {
    pub fn view(&self) -> &'a DocView<'a> {
        self.view
    }

    pub fn anchor(&self) -> &ResolvedPosition<'a> {
        &self.anchor
    }

    pub fn head(&self) -> &ResolvedPosition<'a> {
        &self.head
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn from(&self) -> &ResolvedPosition<'a> {
        if self.anchor <= self.head {
            &self.anchor
        } else {
            &self.head
        }
    }

    pub fn to(&self) -> &ResolvedPosition<'a> {
        if self.anchor >= self.head {
            &self.anchor
        } else {
            &self.head
        }
    }

    pub fn contains_subtree(&self, node: &NodeView) -> bool {
        crate::traversal::contains_subtree(self, node)
    }

    /// Returns whether a direct leaf child slot of `block` is fully covered by
    /// this selection. The slot is interpreted as the half-open path extent
    /// `[slot, slot + 1)`, so coverage is independent of endpoint affinity.
    pub fn contains_leaf_slot(&self, block: &NodeView, slot: usize) -> bool {
        crate::traversal::contains_leaf_slot(self, block, slot)
    }

    pub fn intersects_subtree(&self, node: &NodeView) -> bool {
        crate::traversal::intersects_subtree(self, node)
    }

    pub fn as_cell_rect(&self) -> Option<crate::cell_selection::CellRect<'a>> {
        crate::cell_selection::as_cell_rect(self)
    }

    /// Visible text covered by the selection (chars only, atoms skipped),
    /// concatenated in document order.
    pub fn collect_text(&self) -> String {
        let from = self.from().position();
        let to = self.to().position();
        crate::inline_leaf_dots_in_range(self.view, &from, &to)
            .into_iter()
            .filter_map(|dot| self.view.leaf(dot).and_then(|l| l.as_char()))
            .collect()
    }

    pub fn contains_range(&self, range: Selection) -> bool {
        let Some(resolved) = range.resolve(self.view) else {
            return false;
        };
        self.from() <= resolved.from() && resolved.to() <= self.to()
    }

    pub(crate) fn common_ancestor(&self) -> Dot {
        let a_path = self.anchor.path();
        let h_path = self.head.path();
        let a_node_path = &a_path[..a_path.len().saturating_sub(1)];
        let h_node_path = &h_path[..h_path.len().saturating_sub(1)];

        let prefix_len = a_node_path
            .iter()
            .zip(h_node_path.iter())
            .take_while(|(a, h)| a == h)
            .count();
        let prefix = &a_node_path[..prefix_len];

        let root = match self.view.root() {
            Some(r) => r,
            None => {
                return self
                    .view
                    .root()
                    .map(|r| r.id())
                    .unwrap_or_else(|| self.anchor.node());
            }
        };

        if prefix.is_empty() {
            return root.id();
        }

        let mut node = root;
        for &i in prefix {
            match node.child_at(i) {
                Some(ChildView::Block(b)) => node = b,
                _ => {
                    return self
                        .view
                        .root()
                        .map(|r| r.id())
                        .unwrap_or_else(|| self.anchor.node());
                }
            }
        }
        node.id()
    }
}

impl<'a> From<&ResolvedSelection<'a>> for Selection {
    fn from(rs: &ResolvedSelection<'a>) -> Self {
        Selection {
            anchor: rs.anchor.position(),
            head: rs.head.position(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc, SeqItem, SpanLog,
        project_document,
    };

    use crate::Position;

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
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('y')),
        ];
        (project_document(&logs(&items)).unwrap(), p1, p2)
    }

    fn bq_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let p_inside = Dot::new(1, 2);
        let p_outside = Dot::new(1, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p_inside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
            (
                p_outside,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('y')),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            bq,
            p_inside,
            p_outside,
        )
    }

    fn two_bq_paras() -> (ProjectedDoc, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let p1 = Dot::new(1, 2);
        let p2 = Dot::new(1, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('a')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('b')),
        ];
        (project_document(&logs(&items)).unwrap(), bq, p1, p2)
    }

    #[test]
    fn test_1_collapsed() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let pos = Position::new(p1, 1);
        let sel = Selection::collapsed(pos);
        assert!(sel.is_collapsed());
        let rsel = sel.resolve(&view).unwrap();
        assert!(rsel.is_collapsed());
        assert_eq!(rsel.from().position(), pos);
        assert_eq!(rsel.to().position(), pos);
    }

    #[test]
    fn test_2_direction_preserved() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 0);
        let h = Position::new(p2, 0);
        // head (p2) is after anchor (p1) in document order, but let's do it reversed:
        // anchor = p2 (later), head = p1 (earlier)
        let sel = Selection::new(h, a);
        assert_eq!(sel.anchor, h);
        assert_eq!(sel.head, a);
        let rsel = sel.resolve(&view).unwrap();
        // anchor() and head() reflect original roles
        assert_eq!(rsel.anchor().position(), h);
        assert_eq!(rsel.head().position(), a);
        // from() = doc-order minimum = head (p1, earlier), to() = anchor (p2, later)
        assert_eq!(rsel.from().position(), a);
        assert_eq!(rsel.to().position(), h);
    }

    #[test]
    fn test_4_common_ancestor_same_block() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 0);
        let h = Position::new(p1, 1);
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert_eq!(sel.common_ancestor(), p1);
    }

    #[test]
    fn test_4_common_ancestor_sibling_blocks_root() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let root_id = view.root().unwrap().id();
        let a = Position::new(p1, 0);
        let h = Position::new(p2, 0);
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert_eq!(sel.common_ancestor(), root_id);
    }

    #[test]
    fn test_5_common_ancestor_sibling_blocks_in_blockquote() {
        let (pd, bq, p1, p2) = two_bq_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 0);
        let h = Position::new(p2, 0);
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert_eq!(sel.common_ancestor(), bq);
    }

    #[test]
    fn test_8_round_trip() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p2, 0);
        let h = Position::new(p1, 1);
        let orig = Selection::new(a, h);
        let rsel = orig.resolve(&view).unwrap();
        let back = Selection::from(&rsel);
        assert_eq!(back, orig);
        // direction preserved
        assert_eq!(back.anchor, a);
        assert_eq!(back.head, h);
    }

    #[test]
    fn test_resolve_none_dead_endpoint() {
        let (pd, p1, _p2) = two_paras();
        let view = DocView::new(&pd);
        let good = Position::new(p1, 0);
        let dead = Position::new(Dot::new(9, 9), 0);
        assert!(Selection::new(good, dead).resolve(&view).is_none());
        let out_of_range = Position::new(p1, 99);
        assert!(Selection::new(good, out_of_range).resolve(&view).is_none());
    }

    #[test]
    fn test_common_ancestor_nested() {
        let (pd, root, _bq, p_inside, p_outside) = bq_doc();
        let view = DocView::new(&pd);
        let root_id = root;
        let a = Position::new(p_inside, 0);
        let h = Position::new(p_outside, 0);
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert_eq!(sel.common_ancestor(), root_id);
    }

    proptest::proptest! {
        #[test]
        fn test_proptest_resolve_never_panics_and_invariants(
            a_off in 0usize..=2,
            h_off in 0usize..=2,
        ) {
            let (pd, p1, p2) = two_paras();
            let view = DocView::new(&pd);
            let anchor = Position::new(p1, a_off);
            let head = Position::new(p2, h_off.min(1));
            let sel = Selection::new(anchor, head);
            if let Some(rsel) = sel.resolve(&view) {
                proptest::prop_assert!(rsel.from() <= rsel.to());
                let anc = rsel.common_ancestor();
                proptest::prop_assert!(view.node(anc).is_some());
            }
        }
    }

    #[test]
    fn contains_range_true_false_unresolvable() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let a = Position::new(p1, 0);
        let h = Position::new(p2, 1);
        let rs = Selection::new(a, h).resolve(&view).unwrap();

        let inner = Selection::new(Position::new(p1, 0), Position::new(p1, 1));
        assert!(rs.contains_range(inner));

        let outside = Selection::new(Position::new(p1, 0), Position::new(p2, 1));
        assert!(rs.contains_range(outside));

        let past_end = Selection::new(Position::new(p1, 0), Position::new(p2, 99));
        assert!(!rs.contains_range(past_end));

        let bogus = Selection::new(Position::new(Dot::new(9, 9), 0), Position::new(p1, 1));
        assert!(!rs.contains_range(bogus));
    }

    #[test]
    fn contains_range_resolvable_but_extends_past() {
        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        // outer rs covers only p1 (p1 has 2 children: 'H','i' → offsets 0..2)
        let rs = Selection::new(Position::new(p1, 0), Position::new(p1, 2))
            .resolve(&view)
            .unwrap();

        // p2 has 1 child ('y') → offset 0 is in-bounds and resolves successfully,
        // but p2@0 is after p1@2 in document order, so resolved.to() > self.to()
        // → contains_range returns false via the inequality branch, NOT the early-return
        let extends_past = Selection::new(Position::new(p1, 0), Position::new(p2, 0));
        assert!(
            extends_past.resolve(&view).is_some(),
            "extends_past must resolve so we exercise the inequality branch"
        );
        assert!(
            !rs.contains_range(extends_past),
            "resolves successfully but extends past outer to() — inequality branch returns false"
        );
    }

    #[test]
    fn contains_subtree_method_matches_free_fn() {
        use crate::traversal;

        let (pd, _root, bq, p_inside, p_outside) = bq_doc();
        let view = DocView::new(&pd);
        let a = Position::new(bq, 0);
        let h = Position::new(p_inside, 1);
        let rs = Selection::new(a, h).resolve(&view).unwrap();

        let p_inside_id = p_inside;
        let in_node = view.node(p_inside_id).unwrap();
        assert!(
            rs.contains_subtree(&in_node),
            "p_inside is fully covered by the selection and must be contained"
        );
        assert_eq!(
            rs.contains_subtree(&in_node),
            traversal::contains_subtree(&rs, &in_node),
        );

        let p_outside_id = p_outside;
        let out_node = view.node(p_outside_id).unwrap();
        assert!(
            !rs.contains_subtree(&out_node),
            "p_outside lies after the selection's to() and must not be contained"
        );
        assert_eq!(
            rs.contains_subtree(&out_node),
            traversal::contains_subtree(&rs, &out_node),
        );
    }

    #[test]
    fn contains_leaf_slot_method_matches_free_fn() {
        use crate::traversal;

        let (pd, p1, p2) = two_paras();
        let view = DocView::new(&pd);
        let rs = Selection::new(Position::new(p1, 1), Position::new(p2, 0))
            .resolve(&view)
            .unwrap();
        let p1_node = view.node(p1).unwrap();
        let p2_node = view.node(p2).unwrap();
        let root_node = view.root().unwrap();
        let block_rs = Selection::new(
            Position::new(root_node.id(), 0),
            Position::new(root_node.id(), 1),
        )
        .resolve(&view)
        .unwrap();

        assert!(!rs.contains_leaf_slot(&p1_node, 0));
        assert!(rs.contains_leaf_slot(&p1_node, 1));
        assert!(!rs.contains_leaf_slot(&p2_node, 0));
        assert!(!block_rs.contains_leaf_slot(&root_node, 0));
        assert_eq!(
            rs.contains_leaf_slot(&p1_node, 1),
            traversal::contains_leaf_slot(&rs, &p1_node, 1),
        );
    }

    #[test]
    fn as_cell_rect_method_matches_free_fn() {
        use crate::affinity::Affinity;
        use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
        use editor_model::{
            DocLogs, ModifierAttrLog, NodeAttrLog, NodeType, SeqItem, SpanLog, project_document,
        };

        let root = Dot::ROOT;
        let table = Dot::new(2, 1);
        let row0 = Dot::new(2, 2);
        let cell00 = Dot::new(2, 3);
        let cell01 = Dot::new(2, 4);
        let mut counter = 10u64;
        let mut next = || {
            let d = Dot::new(2, counter);
            counter += 1;
            d
        };
        let items = [
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row0,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell00,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell00],
                    attrs: vec![],
                },
            ),
            (
                cell01,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell01],
                    attrs: vec![],
                },
            ),
        ];

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
        let doc_logs = DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        };
        let pd = project_document(&doc_logs).unwrap();
        let view = DocView::new(&pd);

        let a = Position {
            node: row0,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: row0,
            offset: 2,
            affinity: Affinity::Downstream,
        };
        let rs = Selection::new(a, h).resolve(&view).unwrap();

        assert!(rs.as_cell_rect().is_some());
        let cr_method = rs.as_cell_rect().unwrap();
        let cr_free = crate::cell_selection::as_cell_rect(&rs).unwrap();
        let method_ids: Vec<Dot> = cr_method.cells().into_iter().map(|c| c.id()).collect();
        let free_ids: Vec<Dot> = cr_free.cells().into_iter().map(|c| c.id()).collect();
        assert_eq!(method_ids, free_ids);
        assert!(method_ids.contains(&cell00));
        assert!(method_ids.contains(&cell01));

        let (pd2, p1, _p2) = two_paras();
        let view2 = DocView::new(&pd2);
        let a2 = Position::new(p1, 0);
        let h2 = Position::new(p1, 1);
        let rs2 = Selection::new(a2, h2).resolve(&view2).unwrap();
        assert!(rs2.as_cell_rect().is_none());
    }
}

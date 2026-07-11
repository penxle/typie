use editor_model::{ChildView, DocView, NodeType, NodeView, Schema};

use crate::affinity::Affinity;

use crate::{Position, classify, selection::ResolvedSelection};

pub enum GapCursor<'a> {
    BetweenMonolithic {
        parent: NodeView<'a>,
        before: ChildView<'a>,
        after: ChildView<'a>,
        index: usize,
    },
    /// Between an isolating container's own boundary and its adjacent unit
    /// child: `index == 0` is the leading boundary (unit is the first
    /// child), `index == child count` is the trailing boundary (unit is
    /// the last child). The document root is isolating, so the document's
    /// leading gap is this variant with the root as host.
    IsolatingBoundary {
        host: NodeView<'a>,
        unit: ChildView<'a>,
        index: usize,
    },
}

pub fn isolating_boundary_at<'a>(host: &NodeView<'a>, index: usize) -> Option<ChildView<'a>> {
    if !Schema::node_spec(host.node_type()).isolating {
        return None;
    }
    let children: Vec<ChildView> = host.children().collect();
    let count = children.len();
    if count == 0 || (index != 0 && index != count) {
        return None;
    }
    let unit = host.child_at(if index == 0 { 0 } else { count - 1 })?;
    if !classify::child_is_unit(&unit) {
        return None;
    }
    let ty = |c: &ChildView| match c {
        ChildView::Block(b) => b.node_type(),
        ChildView::Leaf(l) => l.node_type(),
    };
    let mut seq: Vec<NodeType> = children.iter().map(ty).collect();
    seq.insert(index, NodeType::Paragraph);
    Schema::node_spec(host.node_type())
        .content
        .matches_sequence(&seq)
        .then_some(unit)
}

pub fn between_monolithic_at(host: &NodeView, index: usize) -> bool {
    let children: Vec<ChildView> = host.children().collect();
    let count = children.len();
    if index == 0 || index >= count {
        return false;
    }
    let ty = |c: &ChildView| match c {
        ChildView::Block(b) => b.node_type(),
        ChildView::Leaf(l) => l.node_type(),
    };
    if !Schema::node_spec(ty(&children[index - 1])).monolithic
        || !Schema::node_spec(ty(&children[index])).monolithic
    {
        return false;
    }
    let mut seq: Vec<NodeType> = children.iter().map(ty).collect();
    seq.insert(index, NodeType::Paragraph);
    Schema::node_spec(host.node_type())
        .content
        .matches_sequence(&seq)
}

pub fn gap_cursor_at<'a>(pos: &Position, view: &'a DocView<'a>) -> Option<GapCursor<'a>> {
    let host = view.node(pos.node)?;
    // Boundary gaps claim only the position encodings whose affinity faces
    // the boundary — (0, Upstream) / (count, Downstream). The opposite
    // affinities keep their existing meaning (unit node-selection
    // expansion in normalize).
    let count = host.child_count();
    let faces_boundary = (pos.offset == 0 && pos.affinity == Affinity::Upstream)
        || (count > 0 && pos.offset == count && pos.affinity == Affinity::Downstream);
    if faces_boundary && let Some(unit) = isolating_boundary_at(&host, pos.offset) {
        return Some(GapCursor::IsolatingBoundary {
            host,
            unit,
            index: pos.offset,
        });
    }
    if !between_monolithic_at(&host, pos.offset) {
        return None;
    }
    let before = host.child_at(pos.offset - 1)?;
    let after = host.child_at(pos.offset)?;
    Some(GapCursor::BetweenMonolithic {
        parent: host,
        before,
        after,
        index: pos.offset,
    })
}

pub fn as_gap_cursor<'a>(rs: &ResolvedSelection<'a>) -> Option<GapCursor<'a>> {
    if !rs.is_collapsed() {
        return None;
    }
    gap_cursor_at(&rs.head().position(), rs.view())
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc,
        SeqItem, SpanLog, project_document,
    };

    use crate::{Position, selection::Selection};

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
            aliases: AliasLog::new(),
        }
    }

    fn image_first_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let img_dot = Dot::new(1, 1);
        let para = Dot::new(1, 2);
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
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), root)
    }

    fn two_folds_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let fold1 = Dot::new(1, 1);
        let fold1_title = Dot::new(1, 2);
        let fold1_content = Dot::new(1, 3);
        let fold2 = Dot::new(1, 4);
        let fold2_title = Dot::new(1, 5);
        let fold2_content = Dot::new(1, 6);
        let para = Dot::new(1, 7);
        let items = vec![
            (
                fold1,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                fold1_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold1],
                    attrs: vec![],
                },
            ),
            (
                fold1_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold1],
                    attrs: vec![],
                },
            ),
            (
                fold2,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                fold2_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold2],
                    attrs: vec![],
                },
            ),
            (
                fold2_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold2],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), root)
    }

    // root > fold > [fold_title, fold_content > callout > para] + para
    fn fold_callout_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(2, 1);
        let title = Dot::new(2, 2);
        let content = Dot::new(2, 3);
        let callout = Dot::new(2, 4);
        let inner_para = Dot::new(2, 5);
        let para = Dot::new(2, 6);
        let items = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root, fold, content],
                    attrs: vec![],
                },
            ),
            (
                inner_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, content, callout],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), content)
    }

    // root > fold > [fold_title, fold_content > para] + para
    fn fold_paragraph_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(3, 1);
        let title = Dot::new(3, 2);
        let content = Dot::new(3, 3);
        let inner_para = Dot::new(3, 4);
        let para = Dot::new(3, 5);
        let items = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                inner_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, content],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), content)
    }

    #[test]
    fn test_isolating_boundary_at_fold_content_with_callout() {
        let (pd, content) = fold_callout_doc();
        let view = DocView::new(&pd);
        let host = view.node(content).unwrap();
        assert!(
            isolating_boundary_at(&host, 0).is_some(),
            "leading boundary of isolating fold_content next to a callout is a gap"
        );
        assert!(
            isolating_boundary_at(&host, 1).is_some(),
            "trailing boundary of isolating fold_content next to a callout is a gap"
        );
        assert!(
            isolating_boundary_at(&host, 2).is_none(),
            "out-of-range index is not a gap"
        );
    }

    #[test]
    fn test_isolating_boundary_at_root() {
        let (pd, _root) = two_folds_doc();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        assert!(
            isolating_boundary_at(&root_node, 0).is_some(),
            "root is isolating; its leading slot before a fold is a boundary gap"
        );
        assert!(
            isolating_boundary_at(&root_node, 3).is_none(),
            "root trailing slot is never a gap — the last child is always a paragraph"
        );
    }

    #[test]
    fn test_isolating_boundary_at_rejects_non_unit_neighbor() {
        let (pd, content) = fold_paragraph_doc();
        let view = DocView::new(&pd);
        let host = view.node(content).unwrap();
        assert!(
            isolating_boundary_at(&host, 0).is_none(),
            "a paragraph neighbor is not a unit — no boundary gap"
        );
        assert!(
            isolating_boundary_at(&host, 1).is_none(),
            "a paragraph neighbor is not a unit — no boundary gap"
        );
    }

    #[test]
    fn test_gap_cursor_at_isolating_boundary_affinity_gates() {
        let (pd, content) = fold_callout_doc();
        let view = DocView::new(&pd);
        let at = |offset, affinity| {
            gap_cursor_at(
                &Position {
                    node: content,
                    offset,
                    affinity,
                },
                &view,
            )
        };
        assert!(
            matches!(
                at(0, Affinity::Upstream),
                Some(GapCursor::IsolatingBoundary { index: 0, .. })
            ),
            "(0, Upstream) faces the leading boundary"
        );
        assert!(
            at(0, Affinity::Downstream).is_none(),
            "(0, Downstream) keeps its unit node-selection expansion meaning"
        );
        assert!(
            matches!(
                at(1, Affinity::Downstream),
                Some(GapCursor::IsolatingBoundary { index: 1, .. })
            ),
            "(count, Downstream) faces the trailing boundary"
        );
        assert!(
            at(1, Affinity::Upstream).is_none(),
            "(count, Upstream) keeps its unit node-selection expansion meaning"
        );
    }

    #[test]
    fn test_1_leading_unit_image() {
        let (pd, _root) = image_first_doc();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let unit = isolating_boundary_at(&root_node, 0);
        assert!(
            unit.is_some(),
            "image as first root child should be a leading unit"
        );
        let unit = unit.unwrap();
        assert!(
            matches!(unit, ChildView::Leaf(ref l) if l.as_atom().is_some_and(|a| matches!(a, AtomLeaf::Image { .. })))
        );
    }

    #[test]
    fn test_1_leading_unit_none_when_para_first() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        assert!(
            isolating_boundary_at(&root_node, 0).is_none(),
            "paragraph as first child is not a unit"
        );
    }

    #[test]
    fn test_2_between_monolithic_at_two_folds() {
        let (pd, _root) = two_folds_doc();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        assert!(
            between_monolithic_at(&root_node, 1),
            "between two folds at index 1 should be a gap position"
        );
        assert!(
            !between_monolithic_at(&root_node, 0),
            "index 0 is never a gap (nothing before)"
        );
        assert!(
            !between_monolithic_at(&root_node, 2),
            "index 2 is fold then paragraph — fold monolithic but paragraph is not"
        );
    }

    #[test]
    fn test_3_as_gap_cursor_between_folds() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let pos = Position {
            node: root,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::collapsed(pos).resolve(&view).unwrap();
        let gc = as_gap_cursor(&sel);
        assert!(
            gc.is_some(),
            "collapsed selection between two folds should produce a gap cursor"
        );
        assert!(
            matches!(gc, Some(GapCursor::BetweenMonolithic { index: 1, .. })),
            "should be BetweenMonolithic at index 1"
        );
    }

    #[test]
    fn test_3_as_gap_cursor_none_for_non_collapsed() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let a = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let h = Position {
            node: root,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert!(
            as_gap_cursor(&sel).is_none(),
            "non-collapsed selection cannot be a gap cursor"
        );
    }

    #[test]
    fn test_3_as_gap_cursor_leading_unit() {
        let (pd, root) = image_first_doc();
        let view = DocView::new(&pd);
        let pos = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::collapsed(pos).resolve(&view).unwrap();
        let gc = as_gap_cursor(&sel);
        assert!(
            gc.is_some(),
            "upstream at offset 0 before image should produce a gap cursor"
        );
        assert!(matches!(
            gc,
            Some(GapCursor::IsolatingBoundary { index: 0, .. })
        ));
    }

    // §4.3 — affinity-invariant for between: Downstream also resolves BetweenMonolithic
    #[test]
    fn test_3_between_folds_downstream_affinity() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let pos = Position {
            node: root,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let sel = Selection::collapsed(pos).resolve(&view).unwrap();
        let gc = as_gap_cursor(&sel);
        assert!(
            gc.is_some(),
            "Downstream affinity between two folds should also produce BetweenMonolithic"
        );
        let mut reached_between = false;
        if let Some(GapCursor::BetweenMonolithic { index, .. }) = gc {
            assert_eq!(index, 1);
            reached_between = true;
        }
        assert!(reached_between, "expected BetweenMonolithic variant");
    }

    // §4.3 — leading is Upstream-only: Downstream at (root,0) over a leading unit → None
    #[test]
    fn test_3_leading_unit_downstream_is_none() {
        let (pd, root) = image_first_doc();
        let view = DocView::new(&pd);
        let pos = Position {
            node: root,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let sel = Selection::collapsed(pos).resolve(&view).unwrap();
        assert!(
            as_gap_cursor(&sel).is_none(),
            "(root,0,Downstream) over a leading unit must return None — leading gaps are Upstream-only"
        );
    }
}

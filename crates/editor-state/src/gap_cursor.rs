use editor_model::{ChildView, DocView, NodeType, NodeView, Schema};

use crate::affinity::Affinity;

use crate::{Position, classify, selection::ResolvedSelection};

pub enum GapCursor<'a> {
    LeadingUnit {
        unit: ChildView<'a>,
    },
    BetweenMonolithic {
        parent: NodeView<'a>,
        before: ChildView<'a>,
        after: ChildView<'a>,
        index: usize,
    },
}

pub fn leading_unit<'a>(view: &'a DocView<'a>) -> Option<ChildView<'a>> {
    let first = view.root()?.first_child()?;
    classify::child_is_unit(&first).then_some(first)
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
    let is_root = view.root().map(|r| r.id()) == Some(pos.node);
    if is_root && pos.offset == 0 && pos.affinity == Affinity::Upstream {
        return leading_unit(view).map(|unit| GapCursor::LeadingUnit { unit });
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
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog,
        NodeType, ProjectedDoc, SeqItem, SpanLog, StyleLog, project_document,
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
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
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
                },
            ),
            (
                fold1_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold1],
                },
            ),
            (
                fold1_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold1],
                },
            ),
            (
                fold2,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                },
            ),
            (
                fold2_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold2],
                },
            ),
            (
                fold2_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold2],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), root)
    }

    #[test]
    fn test_1_leading_unit_image() {
        let (pd, _root) = image_first_doc();
        let view = DocView::new(&pd);
        let unit = leading_unit(&view);
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
            },
        )];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        assert!(
            leading_unit(&view).is_none(),
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
        assert!(matches!(gc, Some(GapCursor::LeadingUnit { .. })));
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

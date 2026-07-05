use editor_model::{AtomLeaf, ChildView};

use crate::ResolvedPosition;

pub(crate) fn is_inline_position(rp: &ResolvedPosition) -> bool {
    let Some(node) = rp.view().node(rp.node()) else {
        return false;
    };
    let spec = node.spec();
    spec.inline || spec.is_textblock()
}

pub(crate) fn child_is_unit(child: &ChildView) -> bool {
    match child {
        ChildView::Block(b) => b.spec().is_unit(),
        ChildView::Leaf(l) => l.as_atom().is_some_and(AtomLeaf::is_block_level),
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
            node_markers: NodeMarkerLog::new(),
        }
    }

    fn make_doc() -> (ProjectedDoc, Dot, Dot) {
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
            (Dot::new(1, 3), SeqItem::Char('b')),
        ];
        (project_document(&logs(&items)).unwrap(), root, para)
    }

    #[test]
    fn is_inline_position_method() {
        let (pd, root, para) = make_doc();
        let view = DocView::new(&pd);

        let inline_rp = Position::new(para, 1).resolve(&view).unwrap();
        assert!(inline_rp.is_inline_position());
        assert_eq!(
            inline_rp.is_inline_position(),
            is_inline_position(&inline_rp)
        );

        let block_rp = Position::new(root, 0).resolve(&view).unwrap();
        assert!(!block_rp.is_inline_position());
        assert_eq!(block_rp.is_inline_position(), is_inline_position(&block_rp));
    }

    #[test]
    fn test_1_inline_and_block_position() {
        let (pd, root, para) = make_doc();
        let view = DocView::new(&pd);

        let para_pos = Position::new(para, 1).resolve(&view).unwrap();
        assert!(
            is_inline_position(&para_pos),
            "position in paragraph is inline"
        );

        let root_pos = Position::new(root, 0).resolve(&view).unwrap();
        assert!(
            !is_inline_position(&root_pos),
            "position in root is not inline"
        );
    }

    #[test]
    fn test_1_child_is_unit_atom() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let bq = Dot::new(1, 2);
        let bq_para = Dot::new(1, 3);
        let hr = Dot::new(1, 4);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('x')),
            (Dot::new(1, 6), SeqItem::Atom(AtomLeaf::HardBreak)),
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
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: editor_model::HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();

        let mut found_char_leaf = false;
        let mut found_inline_atom_leaf = false;
        let mut found_block_atom_leaf = false;
        let mut found_monolithic_block = false;

        for child in root_node.children() {
            match &child {
                ChildView::Block(b) => {
                    if b.node_type() == NodeType::Paragraph {
                        assert!(!child_is_unit(&child), "Paragraph block-child not a unit");
                        for c in b.children() {
                            if let ChildView::Leaf(l) = &c {
                                if l.as_char().is_some() {
                                    assert!(!child_is_unit(&c), "char leaf not a unit");
                                    found_char_leaf = true;
                                } else if matches!(l.as_atom(), Some(AtomLeaf::HardBreak)) {
                                    assert!(
                                        !child_is_unit(&c),
                                        "HardBreak (inline atom) not block-level unit"
                                    );
                                    found_inline_atom_leaf = true;
                                }
                            }
                        }
                    } else if b.node_type() == NodeType::Blockquote {
                        assert!(
                            child_is_unit(&child),
                            "Blockquote (monolithic block) is a unit"
                        );
                        found_monolithic_block = true;
                    }
                }
                ChildView::Leaf(l) => {
                    if matches!(l.as_atom(), Some(AtomLeaf::HorizontalRule { .. })) {
                        assert!(
                            child_is_unit(&child),
                            "HorizontalRule (block-level atom leaf) must be a unit"
                        );
                        found_block_atom_leaf = true;
                    }
                }
            }
        }

        assert!(found_char_leaf, "char leaf arm was never reached");
        assert!(
            found_inline_atom_leaf,
            "inline atom leaf arm was never reached"
        );
        assert!(
            found_block_atom_leaf,
            "block-level atom leaf arm was never reached"
        );
        assert!(
            found_monolithic_block,
            "monolithic block arm was never reached"
        );
    }
}

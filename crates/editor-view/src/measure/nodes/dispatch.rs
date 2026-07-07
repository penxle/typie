use std::sync::Arc;

use editor_common::EdgeInsets;
use editor_model::{ChildView, NodeType, NodeView};
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;

use super::atom::measure_atom;
use super::blockquote::measure_blockquote;
use super::callout::measure_callout;
use super::fold::{measure_fold, measure_fold_content, measure_fold_title};
use super::list_item::measure_list_item;
use super::paragraph::measure_paragraph_block;
use super::table::{measure_table, measure_table_cell};
use crate::measure::Measurer;
use crate::measure::container::layout_padded;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredContent, MeasuredNode};

pub(crate) fn measure_node(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    match node.node_type() {
        NodeType::Paragraph => measure_paragraph_block(measurer, node, width, ctx, resource),
        NodeType::Callout => measure_callout(measurer, node, width, ctx, resource),
        NodeType::Blockquote => measure_blockquote(measurer, node, width, ctx, resource),
        NodeType::ListItem => measure_list_item(measurer, node, width, ctx, resource),
        NodeType::Fold => measure_fold(measurer, node, width, ctx, resource),
        NodeType::FoldTitle => measure_fold_title(measurer, node, width, ctx, resource),
        NodeType::FoldContent => measure_fold_content(measurer, node, width, ctx, resource),
        NodeType::Table => measure_table(measurer, node, width, ctx, resource),
        NodeType::TableCell => measure_table_cell(measurer, node, width, ctx, resource),
        _ => {
            let mut seam = |child, w, ctx: &MeasureContext, r: &mut Resource| {
                measure_child(measurer, child, w, ctx, r)
            };
            layout_padded(
                node,
                width,
                ctx,
                resource,
                PaddedLayoutConfig {
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    alignment: crate::style::Alignment::Start,
                    page_break_policy: PageBreakPolicy::Auto,
                },
                &mut seam,
            )
        }
    }
}

pub(crate) fn measure_child(
    measurer: &mut Measurer,
    child: ChildView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> Arc<MeasuredNode> {
    match child {
        ChildView::Block(nv) => measurer.measure(&nv, width, ctx, resource),
        ChildView::Leaf(lv) => match lv.node_type() {
            NodeType::Image
            | NodeType::HorizontalRule
            | NodeType::File
            | NodeType::Embed
            | NodeType::Archived => Arc::new(measure_atom(&lv, width, ctx)),
            NodeType::PageBreak => Arc::new(MeasuredNode {
                width,
                height: 0.0,
                content: MeasuredContent::PageBreak,
            }),
            _ => Arc::new(MeasuredNode {
                width,
                height: 0.0,
                content: MeasuredContent::PageBreak,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, ModifierAttrLog, NodeAttrLog, NodeType,
        SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;

    use super::*;

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

    #[test]
    fn root_two_paragraphs() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('H')),
            (Dot::new(1, 4), SeqItem::Char('e')),
            (Dot::new(1, 5), SeqItem::Char('l')),
            (Dot::new(1, 6), SeqItem::Char('l')),
            (Dot::new(1, 7), SeqItem::Char('o')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 8), SeqItem::Char('W')),
            (Dot::new(1, 9), SeqItem::Char('o')),
            (Dot::new(1, 10), SeqItem::Char('r')),
            (Dot::new(1, 11), SeqItem::Char('l')),
            (Dot::new(1, 12), SeqItem::Char('d')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &root_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );

        assert!(result.height > 0.0);
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box at root");
        };
        assert!(b.children.len() >= 2);
        for child in b.children.iter() {
            let MeasuredContent::Box(ref cb) = child.content else {
                panic!("expected each paragraph child to be a Box");
            };
            assert!(!cb.children.is_empty());
            for line in cb.children.iter() {
                assert!(matches!(line.content, MeasuredContent::Line(_)));
            }
        }
    }

    #[test]
    fn block_atom_in_root() {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &root_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box at root");
        };
        assert_eq!(b.children.len(), 2);
        let hr_child = &b.children[0];
        assert!(
            matches!(&hr_child.content, MeasuredContent::Atom(_)),
            "HR child must be Atom"
        );
        assert_eq!(hr_child.height, 24.0);

        let root2 = Dot::ROOT;
        let img = Dot::new(2, 1);
        let p2 = Dot::new(2, 2);
        let img_node = match editor_model::NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items2 = vec![
            (
                img,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root2],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root2],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 3), SeqItem::Char('x')),
        ];
        let doc2 = logs(&items2);
        let pd2 = project_document(&doc2).unwrap();
        let view2 = DocView::new(&pd2);
        let root_node2 = view2.root().unwrap();

        let result2 = measure_node(
            &mut Measurer::new(),
            &root_node2,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b2) = result2.content else {
            panic!("expected Box at root2");
        };
        let img_child = &b2.children[0];
        assert!(
            matches!(&img_child.content, MeasuredContent::Atom(_)),
            "Image child must be Atom"
        );
        assert_eq!(img_child.height, 1.0);
    }

    #[test]
    fn default_container_recurses() {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let p_bq = Dot::new(1, 2);
        let p_root = Dot::new(1, 3);
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
                p_bq,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 4), SeqItem::Char('a')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('b')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &root_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box at root");
        };
        assert_eq!(b.children.len(), 2);

        let bq_child = &b.children[0];
        let MeasuredContent::Box(ref bq_box) = bq_child.content else {
            panic!("expected blockquote child to be a Box");
        };
        assert!(!bq_box.children.is_empty());
        let inner_para = &bq_box.children[0];
        assert!(
            matches!(&inner_para.content, MeasuredContent::Box(_)),
            "inner paragraph must be a Box"
        );
        if let MeasuredContent::Box(ref ib) = inner_para.content {
            assert!(
                ib.children
                    .iter()
                    .all(|c| matches!(c.content, MeasuredContent::Line(_)))
            );
        }

        let p_root_child = &b.children[1];
        assert!(matches!(&p_root_child.content, MeasuredContent::Box(_)));
    }

    #[test]
    fn trailing_page_break_end_to_end() {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let items = vec![
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('x')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::PageBreak)),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &root_node,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box at root");
        };
        let para_child = &b.children[0];
        let MeasuredContent::Box(ref pb) = para_child.content else {
            panic!("expected paragraph to be a Box");
        };
        let last = &pb.children[pb.children.len() - 1];
        assert!(
            matches!(last.content, MeasuredContent::PageBreak),
            "last child of paragraph must be PageBreak"
        );
    }
}

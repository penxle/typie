use std::sync::Arc;

use editor_common::EdgeInsets;
use editor_model::{
    Alignment, ChildView, DEFAULT_ALIGNMENT, DEFAULT_PARAGRAPH_INDENT, Modifier, ModifierType,
    NodeType, NodeView,
};
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::text::measure::measure_paragraph;
use crate::style::{BorderMode, BoxStyle, Direction};

use crate::measure::Measurer;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredBox, MeasuredChildren, MeasuredContent, MeasuredNode};

const DEFAULT_FONT_SIZE_PX: f32 = 16.0;

pub(crate) fn paragraph_indent(node: &NodeView) -> f32 {
    let parent_is_root = node
        .parent()
        .map(|p| p.node_type() == NodeType::Root)
        .unwrap_or(false);
    if !parent_is_root {
        return 0.0;
    }
    let value = match node.effective().get(&ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent { value }) => *value,
        _ => DEFAULT_PARAGRAPH_INDENT,
    };
    value as f32 / 100.0 * DEFAULT_FONT_SIZE_PX
}

pub(crate) fn align_to_layout(align: Alignment) -> crate::style::Alignment {
    match align {
        Alignment::Left | Alignment::Justify => crate::style::Alignment::Start,
        Alignment::Center => crate::style::Alignment::Center,
        Alignment::Right => crate::style::Alignment::End,
    }
}

pub(crate) fn measure_paragraph_block(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let align = match node.effective().get(&ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => *value,
        _ => DEFAULT_ALIGNMENT,
    };
    let indent = match align {
        Alignment::Left | Alignment::Justify => paragraph_indent(node),
        Alignment::Center | Alignment::Right => 0.0,
    };

    let pending = ctx.pending_for(&node.id());
    let (lines, total_height) = measure_paragraph(
        node,
        width,
        align,
        indent,
        pending,
        Some(&mut measurer.seg_cache),
        resource,
    );

    let mut children: Vec<Arc<MeasuredNode>> = lines
        .into_iter()
        .map(|l| Arc::new(MeasuredNode::from_line(width, l)))
        .collect();

    let has_trailing_page_break = node
        .last_child()
        .is_some_and(|c| matches!(c, ChildView::Leaf(lv) if lv.node_type() == NodeType::PageBreak));
    if has_trailing_page_break {
        children.push(Arc::new(MeasuredNode {
            width: 0.0,
            height: 0.0,
            content: MeasuredContent::PageBreak,
        }));
    }

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: align_to_layout(align),
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children: MeasuredChildren::from_blocks(children),
            page_break_policy: PageBreakPolicy::Auto,
            scope: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, Modifier, ModifierAttrLog,
        ModifierAttrOp::SetModifier, NodeAttrLog, NodeType, SeqItem, SpanLog, project_document,
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
            aliases: AliasLog::new(),
        }
    }

    fn build_paragraph(children: Vec<SeqItem>) -> DocLogs {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let mut items = vec![(
            p,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, c) in children.into_iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), c));
        }
        logs(&items)
    }

    fn get_para_node<'a>(view: &'a DocView<'a>) -> editor_model::NodeView<'a> {
        view.root().unwrap().child_blocks().next().unwrap()
    }

    #[test]
    fn wraps_lines_into_box() {
        let doc = build_paragraph(vec![
            SeqItem::Char('H'),
            SeqItem::Char('e'),
            SeqItem::Char('l'),
            SeqItem::Char('l'),
            SeqItem::Char('o'),
        ]);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let para = get_para_node(&view);
        let mut res = Resource::new_test();

        let result = measure_paragraph_block(
            &mut Measurer::new(),
            &para,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );

        assert!(result.height > 0.0);
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box");
        };
        assert!(!b.children.is_empty());
        for child in b.children.iter() {
            assert!(
                matches!(child.content, MeasuredContent::Line(_)),
                "expected all children to be Lines"
            );
            assert!(child.height > 0.0);
        }
    }

    #[test]
    fn alignment_from_effective() {
        let root = Dot::ROOT;
        let p_center = Dot::new(1, 1);
        let p_unset = Dot::new(1, 2);
        let items = vec![
            (
                p_center,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p_unset,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let mut doc = logs(&items);
        doc.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                SetModifier {
                    target: p_center,
                    modifier: Modifier::Alignment {
                        value: Alignment::Center,
                    },
                },
            )
            .unwrap();

        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut blocks = root_node.child_blocks();
        let center_para = blocks.next().unwrap();
        let unset_para = blocks.next().unwrap();
        let mut res = Resource::new_test();

        let r_center = measure_paragraph_block(
            &mut Measurer::new(),
            &center_para,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let r_unset = measure_paragraph_block(
            &mut Measurer::new(),
            &unset_para,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );

        let MeasuredContent::Box(ref bc) = r_center.content else {
            panic!("expected Box");
        };
        assert_eq!(bc.style.alignment, crate::style::Alignment::Center);

        let MeasuredContent::Box(ref bu) = r_unset.content else {
            panic!("expected Box");
        };
        assert_eq!(bu.style.alignment, crate::style::Alignment::Start);
    }

    #[test]
    fn trailing_page_break_marker() {
        let root = Dot::ROOT;
        let p_text_pb = Dot::new(1, 1);
        let p_pb_only = Dot::new(1, 2);
        let items = vec![
            (
                p_text_pb,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
            (Dot::new(1, 4), SeqItem::Atom(AtomLeaf::PageBreak)),
            (
                p_pb_only,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 5), SeqItem::Atom(AtomLeaf::PageBreak)),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut blocks = root_node.child_blocks();
        let para_text_pb = blocks.next().unwrap();
        let para_pb_only = blocks.next().unwrap();
        let mut res = Resource::new_test();

        let r1 = measure_paragraph_block(
            &mut Measurer::new(),
            &para_text_pb,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b1) = r1.content else {
            panic!("expected Box");
        };
        assert!(
            matches!(
                b1.children[b1.children.len() - 1].content,
                MeasuredContent::PageBreak
            ),
            "last child of text+page_break paragraph must be PageBreak"
        );

        let r2 = measure_paragraph_block(
            &mut Measurer::new(),
            &para_pb_only,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b2) = r2.content else {
            panic!("expected Box");
        };
        assert!(
            b2.children.len() >= 2,
            "page_break-only paragraph must have at least 2 children (strut Line + PageBreak)"
        );
        assert!(
            matches!(
                b2.children[b2.children.len() - 1].content,
                MeasuredContent::PageBreak
            ),
            "last child must be PageBreak"
        );
        assert!(
            matches!(b2.children[0].content, MeasuredContent::Line(_)),
            "first child must be a strut Line"
        );
    }

    #[test]
    fn indent_only_when_parent_root() {
        let root = Dot::ROOT;
        let p_root_child = Dot::new(1, 1);
        let char_a1 = Dot::new(1, 2);
        let bq = Dot::new(1, 3);
        let p_bq_child = Dot::new(1, 4);
        let char_a2 = Dot::new(1, 5);
        let items = vec![
            (
                p_root_child,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (char_a1, SeqItem::Char('A')),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p_bq_child,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            ),
            (char_a2, SeqItem::Char('A')),
        ];
        let mut doc = logs(&items);
        doc.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                SetModifier {
                    target: root,
                    modifier: Modifier::ParagraphIndent { value: 200 },
                },
            )
            .unwrap();

        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut root_blocks = root_node.child_blocks();
        let para_root = root_blocks.next().unwrap();
        let bq_node = root_blocks.next().unwrap();
        let para_bq = bq_node.child_blocks().next().unwrap();

        let indent_root = paragraph_indent(&para_root);
        let indent_bq = paragraph_indent(&para_bq);

        assert!(indent_root > 0.0, "root-child paragraph must have indent");
        assert_eq!(
            indent_bq, 0.0,
            "blockquote-child paragraph must have no indent"
        );
    }

    fn first_line_height(result: &MeasuredNode) -> f32 {
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box");
        };
        b.children
            .iter()
            .find(|c| matches!(c.content, MeasuredContent::Line(_)))
            .map(|c| c.height)
            .expect("at least one Line child")
    }

    #[test]
    fn empty_paragraph_pending_font_size_grows_strut_via_block() {
        let doc = build_paragraph(vec![]);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let para = get_para_node(&view);
        let para_id = para.id();
        let mut res = Resource::new_test();

        let r_base = measure_paragraph_block(
            &mut Measurer::new(),
            &para,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let h0 = first_line_height(&r_base);

        let big: editor_state::PendingModifiers = vec![editor_state::PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let ctx_pending = MeasureContext {
            pending_overlay: Some((para_id, big)),
            ..Default::default()
        };
        let r_pending =
            measure_paragraph_block(&mut Measurer::new(), &para, 300.0, &ctx_pending, &mut res);
        let h1 = first_line_height(&r_pending);

        assert!(
            h1 > h0,
            "strut must grow with bigger pending font-size (h0={h0}, h1={h1})"
        );
    }

    #[test]
    fn non_empty_paragraph_pending_font_size_gate_unchanged_via_block() {
        let doc = build_paragraph(vec![SeqItem::Char('x')]);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let para = get_para_node(&view);
        let para_id = para.id();
        let mut res = Resource::new_test();

        let r_base = measure_paragraph_block(
            &mut Measurer::new(),
            &para,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let h0 = first_line_height(&r_base);

        let big: editor_state::PendingModifiers = vec![editor_state::PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let ctx_pending = MeasureContext {
            pending_overlay: Some((para_id, big)),
            ..Default::default()
        };
        let r_pending =
            measure_paragraph_block(&mut Measurer::new(), &para, 300.0, &ctx_pending, &mut res);
        let h1 = first_line_height(&r_pending);

        assert!(
            (h1 - h0).abs() < 0.01,
            "pending must not apply when paragraph has Char leaves (h0={h0}, h1={h1})"
        );
    }
}

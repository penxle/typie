use editor_common::{EdgeInsets, Rect};
use editor_model::{BlockquoteVariant, Node, NodeView};
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;
use crate::style::{Alignment, BorderMode, BoxStyle, Decoration, DecorationData, Direction};

use super::dispatch::measure_child;
use super::line_geometry::first_line_info;
use crate::measure::Measurer;
use crate::measure::container::{layout_padded, layout_vertical};
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredBox, MeasuredContent, MeasuredNode};

const BQ_LINE_WIDTH: f32 = 4.0;
const BQ_CONTENT_PADDING: f32 = 16.0;
const BQ_QUOTE_SIZE: f32 = 16.0;
const BQ_QUOTE_CONTENT_GAP: f32 = 16.0;
const BQ_MESSAGE_PADDING_X: f32 = 14.0;
const BQ_MESSAGE_PADDING_Y: f32 = 8.0;
const BQ_MESSAGE_MAX_WIDTH_RATIO: f32 = 0.8;
const BQ_MESSAGE_MIN_WIDTH: f32 = 40.0;
const BQ_MESSAGE_LAYOUT_GUARD_WIDTH: f32 = 1.0;

pub(crate) fn measure_blockquote(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let Node::Blockquote(bq) = node.node() else {
        unreachable!()
    };

    let mut seam = |child, w, ctx: &MeasureContext, r: &mut Resource| {
        measure_child(measurer, child, w, ctx, r)
    };

    match *bq.variant.get() {
        BlockquoteVariant::LeftLine => {
            let padding = EdgeInsets {
                left: BQ_CONTENT_PADDING,
                ..EdgeInsets::ZERO
            };
            layout_padded(
                node,
                width,
                ctx,
                resource,
                PaddedLayoutConfig {
                    padding,
                    border: EdgeInsets {
                        left: BQ_LINE_WIDTH,
                        ..EdgeInsets::ZERO
                    },
                    alignment: Alignment::Start,
                    page_break_policy: PageBreakPolicy::Auto,
                },
                &mut seam,
            )
        }
        BlockquoteVariant::LeftQuote => {
            let padding = EdgeInsets {
                left: BQ_QUOTE_SIZE + BQ_QUOTE_CONTENT_GAP,
                ..EdgeInsets::ZERO
            };
            let mut measured = layout_padded(
                node,
                width,
                ctx,
                resource,
                PaddedLayoutConfig {
                    padding,
                    border: EdgeInsets::ZERO,
                    alignment: Alignment::Start,
                    page_break_policy: PageBreakPolicy::Auto,
                },
                &mut seam,
            );
            let icon_y = first_line_info(&measured)
                .map(|info| info.top + (info.height - BQ_QUOTE_SIZE) / 2.0)
                .unwrap_or(0.0);

            if let MeasuredContent::Box(ref mut b) = measured.content {
                b.style.decorations.push(Decoration {
                    id: 0,
                    rect: Rect {
                        x: 0.0,
                        y: icon_y,
                        width: BQ_QUOTE_SIZE,
                        height: BQ_QUOTE_SIZE,
                    },
                    data: DecorationData::None,
                });
            }
            measured
        }
        BlockquoteVariant::MessageSent | BlockquoteVariant::MessageReceived => {
            let inner_max_width =
                (width * BQ_MESSAGE_MAX_WIDTH_RATIO - BQ_MESSAGE_PADDING_X * 2.0).max(0.0);
            let min_inner_width = BQ_MESSAGE_MIN_WIDTH - BQ_MESSAGE_PADDING_X * 2.0;

            let (children_pass1, height_pass1) =
                layout_vertical(node, inner_max_width, ctx, resource, &mut seam);

            let intrinsic = children_pass1
                .iter()
                .map(|c| measured_intrinsic_width(c.as_ref()))
                .fold(min_inner_width, f32::max);

            let final_inner_width = (intrinsic + BQ_MESSAGE_LAYOUT_GUARD_WIDTH)
                .min(inner_max_width)
                .max(min_inner_width);

            let (children, total_height) = if final_inner_width >= inner_max_width {
                (children_pass1, height_pass1)
            } else {
                layout_vertical(node, final_inner_width, ctx, resource, &mut seam)
            };

            let bubble_width = (final_inner_width + BQ_MESSAGE_PADDING_X * 2.0).min(width);
            let bubble_height = total_height + BQ_MESSAGE_PADDING_Y * 2.0;
            let padding = EdgeInsets::symmetric(BQ_MESSAGE_PADDING_X, BQ_MESSAGE_PADDING_Y);
            let alignment = if *bq.variant.get() == BlockquoteVariant::MessageSent {
                Alignment::End
            } else {
                Alignment::Start
            };

            MeasuredNode {
                width: bubble_width,
                height: bubble_height,
                content: MeasuredContent::Box(MeasuredBox {
                    node: node.id(),
                    style: BoxStyle {
                        direction: Direction::Vertical,
                        padding,
                        border: EdgeInsets::ZERO,
                        border_mode: BorderMode::Separate,
                        alignment,
                        decorations: vec![],
                        monolithic: node.spec().monolithic,
                    },
                    children,
                    page_break_policy: PageBreakPolicy::Auto,
                }),
            }
        }
    }
}

pub(crate) fn measured_intrinsic_width(node: &MeasuredNode) -> f32 {
    match &node.content {
        MeasuredContent::Line(l) => l.glyph_runs.iter().map(|r| r.width).sum(),
        MeasuredContent::Box(b) => {
            let children_max = b
                .children
                .iter()
                .map(|c| measured_intrinsic_width(c.as_ref()))
                .fold(0.0_f32, f32::max);
            children_max
                + b.style.padding.left
                + b.style.padding.right
                + b.style.border.left
                + b.style.border.right
        }
        MeasuredContent::Atom(_) => node.width,
        MeasuredContent::Spacing(_) | MeasuredContent::PageBreak => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        BlockquoteNodeAttr, BlockquoteVariant, DocLogs, DocView, ModifierAttrLog, NodeAttr,
        NodeAttrLog, NodeAttrOp, NodeMarkerLog, NodeStyleLog, NodeType, SeqItem, SpanLog, StyleLog,
        project_document,
    };
    use editor_resource::Resource;

    use crate::glyph_run::GlyphRun;
    use crate::measure::Measurer;
    use crate::measure::text::measure::MeasuredLine;
    use crate::style::{Alignment, DecorationData};

    use crate::measure::context::MeasureContext;

    use super::super::dispatch::measure_node;
    use super::measured_intrinsic_width;
    use crate::measure::types::{
        MeasuredAtom, MeasuredBox, MeasuredChildren, MeasuredContent, MeasuredNode,
    };

    fn logs_of(items: &[(Dot, SeqItem)]) -> DocLogs {
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

    fn build_blockquote_doc(variant: Option<BlockquoteVariant>) -> DocLogs {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let p_bq = Dot::new(1, 2);
        let p_root = Dot::new(1, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                p_bq,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let mut logs = logs_of(&items);
        if let Some(v) = variant {
            logs.node_attrs = NodeAttrLog::new()
                .apply(
                    Dot::ROOT,
                    NodeAttrOp {
                        target: bq,
                        attr: NodeAttr::Blockquote {
                            attr: BlockquoteNodeAttr::Variant(v),
                        },
                    },
                )
                .unwrap();
        }
        logs
    }

    fn build_message_doc(variant: BlockquoteVariant) -> DocLogs {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let p_bq = Dot::new(1, 2);
        let p_root = Dot::new(1, 4);
        let items = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                p_bq,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('h')),
            (Dot::new(1, 5), SeqItem::Char('i')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let mut logs = logs_of(&items);
        logs.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::ROOT,
                NodeAttrOp {
                    target: bq,
                    attr: NodeAttr::Blockquote {
                        attr: BlockquoteNodeAttr::Variant(variant),
                    },
                },
            )
            .unwrap();
        logs
    }

    #[test]
    fn left_line_border() {
        let doc = build_blockquote_doc(None);
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
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected Box at root");
        };

        let bq_child = &root_box.children[0];
        let MeasuredContent::Box(ref cb) = bq_child.content else {
            panic!("expected blockquote to be a Box");
        };

        assert_eq!(cb.style.border.left, 4.0);
        assert_eq!(cb.style.padding.left, 16.0);
        assert!(cb.style.decorations.is_empty());
    }

    #[test]
    fn left_quote_icon() {
        let doc = build_blockquote_doc(Some(BlockquoteVariant::LeftQuote));
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
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected Box at root");
        };

        let bq_child = &root_box.children[0];
        let MeasuredContent::Box(ref cb) = bq_child.content else {
            panic!("expected blockquote to be a Box");
        };

        assert_eq!(cb.style.padding.left, 32.0);
        assert_eq!(cb.style.decorations.len(), 1);

        let dec = &cb.style.decorations[0];
        assert_eq!(dec.rect.width, 16.0);
        assert_eq!(dec.rect.x, 0.0);
        assert!(matches!(dec.data, DecorationData::None));
    }

    #[test]
    fn message_bubble_alignment_and_width() {
        let sent_doc = build_message_doc(BlockquoteVariant::MessageSent);
        let pd_sent = project_document(&sent_doc).unwrap();
        let sent = DocView::new(&pd_sent);
        let root_sent = sent.root().unwrap();
        let mut res = Resource::new_test();

        let result_sent = measure_node(
            &mut Measurer::new(),
            &root_sent,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref root_box_sent) = result_sent.content else {
            panic!("expected Box at root");
        };

        let bq_sent = &root_box_sent.children[0];
        let MeasuredContent::Box(ref cb_sent) = bq_sent.content else {
            panic!("expected blockquote to be a Box");
        };

        assert_eq!(cb_sent.style.alignment, Alignment::End);
        assert_eq!(cb_sent.style.padding.left, 14.0);
        assert_eq!(cb_sent.style.padding.top, 8.0);
        assert!(bq_sent.width < 400.0 * 0.8);
        assert!(bq_sent.width < 150.0);

        let recv_doc = build_message_doc(BlockquoteVariant::MessageReceived);
        let pd_recv = project_document(&recv_doc).unwrap();
        let recv = DocView::new(&pd_recv);
        let root_recv = recv.root().unwrap();

        let result_recv = measure_node(
            &mut Measurer::new(),
            &root_recv,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref root_box_recv) = result_recv.content else {
            panic!("expected Box at root");
        };

        let bq_recv = &root_box_recv.children[0];
        let MeasuredContent::Box(ref cb_recv) = bq_recv.content else {
            panic!("expected blockquote to be a Box");
        };

        assert_eq!(cb_recv.style.alignment, Alignment::Start);
        assert!(bq_recv.width < 150.0);
    }

    #[test]
    fn intrinsic_width_helper() {
        let node_id = Dot::new(9, 1);

        let glyph_run = GlyphRun {
            family_id: Default::default(),
            weight: 400,
            font_size: 16.0,
            synthesis: Default::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            decoration: Default::default(),
            offset_range: 0..1,
            link: None,
            text: String::from("x"),
            x: 0.0,
            width: 50.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };

        let line = MeasuredLine {
            node: node_id,
            height: 20.0,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![glyph_run],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            offset_range: Some(0..1),
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
        };

        let line_node = Arc::new(MeasuredNode::from_line(400.0, line));

        let box_node = MeasuredNode {
            width: 400.0,
            height: 40.0,
            content: MeasuredContent::Box(MeasuredBox {
                node: node_id,
                style: crate::style::BoxStyle {
                    direction: crate::style::Direction::Vertical,
                    padding: EdgeInsets {
                        left: 5.0,
                        right: 5.0,
                        top: 0.0,
                        bottom: 0.0,
                    },
                    border: EdgeInsets::ZERO,
                    border_mode: crate::style::BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children: MeasuredChildren::from_blocks(vec![line_node]),
                page_break_policy: crate::measure::PageBreakPolicy::Auto,
            }),
        };

        assert_eq!(measured_intrinsic_width(&box_node), 60.0);

        let atom_node = MeasuredNode {
            width: 30.0,
            height: 10.0,
            content: MeasuredContent::Atom(MeasuredAtom { node: node_id }),
        };
        assert_eq!(measured_intrinsic_width(&atom_node), 30.0);

        let spacing_node = MeasuredNode {
            width: 0.0,
            height: 5.0,
            content: MeasuredContent::Spacing(5.0),
        };
        assert_eq!(measured_intrinsic_width(&spacing_node), 0.0);
    }
}

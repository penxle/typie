use std::sync::Arc;

use editor_common::{EdgeInsets, Rect};
use editor_model::{Alignment, ChildView, NodeType, NodeView};
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;
use crate::measure::text::measure::measure_paragraph;
use crate::style::{BorderMode, BoxStyle, Decoration, DecorationData, Direction};

use super::dispatch::measure_child;
use super::line_geometry::first_line_info;
use crate::measure::Measurer;
use crate::measure::container::layout_padded;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredBox, MeasuredChildren, MeasuredContent, MeasuredNode};

const FOLD_TITLE_PADDING_X: f32 = 12.0;
const FOLD_TITLE_PADDING_Y: f32 = 8.0;
const FOLD_TITLE_ICON_WIDTH: f32 = 20.0;
const FOLD_TITLE_ICON_GAP: f32 = 8.0;
const FOLD_CONTENT_PADDING_X: f32 = 24.0;
const FOLD_CONTENT_PADDING_Y: f32 = 16.0;
const FOLD_BORDER_WIDTH: f32 = 1.0;

pub(crate) fn measure_fold_title(
    _measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let expanded = node
        .parent()
        .map(|p| ctx.fold_expanded(&p.id()))
        .unwrap_or(true);
    let padding = EdgeInsets {
        top: FOLD_TITLE_PADDING_Y,
        left: FOLD_TITLE_PADDING_X + FOLD_TITLE_ICON_WIDTH + FOLD_TITLE_ICON_GAP,
        bottom: FOLD_TITLE_PADDING_Y,
        right: FOLD_TITLE_PADDING_X,
    };
    let inner_width = width - padding.left - padding.right;

    let (lines, children_height) = measure_paragraph(
        node,
        inner_width,
        Alignment::Left,
        0.0,
        ctx.pending_for(&node.id()),
        None,
        resource,
    );
    let children: Vec<Arc<MeasuredNode>> = lines
        .into_iter()
        .map(|l| Arc::new(MeasuredNode::from_line(inner_width, l)))
        .collect();

    let mut measured = MeasuredNode {
        width,
        height: children_height + padding.top + padding.bottom,
        content: MeasuredContent::Box(MeasuredBox {
            node: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: crate::style::Alignment::Start,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children: MeasuredChildren::from_blocks(children),
            page_break_policy: PageBreakPolicy::Avoid,
        }),
    };

    let icon_y = first_line_info(&measured)
        .map(|info| info.top + (info.height - FOLD_TITLE_ICON_WIDTH) / 2.0)
        .unwrap_or(FOLD_TITLE_PADDING_Y);

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: FOLD_TITLE_PADDING_X,
                y: icon_y,
                width: FOLD_TITLE_ICON_WIDTH,
                height: FOLD_TITLE_ICON_WIDTH,
            },
            data: DecorationData::Bool(expanded),
        });
    }

    measured
}

pub(crate) fn measure_fold_content(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let padding = EdgeInsets::symmetric(FOLD_CONTENT_PADDING_X, FOLD_CONTENT_PADDING_Y);
    let mut seam = |child, w, ctx: &MeasureContext, r: &mut Resource| {
        measure_child(measurer, child, w, ctx, r)
    };
    layout_padded(
        node,
        width,
        ctx,
        resource,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            alignment: crate::style::Alignment::Start,
            page_break_policy: PageBreakPolicy::Auto,
        },
        &mut seam,
    )
}

pub(crate) fn measure_fold(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let expanded = ctx.fold_expanded(&node.id());
    let border = EdgeInsets::all(FOLD_BORDER_WIDTH);
    let content_width = width - border.left - border.right;

    let mut children: Vec<Arc<MeasuredNode>> = Vec::new();
    let mut children_height = 0.0f32;

    for child in node.children() {
        if !expanded
            && let ChildView::Block(b) = &child
            && b.node_type() == NodeType::FoldContent
        {
            continue;
        }
        let m = measure_child(measurer, child, content_width, ctx, resource);
        children_height += m.height;
        children.push(m);
    }

    let height = border.top + children_height + border.bottom;

    MeasuredNode {
        width,
        height,
        content: MeasuredContent::Box(MeasuredBox {
            node: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border,
                border_mode: BorderMode::Separate,
                alignment: crate::style::Alignment::Start,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children: MeasuredChildren::from_blocks(children),
            page_break_policy: PageBreakPolicy::Auto,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, Alignment, DocLogs, DocView, Modifier, ModifierAttrLog, NodeAttrLog, NodeType,
        SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;
    use editor_state::PendingModifier;
    use hashbrown::HashMap;

    use crate::measure::context::MeasureContext;
    use crate::measure::text::measure::measure_paragraph;
    use crate::style::DecorationData;

    use super::*;
    use crate::measure::types::MeasuredContent;

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

    fn fold_title_doc() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(1, 1);
        let ft = Dot::new(1, 2);
        let para_root = Dot::new(1, 10);
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
                ft,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('T')),
            (Dot::new(1, 4), SeqItem::Char('i')),
            (Dot::new(1, 5), SeqItem::Char('t')),
            (Dot::new(1, 6), SeqItem::Char('l')),
            (Dot::new(1, 7), SeqItem::Char('e')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (logs(&items), ft)
    }

    fn fold_content_doc() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(2, 1);
        let ft = Dot::new(2, 2);
        let fc = Dot::new(2, 3);
        let para_inner = Dot::new(2, 4);
        let para_root = Dot::new(2, 10);
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
                ft,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 9), SeqItem::Char('T')),
            (
                fc,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                para_inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, fc],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 5), SeqItem::Char('C')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (logs(&items), fc)
    }

    fn get_node<'a>(view: &'a DocView<'a>, dot: Dot) -> NodeView<'a> {
        view.node(dot).expect("node")
    }

    #[test]
    fn fold_title_inline_and_chevron() {
        let (doc, ft_dot) = fold_title_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let title = get_node(&view, ft_dot);
        let mut res = Resource::new_test();

        let measured = measure_fold_title(
            &mut Measurer::new(),
            &title,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = measured.content else {
            panic!("expected Box");
        };
        assert_eq!(b.style.padding.left, 40.0);
        assert_eq!(b.style.decorations.len(), 1);
        let dec = &b.style.decorations[0];
        assert_eq!(dec.rect.width, 20.0);
        assert_eq!(dec.rect.x, 12.0);
        assert!(matches!(dec.data, DecorationData::Bool(true)));
        assert!(
            b.children
                .iter()
                .all(|c| matches!(c.content, MeasuredContent::Line(_)))
        );

        let ctx_false = MeasureContext {
            fold_states: HashMap::from([(editor_crdt::Dot::new(1, 1), false)]),
            ..Default::default()
        };
        let measured_false =
            measure_fold_title(&mut Measurer::new(), &title, 300.0, &ctx_false, &mut res);
        let MeasuredContent::Box(ref bf) = measured_false.content else {
            panic!("expected Box");
        };
        let dec_f = &bf.style.decorations[0];
        assert!(matches!(dec_f.data, DecorationData::Bool(false)));
    }

    #[test]
    fn fold_title_height() {
        let (doc, ft_dot) = fold_title_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let title = get_node(&view, ft_dot);
        let mut res = Resource::new_test();

        let inner_width = 300.0 - 40.0 - 12.0;
        let (_, inline_h) = measure_paragraph(
            &title,
            inner_width,
            Alignment::Left,
            0.0,
            None,
            None,
            &mut res,
        );
        let measured = measure_fold_title(
            &mut Measurer::new(),
            &title,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        assert!(
            (measured.height - (inline_h + 8.0 * 2.0)).abs() < 0.01,
            "height mismatch: {} vs {}",
            measured.height,
            inline_h + 8.0 * 2.0
        );
    }

    #[test]
    fn fold_content_padding() {
        let (doc, fc_dot) = fold_content_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let content = get_node(&view, fc_dot);
        let mut res = Resource::new_test();

        let measured = measure_fold_content(
            &mut Measurer::new(),
            &content,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = measured.content else {
            panic!("expected Box");
        };
        assert_eq!(b.style.padding.left, 24.0);
        assert_eq!(b.style.padding.top, 16.0);
        assert!(b.style.decorations.is_empty());
        assert!(!b.children.is_empty());
        assert!(matches!(b.children[0].content, MeasuredContent::Box(_)));
    }

    fn fold_doc() -> (DocLogs, Dot, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(3, 1);
        let ft = Dot::new(3, 2);
        let fc = Dot::new(3, 3);
        let para_inner = Dot::new(3, 4);
        let para_root = Dot::new(3, 10);
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
                ft,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (Dot::new(3, 8), SeqItem::Char('T')),
            (
                fc,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                para_inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, fc],
                    attrs: vec![],
                },
            ),
            (Dot::new(3, 5), SeqItem::Char('C')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (logs(&items), root, fold)
    }

    #[test]
    fn fold_border_and_children() {
        use crate::measure::nodes::dispatch::measure_node;
        let (doc, root_dot, _) = fold_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = get_node(&view, root_dot);
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &root,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref rb) = result.content else {
            panic!("expected Box at root");
        };
        let fold_child = &rb.children[0];
        let MeasuredContent::Box(ref fb) = fold_child.content else {
            panic!("expected Box at fold");
        };
        assert_eq!(fb.style.border.top, 1.0);
        assert_eq!(fb.style.border.left, 1.0);
        assert_eq!(fb.children.len(), 2);
    }

    #[test]
    fn collapse_skips_content() {
        let (doc, _, fold_dot) = fold_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let fold = get_node(&view, fold_dot);
        let mut res = Resource::new_test();

        let ctx_collapsed = MeasureContext {
            fold_states: HashMap::from([(fold_dot, false)]),
            ..Default::default()
        };
        let collapsed = measure_fold(&mut Measurer::new(), &fold, 300.0, &ctx_collapsed, &mut res);
        let MeasuredContent::Box(ref cb) = collapsed.content else {
            panic!("expected Box");
        };
        assert_eq!(cb.children.len(), 1);

        let expanded = measure_fold(
            &mut Measurer::new(),
            &fold,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref eb) = expanded.content else {
            panic!("expected Box");
        };
        assert_eq!(eb.children.len(), 2);
    }

    #[test]
    fn dispatch_wires_fold() {
        use crate::measure::nodes::dispatch::measure_node;
        let (doc, _, fold_dot) = fold_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let fold = get_node(&view, fold_dot);
        let mut res = Resource::new_test();

        let result = measure_node(
            &mut Measurer::new(),
            &fold,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box");
        };
        assert_eq!(b.style.border.top, 1.0);
    }

    #[test]
    fn fold_title_chevron_reflects_parent_state() {
        let (doc, ft_dot) = fold_title_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let title = get_node(&view, ft_dot);
        let fold_id = Dot::new(1, 1);
        let mut res = Resource::new_test();

        let ctx_collapsed = MeasureContext {
            fold_states: HashMap::from([(fold_id, false)]),
            ..Default::default()
        };
        let measured = measure_fold_title(
            &mut Measurer::new(),
            &title,
            300.0,
            &ctx_collapsed,
            &mut res,
        );
        let MeasuredContent::Box(ref b) = measured.content else {
            panic!("expected Box");
        };
        let dec = &b.style.decorations[0];
        assert!(matches!(dec.data, DecorationData::Bool(false)));

        let measured_default = measure_fold_title(
            &mut Measurer::new(),
            &title,
            300.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref bd) = measured_default.content else {
            panic!("expected Box");
        };
        let dec_d = &bd.style.decorations[0];
        assert!(matches!(dec_d.data, DecorationData::Bool(true)));
    }

    fn empty_fold_title_doc() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(5, 1);
        let ft = Dot::new(5, 2);
        let para_root = Dot::new(5, 10);
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
                ft,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (logs(&items), ft)
    }

    #[test]
    fn empty_fold_title_pending_font_size_grows_height() {
        let (doc, ft_dot) = empty_fold_title_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let title = get_node(&view, ft_dot);
        let title_id = ft_dot;
        let mut res = Resource::new_test();

        let h_none = measure_fold_title(
            &mut Measurer::new(),
            &title,
            300.0,
            &MeasureContext::default(),
            &mut res,
        )
        .height;

        let big: editor_state::PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let ctx_pending = MeasureContext {
            pending_overlay: Some((title_id, big)),
            ..Default::default()
        };
        let h_pending =
            measure_fold_title(&mut Measurer::new(), &title, 300.0, &ctx_pending, &mut res).height;

        assert!(
            h_pending > h_none,
            "pending font-size must grow empty fold title height (h_none={h_none}, h_pending={h_pending})"
        );
    }

    #[test]
    fn fold_collapse_threads_through_dispatch() {
        use crate::measure::nodes::dispatch::measure_node;
        let (doc, root_dot, fold_dot) = fold_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = get_node(&view, root_dot);
        let fold_id = fold_dot;
        let mut res = Resource::new_test();

        let ctx_collapsed = MeasureContext {
            fold_states: HashMap::from([(fold_id, false)]),
            ..Default::default()
        };
        let result_collapsed =
            measure_node(&mut Measurer::new(), &root, 400.0, &ctx_collapsed, &mut res);
        let MeasuredContent::Box(ref rb) = result_collapsed.content else {
            panic!("expected Box at root");
        };
        let fold_child = &rb.children[0];
        let MeasuredContent::Box(ref fb) = fold_child.content else {
            panic!("expected Box at fold");
        };
        assert_eq!(fb.children.len(), 1);

        let result_expanded = measure_node(
            &mut Measurer::new(),
            &root,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref rb2) = result_expanded.content else {
            panic!("expected Box at root");
        };
        let fold_child2 = &rb2.children[0];
        let MeasuredContent::Box(ref fb2) = fold_child2.content else {
            panic!("expected Box at fold");
        };
        assert_eq!(fb2.children.len(), 2);
    }
}

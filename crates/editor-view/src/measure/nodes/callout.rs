use editor_common::{EdgeInsets, Rect};
use editor_model::NodeView;
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;
use crate::style::{Alignment, Decoration, DecorationData};

use super::dispatch::measure_child;
use super::line_geometry::first_line_info;
use crate::measure::Measurer;
use crate::measure::container::layout_padded;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredContent, MeasuredNode};

const CALLOUT_PADDING_X: f32 = 12.0;
const CALLOUT_PADDING_Y: f32 = 16.0;
const CALLOUT_ICON_WIDTH: f32 = 20.0;
const CALLOUT_ICON_CONTENT_GAP: f32 = 8.0;

pub(crate) fn measure_callout(
    measurer: &mut Measurer,
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let padding = EdgeInsets {
        top: CALLOUT_PADDING_Y,
        left: CALLOUT_PADDING_X + CALLOUT_ICON_WIDTH + CALLOUT_ICON_CONTENT_GAP,
        bottom: CALLOUT_PADDING_Y,
        right: CALLOUT_PADDING_X,
    };

    let mut seam = |child, w, ctx: &MeasureContext, r: &mut Resource| {
        measure_child(measurer, child, w, ctx, r)
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
        .map(|info| info.top + (info.height - CALLOUT_ICON_WIDTH) / 2.0)
        .unwrap_or(CALLOUT_PADDING_Y);

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: CALLOUT_PADDING_X,
                y: icon_y,
                width: CALLOUT_ICON_WIDTH,
                height: CALLOUT_ICON_WIDTH,
            },
            data: DecorationData::None,
        });
    }

    measured
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, SeqItem, SpanLog,
        project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;

    use super::super::dispatch::measure_node;
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

    #[test]
    fn callout_padding_and_icon() {
        let root = Dot::ROOT;
        let callout = Dot::new(1, 1);
        let para_inner = Dot::new(1, 2);
        let para_root = Dot::new(1, 4);
        let items = vec![
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                para_inner,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
            (
                para_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
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
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected Box at root");
        };

        let callout_child = &root_box.children[0];
        let MeasuredContent::Box(ref cb) = callout_child.content else {
            panic!("expected callout to be a Box");
        };

        assert_eq!(cb.style.padding.left, 40.0);
        assert_eq!(cb.style.decorations.len(), 1);

        let dec = &cb.style.decorations[0];
        assert_eq!(dec.rect.width, 20.0);
        assert_eq!(dec.rect.x, 12.0);
        assert!(matches!(dec.data, DecorationData::None));

        assert!(!cb.children.is_empty());
        assert!(matches!(cb.children[0].content, MeasuredContent::Box(_)));
    }
}

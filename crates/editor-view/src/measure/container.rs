use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use editor_model::{
    Alignment, ChildView, DEFAULT_BLOCK_GAP, DEFAULT_PARAGRAPH_INDENT, Modifier, ModifierType,
    NodeType, NodeView,
};
use editor_resource::Resource;

use crate::measure::PageBreakPolicy;
use crate::measure::text::measure::build_strut_only_line;
use crate::measure::text::resolve::style_from_effective_modifiers;
use crate::style::{BorderMode, BoxStyle, Direction};
use editor_common::EdgeInsets;

use crate::measure::context::MeasureContext;

pub struct PaddedLayoutConfig {
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub alignment: crate::style::Alignment,
    pub page_break_policy: PageBreakPolicy,
}
use crate::measure::types::{MeasuredBox, MeasuredChildren, MeasuredContent, MeasuredNode};

const BLOCK_GAP_BASE_PX: f32 = 16.0;
const PHANTOM_INDENT_BASE_PX: f32 = 16.0;

pub(crate) fn resolve_gap_after(effective: &BTreeMap<ModifierType, Modifier>) -> f32 {
    let value = match effective.get(&ModifierType::BlockGap) {
        Some(Modifier::BlockGap { value }) => *value,
        _ => DEFAULT_BLOCK_GAP,
    };
    value as f32 / 100.0 * BLOCK_GAP_BASE_PX
}

static EMPTY_EFF: LazyLock<BTreeMap<ModifierType, Modifier>> = LazyLock::new(BTreeMap::new);

pub(crate) fn child_effective<'a>(
    node: &NodeView<'a>,
    slot: usize,
    child: &ChildView<'a>,
) -> &'a BTreeMap<ModifierType, Modifier> {
    match child {
        ChildView::Block(nv) => nv.effective(),
        ChildView::Leaf(_) => node
            .leaf_state_at(slot)
            .map(|s| s.eff)
            .unwrap_or(&EMPTY_EFF),
    }
}

fn phantom_indent(node: &NodeView) -> f32 {
    if node.node_type() != NodeType::Root {
        return 0.0;
    }
    let value = match node.effective().get(&ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent { value }) => *value,
        _ => DEFAULT_PARAGRAPH_INDENT,
    };
    value as f32 / 100.0 * PHANTOM_INDENT_BASE_PX
}

fn make_gap_phantom_block(
    node: &NodeView,
    width: f32,
    index: usize,
    resource: &mut Resource,
) -> (Arc<MeasuredNode>, f32) {
    let base_style =
        style_from_effective_modifiers(&node.effective().values().cloned().collect::<Vec<_>>());
    let indent = phantom_indent(node);
    let line = build_strut_only_line(
        node.id(),
        &base_style,
        width,
        Alignment::Left,
        indent,
        index..index,
        resource,
    );
    (
        Arc::new(MeasuredNode::from_line(width, line)),
        resolve_gap_after(node.effective()),
    )
}

pub(crate) fn layout_vertical<'a>(
    node: &NodeView<'a>,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
    measure_child: &mut dyn FnMut(
        ChildView<'a>,
        f32,
        &MeasureContext,
        &mut Resource,
    ) -> Arc<MeasuredNode>,
) -> (MeasuredChildren, f32) {
    let children: Vec<ChildView<'a>> = node.children().collect();
    let gap_phantom_index = ctx.gap_phantom_index(&node.id());

    let mut blocks: Vec<(Arc<MeasuredNode>, f32)> = Vec::with_capacity(children.len() + 1);
    let n_children = children.len();
    for (i, child) in children.into_iter().enumerate() {
        if gap_phantom_index == Some(i) {
            blocks.push(make_gap_phantom_block(node, width, i, resource));
        }
        let gap_after = resolve_gap_after(child_effective(node, i, &child));
        let m = measure_child(child, width, ctx, resource);
        blocks.push((m, gap_after));
    }
    if gap_phantom_index == Some(n_children) {
        blocks.push(make_gap_phantom_block(node, width, n_children, resource));
    }

    let mut result = Vec::with_capacity(blocks.len() * 2);
    let n = blocks.len();
    for (idx, (mnode, gap_after)) in blocks.into_iter().enumerate() {
        result.push(mnode);
        if idx + 1 < n && gap_after > 0.0 {
            result.push(Arc::new(MeasuredNode {
                width,
                height: gap_after,
                content: MeasuredContent::Spacing(gap_after),
            }));
        }
    }

    let children = MeasuredChildren::from_blocks(result);
    let total_height = children.total_height();
    (children, total_height)
}

pub(crate) fn layout_padded<'a>(
    node: &NodeView<'a>,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
    config: PaddedLayoutConfig,
    measure_child: &mut dyn FnMut(
        ChildView<'a>,
        f32,
        &MeasureContext,
        &mut Resource,
    ) -> Arc<MeasuredNode>,
) -> MeasuredNode {
    let PaddedLayoutConfig {
        padding,
        border,
        alignment,
        page_break_policy,
    } = config;
    let inner_width = width - padding.left - padding.right - border.left - border.right;
    let (children, children_height) =
        layout_vertical(node, inner_width, ctx, resource, measure_child);
    let total_height = children_height + padding.top + padding.bottom + border.top + border.bottom;

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding,
                border,
                border_mode: BorderMode::Separate,
                alignment,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children,
            page_break_policy,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, ChildView, DocLogs, DocView, HorizontalRuleVariant, Modifier, ModifierAttrLog,
        ModifierAttrOp::SetModifier, ModifierType, NodeAttrLog, NodeType, SeqItem, SpanLog,
        project_document,
    };
    use editor_resource::Resource;

    use crate::measure::PageBreakPolicy;
    use crate::measure::context::MeasureContext;
    use crate::view_state::GapPhantom;

    use super::*;
    use crate::measure::types::{MeasuredAtom, MeasuredContent, MeasuredNode};

    fn stub() -> impl FnMut(ChildView, f32, &MeasureContext, &mut Resource) -> Arc<MeasuredNode> {
        |child, _w, _ctx, _r| {
            let node = match &child {
                ChildView::Block(nv) => nv.id(),
                ChildView::Leaf(lv) => lv.dot(),
            };
            Arc::new(MeasuredNode {
                width: 100.0,
                height: 10.0,
                content: MeasuredContent::Atom(MeasuredAtom { node }),
            })
        }
    }

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

    fn build_root_two_paragraphs(block_gap: Option<Modifier>) -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let mut doc = logs(&items);
        if let Some(modifier) = block_gap {
            doc.block_modifiers = ModifierAttrLog::new()
                .apply(
                    Dot::ROOT,
                    SetModifier {
                        target: root,
                        modifier,
                    },
                )
                .unwrap();
        }
        doc
    }

    fn build_root_image_paragraph() -> DocLogs {
        let root = Dot::ROOT;
        let img = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                img,
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
                },
            ),
        ];
        logs(&items)
    }

    #[test]
    fn spacing_inserted_between_blocks() {
        let doc_gap = build_root_two_paragraphs(Some(Modifier::BlockGap { value: 100 }));
        let pd = project_document(&doc_gap).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let mut res = Resource::new_test();

        let (children, total) = layout_vertical(
            &root,
            300.0,
            &MeasureContext::default(),
            &mut res,
            &mut stub(),
        );
        assert_eq!(children.len(), 3);
        assert!(matches!(children[1].content, MeasuredContent::Spacing(_)));
        assert!((total - (2.0 * 10.0 + 16.0)).abs() < 1e-3);

        let doc_no_gap = build_root_two_paragraphs(None);
        let pd2 = project_document(&doc_no_gap).unwrap();
        let view2 = DocView::new(&pd2);
        let root2 = view2.root().unwrap();
        let (children2, _) = layout_vertical(
            &root2,
            300.0,
            &MeasureContext::default(),
            &mut res,
            &mut stub(),
        );
        assert_eq!(children2.len(), 2);
        assert!(
            !children2
                .iter()
                .any(|n| matches!(n.content, MeasuredContent::Spacing(_)))
        );
    }

    #[test]
    fn resolve_gap_after_converts_block_gap() {
        let mut eff = BTreeMap::new();
        eff.insert(ModifierType::BlockGap, Modifier::BlockGap { value: 100 });
        assert!((resolve_gap_after(&eff) - 16.0).abs() < 1e-3);

        let empty: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
        assert_eq!(resolve_gap_after(&empty), 0.0);
    }

    #[test]
    fn block_atom_leaf_not_dropped() {
        let doc = build_root_image_paragraph();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let mut res = Resource::new_test();

        let (children, _) = layout_vertical(
            &root,
            300.0,
            &MeasureContext::default(),
            &mut res,
            &mut stub(),
        );
        assert_eq!(children.len(), 2);
        let hr_dot = Dot::new(1, 1);
        assert!(
            children
                .iter()
                .any(|n| matches!(&n.content, MeasuredContent::Atom(a) if a.node == hr_dot))
        );
    }

    #[test]
    fn gap_phantom_inserts_strut_line() {
        let doc = build_root_two_paragraphs(None);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let mut res = Resource::new_test();

        let ctx_phantom = MeasureContext {
            gap_phantom: Some(GapPhantom {
                parent: root.id(),
                index: 0,
            }),
            ..Default::default()
        };
        let (children_phantom, total_phantom) =
            layout_vertical(&root, 300.0, &ctx_phantom, &mut res, &mut stub());
        let (_, total_none) = layout_vertical(
            &root,
            300.0,
            &MeasureContext::default(),
            &mut res,
            &mut stub(),
        );

        let first = &children_phantom[0];
        assert!(matches!(
            &first.content,
            MeasuredContent::Line(l) if l.offset_range == Some(0..0) && !l.is_phantom
        ));
        assert!(total_phantom > total_none);
    }

    #[test]
    fn padded_wraps_box() {
        let doc = build_root_two_paragraphs(None);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = layout_padded(
            &root,
            300.0,
            &MeasureContext::default(),
            &mut res,
            PaddedLayoutConfig {
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                alignment: crate::style::Alignment::Start,
                page_break_policy: PageBreakPolicy::Auto,
            },
            &mut stub(),
        );

        assert!(matches!(result.content, MeasuredContent::Box(_)));
        assert!((result.width - 300.0).abs() < 1e-3);
        if let MeasuredContent::Box(b) = &result.content {
            assert_eq!(b.style.monolithic, root.spec().monolithic);
        }
    }
}

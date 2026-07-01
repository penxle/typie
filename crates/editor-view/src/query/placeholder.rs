use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaceholderMetrics {
    pub page_idx: usize,
    pub rect: Rect,
    pub font_size: Option<u32>,
    pub line_height: Option<u32>,
    pub letter_spacing: Option<i32>,
    pub align: Option<Alignment>,
}

use editor_common::Rect;
use editor_model::{Alignment, DocView, Modifier, ModifierType, NodeType};

use super::layout_index::LayoutIndex;

pub(crate) fn placeholder_metrics(
    layout_index: &LayoutIndex,
    view: &DocView,
) -> Option<PlaceholderMetrics> {
    if !is_single_empty_paragraph(view) {
        return None;
    }
    let nv = view.root()?.child_blocks().next()?;
    let elem_id = nv.id();

    let entry = layout_index.box_entry(&elem_id)?;
    let page_rect = layout_index.page_rect(entry.rect)?;
    let indent = placeholder_indent(&nv);
    let rect = Rect::from_xywh(
        page_rect.rect.x + indent,
        page_rect.rect.y,
        (page_rect.rect.width - indent).max(0.0),
        page_rect.rect.height,
    );

    Some(PlaceholderMetrics {
        page_idx: page_rect.page_idx,
        rect,
        font_size: resolve_u32(&nv, ModifierType::FontSize),
        line_height: resolve_u32(&nv, ModifierType::LineHeight),
        letter_spacing: resolve_i32(&nv, ModifierType::LetterSpacing),
        align: resolve_align(&nv),
    })
}

pub(crate) fn is_single_empty_paragraph(view: &DocView) -> bool {
    let Some(root) = view.root() else {
        return false;
    };
    let mut blocks = root.child_blocks();
    let Some(first) = blocks.next() else {
        return false;
    };
    if blocks.next().is_some() {
        return false;
    }
    if !first.spec().is_textblock() {
        return false;
    }
    first.children().next().is_none()
}

fn resolve_u32(nv: &editor_model::NodeView<'_>, ty: ModifierType) -> Option<u32> {
    match nv.effective().get(&ty) {
        Some(Modifier::FontSize { value }) | Some(Modifier::LineHeight { value }) => Some(*value),
        _ => None,
    }
}

fn resolve_i32(nv: &editor_model::NodeView<'_>, ty: ModifierType) -> Option<i32> {
    match nv.effective().get(&ty) {
        Some(Modifier::LetterSpacing { value }) => Some(*value),
        _ => None,
    }
}

fn resolve_align(nv: &editor_model::NodeView<'_>) -> Option<Alignment> {
    match nv.effective().get(&ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => Some(*value),
        _ => None,
    }
}

fn placeholder_indent(nv: &editor_model::NodeView<'_>) -> f32 {
    let align = match nv.effective().get(&ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => *value,
        _ => Alignment::default(),
    };
    match align {
        Alignment::Left | Alignment::Justify => resolve_paragraph_indent(nv),
        Alignment::Center | Alignment::Right => 0.0,
    }
}

fn resolve_paragraph_indent(nv: &editor_model::NodeView<'_>) -> f32 {
    let parent_is_root = nv
        .parent()
        .map(|p| p.node_type() == NodeType::Root)
        .unwrap_or(false);
    if !parent_is_root {
        return 0.0;
    }
    match nv.effective().get(&ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent { value }) => *value as f32 / 100.0 * 16.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Alignment, AtomLeaf, DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp,
        NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType, SeqItem, SpanLog, StyleLog,
        project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;

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

    fn build_index_and_pd(doc: &DocLogs, width: f32) -> (LayoutIndex, editor_model::ProjectedDoc) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();
        let measured = measure_node(
            &mut crate::measure::Measurer::new(),
            &root_node,
            width,
            &MeasureContext::default(),
            &mut res,
        );
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (index, pd)
    }

    fn empty_para_elems() -> (Vec<(Dot, SeqItem)>, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let elems = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        (elems, root, para)
    }

    #[test]
    fn single_empty_paragraph_is_placeholder() {
        let (elems, _root, _para) = empty_para_elems();
        let doc = logs(&elems);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        assert!(is_single_empty_paragraph(&view));
    }

    #[test]
    fn single_empty_paragraph_metrics_some() {
        let (elems, _root, para) = empty_para_elems();
        let doc = logs(&elems);
        let (index, pd) = build_index_and_pd(&doc, 400.0);
        let view = DocView::new(&pd);

        let m = placeholder_metrics(&index, &view).expect("empty para must have metrics");
        assert_eq!(m.page_idx, 0);
        assert!(m.rect.width > 0.0);

        let para_id = para;
        let entry = index.box_entry(&para_id).unwrap();
        let page_rect = index.page_rect(entry.rect).unwrap();
        assert_eq!(m.rect.y, page_rect.rect.y);
    }

    #[test]
    fn paragraph_with_char_is_not_placeholder() {
        let root = Dot::ROOT;
        let para = Dot::new(2, 1);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(2, 2), SeqItem::Char('x')),
        ];
        let doc = logs(&elems);
        let (index, pd) = build_index_and_pd(&doc, 400.0);
        let view = DocView::new(&pd);

        assert!(!is_single_empty_paragraph(&view));
        assert!(placeholder_metrics(&index, &view).is_none());
    }

    #[test]
    fn paragraph_with_hard_break_is_not_placeholder() {
        let root = Dot::ROOT;
        let para = Dot::new(3, 1);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(3, 2), SeqItem::Atom(AtomLeaf::HardBreak)),
        ];
        let doc = logs(&elems);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);

        assert!(
            !is_single_empty_paragraph(&view),
            "paragraph with HardBreak must not be a placeholder"
        );
    }

    #[test]
    fn center_alignment_suppresses_indent() {
        let (elems, root, para) = empty_para_elems();
        let mut l = logs(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::Alignment {
                        value: Alignment::Center,
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 1),
                ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::ParagraphIndent { value: 200 },
                },
            )
            .unwrap();
        let (index, pd) = build_index_and_pd(&l, 400.0);
        let view = DocView::new(&pd);

        let m = placeholder_metrics(&index, &view).expect("must be placeholder");
        let entry = index.box_entry(&para).unwrap();
        let page_rect = index.page_rect(entry.rect).unwrap();
        assert!(
            (m.rect.x - page_rect.rect.x).abs() < 0.01,
            "center alignment must suppress indent: rect.x ({}) == page_rect.x ({})",
            m.rect.x,
            page_rect.rect.x
        );
    }

    #[test]
    fn left_alignment_with_paragraph_indent_applied() {
        let (elems, root, para) = empty_para_elems();
        let mut l = logs(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::ParagraphIndent { value: 200 },
                },
            )
            .unwrap();
        let (index, pd) = build_index_and_pd(&l, 400.0);
        let view = DocView::new(&pd);

        let m = placeholder_metrics(&index, &view).expect("must be placeholder");
        let entry = index.box_entry(&para).unwrap();
        let page_rect = index.page_rect(entry.rect).unwrap();
        let expected_indent = 200.0f32 / 100.0 * 16.0;
        assert!(
            (m.rect.x - (page_rect.rect.x + expected_indent)).abs() < 0.01,
            "left alignment with paragraph_indent 200 must offset rect.x by {expected_indent}: got {}",
            m.rect.x
        );
    }
}

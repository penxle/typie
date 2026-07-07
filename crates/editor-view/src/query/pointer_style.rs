use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerStyle {
    Default,
    Text,
    Pointer,
}

use editor_model::{DocView, Node};

use crate::paginate::types::{LayoutContent, LayoutNode};

use super::interactive::InteractiveHit;
use super::layout_index::{LayoutEntry, LayoutIndex};

pub(crate) fn pointer_style_at(
    layout_index: &LayoutIndex,
    view: &DocView,
    page_idx: usize,
    x: f32,
    page_y: f32,
    read_only: bool,
) -> PointerStyle {
    if let Some(hit) =
        super::interactive::interactive_hit_test(layout_index, view, page_idx, x, page_y)
    {
        return match hit {
            InteractiveHit::CalloutIcon { .. } => PointerStyle::Pointer,
            InteractiveHit::FoldTitle { text_rect, .. } => {
                if read_only {
                    PointerStyle::Pointer
                } else if text_rect.is_some_and(|r| r.contains(x, page_y)) {
                    PointerStyle::Text
                } else {
                    PointerStyle::Pointer
                }
            }
        };
    }

    let Some(point) = layout_index.point(page_idx, x, page_y) else {
        return PointerStyle::Text;
    };
    if let Some(hit) = layout_index.exact_entry(point, is_pointer_entry) {
        return match hit.content(layout_index) {
            Some(LayoutContent::Line(_)) => PointerStyle::Text,
            Some(LayoutContent::Atom(atom)) => atom_pointer_style(view, atom.node),
            Some(LayoutContent::Box(b)) => {
                let n = view.node(b.node).map(|nv| nv.node());
                n.as_ref()
                    .map(box_pointer_style)
                    .unwrap_or(PointerStyle::Text)
            }
            Some(LayoutContent::Spacing(_)) | None => PointerStyle::Text,
        };
    }

    PointerStyle::Text
}

fn is_pointer_entry(_entry: &LayoutEntry, node: &LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_) | LayoutContent::Atom(_) | LayoutContent::Box(_)
    )
}

fn atom_pointer_style(view: &DocView, id: editor_crdt::Dot) -> PointerStyle {
    use editor_model::NodeType;
    if let Some(nv) = view.node(id) {
        return match nv.node() {
            Node::Image(_)
            | Node::File(_)
            | Node::Embed(_)
            | Node::Archived(_)
            | Node::HorizontalRule(_)
            | Node::PageBreak(_) => PointerStyle::Default,
            _ => PointerStyle::Text,
        };
    }
    if let Some(lv) = view.leaf(id) {
        return match lv.node_type() {
            NodeType::Image
            | NodeType::File
            | NodeType::Embed
            | NodeType::Archived
            | NodeType::HorizontalRule
            | NodeType::PageBreak => PointerStyle::Default,
            _ => PointerStyle::Text,
        };
    }
    PointerStyle::Text
}

fn box_pointer_style(node: &Node) -> PointerStyle {
    match node {
        Node::FoldTitle(_) => PointerStyle::Pointer,
        Node::FoldContent(_) => PointerStyle::Default,
        Node::Image(_)
        | Node::File(_)
        | Node::Embed(_)
        | Node::Archived(_)
        | Node::HorizontalRule(_)
        | Node::PageBreak(_) => PointerStyle::Default,
        _ => PointerStyle::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, ModifierAttrLog, NodeAttrLog,
        NodeType, SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::query::layout_index::LayoutIndex;

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

    fn build_index(logs_data: &DocLogs, width: f32) -> LayoutIndex {
        let pd = project_document(logs_data).unwrap();
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
        LayoutIndex::new(layout.tree, &layout.pages)
    }

    fn style_at(
        layout_index: &LayoutIndex,
        view: &DocView,
        x: f32,
        page_y: f32,
        read_only: bool,
    ) -> PointerStyle {
        pointer_style_at(layout_index, view, 0, x, page_y, read_only)
    }

    #[test]
    fn hr_atom_returns_default() {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let para = Dot::new(1, 2);
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
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let dl = logs(&items);
        let pd = project_document(&dl).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&dl, 400.0);

        let hr_id = hr;
        let hr_entry = index
            .entries_on_page(0)
            .into_iter()
            .find(|e| {
                e.node(&index).is_some_and(
                    |n| matches!(&n.content, LayoutContent::Atom(a) if a.node == hr_id),
                )
            })
            .expect("hr atom entry");
        let hr_rect = hr_entry.rect;
        let mid_x = hr_rect.x + hr_rect.width / 2.0;
        let mid_y = hr_rect.y + hr_rect.height / 2.0;

        assert_eq!(
            style_at(&index, &view, mid_x, mid_y, false),
            PointerStyle::Default
        );
        assert_eq!(
            style_at(&index, &view, mid_x, mid_y, true),
            PointerStyle::Default
        );
    }

    #[test]
    fn text_line_returns_text() {
        let root = Dot::ROOT;
        let para = Dot::new(2, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 2), SeqItem::Char('H')),
            (Dot::new(2, 3), SeqItem::Char('i')),
        ];
        let dl = logs(&items);
        let pd = project_document(&dl).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&dl, 400.0);

        let para_id = para;
        let para_rect = index.box_rect(&para_id).expect("para rect");
        let mid_x = para_rect.x + para_rect.width / 2.0;
        let mid_y = para_rect.y + para_rect.height / 2.0;

        assert_eq!(
            style_at(&index, &view, mid_x, mid_y, false),
            PointerStyle::Text
        );
        assert_eq!(
            style_at(&index, &view, mid_x, mid_y, true),
            PointerStyle::Text
        );
    }

    #[test]
    fn fold_title_read_only_returns_pointer() {
        let root = Dot::ROOT;
        let fold = Dot::new(3, 1);
        let ft = Dot::new(3, 2);
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
            (Dot::new(3, 3), SeqItem::Char('T')),
        ];
        let dl = logs(&items);
        let pd = project_document(&dl).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&dl, 400.0);

        let ft_id = ft;
        let ft_rect = index.box_rect(&ft_id).expect("fold_title rect");
        let outside_x = ft_rect.x + 2.0;
        let mid_y = ft_rect.y + ft_rect.height / 2.0;

        assert_eq!(
            style_at(&index, &view, outside_x, mid_y, true),
            PointerStyle::Pointer
        );
        assert_eq!(
            style_at(&index, &view, outside_x, mid_y, false),
            PointerStyle::Pointer
        );
    }
}

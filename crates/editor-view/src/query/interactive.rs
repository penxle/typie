use editor_common::Rect;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{CalloutVariant, DocView, Node};
use serde::{Deserialize, Serialize};

use crate::paginate::types::{LayoutContent, LayoutNode};

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractiveHit {
    FoldTitle {
        id: Dot,
        text_rect: Option<Rect>,
    },
    CalloutIcon {
        id: Dot,
        next_variant: CalloutVariant,
    },
}

pub(crate) fn interactive_hit_test(
    layout_index: &LayoutIndex,
    view: &DocView,
    page_idx: usize,
    x: f32,
    page_y: f32,
) -> Option<InteractiveHit> {
    let point = layout_index.point(page_idx, x, page_y)?;
    let entry =
        layout_index.exact_entry(point, |entry, node| is_interactive_entry(entry, node, view))?;
    interactive_hit_for_entry(layout_index, view, point, entry)
}

fn is_interactive_entry(_entry: &LayoutEntry, node: &LayoutNode, view: &DocView) -> bool {
    let LayoutContent::Box(b) = &node.content else {
        return false;
    };
    view.node(b.node)
        .is_some_and(|node| matches!(node.node(), Node::Callout(_) | Node::FoldTitle(_)))
}

fn interactive_hit_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    point: LayoutPoint,
    entry: &LayoutEntry,
) -> Option<InteractiveHit> {
    let LayoutContent::Box(b) = entry.content(layout_index)? else {
        return None;
    };
    let node_ref = view.node(b.node)?;
    match node_ref.node() {
        Node::Callout(callout) => {
            let dec = b.style.decorations.iter().find(|d| d.id == 0)?;
            let icon = Rect::from_xywh(
                entry.rect.x + dec.rect.x,
                entry.rect.y + dec.rect.y,
                dec.rect.width,
                dec.rect.height,
            );
            if icon.contains(point.x, point.y) {
                Some(InteractiveHit::CalloutIcon {
                    id: b.node,
                    next_variant: callout.variant.get().next(),
                })
            } else {
                None
            }
        }
        Node::FoldTitle(_) => Some(InteractiveHit::FoldTitle {
            id: node_ref.parent()?.id(),
            text_rect: navigable_union_in(layout_index, point, &entry.rect)
                .map(|r| Rect::from_xywh(r.x, r.y - point.page_y_start, r.width, r.height)),
        }),
        _ => None,
    }
}

fn navigable_union_in(
    layout_index: &LayoutIndex,
    point: LayoutPoint,
    container: &Rect,
) -> Option<Rect> {
    layout_index
        .entries_on_page(point.page_idx)
        .into_iter()
        .filter(|entry| {
            entry.content(layout_index).is_some_and(|content| {
                matches!(content, LayoutContent::Line(_) | LayoutContent::Atom(_))
                    && rect_contains_rect(container, &entry.rect)
            })
        })
        .fold(None, |acc, entry| Some(union_rect(acc, entry.rect)))
}

fn rect_contains_rect(outer: &Rect, inner: &Rect) -> bool {
    inner.x >= outer.x
        && inner.right() <= outer.right()
        && inner.y >= outer.y
        && inner.bottom() <= outer.bottom()
}

fn union_rect(acc: Option<Rect>, rect: Rect) -> Rect {
    match acc {
        None => rect,
        Some(prev) => {
            let x = prev.x.min(rect.x);
            let y = prev.y.min(rect.y);
            Rect::from_xywh(
                x,
                y,
                prev.right().max(rect.right()) - x,
                prev.bottom().max(rect.bottom()) - y,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        SeqItem, SpanLog, StyleLog, project_document,
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
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> LayoutIndex {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();
        let measured = measure_node(&root_node, width, &MeasureContext::default(), &mut res);
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        LayoutIndex::new(layout.tree, &layout.pages)
    }

    #[test]
    fn callout_icon_hit() {
        let root = Dot::ROOT;
        let callout = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let items = vec![
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let callout_id = callout;
        let callout_rect = index.box_rect(&callout_id).expect("callout box rect");

        let icon_x = callout_rect.x + 12.0 + 10.0;
        let icon_y = callout_rect.y + 16.0 + 10.0;
        let page_y = icon_y;

        let hit = interactive_hit_test(&index, &view, 0, icon_x, page_y);
        match hit {
            Some(InteractiveHit::CalloutIcon {
                id,
                next_variant: _,
            }) => {
                assert_eq!(id, callout_id, "hit id must be callout Dot");
            }
            other => panic!("expected CalloutIcon, got {other:?}"),
        }

        let outside_hit = interactive_hit_test(&index, &view, 0, callout_rect.x + 200.0, page_y);
        assert_eq!(outside_hit, None, "hit outside icon must be None");
    }

    #[test]
    fn fold_title_hit() {
        let root = Dot::ROOT;
        let fold = Dot::new(2, 1);
        let ft = Dot::new(2, 2);
        let items = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                },
            ),
            (
                ft,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                },
            ),
            (Dot::new(2, 3), SeqItem::Char('T')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let fold_id = fold;
        let ft_id = ft;
        let ft_rect = index.box_rect(&ft_id).expect("fold_title box rect");

        let hit_x = ft_rect.x + ft_rect.width / 2.0;
        let hit_y = ft_rect.y + ft_rect.height / 2.0;

        let hit = interactive_hit_test(&index, &view, 0, hit_x, hit_y);
        match hit {
            Some(InteractiveHit::FoldTitle { id, text_rect }) => {
                assert_eq!(id, fold_id, "toggle target must be parent fold Dot");
                assert!(
                    text_rect.is_some(),
                    "text_rect must be Some for a title with text"
                );
            }
            other => panic!("expected FoldTitle, got {other:?}"),
        }
    }

    #[test]
    fn non_interactive_none() {
        let root = Dot::ROOT;
        let para = Dot::new(3, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(3, 2), SeqItem::Char('p')),
            (Dot::new(3, 3), SeqItem::Char('l')),
            (Dot::new(3, 4), SeqItem::Char('a')),
            (Dot::new(3, 5), SeqItem::Char('i')),
            (Dot::new(3, 6), SeqItem::Char('n')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let index = build_index(&doc, 400.0);

        let para_id = para;
        let para_rect = index.box_rect(&para_id).expect("para box rect");

        let hit = interactive_hit_test(&index, &view, 0, para_rect.x + 10.0, para_rect.y + 5.0);
        assert_eq!(hit, None, "plain paragraph hit must be None");
    }
}

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

// Ordered by owning entry area ascending, mirroring exact_entry's smallest-wins
// rule: consumers must take the first region whose entry_rect contains the
// point, then answer from effective_rect alone without falling through.
#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InteractiveRegion {
    pub page_idx: usize,
    pub entry_rect: Rect,
    pub effective_rect: Rect,
    pub hit: InteractiveHit,
}

pub(crate) fn interactive_regions(
    layout_index: &LayoutIndex,
    view: &DocView,
) -> Vec<InteractiveRegion> {
    let pages = layout_index.pages();
    let mut scored: Vec<(f32, InteractiveRegion)> = Vec::new();
    for entry in layout_index.entries() {
        let Some(LayoutContent::Box(b)) = entry.content(layout_index) else {
            continue;
        };
        let Some(node_ref) = view.node(b.node) else {
            continue;
        };
        let area = entry.rect.width * entry.rect.height;
        let to_local = |rect: &Rect, y_start: f32| {
            Rect::from_xywh(rect.x, rect.y - y_start, rect.width, rect.height)
        };
        match node_ref.node() {
            Node::Callout(callout) => {
                // A callout without its icon decoration still owns its entry area in
                // exact_entry and answers None, so it must block rather than vanish.
                let icon = match b.style.decorations.iter().find(|d| d.id == 0) {
                    Some(dec) => Rect::from_xywh(
                        entry.rect.x + dec.rect.x,
                        entry.rect.y + dec.rect.y,
                        dec.rect.width,
                        dec.rect.height,
                    ),
                    None => Rect::from_xywh(0.0, 0.0, -1.0, -1.0),
                };
                for (page_idx, page) in pages.iter().enumerate() {
                    if entry.rect.y >= page.y_end || entry.rect.bottom() <= page.y_start {
                        continue;
                    }
                    scored.push((
                        area,
                        InteractiveRegion {
                            page_idx,
                            entry_rect: to_local(&entry.rect, page.y_start),
                            effective_rect: to_local(&icon, page.y_start),
                            hit: InteractiveHit::CalloutIcon {
                                id: b.node,
                                next_variant: callout.variant.get().next(),
                            },
                        },
                    ));
                }
            }
            Node::FoldTitle(_) => {
                let Some(parent) = node_ref.parent() else {
                    continue;
                };
                for (page_idx, page) in pages.iter().enumerate() {
                    if entry.rect.y >= page.y_end || entry.rect.bottom() <= page.y_start {
                        continue;
                    }
                    let local_entry = to_local(&entry.rect, page.y_start);
                    scored.push((
                        area,
                        InteractiveRegion {
                            page_idx,
                            entry_rect: local_entry,
                            effective_rect: local_entry,
                            hit: InteractiveHit::FoldTitle {
                                id: parent.id(),
                                text_rect: navigable_union_in(layout_index, page_idx, &entry.rect)
                                    .map(|r| {
                                        Rect::from_xywh(r.x, r.y - page.y_start, r.width, r.height)
                                    }),
                            },
                        },
                    ));
                }
            }
            _ => {}
        }
    }
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    scored.into_iter().map(|(_, region)| region).collect()
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
            text_rect: navigable_union_in(layout_index, point.page_idx, &entry.rect)
                .map(|r| Rect::from_xywh(r.x, r.y - point.page_y_start, r.width, r.height)),
        }),
        _ => None,
    }
}

fn navigable_union_in(
    layout_index: &LayoutIndex,
    page_idx: usize,
    container: &Rect,
) -> Option<Rect> {
    layout_index
        .entries_on_page(page_idx)
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
        AliasLog, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, SeqItem, SpanLog,
        project_document,
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

    fn build_index(doc: &DocLogs, width: f32) -> LayoutIndex {
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
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, callout],
                    attrs: vec![],
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

    fn region_lookup(
        regions: &[InteractiveRegion],
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<InteractiveHit> {
        let region = regions
            .iter()
            .find(|r| r.page_idx == page_idx && r.entry_rect.contains(x, y))?;
        region
            .effective_rect
            .contains(x, y)
            .then(|| region.hit.clone())
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 24,
            ..Default::default()
        })]
        #[test]
        fn interactive_regions_agree_with_hit_test(
            width in 200.0f32..500.0,
            title_text in proptest::collection::vec(
                proptest::sample::select(vec!['t', '한', ' ', 'q']),
                1..8,
            ),
            body_text in proptest::collection::vec(
                proptest::sample::select(vec!['b', '글', ' ', 'z']),
                0..12,
            ),
            include_callout in proptest::prelude::any::<bool>(),
            include_fold in proptest::prelude::any::<bool>(),
        ) {
            let root = Dot::ROOT;
            let mut items = Vec::new();
            if include_callout {
                let callout = Dot::new(1, 1);
                let para = Dot::new(1, 2);
                items.push((
                    callout,
                    SeqItem::Block {
                        node_type: NodeType::Callout,
                        parents: vec![root],
                        attrs: vec![],
                    },
                ));
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root, callout],
                        attrs: vec![],
                    },
                ));
                for (j, ch) in body_text.iter().enumerate() {
                    items.push((Dot::new(1, 3 + j as u64), SeqItem::Char(*ch)));
                }
            }
            if include_fold {
                let fold = Dot::new(2, 1);
                let ft = Dot::new(2, 2);
                items.push((
                    fold,
                    SeqItem::Block {
                        node_type: NodeType::Fold,
                        parents: vec![root],
                        attrs: vec![],
                    },
                ));
                items.push((
                    ft,
                    SeqItem::Block {
                        node_type: NodeType::FoldTitle,
                        parents: vec![root, fold],
                        attrs: vec![],
                    },
                ));
                for (j, ch) in title_text.iter().enumerate() {
                    items.push((Dot::new(2, 3 + j as u64), SeqItem::Char(*ch)));
                }
            }
            let plain = Dot::new(3, 1);
            items.push((
                plain,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ));
            for (j, ch) in body_text.iter().enumerate() {
                items.push((Dot::new(3, 2 + j as u64), SeqItem::Char(*ch)));
            }

            let doc = logs(&items);
            let pd = project_document(&doc).unwrap();
            let view = DocView::new(&pd);
            let index = build_index(&doc, width);
            let regions = interactive_regions(&index, &view);

            let mut bbox: Option<Rect> = None;
            for entry in index.entries() {
                let rect = entry.rect;
                bbox = Some(match bbox {
                    None => rect,
                    Some(prev) => {
                        let x0 = prev.x.min(rect.x);
                        let y0 = prev.y.min(rect.y);
                        Rect::from_xywh(
                            x0,
                            y0,
                            prev.right().max(rect.right()) - x0,
                            prev.bottom().max(rect.bottom()) - y0,
                        )
                    }
                });
            }
            let Some(bbox) = bbox else { return Ok(()) };

            let steps = 20;
            for iy in 0..=steps {
                for ix in 0..=steps {
                    let x = bbox.x - 15.0
                        + (bbox.width + 30.0) * (ix as f32 + 0.37) / (steps as f32 + 1.0);
                    let y = bbox.y - 15.0
                        + (bbox.height + 30.0) * (iy as f32 + 0.41) / (steps as f32 + 1.0);
                    let from_regions = region_lookup(&regions, 0, x, y);
                    let from_hit_test = interactive_hit_test(&index, &view, 0, x, y);
                    proptest::prop_assert_eq!(
                        &from_regions,
                        &from_hit_test,
                        "divergence at ({}, {})",
                        x,
                        y
                    );
                }
            }
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
                    attrs: vec![],
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

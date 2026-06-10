use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{CalloutVariant, Doc, Node, NodeId};
use serde::{Deserialize, Serialize};

use crate::paginate::{LayoutContent, LayoutNode};

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractiveHit {
    FoldTitle {
        id: NodeId,
        text_rect: Option<Rect>,
    },
    CalloutIcon {
        id: NodeId,
        next_variant: CalloutVariant,
    },
}

pub(crate) fn interactive_hit_test(
    layout_index: &LayoutIndex,
    doc: &Doc,
    page_idx: usize,
    x: f32,
    page_y: f32,
) -> Option<InteractiveHit> {
    let point = layout_index.point(page_idx, x, page_y)?;
    let entry =
        layout_index.exact_entry(point, |entry, node| is_interactive_entry(entry, node, doc))?;
    interactive_hit_for_entry(layout_index, doc, point, entry)
}

fn is_interactive_entry(_entry: &LayoutEntry, node: &LayoutNode, doc: &Doc) -> bool {
    let LayoutContent::Box(b) = &node.content else {
        return false;
    };
    doc.node(b.node_id)
        .is_some_and(|node| matches!(node.node(), Node::Callout(_) | Node::FoldTitle(_)))
}

fn interactive_hit_for_entry(
    layout_index: &LayoutIndex,
    doc: &Doc,
    point: LayoutPoint,
    entry: &LayoutEntry,
) -> Option<InteractiveHit> {
    let LayoutContent::Box(b) = entry.content(layout_index)? else {
        return None;
    };
    let node_ref = doc.node(b.node_id)?;
    match node_ref.node() {
        Node::Callout(callout) => {
            // measure_callout assigns the icon decoration id 0.
            let dec = b.style.decorations.iter().find(|d| d.id == 0)?;
            let icon = Rect::from_xywh(
                entry.rect.x + dec.rect.x,
                entry.rect.y + dec.rect.y,
                dec.rect.width,
                dec.rect.height,
            );
            if icon.contains(point.x, point.y) {
                Some(InteractiveHit::CalloutIcon {
                    id: b.node_id,
                    next_variant: callout.variant.get().next(),
                })
            } else {
                None
            }
        }
        Node::FoldTitle(_) => Some(InteractiveHit::FoldTitle {
            id: node_ref.parent()?.id(),
            // Legacy parity: cursor-placeable (Line/Atom)
            // leaves, so the host can apply edit-mode passthrough.
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
    use crate::page::LayoutPage;
    use crate::paginate::*;
    use crate::style::*;
    use editor_common::{EdgeInsets, Size};
    use editor_macros::doc;

    fn page(y_start: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + 1000.0, Size::new(800.0, 1000.0))
    }

    fn hit(
        tree: &LayoutTree,
        pages: &[LayoutPage],
        doc: &Doc,
        x: f32,
        y: f32,
    ) -> Option<InteractiveHit> {
        let layout_index = LayoutIndex::new(tree.clone(), pages);
        interactive_hit_test(&layout_index, doc, 0, x, y)
    }

    fn line_node(id: NodeId, x: f32, y: f32, w: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: h,
                ascent: h,
                descent: 0.0,
                cursor_ascent: h,
                cursor_descent: 0.0,
                glyph_runs: vec![],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn box_node(
        id: NodeId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        decorations: Vec<Decoration>,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations,
                    monolithic: false,
                    ..Default::default()
                },
                children,
                nav: None,
            }),
        }
    }

    #[test]
    fn fold_title_hit_returns_fold_id_and_page_local_text_union() {
        let (doc, f1, ft1) = doc! {
            root {
                f1: fold {
                    ft1: fold_title { text("Hi") }
                    fold_content { paragraph { text("Body") } }
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                140.0,
                vec![],
                vec![box_node(
                    f1,
                    0.0,
                    100.0,
                    200.0,
                    40.0,
                    vec![],
                    vec![box_node(
                        ft1,
                        0.0,
                        100.0,
                        200.0,
                        40.0,
                        vec![],
                        vec![line_node(ft1, 40.0, 108.0, 30.0, 20.0)],
                    )],
                )],
            ),
        };
        // click point is in the fold-title chevron area (page at y_start 100).
        let pages = [page(100.0)];
        let hit = hit(&tree, &pages, &doc, 20.0, 4.0);
        match hit {
            Some(InteractiveHit::FoldTitle { id, text_rect }) => {
                assert_eq!(id, f1, "toggle target = parent fold");
                // text_rect is returned page-local (page starts at y_start 100).
                assert_eq!(text_rect, Some(Rect::from_xywh(40.0, 8.0, 30.0, 20.0)));
            }
            other => panic!("expected FoldTitle, got {other:?}"),
        }
    }

    #[test]
    fn callout_icon_hit_returns_next_variant() {
        let (doc, c1) = doc! { root { c1: callout { paragraph { text("x") } } } };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![],
                vec![box_node(
                    c1,
                    0.0,
                    0.0,
                    200.0,
                    40.0,
                    vec![Decoration {
                        id: 0,
                        rect: Rect::from_xywh(12.0, 10.0, 20.0, 20.0),
                        data: DecorationData::None,
                    }],
                    vec![line_node(NodeId::new(), 40.0, 8.0, 20.0, 20.0)],
                )],
            ),
        };
        // page at y_start 0; (20,18) is inside the icon rect.
        assert_eq!(
            hit(&tree, &[page(0.0)], &doc, 20.0, 18.0),
            Some(InteractiveHit::CalloutIcon {
                id: c1,
                next_variant: CalloutVariant::Success,
            })
        );
        assert_eq!(hit(&tree, &[page(0.0)], &doc, 50.0, 18.0), None);
    }

    #[test]
    fn miss_returns_none() {
        let (doc,) = doc! { root { paragraph { text("plain") } } };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![],
                vec![line_node(NodeId::new(), 0.0, 0.0, 50.0, 20.0)],
            ),
        };
        assert_eq!(hit(&tree, &[page(0.0)], &doc, 10.0, 10.0), None);
    }
}

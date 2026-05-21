use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{CalloutVariant, Doc, Node, NodeId};
use serde::{Deserialize, Serialize};

use crate::page::LayoutPage;
use crate::paginate::*;

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
    tree: &LayoutTree,
    page: &LayoutPage,
    doc: &Doc,
    x: f32,
    page_y: f32,
) -> Option<InteractiveHit> {
    let abs_y = page_y + page.y_start;
    let hit = hit_node(&tree.root, x, abs_y, doc)?;
    // page-local output, consistent with cursor_metrics / node_box_rects.
    Some(match hit {
        InteractiveHit::FoldTitle { id, text_rect } => InteractiveHit::FoldTitle {
            id,
            text_rect: text_rect
                .map(|r| Rect::from_xywh(r.x, r.y - page.y_start, r.width, r.height)),
        },
        other => other,
    })
}

fn hit_node(node: &LayoutNode, x: f32, y: f32, doc: &Doc) -> Option<InteractiveHit> {
    let LayoutContent::Box(b) = &node.content else {
        return None;
    };
    if !node.rect.contains(x, y) {
        return None;
    }
    for child in &b.children {
        if let Some(hit) = hit_node(child, x, y, doc) {
            return Some(hit);
        }
    }
    let node_ref = doc.node(b.node_id)?;
    match node_ref.node() {
        Node::Callout(callout) => {
            // measure_callout assigns the icon decoration id 0.
            let dec = b.style.decorations.iter().find(|d| d.id == 0)?;
            let icon = Rect::from_xywh(
                node.rect.x + dec.rect.x,
                node.rect.y + dec.rect.y,
                dec.rect.width,
                dec.rect.height,
            );
            if icon.contains(x, y) {
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
            // Legacy navigable_union_in parity: cursor-placeable (Line/Atom)
            // leaves, so the host can apply edit-mode passthrough.
            text_rect: navigable_union(node),
        }),
        _ => None,
    }
}

fn navigable_union(node: &LayoutNode) -> Option<Rect> {
    fn walk(node: &LayoutNode, acc: &mut Option<Rect>) {
        match &node.content {
            LayoutContent::Line(_) | LayoutContent::Atom(_) => {
                let r = node.rect;
                *acc = Some(match *acc {
                    None => r,
                    Some(p) => {
                        let x = p.x.min(r.x);
                        let y = p.y.min(r.y);
                        Rect::from_xywh(
                            x,
                            y,
                            p.right().max(r.right()) - x,
                            p.bottom().max(r.bottom()) - y,
                        )
                    }
                });
            }
            LayoutContent::Box(b) => {
                for c in &b.children {
                    walk(c, acc);
                }
            }
            LayoutContent::Spacing(_) => {}
        }
    }
    let mut acc = None;
    if let LayoutContent::Box(b) = &node.content {
        for c in &b.children {
            walk(c, &mut acc);
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::*;
    use editor_common::{EdgeInsets, Size};
    use editor_macros::doc;

    fn page(y_start: f32) -> LayoutPage {
        LayoutPage {
            y_start,
            y_end: y_start + 1000.0,
            size: Size::new(800.0, 1000.0),
        }
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
                text_indent: 0.0,
                child_range: None,
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
                    scope: false,
                    decorations,
                    monolithic: false,
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
        let hit = interactive_hit_test(&tree, &page(100.0), &doc, 20.0, 4.0);
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
            interactive_hit_test(&tree, &page(0.0), &doc, 20.0, 18.0),
            Some(InteractiveHit::CalloutIcon {
                id: c1,
                next_variant: CalloutVariant::Success,
            })
        );
        assert_eq!(
            interactive_hit_test(&tree, &page(0.0), &doc, 50.0, 18.0),
            None
        );
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
        assert_eq!(
            interactive_hit_test(&tree, &page(0.0), &doc, 10.0, 10.0),
            None
        );
    }
}

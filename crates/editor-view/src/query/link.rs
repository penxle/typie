use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Doc, Modifier, NodeId};
use serde::{Deserialize, Serialize};

use crate::page::LayoutPage;
use crate::paginate::{LayoutContent, LayoutNode, LayoutTree};

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LinkRect {
    pub node_id: NodeId,
    pub page_idx: usize,
    pub href: String,
    pub rects: Vec<Rect>,
}

pub(crate) fn page_link_rects(
    tree: &LayoutTree,
    page: &LayoutPage,
    page_idx: usize,
    doc: &Doc,
) -> Vec<LinkRect> {
    let mut out: Vec<LinkRect> = Vec::new();
    collect(&tree.root, page, page_idx, doc, &mut out);
    out
}

pub(crate) fn link_hit_test(
    tree: &LayoutTree,
    page: &LayoutPage,
    page_idx: usize,
    doc: &Doc,
    x: f32,
    page_y: f32,
) -> Option<LinkRect> {
    page_link_rects(tree, page, page_idx, doc)
        .into_iter()
        .find(|link| link.rects.iter().any(|r| r.contains(x, page_y)))
}

fn collect(
    node: &LayoutNode,
    page: &LayoutPage,
    page_idx: usize,
    doc: &Doc,
    out: &mut Vec<LinkRect>,
) {
    let node_top = node.rect.y;
    let node_bottom = node.rect.y + node.rect.height;
    if node_bottom <= page.y_start || node_top >= page.y_end {
        return;
    }

    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                collect(child, page, page_idx, doc, out);
            }
        }
        LayoutContent::Line(l) => {
            for run in &l.glyph_runs {
                let Some(href) = link_href(doc, run.node_id) else {
                    continue;
                };
                let rect = Rect::from_xywh(
                    node.rect.x + run.x,
                    node.rect.y - page.y_start,
                    run.width,
                    node.rect.height,
                );
                push_rect(out, run.node_id, page_idx, href, rect);
            }
        }
        LayoutContent::Atom(_) | LayoutContent::Spacing(_) => {}
    }
}

fn link_href(doc: &Doc, node_id: NodeId) -> Option<String> {
    doc.node(node_id)?.modifiers().find_map(|m| match m {
        Modifier::Link { href } => Some(href.clone()),
        _ => None,
    })
}

fn push_rect(out: &mut Vec<LinkRect>, node_id: NodeId, page_idx: usize, href: String, rect: Rect) {
    if let Some(existing) = out.iter_mut().find(|l| l.node_id == node_id) {
        existing.rects.push(rect);
        return;
    }
    out.push(LinkRect {
        node_id,
        page_idx,
        href,
        rects: vec![rect],
    });
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Size};
    use editor_macros::doc;

    use super::*;
    use crate::glyph_run::GlyphRun;
    use crate::paginate::*;
    use crate::style::*;

    fn page(y_start: f32, height: f32) -> LayoutPage {
        LayoutPage {
            y_start,
            y_end: y_start + height,
            size: Size::new(800.0, height),
        }
    }

    fn box_node(
        id: NodeId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
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
                    decorations: vec![],
                    monolithic: false,
                    ..Default::default()
                },
                table_info: None,
                children,
                nav: None,
            }),
        }
    }

    fn line_node(x: f32, y: f32, w: f32, h: f32, glyph_runs: Vec<GlyphRun>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Line(LayoutLine {
                node_id: NodeId::new(),
                baseline: h * 0.8,
                ascent: h * 0.8,
                descent: h * 0.2,
                cursor_ascent: h * 0.8,
                cursor_descent: h * 0.2,
                glyph_runs,
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
            }),
        }
    }

    fn run(node_id: NodeId, x: f32, width: f32) -> GlyphRun {
        let mut r = GlyphRun::make_test_run(node_id, 0, "x", x, vec![]);
        r.width = width;
        r
    }

    #[test]
    fn collects_link_rect_from_glyph_run() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![line_node(20.0, 10.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        let links = page_link_rects(&tree, &page(0.0, 100.0), 0, &doc);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].node_id, t);
        assert_eq!(links[0].href, "https://a.example");
        assert_eq!(
            links[0].rects,
            vec![Rect::from_xywh(20.0, 10.0, 50.0, 20.0)]
        );
    }

    #[test]
    fn ignores_runs_without_link_modifier() {
        let (doc, t) = doc! {
            root { paragraph { t: text("plain") } }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![line_node(20.0, 10.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        assert!(page_link_rects(&tree, &page(0.0, 100.0), 0, &doc).is_empty());
    }

    #[test]
    fn groups_rects_per_text_node_across_lines() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("wrapped link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                80.0,
                vec![
                    line_node(20.0, 10.0, 180.0, 20.0, vec![run(t, 0.0, 60.0)]),
                    line_node(20.0, 30.0, 180.0, 20.0, vec![run(t, 0.0, 40.0)]),
                ],
            ),
        };
        let links = page_link_rects(&tree, &page(0.0, 100.0), 0, &doc);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].rects,
            vec![
                Rect::from_xywh(20.0, 10.0, 60.0, 20.0),
                Rect::from_xywh(20.0, 30.0, 40.0, 20.0),
            ]
        );
    }

    #[test]
    fn returns_separate_entries_for_distinct_text_nodes() {
        let (doc, a, b) = doc! {
            root {
                paragraph {
                    a: text("first") [link(href: "https://a.example".into())]
                    b: text("second") [link(href: "https://b.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![line_node(
                    20.0,
                    10.0,
                    180.0,
                    20.0,
                    vec![run(a, 0.0, 40.0), run(b, 40.0, 50.0)],
                )],
            ),
        };
        let links = page_link_rects(&tree, &page(0.0, 100.0), 0, &doc);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].href, "https://a.example");
        assert_eq!(
            links[0].rects,
            vec![Rect::from_xywh(20.0, 10.0, 40.0, 20.0)]
        );
        assert_eq!(links[1].href, "https://b.example");
        assert_eq!(
            links[1].rects,
            vec![Rect::from_xywh(60.0, 10.0, 50.0, 20.0)]
        );
    }

    #[test]
    fn page_local_y_offsets_rects() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                400.0,
                vec![line_node(20.0, 220.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        let links = page_link_rects(&tree, &page(200.0, 100.0), 1, &doc);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].rects,
            vec![Rect::from_xywh(20.0, 20.0, 50.0, 20.0)]
        );
    }

    #[test]
    fn skips_lines_outside_page_window() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                600.0,
                vec![line_node(20.0, 500.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        assert!(page_link_rects(&tree, &page(0.0, 100.0), 0, &doc).is_empty());
    }

    #[test]
    fn link_hit_test_finds_containing_rect() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![line_node(20.0, 10.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        let hit = link_hit_test(&tree, &page(0.0, 100.0), 0, &doc, 40.0, 20.0);
        assert_eq!(hit.map(|l| l.href), Some("https://a.example".to_string()));
    }

    #[test]
    fn link_hit_test_misses_outside_rect() {
        let (doc, t) = doc! {
            root {
                paragraph {
                    t: text("link") [link(href: "https://a.example".into())]
                }
            }
        };
        let tree = LayoutTree {
            root: box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                200.0,
                40.0,
                vec![line_node(20.0, 10.0, 180.0, 20.0, vec![run(t, 0.0, 50.0)])],
            ),
        };
        assert!(link_hit_test(&tree, &page(0.0, 100.0), 0, &doc, 150.0, 20.0).is_none());
    }
}

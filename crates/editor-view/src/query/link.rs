use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Doc, Modifier, NodeId};
use serde::{Deserialize, Serialize};

use crate::paginate::LayoutContent;

use super::layout_index::LayoutIndex;

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
    layout_index: &LayoutIndex,
    page_idx: usize,
    doc: &Doc,
) -> Vec<LinkRect> {
    let Some(page) = layout_index.page(page_idx) else {
        return Vec::new();
    };
    let mut out: Vec<LinkRect> = Vec::new();
    for entry in layout_index.entries_on_page(page_idx) {
        if let Some(LayoutContent::Line(line)) = entry.content(layout_index) {
            for run in &line.glyph_runs {
                let Some(href) = link_href(doc, run.node_id) else {
                    continue;
                };
                let rect = Rect::from_xywh(
                    entry.rect.x + run.x,
                    entry.rect.y - page.y_start,
                    run.width,
                    entry.rect.height,
                );
                push_rect(&mut out, run.node_id, page_idx, href, rect);
            }
        }
    }
    out
}

pub(crate) fn link_hit_test(
    layout_index: &LayoutIndex,
    page_idx: usize,
    doc: &Doc,
    x: f32,
    page_y: f32,
) -> Option<LinkRect> {
    page_link_rects(layout_index, page_idx, doc)
        .into_iter()
        .find(|link| link.rects.iter().any(|r| r.contains(x, page_y)))
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
    use crate::page::LayoutPage;
    use crate::paginate::*;
    use crate::query::layout_index::LayoutIndex;
    use crate::style::*;

    fn page(y_start: f32, height: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + height, Size::new(800.0, height))
    }

    fn index(tree: LayoutTree, pages: Vec<LayoutPage>) -> LayoutIndex {
        LayoutIndex::new(tree, &pages)
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
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                attachment: None,
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
                is_phantom: false,
                content_edge_x: None,
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        let links = page_link_rects(&layout_index, 0, &doc);
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        assert!(page_link_rects(&layout_index, 0, &doc).is_empty());
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        let links = page_link_rects(&layout_index, 0, &doc);
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        let links = page_link_rects(&layout_index, 0, &doc);
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
        let layout_index = index(tree, vec![page(0.0, 100.0), page(200.0, 100.0)]);
        let links = page_link_rects(&layout_index, 1, &doc);
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        assert!(page_link_rects(&layout_index, 0, &doc).is_empty());
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        let hit = link_hit_test(&layout_index, 0, &doc, 40.0, 20.0);
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
        let layout_index = index(tree, vec![page(0.0, 100.0)]);
        assert!(link_hit_test(&layout_index, 0, &doc, 150.0, 20.0).is_none());
    }
}

use editor_common::Rect;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::paginate::types::LayoutContent;
use crate::query::layout_index::LayoutIndex;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LinkRect {
    pub page_idx: usize,
    pub href: String,
    pub rects: Vec<Rect>,
}

pub(crate) fn page_link_rects(layout_index: &LayoutIndex, page_idx: usize) -> Vec<LinkRect> {
    let Some(page) = layout_index.page(page_idx) else {
        return Vec::new();
    };
    let mut out: Vec<LinkRect> = Vec::new();
    for entry in layout_index.entries_on_page(page_idx) {
        if let Some(LayoutContent::Line(line)) = entry.content(layout_index) {
            for run in &line.glyph_runs {
                let Some(ref href) = run.link else {
                    continue;
                };
                let rect = Rect::from_xywh(
                    entry.rect.x + run.x,
                    entry.rect.y - page.y_start,
                    run.width,
                    entry.rect.height,
                );
                push_rect(&mut out, page_idx, href.clone(), rect);
            }
        }
    }
    out
}

pub(crate) fn link_hit_test(
    layout_index: &LayoutIndex,
    page_idx: usize,
    x: f32,
    page_y: f32,
) -> Option<LinkRect> {
    page_link_rects(layout_index, page_idx)
        .into_iter()
        .find(|link| link.rects.iter().any(|r| r.contains(x, page_y)))
}

fn push_rect(out: &mut Vec<LinkRect>, page_idx: usize, href: String, rect: Rect) {
    if let Some(existing) = out.iter_mut().find(|l| l.href == href) {
        existing.rects.push(rect);
        return;
    }
    out.push(LinkRect {
        page_idx,
        href,
        rects: vec![rect],
    });
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_crdt::Dot;

    use crate::glyph_run::GlyphRun;
    use crate::glyph_run::{GraphemeSpan, Synthesis, TextDecoration};
    use crate::page::LayoutPage;
    use crate::paginate::types::{LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree};
    use crate::query::layout_index::LayoutIndex;
    use crate::style::{Alignment, BorderMode, BoxStyle, Direction};

    use super::*;

    fn gs(advance: f32, codepoints: u8) -> GraphemeSpan {
        GraphemeSpan {
            advance,
            codepoints,
        }
    }

    fn run(
        offset_range: std::ops::Range<usize>,
        x: f32,
        graphemes: Vec<GraphemeSpan>,
        link: Option<&str>,
    ) -> GlyphRun {
        let width = graphemes.iter().map(|g| g.advance).sum();
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range,
            link: link.map(|s| s.to_string()),
            text: String::new(),
            x,
            width,
            graphemes,
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }

    fn line_node(
        node: Dot,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        glyph_runs: Vec<GlyphRun>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Line(LayoutLine {
                measured: std::sync::Arc::new(crate::measure::text::measure::MeasuredLine {
                    height: 0.0,
                    node,
                    baseline: h * 0.8,
                    ascent: h * 0.8,
                    descent: h * 0.2,
                    cursor_ascent: h * 0.8,
                    cursor_descent: h * 0.2,
                    glyph_runs,
                    ruby_annotations: vec![],
                    empty_caret_x: 0.0,
                    offset_range: None,
                    tab_gaps: vec![],
                    is_phantom: false,
                    content_edge_x: None,
                }),
            }),
        }
    }

    fn box_node(
        node: Dot,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node,
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

    fn page(y_start: f32, height: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + height, Size::new(800.0, height))
    }

    fn elem(peer: u64, clock: u64) -> Dot {
        Dot::new(peer, clock)
    }

    fn build_index(root: LayoutNode, pages: Vec<LayoutPage>) -> LayoutIndex {
        LayoutIndex::new(LayoutTree { root }, &pages)
    }

    #[test]
    fn page_link_rects_merge_same_href_exclude_none() {
        let root_id = elem(1, 0);
        let para_id = elem(1, 1);

        let run_a = run(0..3, 0.0, vec![gs(10.0, 1); 3], Some("https://a.example"));
        let run_b = run(3..6, 30.0, vec![gs(10.0, 1); 3], None);
        let run_c = run(6..9, 60.0, vec![gs(10.0, 1); 3], Some("https://a.example"));
        let run_d = run(9..12, 90.0, vec![gs(10.0, 1); 3], Some("https://b.example"));

        let ln = line_node(
            para_id,
            20.0,
            10.0,
            180.0,
            20.0,
            vec![run_a, run_b, run_c, run_d],
        );
        let root = box_node(root_id, 0.0, 0.0, 200.0, 40.0, vec![ln]);
        let index = build_index(root, vec![page(0.0, 100.0)]);

        let links = page_link_rects(&index, 0);

        assert_eq!(links.len(), 2, "must have exactly 2 distinct href entries");

        let link_a = links
            .iter()
            .find(|l| l.href == "https://a.example")
            .expect("must have href a");
        assert_eq!(
            link_a.rects.len(),
            2,
            "run A and C must merge into two rects"
        );
        assert_eq!(
            link_a.rects[0],
            Rect::from_xywh(20.0, 10.0, 30.0, 20.0),
            "run A rect"
        );
        assert_eq!(
            link_a.rects[1],
            Rect::from_xywh(80.0, 10.0, 30.0, 20.0),
            "run C rect"
        );

        let link_b = links
            .iter()
            .find(|l| l.href == "https://b.example")
            .expect("must have href b");
        assert_eq!(link_b.rects.len(), 1);
        assert_eq!(
            link_b.rects[0],
            Rect::from_xywh(110.0, 10.0, 30.0, 20.0),
            "run D rect"
        );
    }

    #[test]
    fn link_hit_test_hit_and_miss() {
        let root_id = elem(1, 0);
        let para_id = elem(1, 1);

        let run_a = run(0..5, 0.0, vec![gs(10.0, 1); 5], Some("https://a.example"));
        let run_b = run(5..10, 50.0, vec![gs(10.0, 1); 5], None);

        let ln = line_node(para_id, 20.0, 10.0, 180.0, 20.0, vec![run_a, run_b]);
        let root = box_node(root_id, 0.0, 0.0, 200.0, 40.0, vec![ln]);
        let index = build_index(root, vec![page(0.0, 100.0)]);

        let hit = link_hit_test(&index, 0, 30.0, 15.0);
        assert!(hit.is_some(), "x=30 (inside run A at page_y=15) must hit");
        assert_eq!(hit.unwrap().href, "https://a.example");

        let miss_none_run = link_hit_test(&index, 0, 85.0, 15.0);
        assert!(
            miss_none_run.is_none(),
            "x=85 (inside None run B) must miss"
        );

        let miss_outside = link_hit_test(&index, 0, 200.0, 15.0);
        assert!(miss_outside.is_none(), "x=200 (outside any run) must miss");
    }
}

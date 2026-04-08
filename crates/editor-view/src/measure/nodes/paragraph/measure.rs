use std::sync::Arc;

use editor_common::{Alignment, EdgeInsets};
use editor_model::{Doc, Node, NodeRef, TextAlign};

use crate::measure::Measurer;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredLine, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Direction};

use super::extract::{LineHeightConfig, extract_lines};
use super::layout::build_layout;
use super::resolve::{resolve_paragraph_indent, resolve_text_style};
use super::strut::compute_strut;
use super::style_run::resolve_style_runs;
use super::text_run::collect_text_runs;

pub fn measure_paragraph(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
) -> MeasuredNode {
    let (text, runs) = collect_text_runs(doc, node);

    if text.is_empty() {
        return measure_empty_paragraph(measurer, node, width);
    }

    let base_style = resolve_text_style(node);
    let indent = resolve_paragraph_indent(node);
    let align = match node.node() {
        Node::Paragraph(p) => p.align,
        _ => TextAlign::Left,
    };

    let mut resource = measurer.resource.lock().unwrap();
    let strut = compute_strut(&mut resource, &base_style);
    let style_runs = resolve_style_runs(&text, &runs, &mut resource.font_registry);
    let layout = build_layout(&text, &style_runs, align, indent, width, &mut resource);
    let segmenters = Arc::clone(&resource.segmenters);
    drop(resource);

    let strut = strut.expect("strut layout should have one line and run");
    let grapheme_segmenter = &segmenters.grapheme;

    let old_lines = extract_lines(
        doc,
        &text,
        &layout,
        &style_runs,
        &runs,
        &strut,
        LineHeightConfig {
            line_height_ratio: base_style.line_height,
            base_font_size: base_style.font_size,
        },
        grapheme_segmenter,
    );

    let node_id = node.id();
    let alignment = text_align_to_alignment(align);

    let children: Vec<Arc<MeasuredNode>> = old_lines
        .into_iter()
        .map(|line| {
            let height = line.height;
            Arc::new(MeasuredNode {
                width,
                height,
                content: MeasuredContent::Line(MeasuredLine {
                    node_id,
                    baseline: line.baseline,
                    ascent: line.ascent,
                    descent: line.descent,
                    glyph_runs: line.glyph_runs,
                    text_indent: indent,
                }),
            })
        })
        .collect();

    let total_height: f32 = children.iter().map(|c| c.height).sum();

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id,
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment,
                scope: false,
                decorations: vec![],
            },
            children,
        }),
    }
}

fn measure_empty_paragraph(
    measurer: &mut Measurer,
    node: &NodeRef<'_>,
    width: f32,
) -> MeasuredNode {
    let base_style = resolve_text_style(node);
    let indent = resolve_paragraph_indent(node);

    let mut resource = measurer.resource.lock().unwrap();
    let strut = compute_strut(&mut resource, &base_style);
    drop(resource);

    let strut = strut.expect("strut layout should have one line and run");

    let height = (base_style.font_size * base_style.line_height).max(strut.ascent + strut.descent);
    let node_id = node.id();

    let line = Arc::new(MeasuredNode {
        width,
        height,
        content: MeasuredContent::Line(MeasuredLine {
            node_id,
            baseline: strut.ascent,
            ascent: strut.ascent,
            descent: strut.descent,
            glyph_runs: vec![],
            text_indent: indent,
        }),
    });

    MeasuredNode {
        width,
        height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id,
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: Alignment::Start,
                scope: false,
                decorations: vec![],
            },
            children: vec![line],
        }),
    }
}

fn text_align_to_alignment(align: TextAlign) -> Alignment {
    match align {
        TextAlign::Left => Alignment::Start,
        TextAlign::Center => Alignment::Center,
        TextAlign::Right => Alignment::End,
        TextAlign::Justify => Alignment::Start,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use crate::measure::Measurer;
    use crate::measure::*;
    use crate::view_state::ViewState;

    #[test]
    fn paragraph_produces_box_with_lines() {
        let (doc, p1) = doc! { root { p1: paragraph { text("Hello") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        match &m.content {
            MeasuredContent::Box(b) => {
                assert!(!b.children.is_empty());
                assert!(matches!(b.children[0].content, MeasuredContent::Line(_)));
            }
            _ => panic!("expected Box"),
        }
        assert!(m.height > 0.0);
    }

    #[test]
    fn empty_paragraph_has_strut_height() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        assert!(m.height > 0.0);
        match &m.content {
            MeasuredContent::Box(b) => {
                assert_eq!(b.children.len(), 1);
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn paragraph_multiple_styled_runs() {
        let (doc, p1) =
            doc! { root { p1: paragraph { text("normal") text("bold") [font_size(2400)] } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        assert!(matches!(&m.content, MeasuredContent::Box(_)));
        assert!(m.height > 0.0);
    }

    #[test]
    fn bold_middle_text_produces_three_glyph_runs() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("Hello, ")
                    text("World") [bold]
                    text("!")
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };

        let mut all_runs = vec![];
        for child in &b.children {
            let MeasuredContent::Line(l) = &child.content else {
                panic!("expected Line")
            };
            all_runs.extend(l.glyph_runs.iter());
        }

        assert_eq!(all_runs.len(), 3);
        assert!(!all_runs[0].synthesis.embolden);
        assert!(all_runs[1].synthesis.embolden);
        assert!(!all_runs[2].synthesis.embolden);
        assert_eq!(all_runs[0].text, "Hello, ");
        assert_eq!(all_runs[1].text, "World");
        assert_eq!(all_runs[2].text, "!");
    }
}

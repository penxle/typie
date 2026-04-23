use std::sync::Arc;

use editor_model::{Alignment, Doc, NodeRef};

use crate::measure::{MeasuredContent, MeasuredLine, MeasuredNode, Measurer};

use super::extract::{LineHeightConfig, extract_lines};
use super::layout::build_layout;
use super::resolve::resolve_text_style;
use super::strut::compute_strut;
use super::style_run::resolve_style_runs;
use super::text_run::collect_text_runs;

pub fn measure_inline_text(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    align: Alignment,
    indent: f32,
) -> (Vec<Arc<MeasuredNode>>, f32) {
    let (text, runs) = collect_text_runs(doc, node);
    let node_id = node.id();
    let base_style = resolve_text_style(node);

    let mut resource = measurer.resource.lock().unwrap();
    let strut = compute_strut(&mut resource, &base_style)
        .expect("strut layout should have one line and run");

    if text.is_empty() {
        drop(resource);
        let ascent = strut.ascent;
        let descent = strut.descent;
        let content_height = ascent + descent;
        let line_box_height = (base_style.font_size * base_style.line_height).max(content_height);
        let leading = (line_box_height - content_height).max(0.0);
        let baseline = leading / 2.0 + ascent;
        let line = Arc::new(MeasuredNode {
            width,
            height: line_box_height,
            content: MeasuredContent::Line(MeasuredLine {
                node_id,
                baseline,
                ascent,
                descent,
                cursor_ascent: strut.ascent,
                cursor_descent: strut.descent,
                glyph_runs: vec![],
                text_indent: indent,
            }),
        });
        return (vec![line], line_box_height);
    }

    let style_runs = resolve_style_runs(&text, &runs, &mut resource.font_registry);
    let layout = build_layout(&text, &style_runs, align, indent, width, &mut resource);
    let segmenters = Arc::clone(&resource.segmenters);
    drop(resource);

    let lines = extract_lines(
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
        &segmenters.grapheme,
    );

    let cursor_ascent = strut.ascent;
    let cursor_descent = strut.descent;
    let children: Vec<Arc<MeasuredNode>> = lines
        .into_iter()
        .map(|line| {
            Arc::new(MeasuredNode {
                width,
                height: line.height,
                content: MeasuredContent::Line(MeasuredLine {
                    node_id,
                    baseline: line.baseline,
                    ascent: line.ascent,
                    descent: line.descent,
                    cursor_ascent,
                    cursor_descent,
                    glyph_runs: line.glyph_runs,
                    text_indent: indent,
                }),
            })
        })
        .collect();

    let total_height: f32 = children.iter().map(|c| c.height).sum();
    (children, total_height)
}

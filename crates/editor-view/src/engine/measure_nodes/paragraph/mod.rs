mod layout_builder;
mod line_extraction;
mod style_run;
mod text_run;

pub use layout_builder::*;
pub use line_extraction::*;
pub use style_run::*;
pub use text_run::*;

use editor_common::{Alignment, Size};
use editor_model::{Doc, Node, NodeRef, TextAlign};

use crate::engine::LayoutEngine;
use crate::engine::resolve::{resolve_gap_after, resolve_paragraph_indent, resolve_text_style};
use crate::measure::{MeasuredContent, Measurement};
use crate::strut::measure_strut;

pub fn measure_paragraph(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
) -> Measurement {
    let (text, runs) = collect_text_runs(doc, node);

    if text.is_empty() {
        return empty_paragraph_measurement(engine, node, width);
    }

    let base_style = resolve_text_style(node);
    let indent = resolve_paragraph_indent(node);
    let align = match node.node() {
        Node::Paragraph(p) => p.align,
        _ => TextAlign::Left,
    };

    let mut resource = engine.resource.lock().unwrap();

    let strut = measure_strut(&mut resource, &base_style);
    let style_runs = resolve_style_runs(&text, &runs, &mut resource.font_registry);
    let layout = build_parley_layout(&text, &style_runs, align, indent, width, &mut resource);

    let lines = extract_measured_lines(
        doc,
        &text,
        &layout,
        &style_runs,
        &strut,
        base_style.line_height,
        base_style.font_size,
    );

    drop(resource);
    let height: f32 = lines.iter().map(|l| l.height).sum();

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::TextBlock { lines },
    }
}

fn empty_paragraph_measurement(
    engine: &mut LayoutEngine,
    node: &NodeRef<'_>,
    width: f32,
) -> Measurement {
    let base_style = resolve_text_style(node);

    let mut resource = engine.resource.lock().unwrap();
    let strut = measure_strut(&mut resource, &base_style);
    drop(resource);

    let height = (base_style.font_size * base_style.line_height).max(strut.ascent + strut.descent);

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        alignment: Alignment::Start,
        content: MeasuredContent::TextBlock { lines: vec![] },
    }
}

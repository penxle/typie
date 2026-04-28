use std::sync::Arc;

use editor_model::{Alignment, Doc, NodeRef};

use crate::measure::{MeasuredContent, MeasuredLine, MeasuredNode, Measurer};
use crate::view_state::ViewState;

use super::extract::{LineHeightConfig, extract_lines};
use super::layout::build_layout;
use super::resolve::{apply_pending_to_style, resolve_text_style};
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
    view_state: &ViewState,
) -> (Vec<Arc<MeasuredNode>>, f32) {
    let (text, runs) = collect_text_runs(doc, node);
    let node_id = node.id();
    let mut base_style = resolve_text_style(node);

    if text.is_empty() {
        if let Some(ps) = &view_state.pending_style
            && ps.node_id == node_id
        {
            apply_pending_to_style(&mut base_style, &ps.modifiers);
        }

        let mut resource = measurer.resource.lock().unwrap();
        let strut = compute_strut(&mut resource, &base_style)
            .expect("strut layout should have one line and run");
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

    let mut resource = measurer.resource.lock().unwrap();
    let strut = compute_strut(&mut resource, &base_style)
        .expect("strut layout should have one line and run");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::Measurer;
    use crate::view_state::{PendingStyle, ViewState};
    use editor_macros::doc;
    use editor_model::Modifier;
    use editor_state::PendingModifier;

    fn measured_line_height(
        measurer: &mut Measurer,
        doc: &editor_model::Doc,
        p: editor_model::NodeId,
        vs: &ViewState,
    ) -> f32 {
        let m = measurer.measure(doc, p, 400.0, vs);
        match &m.content {
            MeasuredContent::Box(b) => b.children[0].height,
            _ => panic!("expected box"),
        }
    }

    #[test]
    fn empty_paragraph_applies_pending_font_size() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut measurer = Measurer::new_test();

        let vs = ViewState::new();
        let baseline_h = measured_line_height(&mut measurer, &doc, p1, &vs);

        measurer.clear_cache();
        let mut vs2 = ViewState::new();
        vs2.pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        let pending_h = measured_line_height(&mut measurer, &doc, p1, &vs2);

        assert!(
            pending_h > baseline_h,
            "line height should grow (baseline={baseline_h}, pending={pending_h})",
        );
    }

    #[test]
    fn empty_paragraph_with_mismatched_pending_node_id_unchanged() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut measurer = Measurer::new_test();

        let vs_base = ViewState::new();
        let baseline_h = measured_line_height(&mut measurer, &doc, p1, &vs_base);

        measurer.clear_cache();
        let mut vs = ViewState::new();
        vs.pending_style = Some(PendingStyle {
            node_id: editor_model::NodeId::new(),
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        let h = measured_line_height(&mut measurer, &doc, p1, &vs);

        assert!((h - baseline_h).abs() < 0.01);
    }

    #[test]
    fn non_empty_paragraph_ignores_pending_style() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hello") } } };
        let mut measurer = Measurer::new_test();

        let vs_base = ViewState::new();
        let baseline_h = measured_line_height(&mut measurer, &doc, p1, &vs_base);

        measurer.clear_cache();
        let mut vs = ViewState::new();
        vs.pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        let h = measured_line_height(&mut measurer, &doc, p1, &vs);

        assert!((h - baseline_h).abs() < 0.01);
    }

    #[test]
    fn empty_pending_matches_non_empty_same_font_size() {
        let (empty_doc, p1) = doc! { root { p1: paragraph } };
        let (non_empty_doc, p2) = doc! { root { p2: paragraph { text("a") [font_size(9600)] } } };

        let mut measurer = Measurer::new_test();

        let mut vs_pending = ViewState::new();
        vs_pending.pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        let empty_pending_h = measured_line_height(&mut measurer, &empty_doc, p1, &vs_pending);

        measurer.clear_cache();
        let vs_none = ViewState::new();
        let non_empty_h = measured_line_height(&mut measurer, &non_empty_doc, p2, &vs_none);

        assert!(
            (empty_pending_h - non_empty_h).abs() < 1.0,
            "line height mismatch between empty+pending and non-empty \
             (empty_pending={empty_pending_h}px, non_empty={non_empty_h}px) — \
             first keystroke would cause layout jump",
        );
    }
}

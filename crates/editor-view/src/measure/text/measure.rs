use std::sync::Arc;

use editor_model::{Alignment, Doc, Node, NodeRef};

use crate::measure::{MeasuredContent, MeasuredLine, MeasuredNode, Measurer};
use crate::view_state::ViewState;

use super::resolve::{apply_pending_to_style, resolve_text_style};
use super::strut::compute_strut;

fn build_strut_only_line(
    measurer: &mut Measurer,
    paragraph_id: editor_model::NodeId,
    base_style: &super::resolve::ResolvedTextStyle,
    width: f32,
    indent: f32,
    child_range: std::ops::Range<usize>,
) -> Arc<MeasuredNode> {
    let mut resource = measurer.resource.lock().unwrap();
    let strut = compute_strut(&mut resource, base_style)
        .expect("strut layout should have one line and run");
    drop(resource);
    let ascent = strut.ascent;
    let descent = strut.descent;
    let content_height = ascent + descent;
    let line_box_height = (base_style.font_size * base_style.line_height).max(content_height);
    let leading = (line_box_height - content_height).max(0.0);
    let baseline = leading / 2.0 + ascent;
    Arc::new(MeasuredNode {
        width,
        height: line_box_height,
        content: MeasuredContent::Line(MeasuredLine {
            node_id: paragraph_id,
            baseline,
            ascent,
            descent,
            cursor_ascent: strut.ascent,
            cursor_descent: strut.descent,
            glyph_runs: vec![],
            text_indent: indent,
            child_range: Some(child_range),
        }),
    })
}

pub fn measure_inline_text(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    align: Alignment,
    indent: f32,
    view_state: &ViewState,
) -> (Vec<Arc<MeasuredNode>>, f32) {
    let node_id = node.id();
    let mut base_style = resolve_text_style(node);

    let segments = super::segment::split_into_segments(node);

    // pending_style applies only when the paragraph contributes no text at all
    // (no children, or every child is either non-text or a zero-length text
    // node). Preserves the pre-segmentation contract — the previous code gated
    // on `collect_text_runs(...).0.is_empty()`, which covered both the
    // zero-children case AND paragraphs whose only children are empty text
    // nodes (a common transient state with IME / pending modifiers). Without
    // this widened check, `paragraph { text("") }` with a pending font size
    // change would not grow the placeholder strut, causing a layout jump on
    // the first keystroke.
    //
    // Hard-break-induced empty segments do NOT receive pending_style: e.g.
    // `paragraph { text("a") hard_break }` has a non-empty `text("a")` child,
    // so the gate is false and the trailing empty line uses base_style as-is.
    let paragraph_has_no_text = node.children().all(|c| match c.node() {
        Node::Text(t) => t.text.is_empty(),
        _ => true,
    });
    if paragraph_has_no_text
        && let Some(ps) = &view_state.pending_style
        && ps.node_id == node_id
    {
        apply_pending_to_style(&mut base_style, &ps.modifiers);
    }

    let mut lines: Vec<Arc<MeasuredNode>> = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        let seg_indent = if i == 0 { indent } else { 0.0 };
        lines.extend(measure_segment(
            measurer,
            doc,
            node,
            seg,
            width,
            align,
            seg_indent,
            &base_style,
        ));
    }

    let total_height: f32 = lines.iter().map(|l| l.height).sum();
    (lines, total_height)
}

fn measure_segment(
    measurer: &mut Measurer,
    doc: &editor_model::Doc,
    paragraph: &editor_model::NodeRef<'_>,
    seg: &super::segment::Segment<'_>,
    width: f32,
    align: editor_model::Alignment,
    indent: f32,
    base_style: &super::resolve::ResolvedTextStyle,
) -> Vec<Arc<MeasuredNode>> {
    let paragraph_id = paragraph.id();
    match seg {
        super::segment::Segment::Empty { child_range } => {
            vec![build_strut_only_line(
                measurer,
                paragraph_id,
                base_style,
                width,
                indent,
                child_range.clone(),
            )]
        }
        super::segment::Segment::Text {
            children,
            child_range,
        } => {
            let (text, runs) = super::text_run::collect_text_runs_for(children);

            // A Text segment that contributes no actual text (every child is an
            // empty text node, or every child was skipped from the inline flow —
            // e.g. trailing `PageBreak`) renders the same as Segment::Empty:
            // one strut-only line covering the segment's child_range. This
            // matches the pre-segmentation behavior where `measure_inline_text`
            // short-circuited on the collected text being empty.
            if text.is_empty() {
                return vec![build_strut_only_line(
                    measurer,
                    paragraph_id,
                    base_style,
                    width,
                    indent,
                    child_range.clone(),
                )];
            }

            let mut resource = measurer.resource.lock().unwrap();
            let strut = super::strut::compute_strut(&mut resource, base_style)
                .expect("strut layout should have one line and run");
            let style_runs =
                super::style_run::resolve_style_runs(&text, &runs, &mut resource.font_registry);
            let layout = super::layout::build_layout(
                &text,
                &style_runs,
                align,
                indent,
                width,
                &mut resource,
            );
            let segmenters = std::sync::Arc::clone(&resource.segmenters);
            drop(resource);

            let lines = super::extract::extract_lines(
                doc,
                &text,
                &layout,
                &style_runs,
                &runs,
                &strut,
                super::extract::LineHeightConfig {
                    line_height_ratio: base_style.line_height,
                    base_font_size: base_style.font_size,
                },
                &segmenters.grapheme,
            );

            let n = lines.len();
            let cursor_ascent = strut.ascent;
            let cursor_descent = strut.descent;
            lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| {
                    let line_child_range = if n == 1 {
                        Some(child_range.clone())
                    } else if i == 0 {
                        Some(child_range.start..child_range.start)
                    } else if i + 1 == n {
                        Some(child_range.end..child_range.end)
                    } else {
                        None
                    };
                    Arc::new(MeasuredNode {
                        width,
                        height: line.height,
                        content: MeasuredContent::Line(MeasuredLine {
                            node_id: paragraph_id,
                            baseline: line.baseline,
                            ascent: line.ascent,
                            descent: line.descent,
                            cursor_ascent,
                            cursor_descent,
                            glyph_runs: line.glyph_runs,
                            text_indent: indent,
                            child_range: line_child_range,
                        }),
                    })
                })
                .collect()
        }
    }
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
    fn single_text_segment_owns_full_range() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hi") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.child_range, Some(0..1));
    }

    #[test]
    fn empty_paragraph_owns_0_0() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.child_range, Some(0..0));
    }

    #[test]
    fn text_break_text_per_line_ranges() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hel") hard_break text("lo") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(a) = &b.children[0].content else {
            panic!()
        };
        let MeasuredContent::Line(c) = &b.children[1].content else {
            panic!()
        };
        assert_eq!(a.child_range, Some(0..2));
        assert_eq!(c.child_range, Some(2..3));
    }

    #[test]
    fn only_hard_break_two_empty_lines() {
        let (doc, p1) = doc! { root { p1: paragraph { hard_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert_eq!(b.children.len(), 2);
        let MeasuredContent::Line(a) = &b.children[0].content else {
            panic!()
        };
        let MeasuredContent::Line(b2) = &b.children[1].content else {
            panic!()
        };
        assert_eq!(a.child_range, Some(0..1));
        assert_eq!(b2.child_range, Some(1..1));
    }

    #[test]
    fn double_hard_break_middle_line_owns_range() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("a") hard_break hard_break text("b") } }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert_eq!(b.children.len(), 3);
        let ranges: Vec<_> = b
            .children
            .iter()
            .map(|c| match &c.content {
                MeasuredContent::Line(l) => l.child_range.clone(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(ranges, vec![Some(0..2), Some(2..3), Some(3..4)]);
    }

    #[test]
    fn empty_text_node_routes_to_strut_only_line() {
        let (doc, p1) = doc! { root { p1: paragraph { text("") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert_eq!(b.children.len(), 1);
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert!(l.glyph_runs.is_empty());
        assert_eq!(l.child_range, Some(0..1));
        assert!(b.children[0].height > 0.0);
    }

    #[test]
    fn page_break_only_paragraph_routes_to_strut_only_line() {
        let (doc, p1) = doc! { root { p1: paragraph { page_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert_eq!(b.children.len(), 2);
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert!(l.glyph_runs.is_empty());
        assert_eq!(l.child_range, Some(0..1));
        assert!(b.children[0].height > 0.0);
        assert!(matches!(b.children[1].content, MeasuredContent::PageBreak,));
    }

    #[test]
    fn empty_text_segment_after_hard_break_strut_only() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") hard_break text("") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert_eq!(b.children.len(), 2);
        let MeasuredContent::Line(trailing) = &b.children[1].content else {
            panic!()
        };
        assert!(trailing.glyph_runs.is_empty());
        assert_eq!(trailing.child_range, Some(2..3));
    }

    #[test]
    fn soft_wrap_multi_line_segment_child_range_assignment() {
        // A single Text segment that wraps to >= 3 visual lines must assign
        // child_range as: first → Some(seg.start..seg.start),
        // last  → Some(seg.end..seg.end), middle → None.
        let (doc, p1) = doc! {
            root { p1: paragraph { text("aaaaaaaaaaaaaaaaaaaaaaaa") } }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 30.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        assert!(
            b.children.len() >= 3,
            "expected >=3 wrapped lines, got {}",
            b.children.len()
        );
        let n = b.children.len();
        for (i, c) in b.children.iter().enumerate() {
            let MeasuredContent::Line(l) = &c.content else {
                panic!()
            };
            if i == 0 {
                assert_eq!(
                    l.child_range,
                    Some(0..0),
                    "first line owns leading boundary"
                );
            } else if i + 1 == n {
                assert_eq!(
                    l.child_range,
                    Some(1..1),
                    "last line owns trailing boundary"
                );
            } else {
                assert!(l.child_range.is_none(), "middle line owns no boundary");
            }
        }
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

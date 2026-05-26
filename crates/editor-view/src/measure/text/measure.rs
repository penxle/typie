use std::sync::Arc;

use editor_model::{Alignment, Doc, Node, NodeRef};

use crate::measure::{MeasuredContent, MeasuredLine, MeasuredNode, Measurer};
use crate::view_state::ViewState;

use super::resolve::{apply_pending_to_style, resolve_text_style};
use super::strut::compute_strut;

pub(crate) fn build_strut_only_line(
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
            ruby_annotations: vec![],
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

            // identify_ruby_groups inspects all paragraph children, not just this segment's slice.
            // group_offsets accumulates consumed char counts across lines for wrap distribution.
            let ruby_groups = super::ruby::identify_ruby_groups(paragraph);
            let mut group_offsets = vec![0usize; ruby_groups.len()];

            let mut resource = measurer.resource.lock().unwrap();
            let mut line_outputs: Vec<(
                super::extract::ExtractedLine,
                Vec<crate::glyph_run::RubyAnnotation>,
            )> = Vec::with_capacity(lines.len());
            for line in lines {
                let ruby_annotations = super::ruby::build_ruby_annotations_for_line(
                    &line,
                    width,
                    &ruby_groups,
                    &mut group_offsets,
                    &mut resource,
                );
                line_outputs.push((line, ruby_annotations));
            }
            drop(resource);

            let n = line_outputs.len();
            let cursor_ascent = strut.ascent;
            let cursor_descent = strut.descent;
            line_outputs
                .into_iter()
                .enumerate()
                .map(|(i, (line, ruby_annotations))| {
                    let max_ruby_ascent = ruby_annotations
                        .iter()
                        .map(|r| r.ascent)
                        .fold(0.0, f32::max);
                    let max_ruby_descent = ruby_annotations
                        .iter()
                        .map(|r| r.descent)
                        .fold(0.0, f32::max);
                    let extra_top = if ruby_annotations.is_empty() {
                        0.0
                    } else {
                        let required_top =
                            max_ruby_ascent + max_ruby_descent + super::ruby::RUBY_GAP;
                        let available_top = (line.baseline - line.ascent).max(0.0);
                        (required_top - available_top).max(0.0)
                    };

                    let new_ascent = line.ascent + extra_top;
                    let new_baseline = line.baseline + extra_top;
                    let new_height = line.height + extra_top;

                    let line_child_range = if n == 1 {
                        Some(child_range.clone())
                    } else if i == 0 {
                        Some(child_range.start..child_range.start)
                    } else if i + 1 == n {
                        Some(child_range.end..child_range.end)
                    } else {
                        None
                    };

                    // shift base glyph y to match the shifted baseline.
                    let glyph_runs = line
                        .glyph_runs
                        .into_iter()
                        .map(|mut r| {
                            if extra_top != 0.0 {
                                for g in &mut r.glyphs {
                                    g.y += extra_top;
                                }
                            }
                            r
                        })
                        .collect::<Vec<_>>();

                    // shift ruby glyph y by the same extra_top to keep them relative to the new baseline.
                    let ruby_annotations = ruby_annotations
                        .into_iter()
                        .map(|mut a| {
                            a.baseline_y += extra_top;
                            for g in &mut a.glyphs {
                                g.y += extra_top;
                            }
                            a
                        })
                        .collect::<Vec<_>>();

                    Arc::new(MeasuredNode {
                        width,
                        height: new_height,
                        content: MeasuredContent::Line(MeasuredLine {
                            node_id: paragraph_id,
                            baseline: new_baseline,
                            ascent: new_ascent,
                            descent: line.descent,
                            cursor_ascent,
                            cursor_descent,
                            glyph_runs,
                            ruby_annotations,
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
    use crate::glyph_run::RubyAnnotation;
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

    fn first_glyph_run(
        measurer: &mut Measurer,
        doc: &editor_model::Doc,
        p: editor_model::NodeId,
        vs: &ViewState,
    ) -> crate::glyph_run::GlyphRun {
        let m = measurer.measure(doc, p, 400.0, vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected box");
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!("expected line");
        };
        l.glyph_runs.first().cloned().expect("expected glyph run")
    }

    #[test]
    fn link_modifier_produces_blue_underlined_glyph_run() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph {
                    text("hello") [link(href: "https://example.com".into())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let run = first_glyph_run(&mut measurer, &doc, p1, &vs);
        assert_eq!(run.color, "text.blue");
        assert!(run.decoration.underline);
    }

    struct FirstLineMetrics {
        height: f32,
        baseline: f32,
        ascent: f32,
        ruby: Option<RubyAnnotation>,
    }

    fn measure_first_line(
        measurer: &mut Measurer,
        doc: &editor_model::Doc,
        p: editor_model::NodeId,
        vs: &ViewState,
    ) -> FirstLineMetrics {
        let m = measurer.measure(doc, p, 400.0, vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected box");
        };
        let child = &b.children[0];
        let height = child.height;
        let MeasuredContent::Line(l) = &child.content else {
            panic!("expected line");
        };
        FirstLineMetrics {
            height,
            baseline: l.baseline,
            ascent: l.ascent,
            ruby: l.ruby_annotations.first().cloned(),
        }
    }

    fn assert_ruby_inside_line(ann: &RubyAnnotation, line_height: f32) {
        let top = ann.baseline_y - ann.ascent;
        let bottom = ann.baseline_y + ann.descent;
        assert!(
            top >= -0.1,
            "ruby top must be inside line box (top={}, baseline_y={}, ascent={})",
            top,
            ann.baseline_y,
            ann.ascent
        );
        assert!(
            bottom <= line_height + 0.1,
            "ruby bottom must be inside line box (bottom={}, line_height={})",
            bottom,
            line_height
        );
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
    fn adjacent_same_ruby_text_one_annotation() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("굵게") [font_weight(700), ruby(text: "루비".to_string())]
                    text("보통")  [ruby(text: "루비".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(
            l.ruby_annotations.len(),
            1,
            "adjacent identical ruby runs must be merged"
        );
    }

    #[test]
    fn adjacent_different_ruby_two_annotations() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "a".to_string())]
                    text("B") [ruby(text: "b".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.ruby_annotations.len(), 2);
    }

    #[test]
    fn no_ruby_no_annotations() {
        let (d, p1) = doc! { root { p1: paragraph { text("hello") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert!(l.ruby_annotations.is_empty());
    }

    #[test]
    fn line_height_absorbs_ruby_when_sufficient() {
        let (d_plain, p_plain) = doc! {
            root {
                p: paragraph [line_height(500)] { text("ABCD") }
            }
        };
        let (d_ruby, p_ruby) = doc! {
            root {
                p: paragraph [line_height(500)] {
                    text("ABCD") [ruby(text: "xy".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let plain = measure_first_line(&mut measurer, &d_plain, p_plain, &vs);
        measurer.clear_cache();
        let ruby = measure_first_line(&mut measurer, &d_ruby, p_ruby, &vs);

        let ann = ruby.ruby.as_ref().expect("ruby annotation must exist");
        let required_top = ann.ascent + ann.descent + crate::measure::text::ruby::RUBY_GAP;
        let available_top = (plain.baseline - plain.ascent).max(0.0);

        assert!(
            available_top >= required_top,
            "test precondition violated: line-height(500) must provide enough half-leading \
             to fully absorb ruby (available_top={}, required_top={}). \
             Retune the line_height value for the current test font.",
            available_top,
            required_top
        );

        assert!(
            (ruby.height - plain.height).abs() < 0.1,
            "fully absorbed ruby line height must equal plain line height \
             (plain={}, ruby={})",
            plain.height,
            ruby.height
        );
        assert_ruby_inside_line(ann, ruby.height);
    }

    #[test]
    fn line_height_grows_partially_when_ruby_partially_fits() {
        let (d_plain, p_plain) = doc! {
            root {
                p: paragraph [line_height(180)] { text("ABCD") }
            }
        };
        let (d_ruby, p_ruby) = doc! {
            root {
                p: paragraph [line_height(180)] {
                    text("ABCD") [ruby(text: "xy".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let plain = measure_first_line(&mut measurer, &d_plain, p_plain, &vs);
        measurer.clear_cache();
        let ruby = measure_first_line(&mut measurer, &d_ruby, p_ruby, &vs);

        let ann = ruby.ruby.as_ref().expect("ruby annotation must exist");
        let required_top = ann.ascent + ann.descent + crate::measure::text::ruby::RUBY_GAP;
        let available_top = (plain.baseline - plain.ascent).max(0.0);

        assert!(
            available_top > 0.1 && available_top < required_top - 0.1,
            "test precondition violated: line-height(180) must produce a partial-fit \
             (0 < available_top < required_top). available_top={}, required_top={}. \
             Retune the line_height value for the current test font.",
            available_top,
            required_top
        );

        let expected_diff = required_top - available_top;
        let diff = ruby.height - plain.height;
        assert!(
            (diff - expected_diff).abs() < 0.1,
            "line height growth must equal exact shortfall \
             (diff={}, expected={}, required_top={}, available_top={})",
            diff,
            expected_diff,
            required_top,
            available_top
        );
        assert_ruby_inside_line(ann, ruby.height);
    }

    #[test]
    fn line_height_grows_fully_when_no_leading() {
        let (d_plain, p_plain) = doc! {
            root {
                p: paragraph [line_height(100)] { text("ABCD") }
            }
        };
        let (d_ruby, p_ruby) = doc! {
            root {
                p: paragraph [line_height(100)] {
                    text("ABCD") [ruby(text: "xy".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let plain = measure_first_line(&mut measurer, &d_plain, p_plain, &vs);
        measurer.clear_cache();
        let ruby = measure_first_line(&mut measurer, &d_ruby, p_ruby, &vs);

        let ann = ruby.ruby.as_ref().expect("ruby annotation must exist");
        let required_top = ann.ascent + ann.descent + crate::measure::text::ruby::RUBY_GAP;
        let available_top = (plain.baseline - plain.ascent).max(0.0);

        assert!(
            available_top <= 0.1,
            "test precondition violated: line-height(100) must produce ~zero half-leading. \
             available_top={}. Retune the line_height value for the current test font.",
            available_top
        );

        let diff = ruby.height - plain.height;
        assert!(
            (diff - required_top).abs() < 0.1,
            "line height must grow by full required_top when no leading is available \
             (diff={}, required_top={})",
            diff,
            required_top
        );
        assert_ruby_inside_line(ann, ruby.height);
    }

    #[test]
    fn line_height_grows_when_ruby_added() {
        let (d_plain, p_plain) = doc! { root { p1: paragraph { text("ABCD") } } };
        let (d_ruby, p_ruby) = doc! {
            root {
                p2: paragraph { text("ABCD") [ruby(text: "xy".to_string())] }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let plain = measure_first_line(&mut measurer, &d_plain, p_plain, &vs);
        measurer.clear_cache();
        let ruby = measure_first_line(&mut measurer, &d_ruby, p_ruby, &vs);

        let ann = ruby.ruby.as_ref().expect("ruby annotation must exist");
        let required_top = ann.ascent + ann.descent + crate::measure::text::ruby::RUBY_GAP;
        let available_top = (plain.baseline - plain.ascent).max(0.0);
        let expected_extra = (required_top - available_top).max(0.0);
        let diff = ruby.height - plain.height;

        assert!(
            available_top < required_top,
            "test precondition violated: default line-height must be in the partial-fit bucket \
             (available_top={}, required_top={})",
            available_top,
            required_top
        );

        assert!(
            (diff - expected_extra).abs() < 0.1,
            "line height increase must equal shortfall = max(0, required_top - available_top) \
             (diff={}, expected={}, required_top={}, available_top={}, plain_h={}, ruby_h={})",
            diff,
            expected_extra,
            required_top,
            available_top,
            plain.height,
            ruby.height
        );
        assert_ruby_inside_line(ann, ruby.height);
    }

    #[test]
    fn list_item_ruby_stays_inside_line() {
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph [line_height(300)] {
                            text("ABCD") [ruby(text: "xy".to_string())]
                        }
                    }
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let result = measurer.measure(&doc, li1, 300.0, &ViewState::new());
        let MeasuredContent::Box(b) = &result.content else {
            panic!("expected list-item box");
        };
        fn find_first_line(node: &MeasuredNode) -> Option<(&MeasuredLine, f32)> {
            match &node.content {
                MeasuredContent::Line(l) => Some((l, node.height)),
                MeasuredContent::Box(b) => b.children.iter().find_map(|c| find_first_line(c)),
                _ => None,
            }
        }
        let (line, line_height) =
            find_first_line(&b.children[0]).expect("list-item must contain a measured line");
        assert_eq!(line.ruby_annotations.len(), 1);
        let ann = &line.ruby_annotations[0];

        assert_ruby_inside_line(ann, line_height);
    }

    #[test]
    fn cursor_ascent_unaffected_by_ruby() {
        let (d1, p1) = doc! { root { p1: paragraph { text("ABCD") } } };
        let (d2, p2) = doc! {
            root {
                p2: paragraph { text("ABCD") [ruby(text: "xy".to_string())] }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let line_of = |measurer: &mut Measurer, doc: &editor_model::Doc, p| {
            let m = measurer.measure(doc, p, 400.0, &vs);
            match &m.content {
                MeasuredContent::Box(b) => match &b.children[0].content {
                    MeasuredContent::Line(l) => (l.cursor_ascent, l.cursor_descent),
                    _ => panic!(),
                },
                _ => panic!(),
            }
        };
        let (ca1, cd1) = line_of(&mut measurer, &d1, p1);
        measurer.clear_cache();
        let (ca2, cd2) = line_of(&mut measurer, &d2, p2);
        assert!((ca1 - ca2).abs() < 0.01);
        assert!((cd1 - cd2).abs() < 0.01);
    }

    #[test]
    fn ruby_wraps_distributes_across_lines() {
        // narrow width forces ruby to wrap across two lines.
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("AAAA AAAA AAAA AAAA") [ruby(text: "한글영문".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 50.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let mut ruby_lines_with_glyphs = 0;
        for c in &b.children {
            if let MeasuredContent::Line(l) = &c.content {
                for ann in &l.ruby_annotations {
                    if !ann.glyphs.is_empty() {
                        ruby_lines_with_glyphs += 1;
                        break;
                    }
                }
            }
        }
        assert!(
            ruby_lines_with_glyphs >= 2,
            "ruby must distribute across at least two lines with non-empty annotations when wrapped (got {})",
            ruby_lines_with_glyphs
        );
    }

    #[test]
    fn ruby_positioned_above_base_visual_top() {
        // post-correction ruby must sit above the base glyph's visual top.
        let (d, p1) = doc! {
            root { p1: paragraph { text("ABCD") [ruby(text: "xy".to_string())] } }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.ruby_annotations.len(), 1);
        let ann = &l.ruby_annotations[0];

        let base_glyph_y = l
            .glyph_runs
            .first()
            .and_then(|r| r.glyphs.first())
            .map(|g| g.y)
            .expect("at least one base glyph must be present");

        let ruby_descent_bottom_y = ann.baseline_y + ann.descent;
        assert!(
            ruby_descent_bottom_y < base_glyph_y,
            "ruby descent bottom must be above base glyph visual y (ruby_bottom={}, base_y={})",
            ruby_descent_bottom_y,
            base_glyph_y
        );
    }

    #[test]
    fn ruby_baseline_y_shared_within_line() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "x".to_string())]
                    text("B") [font_size(2400), ruby(text: "y".to_string())]
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&d, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!()
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!()
        };
        assert_eq!(l.ruby_annotations.len(), 2);
        assert!(
            (l.ruby_annotations[0].baseline_y - l.ruby_annotations[1].baseline_y).abs() < 0.01,
            "all ruby annotations on a line must share a common baseline_y"
        );
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

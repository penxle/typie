use std::sync::Arc;

use editor_common::EdgeInsets;

use crate::style::Alignment as LayoutAlignment;
use editor_model::{Alignment, Doc, Modifier, ModifierType, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::text::measure::measure_inline_text;
use crate::measure::text::resolve::resolve_paragraph_indent;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode, PageBreakPolicy};
use crate::style::{BorderMode, BoxStyle, Direction};
use crate::view_state::ViewState;

pub fn measure_paragraph(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let align = match node.own_modifier(ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => *value,
        _ => Alignment::default(),
    };

    let indent = match align {
        Alignment::Left | Alignment::Justify => resolve_paragraph_indent(node),
        Alignment::Center | Alignment::Right => 0.0,
    };

    let (mut children, total_height) =
        measure_inline_text(measurer, doc, node, width, align, indent, view_state);

    // The segmenter skips `page_break` from the inline flow; emit a marker
    // here so the paginator's forced-break branch can see it.
    let has_trailing_page_break = node
        .last_child()
        .is_some_and(|c| matches!(c.node(), Node::PageBreak(_)));
    if has_trailing_page_break {
        children.push(Arc::new(MeasuredNode {
            width: 0.0,
            height: 0.0,
            content: MeasuredContent::PageBreak,
        }));
    }

    let alignment = align_to_layout(align);

    MeasuredNode {
        width,
        height: total_height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment,
                scope: false,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children,
            page_break_policy: PageBreakPolicy::Auto,
        }),
    }
}

fn align_to_layout(align: Alignment) -> LayoutAlignment {
    match align {
        Alignment::Left => LayoutAlignment::Start,
        Alignment::Center => LayoutAlignment::Center,
        Alignment::Right => LayoutAlignment::End,
        Alignment::Justify => LayoutAlignment::Start,
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
    fn paragraph_indent_applies_on_left_alignment() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                p1: paragraph { text("hi") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!("expected Line")
        };
        let first_x = l.glyph_runs.first().map(|r| r.x).unwrap_or(l.empty_caret_x);
        assert!(
            first_x > 1.0,
            "left-aligned paragraph_indent must push first run rightward (first_x={first_x})",
        );
    }

    #[test]
    fn paragraph_indent_suppressed_on_right_alignment() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                p1: paragraph [alignment(Alignment::Right)] { text("hi") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!("expected Line")
        };
        let last = l.glyph_runs.last().expect("expected glyph run");
        let trailing_gap = b.children[0].width - (last.x + last.width);
        assert!(
            trailing_gap.abs() < 1.0,
            "right-aligned paragraph must hug the right edge regardless of paragraph_indent \
             (trailing_gap={trailing_gap}, line_width={})",
            b.children[0].width,
        );
    }

    #[test]
    fn paragraph_indent_suppressed_on_center_alignment() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                p1: paragraph [alignment(Alignment::Center)] { text("hi") }
            }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        let MeasuredContent::Line(l) = &b.children[0].content else {
            panic!("expected Line")
        };
        let first = l.glyph_runs.first().expect("expected glyph run");
        let last = l.glyph_runs.last().expect("expected glyph run");
        let left_gap = first.x;
        let right_gap = b.children[0].width - (last.x + last.width);
        assert!(
            (left_gap - right_gap).abs() < 1.0,
            "center-aligned paragraph must be symmetric around the center regardless of \
             paragraph_indent (left_gap={left_gap}, right_gap={right_gap})",
        );
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

    #[test]
    fn paragraph_with_hard_break_produces_two_lines() {
        let (doc, p1, _t1) = doc! {
            root { p1: paragraph { text("hel") hard_break t1: text("lo") } }
        };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert_eq!(b.children.len(), 2);
        for c in &b.children {
            assert!(matches!(c.content, MeasuredContent::Line(_)));
        }
        assert!(m.height > 0.0);
    }

    #[test]
    fn trailing_hard_break_produces_empty_trailing_line() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") hard_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert_eq!(b.children.len(), 2);
        let MeasuredContent::Line(trailing) = &b.children[1].content else {
            panic!("expected Line")
        };
        assert!(trailing.glyph_runs.is_empty());
        assert!(b.children[1].height > 0.0);
        assert_eq!(trailing.child_range, Some(2..2));
    }

    #[test]
    fn paragraph_with_trailing_page_break_emits_marker_child() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") page_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert_eq!(b.children.len(), 2);
        assert!(
            matches!(b.children[0].content, MeasuredContent::Line(_)),
            "first child must be the text line",
        );
        assert!(
            matches!(b.children[1].content, MeasuredContent::PageBreak),
            "second child must be the PageBreak marker",
        );
        assert_eq!(b.children[1].width, 0.0);
        assert_eq!(b.children[1].height, 0.0);
    }

    #[test]
    fn page_break_only_paragraph_emits_strut_line_then_marker() {
        let (doc, p1) = doc! { root { p1: paragraph { page_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert_eq!(b.children.len(), 2);
        let MeasuredContent::Line(strut) = &b.children[0].content else {
            panic!("expected Line as first child")
        };
        assert!(
            strut.glyph_runs.is_empty(),
            "first child must be a strut-only line for caret anchoring",
        );
        assert!(
            b.children[0].height > 0.0,
            "strut-only line must have non-zero height so the caret has vertical room",
        );
        assert!(
            matches!(b.children[1].content, MeasuredContent::PageBreak),
            "second child must be the PageBreak marker",
        );
    }

    #[test]
    fn paragraph_without_page_break_has_no_marker() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert!(
            b.children
                .iter()
                .all(|c| !matches!(c.content, MeasuredContent::PageBreak)),
            "paragraph without a trailing page_break must not emit a marker",
        );
    }

    #[test]
    fn paragraph_with_trailing_hard_break_has_no_page_break_marker() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") hard_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert!(
            b.children
                .iter()
                .all(|c| !matches!(c.content, MeasuredContent::PageBreak)),
            "hard_break and page_break must not be conflated — trailing hard_break emits no marker",
        );
    }

    #[test]
    fn paragraph_with_hard_break_then_page_break_emits_marker_after_lines() {
        let (doc, p1) = doc! { root { p1: paragraph { text("a") hard_break page_break } } };
        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();
        let m = measurer.measure(&doc, p1, 400.0, &vs);
        let MeasuredContent::Box(b) = &m.content else {
            panic!("expected Box")
        };
        assert_eq!(b.children.len(), 3);
        assert!(matches!(b.children[0].content, MeasuredContent::Line(_)));
        assert!(matches!(b.children[1].content, MeasuredContent::Line(_)));
        assert!(
            matches!(b.children[2].content, MeasuredContent::PageBreak),
            "marker must sit at the very end, after the empty line produced by the hard_break",
        );
    }
}

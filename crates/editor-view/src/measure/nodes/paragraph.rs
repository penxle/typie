use editor_common::EdgeInsets;

use crate::style::Alignment as LayoutAlignment;
use editor_model::{Alignment, Doc, Modifier, NodeRef};

use crate::measure::Measurer;
use crate::measure::text::measure::measure_inline_text;
use crate::measure::text::resolve::resolve_paragraph_indent;
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode};
use crate::style::{BorderMode, BoxStyle, Direction};
use crate::view_state::ViewState;

pub fn measure_paragraph(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let indent = resolve_paragraph_indent(node);
    let align = node
        .modifiers()
        .find_map(|m| match m {
            Modifier::Alignment { value } => Some(*value),
            _ => None,
        })
        .unwrap_or_default();

    let (children, total_height) =
        measure_inline_text(measurer, doc, node, width, align, indent, view_state);
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
            },
            children,
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

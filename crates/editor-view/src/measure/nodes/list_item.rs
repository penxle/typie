use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{Doc, Node, NodeRef};

use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::{MeasuredContent, MeasuredNode};
use crate::style::{Decoration, DecorationData};
use crate::view_state::ViewState;

const LIST_ITEM_MARKER_WIDTH: f32 = 20.0;
const LIST_ITEM_MARKER_GAP: f32 = 8.0;

fn resolve_marker_data(node: &NodeRef<'_>) -> DecorationData {
    let parent = match node.parent() {
        Some(p) => p,
        None => return DecorationData::Text("\u{2022}".to_string()),
    };

    match parent.node() {
        Node::OrderedList(_) => {
            let index = node.index().unwrap_or(0);
            DecorationData::Number((index + 1) as f32)
        }
        _ => DecorationData::Text("\u{2022}".to_string()),
    }
}

pub fn measure_list_item(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let padding = EdgeInsets {
        left: LIST_ITEM_MARKER_WIDTH + LIST_ITEM_MARKER_GAP,
        ..EdgeInsets::ZERO
    };

    let mut measured = layout_padded(
        measurer,
        doc,
        node,
        width,
        view_state,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            scope: false,
            alignment: Alignment::Start,
        },
    );

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: LIST_ITEM_MARKER_WIDTH,
                height: LIST_ITEM_MARKER_WIDTH,
            },
            data: resolve_marker_data(node),
        });
    }

    measured
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn applies_left_indent() {
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert_eq!(b.style.padding.left, 28.0);
        assert_eq!(result.width, 300.0);
    }

    #[test]
    fn ordered_list_uses_number() {
        let (doc, li1) = doc! {
            root {
                ordered_list {
                    li1: list_item {
                        paragraph { text("first") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert!(
            matches!(b.style.decorations[0].data, DecorationData::Number(n) if (n - 1.0).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn bullet_list_uses_bullet() {
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph { text("item") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert!(matches!(&b.style.decorations[0].data, DecorationData::Text(s) if s == "\u{2022}"));
    }
}

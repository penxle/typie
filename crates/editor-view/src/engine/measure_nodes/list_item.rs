use editor_common::{Alignment, EdgeInsets, Rect};
use editor_model::{Doc, Node, NodeRef};

use super::super::LayoutEngine;
use super::container::measure_padded_container;
use crate::fragment::PlaceholderData;
use crate::measure::*;
use crate::view_state::ViewState;

const LIST_ITEM_MARKER_WIDTH: f32 = 20.0;
const LIST_ITEM_MARKER_GAP: f32 = 8.0;

fn resolve_marker_data(node: &NodeRef<'_>) -> PlaceholderData {
    let parent = match node.parent() {
        Some(p) => p,
        None => return PlaceholderData::Text("\u{2022}".to_string()),
    };

    match parent.node() {
        Node::OrderedList(_) => {
            let index = node.index().unwrap_or(0);
            PlaceholderData::Number((index + 1) as f64)
        }
        _ => PlaceholderData::Text("\u{2022}".to_string()),
    }
}

pub fn measure_list_item(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let padding = EdgeInsets {
        left: LIST_ITEM_MARKER_WIDTH + LIST_ITEM_MARKER_GAP,
        ..EdgeInsets::ZERO
    };

    let mut measurement = measure_padded_container(
        engine,
        doc,
        node,
        width,
        view_state,
        padding,
        EdgeInsets::ZERO,
        false,
        Alignment::Start,
    );

    if let MeasuredContent::Container(ref mut content) = measurement.content {
        content.placeholders.push(MeasuredPlaceholder {
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

    measurement
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::engine::LayoutEngine;

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
        let mut engine = LayoutEngine::new_test();
        let result = measure_list_item(&mut engine, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Container(ContainerContent { padding, .. }) = &result.content else {
            panic!()
        };

        assert_eq!(padding.left, 28.0);
        assert_eq!(result.size.width, 300.0);
    }
}

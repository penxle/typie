use editor_common::{Alignment, EdgeInsets, Size};
use editor_model::{Doc, Node, NodeRef};

use super::super::LayoutEngine;
use super::super::resolve::resolve_gap_after;
use crate::measure::*;
use crate::view_state::ViewState;

pub(super) fn measure_padded_container(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
    padding: EdgeInsets,
    border: EdgeInsets,
    scope: bool,
    alignment: Alignment,
) -> Measurement {
    let content_width = width - padding.left - padding.right - border.left - border.right;

    let children: Vec<ChildMeasurement> = node
        .children()
        .map(|child| {
            let m = engine.measure(doc, child.id(), content_width, view_state);
            ChildMeasurement {
                node_id: child.id(),
                measurement: m,
            }
        })
        .collect();

    let children_height: f32 = children.iter().map(|c| c.measurement.size.height).sum();
    let height = border.top + padding.top + children_height + padding.bottom + border.bottom;

    Measurement {
        size: Size { width, height },
        gap_after: resolve_gap_after(node),
        content: MeasuredContent::Container(ContainerContent {
            children,
            scope,
            direction: LayoutDirection::Vertical,
            padding,
            border,
            border_mode: BorderMode::Separate,
            placeholders: vec![],
        }),
        alignment,
    }
}

pub fn measure_default_container(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let children: Vec<ChildMeasurement> = node
        .children()
        .map(|child| {
            let m = engine.measure(doc, child.id(), width, view_state);
            ChildMeasurement {
                node_id: child.id(),
                measurement: m,
            }
        })
        .collect();

    let height: f32 = children.iter().map(|c| c.measurement.size.height).sum();

    let direction = if matches!(node.node(), Node::TableRow(_)) {
        LayoutDirection::Horizontal
    } else {
        LayoutDirection::Vertical
    };

    let gap_after = if matches!(node.node(), Node::Root(_) | Node::Text(_)) {
        0.0
    } else {
        resolve_gap_after(node)
    };

    Measurement {
        size: Size {
            width,
            height: height.max(0.0),
        },
        gap_after,
        content: MeasuredContent::Container(ContainerContent {
            children,
            scope: false,
            direction,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            placeholders: vec![],
        }),
        alignment: Alignment::Start,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::engine::LayoutEngine;

    #[test]
    fn sums_children() {
        let (doc, p1) = doc! {
            root {
                p1: paragraph { text("hello") }
            }
        };

        let node = doc.node(p1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_default_container(&mut engine, &doc, &node, 300.0, &ViewState::new());

        assert!(matches!(
            result.content,
            MeasuredContent::Container(ContainerContent {
                direction: LayoutDirection::Vertical,
                ..
            })
        ));
        assert_eq!(result.alignment, Alignment::Start);
        assert_eq!(result.size.width, 300.0);
    }

    #[test]
    fn resolves_gap_after() {
        let (doc, p1) = doc! {
            root [block_gap(200)] {
                p1: paragraph
            }
        };

        let node = doc.node(p1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_default_container(&mut engine, &doc, &node, 300.0, &ViewState::new());

        assert_eq!(result.gap_after, 32.0);
    }
}

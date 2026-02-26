use crate::layout::elements::list_marker::ListMarkerElement;
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ListItemNode {}

impl NodeHtmlCodec for ListItemNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("li").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("li", |_| {
            Some(Node::ListItem(ListItemNode {}))
        })]
    }
}

impl Layout for ListItemNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        const MARKER_WIDTH: f32 = 20.0;
        const MARKER_GAP: f32 = 8.0;
        const CONTENT_OFFSET: f32 = MARKER_WIDTH + MARKER_GAP;

        let child_constraints = BoxConstraints::new(
            (constraints.min_width - CONTENT_OFFSET).max(0.0),
            (constraints.max_width - CONTENT_OFFSET).max(0.0),
            constraints.min_height,
            constraints.max_height,
        );

        let children: Vec<_> = ctx.node.children().collect();
        let mut child_nodes = Vec::new();
        let mut y_offset = 0.0;
        let mut max_width = 0.0f32;

        let marker_type = if let Some(parent) = ctx.node.parent() {
            match parent.node() {
                Some(crate::model::Node::OrderedList(_)) => {
                    let index = ctx.node.index().unwrap_or(0) + 1;
                    crate::layout::elements::list_marker::ListMarkerType::Ordered(index)
                }
                _ => crate::layout::elements::list_marker::ListMarkerType::Bullet,
            }
        } else {
            crate::layout::elements::list_marker::ListMarkerType::Bullet
        };

        for (idx, child) in children.iter().enumerate() {
            let child_layout = ctx.layout(child, child_constraints);

            let is_last = idx == children.len() - 1;
            let child_height = child_layout.size.height;
            let child_width = child_layout.size.width;

            child_nodes.push(PositionedNode {
                position: Point::new(CONTENT_OFFSET, y_offset),
                node: child_layout,
            });

            y_offset += child_height + (if is_last { 0.0 } else { 0.0 });
            max_width = max_width.max(child_width);
        }

        let Some((baseline, line_mid, marker_height)) = child_nodes.get(0).and_then(|positioned| {
            positioned.node.children.as_ref().and_then(|children| {
                children.first().and_then(|first_line| {
                    if let Some(Element::Line(line_element)) = &first_line.node.element {
                        let baseline = first_line.position.y + line_element.metric.baseline;
                        let line_mid = first_line.position.y
                            + line_element.metric.top
                            + line_element.metric.height / 2.0;
                        Some((baseline, line_mid, line_element.metric.height))
                    } else {
                        None
                    }
                })
            })
        }) else {
            return LayoutNode {
                size: Size::new(max_width + CONTENT_OFFSET, y_offset),
                element: None,
                children: Some(child_nodes),
                page_break_policy: Default::default(),
                render_hints: Default::default(),
                scope_id: None,
            };
        };

        let marker_node = LayoutNode {
            size: Size::new(MARKER_WIDTH, marker_height),
            element: Some(Element::ListMarker(ListMarkerElement::new(
                marker_type,
                baseline,
                line_mid,
                MARKER_WIDTH,
            ))),
            children: None,
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        };

        child_nodes.insert(
            0,
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: Rc::new(marker_node),
            },
        );

        LayoutNode {
            size: Size::new(max_width + CONTENT_OFFSET, y_offset),
            element: None,
            children: Some(child_nodes),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

use crate::layout::elements::blockquote::BlockquoteLineElement;
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use tsify::Tsify;

const LINE_WIDTH: f32 = 4.0;
const CONTENT_PADDING: f32 = 16.0;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
pub struct BlockquoteNode {}

impl NodeHtmlCodec for BlockquoteNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("blockquote").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("blockquote", |_| {
            Some(Node::Blockquote(BlockquoteNode {}))
        })]
    }
}

impl Layout for BlockquoteNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        const CONTENT_OFFSET: f32 = LINE_WIDTH + CONTENT_PADDING;

        let child_constraints = BoxConstraints::new(
            (constraints.min_width - CONTENT_OFFSET).max(0.0),
            (constraints.max_width - CONTENT_OFFSET).max(0.0),
            constraints.min_height,
            constraints.max_height,
        );

        let children: Vec<_> = ctx.node.children().collect();
        let child_count = children.len();
        let mut child_nodes = Vec::new();
        let mut y_offset = 0.0;
        let mut max_width = 0.0f32;

        let block_gap = ctx.settings.block_gap;

        for (idx, child) in children.iter().enumerate() {
            let child_layout = ctx.layout(child, child_constraints);

            let is_last = idx == child_count - 1;
            let extra_height = if is_last { 0.0 } else { block_gap * 16.0 };
            let line_height = (child_layout.size.height + extra_height).max(LINE_WIDTH);
            let child_height = child_layout.size.height;
            let child_width = child_layout.size.width;

            let line_node = LayoutNode {
                size: Size::new(LINE_WIDTH, line_height),
                element: Some(Element::Blockquote(BlockquoteLineElement::new(
                    Size::new(LINE_WIDTH, line_height),
                    ctx.node.node_id(),
                ))),
                children: None,
                page_break_policy: Default::default(),
            };

            child_nodes.push(PositionedNode {
                position: Point::new(0.0, y_offset),
                node: Rc::new(line_node),
            });

            child_nodes.push(PositionedNode {
                position: Point::new(CONTENT_OFFSET, y_offset),
                node: child_layout,
            });

            y_offset += child_height + (if is_last { 0.0 } else { block_gap * 16.0 });
            max_width = max_width.max(child_width);
        }

        LayoutNode {
            size: Size::new(max_width + CONTENT_OFFSET, y_offset),
            element: None,
            children: Some(child_nodes),
            page_break_policy: Default::default(),
        }
    }
}

use crate::layout::{Layout, LayoutContext, LayoutNode, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
pub struct BulletListNode {}

impl NodeHtmlCodec for BulletListNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("ul").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("ul", |_| {
            Some(Node::BulletList(BulletListNode {}))
        })]
    }
}

impl Layout for BulletListNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let children: Vec<_> = ctx.node.children().collect();
        let mut child_nodes = Vec::new();
        let mut y_offset = 0.0;
        let mut max_width = 0.0f32;

        for child in children {
            let child_layout = ctx.layout(&child, constraints);
            let child_height = child_layout.size.height;
            let child_width = child_layout.size.width;

            child_nodes.push(PositionedNode {
                position: Point::new(0.0, y_offset),
                node: child_layout,
            });

            y_offset += child_height;
            max_width = max_width.max(child_width);
        }

        LayoutNode {
            size: Size::new(max_width, y_offset),
            element: None,
            children: Some(child_nodes),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
        }
    }
}

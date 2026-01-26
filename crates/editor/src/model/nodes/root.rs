use crate::layout::{Layout, LayoutContext, LayoutNode, PositionedNode};
use crate::model::html::NodeHtmlCodec;
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RootNode {}

impl NodeHtmlCodec for RootNode {}

impl Default for RootNode {
    fn default() -> Self {
        Self {}
    }
}

impl Hash for RootNode {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl Layout for RootNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let children: Vec<_> = ctx.node.children().collect();
        let mut child_nodes = Vec::new();
        let mut y_offset = 0.0;
        let mut max_width = 0.0f32;

        let block_gap = ctx.settings.block_gap;

        for child in &children {
            let child_layout = ctx.layout(child, constraints);

            let positioned = PositionedNode {
                position: Point::new(0.0, y_offset),
                node: child_layout,
            };

            y_offset += positioned.node.size.height + (block_gap * 16.0);
            max_width = max_width.max(positioned.node.size.width);

            child_nodes.push(positioned);
        }

        let size = Size::new(max_width, y_offset);

        LayoutNode {
            size,
            element: None,
            children: Some(child_nodes),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

use crate::layout::{Layout, LayoutContext, LayoutNode, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FoldContentNode {}

impl NodeHtmlCodec for FoldContentNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("div").attr("class", "fold-content").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::new(
            "div",
            55,
            |elem| elem.value().attr("class") == Some("fold-content"),
            |_| Some(Node::FoldContent(FoldContentNode {})),
        )]
    }
}

impl Layout for FoldContentNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let block_gap = ctx.settings.block_gap as f32 / 100.0 * 16.0;
        let children: Vec<_> = ctx.node.children().collect();
        let child_count = children.len();

        let mut positioned_children = Vec::new();
        let mut y = 0.0;

        for (idx, child) in children.into_iter().enumerate() {
            let layout = ctx.layout(&child, constraints);
            let height = layout.size.height;

            positioned_children.push(PositionedNode {
                position: Point::new(0.0, y),
                node: layout,
            });

            y += height;
            if idx < child_count - 1 {
                y += block_gap;
            }
        }

        LayoutNode {
            size: Size::new(constraints.max_width, y),
            element: None,
            children: Some(positioned_children),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

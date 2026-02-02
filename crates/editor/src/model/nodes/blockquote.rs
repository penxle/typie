use crate::layout::elements::SplitEdges;
use crate::layout::elements::blockquote::{
    BlockquoteLineElement, BlockquoteMessageElement, BlockquoteQuoteElement,
};
use crate::layout::{
    Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode, RenderHints,
};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const LINE_WIDTH: f32 = 4.0;
const CONTENT_PADDING: f32 = 16.0;

const QUOTE_SIZE: f32 = 16.0;
const QUOTE_CONTENT_GAP: f32 = 16.0;

const MESSAGE_PADDING_X: f32 = 14.0;
const MESSAGE_PADDING_Y: f32 = 8.0;
const MESSAGE_MAX_WIDTH_RATIO: f32 = 0.8;
const MESSAGE_MIN_WIDTH: f32 = 40.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum BlockquoteVariant {
    #[default]
    LeftLine,
    LeftQuote,
    MessageSent,
    MessageReceived,
}

impl std::fmt::Display for BlockquoteVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BlockquoteVariant::LeftLine => "left_line",
            BlockquoteVariant::LeftQuote => "left_quote",
            BlockquoteVariant::MessageSent => "message_sent",
            BlockquoteVariant::MessageReceived => "message_received",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for BlockquoteVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "left_quote" => BlockquoteVariant::LeftQuote,
            "message_sent" => BlockquoteVariant::MessageSent,
            "message_received" => BlockquoteVariant::MessageReceived,
            _ => BlockquoteVariant::LeftLine,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct BlockquoteNode {
    #[serde(default)]
    pub variant: BlockquoteVariant,
}

impl NodeHtmlCodec for BlockquoteNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(
            DomSpec::el("blockquote")
                .attr("data-variant", self.variant.to_string())
                .hole(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::new(
            "blockquote",
            50,
            |_| true,
            |elem| {
                let variant = elem
                    .value()
                    .attr("data-variant")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default();
                Some(Node::Blockquote(BlockquoteNode { variant }))
            },
        )]
    }
}

impl Layout for BlockquoteNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        match self.variant {
            BlockquoteVariant::LeftLine => self.layout_left_line(ctx, constraints),
            BlockquoteVariant::LeftQuote => self.layout_left_quote(ctx, constraints),
            BlockquoteVariant::MessageSent | BlockquoteVariant::MessageReceived => {
                self.layout_message(ctx, constraints)
            }
        }
    }
}

impl BlockquoteNode {
    fn layout_left_line(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
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
                render_hints: Default::default(),
                scope_id: None,
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
            render_hints: Default::default(),
            scope_id: None,
        }
    }

    fn layout_left_quote(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        const CONTENT_OFFSET: f32 = QUOTE_SIZE + QUOTE_CONTENT_GAP;

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
        let node_id = ctx.node.node_id();

        let quote_node = LayoutNode {
            size: Size::new(QUOTE_SIZE, QUOTE_SIZE),
            element: Some(Element::BlockquoteQuote(BlockquoteQuoteElement::new(
                Size::new(QUOTE_SIZE, QUOTE_SIZE),
                node_id,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        };

        child_nodes.push(PositionedNode {
            position: Point::new(0.0, 0.0),
            node: Rc::new(quote_node),
        });

        for (idx, child) in children.iter().enumerate() {
            let child_layout = ctx.layout(child, child_constraints);
            let is_last = idx == child_count - 1;
            let child_height = child_layout.size.height;
            let child_width = child_layout.size.width;

            child_nodes.push(PositionedNode {
                position: Point::new(CONTENT_OFFSET, y_offset),
                node: child_layout,
            });

            y_offset += child_height + (if is_last { 0.0 } else { block_gap * 16.0 });
            max_width = max_width.max(child_width);
        }

        let total_height = y_offset.max(QUOTE_SIZE);

        LayoutNode {
            size: Size::new(max_width + CONTENT_OFFSET, total_height),
            element: None,
            children: Some(child_nodes),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        }
    }

    fn layout_message(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_bubble_width = constraints.max_width * MESSAGE_MAX_WIDTH_RATIO;
        let max_content_width = (max_bubble_width - MESSAGE_PADDING_X * 2.0).max(0.0);

        let child_constraints =
            BoxConstraints::new(0.0, max_content_width, 0.0, constraints.max_height);

        let children: Vec<_> = ctx.node.children().collect();
        let child_count = children.len();
        let block_gap = ctx.settings.block_gap;
        let node_id = ctx.node.node_id();

        let mut content_nodes = Vec::new();
        let mut y_offset = MESSAGE_PADDING_Y;
        let mut actual_content_width = 0.0f32;

        for (idx, child) in children.iter().enumerate() {
            let child_layout = ctx.layout(child, child_constraints);
            let is_last = idx == child_count - 1;
            let child_height = child_layout.size.height;
            let child_width = child_layout.size.width;

            content_nodes.push(PositionedNode {
                position: Point::new(MESSAGE_PADDING_X, y_offset),
                node: child_layout,
            });

            y_offset += child_height + (if is_last { 0.0 } else { block_gap * 16.0 });
            actual_content_width = actual_content_width.max(child_width);
        }

        y_offset += MESSAGE_PADDING_Y;

        let bubble_width = (actual_content_width + MESSAGE_PADDING_X * 2.0).max(MESSAGE_MIN_WIDTH);
        let bubble_height = y_offset;
        let bubble_size = Size::new(bubble_width, bubble_height);

        let is_sent = matches!(self.variant, BlockquoteVariant::MessageSent);
        let bubble_x = if is_sent {
            constraints.max_width - bubble_width
        } else {
            0.0
        };

        let background_element = BlockquoteMessageElement::new(
            bubble_size,
            node_id,
            self.variant,
            SplitEdges::default(),
        );

        let bubble_node = LayoutNode {
            size: bubble_size,
            element: Some(Element::BlockquoteMessage(background_element)),
            children: Some(content_nodes),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: if is_sent {
                RenderHints {
                    default_text_color: Some("text.bright".into()),
                }
            } else {
                Default::default()
            },
            scope_id: None,
        };

        LayoutNode {
            size: Size::new(constraints.max_width, bubble_height),
            element: None,
            children: Some(vec![PositionedNode {
                position: Point::new(bubble_x, 0.0),
                node: Rc::new(bubble_node),
            }]),
            page_break_policy: Default::default(),
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

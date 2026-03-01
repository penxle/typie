use crate::layout::elements::SplitEdges;
use crate::layout::elements::blockquote::{
    BlockquoteLineElement, BlockquoteMessageElement, BlockquoteQuoteElement,
};
use crate::layout::{
    Element, Layout, LayoutCache, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode,
    RenderHints,
};
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::model::{Node, NodeRef, TextAlign};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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
    fn merge_x_bounds(acc: &mut Option<(f32, f32)>, left: f32, right: f32) {
        if right < left {
            return;
        }

        match acc {
            Some((min_x, max_x)) => {
                *min_x = min_x.min(left);
                *max_x = max_x.max(right);
            }
            None => *acc = Some((left, right)),
        }
    }

    fn line_visual_x_bounds(
        line: &crate::layout::elements::LineElement,
        x_offset: f32,
    ) -> (f32, f32) {
        let left = x_offset + line.metric.left;
        let right = line
            .metric
            .clusters
            .last()
            .map(|cluster| left + cluster.x + cluster.width)
            .unwrap_or(left + line.metric.content_width.max(0.0))
            .max(left);
        (left, right)
    }

    fn line_intrinsic_x_bounds(
        line: &crate::layout::elements::LineElement,
        x_offset: f32,
    ) -> (f32, f32) {
        let width = line
            .metric
            .clusters
            .last()
            .map(|cluster| cluster.x + cluster.width)
            .unwrap_or(line.metric.content_width.max(0.0))
            .max(0.0);
        (x_offset, x_offset + width)
    }

    fn layout_visual_x_bounds(node: &LayoutNode, x_offset: f32) -> Option<(f32, f32)> {
        let mut bounds = None;

        if let Some(children) = node.children.as_ref() {
            for child in children {
                if let Some((left, right)) =
                    Self::layout_visual_x_bounds(child.node.as_ref(), x_offset + child.position.x)
                {
                    Self::merge_x_bounds(&mut bounds, left, right);
                }
            }
        }

        if bounds.is_some() {
            return bounds;
        }

        if let Some(Element::Line(line)) = node.element.as_ref() {
            let (left, right) = Self::line_visual_x_bounds(line, x_offset);
            return Some((left, right));
        }

        if node.size.width > 0.0 {
            return Some((x_offset, x_offset + node.size.width));
        }

        None
    }

    fn layout_intrinsic_x_bounds(node: &LayoutNode, x_offset: f32) -> Option<(f32, f32)> {
        let mut bounds = None;

        if let Some(children) = node.children.as_ref() {
            for child in children {
                if let Some((left, right)) = Self::layout_intrinsic_x_bounds(
                    child.node.as_ref(),
                    x_offset + child.position.x,
                ) {
                    Self::merge_x_bounds(&mut bounds, left, right);
                }
            }
        }

        if bounds.is_some() {
            return bounds;
        }

        if let Some(Element::Line(line)) = node.element.as_ref() {
            let (left, right) = Self::line_intrinsic_x_bounds(line, x_offset);
            return Some((left, right));
        }

        if node.size.width > 0.0 {
            return Some((x_offset, x_offset + node.size.width));
        }

        None
    }

    fn node_intrinsic_width(node: &LayoutNode) -> f32 {
        Self::layout_intrinsic_x_bounds(node, 0.0)
            .map(|(left, right)| (right - left).max(0.0))
            .unwrap_or(node.size.width.max(0.0))
    }

    fn node_visual_left_and_width(node: &LayoutNode) -> (f32, f32) {
        let (left, right) = Self::layout_visual_x_bounds(node, 0.0)
            .or_else(|| Self::layout_intrinsic_x_bounds(node, 0.0))
            .unwrap_or((0.0, node.size.width.max(0.0)));
        (left, (right - left).max(0.0))
    }

    fn message_child_align(child: &NodeRef<'_>) -> TextAlign {
        match child.node() {
            Some(Node::Paragraph(paragraph)) => paragraph.align,
            _ => TextAlign::Left,
        }
    }

    fn message_target_left(inner_width: f32, child_width: f32, align: TextAlign) -> f32 {
        match align {
            TextAlign::Center => MESSAGE_PADDING_X + (inner_width - child_width).max(0.0) / 2.0,
            TextAlign::Right => MESSAGE_PADDING_X + (inner_width - child_width).max(0.0),
            TextAlign::Left | TextAlign::Justify => MESSAGE_PADDING_X,
        }
    }

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
            let extra_height = if is_last {
                0.0
            } else {
                block_gap as f32 / 100.0 * 16.0
            };
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

            y_offset += child_height
                + (if is_last {
                    0.0
                } else {
                    block_gap as f32 / 100.0 * 16.0
                });
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

            y_offset += child_height
                + (if is_last {
                    0.0
                } else {
                    block_gap as f32 / 100.0 * 16.0
                });
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
        let min_content_width = (MESSAGE_MIN_WIDTH - MESSAGE_PADDING_X * 2.0).max(0.0);
        let measure_constraints =
            BoxConstraints::new(0.0, max_content_width, 0.0, constraints.max_height);
        let children: Vec<_> = ctx.node.children().collect();
        let block_gap = ctx.settings.block_gap;
        let node_id = ctx.node.node_id();

        let measured_content_width = {
            let measure_cache = RefCell::new(LayoutCache::new());
            let measure_ctx = LayoutContext::new(
                ctx.node,
                ctx.settings,
                ctx.default_attrs,
                ctx.decorations,
                ctx.scale_factor,
                ctx.view_states,
                &measure_cache,
            );
            children.iter().fold(min_content_width, |acc, child| {
                let child_layout = measure_ctx.layout(child, measure_constraints);
                acc.max(Self::node_intrinsic_width(child_layout.as_ref()))
            })
        };

        let final_constraints =
            BoxConstraints::new(0.0, measured_content_width, 0.0, constraints.max_height);

        struct PreparedChild {
            layout: Rc<LayoutNode>,
            height: f32,
            visual_left: f32,
            visual_width: f32,
            align: TextAlign,
        }

        let (prepared, actual_content_width) = {
            let final_cache = RefCell::new(LayoutCache::new());
            let final_ctx = LayoutContext::new(
                ctx.node,
                ctx.settings,
                ctx.default_attrs,
                ctx.decorations,
                ctx.scale_factor,
                ctx.view_states,
                &final_cache,
            );
            let mut prepared = Vec::with_capacity(children.len());
            let mut actual_content_width = min_content_width;
            for child in children.iter() {
                let child_layout = final_ctx.layout(child, final_constraints);
                let (visual_left, visual_width) =
                    Self::node_visual_left_and_width(child_layout.as_ref());
                let align = Self::message_child_align(child);
                actual_content_width = actual_content_width.max(visual_width);

                prepared.push(PreparedChild {
                    height: child_layout.size.height,
                    layout: child_layout,
                    visual_left,
                    visual_width,
                    align,
                });
            }
            (prepared, actual_content_width)
        };

        let mut content_nodes = Vec::with_capacity(prepared.len());
        let mut y_offset = MESSAGE_PADDING_Y;

        for (idx, child) in prepared.iter().enumerate() {
            let target_left =
                Self::message_target_left(actual_content_width, child.visual_width, child.align);
            let child_x = target_left - child.visual_left;
            let is_last = idx == prepared.len() - 1;

            content_nodes.push(PositionedNode {
                position: Point::new(child_x, y_offset),
                node: child.layout.clone(),
            });

            y_offset += child.height
                + (if is_last {
                    0.0
                } else {
                    block_gap as f32 / 100.0 * 16.0
                });
        }

        y_offset += MESSAGE_PADDING_Y;

        let bubble_width = actual_content_width + MESSAGE_PADDING_X * 2.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutCache;
    use crate::model::{Decorations, DefaultAttrs, TextAlign};
    use crate::runtime::ViewStates;
    use crate::state::build_selection_decorations;
    use crate::types::Point;
    use std::cell::RefCell;

    fn get_message_sent_bubble(layout: &LayoutNode) -> &LayoutNode {
        layout
            .children
            .as_ref()
            .and_then(|children| children.first())
            .map(|child| child.node.as_ref())
            .expect("bubble child missing")
    }

    fn content_x_bounds_in_bubble(bubble: &LayoutNode) -> (f32, f32) {
        let Some(content_nodes) = bubble.children.as_ref() else {
            return (0.0, 0.0);
        };

        let mut bounds = None;
        for content in content_nodes {
            if let Some((left, right)) =
                BlockquoteNode::layout_visual_x_bounds(content.node.as_ref(), content.position.x)
            {
                BlockquoteNode::merge_x_bounds(&mut bounds, left, right);
            }
        }

        bounds.unwrap_or((0.0, 0.0))
    }

    fn content_node_x_bounds(content: &PositionedNode) -> Option<(f32, f32)> {
        BlockquoteNode::layout_visual_x_bounds(content.node.as_ref(), content.position.x)
    }

    #[test]
    fn message_sent_right_align_keeps_compact_width_without_overflow() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p paragraph(align: TextAlign::Right,) {
                        text { "hello" }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let expected_max_bubble_width = constraints.max_width * MESSAGE_MAX_WIDTH_RATIO;

        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);
        let (content_min_x, content_max_x) = content_x_bounds_in_bubble(bubble);

        assert!(bubble.size.width < expected_max_bubble_width - 0.01);
        assert!(content_min_x >= MESSAGE_PADDING_X - 0.01);
        assert!(content_max_x <= bubble.size.width - MESSAGE_PADDING_X + 0.01);
    }

    #[test]
    fn message_sent_center_align_keeps_compact_width_without_overflow() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p paragraph(align: TextAlign::Center,) {
                        text { "hello" }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let expected_max_bubble_width = constraints.max_width * MESSAGE_MAX_WIDTH_RATIO;

        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);
        let (content_min_x, content_max_x) = content_x_bounds_in_bubble(bubble);

        assert!(bubble.size.width < expected_max_bubble_width - 0.01);
        assert!(content_min_x >= MESSAGE_PADDING_X - 0.01);
        assert!(content_max_x <= bubble.size.width - MESSAGE_PADDING_X + 0.01);
    }

    #[test]
    fn message_sent_keeps_compact_width_when_paragraph_is_left_aligned() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p paragraph(align: TextAlign::Left,) {
                        text { "hello" }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let expected_max_bubble_width = constraints.max_width * MESSAGE_MAX_WIDTH_RATIO;

        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);

        assert!(bubble.size.width < expected_max_bubble_width - 0.01);
        assert!(bubble.size.width >= MESSAGE_MIN_WIDTH);
    }

    #[test]
    fn message_sent_uses_max_item_width_and_aligns_each_paragraph_within_bubble() {
        let mut bq = id!();
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p1 paragraph(align: TextAlign::Left,) {
                        text { "long long long message" }
                    }
                    @p2 paragraph(align: TextAlign::Center,) {
                        text { "mid" }
                    }
                    @p3 paragraph(align: TextAlign::Right,) {
                        text { "rt" }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);
        let Some(content_nodes) = bubble.children.as_ref() else {
            panic!("bubble content missing");
        };
        assert_eq!(content_nodes.len(), 3);

        let (l1, r1) = content_node_x_bounds(&content_nodes[0]).expect("p1 bounds");
        let (l2, r2) = content_node_x_bounds(&content_nodes[1]).expect("p2 bounds");
        let (l3, r3) = content_node_x_bounds(&content_nodes[2]).expect("p3 bounds");
        let w1 = r1 - l1;
        let w2 = r2 - l2;
        let w3 = r3 - l3;

        let inner_width = bubble.size.width - MESSAGE_PADDING_X * 2.0;
        let max_width = w1.max(w2).max(w3);

        assert!(
            (inner_width - max_width).abs() < 0.51,
            "inner/max mismatch: inner={}, max={}, w1={}, w2={}, w3={}, l1={}, r1={}, l2={}, r2={}, l3={}, r3={}",
            inner_width,
            max_width,
            w1,
            w2,
            w3,
            l1,
            r1,
            l2,
            r2,
            l3,
            r3
        );
        assert!((l1 - MESSAGE_PADDING_X).abs() < 0.51);
        assert!((l2 - (MESSAGE_PADDING_X + (inner_width - w2) / 2.0)).abs() < 0.51);
        assert!(
            (r3 - (MESSAGE_PADDING_X + inner_width)).abs() < 0.51,
            "right align mismatch: r3={}, expected={}, inner_width={}, w3={}, l3={}",
            r3,
            MESSAGE_PADDING_X + inner_width,
            inner_width,
            w3,
            l3
        );
    }

    #[test]
    fn message_sent_bullet_list_right_align_stays_inside_bubble() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    bullet_list {
                        list_item {
                            @p paragraph(align: TextAlign::Right,) {
                                text { "hello" }
                            }
                        }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);
        let (content_min_x, content_max_x) = content_x_bounds_in_bubble(bubble);

        assert!(content_min_x >= MESSAGE_PADDING_X - 0.01);
        assert!(content_max_x <= bubble.size.width - MESSAGE_PADDING_X + 0.01);
        assert!(
            (content_max_x - (bubble.size.width - MESSAGE_PADDING_X)).abs() < 0.51,
            "list right align mismatch: right={}, expected={}",
            content_max_x,
            bubble.size.width - MESSAGE_PADDING_X
        );
    }

    #[test]
    fn message_sent_ordered_list_right_align_stays_inside_bubble() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    ordered_list {
                        list_item {
                            @p paragraph(align: TextAlign::Right,) {
                                text { "hello" }
                            }
                        }
                    }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);
        let (content_min_x, content_max_x) = content_x_bounds_in_bubble(bubble);

        assert!(content_min_x >= MESSAGE_PADDING_X - 0.01);
        assert!(content_max_x <= bubble.size.width - MESSAGE_PADDING_X + 0.01);
        assert!(
            (content_max_x - (bubble.size.width - MESSAGE_PADDING_X)).abs() < 0.51,
            "list right align mismatch: right={}, expected={}",
            content_max_x,
            bubble.size.width - MESSAGE_PADDING_X
        );
    }

    #[test]
    fn message_sent_right_align_selection_rect_stays_inside_bubble() {
        let mut bq = id!();
        let mut p = id!();

        let state = state! {
            doc {
                @bq blockquote(variant: BlockquoteVariant::MessageSent,) {
                    @p paragraph(align: TextAlign::Right,) {
                        text { "hello" }
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let doc = &state.doc;
        let blockquote_ref = doc.node(bq).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &blockquote_ref,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, f32::INFINITY);
        let selections = build_selection_decorations(&state.doc, &state.selection, None);

        let Some(Node::Blockquote(blockquote)) = blockquote_ref.node() else {
            panic!("blockquote node expected");
        };

        let layout = blockquote.layout(&ctx, constraints);
        let bubble = get_message_sent_bubble(&layout);

        let mut found_rect = false;

        let Some(content_nodes) = bubble.children.as_ref() else {
            panic!("bubble content missing");
        };

        for content in content_nodes {
            let Some(lines) = content.node.children.as_ref() else {
                continue;
            };

            for line in lines {
                let Some(Element::Line(line_element)) = line.node.element.as_ref() else {
                    continue;
                };

                let rects = line_element.compute_selection_rects(
                    Point::new(
                        content.position.x + line.position.x,
                        content.position.y + line.position.y,
                    ),
                    &selections,
                );

                for rect in rects {
                    found_rect = true;
                    assert!(rect.x >= MESSAGE_PADDING_X - 0.01);
                    assert!(
                        rect.x + rect.width <= bubble.size.width - MESSAGE_PADDING_X + 0.01,
                        "selection rect overflow: x={}, width={}, bubble_width={}",
                        rect.x,
                        rect.width,
                        bubble.size.width
                    );
                }
            }
        }

        assert!(found_rect, "selection rect should be generated");
    }
}

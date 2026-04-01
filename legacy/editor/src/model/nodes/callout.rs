use crate::layout::elements::{CalloutBackgroundElement, CalloutIconElement, SplitEdges};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const ICON_WIDTH: f32 = 20.0;
const ICON_HEIGHT: f32 = 28.0;
const ICON_CONTENT_GAP: f32 = 8.0;
const PADDING_X: f32 = 12.0;
const PADDING_Y: f32 = 16.0;

pub const CALLOUT_BORDER_RADIUS: f32 = 8.0;
pub const CALLOUT_BORDER_WIDTH: f32 = 1.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum CalloutVariant {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl std::fmt::Display for CalloutVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CalloutVariant::Info => "info",
            CalloutVariant::Success => "success",
            CalloutVariant::Warning => "warning",
            CalloutVariant::Danger => "danger",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for CalloutVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "success" => CalloutVariant::Success,
            "warning" => CalloutVariant::Warning,
            "danger" => CalloutVariant::Danger,
            _ => CalloutVariant::Info,
        })
    }
}

impl CalloutVariant {
    pub fn next(&self) -> Self {
        match self {
            CalloutVariant::Info => CalloutVariant::Success,
            CalloutVariant::Success => CalloutVariant::Warning,
            CalloutVariant::Warning => CalloutVariant::Danger,
            CalloutVariant::Danger => CalloutVariant::Info,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct CalloutNode {
    #[serde(default)]
    pub variant: CalloutVariant,
}

impl NodeHtmlCodec for CalloutNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(
            DomSpec::el("div")
                .attr("class", "callout")
                .attr("data-variant", self.variant.to_string())
                .hole(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::new(
            "div",
            55,
            |elem| elem.value().attr("class") == Some("callout"),
            |elem| {
                let variant = elem
                    .value()
                    .attr("data-variant")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default();
                Some(Node::Callout(CalloutNode { variant }))
            },
        )]
    }
}

impl Layout for CalloutNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let block_gap = ctx.settings.block_gap as f32 / 100.0 * 16.0;
        let content_offset_x = PADDING_X + ICON_WIDTH + ICON_CONTENT_GAP;
        let content_width = (constraints.max_width - content_offset_x - PADDING_X).max(0.0);

        let children: Vec<_> = ctx.node.children().collect();
        let child_count = children.len();

        let mut positioned_children = Vec::new();
        let mut y = PADDING_Y;

        for (idx, child) in children.into_iter().enumerate() {
            let child_constraints =
                BoxConstraints::new(content_width, content_width, 0.0, f32::MAX);
            let layout = ctx.layout(&child, child_constraints);
            let height = layout.size.height;

            positioned_children.push(PositionedNode {
                position: Point::new(content_offset_x, y),
                node: layout,
            });

            y += height;
            if idx < child_count - 1 {
                y += block_gap;
            }
        }

        y += PADDING_Y;

        let total_size = Size::new(constraints.max_width, y);
        let node_id = ctx.node.node_id();

        let mut bg_children = Vec::new();

        let icon_height = positioned_children
            .first()
            .and_then(|first_child| {
                first_child.node.children.as_ref().and_then(|lines| {
                    lines.first().and_then(|first_line| {
                        if let Some(Element::Line(line)) = &first_line.node.element {
                            Some(line.size.height)
                        } else {
                            None
                        }
                    })
                })
            })
            .unwrap_or(ICON_HEIGHT);

        let icon_element =
            CalloutIconElement::new(Size::new(ICON_WIDTH, icon_height), self.variant, node_id);

        let icon_wrapper = PositionedNode {
            position: Point::new(PADDING_X, PADDING_Y),
            node: Rc::new(LayoutNode {
                size: Size::new(ICON_WIDTH, icon_height),
                element: Some(Element::CalloutIcon(icon_element)),
                children: None,
                page_break_policy: PageBreakPolicy::Avoid,
                render_hints: Default::default(),
                scope_id: None,
            }),
        };

        if !positioned_children.is_empty() {
            let first_child = positioned_children.remove(0);
            let child_bottom = first_child.position.y + first_child.node.size.height;
            let header_height = child_bottom; // icon bottom 무시

            let header_wrapper = PositionedNode {
                position: Point::new(0.0, 0.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(total_size.width, header_height),
                    element: None,
                    children: Some(vec![icon_wrapper, first_child]),
                    page_break_policy: PageBreakPolicy::Avoid,
                    render_hints: Default::default(),
                    scope_id: None,
                }),
            };
            bg_children.push(header_wrapper);
            bg_children.extend(positioned_children);
        } else {
            bg_children.push(icon_wrapper);
        }

        let background_element =
            CalloutBackgroundElement::new(total_size, self.variant, node_id, SplitEdges::default());

        let background_wrapper = PositionedNode {
            position: Point::new(0.0, 0.0),
            node: Rc::new(LayoutNode {
                size: total_size,
                element: Some(Element::CalloutBackground(background_element)),
                children: Some(bg_children),
                page_break_policy: PageBreakPolicy::Auto,
                render_hints: Default::default(),
                scope_id: None,
            }),
        };

        LayoutNode {
            size: total_size,
            element: None,
            children: Some(vec![background_wrapper]),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

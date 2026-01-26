use crate::layout::elements::HorizontalRuleElement;
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};

const DEFAULT_HEIGHT: f32 = 24.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum HorizontalRuleVariant {
    #[default]
    Line,
    DashedLine,
    CircleLine,
    DiamondLine,
    Circle,
    Diamond,
    ThreeCircles,
    ThreeDiamonds,
    Zigzag,
}

impl std::fmt::Display for HorizontalRuleVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HorizontalRuleVariant::Line => "line",
            HorizontalRuleVariant::DashedLine => "dashed_line",
            HorizontalRuleVariant::CircleLine => "circle_line",
            HorizontalRuleVariant::DiamondLine => "diamond_line",
            HorizontalRuleVariant::Circle => "circle",
            HorizontalRuleVariant::Diamond => "diamond",
            HorizontalRuleVariant::ThreeCircles => "three_circles",
            HorizontalRuleVariant::ThreeDiamonds => "three_diamonds",
            HorizontalRuleVariant::Zigzag => "zigzag",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for HorizontalRuleVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "dashed_line" => HorizontalRuleVariant::DashedLine,
            "circle_line" => HorizontalRuleVariant::CircleLine,
            "diamond_line" => HorizontalRuleVariant::DiamondLine,
            "circle" => HorizontalRuleVariant::Circle,
            "diamond" => HorizontalRuleVariant::Diamond,
            "three_circles" => HorizontalRuleVariant::ThreeCircles,
            "three_diamonds" => HorizontalRuleVariant::ThreeDiamonds,
            "zigzag" => HorizontalRuleVariant::Zigzag,
            _ => HorizontalRuleVariant::Line,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct HorizontalRuleNode {
    #[serde(default)]
    pub variant: HorizontalRuleVariant,
}

impl NodeHtmlCodec for HorizontalRuleNode {
    fn to_dom(&self) -> Option<DomSpec> {
        let spec = if self.variant != HorizontalRuleVariant::Line {
            DomSpec::el("hr")
                .data("variant", self.variant.to_string())
                .void()
        } else {
            DomSpec::el("hr").void()
        };
        Some(spec)
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("hr", |elem| {
            let variant = elem
                .value()
                .attr("data-variant")
                .and_then(|s| s.parse().ok())
                .unwrap_or(HorizontalRuleVariant::Line);
            Some(Node::HorizontalRule(HorizontalRuleNode { variant }))
        })]
    }
}

impl Layout for HorizontalRuleNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let width = constraints.max_width;
        let height = DEFAULT_HEIGHT;

        let node_id = ctx.node.node_id();
        let parent_id = ctx.node.parent().map(|p| p.node_id()).unwrap_or(node_id);

        let element =
            HorizontalRuleElement::new(node_id, parent_id, Size::new(width, height), self.variant);

        LayoutNode {
            size: Size::new(width, height),
            element: Some(Element::HorizontalRule(element)),
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

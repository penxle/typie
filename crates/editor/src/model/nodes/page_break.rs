use crate::layout::{Layout, LayoutContext, LayoutNode, PageBreakPolicy};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PageBreakNode {}

impl NodeHtmlCodec for PageBreakNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("div").data("page-break", "true").empty())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::new(
            "div",
            60,
            |elem| elem.value().attr("data-page-break").is_some(),
            |_| Some(Node::PageBreak(PageBreakNode {})),
        )]
    }
}

impl Layout for PageBreakNode {
    fn layout(&self, _ctx: &LayoutContext, _constraints: BoxConstraints) -> LayoutNode {
        LayoutNode {
            size: Size::zero(),
            element: None,
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

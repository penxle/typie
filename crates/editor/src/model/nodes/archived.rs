use crate::layout::elements::external::{ExternalElement, ExternalElementData};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ArchivedNode {
    pub id: Option<String>,
}

impl NodeHtmlCodec for ArchivedNode {
    fn to_dom(&self) -> Option<DomSpec> {
        if self.id.is_none() {
            return None;
        }

        Some(
            DomSpec::el("div")
                .attr("data-archived-id", self.id.clone().unwrap())
                .text(""),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::new(
            "div",
            50,
            |elem| elem.value().attr("data-archived-id").is_some(),
            |elem| {
                let id = elem.value().attr("data-archived-id").map(|s| s.to_string());
                Some(Node::Archived(ArchivedNode { id }))
            },
        )]
    }
}

impl Default for ArchivedNode {
    fn default() -> Self {
        Self { id: None }
    }
}

impl Hash for ArchivedNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Layout for ArchivedNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;

        let display_height = ctx
            .view_states
            .get(&ctx.node.node_id())
            .and_then(|s| s.external_height())
            .unwrap_or(1.0);

        let data = ExternalElementData::Archived {
            id: self.id.clone(),
        };

        let Some(parent_block) = ctx.node.parent() else {
            return LayoutNode {
                size: Size::new(constraints.max_width, 1.0),
                element: None,
                children: None,
                page_break_policy: Default::default(),
                render_hints: Default::default(),
                scope_id: None,
            };
        };

        let element = ExternalElement::new(
            ctx.node.node_id(),
            parent_block.node_id(),
            Size::new(max_width, display_height),
            data,
        );

        LayoutNode {
            size: Size::new(max_width, display_height),
            element: Some(Element::External(element)),
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

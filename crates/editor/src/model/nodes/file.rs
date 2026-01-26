use crate::layout::elements::external::{ExternalElement, ExternalElementData};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec, Tsify)]
pub struct FileNode {
    pub id: Option<String>,
    #[serde(skip_serializing, default)]
    pub upload_id: Option<String>,
}

impl NodeHtmlCodec for FileNode {
    fn to_dom(&self) -> Option<DomSpec> {
        if self.id.is_none() {
            return None;
        }

        Some(
            DomSpec::el("a")
                .attr("data-file-id", self.id.clone().unwrap())
                .text(""),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("a[data-file-id]", |elem| {
            let id = elem.value().attr("data-file-id").map(|s| s.to_string());
            Some(Node::File(FileNode { id, upload_id: None }))
        })]
    }
}

impl Default for FileNode {
    fn default() -> Self {
        Self {
            id: None,
            upload_id: None,
        }
    }
}

impl Hash for FileNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Layout for FileNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;

        let display_height = ctx
            .view_states
            .get(&ctx.node.node_id())
            .and_then(|s| s.external_height())
            .unwrap_or(1.0);

        let data = ExternalElementData::File {
            id: self.id.clone(),
            upload_id: self.upload_id.clone(),
        };

        let parent_block = ctx.node.parent().expect("File node must have a parent");

        let element = ExternalElement::new(
            ctx.node.node_id(),
            parent_block.node_id(),
            Size::new(max_width, display_height),
            data,
        );

        let size = Size::new(max_width, display_height);

        LayoutNode {
            size,
            element: Some(Element::External(element)),
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

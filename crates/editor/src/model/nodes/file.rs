use crate::layout::elements::external::{ExternalElement, ExternalElementData};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tsify::Tsify;

const FILE_NODE_HEIGHT: f32 = 48.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec, Tsify)]
pub struct FileNode {
    pub name: Option<String>,
    pub size: Option<u64>,
    pub src: Option<String>,
    #[serde(skip_serializing, default)]
    pub upload_id: Option<String>,
}

impl NodeHtmlCodec for FileNode {
    fn to_dom(&self) -> Option<DomSpec> {
        if self.src.is_none() {
            return None;
        }

        let mut spec = DomSpec::el("a")
            .attr("href", self.src.clone().unwrap())
            .attr("data-file", "true".to_string())
            .attr("download", self.name.clone().unwrap_or_default());

        if let Some(size) = self.size {
            spec = spec.attr("data-size", size.to_string());
        }

        Some(spec.text(self.name.clone().unwrap_or_default()))
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("a[data-file]", |elem| {
            let src = elem.value().attr("href").map(|s| s.to_string());
            let name = elem.value().attr("download").map(|s| s.to_string());
            let size = elem.value().attr("data-size").and_then(|s| s.parse().ok());
            Some(Node::File(FileNode {
                name,
                size,
                src,
                upload_id: None,
            }))
        })]
    }
}

impl Default for FileNode {
    fn default() -> Self {
        Self {
            name: None,
            size: None,
            src: None,
            upload_id: None,
        }
    }
}

impl Hash for FileNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.size.hash(state);
        self.src.hash(state);
    }
}

impl Layout for FileNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;

        let data = ExternalElementData::File {
            name: self.name.clone(),
            size: self.size,
            src: self.src.clone(),
            upload_id: self.upload_id.clone(),
        };

        let parent_block = ctx.node.parent().expect("File node must have a parent");

        let element = ExternalElement::new(
            ctx.node.node_id(),
            parent_block.node_id(),
            Size::new(max_width, FILE_NODE_HEIGHT),
            data,
        );

        let size = Size::new(max_width, FILE_NODE_HEIGHT);

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

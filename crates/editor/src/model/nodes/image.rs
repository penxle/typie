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
pub struct ImageNode {
    pub id: Option<String>,
    #[serde(default = "default_proportion")]
    pub proportion: f32,
    #[serde(skip_serializing, default)]
    pub upload_id: Option<String>,
}

fn default_proportion() -> f32 {
    1.0
}

impl NodeHtmlCodec for ImageNode {
    fn to_dom(&self) -> Option<DomSpec> {
        if self.id.is_none() {
            return None;
        }

        Some(
            DomSpec::el("img")
                .attr("data-image-id", self.id.clone().unwrap())
                .attr("data-proportion", self.proportion.to_string())
                .void(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("img[data-image-id]", |elem| {
            let id = elem.value().attr("data-image-id").map(|s| s.to_string());
            let proportion = elem
                .value()
                .attr("data-proportion")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            Some(Node::Image(ImageNode {
                id,
                proportion,
                upload_id: None,
            }))
        })]
    }
}

impl Default for ImageNode {
    fn default() -> Self {
        Self {
            id: None,
            proportion: 1.0,
            upload_id: None,
        }
    }
}

impl Hash for ImageNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.proportion.to_bits().hash(state);
    }
}

impl Layout for ImageNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;

        let display_height = ctx
            .view_states
            .get(&ctx.node.node_id())
            .and_then(|s| s.external_height())
            .unwrap_or(1.0);

        let data = ExternalElementData::Image {
            id: self.id.clone(),
            proportion: self.proportion,
            upload_id: self.upload_id.clone(),
        };

        let parent_block = ctx.node.parent().expect("Image node must have a parent");

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_image_layout_with_placeholder_height() {
        let mut p = id!();
        let runtime = runtime! {
            viewport {
                paginated { width: 800.0, height: 500.0, margin: 50.0 }
            }
            doc {
               @p image (
                   id: Some("test-image-id".to_string()),
               )
            }
            selection { (p, 0) }
        };

        let pages = runtime.pages();
        let mut found = false;

        for page in pages {
            for (_, element) in page.external_elements() {
                match &element.data {
                    crate::layout::elements::ExternalElementData::Image { id, .. } => {
                        assert_eq!(id.as_deref(), Some("test-image-id"));
                        assert_eq!(element.size.height, 1.0);
                        assert_eq!(element.size.width, 700.0); // 800 - 2 * 50
                        found = true;
                    }
                    _ => {}
                }
            }
        }

        assert!(found, "Image should be present in the layout");
    }
}

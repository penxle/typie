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
    pub src: String,
    pub width: f32,
    pub height: f32,
}

impl NodeHtmlCodec for ImageNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(
            DomSpec::el("img")
                .attr("src", &self.src)
                .attr("width", self.width.to_string())
                .attr("height", self.height.to_string())
                .void(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("img", |elem| {
            let src = elem.value().attr("src").unwrap_or("").into();
            let width = elem
                .value()
                .attr("width")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100.0);
            let height = elem
                .value()
                .attr("height")
                .and_then(|s| s.parse().ok())
                .unwrap_or(100.0);
            Some(Node::Image(ImageNode { src, width, height }))
        })]
    }
}

impl Default for ImageNode {
    fn default() -> Self {
        Self {
            src: String::new(),
            width: 1.0,
            height: 1.0,
        }
    }
}

impl Hash for ImageNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.src.hash(state);
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

impl Layout for ImageNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;
        let max_height = constraints.max_height;

        let scale = (max_width / self.width)
            .min(max_height / self.height)
            .min(1.0);

        let display_width = self.width * scale;
        let display_height = self.height * scale;

        let data = ExternalElementData::Image {
            src: self.src.clone(),
            original_width: self.width,
            original_height: self.height,
        };

        let parent_block = ctx.node.parent().expect("Image node must have a parent");

        let element = ExternalElement::new(
            ctx.node.node_id(),
            parent_block.node_id(),
            Size::new(display_width, display_height),
            data,
        );

        let size = Size::new(display_width, display_height);

        LayoutNode {
            size,
            element: Some(Element::External(element)),
            children: None,
            page_break_policy: PageBreakPolicy::Avoid,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_image_layout_height_limit_in_paginated_mode() {
        let mut p = id!();
        let runtime = runtime! {
            viewport {
                paginated { width: 800.0, height: 500.0, margin: 50.0 }
            }
            doc {
               @p image (
                   src: "test.png".to_string(),
                   width: 1000.0,
                   height: 1000.0,
               )
            }
            selection { (p, 0) }
        };

        let pages = runtime.pages();
        let mut found = false;

        for page in pages {
            for (_, element) in page.external_elements() {
                match element.data {
                    crate::layout::elements::ExternalElementData::Image { .. } => {
                        assert_eq!(element.size.height, 400.0);
                        assert_eq!(element.size.width, 400.0);
                        found = true;
                    }
                }
            }
        }

        assert!(found, "Image should be present in the layout");
    }
}

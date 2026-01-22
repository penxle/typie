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
    pub src: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
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
        if self.src.is_none() {
            return None;
        }

        let mut spec = DomSpec::el("img").attr("src", self.src.clone().unwrap());

        if let Some(width) = self.width {
            spec = spec.attr("width", width.to_string());
        }

        if let Some(height) = self.height {
            spec = spec.attr("height", height.to_string());
        }

        Some(
            spec.attr("data-proportion", self.proportion.to_string())
                .void(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("img", |elem| {
            let src = elem.value().attr("src").map(|s| s.to_string());
            let width = elem.value().attr("width").and_then(|s| s.parse().ok());
            let height = elem.value().attr("height").and_then(|s| s.parse().ok());
            let proportion = elem
                .value()
                .attr("data-proportion")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0);
            Some(Node::Image(ImageNode {
                src,
                width,
                height,
                proportion,
                upload_id: None,
            }))
        })]
    }
}

impl Default for ImageNode {
    fn default() -> Self {
        Self {
            src: None,
            width: None,
            height: None,
            proportion: 1.0,
            upload_id: None,
        }
    }
}

impl Hash for ImageNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.src.hash(state);
        self.width.map(|w| w.to_bits()).hash(state);
        self.height.map(|h| h.to_bits()).hash(state);
        self.proportion.to_bits().hash(state);
    }
}

impl Layout for ImageNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        const PLACEHOLDER_HEIGHT: f32 = 48.0;

        let max_width = constraints.max_width;
        let max_height = constraints.max_height;

        let dimensions = ctx
            .view_states
            .get(&ctx.node.node_id())
            .and_then(|s| s.image_dimensions())
            .or_else(|| self.width.zip(self.height));

        let display_height = if let Some((w, h)) = dimensions {
            let proportioned_width = w * self.proportion;
            let proportioned_height = h * self.proportion;

            let scale = (max_width / proportioned_width)
                .min(max_height / proportioned_height)
                .min(1.0);

            proportioned_height * scale
        } else {
            PLACEHOLDER_HEIGHT
        };

        let data = ExternalElementData::Image {
            src: self.src.clone(),
            original_width: dimensions.map(|(w, _)| w).or(self.width),
            original_height: dimensions.map(|(_, h)| h).or(self.height),
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
    fn test_image_layout_height_limit_in_paginated_mode() {
        let mut p = id!();
        let runtime = runtime! {
            viewport {
                paginated { width: 800.0, height: 500.0, margin: 50.0 }
            }
            doc {
               @p image (
                   src: Some("test.png".to_string()),
                   width: Some(1000.0),
                   height: Some(1000.0),
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
                        assert_eq!(element.size.width, 700.0); // 800 - 2 * 50
                        found = true;
                    }
                }
            }
        }

        assert!(found, "Image should be present in the layout");
    }
}

use crate::model::Text;
use crate::model::html::{DomSpec, NodeHtmlCodec, StyleHtmlCodec};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TextNode {
    pub text: Text,
}

impl NodeHtmlCodec for TextNode {
    fn to_dom(&self) -> Option<DomSpec> {
        let specs: Vec<DomSpec> = self
            .text
            .get_segments()
            .into_iter()
            .map(|seg| {
                let style_specs: Vec<DomSpec> = seg
                    .styles
                    .iter()
                    .map(|s| StyleHtmlCodec::to_dom(s))
                    .collect();
                DomSpec::wrap_with_styles(seg.text, style_specs)
            })
            .collect();

        Some(DomSpec::Fragment(specs))
    }
}

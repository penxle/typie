use crate::model::Text;
use crate::model::html::{DomSpec, MarkHtmlCodec, NodeHtmlCodec};
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
            .get_rich_text_segments()
            .into_iter()
            .map(|(text, marks)| {
                if marks.is_empty() {
                    DomSpec::Text(text)
                } else {
                    let mark_specs: Vec<DomSpec> = marks.iter().map(|m| m.to_dom()).collect();
                    DomSpec::wrap_with_marks(text, mark_specs)
                }
            })
            .collect();

        Some(DomSpec::Fragment(specs))
    }
}

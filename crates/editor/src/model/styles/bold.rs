use crate::model::html::{DomSpec, StyleHtmlCodec};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct BoldStyle {}

impl StyleHtmlCodec for BoldStyle {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("span")
            .style("font-weight:bold".to_string())
            .hole()
    }
}

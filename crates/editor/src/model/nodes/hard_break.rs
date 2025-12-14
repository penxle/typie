use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use macros::Codec;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
pub struct HardBreakNode {}

impl NodeHtmlCodec for HardBreakNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("br").void())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("br", |_| {
            Some(Node::HardBreak(HardBreakNode {}))
        })]
    }
}

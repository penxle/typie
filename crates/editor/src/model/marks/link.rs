use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule};
use macros::Codec;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
pub struct LinkMark {
    pub href: String,
}

impl Default for LinkMark {
    fn default() -> Self {
        Self {
            href: String::new(),
        }
    }
}

impl MarkHtmlCodec for LinkMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("a")
            .attr("href", &self.href)
            .attr("target", "_blank")
            .attr("rel", "noreferrer nofollow")
            .hole()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![MarkParseRule::from_tag("a", |elem| {
            elem.value()
                .attr("href")
                .map(|href| Mark::Link(LinkMark { href: href.into() }))
        })]
    }
}

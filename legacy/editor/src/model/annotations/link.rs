use crate::model::Annotation;
use crate::model::html::{AnnotationHtmlCodec, AnnotationParseRule, DomSpec};
use macros::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LinkAnnotation {
    pub href: String,
}

impl Default for LinkAnnotation {
    fn default() -> Self {
        Self {
            href: String::new(),
        }
    }
}

impl AnnotationHtmlCodec for LinkAnnotation {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("a")
            .attr("href", &self.href)
            .attr("target", "_blank")
            .attr("rel", "noreferrer nofollow")
            .hole()
    }

    fn parse_rules() -> Vec<AnnotationParseRule> {
        vec![AnnotationParseRule::from_tag("a", |elem| {
            elem.value()
                .attr("href")
                .map(|href| Annotation::Link(LinkAnnotation { href: href.into() }))
        })]
    }
}

use crate::model::Annotation;
use crate::model::html::{AnnotationHtmlCodec, AnnotationParseRule, DomSpec};
use macros::Codec;
use scraper::{ElementRef, Node as ScraperNode, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RubyAnnotation {
    pub text: String,
}

impl Default for RubyAnnotation {
    fn default() -> Self {
        Self {
            text: String::new(),
        }
    }
}

fn get_ruby_base_content(elem: &ElementRef) -> String {
    elem.children()
        .filter_map(|c| match c.value() {
            ScraperNode::Text(t) => Some(t.text.to_string()),
            ScraperNode::Element(e) if e.name() != "rt" && e.name() != "rp" => {
                ElementRef::wrap(c).map(|e| e.text().collect())
            }
            _ => None,
        })
        .collect()
}

impl AnnotationHtmlCodec for RubyAnnotation {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("ruby")
            .child(DomSpec::Hole)
            .child(DomSpec::el("rp").text("("))
            .child(DomSpec::el("rt").text(&self.text))
            .child(DomSpec::el("rp").text(")"))
            .build()
    }

    fn parse_rules() -> Vec<AnnotationParseRule> {
        vec![AnnotationParseRule::from_tag_with_content(
            "ruby",
            |elem| {
                let rt_sel = Selector::parse("rt").unwrap();
                let ruby_text = elem
                    .select(&rt_sel)
                    .next()
                    .map(|rt| rt.text().collect::<String>())
                    .unwrap_or_default();
                Some(Annotation::Ruby(RubyAnnotation { text: ruby_text }))
            },
            get_ruby_base_content,
        )]
    }
}

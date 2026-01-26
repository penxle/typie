use crate::model::Mark;
use crate::model::html::{DomSpec, MarkHtmlCodec, MarkParseRule};
use macros::Codec;
use scraper::{ElementRef, Node as ScraperNode, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RubyMark {
    pub text: String,
}

impl Default for RubyMark {
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

impl MarkHtmlCodec for RubyMark {
    fn to_dom(&self) -> DomSpec {
        DomSpec::el("ruby")
            .child(DomSpec::Hole)
            .child(DomSpec::el("rp").text("("))
            .child(DomSpec::el("rt").text(&self.text))
            .child(DomSpec::el("rp").text(")"))
            .build()
    }

    fn parse_rules() -> Vec<MarkParseRule> {
        vec![MarkParseRule::from_tag_with_content(
            "ruby",
            |elem| {
                let rt_sel = Selector::parse("rt").unwrap();
                let ruby_text = elem
                    .select(&rt_sel)
                    .next()
                    .map(|rt| rt.text().collect::<String>())
                    .unwrap_or_default();
                Some(Mark::Ruby(RubyMark { text: ruby_text }))
            },
            get_ruby_base_content,
        )]
    }
}

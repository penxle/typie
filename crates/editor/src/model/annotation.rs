use loro::LoroValue;
use serde::{Deserialize, Serialize};

use super::annotations::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    Link,
    Ruby,
}

impl AnnotationType {
    pub fn key(&self) -> &'static str {
        match self {
            AnnotationType::Link => "annotation:link",
            AnnotationType::Ruby => "annotation:ruby",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Annotation {
    Link(LinkAnnotation),
    Ruby(RubyAnnotation),
}

impl crate::model::html::AnnotationHtmlCodec for Annotation {
    fn to_dom(&self) -> crate::model::html::DomSpec {
        match self {
            Annotation::Link(a) => crate::model::html::AnnotationHtmlCodec::to_dom(a),
            Annotation::Ruby(a) => crate::model::html::AnnotationHtmlCodec::to_dom(a),
        }
    }
}

impl Annotation {
    pub fn as_type(&self) -> AnnotationType {
        match self {
            Annotation::Link(_) => AnnotationType::Link,
            Annotation::Ruby(_) => AnnotationType::Ruby,
        }
    }

    pub fn key(&self) -> &'static str {
        self.as_type().key()
    }

    pub fn to_loro_value(&self) -> LoroValue {
        use rustc_hash::FxHashMap;
        let mut map = FxHashMap::default();
        match self {
            Annotation::Link(l) => {
                map.insert("href".to_string(), LoroValue::String(l.href.clone().into()));
            }
            Annotation::Ruby(r) => {
                map.insert("text".to_string(), LoroValue::String(r.text.clone().into()));
            }
        }
        LoroValue::Map(map.into())
    }

    pub fn from_key_value(key: &str, value: &LoroValue) -> Option<Self> {
        let ann_key = key.strip_prefix("annotation:")?;
        let map = match value {
            LoroValue::Map(m) => m,
            _ => return None,
        };
        match ann_key {
            "link" => {
                let href = match map.get("href")? {
                    LoroValue::String(s) => s.to_string(),
                    _ => return None,
                };
                Some(Annotation::Link(LinkAnnotation { href }))
            }
            "ruby" => {
                let text = match map.get("text")? {
                    LoroValue::String(s) => s.to_string(),
                    _ => return None,
                };
                Some(Annotation::Ruby(RubyAnnotation { text }))
            }
            _ => None,
        }
    }
}

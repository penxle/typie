use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::annotations::*;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct AnnotationId(Uuid);

impl AnnotationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    pub fn loro_key(&self) -> String {
        format!("annotation:{}", self.0)
    }

    pub fn parse_loro_key(key: &str) -> Option<Self> {
        key.strip_prefix("annotation:")
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(Self)
    }
}

impl std::fmt::Display for AnnotationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum AnnotationType {
    Link,
    Ruby,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    pub fn encode_to_map(&self, map: &loro::LoroMap) -> anyhow::Result<()> {
        use crate::model::Codec;
        match self {
            Annotation::Link(inner) => {
                map.insert("type", "link")?;
                let mut inner = inner.clone();
                inner.encode(map)?;
            }
            Annotation::Ruby(inner) => {
                map.insert("type", "ruby")?;
                let mut inner = inner.clone();
                inner.encode(map)?;
            }
        }
        Ok(())
    }

    pub fn decode_from_map(map: &loro::LoroMap) -> Option<Annotation> {
        use crate::model::Codec;
        let type_value = map.get("type")?.into_value().ok()?;
        let type_str = match type_value {
            loro::LoroValue::String(s) => s.to_string(),
            _ => return None,
        };
        match type_str.as_str() {
            "link" => LinkAnnotation::decode(map).ok().map(Annotation::Link),
            "ruby" => RubyAnnotation::decode(map).ok().map(Annotation::Ruby),
            _ => None,
        }
    }
}

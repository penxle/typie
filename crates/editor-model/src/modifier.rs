use editor_macros::ffi;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumDiscriminants, EnumIter};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(ModifierType))]
#[strum_discriminants(ffi)]
#[strum_discriminants(derive(
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumIter,
    EnumCount,
    Enum,
))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Modifier {
    Bold,
    Italic,
    Underline,
    Strikethrough,

    /// pt x 100 (e.g. 16pt -> 1600)
    FontSize(u32),
    FontFamily(String),
    FontWeight(u16),
    TextColor(String),
    BackgroundColor(String),
    /// em x 100 (e.g. 0.05em -> 5)
    LetterSpacing(i32),

    Link {
        href: String,
    },
    Ruby {
        text: String,
    },

    /// % (e.g. 160 -> 160%)
    LineHeight(u32),
    /// x 100 (e.g. 100% -> 100)
    BlockGap(u32),
    /// x 100 (e.g. 100% -> 100)
    ParagraphIndent(u32),
}

impl Modifier {
    pub fn as_type(&self) -> ModifierType {
        ModifierType::from(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_type_discriminant() {
        assert_eq!(Modifier::Bold.as_type(), ModifierType::Bold);
        assert_eq!(Modifier::FontSize(1600).as_type(), ModifierType::FontSize);
        assert_eq!(
            Modifier::Link {
                href: "x".to_string()
            }
            .as_type(),
            ModifierType::Link
        );
        assert_eq!(
            Modifier::LineHeight(160).as_type(),
            ModifierType::LineHeight
        );
    }

    #[test]
    fn as_type_count() {
        assert_eq!(ModifierType::COUNT, 15);
    }

    #[test]
    fn serde_unit_variant() {
        let m = Modifier::Bold;
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"bold"}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn serde_tuple_variant() {
        let m = Modifier::FontSize(1600);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"font_size","value":1600}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn serde_struct_variant() {
        let m = Modifier::Link {
            href: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
        assert!(json.contains(r#""type":"link""#));
        assert!(json.contains(r#""href":"https://example.com""#));
    }

    #[test]
    fn serde_block_gap() {
        let m = Modifier::BlockGap(100);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"block_gap","value":100}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn serde_paragraph_indent() {
        let m = Modifier::ParagraphIndent(200);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"paragraph_indent","value":200}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn eq_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Modifier::Bold);
        set.insert(Modifier::Bold);
        assert_eq!(set.len(), 1);

        set.insert(Modifier::FontSize(1600));
        set.insert(Modifier::FontSize(1200));
        assert_eq!(set.len(), 3);
    }
}

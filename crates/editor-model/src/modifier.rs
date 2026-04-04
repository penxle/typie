use editor_macros::ffi;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoStaticStr};

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
    IntoStaticStr,
))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Modifier {
    Bold,
    Italic,
    Underline,
    Strikethrough,

    /// pt x 100 (e.g. 16pt -> 1600)
    FontSize {
        value: u32,
    },

    FontFamily {
        value: String,
    },

    FontWeight {
        value: u16,
    },

    TextColor {
        value: String,
    },

    BackgroundColor {
        value: String,
    },

    /// em x 100 (e.g. 0.05em -> 5)
    LetterSpacing {
        value: i32,
    },

    Link {
        href: String,
    },

    Ruby {
        text: String,
    },

    /// % (e.g. 160 -> 160%)
    LineHeight {
        value: u32,
    },

    /// x 100 (e.g. 100% -> 100)
    BlockGap {
        value: u32,
    },

    /// x 100 (e.g. 100% -> 100)
    ParagraphIndent {
        value: u32,
    },
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
        assert_eq!(
            Modifier::FontSize { value: 1600 }.as_type(),
            ModifierType::FontSize
        );
        assert_eq!(
            Modifier::Link {
                href: "x".to_string()
            }
            .as_type(),
            ModifierType::Link
        );
        assert_eq!(
            Modifier::LineHeight { value: 160 }.as_type(),
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
        let m = Modifier::FontSize { value: 1600 };
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
        // internally tagged: {"type":"link","href":"https://example.com"}
        assert_eq!(json, r#"{"type":"link","href":"https://example.com"}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn serde_block_gap() {
        let m = Modifier::BlockGap { value: 100 };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"block_gap","value":100}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn serde_paragraph_indent() {
        let m = Modifier::ParagraphIndent { value: 200 };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"paragraph_indent","value":200}"#);
        let parsed: Modifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn eq_and_hash() {
        use hashbrown::HashSet;
        let mut set = HashSet::new();
        set.insert(Modifier::Bold);
        set.insert(Modifier::Bold);
        assert_eq!(set.len(), 1);

        set.insert(Modifier::FontSize { value: 1600 });
        set.insert(Modifier::FontSize { value: 1200 });
        assert_eq!(set.len(), 3);
    }
}

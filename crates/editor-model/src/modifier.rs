use crate::alignment::Alignment;
use editor_common::Tri;
use editor_macros::ffi;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumDiscriminants, EnumIter, IntoStaticStr};

#[ffi]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    EnumDiscriminants,
    editor_macros::Wire,
    editor_macros::ModifierState,
)]
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
    editor_macros::Wire,
))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[serde(tag = "type", rename_all = "snake_case")]
#[modifier_state(computed(effective_bold))]
pub enum Modifier {
    #[wire(n(0))]
    #[strum_discriminants(wire(n(0)))]
    Bold,
    #[wire(n(1))]
    #[strum_discriminants(wire(n(1)))]
    Italic,
    #[wire(n(2))]
    #[strum_discriminants(wire(n(2)))]
    Underline,
    #[wire(n(3))]
    #[strum_discriminants(wire(n(3)))]
    Strikethrough,

    /// pt x 100 (e.g. 16pt -> 1600)
    #[wire(n(4))]
    #[strum_discriminants(wire(n(4)))]
    FontSize {
        #[wire(n(0))]
        value: u32,
    },

    #[wire(n(5))]
    #[strum_discriminants(wire(n(5)))]
    FontFamily {
        #[wire(n(0))]
        value: String,
    },

    #[wire(n(6))]
    #[strum_discriminants(wire(n(6)))]
    FontWeight {
        #[wire(n(0))]
        value: u16,
    },

    #[wire(n(7))]
    #[strum_discriminants(wire(n(7)))]
    TextColor {
        #[wire(n(0))]
        value: String,
    },

    #[wire(n(8))]
    #[strum_discriminants(wire(n(8)))]
    BackgroundColor {
        #[wire(n(0))]
        value: String,
    },

    /// em x 100 (e.g. 0.05em -> 5)
    #[wire(n(9))]
    #[strum_discriminants(wire(n(9)))]
    LetterSpacing {
        #[wire(n(0))]
        value: i32,
    },

    #[wire(n(10))]
    #[strum_discriminants(wire(n(10)))]
    Link {
        #[wire(n(0))]
        href: String,
    },

    #[wire(n(11))]
    #[strum_discriminants(wire(n(11)))]
    Ruby {
        #[wire(n(0))]
        text: String,
    },

    /// % (e.g. 160 -> 160%)
    #[wire(n(12))]
    #[strum_discriminants(wire(n(12)))]
    LineHeight {
        #[wire(n(0))]
        value: u32,
    },

    /// x 100 (e.g. 100% -> 100)
    #[wire(n(13))]
    #[strum_discriminants(wire(n(13)))]
    BlockGap {
        #[wire(n(0))]
        value: u32,
    },

    /// x 100 (e.g. 100% -> 100)
    #[wire(n(14))]
    #[strum_discriminants(wire(n(14)))]
    ParagraphIndent {
        #[wire(n(0))]
        value: u32,
    },

    #[wire(n(15))]
    #[strum_discriminants(wire(n(15)))]
    Alignment {
        #[wire(n(0))]
        value: Alignment,
    },
}

impl Modifier {
    pub fn as_type(&self) -> ModifierType {
        ModifierType::from(self)
    }
}

pub const DEFAULT_FONT_FAMILY: &str = "";
pub const DEFAULT_FONT_SIZE: u32 = 1200;
pub const DEFAULT_FONT_WEIGHT: u16 = 400;
pub const DEFAULT_LETTER_SPACING: i32 = 0;
pub const DEFAULT_LINE_HEIGHT: u32 = 160;

pub fn text_style_default_modifier(ty: ModifierType) -> Option<Modifier> {
    match ty {
        ModifierType::FontFamily => Some(Modifier::FontFamily {
            value: DEFAULT_FONT_FAMILY.to_string(),
        }),
        ModifierType::FontSize => Some(Modifier::FontSize {
            value: DEFAULT_FONT_SIZE,
        }),
        ModifierType::FontWeight => Some(Modifier::FontWeight {
            value: DEFAULT_FONT_WEIGHT,
        }),
        ModifierType::LetterSpacing => Some(Modifier::LetterSpacing {
            value: DEFAULT_LETTER_SPACING,
        }),
        ModifierType::LineHeight => Some(Modifier::LineHeight {
            value: DEFAULT_LINE_HEIGHT,
        }),
        _ => None,
    }
}

impl ModifierState {
    pub fn set_uniform(&mut self, m: &Modifier) {
        match m {
            Modifier::Bold => self.bold = Tri::Uniform { value: () },
            Modifier::Italic => self.italic = Tri::Uniform { value: () },
            Modifier::Underline => self.underline = Tri::Uniform { value: () },
            Modifier::Strikethrough => self.strikethrough = Tri::Uniform { value: () },
            Modifier::FontSize { value } => {
                self.font_size = Tri::Uniform {
                    value: FontSizeValue { value: *value },
                }
            }
            Modifier::FontFamily { value } => {
                self.font_family = Tri::Uniform {
                    value: FontFamilyValue {
                        value: value.clone(),
                    },
                }
            }
            Modifier::FontWeight { value } => {
                self.font_weight = Tri::Uniform {
                    value: FontWeightValue { value: *value },
                }
            }
            Modifier::TextColor { value } => {
                self.text_color = Tri::Uniform {
                    value: TextColorValue {
                        value: value.clone(),
                    },
                }
            }
            Modifier::BackgroundColor { value } => {
                self.background_color = Tri::Uniform {
                    value: BackgroundColorValue {
                        value: value.clone(),
                    },
                }
            }
            Modifier::LetterSpacing { value } => {
                self.letter_spacing = Tri::Uniform {
                    value: LetterSpacingValue { value: *value },
                }
            }
            Modifier::Link { href } => {
                self.link = Tri::Uniform {
                    value: LinkValue { href: href.clone() },
                }
            }
            Modifier::Ruby { text } => {
                self.ruby = Tri::Uniform {
                    value: RubyValue { text: text.clone() },
                }
            }
            Modifier::LineHeight { value } => {
                self.line_height = Tri::Uniform {
                    value: LineHeightValue { value: *value },
                }
            }
            Modifier::BlockGap { value } => {
                self.block_gap = Tri::Uniform {
                    value: BlockGapValue { value: *value },
                }
            }
            Modifier::ParagraphIndent { value } => {
                self.paragraph_indent = Tri::Uniform {
                    value: ParagraphIndentValue { value: *value },
                }
            }
            Modifier::Alignment { value } => {
                self.alignment = Tri::Uniform {
                    value: AlignmentValue { value: *value },
                }
            }
        }
    }

    pub fn set_mixed(&mut self, t: ModifierType) {
        match t {
            ModifierType::Bold => self.bold = Tri::Mixed,
            ModifierType::Italic => self.italic = Tri::Mixed,
            ModifierType::Underline => self.underline = Tri::Mixed,
            ModifierType::Strikethrough => self.strikethrough = Tri::Mixed,
            ModifierType::FontSize => self.font_size = Tri::Mixed,
            ModifierType::FontFamily => self.font_family = Tri::Mixed,
            ModifierType::FontWeight => self.font_weight = Tri::Mixed,
            ModifierType::TextColor => self.text_color = Tri::Mixed,
            ModifierType::BackgroundColor => self.background_color = Tri::Mixed,
            ModifierType::LetterSpacing => self.letter_spacing = Tri::Mixed,
            ModifierType::Link => self.link = Tri::Mixed,
            ModifierType::Ruby => self.ruby = Tri::Mixed,
            ModifierType::LineHeight => self.line_height = Tri::Mixed,
            ModifierType::BlockGap => self.block_gap = Tri::Mixed,
            ModifierType::ParagraphIndent => self.paragraph_indent = Tri::Mixed,
            ModifierType::Alignment => self.alignment = Tri::Mixed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_state_default_has_absent_effective_bold() {
        let s = ModifierState::default();
        assert_eq!(s.effective_bold, editor_common::Tri::Absent);
    }

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
        assert_eq!(
            Modifier::Alignment {
                value: Alignment::Center
            }
            .as_type(),
            ModifierType::Alignment
        );
    }

    #[test]
    fn as_type_count() {
        assert_eq!(ModifierType::COUNT, 16);
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
    fn serde_alignment() {
        let m = Modifier::Alignment {
            value: Alignment::Center,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"type":"alignment","value":"center"}"#);
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

    #[test]
    fn modifier_state_default_all_absent() {
        let s = ModifierState::default();
        assert_eq!(s.bold, editor_common::Tri::Absent);
        assert_eq!(s.font_size, editor_common::Tri::Absent);
        assert_eq!(s.link, editor_common::Tri::Absent);
    }

    #[test]
    fn set_uniform_bold() {
        let mut s = ModifierState::default();
        s.set_uniform(&Modifier::Bold);
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn set_uniform_font_size() {
        let mut s = ModifierState::default();
        s.set_uniform(&Modifier::FontSize { value: 1600 });
        assert_eq!(
            s.font_size,
            editor_common::Tri::Uniform {
                value: FontSizeValue { value: 1600 }
            }
        );
    }

    #[test]
    fn set_mixed_bold() {
        let mut s = ModifierState::default();
        s.set_mixed(ModifierType::Bold);
        assert_eq!(s.bold, editor_common::Tri::Mixed);
    }

    #[test]
    fn set_uniform_link_preserves_href_field_name() {
        // Pin down that the macro emits LinkValue { href: ... } (not LinkValue { value: ... }),
        // and that set_uniform threads `href` through correctly.
        let mut s = ModifierState::default();
        s.set_uniform(&Modifier::Link {
            href: "https://example.com".to_string(),
        });
        assert_eq!(
            s.link,
            Tri::Uniform {
                value: LinkValue {
                    href: "https://example.com".to_string()
                }
            }
        );
    }

    #[test]
    fn set_uniform_alignment_uses_copy_path() {
        let mut s = ModifierState::default();
        s.set_uniform(&Modifier::Alignment {
            value: Alignment::Center,
        });
        assert_eq!(
            s.alignment,
            Tri::Uniform {
                value: AlignmentValue {
                    value: Alignment::Center
                }
            }
        );
    }

    #[test]
    fn modifier_wire_round_trip_all_variants() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        let cases = vec![
            Modifier::Bold,
            Modifier::Italic,
            Modifier::Underline,
            Modifier::Strikethrough,
            Modifier::FontSize { value: 1600 },
            Modifier::FontFamily {
                value: "Pretendard".to_owned(),
            },
            Modifier::FontWeight { value: 700 },
            Modifier::TextColor {
                value: "#ff0000".to_owned(),
            },
            Modifier::BackgroundColor {
                value: "#00ff00".to_owned(),
            },
            Modifier::LetterSpacing { value: -5 },
            Modifier::Link {
                href: "https://example.com".to_owned(),
            },
            Modifier::Ruby {
                text: "ruby".to_owned(),
            },
            Modifier::LineHeight { value: 160 },
            Modifier::BlockGap { value: 100 },
            Modifier::ParagraphIndent { value: 100 },
            Modifier::Alignment {
                value: Alignment::Center,
            },
        ];
        for v in cases {
            let mut buf = Vec::new();
            <Modifier as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <Modifier as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }

    #[test]
    fn modifier_type_wire_round_trip_all_variants() {
        use editor_crdt::wire::{DecCtx, EncCtx, Wire};
        use strum::IntoEnumIterator;
        let ec = EncCtx::from_table(&[], vec![]);
        let dc = DecCtx {
            actor_table: vec![],
            baselines: vec![],
        };
        for v in ModifierType::iter() {
            let mut buf = Vec::new();
            <ModifierType as Wire>::encode(&v, &ec, &mut buf).unwrap();
            let mut slice = &buf[..];
            let got = <ModifierType as Wire>::decode(&dc, &mut slice).unwrap();
            assert_eq!(got, v);
        }
    }
}

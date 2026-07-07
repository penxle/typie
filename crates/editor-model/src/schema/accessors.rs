use crate::{Modifier, ModifierType, Node, NodeType};

use super::{ModifierSpec, NodeSpec, Schema};

impl NodeType {
    pub fn spec(self) -> &'static NodeSpec {
        Schema::node_spec(self)
    }
}

impl Node {
    pub fn spec(&self) -> &'static NodeSpec {
        Schema::node_spec(self.as_type())
    }
}

impl ModifierType {
    pub fn spec(self) -> &'static ModifierSpec {
        Schema::modifier_spec(self)
    }

    pub fn is_carry_kind(self) -> bool {
        matches!(
            self,
            ModifierType::Bold
                | ModifierType::Italic
                | ModifierType::Underline
                | ModifierType::Strikethrough
                | ModifierType::FontSize
                | ModifierType::FontFamily
                | ModifierType::FontWeight
                | ModifierType::TextColor
                | ModifierType::BackgroundColor
                | ModifierType::LetterSpacing
        )
    }

    pub fn is_text_applicable(self) -> bool {
        Schema::modifier_spec(self)
            .target
            .rightmost_node_types()
            .contains(&NodeType::Text)
    }
}

impl Modifier {
    pub fn spec(&self) -> &'static ModifierSpec {
        Schema::modifier_spec(self.as_type())
    }
}

#[cfg(test)]
mod tests {
    use crate::ModifierType;
    use strum::IntoEnumIterator;

    #[test]
    fn carry_kinds_are_the_ten_character_styles() {
        let carry: Vec<ModifierType> = ModifierType::iter().filter(|t| t.is_carry_kind()).collect();
        assert_eq!(carry.len(), 10);
        for ty in [
            ModifierType::Bold,
            ModifierType::Italic,
            ModifierType::Underline,
            ModifierType::Strikethrough,
            ModifierType::FontSize,
            ModifierType::FontFamily,
            ModifierType::FontWeight,
            ModifierType::TextColor,
            ModifierType::BackgroundColor,
            ModifierType::LetterSpacing,
        ] {
            assert!(ty.is_carry_kind(), "{ty:?} must be a carry kind");
        }
    }

    #[test]
    fn non_carry_kinds_are_link_ruby_and_block_styles() {
        for ty in [
            ModifierType::Link,
            ModifierType::Ruby,
            ModifierType::LineHeight,
            ModifierType::BlockGap,
            ModifierType::ParagraphIndent,
            ModifierType::Alignment,
        ] {
            assert!(!ty.is_carry_kind(), "{ty:?} must not be a carry kind");
        }
    }
}

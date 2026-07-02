use editor_model::{
    DEFAULT_FONT_FAMILY, DEFAULT_FONT_SIZE, DEFAULT_FONT_WEIGHT, DEFAULT_LETTER_SPACING,
    DEFAULT_LINE_HEIGHT, Modifier, ModifierType,
};
use editor_state::{PendingModifier, PendingModifiers};

#[derive(Clone)]
pub struct ResolvedTextStyle {
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub line_height: f32,
}

const PT_TO_PX: f32 = 96.0 / 72.0;
const DEFAULT_FONT_SIZE_PX: f32 = DEFAULT_FONT_SIZE as f32 / 100.0 * PT_TO_PX;
const DEFAULT_LINE_HEIGHT_RATIO: f32 = DEFAULT_LINE_HEIGHT as f32 / 100.0;

/// `resolve_text_style`와 달리 ancestor 순회·Expand 필터링을 하지 않는다 — 호출자가
/// 미리 평탄화한 effective set을 넘긴다는 계약.
pub fn style_from_effective_modifiers(modifiers: &[Modifier]) -> ResolvedTextStyle {
    let mut font_family: Option<String> = None;
    let mut font_weight: Option<u16> = None;
    let mut font_size: Option<f32> = None;
    let mut letter_spacing: Option<f32> = None;
    let mut line_height: Option<f32> = None;

    for m in modifiers {
        match m {
            Modifier::FontFamily { value } if font_family.is_none() => {
                font_family = Some(value.clone());
            }
            Modifier::FontWeight { value } if font_weight.is_none() => {
                font_weight = Some(*value);
            }
            Modifier::FontSize { value } if font_size.is_none() => {
                let pt = *value as f32 / 100.0;
                font_size = Some(pt * PT_TO_PX);
            }
            Modifier::LetterSpacing { value } if letter_spacing.is_none() => {
                letter_spacing = Some(*value as f32 / 100.0);
            }
            Modifier::LineHeight { value } if line_height.is_none() => {
                line_height = Some(*value as f32 / 100.0);
            }
            _ => {}
        }
    }

    let final_font_size = font_size.unwrap_or(DEFAULT_FONT_SIZE_PX);
    let ls_em = letter_spacing.unwrap_or(DEFAULT_LETTER_SPACING as f32 / 100.0);

    ResolvedTextStyle {
        font_family: font_family.unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string()),
        font_weight: font_weight.unwrap_or(DEFAULT_FONT_WEIGHT),
        font_size: final_font_size,
        letter_spacing: ls_em * final_font_size,
        line_height: line_height.unwrap_or(DEFAULT_LINE_HEIGHT_RATIO),
    }
}

pub fn apply_pending_to_style(style: &mut ResolvedTextStyle, pending: &PendingModifiers) {
    let mut font_family = style.font_family.clone();
    let mut font_weight = style.font_weight;
    let mut font_size = style.font_size;
    let mut letter_spacing_em = if style.font_size > 0.0 {
        style.letter_spacing / style.font_size
    } else {
        0.0
    };
    let mut line_height = style.line_height;

    for p in pending {
        match p {
            PendingModifier::Set { modifier } => match modifier {
                Modifier::FontFamily { value } => font_family = value.clone(),
                Modifier::FontWeight { value } => font_weight = *value,
                Modifier::FontSize { value } => {
                    // centipoints → pixels (resolve_text_style과 동일 변환).
                    font_size = (*value as f32 / 100.0) * PT_TO_PX;
                }
                Modifier::LetterSpacing { value } => {
                    letter_spacing_em = *value as f32 / 100.0;
                }
                Modifier::LineHeight { value } => {
                    line_height = *value as f32 / 100.0;
                }
                _ => {}
            },
            PendingModifier::Unset { ty } => match ty {
                ModifierType::FontFamily => font_family = DEFAULT_FONT_FAMILY.to_string(),
                ModifierType::FontWeight => font_weight = DEFAULT_FONT_WEIGHT,
                ModifierType::FontSize => font_size = DEFAULT_FONT_SIZE_PX,
                ModifierType::LetterSpacing => {
                    letter_spacing_em = DEFAULT_LETTER_SPACING as f32 / 100.0;
                }
                ModifierType::LineHeight => line_height = DEFAULT_LINE_HEIGHT_RATIO,
                _ => {}
            },
        }
    }

    style.font_family = font_family;
    style.font_weight = font_weight;
    style.font_size = font_size;
    style.letter_spacing = letter_spacing_em * font_size;
    style.line_height = line_height;
}

#[cfg(test)]
mod tests {

    use super::*;
    use editor_model::ModifierType;

    #[test]
    fn style_from_effective_modifiers_empty_uses_defaults() {
        let style = style_from_effective_modifiers(&[]);
        assert_eq!(style.font_weight, DEFAULT_FONT_WEIGHT);
        assert!((style.font_size - DEFAULT_FONT_SIZE_PX).abs() < 0.01);
        assert!((style.line_height - DEFAULT_LINE_HEIGHT_RATIO).abs() < 0.01);
        assert!((style.letter_spacing - 0.0).abs() < 0.01);
        assert_eq!(style.font_family, "");
    }

    #[test]
    fn style_from_effective_modifiers_applies_font_size() {
        let style = style_from_effective_modifiers(&[Modifier::FontSize { value: 2400 }]);
        // 24pt * (96/72) = 32px
        assert!((style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn style_from_effective_modifiers_first_wins_per_type() {
        let style = style_from_effective_modifiers(&[
            Modifier::FontSize { value: 1200 },
            Modifier::FontSize { value: 2400 },
        ]);
        // 12pt * (96/72) = 16px
        assert!((style.font_size - 16.0).abs() < 0.01);
    }

    #[test]
    fn apply_pending_font_size_overrides_base() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.5,
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        apply_pending_to_style(&mut style, &pending);
        // 96pt * (96/72) = 128px
        assert!((style.font_size - 128.0).abs() < 0.01);
    }

    #[test]
    fn apply_pending_letter_spacing_uses_current_font_size() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 400,
            font_size: 20.0,
            letter_spacing: 0.0,
            line_height: 1.5,
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::LetterSpacing { value: 40 },
        }];

        apply_pending_to_style(&mut style, &pending);

        assert!((style.letter_spacing - 8.0).abs() < 0.01);
    }

    #[test]
    fn apply_pending_letter_spacing_uses_final_font_size_independent_of_order() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 400,
            font_size: 20.0,
            letter_spacing: 0.0,
            line_height: 1.5,
        };
        let pending: PendingModifiers = vec![
            PendingModifier::Set {
                modifier: Modifier::LetterSpacing { value: 10 },
            },
            PendingModifier::Set {
                modifier: Modifier::FontSize { value: 3000 },
            },
        ];

        apply_pending_to_style(&mut style, &pending);

        // 30pt * (96/72) = 40px, 0.1em = 4px
        assert!((style.font_size - 40.0).abs() < 0.01);
        assert!((style.letter_spacing - 4.0).abs() < 0.01);
    }

    #[test]
    fn apply_pending_font_size_rescales_existing_letter_spacing_em() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 400,
            font_size: 20.0,
            letter_spacing: 2.0,
            line_height: 1.5,
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::FontSize { value: 3000 },
        }];

        apply_pending_to_style(&mut style, &pending);

        assert!((style.font_size - 40.0).abs() < 0.01);
        assert!((style.letter_spacing - 4.0).abs() < 0.01);
    }

    #[test]
    fn apply_pending_unset_restores_default_style_values() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 700,
            font_size: 24.0,
            letter_spacing: 2.4,
            line_height: 2.0,
        };
        let pending: PendingModifiers = vec![
            PendingModifier::Unset {
                ty: ModifierType::FontFamily,
            },
            PendingModifier::Unset {
                ty: ModifierType::FontWeight,
            },
            PendingModifier::Unset {
                ty: ModifierType::FontSize,
            },
            PendingModifier::Unset {
                ty: ModifierType::LetterSpacing,
            },
            PendingModifier::Unset {
                ty: ModifierType::LineHeight,
            },
        ];

        apply_pending_to_style(&mut style, &pending);

        assert_eq!(style.font_family, "");
        assert_eq!(style.font_weight, DEFAULT_FONT_WEIGHT);
        assert!((style.font_size - DEFAULT_FONT_SIZE_PX).abs() < 0.01);
        assert!((style.letter_spacing - 0.0).abs() < 0.01);
        assert!((style.line_height - DEFAULT_LINE_HEIGHT_RATIO).abs() < 0.01);
    }

    #[test]
    fn apply_pending_line_height_overrides_base() {
        let mut style = ResolvedTextStyle {
            font_family: "test".into(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.5,
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::LineHeight { value: 220 },
        }];

        apply_pending_to_style(&mut style, &pending);

        assert!((style.line_height - 2.2).abs() < 0.01);
    }
}

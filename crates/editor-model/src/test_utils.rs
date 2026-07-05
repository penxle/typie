use crate::{Alignment, Modifier};

pub fn default_modifiers() -> Vec<Modifier> {
    vec![
        Modifier::FontFamily {
            value: "Pretendard".to_string(),
        },
        Modifier::FontSize { value: 1200 },
        Modifier::FontWeight { value: 400 },
        Modifier::LetterSpacing { value: 0 },
        Modifier::LineHeight { value: 160 },
        Modifier::Alignment {
            value: Alignment::Left,
        },
        Modifier::ParagraphIndent { value: 100 },
        Modifier::BlockGap { value: 100 },
    ]
}

pub fn default_modifiers_with(overrides: Vec<Modifier>) -> Vec<Modifier> {
    let override_types: Vec<_> = overrides.iter().map(|m| m.as_type()).collect();
    let mut mods: Vec<_> = default_modifiers()
        .into_iter()
        .filter(|m| !override_types.contains(&m.as_type()))
        .collect();
    mods.extend(overrides);
    mods
}

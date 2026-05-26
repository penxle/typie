use editor_model::Modifier;
use editor_resource::{Resource, find_bold_target, match_weight};

pub fn resolve_font_weight(
    modifiers: Vec<Modifier>,
    inline_mods: &[Modifier],
    resource: &Resource,
) -> Vec<Modifier> {
    let effective_family = find_family(&modifiers, inline_mods);
    let inherited_weight = effective_inherited_weight(inline_mods);

    let mut out: Vec<Modifier> = Vec::with_capacity(modifiers.len() + 1);
    let mut intent_bold = false;
    let mut intent_weight: Option<u16> = None;
    for m in modifiers {
        match m {
            Modifier::Bold => intent_bold = true,
            Modifier::FontWeight { value } => intent_weight = Some(value),
            other => out.push(other),
        }
    }

    if let Some(value) = intent_weight {
        match effective_family
            .as_deref()
            .and_then(|f| resource.font_registry.weights(f))
        {
            Some(available) => match match_weight(available, value) {
                Some(matched) => {
                    if value >= 700 && matched < 700 {
                        out.push(Modifier::Bold);
                    } else {
                        out.push(Modifier::FontWeight { value: matched });
                    }
                }
                None => {}
            },
            None => {
                out.push(Modifier::FontWeight { value });
            }
        }
    } else if intent_bold {
        match effective_family
            .as_deref()
            .and_then(|f| resource.font_registry.weights(f))
        {
            Some(available) => match find_bold_target(inherited_weight, available) {
                Some(target) => out.push(Modifier::FontWeight { value: target }),
                None => out.push(Modifier::Bold),
            },
            None => out.push(Modifier::Bold),
        }
    }

    out
}

pub fn effective_inherited_weight(inline_mods: &[Modifier]) -> u16 {
    if let Some(w) = inherited_font_weight(inline_mods) {
        return w;
    }
    if inline_mods.iter().any(|m| matches!(m, Modifier::Bold)) {
        return 700;
    }
    400
}

pub fn inherited_font_weight(inline_mods: &[Modifier]) -> Option<u16> {
    inline_mods.iter().find_map(|m| match m {
        Modifier::FontWeight { value } => Some(*value),
        _ => None,
    })
}

pub fn compute_relative_weight(keyword: &str, parent: u16) -> u16 {
    match keyword {
        "bolder" => match parent {
            w if w < 400 => 400,
            w if w < 550 => 700,
            w if w < 750 => 900,
            _ => parent,
        },
        "lighter" => match parent {
            w if w < 550 => 100,
            w if w < 750 => 400,
            _ => 700,
        },
        _ => parent,
    }
}

fn find_family(modifiers: &[Modifier], inline_mods: &[Modifier]) -> Option<String> {
    modifiers
        .iter()
        .find_map(|m| match m {
            Modifier::FontFamily { value } => Some(value.clone()),
            _ => None,
        })
        .or_else(|| {
            inline_mods.iter().find_map(|m| match m {
                Modifier::FontFamily { value } => Some(value.clone()),
                _ => None,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolder_below_400() {
        assert_eq!(compute_relative_weight("bolder", 100), 400);
        assert_eq!(compute_relative_weight("bolder", 300), 400);
    }

    #[test]
    fn bolder_400_to_549() {
        assert_eq!(compute_relative_weight("bolder", 400), 700);
        assert_eq!(compute_relative_weight("bolder", 500), 700);
        assert_eq!(compute_relative_weight("bolder", 549), 700);
    }

    #[test]
    fn bolder_550_to_749() {
        assert_eq!(compute_relative_weight("bolder", 550), 900);
        assert_eq!(compute_relative_weight("bolder", 700), 900);
        assert_eq!(compute_relative_weight("bolder", 749), 900);
    }

    #[test]
    fn bolder_at_or_above_750() {
        assert_eq!(compute_relative_weight("bolder", 750), 750);
        assert_eq!(compute_relative_weight("bolder", 900), 900);
    }

    #[test]
    fn lighter_below_550() {
        assert_eq!(compute_relative_weight("lighter", 100), 100);
        assert_eq!(compute_relative_weight("lighter", 400), 100);
        assert_eq!(compute_relative_weight("lighter", 549), 100);
    }

    #[test]
    fn lighter_550_to_749() {
        assert_eq!(compute_relative_weight("lighter", 550), 400);
        assert_eq!(compute_relative_weight("lighter", 700), 400);
    }

    #[test]
    fn lighter_at_or_above_750() {
        assert_eq!(compute_relative_weight("lighter", 750), 700);
        assert_eq!(compute_relative_weight("lighter", 900), 700);
    }
}

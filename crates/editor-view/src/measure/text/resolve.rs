use editor_model::{Modifier, ModifierType, Node, NodeRef};
use editor_state::{PendingModifier, PendingModifiers};

use crate::measure::resolve::resolve_inherited;

#[derive(Clone)]
pub struct ResolvedTextStyle {
    pub font_family: String,
    pub font_weight: u16,
    pub font_size: f32,
    pub letter_spacing: f32,
    pub line_height: f32,
}

const DEFAULT_FONT_SIZE_PX: f32 = 16.0;
const DEFAULT_FONT_WEIGHT: u16 = 400;
const DEFAULT_LINE_HEIGHT: f32 = 1.6;
const PT_TO_PX: f32 = 96.0 / 72.0;

pub fn resolve_text_style(node: &NodeRef<'_>) -> ResolvedTextStyle {
    let mut font_family: Option<String> = None;
    let mut font_weight: Option<u16> = None;
    let mut font_size: Option<f32> = None;
    let mut letter_spacing: Option<f32> = None;
    let mut line_height: Option<f32> = None;

    let mut resolved_count = 0u8;
    const TOTAL_PROPERTIES: u8 = 5;

    for ancestor in node.ancestors() {
        if resolved_count >= TOTAL_PROPERTIES {
            break;
        }
        for m in ancestor.modifiers_with_style() {
            match m {
                Modifier::FontFamily { value } if font_family.is_none() => {
                    font_family = Some(value.clone());
                    resolved_count += 1;
                }
                Modifier::FontWeight { value } if font_weight.is_none() => {
                    font_weight = Some(*value);
                    resolved_count += 1;
                }
                Modifier::FontSize { value } if font_size.is_none() => {
                    let pt = *value as f32 / 100.0;
                    font_size = Some(pt * PT_TO_PX);
                    resolved_count += 1;
                }
                Modifier::LetterSpacing { value } if letter_spacing.is_none() => {
                    letter_spacing = Some(*value as f32 / 100.0);
                    resolved_count += 1;
                }
                Modifier::LineHeight { value } if line_height.is_none() => {
                    line_height = Some(*value as f32 / 100.0);
                    resolved_count += 1;
                }
                _ => {}
            }
        }
    }

    let final_font_size = font_size.unwrap_or(DEFAULT_FONT_SIZE_PX);
    let ls_em = letter_spacing.unwrap_or(0.0);

    ResolvedTextStyle {
        font_family: font_family.unwrap_or_default(),
        font_weight: font_weight.unwrap_or(DEFAULT_FONT_WEIGHT),
        font_size: final_font_size,
        letter_spacing: ls_em * final_font_size,
        line_height: line_height.unwrap_or(DEFAULT_LINE_HEIGHT),
    }
}

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
    let ls_em = letter_spacing.unwrap_or(0.0);

    ResolvedTextStyle {
        font_family: font_family.unwrap_or_default(),
        font_weight: font_weight.unwrap_or(DEFAULT_FONT_WEIGHT),
        font_size: final_font_size,
        letter_spacing: ls_em * final_font_size,
        line_height: line_height.unwrap_or(DEFAULT_LINE_HEIGHT),
    }
}

pub fn apply_pending_to_style(style: &mut ResolvedTextStyle, pending: &PendingModifiers) {
    for p in pending {
        if let PendingModifier::Set { modifier } = p {
            match modifier {
                Modifier::FontFamily { value } => style.font_family = value.clone(),
                Modifier::FontWeight { value } => style.font_weight = *value,
                Modifier::FontSize { value } => {
                    // centipoints → pixels (resolve_text_style과 동일 변환).
                    style.font_size = (*value as f32 / 100.0) * PT_TO_PX;
                }
                _ => {}
            }
        }
    }
}

pub fn resolve_paragraph_indent(node: &NodeRef<'_>) -> f32 {
    let parent_is_root = node
        .parent()
        .map(|p| matches!(p.node(), Node::Root(_)))
        .unwrap_or(false);
    if !parent_is_root {
        return 0.0;
    }
    match resolve_inherited(node, ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent { value }) => *value as f32 / 100.0 * DEFAULT_FONT_SIZE_PX,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
    use editor_model::PlainStyleEntry;
    use editor_transaction::Transaction;

    use super::*;

    #[test]
    fn resolve_text_style_from_self() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [font_size(2400), font_weight(700)]
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let style = resolve_text_style(&node);

        assert_eq!(style.font_weight, 700);
        assert!((style.font_size - 32.0).abs() < 0.01);
    }

    #[test]
    fn resolve_text_style_inherits_from_ancestor() {
        let (doc, t1) = doc! {
            root [font_size(1600), line_height(200)] {
                paragraph {
                    t1: text("hello")
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let style = resolve_text_style(&node);

        assert!((style.font_size - 21.333).abs() < 0.01);
        assert!((style.line_height - 2.0).abs() < 0.01);
    }

    #[test]
    fn resolve_text_style_picks_up_run_style_modifiers() {
        let (initial, _p1, t1, ..) = state! {
            doc { root { p1: paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![
                    Modifier::FontSize { value: 1800 },
                    Modifier::FontWeight { value: 700 },
                ]
                .into_iter()
                .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (next, ..) = tr.commit();

        let text = next.doc.node(t1).unwrap();
        let style = resolve_text_style(&text);

        // 18pt * (96/72) = 24px
        assert!((style.font_size - 24.0).abs() < 0.01);
        assert_eq!(style.font_weight, 700);
    }

    #[test]
    fn resolve_text_style_own_overrides_style_modifier() {
        let (initial, _p1, t1, ..) = state! {
            doc { root { p1: paragraph { t1: text("hello") [font_size(1200)] } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![Modifier::FontSize { value: 1800 }]
                    .into_iter()
                    .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (next, ..) = tr.commit();

        let text = next.doc.node(t1).unwrap();
        let style = resolve_text_style(&text);

        // 12pt * (96/72) = 16px (run's own modifier wins over its style)
        assert!((style.font_size - 16.0).abs() < 0.01);
    }

    #[test]
    fn resolve_text_style_defaults_when_absent() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello")
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let style = resolve_text_style(&node);

        assert_eq!(style.font_weight, 400);
        assert!((style.font_size - 16.0).abs() < 0.01);
        assert!((style.line_height - 1.6).abs() < 0.01);
        assert!((style.letter_spacing - 0.0).abs() < 0.01);
    }

    #[test]
    fn style_from_effective_modifiers_empty_uses_defaults() {
        let style = style_from_effective_modifiers(&[]);
        assert_eq!(style.font_weight, DEFAULT_FONT_WEIGHT);
        assert!((style.font_size - DEFAULT_FONT_SIZE_PX).abs() < 0.01);
        assert!((style.line_height - DEFAULT_LINE_HEIGHT).abs() < 0.01);
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
    fn paragraph_indent_applies_only_when_parent_is_root() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                p1: paragraph { text("hello") }
            }
        };

        let node = doc.node(p1).unwrap();
        let indent = resolve_paragraph_indent(&node);

        // 200 / 100 * 16.0 = 32.0
        assert!((indent - 32.0).abs() < 0.01);
    }

    #[test]
    fn paragraph_indent_zero_inside_blockquote() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                blockquote {
                    p1: paragraph { text("hello") }
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let indent = resolve_paragraph_indent(&node);

        assert!(indent.abs() < 0.01);
    }

    #[test]
    fn paragraph_indent_zero_inside_list_item() {
        let (doc, p1) = doc! {
            root [paragraph_indent(200)] {
                bullet_list {
                    list_item {
                        p1: paragraph { text("hello") }
                    }
                }
            }
        };

        let node = doc.node(p1).unwrap();
        let indent = resolve_paragraph_indent(&node);

        assert!(indent.abs() < 0.01);
    }
}

use editor_model::{Modifier, ModifierType, NodeRef};

use crate::measure::resolve::resolve_inherited;

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
        for m in ancestor.modifiers() {
            match m {
                Modifier::FontFamily(f) if font_family.is_none() => {
                    font_family = Some(f.clone());
                    resolved_count += 1;
                }
                Modifier::FontWeight(w) if font_weight.is_none() => {
                    font_weight = Some(*w);
                    resolved_count += 1;
                }
                Modifier::FontSize(s) if font_size.is_none() => {
                    let pt = *s as f32 / 100.0;
                    font_size = Some(pt * PT_TO_PX);
                    resolved_count += 1;
                }
                Modifier::LetterSpacing(ls) if letter_spacing.is_none() => {
                    letter_spacing = Some(*ls as f32 / 100.0);
                    resolved_count += 1;
                }
                Modifier::LineHeight(lh) if line_height.is_none() => {
                    line_height = Some(*lh as f32 / 100.0);
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

pub fn resolve_paragraph_indent(node: &NodeRef<'_>) -> f32 {
    match resolve_inherited(node, ModifierType::ParagraphIndent) {
        Some(Modifier::ParagraphIndent(v)) => *v as f32 / 100.0 * DEFAULT_FONT_SIZE_PX,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

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
}

pub mod color;
pub mod font_family;
pub mod font_size;
pub mod font_weight;
pub mod letter_spacing;

use editor_model::Modifier;
use editor_resource::Resource;

pub fn normalize_modifier(m: Modifier, resource: &Resource) -> Option<Modifier> {
    match m {
        Modifier::TextColor { value } => color::normalize_text_color(&value, resource),
        Modifier::BackgroundColor { value } => color::normalize_background_color(&value, resource),
        Modifier::FontFamily { value } => font_family::normalize(&value, resource),
        Modifier::FontWeight { value } => Some(font_weight::normalize(value)),
        Modifier::FontSize { value } => Some(font_size::normalize(value)),
        Modifier::LetterSpacing { value } => Some(letter_spacing::normalize(value)),
        other => Some(other),
    }
}

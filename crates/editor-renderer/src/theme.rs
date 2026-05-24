use crate::theme_data::ThemeVariant;
use crate::types::Color;

#[derive(Debug)]
pub struct Theme {
    variant: ThemeVariant,
    colors: &'static phf::Map<&'static str, Color>,
}

impl Theme {
    pub fn new(variant: ThemeVariant) -> Self {
        Self {
            variant,
            colors: variant.colors(),
        }
    }

    pub fn variant(&self) -> ThemeVariant {
        self.variant
    }

    pub fn set_variant(&mut self, variant: ThemeVariant) -> bool {
        if self.variant == variant {
            return false;
        }

        self.variant = variant;
        self.colors = variant.colors();
        true
    }

    pub fn color(&self, token: &str) -> Color {
        self.colors.get(token).copied().unwrap_or(Color::BLACK)
    }

    pub fn color_with_alpha(&self, token: &str, alpha: u8) -> Color {
        self.color(token).with_alpha(alpha)
    }
}

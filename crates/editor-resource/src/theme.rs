use crate::theme_data::ThemeVariant;
use editor_common::Color;

#[derive(Debug, Clone, Copy)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_with_given_variant() {
        let theme = Theme::new(ThemeVariant::LightWhite);
        assert_eq!(theme.variant(), ThemeVariant::LightWhite);
    }

    #[test]
    fn set_variant_returns_false_for_same() {
        let mut theme = Theme::new(ThemeVariant::LightWhite);
        assert!(!theme.set_variant(ThemeVariant::LightWhite));
        assert_eq!(theme.variant(), ThemeVariant::LightWhite);
    }

    #[test]
    fn set_variant_returns_true_for_different_and_updates() {
        let mut theme = Theme::new(ThemeVariant::LightWhite);
        assert!(theme.set_variant(ThemeVariant::DarkBlack));
        assert_eq!(theme.variant(), ThemeVariant::DarkBlack);
    }

    #[test]
    fn color_returns_variant_specific_rgb() {
        let light = Theme::new(ThemeVariant::LightWhite);
        let dark = Theme::new(ThemeVariant::DarkBlack);
        assert_ne!(
            light.color("ui.text.default"),
            dark.color("ui.text.default")
        );
    }

    #[test]
    fn color_unknown_token_falls_back_to_black() {
        let theme = Theme::new(ThemeVariant::LightWhite);
        assert_eq!(theme.color("__nonexistent_token__"), Color::BLACK);
    }

    #[test]
    fn color_with_alpha_overrides_alpha_channel() {
        let theme = Theme::new(ThemeVariant::LightWhite);
        let base = theme.color("ui.text.default");
        let with_alpha = theme.color_with_alpha("ui.text.default", 128);
        assert_eq!(with_alpha.a, 128);
        assert_eq!(with_alpha.r, base.r);
        assert_eq!(with_alpha.g, base.g);
        assert_eq!(with_alpha.b, base.b);
    }
}

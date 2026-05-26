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

    pub fn text_paste_palette(&self) -> impl Iterator<Item = (&'static str, Color)> + '_ {
        const DENY: &[&str] = &["bright"];
        self.colors.entries().filter_map(|(token, color)| {
            token
                .strip_prefix("text.")
                .filter(|suffix| !DENY.contains(suffix))
                .map(|suffix| (suffix, *color))
        })
    }

    pub fn bg_paste_palette(&self) -> impl Iterator<Item = (&'static str, Color)> + '_ {
        self.colors.entries().filter_map(|(token, color)| {
            token.strip_prefix("bg.").map(|suffix| (suffix, *color))
        })
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

    #[test]
    fn text_paste_palette_excludes_bright_and_ui_prefix() {
        let theme = Theme::new(ThemeVariant::LightWhite);
        let keys: Vec<&str> = theme.text_paste_palette().map(|(k, _)| k).collect();
        assert!(keys.contains(&"red"), "shared palette key 'red' must be present");
        assert!(keys.contains(&"black"), "variant palette key 'black' must be present");
        assert!(keys.contains(&"white"), "variant palette key 'white' must be present");
        assert!(!keys.contains(&"bright"), "'bright' must be excluded from paste palette");
        assert!(!keys.iter().any(|k| k.starts_with("ui.")), "ui.* tokens must not appear");
    }

    #[test]
    fn bg_paste_palette_lists_seven_colors() {
        let theme = Theme::new(ThemeVariant::LightWhite);
        let keys: Vec<&str> = theme.bg_paste_palette().map(|(k, _)| k).collect();
        for expected in ["gray", "red", "orange", "yellow", "green", "blue", "purple"] {
            assert!(keys.contains(&expected), "bg palette must include '{expected}'");
        }
        assert!(!keys.contains(&"none"));
    }

    #[test]
    fn text_paste_palette_returns_dark_variant_colors() {
        let theme = Theme::new(ThemeVariant::DarkBlack);
        let palette: Vec<(&str, Color)> = theme.text_paste_palette().collect();
        let white = palette.iter().find(|(k, _)| *k == "white").map(|(_, c)| *c);
        assert!(white.is_some(), "'white' key must be in dark variant palette");
        let w = white.unwrap();
        assert!(w.r < 0x40 && w.g < 0x40 && w.b < 0x40,
            "dark variant 'white' palette entry should be dark, got {:?}", w);
    }
}

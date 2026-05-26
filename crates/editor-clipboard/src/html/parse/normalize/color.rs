use editor_common::Color;
use editor_model::Modifier;
use editor_resource::Resource;

fn weighted_rgb_distance_sq(a: Color, b: Color) -> u32 {
    let dr = (a.r as i32 - b.r as i32).unsigned_abs();
    let dg = (a.g as i32 - b.g as i32).unsigned_abs();
    let db = (a.b as i32 - b.b as i32).unsigned_abs();
    2 * dr * dr + 4 * dg * dg + 3 * db * db
}

fn parse_css_color(value: &str) -> Option<Color> {
    let parsed = csscolorparser::parse(value.trim()).ok()?;
    let [r, g, b, a] = parsed.to_rgba8();
    Some(Color { r, g, b, a })
}

pub fn normalize_text_color(value: &str, resource: &Resource) -> Option<Modifier> {
    let input = parse_css_color(value)?;
    let (best_key, _) = resource
        .theme
        .text_paste_palette()
        .min_by_key(|(_, c)| weighted_rgb_distance_sq(input, *c))?;
    Some(Modifier::TextColor {
        value: best_key.to_string(),
    })
}

pub fn normalize_background_color(value: &str, resource: &Resource) -> Option<Modifier> {
    let input = parse_css_color(value)?;
    if input.a == 0 {
        return Some(Modifier::BackgroundColor {
            value: "none".to_string(),
        });
    }
    let (best_key, _) = resource
        .theme
        .bg_paste_palette()
        .min_by_key(|(_, c)| weighted_rgb_distance_sq(input, *c))?;
    Some(Modifier::BackgroundColor {
        value: best_key.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_resource::{Resource, ThemeVariant};

    fn text_color_key(input: &str, r: &Resource) -> Option<String> {
        match normalize_text_color(input, r)? {
            Modifier::TextColor { value } => Some(value),
            _ => None,
        }
    }

    fn bg_color_key(input: &str, r: &Resource) -> Option<String> {
        match normalize_background_color(input, r)? {
            Modifier::BackgroundColor { value } => Some(value),
            _ => None,
        }
    }

    #[test]
    fn pure_red_rgb_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("rgb(255, 0, 0)", &r).as_deref(), Some("red"));
    }

    #[test]
    fn named_red_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("red", &r).as_deref(), Some("red"));
    }

    #[test]
    fn hex_red_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("#ff0000", &r).as_deref(), Some("red"));
    }

    #[test]
    fn hsl_red_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(
            text_color_key("hsl(0, 100%, 50%)", &r).as_deref(),
            Some("red")
        );
    }

    #[test]
    fn rgba_alpha_ignored() {
        let r = Resource::new_test();
        assert_eq!(
            text_color_key("rgba(255, 0, 0, 0.5)", &r).as_deref(),
            Some("red")
        );
    }

    #[test]
    fn tailwind_red_400_snaps_to_rose() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("#f87171", &r).as_deref(), Some("rose"));
    }

    #[test]
    fn pure_black_snaps_to_black_in_light_theme() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("#000000", &r).as_deref(), Some("black"));
    }

    #[test]
    fn pure_white_in_dark_theme_does_not_snap_to_bright() {
        let mut r = Resource::new_test();
        r.theme.set_variant(ThemeVariant::DarkBlack);
        let key = text_color_key("#ffffff", &r);
        assert_eq!(key.as_deref(), Some("black"));
        assert_ne!(key.as_deref(), Some("bright"));
    }

    #[test]
    fn invalid_color_returns_none() {
        let r = Resource::new_test();
        assert!(normalize_text_color("not-a-color", &r).is_none());
        assert!(normalize_text_color("currentColor", &r).is_none());
        assert!(normalize_text_color("inherit", &r).is_none());
    }

    #[test]
    fn empty_string_returns_none() {
        let r = Resource::new_test();
        assert!(normalize_text_color("", &r).is_none());
    }

    #[test]
    fn bg_pure_red_snaps_to_yellow() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("rgb(255, 0, 0)", &r).as_deref(), Some("yellow"));
    }

    #[test]
    fn bg_yellow_named_snaps_to_yellow() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("yellow", &r).as_deref(), Some("yellow"));
    }

    #[test]
    fn bg_transparent_keyword_returns_none_value() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("transparent", &r).as_deref(), Some("none"));
    }

    #[test]
    fn bg_rgba_zero_alpha_returns_none_value() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("rgba(255, 0, 0, 0)", &r).as_deref(), Some("none"));
    }

    #[test]
    fn bg_invalid_returns_none() {
        let r = Resource::new_test();
        assert!(normalize_background_color("not-a-color", &r).is_none());
    }
}

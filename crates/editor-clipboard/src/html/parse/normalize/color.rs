use editor_common::Color;
use editor_model::Modifier;
use editor_resource::Resource;

const NEUTRAL_TEXT_KEYS: &[&str] = &["black", "darkgray", "gray", "lightgray", "white", "bright"];
const TEXT_ACHROMATIC_GATE: f32 = 0.04;
const BG_ACHROMATIC_GATE: f32 = 0.008;
const BG_PAGE_WHITE_LIGHTNESS: f32 = 0.97;
const LIGHTNESS_WEIGHT: f32 = 0.4;

struct Oklab {
    l: f32,
    a: f32,
    b: f32,
}

fn srgb_channel_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn oklab(color: Color) -> Oklab {
    let r = srgb_channel_to_linear(color.r);
    let g = srgb_channel_to_linear(color.g);
    let b = srgb_channel_to_linear(color.b);
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    Oklab {
        l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    }
}

fn chroma(lab: &Oklab) -> f32 {
    lab.a.hypot(lab.b)
}

fn hue_distance(a: &Oklab, b: &Oklab) -> f32 {
    let d = (a.b.atan2(a.a) - b.b.atan2(b.a)).abs();
    d.min(2.0 * std::f32::consts::PI - d)
}

fn chromatic_distance_sq(a: &Oklab, b: &Oklab) -> f32 {
    let dl = LIGHTNESS_WEIGHT * (a.l - b.l);
    dl * dl + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)
}

fn parse_css_color(value: &str) -> Option<Color> {
    let parsed = csscolorparser::parse(value.trim()).ok()?;
    let [r, g, b, a] = parsed.to_rgba8();
    Some(Color { r, g, b, a })
}

pub fn normalize_text_color(value: &str, resource: &Resource) -> Option<Modifier> {
    let input = parse_css_color(value)?;
    let lab = oklab(input);
    let achromatic = chroma(&lab) < TEXT_ACHROMATIC_GATE;
    let (best_key, _) = resource
        .theme
        .text_paste_palette()
        .filter(|(key, _)| NEUTRAL_TEXT_KEYS.contains(key) == achromatic)
        .map(|(key, c)| {
            let entry = oklab(c);
            let d = if achromatic {
                (lab.l - entry.l).abs()
            } else {
                chromatic_distance_sq(&lab, &entry)
            };
            (key, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
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
    let lab = oklab(input);
    let achromatic = chroma(&lab) < BG_ACHROMATIC_GATE;
    if achromatic && lab.l >= BG_PAGE_WHITE_LIGHTNESS {
        return Some(Modifier::BackgroundColor {
            value: "none".to_string(),
        });
    }
    let (best_key, _) = resource
        .theme
        .bg_paste_palette()
        .filter(|(key, _)| (*key == "gray") == achromatic)
        .map(|(key, c)| {
            let entry = oklab(c);
            let d = if achromatic {
                (lab.l - entry.l).abs()
            } else {
                hue_distance(&lab, &entry)
            };
            (key, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
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
    fn tailwind_red_400_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("#f87171", &r).as_deref(), Some("red"));
    }

    #[test]
    fn css_green_snaps_to_green() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("green", &r).as_deref(), Some("green"));
    }

    #[test]
    fn css_navy_snaps_to_blue_family() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("navy", &r).as_deref(), Some("indigo"));
    }

    #[test]
    fn css_maroon_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("maroon", &r).as_deref(), Some("red"));
    }

    #[test]
    fn css_silver_snaps_to_lightgray() {
        let r = Resource::new_test();
        assert_eq!(text_color_key("silver", &r).as_deref(), Some("lightgray"));
    }

    #[test]
    fn near_neutral_dark_snaps_to_darkgray() {
        let r = Resource::new_test();
        assert_eq!(
            text_color_key("darkslategray", &r).as_deref(),
            Some("darkgray")
        );
    }

    #[test]
    fn chromatic_input_never_snaps_to_neutral() {
        let r = Resource::new_test();
        const NEUTRALS: &[&str] = &["black", "darkgray", "gray", "lightgray", "white"];
        for input in ["#008000", "#000080", "#800000", "#800080", "#8b4513"] {
            let key = text_color_key(input, &r).unwrap();
            assert!(
                !NEUTRALS.contains(&key.as_str()),
                "{input} snapped to neutral {key}"
            );
        }
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
    fn bg_pure_red_snaps_to_red() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("rgb(255, 0, 0)", &r).as_deref(), Some("red"));
    }

    #[test]
    fn bg_pure_green_snaps_to_green() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#00ff00", &r).as_deref(), Some("green"));
    }

    #[test]
    fn bg_near_neutral_snaps_to_gray() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#dddddd", &r).as_deref(), Some("gray"));
    }

    #[test]
    fn bg_pure_white_snaps_to_none() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#ffffff", &r).as_deref(), Some("none"));
        assert_eq!(bg_color_key("white", &r).as_deref(), Some("none"));
    }

    #[test]
    fn bg_near_white_snaps_to_none() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#fafafa", &r).as_deref(), Some("none"));
        assert_eq!(bg_color_key("#f8f9fa", &r).as_deref(), Some("none"));
    }

    #[test]
    fn bg_own_gray_stays_gray_below_page_white() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#f1f1f2", &r).as_deref(), Some("gray"));
    }

    #[test]
    fn bg_pure_white_snaps_to_none_in_dark_theme() {
        let mut r = Resource::new_test();
        r.theme.set_variant(ThemeVariant::DarkBlack);
        assert_eq!(bg_color_key("#ffffff", &r).as_deref(), Some("none"));
    }

    #[test]
    fn bg_lavender_snaps_to_purple() {
        let r = Resource::new_test();
        assert_eq!(bg_color_key("#e6e6fa", &r).as_deref(), Some("purple"));
    }

    #[test]
    fn bg_own_pale_entries_round_trip() {
        let r = Resource::new_test();
        for (key, hex) in [
            ("red", "#fdebec"),
            ("orange", "#ffecd5"),
            ("yellow", "#fef3c7"),
            ("green", "#dff3e3"),
            ("blue", "#e7f3f8"),
            ("purple", "#f0e7fe"),
            ("gray", "#f1f1f2"),
        ] {
            assert_eq!(bg_color_key(hex, &r).as_deref(), Some(key), "input {hex}");
        }
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
        assert_eq!(
            bg_color_key("rgba(255, 0, 0, 0)", &r).as_deref(),
            Some("none")
        );
    }

    #[test]
    fn bg_invalid_returns_none() {
        let r = Resource::new_test();
        assert!(normalize_background_color("not-a-color", &r).is_none());
    }
}

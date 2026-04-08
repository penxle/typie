use crate::utils::rgba_from_u32;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use tiny_skia::Color;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub colors: FxHashMap<String, u32>,
}

impl Hash for Theme {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut keys: Vec<_> = self.colors.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
            self.colors[key].hash(state);
        }
    }
}

pub const TEXT_COLORS: &[(&str, u32)] = &[
    ("black", 0x18_18_1b_ff),
    ("darkgray", 0x52_52_54_ff),
    ("gray", 0x8c_8c_8d_ff),
    ("lightgray", 0xc5_c5_c6_ff),
    ("white", 0xff_ff_ff_ff),
    ("red", 0xef_44_44_ff),
    ("orange", 0xf9_73_16_ff),
    ("amber", 0xf5_9e_0b_ff),
    ("yellow", 0xea_b3_08_ff),
    ("lime", 0x84_cc_16_ff),
    ("green", 0x22_c5_5e_ff),
    ("emerald", 0x10_b9_81_ff),
    ("teal", 0x14_b8_a6_ff),
    ("cyan", 0x06_b6_d4_ff),
    ("sky", 0x0e_a5_e9_ff),
    ("blue", 0x3b_82_f6_ff),
    ("indigo", 0x63_66_f1_ff),
    ("violet", 0x8b_5c_f6_ff),
    ("purple", 0xa8_55_f7_ff),
    ("fuchsia", 0xd9_46_ef_ff),
    ("pink", 0xec_48_99_ff),
    ("rose", 0xf4_3f_5e_ff),
];

pub const BG_COLORS: &[(&str, u32)] = &[
    ("gray", 0xf1_f1_f2_ff),
    ("red", 0xfd_eb_ec_ff),
    ("orange", 0xff_ec_d5_ff),
    ("yellow", 0xfe_f3_c7_ff),
    ("green", 0xdf_f3_e3_ff),
    ("blue", 0xe7_f3_f8_ff),
    ("purple", 0xf0_e7_fe_ff),
];

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: FxHashMap::default(),
        }
    }
}

impl Theme {
    pub fn is_valid_text_color_key(key: &str) -> bool {
        TEXT_COLORS.iter().any(|&(k, _)| k == key)
    }

    pub fn is_valid_bg_color_key(key: &str) -> bool {
        BG_COLORS.iter().any(|&(k, _)| k == key)
    }

    pub fn text_color_rgba(key: &str) -> Option<u32> {
        TEXT_COLORS
            .iter()
            .find(|&&(k, _)| k == key)
            .map(|&(_, v)| v)
    }

    pub fn bg_color_rgba(key: &str) -> Option<u32> {
        BG_COLORS.iter().find(|&&(k, _)| k == key).map(|&(_, v)| v)
    }

    pub fn nearest_text_color(css: &str) -> Option<&'static str> {
        let c = csscolorparser::parse(css).ok()?;
        let [r, g, b, _] = c.to_rgba8();
        Some(nearest_key(r, g, b, TEXT_COLORS))
    }

    pub fn nearest_bg_color(css: &str) -> Option<&'static str> {
        let c = csscolorparser::parse(css).ok()?;
        let [r, g, b, _] = c.to_rgba8();
        let (ri, gi, bi) = (r as i32, g as i32, b as i32);
        let d_white = (255 - ri) * (255 - ri) + (255 - gi) * (255 - gi) + (255 - bi) * (255 - bi);
        let key = nearest_key(r, g, b, BG_COLORS);
        let rgba = BG_COLORS.iter().find(|&&(k, _)| k == key).unwrap().1;
        let [tr, tg, tb, _] = rgba_from_u32(rgba);
        let (tr, tg, tb) = (tr as i32, tg as i32, tb as i32);
        let d_palette = (ri - tr) * (ri - tr) + (gi - tg) * (gi - tg) + (bi - tb) * (bi - tb);
        if d_palette < d_white { Some(key) } else { None }
    }

    pub fn color_u32(&self, key: &str) -> u32 {
        self.colors.get(key).copied().unwrap_or_else(|| {
            eprintln!("Warning: missing color token '{}', using default", key);
            0x00_00_00_ff
        })
    }

    pub fn color(&self, key: &str) -> Color {
        let color = self.color_u32(key);
        let [r, g, b, a] = rgba_from_u32(color);
        Color::from_rgba8(r, g, b, a)
    }

    pub fn color_rgba(&self, key: &str) -> [u8; 4] {
        let color = self.color_u32(key);
        rgba_from_u32(color)
    }

    pub fn color_with_alpha(&self, key: &str, alpha: u8) -> Color {
        let [r, g, b, _] = self.color_rgba(key);
        Color::from_rgba8(r, g, b, alpha)
    }
}

fn nearest_key<'a>(r: u8, g: u8, b: u8, palette: &'a [(&'a str, u32)]) -> &'a str {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let mut best = palette[0].0;
    let mut best_dist = i32::MAX;
    for &(key, rgba) in palette {
        let [tr, tg, tb, _] = rgba_from_u32(rgba);
        let (tr, tg, tb) = (tr as i32, tg as i32, tb as i32);
        let d = (r - tr) * (r - tr) + (g - tg) * (g - tg) + (b - tb) * (b - tb);
        if d < best_dist {
            best_dist = d;
            best = key;
        }
    }
    best
}

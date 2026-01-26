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

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: FxHashMap::default(),
        }
    }
}

impl Theme {
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

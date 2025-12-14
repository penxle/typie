use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tiny_skia::Color;
use tsify::Tsify;

use crate::utils::rgba_from_u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub background: u32,
    pub text: u32,
    pub colors: HashMap<String, u32>,
}

impl Hash for Theme {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.background.hash(state);
        self.text.hash(state);
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
            background: 0xFFFFFFFF,
            text: 0x000000FF,
            colors: HashMap::new(),
        }
    }
}

impl Theme {
    pub fn text_color(&self, key: Option<&String>) -> Color {
        let color = if let Some(key) = key {
            self.colors.get(key).copied().unwrap_or(self.text)
        } else {
            self.text
        };

        let [r, g, b, a] = rgba_from_u32(color);
        Color::from_rgba8(r, g, b, a)
    }

    pub fn highlight_color(&self, key: &str) -> Option<Color> {
        self.colors.get(key).map(|&color| {
            let [r, g, b, a] = rgba_from_u32(color);
            Color::from_rgba8(r, g, b, a)
        })
    }
}

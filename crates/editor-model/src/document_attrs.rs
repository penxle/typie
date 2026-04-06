use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LayoutMode {
    #[serde(rename_all = "snake_case")]
    Paginated {
        page_width: f32,
        page_height: f32,
        page_margin_top: f32,
        page_margin_bottom: f32,
        page_margin_left: f32,
        page_margin_right: f32,
    },
    #[serde(rename_all = "snake_case")]
    Continuous { max_width: f32 },
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Continuous { max_width: 600.0 }
    }
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DocumentAttrs {
    pub layout_mode: LayoutMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_attrs() {
        let a = DocumentAttrs::default();
        assert!(matches!(a.layout_mode, LayoutMode::Continuous { .. }));
    }

    #[test]
    fn serde_roundtrip() {
        let a = DocumentAttrs::default();
        let json = serde_json::to_string(&a).unwrap();
        let parsed: DocumentAttrs = serde_json::from_str(&json).unwrap();
        assert_eq!(a, parsed);
    }
}

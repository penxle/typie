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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RootNode {
    pub layout_mode: LayoutMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_mode_is_continuous() {
        let m = LayoutMode::default();
        assert!(matches!(m, LayoutMode::Continuous { .. }));
    }

    #[test]
    fn root_node_default_has_continuous_layout() {
        let r = RootNode::default();
        assert!(matches!(r.layout_mode, LayoutMode::Continuous { .. }));
    }

    #[test]
    fn layout_mode_serde_roundtrip() {
        let m = LayoutMode::default();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: LayoutMode = serde_json::from_str(&json).unwrap();
        assert_eq!(m, parsed);
    }

    #[test]
    fn root_node_serde_roundtrip() {
        let r = RootNode::default();
        let json = serde_json::to_string(&r).unwrap();
        let parsed: RootNode = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }
}

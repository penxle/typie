use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutMode {
    #[serde(rename_all = "snake_case")]
    Paginated {
        page_width: u32,
        page_height: u32,
        page_margin_top: u32,
        page_margin_bottom: u32,
        page_margin_left: u32,
        page_margin_right: u32,
    },
    #[serde(rename_all = "snake_case")]
    Continuous { max_width: u32 },
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Continuous { max_width: 600 }
    }
}

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct RootNode {
    pub layout_mode: LwwReg<LayoutMode>,
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
        assert!(matches!(
            *r.layout_mode.get(),
            LayoutMode::Continuous { .. }
        ));
    }

    #[test]
    fn layout_mode_serde_roundtrip() {
        let m = LayoutMode::default();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: LayoutMode = serde_json::from_str(&json).unwrap();
        assert_eq!(m, parsed);
    }
}

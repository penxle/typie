use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Affinity {
    Upstream,
    Downstream,
}

impl Default for Affinity {
    fn default() -> Self {
        Affinity::Downstream
    }
}

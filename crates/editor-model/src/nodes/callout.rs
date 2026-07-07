use editor_crdt::LwwReg;
use editor_macros::{NodeAttr, ffi};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, NodeAttr)]
pub struct CalloutNode {
    #[plain(serde(default))]
    pub variant: LwwReg<CalloutVariant>,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalloutVariant {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl CalloutVariant {
    pub fn next(self) -> Self {
        match self {
            Self::Info => Self::Success,
            Self::Success => Self::Warning,
            Self::Warning => Self::Danger,
            Self::Danger => Self::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_variant_next_cycles() {
        assert_eq!(CalloutVariant::Info.next(), CalloutVariant::Success);
        assert_eq!(CalloutVariant::Success.next(), CalloutVariant::Warning);
        assert_eq!(CalloutVariant::Warning.next(), CalloutVariant::Danger);
        assert_eq!(CalloutVariant::Danger.next(), CalloutVariant::Info);
    }
}

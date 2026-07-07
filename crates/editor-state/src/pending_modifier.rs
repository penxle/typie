use editor_macros::ffi;
use editor_model::{Modifier, ModifierType};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PendingModifier {
    Set { modifier: Modifier },
    Unset { ty: ModifierType },
}

impl PendingModifier {
    pub fn as_type(&self) -> ModifierType {
        match self {
            Self::Set { modifier } => modifier.as_type(),
            Self::Unset { ty } => *ty,
        }
    }
}

#[ffi]
pub type PendingModifiers = Vec<PendingModifier>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_modifier_serde_roundtrip() {
        let pm = PendingModifier::Set {
            modifier: Modifier::Bold,
        };
        let json = serde_json::to_string(&pm).unwrap();
        let back: PendingModifier = serde_json::from_str(&json).unwrap();
        assert_eq!(pm, back);
    }

    #[test]
    fn pending_modifiers_serde_roundtrip() {
        let pms: PendingModifiers = vec![
            PendingModifier::Set {
                modifier: Modifier::Bold,
            },
            PendingModifier::Unset {
                ty: ModifierType::Italic,
            },
        ];
        let json = serde_json::to_string(&pms).unwrap();
        let back: PendingModifiers = serde_json::from_str(&json).unwrap();
        assert_eq!(pms, back);
    }
}

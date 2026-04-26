use editor_model::{Modifier, ModifierType};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingModifier {
    Set(Modifier),
    Unset(ModifierType),
}

impl PendingModifier {
    pub fn as_type(&self) -> ModifierType {
        match self {
            Self::Set(m) => m.as_type(),
            Self::Unset(t) => *t,
        }
    }
}

pub type PendingModifiers = SmallVec<[PendingModifier; 2]>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_modifier_serde_roundtrip() {
        let pm = PendingModifier::Set(Modifier::Bold);
        let json = serde_json::to_string(&pm).unwrap();
        let back: PendingModifier = serde_json::from_str(&json).unwrap();
        assert_eq!(pm, back);
    }

    #[test]
    fn pending_modifiers_serde_roundtrip() {
        let mut pms = PendingModifiers::new();
        pms.push(PendingModifier::Set(Modifier::Bold));
        pms.push(PendingModifier::Unset(ModifierType::Italic));
        let json = serde_json::to_string(&pms).unwrap();
        let back: PendingModifiers = serde_json::from_str(&json).unwrap();
        assert_eq!(pms, back);
    }
}

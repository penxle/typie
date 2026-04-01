use editor_model::{Modifier, ModifierType};
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq, Eq)]
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

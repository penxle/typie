use editor_macros::ffi;
use editor_model::{Modifier, ModifierType, NodeType, Schema};
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

    /// Builds a pending modifier diff from `base` to `target`.
    pub fn diff(base: &[Modifier], target: &[Modifier]) -> PendingModifiers {
        let mut pending = PendingModifiers::new();

        let base = base
            .iter()
            .filter(|modifier| is_valid_pending_modifier_type(modifier.as_type()))
            .collect::<Vec<_>>();
        let target = target
            .iter()
            .filter(|modifier| is_valid_pending_modifier_type(modifier.as_type()))
            .collect::<Vec<_>>();

        for modifier in &base {
            let ty = modifier.as_type();
            if !target.iter().any(|target| target.as_type() == ty) {
                pending.push(Self::Unset { ty });
            }
        }

        for modifier in &target {
            if !base
                .iter()
                .any(|base| base.as_type() == modifier.as_type() && base == modifier)
            {
                pending.push(Self::Set {
                    modifier: (*modifier).clone(),
                });
            }
        }

        pending
    }
}

fn is_valid_pending_modifier_type(ty: ModifierType) -> bool {
    Schema::modifier_spec(ty)
        .target
        .rightmost_node_types()
        .contains(&NodeType::Text)
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
        let mut pms = PendingModifiers::new();
        pms.push(PendingModifier::Set {
            modifier: Modifier::Bold,
        });
        pms.push(PendingModifier::Unset {
            ty: ModifierType::Italic,
        });
        let json = serde_json::to_string(&pms).unwrap();
        let back: PendingModifiers = serde_json::from_str(&json).unwrap();
        assert_eq!(pms, back);
    }

    #[test]
    fn diff_sets_missing_modifiers() {
        let pending = PendingModifier::diff(&[], &[Modifier::Bold]);
        assert_eq!(
            pending,
            vec![PendingModifier::Set {
                modifier: Modifier::Bold
            }]
        );
    }

    #[test]
    fn diff_unsets_extra_modifiers() {
        let pending = PendingModifier::diff(&[Modifier::Bold, Modifier::Italic], &[Modifier::Bold]);
        assert_eq!(
            pending,
            vec![PendingModifier::Unset {
                ty: ModifierType::Italic
            }]
        );
    }

    #[test]
    fn diff_replaces_same_type_with_different_value() {
        let pending = PendingModifier::diff(
            &[Modifier::FontSize { value: 1200 }],
            &[Modifier::FontSize { value: 1400 }],
        );
        assert_eq!(
            pending,
            vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 1400 }
            }]
        );
    }

    #[test]
    fn diff_ignores_non_text_modifiers() {
        let pending = PendingModifier::diff(
            &[Modifier::BlockGap { value: 100 }],
            &[Modifier::ParagraphIndent { value: 100 }],
        );
        assert!(pending.is_empty());
    }
}

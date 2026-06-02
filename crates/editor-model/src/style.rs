use editor_crdt::{LwwReg, OrSet};

use crate::modifier::Modifier;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleEntry {
    pub name: LwwReg<String>,
    pub modifiers: OrSet<Modifier>,
}

impl StyleEntry {
    pub fn new() -> Self {
        Self {
            name: LwwReg::with_value(String::new()),
            modifiers: OrSet::new(),
        }
    }
}

impl Default for StyleEntry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, OrSetOp};

    #[test]
    fn new_is_empty() {
        let s = StyleEntry::new();
        assert_eq!(s.name.get(), "");
        assert_eq!(s.modifiers.iter().count(), 0);
    }

    #[test]
    fn modifier_can_be_added() {
        let s = StyleEntry::new();
        let modifiers = s
            .modifiers
            .apply(
                Dot::new(1, 0),
                OrSetOp::Add {
                    elem: Modifier::Bold,
                },
            )
            .unwrap();
        assert!(modifiers.contains(&Modifier::Bold));
    }
}

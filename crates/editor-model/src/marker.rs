use crate::modifier::Modifier;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
pub struct Marker {
    #[wire(n(0))]
    pub modifiers: Vec<Modifier>,
}

impl Marker {
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_is_empty() {
        assert!(Marker { modifiers: vec![] }.is_empty());
        assert!(
            !Marker {
                modifiers: vec![Modifier::Bold]
            }
            .is_empty()
        );
    }
}

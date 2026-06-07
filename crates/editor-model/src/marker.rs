use crate::modifier::Modifier;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
pub struct Marker {
    #[wire(n(0))]
    pub modifiers: Vec<Modifier>,
    #[wire(n(1))]
    pub style: Option<String>,
}

impl Marker {
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.style.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_is_empty() {
        assert!(
            Marker {
                modifiers: vec![],
                style: None
            }
            .is_empty()
        );
        assert!(
            !Marker {
                modifiers: vec![Modifier::Bold],
                style: None
            }
            .is_empty()
        );
    }
}

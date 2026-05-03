use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{CrdtError, Dot, Rga, RgaOp};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextOp {
    InsertChar { after: Option<Dot>, ch: char },
    RemoveChar { target: Dot },
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Text(Rga<char>);

impl Text {
    pub fn new() -> Self {
        Self(Rga::new())
    }

    /// Returns `Err` if the same `Dot` arrives with a different payload.
    /// Delegates to `Rga<char>::apply` after mapping `TextOp` → `RgaOp<char>`.
    pub fn apply(&self, id: Dot, op: TextOp) -> Result<Self, CrdtError> {
        let rga_op = match op {
            TextOp::InsertChar { after, ch } => RgaOp::Insert { after, value: ch },
            TextOp::RemoveChar { target } => RgaOp::Remove { target },
        };
        self.0.apply(id, rga_op).map(Self)
    }

    /// Count of reachable + alive characters — equal to `to_string().chars().count()`.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in self.0.iter() {
            write!(f, "{ch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_yields_empty_state() {
        let t = Text::new();
        assert_eq!(t.len(), 0);
        assert!(t.is_empty());
        assert_eq!(t.to_string(), "");
    }

    #[test]
    fn default_equals_new() {
        assert_eq!(Text::default(), Text::new());
    }

    #[test]
    fn apply_insert_char_yields_value_via_display() {
        let t = Text::new()
            .apply(
                Dot::new(1, 0),
                TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            )
            .unwrap();
        assert_eq!(t.to_string(), "a");
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn apply_remove_char_tombstones_target() {
        let t = Text::new()
            .apply(
                Dot::new(1, 0),
                TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                TextOp::RemoveChar {
                    target: Dot::new(1, 0),
                },
            )
            .unwrap();
        assert_eq!(t.to_string(), "");
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn display_chars_count_matches_len() {
        let t = Text::new()
            .apply(
                Dot::new(1, 0),
                TextOp::InsertChar {
                    after: None,
                    ch: 'h',
                },
            )
            .unwrap()
            .apply(
                Dot::new(1, 1),
                TextOp::InsertChar {
                    after: Some(Dot::new(1, 0)),
                    ch: 'i',
                },
            )
            .unwrap();
        assert_eq!(t.to_string(), "hi");
        assert_eq!(t.len(), t.to_string().chars().count());
    }
}

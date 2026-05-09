use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{CrdtError, Dot, Rga, RgaOp, ToPlain};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextOp {
    #[n(0)]
    InsertChar {
        #[n(0)]
        after: Option<Dot>,
        #[n(1)]
        ch: char,
    },
    #[n(1)]
    RemoveChar {
        #[n(0)]
        observed: Dot,
    },
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
            TextOp::RemoveChar { observed } => RgaOp::Remove { observed },
        };
        self.0.apply(id, rga_op).map(Self)
    }

    pub fn contains_dot(&self, dot: Dot) -> bool {
        self.0.contains_dot(dot)
    }

    pub fn dot_at(&self, char_offset: usize) -> Result<Option<Dot>, CrdtError> {
        self.0.dot_at(char_offset)
    }

    pub fn iter_with_dot(&self) -> impl Iterator<Item = (Dot, char)> + '_ {
        self.0.iter_with_dot().map(|(d, &c)| (d, c))
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

impl ToPlain for Text {
    type Plain = String;
    fn to_plain(&self) -> String {
        self.to_string()
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
    fn apply_remove_char_tombstones_observed() {
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
                    observed: Dot::new(1, 0),
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

    #[test]
    fn dot_at_delegates_to_rga() {
        let d1 = Dot::new(1, 0);
        let d2 = Dot::new(1, 1);
        let t = Text::new()
            .apply(
                d1,
                TextOp::InsertChar {
                    after: None,
                    ch: 'h',
                },
            )
            .unwrap()
            .apply(
                d2,
                TextOp::InsertChar {
                    after: Some(d1),
                    ch: 'i',
                },
            )
            .unwrap();
        assert_eq!(t.dot_at(0), Ok(None));
        assert_eq!(t.dot_at(1), Ok(Some(d1)));
        assert_eq!(t.dot_at(2), Ok(Some(d2)));
        assert!(t.dot_at(3).is_err());
    }

    #[test]
    fn empty_text_plain_is_empty_string() {
        let t = Text::new();
        assert_eq!(t.to_plain(), "");
    }

    #[test]
    fn two_chars_plain() {
        let d1 = Dot::new(1, 0);
        let d2 = Dot::new(1, 1);
        let t = Text::new()
            .apply(
                d1,
                TextOp::InsertChar {
                    after: None,
                    ch: 'h',
                },
            )
            .unwrap()
            .apply(
                d2,
                TextOp::InsertChar {
                    after: Some(d1),
                    ch: 'i',
                },
            )
            .unwrap();
        assert_eq!(t.to_plain(), "hi");
    }

    #[test]
    fn iter_with_dot_yields_pairs() {
        let d1 = Dot::new(1, 0);
        let d2 = Dot::new(1, 1);
        let t = Text::new()
            .apply(
                d1,
                TextOp::InsertChar {
                    after: None,
                    ch: 'h',
                },
            )
            .unwrap()
            .apply(
                d2,
                TextOp::InsertChar {
                    after: Some(d1),
                    ch: 'i',
                },
            )
            .unwrap();
        let pairs: Vec<(Dot, char)> = t.iter_with_dot().collect();
        assert_eq!(pairs, vec![(d1, 'h'), (d2, 'i')]);
    }
}

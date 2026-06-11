use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{CrdtError, Dot, ToPlain};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextOp {
    #[wire(n(0))]
    InsertChar {
        #[wire(n(0))]
        after: Option<PlacementId>,
        #[wire(n(1))]
        ch: char,
    },
    #[wire(n(1))]
    RemoveChar {
        #[wire(n(0))]
        observed: EntryDot,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, editor_macros::Wire)]
#[wire(transparent)]
#[serde(transparent)]
pub struct EntryDot(pub Dot);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, editor_macros::Wire)]
#[wire(transparent)]
#[serde(transparent)]
pub struct PlacementId(pub Dot);

impl From<Dot> for EntryDot {
    fn from(dot: Dot) -> Self {
        Self(dot)
    }
}

impl From<EntryDot> for Dot {
    fn from(entry: EntryDot) -> Self {
        entry.0
    }
}

impl From<Dot> for PlacementId {
    fn from(dot: Dot) -> Self {
        Self(dot)
    }
}

impl From<PlacementId> for Dot {
    fn from(placement: PlacementId) -> Self {
        placement.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPlacement {
    pub placement_id: PlacementId,
    pub entry_dot: EntryDot,
    pub ch: char,
}

/// Materialized visible text for a text node.
///
/// The canonical CRDT placement history lives at the document level. This type
/// intentionally stores only the current visible projection so callers can read
/// text nodes ergonomically without carrying `Doc` through every path.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Text {
    visible: imbl::Vector<TextPlacement>,
}

impl Text {
    pub fn new() -> Self {
        Self {
            visible: imbl::Vector::new(),
        }
    }

    pub fn from_visible_placements<I>(visible: I) -> Self
    where
        I: IntoIterator<Item = TextPlacement>,
    {
        Self {
            visible: visible.into_iter().collect(),
        }
    }

    pub fn entry_dot_at(&self, char_index: usize) -> Result<EntryDot, CrdtError> {
        let Some(placement) = self.visible.get(char_index) else {
            return Err(CrdtError::OffsetOutOfBounds {
                offset: char_index,
                len: self.len(),
            });
        };
        Ok(placement.entry_dot)
    }

    pub fn placement_at_visible_offset(
        &self,
        char_offset: usize,
    ) -> Result<Option<PlacementId>, CrdtError> {
        if char_offset == 0 {
            return Ok(None);
        }
        self.visible
            .get(char_offset - 1)
            .map(|placement| Some(placement.placement_id))
            .ok_or(CrdtError::OffsetOutOfBounds {
                offset: char_offset,
                len: self.len(),
            })
    }

    pub fn placement_before_offset(
        &self,
        char_offset: usize,
    ) -> Result<Option<PlacementId>, CrdtError> {
        self.placement_at_visible_offset(char_offset)
    }

    pub fn contains_visible_entry(&self, entry_dot: EntryDot) -> bool {
        self.visible
            .iter()
            .any(|placement| placement.entry_dot == entry_dot)
    }

    pub fn visible_offset_of_entry(&self, entry_dot: EntryDot) -> Option<usize> {
        self.visible
            .iter()
            .position(|placement| placement.entry_dot == entry_dot)
    }

    pub fn iter_visible_entries(&self) -> impl Iterator<Item = (EntryDot, char)> + '_ {
        self.visible
            .iter()
            .map(|placement| (placement.entry_dot, placement.ch))
    }

    pub fn iter_visible_placements(&self) -> impl Iterator<Item = TextPlacement> + '_ {
        self.visible.iter().copied()
    }

    pub fn visible_rank_of_placement(&self, placement: PlacementId) -> Option<usize> {
        self.visible
            .iter()
            .position(|entry| entry.placement_id == placement)
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for placement in &self.visible {
            write!(f, "{}", placement.ch)?;
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

    fn placement(id: u64, clock: u64, ch: char) -> TextPlacement {
        let dot = Dot::new(id, clock);
        TextPlacement {
            placement_id: PlacementId(dot),
            entry_dot: EntryDot(dot),
            ch,
        }
    }

    #[test]
    fn new_yields_empty_projection() {
        let t = Text::new();
        assert_eq!(t.len(), 0);
        assert!(t.is_empty());
        assert_eq!(t.to_string(), "");
    }

    #[test]
    fn visible_projection_reads_like_text() {
        let t = Text::from_visible_placements([placement(1, 0, 'h'), placement(1, 1, 'i')]);
        assert_eq!(t.to_string(), "hi");
        assert_eq!(t.len(), 2);
        assert_eq!(
            t.iter_visible_entries().collect::<Vec<_>>(),
            vec![
                (EntryDot(Dot::new(1, 0)), 'h'),
                (EntryDot(Dot::new(1, 1)), 'i')
            ]
        );
    }

    #[test]
    fn entry_and_placement_offsets_are_explicit() {
        let d1 = Dot::new(1, 0);
        let d2 = Dot::new(2, 0);
        let t = Text::from_visible_placements([
            TextPlacement {
                placement_id: PlacementId(d1),
                entry_dot: EntryDot(d1),
                ch: 'h',
            },
            TextPlacement {
                placement_id: PlacementId(d2),
                entry_dot: EntryDot(Dot::new(1, 1)),
                ch: 'i',
            },
        ]);

        assert_eq!(t.entry_dot_at(0), Ok(EntryDot(d1)));
        assert_eq!(t.entry_dot_at(1), Ok(EntryDot(Dot::new(1, 1))));
        assert!(t.entry_dot_at(2).is_err());
        assert_eq!(t.placement_at_visible_offset(0), Ok(None));
        assert_eq!(t.placement_at_visible_offset(1), Ok(Some(PlacementId(d1))));
        assert_eq!(t.placement_at_visible_offset(2), Ok(Some(PlacementId(d2))));
        assert!(t.placement_at_visible_offset(3).is_err());
    }
}

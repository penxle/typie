use editor_model::Doc;
use std::marker::PhantomData;

use crate::position::Position;
use crate::resolved_position::ResolvedPosition;
use crate::selection::Selection;

/// A [`Selection`](crate::Selection) resolved against a specific
/// [`Doc`] (via [`Selection::resolve`](crate::Selection::resolve)),
/// holding two [`ResolvedPosition`]s.
///
/// Provides direction-independent views via [`from`](Self::from) and
/// [`to`](Self::to), which return the earlier/later endpoint by
/// `ResolvedPosition` ordering (path, then affinity — see
/// [`ResolvedPosition`]). The underlying `anchor`/`head` pair retains
/// its directional intent and is **not** normalized.
///
/// [`is_collapsed`](Self::is_collapsed) returns true iff `anchor` and
/// `head` match on every field of [`Position`](crate::Position)
/// (node_id, offset, affinity) — same semantics as
/// [`Selection::is_collapsed`](crate::Selection::is_collapsed).
pub struct ResolvedSelection<'a> {
    _marker: PhantomData<&'a Doc>,
    anchor: ResolvedPosition<'a>,
    head: ResolvedPosition<'a>,
}

impl<'a> ResolvedSelection<'a> {
    pub(crate) fn resolve(doc: &'a Doc, selection: Selection) -> Option<Self> {
        let anchor = ResolvedPosition::resolve(doc, selection.anchor)?;
        let head = ResolvedPosition::resolve(doc, selection.head)?;
        Some(Self {
            _marker: PhantomData,
            anchor,
            head,
        })
    }

    pub fn anchor(&self) -> &ResolvedPosition<'a> {
        &self.anchor
    }

    pub fn head(&self) -> &ResolvedPosition<'a> {
        &self.head
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }

    pub fn from(&self) -> &ResolvedPosition<'a> {
        if self.anchor <= self.head {
            &self.anchor
        } else {
            &self.head
        }
    }

    pub fn to(&self) -> &ResolvedPosition<'a> {
        if self.anchor <= self.head {
            &self.head
        } else {
            &self.anchor
        }
    }

    pub fn contains(&self, pos: &ResolvedPosition) -> bool {
        self.from() <= pos && pos <= self.to()
    }
}

impl From<&ResolvedSelection<'_>> for Selection {
    fn from(resolved: &ResolvedSelection<'_>) -> Self {
        Selection::new(
            Position::from(&resolved.anchor),
            Position::from(&resolved.head),
        )
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

    use crate::{Position, Selection};

    /// Build:
    /// Root
    ///   ├── P1 (Paragraph)
    ///   │   ├── T1 (Text "Hello")
    ///   │   └── T2 (Text "World")
    ///   └── P2 (Paragraph)
    ///       └── T3 (Text "!")
    fn make_doc() -> (Doc, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let (doc, p1, t1, t2, p2, t3, ..) = doc! {
            root {
                p1: paragraph {
                    t1: text("Hello")
                    t2: text("World")
                }
                p2: paragraph {
                    t3: text("!")
                }
            }
        };
        (doc, p1, p2, t1, t2, t3)
    }

    #[test]
    fn resolve_valid_selection() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        assert!(sel.resolve(&doc).is_some());
    }

    #[test]
    fn resolve_invalid_anchor() {
        let (doc, _, _, _, t2, _) = make_doc();
        let sel = Selection::new(Position::new(NodeId::new(), 0), Position::new(t2, 0));
        assert!(sel.resolve(&doc).is_none());
    }

    #[test]
    fn resolve_invalid_head() {
        let (doc, _, _, t1, _, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 0), Position::new(NodeId::new(), 0));
        assert!(sel.resolve(&doc).is_none());
    }

    #[test]
    fn is_collapsed_true() {
        let (doc, _, _, t1, _, _) = make_doc();
        let sel = Selection::collapsed(Position::new(t1, 2));
        let resolved = sel.resolve(&doc).unwrap();
        assert!(resolved.is_collapsed());
    }

    #[test]
    fn is_collapsed_false() {
        let (doc, _, _, t1, _, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 1), Position::new(t1, 3));
        let resolved = sel.resolve(&doc).unwrap();
        assert!(!resolved.is_collapsed());
    }

    #[test]
    fn from_to_forward_selection() {
        let (doc, _, _, t1, t2, _) = make_doc();
        // anchor before head (forward)
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();
        assert_eq!(resolved.from().node_id(), t1);
        assert_eq!(resolved.from().offset(), 2);
        assert_eq!(resolved.to().node_id(), t2);
        assert_eq!(resolved.to().offset(), 3);
    }

    #[test]
    fn from_to_backward_selection() {
        let (doc, _, _, t1, t2, _) = make_doc();
        // anchor after head (backward)
        let sel = Selection::new(Position::new(t2, 3), Position::new(t1, 2));
        let resolved = sel.resolve(&doc).unwrap();
        assert_eq!(resolved.from().node_id(), t1);
        assert_eq!(resolved.from().offset(), 2);
        assert_eq!(resolved.to().node_id(), t2);
        assert_eq!(resolved.to().offset(), 3);
    }

    #[test]
    fn contains_position_inside() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();

        // t1 offset 4 is between t1:2 and t2:3
        let pos = Position::new(t1, 4).resolve(&doc).unwrap();
        assert!(resolved.contains(&pos));
    }

    #[test]
    fn contains_position_at_from_boundary() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();

        let pos = Position::new(t1, 2).resolve(&doc).unwrap();
        assert!(resolved.contains(&pos));
    }

    #[test]
    fn contains_position_at_to_boundary() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();

        let pos = Position::new(t2, 3).resolve(&doc).unwrap();
        assert!(resolved.contains(&pos));
    }

    #[test]
    fn contains_position_outside_before() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();

        let pos = Position::new(t1, 0).resolve(&doc).unwrap();
        assert!(!resolved.contains(&pos));
    }

    #[test]
    fn contains_position_outside_after() {
        let (doc, _, _, t1, _, t3) = make_doc();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t1, 4));
        let resolved = sel.resolve(&doc).unwrap();

        let pos = Position::new(t3, 0).resolve(&doc).unwrap();
        assert!(!resolved.contains(&pos));
    }
}

use editor_model::{Doc, NodeId};
use std::cmp::Ordering;
use std::marker::PhantomData;

use crate::affinity::Affinity;
use crate::position::Position;

pub struct ResolvedPosition<'a> {
    position: Position,
    path: Vec<usize>,
    _marker: PhantomData<&'a Doc>,
}

impl<'a> ResolvedPosition<'a> {
    pub(crate) fn resolve(doc: &'a Doc, position: Position) -> Option<Self> {
        let node_ref = doc.node(position.node_id)?;
        let mut path = node_ref.path();
        path.push(position.offset);

        Some(Self {
            position,
            path,
            _marker: PhantomData,
        })
    }

    pub fn node_id(&self) -> NodeId {
        self.position.node_id
    }

    pub fn offset(&self) -> usize {
        self.position.offset
    }

    pub fn affinity(&self) -> Affinity {
        self.position.affinity
    }

    pub fn path(&self) -> &[usize] {
        &self.path
    }
}

impl PartialEq for ResolvedPosition<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for ResolvedPosition<'_> {}

impl PartialOrd for ResolvedPosition<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResolvedPosition<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl From<&ResolvedPosition<'_>> for Position {
    fn from(resolved: &ResolvedPosition<'_>) -> Self {
        resolved.position
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::*;

    use crate::Position;

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
    fn resolve_valid_position() {
        let (doc, _, _, t1, _, _) = make_doc();
        let pos = Position::new(t1, 2);
        let resolved = pos.resolve(&doc);
        assert!(resolved.is_some());

        let resolved = resolved.unwrap();
        assert_eq!(resolved.node_id(), t1);
        assert_eq!(resolved.offset(), 2);
        assert_eq!(resolved.path(), &[0, 0, 2]);
    }

    #[test]
    fn resolve_invalid_position() {
        let (doc, _, _, _, _, _) = make_doc();
        let pos = Position::new(NodeId::new(), 0);
        assert!(pos.resolve(&doc).is_none());
    }

    #[test]
    fn cmp_same_node_different_offset() {
        let (doc, _, _, t1, _, _) = make_doc();
        let a = Position::new(t1, 1).resolve(&doc).unwrap();
        let b = Position::new(t1, 3).resolve(&doc).unwrap();
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Less);
        assert_eq!(b.cmp(&a), std::cmp::Ordering::Greater);
    }

    #[test]
    fn cmp_same_position() {
        let (doc, _, _, t1, _, _) = make_doc();
        let a = Position::new(t1, 2).resolve(&doc).unwrap();
        let b = Position::new(t1, 2).resolve(&doc).unwrap();
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }

    #[test]
    fn cmp_different_nodes_same_parent() {
        let (doc, _, _, t1, t2, _) = make_doc();
        let a = Position::new(t1, 4).resolve(&doc).unwrap();
        let b = Position::new(t2, 0).resolve(&doc).unwrap();
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Less);
    }

    #[test]
    fn cmp_across_paragraphs() {
        let (doc, _, _, _, t2, t3) = make_doc();
        let a = Position::new(t2, 3).resolve(&doc).unwrap();
        let b = Position::new(t3, 0).resolve(&doc).unwrap();
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Less);
    }
}

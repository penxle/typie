use editor_common::StrExt;
use editor_model::{Doc, Node, NodeId};
use editor_resource::Resource;
use std::cmp::Ordering;

use crate::affinity::Affinity;
use crate::position::Position;

/// A [`Position`](crate::Position) resolved against a specific [`Doc`],
/// with its `path` from the document root pre-computed.
///
/// `ResolvedPosition` is obtained via [`Position::resolve`](crate::Position::resolve)
/// and borrows from the document. It guarantees that `node_id` exists in
/// the doc; other value-level invariants (offset range, node type) are
/// **not** currently checked by `resolve`.
///
/// `PartialEq` compares the full triple `(path, offset, affinity)`
/// via `(path, affinity)` (path already includes `offset` as its last
/// element). `Ord` compares `(path, affinity)` lexicographically, with
/// `Upstream < Downstream` — so at the same boundary, the `Upstream`
/// position sorts before the `Downstream` one.
pub struct ResolvedPosition<'a> {
    doc: &'a Doc,
    position: Position,
    path: Vec<usize>,
}

impl<'a> ResolvedPosition<'a> {
    pub(crate) fn resolve(doc: &'a Doc, position: Position) -> Option<Self> {
        let node_ref = doc.node(position.node_id)?;
        let mut path = node_ref.path();
        path.push(position.offset);

        Some(Self {
            doc,
            position,
            path,
        })
    }

    pub fn doc(&self) -> &'a Doc {
        self.doc
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

    fn grapheme_boundaries(&self, resource: &Resource) -> Option<Vec<usize>> {
        let node = self.doc.node(self.position.node_id)?;
        let Node::Text(text_node) = node.node() else {
            return None;
        };
        let text = &text_node.text;
        let segmenters = resource.segmenters.as_ref()?;

        let mut boundaries = vec![0usize];
        for byte_offset in segmenters.grapheme.as_borrowed().segment_str(text) {
            boundaries.push(text.nth_byte_char_offset(byte_offset));
        }
        Some(boundaries)
    }

    pub fn snap_to_grapheme(&self, resource: &Resource) -> ResolvedPosition<'a> {
        let Some(boundaries) = self.grapheme_boundaries(resource) else {
            return Self::resolve(self.doc, self.position).unwrap();
        };
        let offset = self.position.offset;
        if boundaries.contains(&offset) {
            return Self::resolve(self.doc, self.position).unwrap();
        }
        let snapped = match self.position.affinity {
            Affinity::Upstream => boundaries
                .iter()
                .copied()
                .rfind(|&b| b <= offset)
                .unwrap_or(0),
            Affinity::Downstream => boundaries
                .iter()
                .copied()
                .find(|&b| b >= offset)
                .unwrap_or(*boundaries.last().unwrap_or(&0)),
        };
        let new_pos = Position {
            node_id: self.position.node_id,
            offset: snapped,
            affinity: self.position.affinity,
        };
        Self::resolve(self.doc, new_pos).unwrap()
    }

    pub fn next_grapheme(&self, resource: &Resource) -> Option<ResolvedPosition<'a>> {
        let Some(boundaries) = self.grapheme_boundaries(resource) else {
            let node = self.doc.node(self.position.node_id)?;
            let Node::Text(text_node) = node.node() else {
                return None;
            };
            let next_offset = self.position.offset + 1;
            if next_offset > text_node.text.char_count() {
                return None;
            }
            let new_pos = Position::new(self.position.node_id, next_offset);
            return Self::resolve(self.doc, new_pos);
        };
        let offset = self.position.offset;
        let next = boundaries.iter().copied().find(|&b| b > offset)?;
        let new_pos = Position::new(self.position.node_id, next);
        Self::resolve(self.doc, new_pos)
    }

    pub fn prev_grapheme(&self, resource: &Resource) -> Option<ResolvedPosition<'a>> {
        let offset = self.position.offset;
        if offset == 0 {
            return None;
        }
        let Some(boundaries) = self.grapheme_boundaries(resource) else {
            let new_pos = Position::new(self.position.node_id, offset - 1);
            return Self::resolve(self.doc, new_pos);
        };
        let prev = boundaries.iter().copied().rfind(|&b| b < offset)?;
        let new_pos = Position::new(self.position.node_id, prev);
        Self::resolve(self.doc, new_pos)
    }
}

impl PartialEq for ResolvedPosition<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.position.affinity == other.position.affinity
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
        self.path
            .cmp(&other.path)
            .then_with(|| self.position.affinity.cmp(&other.position.affinity))
    }
}

// Omits `doc` to avoid printing the full document tree.
impl std::fmt::Debug for ResolvedPosition<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedPosition")
            .field("position", &self.position)
            .field("path", &self.path)
            .finish()
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

    use crate::{Affinity, Position};

    use editor_resource::{Resource, TextSegmenters};
    use std::sync::Arc;

    fn resource_with_segmenters() -> Resource {
        let mut r = Resource::new();
        r.segmenters = Some(Arc::new(TextSegmenters::new_test()));
        r
    }

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

    #[test]
    fn eq_differs_when_affinity_differs() {
        let (doc, _, _, t1, _, _) = make_doc();
        let a = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Upstream,
        }
        .resolve(&doc)
        .unwrap();
        let b = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Downstream,
        }
        .resolve(&doc)
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn eq_same_when_affinity_same() {
        let (doc, _, _, t1, _, _) = make_doc();
        let a = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Upstream,
        }
        .resolve(&doc)
        .unwrap();
        let b = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Upstream,
        }
        .resolve(&doc)
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn cmp_same_path_upstream_before_downstream() {
        let (doc, _, _, t1, _, _) = make_doc();
        let up = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Upstream,
        }
        .resolve(&doc)
        .unwrap();
        let down = Position {
            node_id: t1,
            offset: 2,
            affinity: Affinity::Downstream,
        }
        .resolve(&doc)
        .unwrap();
        assert_eq!(up.cmp(&down), std::cmp::Ordering::Less);
        assert_eq!(down.cmp(&up), std::cmp::Ordering::Greater);
    }

    #[test]
    fn snap_noop_at_boundary() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let rp = Position::new(t, 1).resolve(&doc).unwrap();
        assert_eq!(rp.snap_to_grapheme(&r).offset(), 1);
    }

    #[test]
    fn snap_noop_at_start_and_end() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let start = Position::new(t, 0).resolve(&doc).unwrap();
        let end = Position::new(t, 3).resolve(&doc).unwrap();
        assert_eq!(start.snap_to_grapheme(&r).offset(), 0);
        assert_eq!(end.snap_to_grapheme(&r).offset(), 3);
    }

    #[test]
    fn snap_upstream_combining_mark() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("e\u{0301}") } } };
        let rp = Position {
            node_id: t,
            offset: 1,
            affinity: Affinity::Upstream,
        }
        .resolve(&doc)
        .unwrap();
        assert_eq!(rp.snap_to_grapheme(&r).offset(), 0);
    }

    #[test]
    fn snap_downstream_combining_mark() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("e\u{0301}") } } };
        let rp = Position {
            node_id: t,
            offset: 1,
            affinity: Affinity::Downstream,
        }
        .resolve(&doc)
        .unwrap();
        assert_eq!(rp.snap_to_grapheme(&r).offset(), 2);
    }

    #[test]
    fn snap_without_segmenters_is_noop() {
        let r = Resource::new();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let rp = Position::new(t, 1).resolve(&doc).unwrap();
        assert_eq!(rp.snap_to_grapheme(&r).offset(), 1);
    }

    #[test]
    fn next_grapheme_ascii() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(0).next_grapheme(&r).unwrap().offset(), 1);
        assert_eq!(at(1).next_grapheme(&r).unwrap().offset(), 2);
        assert_eq!(at(2).next_grapheme(&r).unwrap().offset(), 3);
        assert!(at(3).next_grapheme(&r).is_none());
    }

    #[test]
    fn next_grapheme_skips_combining_mark() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("ae\u{0301}b") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(0).next_grapheme(&r).unwrap().offset(), 1);
        assert_eq!(at(1).next_grapheme(&r).unwrap().offset(), 3);
        assert_eq!(at(3).next_grapheme(&r).unwrap().offset(), 4);
        assert!(at(4).next_grapheme(&r).is_none());
    }

    #[test]
    fn next_grapheme_flag_emoji() {
        let r = resource_with_segmenters();
        // 🇰🇷 = U+1F1F0 U+1F1F7, each is 2 codepoints wide in UTF-32 — actually each is a single codepoint
        // U+1F1F0 = 1 codepoint, U+1F1F7 = 1 codepoint, together = 1 grapheme
        let (doc, t) = doc! { root { paragraph { t: text("\u{1F1F0}\u{1F1F7}") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(0).next_grapheme(&r).unwrap().offset(), 2);
        assert!(at(2).next_grapheme(&r).is_none());
    }

    #[test]
    fn next_grapheme_from_middle_of_grapheme() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("e\u{0301}") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(1).next_grapheme(&r).unwrap().offset(), 2);
    }

    #[test]
    fn next_grapheme_on_empty_text() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("") } } };
        let rp = Position::new(t, 0).resolve(&doc).unwrap();
        assert!(rp.next_grapheme(&r).is_none());
    }

    #[test]
    fn next_grapheme_without_segmenters_falls_back() {
        let r = Resource::new();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let rp = Position::new(t, 0).resolve(&doc).unwrap();
        assert_eq!(rp.next_grapheme(&r).unwrap().offset(), 1);
    }

    #[test]
    fn prev_grapheme_ascii() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("abc") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert!(at(0).prev_grapheme(&r).is_none());
        assert_eq!(at(1).prev_grapheme(&r).unwrap().offset(), 0);
        assert_eq!(at(3).prev_grapheme(&r).unwrap().offset(), 2);
    }

    #[test]
    fn prev_grapheme_skips_combining_mark() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("ae\u{0301}b") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(4).prev_grapheme(&r).unwrap().offset(), 3);
        assert_eq!(at(3).prev_grapheme(&r).unwrap().offset(), 1);
        assert_eq!(at(1).prev_grapheme(&r).unwrap().offset(), 0);
        assert!(at(0).prev_grapheme(&r).is_none());
    }

    #[test]
    fn prev_grapheme_flag_emoji() {
        let r = resource_with_segmenters();
        let (doc, t) = doc! { root { paragraph { t: text("\u{1F1F0}\u{1F1F7}") } } };
        let at = |off: usize| Position::new(t, off).resolve(&doc).unwrap();
        assert_eq!(at(2).prev_grapheme(&r).unwrap().offset(), 0);
        assert!(at(0).prev_grapheme(&r).is_none());
    }

    #[test]
    fn grapheme_ops_on_non_text_node() {
        let r = resource_with_segmenters();
        let (doc, p, ..) = doc! { root { p: paragraph { t: text("hello") } } };
        let rp = Position::new(p, 0).resolve(&doc).unwrap();
        assert!(rp.next_grapheme(&r).is_none());
        assert!(rp.prev_grapheme(&r).is_none());
        assert_eq!(rp.snap_to_grapheme(&r).offset(), 0);
    }
}

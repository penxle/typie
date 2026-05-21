use editor_model::{Doc, Node, NodeRef};

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
    doc: &'a Doc,
    anchor: ResolvedPosition<'a>,
    head: ResolvedPosition<'a>,
}

impl<'a> ResolvedSelection<'a> {
    pub(crate) fn resolve(doc: &'a Doc, selection: Selection) -> Option<Self> {
        let anchor = ResolvedPosition::resolve(doc, selection.anchor)?;
        let head = ResolvedPosition::resolve(doc, selection.head)?;
        Some(Self { doc, anchor, head })
    }

    pub fn doc(&self) -> &'a Doc {
        self.doc
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

    /// Deepest ancestor that contains both `anchor` and `head`.
    ///
    /// Walks the two ancestor chains from the root downward and returns the last node
    /// they share. Both endpoints always share at least the document root.
    pub fn common_ancestor(&self) -> NodeRef<'a> {
        let doc = self.doc;
        let anchor_node = doc.node(self.anchor.node_id()).expect("anchor node exists");
        let head_node = doc.node(self.head.node_id()).expect("head node exists");

        // ancestors() yields self → root; reverse to compare from the root downward.
        let anchor_chain: Vec<NodeRef<'a>> = anchor_node.ancestors().collect();
        let head_chain: Vec<NodeRef<'a>> = head_node.ancestors().collect();
        let mut common: Option<NodeRef<'a>> = None;
        for (a, h) in anchor_chain.iter().rev().zip(head_chain.iter().rev()) {
            if a.id() == h.id() {
                common = Some(*a);
            } else {
                break;
            }
        }
        common.expect("both anchor and head share at least the root ancestor")
    }

    /// `true` iff `node`'s subtree overlaps the selection range.
    ///
    /// "Overlap" means the subtree of `node` shares at least one position with the
    /// selection (partial intersection counts). Implemented via path comparison.
    pub fn intersects_subtree(&self, node: &NodeRef<'_>) -> bool {
        let from_path = self.from().path();
        let to_path = self.to().path();
        let node_path = node.path();

        // node is an ancestor of either endpoint — the endpoint lies inside its subtree.
        if is_prefix_of(&node_path, from_path) || is_prefix_of(&node_path, to_path) {
            return true;
        }

        // node is a sibling under a parent that contains both endpoints —
        // check whether node's index sits between the endpoints' indices at that depth.
        if !node_path.is_empty() {
            let (&node_idx, node_parent) = node_path.split_last().unwrap();
            if is_prefix_of(node_parent, from_path) && is_prefix_of(node_parent, to_path) {
                let from_idx = from_path.get(node_parent.len()).copied().unwrap_or(0);
                let to_idx = to_path.get(node_parent.len()).copied().unwrap_or(0);
                let lo = from_idx.min(to_idx);
                let hi = from_idx.max(to_idx);
                if lo <= node_idx && node_idx <= hi {
                    return true;
                }
            }
        }

        if node_path.as_slice() > from_path && node_path.as_slice() < to_path {
            return true;
        }

        false
    }

    /// `true` iff `node` and its entire subtree are wholly contained within the selection.
    ///
    /// Path-based: the selection's `from` must be at-or-before the boundary just before
    /// `node`'s subtree, and `to` at-or-after the boundary just after. A position counts as
    /// "inside" `node` (and therefore not at-or-before-start) if its node lies strictly
    /// deeper than `node` in the tree — the selection then starts within the subtree,
    /// leaving the leading content outside.
    ///
    /// Text-node end uses `text.len()` (offset is a character index, not a child
    /// index); container nodes use `children().count()`.
    pub fn contains_subtree(&self, node: &NodeRef<'_>) -> bool {
        let from = self.from();
        let to = self.to();
        let from_node = from.doc().node(from.node_id()).expect("from node exists");
        let to_node = to.doc().node(to.node_id()).expect("to node exists");

        let from_node_path = from_node.path();
        let to_node_path = to_node.path();
        let node_path = node.path();
        let node_end_offset = match node.node() {
            Node::Text(t) => t.text.len(),
            _ => node.children().count(),
        };

        position_before_or_at_node_start(&from_node_path, from.offset(), &node_path)
            && position_after_or_at_node_end(
                &to_node_path,
                to.offset(),
                &node_path,
                node_end_offset,
            )
    }
}

/// `true` iff the position `(pos_path, pos_offset)` is at or before the boundary
/// immediately before `node_path`'s subtree. `pos_path` is the position node's path
/// (not the resolved-position path that includes the offset).
fn position_before_or_at_node_start(
    pos_path: &[usize],
    pos_offset: usize,
    node_path: &[usize],
) -> bool {
    for (i, &node_idx) in node_path.iter().enumerate() {
        match pos_path.get(i).copied() {
            Some(p) if p < node_idx => return true,
            Some(p) if p > node_idx => return false,
            Some(_) => continue,
            None => return pos_offset <= node_idx,
        }
    }
    // pos_path matched all of node_path. If equal length, pos is on node itself —
    // offset 0 is the start boundary. If pos_path is longer, pos is strictly inside
    // node's subtree, so the selection starts inside node — not at-or-before-start.
    pos_path.len() == node_path.len() && pos_offset == 0
}

/// `true` iff the position `(pos_path, pos_offset)` is at or after the boundary
/// immediately after `node_path`'s subtree. `node_end_offset` is the offset that
/// represents "end of node's content" — `text.len()` for text, or
/// `children().count()` for containers.
fn position_after_or_at_node_end(
    pos_path: &[usize],
    pos_offset: usize,
    node_path: &[usize],
    node_end_offset: usize,
) -> bool {
    for (i, &node_idx) in node_path.iter().enumerate() {
        match pos_path.get(i).copied() {
            Some(p) if p > node_idx => return true,
            Some(p) if p < node_idx => return false,
            Some(_) => continue,
            None => return pos_offset > node_idx,
        }
    }
    pos_path.len() == node_path.len() && pos_offset >= node_end_offset
}

fn is_prefix_of(prefix: &[usize], full: &[usize]) -> bool {
    prefix.len() <= full.len() && prefix == &full[..prefix.len()]
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
    use editor_macros::{doc, state};
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

    #[test]
    fn common_ancestor_two_text_in_same_paragraph_returns_paragraph() {
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Hello")
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let ancestor = rs.common_ancestor();
        assert_eq!(ancestor.id(), p1);
    }

    #[test]
    fn common_ancestor_text_across_paragraphs_returns_root() {
        let (state, ..) = state! {
            doc { root {
                paragraph { t1: text("A") }
                paragraph { t2: text("B") }
            } }
            selection: (t1, 0) -> (t2, 1)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        assert_eq!(rs.common_ancestor().id(), NodeId::ROOT);
    }

    #[test]
    fn intersects_subtree_node_inside_range_returns_true() {
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let p1_ref = state.doc.node(p1).unwrap();
        assert!(rs.intersects_subtree(&p1_ref));
    }

    #[test]
    fn intersects_subtree_descendant_inside_band_across_rows_returns_true() {
        let (state, tr1, c1, p1, tr2, ..) = state! {
            doc {
                root {
                    table {
                        tr1: table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            c1: table_cell {
                                p1: paragraph { text("mid") }
                            }
                            table_cell { paragraph {} }
                        }
                        tr2: table_row {
                            table_cell { paragraph { text("z") } }
                            table_cell { paragraph {} }
                        }
                    }
                }
            }
            selection: (tr1, 0, >) -> (tr2, 2, <)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let c1_ref = state.doc.node(c1).unwrap();
        let p1_ref = state.doc.node(p1).unwrap();
        assert!(rs.intersects_subtree(&c1_ref));
        assert!(rs.intersects_subtree(&p1_ref));
    }

    #[test]
    fn intersects_subtree_node_disjoint_returns_false() {
        let (state, _p1, _t1, p2, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("A") }
                p2: paragraph { t2: text("B") }
            } }
            selection: (t1, 0) -> (t1, 1)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let p2_ref = state.doc.node(p2).unwrap();
        assert!(!rs.intersects_subtree(&p2_ref));
    }

    #[test]
    fn contains_subtree_partial_text_returns_false() {
        // Selection fully inside a single text node does NOT wholly contain its parent paragraph.
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let p1_ref = state.doc.node(p1).unwrap();
        assert!(!rs.contains_subtree(&p1_ref));
    }

    #[test]
    fn contains_subtree_partial_returns_false() {
        let (state, p1, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("Hello") }
                p2: paragraph { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let p1_ref = state.doc.node(p1).unwrap();
        assert!(!rs.contains_subtree(&p1_ref));
    }

    #[test]
    fn contains_subtree_root_when_selection_spans_whole_doc_returns_true() {
        let (state, root, ..) = state! {
            doc { root: root { paragraph { text("Hello") } } }
            selection: (root, 0) -> (root, 1)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let root_ref = state.doc.node(root).unwrap();
        assert!(rs.contains_subtree(&root_ref));
    }

    #[test]
    fn contains_subtree_paragraph_wholly_inside_multi_paragraph_selection_returns_true() {
        let (state, _, p2, _) = state! {
            doc { root {
                paragraph { t1: text("first") }
                p2: paragraph { text("middle") }
                paragraph { t3: text("last") }
            } }
            selection: (t1, 0) -> (t3, 4)
        };
        let rs = state.selection.resolve(&state.doc).unwrap();
        let p2_ref = state.doc.node(p2).unwrap();
        assert!(rs.contains_subtree(&p2_ref));
    }
}

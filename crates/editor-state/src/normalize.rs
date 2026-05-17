use editor_model::{Doc, Node, NodeRef, Schema};

use crate::affinity::Affinity;
use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;
use crate::selection::Selection;

fn boundary_identity(doc: &Doc, pos: Position) -> Vec<usize> {
    let node = doc
        .node(pos.node_id)
        .expect("boundary_identity: node must exist");
    if let Node::Text(text) = node.node() {
        let len = text.text.len();
        let mut path = node.path();
        if pos.offset == 0 {
            return path;
        }
        if pos.offset == len {
            // A text node's path last element is its index inside the parent;
            // the boundary immediately after the node is that index + 1.
            if let Some(last) = path.last_mut() {
                *last += 1;
            }
            return path;
        }
        path.push(pos.offset);
        return path;
    }
    let mut path = node.path();
    path.push(pos.offset);
    path
}

fn descend_or_stay_at_textblock<'a>(
    textblock: NodeRef<'a>,
    offset_in_parent: usize,
    aff: Affinity,
) -> Position {
    let prev_text_end = || -> Option<Position> {
        if offset_in_parent == 0 {
            return None;
        }
        let prev = textblock.children().nth(offset_in_parent - 1)?;
        if let Node::Text(t) = prev.node() {
            return Some(Position {
                node_id: prev.id(),
                offset: t.text.len(),
                affinity: aff,
            });
        }
        None
    };
    let next_text_start = || -> Option<Position> {
        let next = textblock.children().nth(offset_in_parent)?;
        if matches!(next.node(), Node::Text(_)) {
            return Some(Position {
                node_id: next.id(),
                offset: 0,
                affinity: aff,
            });
        }
        None
    };

    let primary = match aff {
        Affinity::Upstream => prev_text_end(),
        Affinity::Downstream => next_text_start(),
    };
    let fallback = || match aff {
        Affinity::Upstream => next_text_start(),
        Affinity::Downstream => prev_text_end(),
    };
    primary.or_else(fallback).unwrap_or(Position {
        node_id: textblock.id(),
        offset: offset_in_parent,
        affinity: aff,
    })
}

fn validate_position(doc: &Doc, pos: Position) -> bool {
    let Some(node) = doc.node(pos.node_id) else {
        return false;
    };
    if let Node::Text(t) = node.node() {
        return pos.offset <= t.text.len();
    }
    let spec = Schema::node_spec(node.as_type());
    if spec.is_leaf() {
        return false;
    }
    pos.offset <= node.children().count()
}

fn subtree_violation(a_path: &[usize], h_path: &[usize]) -> bool {
    let a_node = &a_path[..a_path.len() - 1];
    let h_node = &h_path[..h_path.len() - 1];
    if a_node == h_node {
        return false;
    }
    let (anc_node, anc_full, desc_node) = if a_node.starts_with(h_node) {
        (h_node, h_path, a_node)
    } else if h_node.starts_with(a_node) {
        (a_node, a_path, h_node)
    } else {
        return false;
    };
    let anc_slot = anc_full[anc_full.len() - 1];
    let desc_child_idx = desc_node[anc_node.len()];
    anc_slot == desc_child_idx || anc_slot == desc_child_idx + 1
}

fn is_unit_node(node: &NodeRef<'_>) -> bool {
    // A node-selectable unit = atom (selectable, non-inline, leaf) OR a
    // monolithic block (fold/table/callout/blockquote). Single source of
    // truth lives on NodeSpec so editor-commands shares the same predicate.
    Schema::node_spec(node.as_type()).is_unit()
}

/// At a container boundary, if the affinity-selected adjacent child is a unit
/// (atom or monolithic block), expands to that child's node selection. Affinity
/// convention: Upstream selects child[offset-1], Downstream selects
/// child[offset]. Returns None for a text node position or when the adjacent
/// child is not a unit.
fn expand_unit_at(doc: &Doc, pos: Position) -> Option<Selection> {
    let node = doc.node(pos.node_id)?;
    if matches!(node.node(), Node::Text(_)) {
        return None;
    }
    match pos.affinity {
        Affinity::Downstream => {
            let child = node.children().nth(pos.offset)?;
            if is_unit_node(&child) {
                return Some(Selection::new(
                    Position {
                        node_id: pos.node_id,
                        offset: pos.offset,
                        affinity: Affinity::Downstream,
                    },
                    Position {
                        node_id: pos.node_id,
                        offset: pos.offset + 1,
                        affinity: Affinity::Upstream,
                    },
                ));
            }
        }
        Affinity::Upstream => {
            let prev_idx = pos.offset.checked_sub(1)?;
            let child = node.children().nth(prev_idx)?;
            if is_unit_node(&child) {
                return Some(Selection::new(
                    Position {
                        node_id: pos.node_id,
                        offset: pos.offset,
                        affinity: Affinity::Upstream,
                    },
                    Position {
                        node_id: pos.node_id,
                        offset: prev_idx,
                        affinity: Affinity::Downstream,
                    },
                ));
            }
        }
    }
    None
}

/// Invariant: a normalized selection must never be collapsed at a unit boundary.
fn collapsed_or_unit(doc: &Doc, pos: Position) -> Selection {
    if let Some(node_sel) = expand_unit_at(doc, pos) {
        return node_sel;
    }
    Selection::collapsed(pos)
}

impl<'a> ResolvedSelection<'a> {
    /// Caller must supply endpoints that already pass `validate_position`.
    /// `Selection::normalize` enforces that gate; direct callers do not.
    pub fn normalize(&self) -> Selection {
        let doc = self.doc();
        let a_in: Position = self.anchor().into();
        let h_in: Position = self.head().into();

        let a_bd = boundary_identity(doc, a_in);
        let h_bd = boundary_identity(doc, h_in);

        if a_bd == h_bd {
            return collapsed_or_unit(doc, normalize_position(doc, h_in));
        }

        let (a_aff, h_aff) = if a_bd < h_bd {
            (Affinity::Downstream, Affinity::Upstream)
        } else {
            (Affinity::Upstream, Affinity::Downstream)
        };

        let a = Position {
            affinity: a_aff,
            ..a_in
        };
        let h = Position {
            affinity: h_aff,
            ..h_in
        };

        let a = normalize_position(doc, a);
        let h = normalize_position(doc, h);

        let a_resolved = a.resolve(doc).expect("normalized anchor resolves");
        let h_resolved = h.resolve(doc).expect("normalized head resolves");
        if subtree_violation(a_resolved.path(), h_resolved.path()) {
            // Preserve the caller's original head affinity in the fallback.
            return collapsed_or_unit(doc, normalize_position(doc, h_in));
        }

        Selection { anchor: a, head: h }
    }
}

impl Selection {
    pub fn normalize(&self, doc: &Doc) -> Option<Selection> {
        if !validate_position(doc, self.anchor) || !validate_position(doc, self.head) {
            return None;
        }
        self.resolve(doc).map(|r| r.normalize())
    }

    /// Navigation extend must treat a selection that already brackets a unit
    /// (atom or monolithic block) differently from a text/collapsed selection:
    /// it re-anchors to the unit's far edge so the unit stays selected while the
    /// range grows. This predicate is that classification gate. Callers pass
    /// selections already validated by `normalize`; an unnormalized/out-of-range
    /// selection harmlessly returns false. Direction-agnostic (forward or
    /// reversed unit selection both true).
    pub fn is_unit_node_selection(&self, doc: &Doc) -> bool {
        if self.anchor.node_id != self.head.node_id {
            return false;
        }
        let (lo, hi) = if self.anchor.offset <= self.head.offset {
            (self.anchor.offset, self.head.offset)
        } else {
            (self.head.offset, self.anchor.offset)
        };
        if lo.checked_add(1) != Some(hi) {
            return false;
        }
        let Some(node) = doc.node(self.anchor.node_id) else {
            return false;
        };
        // Text offsets are char positions, not child indices; children() is
        // meaningless on a text node.
        if matches!(node.node(), Node::Text(_)) {
            return false;
        }
        node.children()
            .nth(lo)
            .is_some_and(|child| is_unit_node(&child))
    }
}

fn normalize_position(doc: &Doc, pos: Position) -> Position {
    let Some(node) = doc.node(pos.node_id) else {
        return pos;
    };

    if let Node::Text(t) = node.node() {
        let len = t.text.len();
        if pos.offset > len {
            // Out-of-range; `validate_position` normally catches this, but
            // keep the helper itself safe for direct callers.
            return pos;
        }
        if pos.offset > 0 && pos.offset < len {
            return pos;
        }
        let Some(parent) = node.parent() else {
            return pos;
        };
        let Some(idx_in_parent) = node.index() else {
            return pos;
        };
        let offset_in_parent = if pos.offset == 0 {
            idx_in_parent
        } else {
            idx_in_parent + 1
        };
        return descend_or_stay_at_textblock(parent, offset_in_parent, pos.affinity);
    }

    let spec = Schema::node_spec(node.as_type());
    if spec.is_textblock() {
        return descend_or_stay_at_textblock(node, pos.offset, pos.affinity);
    }

    pos
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::{doc, state};
    use editor_model::NodeId;

    fn pos_eq(a: Position, b: Position) -> bool {
        a.node_id == b.node_id && a.offset == b.offset && a.affinity == b.affinity
    }

    #[test]
    fn boundary_identity_text_text_adjacency_all_equal() {
        let (d, ta, tb) = doc! {
            root {
                paragraph {
                    ta: text("Hello")
                    tb: text("World")
                }
            }
        };

        let p_id = d.node(ta).unwrap().parent().unwrap().id();

        let a_end_up = Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        let a_end_down = Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Downstream,
        };
        let b_start_up = Position {
            node_id: tb,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let b_start_dn = Position {
            node_id: tb,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let p_mid_up = Position {
            node_id: p_id,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let p_mid_dn = Position {
            node_id: p_id,
            offset: 1,
            affinity: Affinity::Downstream,
        };

        let bid = |pos| boundary_identity(&d, pos);
        let expected = bid(a_end_up);
        assert_eq!(bid(a_end_down), expected);
        assert_eq!(bid(b_start_up), expected);
        assert_eq!(bid(b_start_dn), expected);
        assert_eq!(bid(p_mid_up), expected);
        assert_eq!(bid(p_mid_dn), expected);
    }

    #[test]
    fn boundary_identity_text_strict_interior() {
        let (d, t) = doc! { root { paragraph { t: text("hello") } } };
        let up = Position {
            node_id: t,
            offset: 3,
            affinity: Affinity::Upstream,
        };
        let dn = Position {
            node_id: t,
            offset: 3,
            affinity: Affinity::Downstream,
        };
        assert_eq!(boundary_identity(&d, up), boundary_identity(&d, dn));
    }

    #[test]
    fn boundary_identity_distinct_block_boundaries_differ() {
        let (d, p1, p2) = doc! {
            root {
                p1: paragraph { text("a") }
                p2: paragraph { text("b") }
            }
        };
        let root_id = NodeId::ROOT;
        let bid_root_1 = boundary_identity(
            &d,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Downstream,
            },
        );
        let bid_p1_end = boundary_identity(
            &d,
            Position {
                node_id: p1,
                offset: 1,
                affinity: Affinity::Downstream,
            },
        );
        let bid_p2_start = boundary_identity(
            &d,
            Position {
                node_id: p2,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        );
        assert_ne!(bid_root_1, bid_p1_end);
        assert_ne!(bid_root_1, bid_p2_start);
        assert_ne!(bid_p1_end, bid_p2_start);
    }

    #[test]
    fn boundary_identity_empty_text_maps_to_before_boundary() {
        let (d, ta, te) = doc! {
            root {
                paragraph {
                    ta: text("a")
                    te: text("")
                    text("b")
                }
            }
        };
        let p_id = d.node(ta).unwrap().parent().unwrap().id();
        let bid_te0 = boundary_identity(&d, Position::new(te, 0));
        let bid_before_te = boundary_identity(&d, Position::new(p_id, 1));
        let bid_after_te = boundary_identity(&d, Position::new(p_id, 2));
        assert_eq!(bid_te0, bid_before_te);
        assert_ne!(bid_te0, bid_after_te);
    }

    #[test]
    fn normalize_position_text_text_adjacency_down_prefers_next_leaf() {
        let (d, ta, tb) = doc! {
            root {
                paragraph {
                    ta: text("Hello")
                    tb: text("World")
                }
            }
        };
        let p_id = d.node(ta).unwrap().parent().unwrap().id();
        let expected = Position {
            node_id: tb,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let inputs = [
            Position {
                node_id: ta,
                offset: 5,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tb,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Downstream,
            },
        ];
        for i in inputs {
            assert!(
                pos_eq(normalize_position(&d, i), expected),
                "input {:?} canonical {:?} expected {:?}",
                i,
                normalize_position(&d, i),
                expected
            );
        }
    }

    #[test]
    fn normalize_position_text_text_adjacency_up_prefers_prev_leaf() {
        let (d, ta, tb) = doc! {
            root {
                paragraph {
                    ta: text("Hello")
                    tb: text("World")
                }
            }
        };
        let p_id = d.node(ta).unwrap().parent().unwrap().id();
        let expected = Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        let inputs = [
            Position {
                node_id: ta,
                offset: 5,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: tb,
                offset: 0,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        ];
        for i in inputs {
            assert!(
                pos_eq(normalize_position(&d, i), expected),
                "input {:?} canonical {:?} expected {:?}",
                i,
                normalize_position(&d, i),
                expected
            );
        }
    }

    #[test]
    fn normalize_position_hard_break_neighbors() {
        let (d, ta, tb) = doc! {
            root {
                paragraph {
                    ta: text("Hello")
                    hard_break
                    tb: text("World")
                }
            }
        };
        let p_id = d.node(ta).unwrap().parent().unwrap().id();

        let exp_prev_up = Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: ta,
                offset: 5,
                affinity: Affinity::Upstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_prev_up));
        }

        let exp_prev_down = Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Downstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: ta,
                offset: 5,
                affinity: Affinity::Downstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_prev_down));
        }

        let exp_next_down = Position {
            node_id: tb,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 2,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tb,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_next_down));
        }

        let exp_next_up = Position {
            node_id: tb,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: tb,
                offset: 0,
                affinity: Affinity::Upstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_next_up));
        }
    }

    #[test]
    fn normalize_position_textblock_start_both_descend() {
        let (d, t) = doc! {
            root {
                paragraph {
                    t: text("Hello")
                }
            }
        };
        let p_id = d.node(t).unwrap().parent().unwrap().id();

        let exp_down = Position {
            node_id: t,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: t,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_down));
        }

        let exp_up = Position {
            node_id: t,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 0,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: t,
                offset: 0,
                affinity: Affinity::Upstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_up));
        }
    }

    #[test]
    fn normalize_position_textblock_end_both_descend() {
        let (d, t) = doc! {
            root {
                paragraph {
                    t: text("Hello")
                }
            }
        };
        let p_id = d.node(t).unwrap().parent().unwrap().id();

        let exp_up = Position {
            node_id: t,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: t,
                offset: 5,
                affinity: Affinity::Upstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_up));
        }

        let exp_down = Position {
            node_id: t,
            offset: 5,
            affinity: Affinity::Downstream,
        };
        for inp in [
            Position {
                node_id: p_id,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: t,
                offset: 5,
                affinity: Affinity::Downstream,
            },
        ] {
            assert!(pos_eq(normalize_position(&d, inp), exp_down));
        }
    }

    #[test]
    fn normalize_position_empty_textblock_stays() {
        let (d, p) = doc! {
            root { p: paragraph {} }
        };
        for aff in [Affinity::Upstream, Affinity::Downstream] {
            let inp = Position {
                node_id: p,
                offset: 0,
                affinity: aff,
            };
            assert!(pos_eq(normalize_position(&d, inp), inp));
        }
    }

    #[test]
    fn normalize_position_text_strict_interior_unchanged() {
        let (d, t) = doc! { root { paragraph { t: text("Hello") } } };
        for aff in [Affinity::Upstream, Affinity::Downstream] {
            let inp = Position {
                node_id: t,
                offset: 3,
                affinity: aff,
            };
            assert!(pos_eq(normalize_position(&d, inp), inp));
        }
    }

    #[test]
    fn normalize_position_non_textblock_container_unchanged() {
        let (d, ..) = doc! {
            root {
                paragraph { text("a") }
            }
        };
        let root_id = NodeId::ROOT;
        for off in [0usize, 1] {
            for aff in [Affinity::Upstream, Affinity::Downstream] {
                let inp = Position {
                    node_id: root_id,
                    offset: off,
                    affinity: aff,
                };
                assert!(pos_eq(normalize_position(&d, inp), inp));
            }
        }
    }

    #[test]
    fn validate_position_valid_text_offsets() {
        let (d, t) = doc! { root { paragraph { t: text("hi") } } };
        for off in 0..=2 {
            assert!(validate_position(&d, Position::new(t, off)));
        }
    }

    #[test]
    fn validate_position_text_offset_out_of_range() {
        let (d, t) = doc! { root { paragraph { t: text("hi") } } };
        assert!(!validate_position(&d, Position::new(t, 3)));
    }

    #[test]
    fn validate_position_container_offsets() {
        let (d, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        let root_id = editor_model::NodeId::ROOT;
        for off in 0..=2 {
            assert!(validate_position(&d, Position::new(root_id, off)));
        }
        assert!(!validate_position(&d, Position::new(root_id, 3)));
    }

    #[test]
    fn validate_position_non_text_leaf_rejected() {
        let (d, hb) = doc! {
            root {
                paragraph {
                    text("a")
                    hb: hard_break
                    text("b")
                }
            }
        };
        assert!(!validate_position(&d, Position::new(hb, 0)));
    }

    #[test]
    fn validate_position_unknown_node_id() {
        let (d, ..) = doc! { root { paragraph { text("hi") } } };
        assert!(!validate_position(
            &d,
            Position::new(editor_model::NodeId::new(), 0)
        ));
    }

    #[test]
    fn subtree_violation_disjoint_siblings() {
        let a = vec![0usize, 0, 3];
        let h = vec![0usize, 1, 2];
        assert!(!subtree_violation(&a, &h));
    }

    #[test]
    fn subtree_violation_same_node() {
        let a = vec![0usize, 0, 3];
        let h = vec![0usize, 0, 5];
        assert!(!subtree_violation(&a, &h));
    }

    #[test]
    fn subtree_violation_anchor_inside_head() {
        let a = vec![0usize, 0, 0];
        let h = vec![0usize, 1];
        assert!(subtree_violation(&a, &h));
    }

    #[test]
    fn subtree_violation_head_inside_anchor() {
        let a = vec![0usize, 1];
        let h = vec![0usize, 0, 0];
        assert!(subtree_violation(&a, &h));
    }

    #[test]
    fn subtree_violation_non_adjacent_slot_is_preserved() {
        let a = vec![0usize]; // (root, 0): node []=root, offset 0
        let h = vec![1usize, 0, 3]; // node [1,0], offset 3
        assert!(!subtree_violation(&a, &h));
        assert!(!subtree_violation(&h, &a));
    }

    #[test]
    fn subtree_violation_far_back_slot_is_preserved() {
        let a = vec![2usize]; // (root, 2)
        let h = vec![0usize, 0, 1]; // node [0,0], offset 1
        assert!(!subtree_violation(&a, &h));
        assert!(!subtree_violation(&h, &a));
    }

    #[test]
    fn normalize_position_empty_text_leaf_maps_consistently() {
        let (d, ta, te) = doc! {
            root {
                paragraph {
                    ta: text("a")
                    te: text("")
                    text("b")
                }
            }
        };
        let inp_down = Position {
            node_id: te,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let canonical_down = normalize_position(&d, inp_down);
        assert!(pos_eq(canonical_down, inp_down));

        let inp_up = Position {
            node_id: te,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let canonical_up = normalize_position(&d, inp_up);
        let exp_up = Position {
            node_id: ta,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        assert!(pos_eq(canonical_up, exp_up));
    }

    #[test]
    fn normalize_same_boundary_collapses_to_head() {
        let (state, ta, _) = state! {
            doc {
                root {
                    paragraph {
                        ta: text("Hello")
                        tb: text("World")
                    }
                }
            }
            selection: (ta, 5, <) -> (tb, 0, <)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(canonical.is_collapsed());
        assert_eq!(canonical.anchor.node_id, ta);
        assert_eq!(canonical.anchor.offset, 5);
        assert_eq!(canonical.anchor.affinity, Affinity::Upstream);
    }

    #[test]
    fn normalize_forward_selection_enforces_affinities_and_descents() {
        let (state, ta, tb) = state! {
            doc {
                root {
                    paragraph {
                        ta: text("Hello")
                        tb: text("World")
                    }
                }
            }
            selection: (ta, 0) -> (tb, 5)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(!canonical.is_collapsed());
        assert_eq!(canonical.anchor.node_id, ta);
        assert_eq!(canonical.anchor.offset, 0);
        assert_eq!(canonical.anchor.affinity, Affinity::Downstream);
        assert_eq!(canonical.head.node_id, tb);
        assert_eq!(canonical.head.offset, 5);
        assert_eq!(canonical.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn normalize_backward_selection_preserves_direction() {
        let (state, ta) = state! {
            doc { root { paragraph { ta: text("Hello") } } }
            selection: (ta, 5) -> (ta, 0)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert_eq!(canonical.anchor.node_id, ta);
        assert_eq!(canonical.anchor.offset, 5);
        assert_eq!(canonical.anchor.affinity, Affinity::Upstream);
        assert_eq!(canonical.head.node_id, ta);
        assert_eq!(canonical.head.offset, 0);
        assert_eq!(canonical.head.affinity, Affinity::Downstream);
    }

    #[test]
    fn normalize_subtree_violation_falls_back_to_collapsed_head() {
        let (state, _, ta) = state! {
            doc { root: root { paragraph { ta: text("Hello") } } }
            selection: (root, 0) -> (ta, 3)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(canonical.is_collapsed());
        assert_eq!(canonical.head.node_id, ta);
        assert_eq!(canonical.head.offset, 3);
        assert_eq!(canonical.head.affinity, Affinity::Downstream);
    }

    #[test]
    fn normalize_subtree_violation_preserves_original_head_affinity() {
        let (state, _, ta) = state! {
            doc { root: root { paragraph { ta: text("Hello") } } }
            selection: (root, 0) -> (ta, 3, <)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(canonical.is_collapsed());
        assert_eq!(canonical.head.node_id, ta);
        assert_eq!(canonical.head.offset, 3);
        assert_eq!(canonical.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn selection_normalize_success_matches_resolved_normalize() {
        let (state, ..) = state! {
            doc {
                root { paragraph {
                    ta: text("Hello")
                    tb: text("World")
                } }
            }
            selection: (ta, 5, <) -> (tb, 0, <)
        };
        let via_selection = state.selection.normalize(&state.doc);
        let via_resolved = state.selection.resolve(&state.doc).map(|r| r.normalize());
        assert_eq!(via_selection, via_resolved);
    }

    #[test]
    fn selection_normalize_unresolvable_returns_none() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 0)
        };
        let dead = Selection::new(
            Position::new(editor_model::NodeId::new(), 0),
            Position::new(editor_model::NodeId::new(), 0),
        );
        assert!(dead.normalize(&state.doc).is_none());
    }

    #[test]
    fn selection_normalize_invalid_text_offset_returns_none() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 0)
        };
        let bad = Selection::new(Position::new(t, 0), Position::new(t, 99));
        assert!(bad.normalize(&state.doc).is_none());
    }

    #[test]
    fn selection_normalize_invalid_container_offset_returns_none() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 0)
        };
        let root_id = editor_model::NodeId::ROOT;
        let bad = Selection::new(Position::new(root_id, 0), Position::new(root_id, 99));
        assert!(bad.normalize(&state.doc).is_none());
    }

    #[test]
    fn selection_normalize_non_text_leaf_endpoint_returns_none() {
        let (state, _, hb, ..) = state! {
            doc {
                root {
                    paragraph {
                        ta: text("a")
                        hb: hard_break
                        tb: text("b")
                    }
                }
            }
            selection: (ta, 1)
        };
        let bad = Selection::collapsed(Position::new(hb, 0));
        assert!(bad.normalize(&state.doc).is_none());
    }

    #[test]
    fn normalize_idempotent_text_text_adjacency() {
        let (state, ..) = state! {
            doc {
                root { paragraph {
                    ta: text("Hello")
                    tb: text("World")
                } }
            }
            selection: (ta, 5, <) -> (tb, 0, <)
        };
        let once = state.selection.normalize(&state.doc).unwrap();
        let twice = once.normalize(&state.doc).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_idempotent_forward_selection() {
        let (state, ..) = state! {
            doc {
                root { paragraph {
                    ta: text("Hello")
                    tb: text("World")
                } }
            }
            selection: (ta, 0) -> (tb, 5)
        };
        let once = state.selection.normalize(&state.doc).unwrap();
        let twice = once.normalize(&state.doc).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_idempotent_after_subtree_fallback() {
        let (state, ..) = state! {
            doc { root: root { paragraph { ta: text("Hello") } } }
            selection: (root, 0) -> (ta, 3)
        };
        let once = state.selection.normalize(&state.doc).unwrap();
        let twice = once.normalize(&state.doc).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_preserves_envelope_over_leading_block() {
        let (state, ..) = state! {
            doc {
                root: root {
                    fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("c") } }
                    }
                    paragraph { ta: text("after") }
                }
            }
            selection: (root, 0) -> (ta, 3)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(
            !canonical.is_collapsed(),
            "envelope over leading fold must survive normalize, got {:?}",
            canonical
        );
    }

    #[test]
    fn normalize_collapses_back_adjacent_slot() {
        let (state, root, _) = state! {
            doc {
                root: root {
                    paragraph { ta: text("Hello") }
                    paragraph { text("x") }
                }
            }
            selection: (ta, 5) -> (root, 1)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let canonical = resolved.normalize();
        assert!(
            canonical.is_collapsed(),
            "back-adjacent slot boundary must still collapse, got {:?}",
            canonical
        );
        assert_eq!(canonical.head.node_id, root);
        assert_eq!(canonical.head.offset, 1);
    }

    #[test]
    fn normalize_collapsed_before_atom_downstream_expands_to_node_selection() {
        let (d, ..) = doc! {
            root { paragraph { text("a") } image paragraph { text("b") } }
        };
        // (root, 1, Downstream) is the boundary just before the image; must expand to node selection.
        let root_id = NodeId::ROOT;
        let sel = Selection::collapsed(Position {
            node_id: root_id,
            offset: 1,
            affinity: Affinity::Downstream,
        });
        let out = sel.normalize(&d).unwrap();
        assert!(
            !out.is_collapsed(),
            "must expand to node selection, got {:?}",
            out
        );
        assert_eq!(
            out.anchor,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            out.head,
            Position {
                node_id: root_id,
                offset: 2,
                affinity: Affinity::Upstream
            }
        );
    }

    #[test]
    fn normalize_collapsed_after_atom_upstream_expands_backward_node_selection() {
        let (d, ..) = doc! {
            root { paragraph { text("a") } image paragraph { text("b") } }
        };
        let root_id = NodeId::ROOT;
        // (root, 2, Upstream) is the boundary just after the image; Upstream selects child[1]=image.
        let sel = Selection::collapsed(Position {
            node_id: root_id,
            offset: 2,
            affinity: Affinity::Upstream,
        });
        let out = sel.normalize(&d).unwrap();
        assert!(!out.is_collapsed(), "must expand, got {:?}", out);
        assert_eq!(
            out.anchor,
            Position {
                node_id: root_id,
                offset: 2,
                affinity: Affinity::Upstream
            }
        );
        assert_eq!(
            out.head,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Downstream
            }
        );
    }

    #[test]
    fn normalize_collapsed_between_adjacent_atoms_uses_affinity() {
        let (d, ..) = doc! {
            root { paragraph { text("x") } image horizontal_rule paragraph { text("y") } }
        };
        let root_id = NodeId::ROOT;
        // (root,2,Down) → child[2]=horizontal_rule.
        let down = Selection::collapsed(Position {
            node_id: root_id,
            offset: 2,
            affinity: Affinity::Downstream,
        })
        .normalize(&d)
        .unwrap();
        assert_eq!(
            down.anchor,
            Position {
                node_id: root_id,
                offset: 2,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            down.head,
            Position {
                node_id: root_id,
                offset: 3,
                affinity: Affinity::Upstream
            }
        );
        // (root,2,Up) → child[1]=image.
        let up = Selection::collapsed(Position {
            node_id: root_id,
            offset: 2,
            affinity: Affinity::Upstream,
        })
        .normalize(&d)
        .unwrap();
        assert_eq!(
            up.anchor,
            Position {
                node_id: root_id,
                offset: 2,
                affinity: Affinity::Upstream
            }
        );
        assert_eq!(
            up.head,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Downstream
            }
        );
    }

    #[test]
    fn normalize_collapsed_non_atom_container_position_unchanged() {
        let (d, ..) = doc! {
            root { paragraph { text("a") } paragraph { text("b") } }
        };
        let root_id = NodeId::ROOT;
        // Both sides of (root,1) are paragraphs (non-atom), so the selection must stay collapsed.
        for aff in [Affinity::Downstream, Affinity::Upstream] {
            let sel = Selection::collapsed(Position {
                node_id: root_id,
                offset: 1,
                affinity: aff,
            });
            let out = sel.normalize(&d).unwrap();
            assert!(
                out.is_collapsed(),
                "non-atom container pos must stay collapsed, got {:?}",
                out
            );
        }
    }

    #[test]
    fn normalize_atom_node_selection_idempotent() {
        let (d, ..) = doc! {
            root { paragraph { text("a") } image paragraph { text("b") } }
        };
        let root_id = NodeId::ROOT;
        let sel = Selection::collapsed(Position {
            node_id: root_id,
            offset: 1,
            affinity: Affinity::Downstream,
        });
        let once = sel.normalize(&d).unwrap();
        let twice = once.normalize(&d).unwrap();
        assert_eq!(
            once, twice,
            "atom node selection normalize must be idempotent"
        );
    }

    #[test]
    fn is_unit_node_selection_classifies_correctly() {
        let (d, r, t) = doc! {
            r: root {
                image
                paragraph { t: text("x") }
            }
        };
        let atom = Selection::new(
            Position {
                node_id: r,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: r,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        assert!(atom.is_unit_node_selection(&d));
        assert!(Selection::new(atom.head, atom.anchor).is_unit_node_selection(&d));
        assert!(!Selection::collapsed(Position::new(t, 0)).is_unit_node_selection(&d));
        assert!(
            !Selection::new(Position::new(t, 0), Position::new(t, 1)).is_unit_node_selection(&d)
        );
        assert!(
            !Selection::new(
                Position {
                    node_id: r,
                    offset: 1,
                    affinity: Affinity::Downstream
                },
                Position {
                    node_id: r,
                    offset: 2,
                    affinity: Affinity::Upstream
                },
            )
            .is_unit_node_selection(&d)
        );
    }

    #[test]
    fn normalize_collapsed_before_leading_atom_index0_expands() {
        // In a leading-atom doc, first_cursor_position yields (root,0) collapsed;
        // normalize must expand it to a node selection.
        let (d, ..) = doc! {
            root { image paragraph { text("b") } }
        };
        let root_id = NodeId::ROOT;
        let sel = Selection::collapsed(Position {
            node_id: root_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
        let out = sel.normalize(&d).unwrap();
        assert!(
            !out.is_collapsed(),
            "leading atom must node-select, got {:?}",
            out
        );
        assert_eq!(
            out.anchor,
            Position {
                node_id: root_id,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            out.head,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
    }

    #[test]
    fn is_unit_node_selection_true_for_monolithic() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            }
        };
        let root_id = NodeId::ROOT;
        let fold_sel = Selection::new(
            Position {
                node_id: root_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        assert!(fold_sel.is_unit_node_selection(&d));
        assert!(Selection::new(fold_sel.head, fold_sel.anchor).is_unit_node_selection(&d));
    }

    #[test]
    fn expand_unit_at_expands_caret_adjacent_to_monolithic() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph { text("b") }
            }
        };
        let root_id = NodeId::ROOT;
        let sel = Selection::collapsed(Position {
            node_id: root_id,
            offset: 0,
            affinity: Affinity::Downstream,
        });
        let out = sel.normalize(&d).unwrap();
        assert!(
            !out.is_collapsed(),
            "caret bracketing a leading fold must node-select it, got {:?}",
            out
        );
        assert_eq!(
            out.anchor,
            Position {
                node_id: root_id,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            out.head,
            Position {
                node_id: root_id,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
    }
}

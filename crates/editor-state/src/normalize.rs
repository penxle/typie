//! Selection / Position normalization.
//!
//! Maps the many `Position` / `Selection` representations that point at the
//! same caret or range to a single canonical form. The canonical form prefers
//! the text-leaf representation over the textblock-container representation
//! and uses `Affinity` to choose between adjacent leaves on either side of a
//! boundary; when neither side is a text leaf, the container representation
//! is kept as-is.

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
    a_node != h_node && (a_node.starts_with(h_node) || h_node.starts_with(a_node))
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
            return Selection::collapsed(normalize_position(doc, h_in));
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
            return Selection::collapsed(normalize_position(doc, h_in));
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
}

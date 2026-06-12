use editor_macros::ffi;
use editor_model::Doc;
use serde::{Deserialize, Serialize};

use crate::normalize::enclosing_unit_at_subtree_overlap;
use crate::selection::Selection;
use crate::stable_position::{StablePosition, restore_position};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StableSelection {
    pub anchor: StablePosition,
    pub head: StablePosition,
}

impl StableSelection {
    pub fn capture(sel: &Selection, doc: &Doc) -> Self {
        let anchor = StablePosition::capture(sel.anchor, doc);
        let head = if sel.is_collapsed() {
            anchor.clone()
        } else {
            StablePosition::capture(sel.head, doc)
        };
        StableSelection { anchor, head }
    }

    /// Returns whether both stored endpoints still preserve their stable
    /// identity in `doc`. Fallback-only restoration is intentionally rejected.
    pub fn is_preserved(&self, doc: &Doc) -> bool {
        self.anchor.is_preserved(doc) && (self.anchor == self.head || self.head.is_preserved(doc))
    }

    pub fn restore(&self, doc: &Doc) -> Selection {
        let was_collapsed = self.anchor == self.head;
        let a = restore_position(&self.anchor, doc);
        if was_collapsed {
            return Selection { anchor: a, head: a };
        }
        let h = restore_position(&self.head, doc);
        let candidate = Selection { anchor: a, head: h };
        if invariants_ok(&candidate, doc) {
            candidate
        } else if let Some(sel) = enclosing_unit_at_subtree_overlap(doc, candidate.anchor, h) {
            sel
        } else {
            Selection::collapsed(h)
        }
    }
}

fn invariants_ok(sel: &Selection, doc: &Doc) -> bool {
    let Some(ra) = sel.anchor.resolve(doc) else {
        return false;
    };
    let Some(rh) = sel.head.resolve(doc) else {
        return false;
    };
    if ra.node_id() != rh.node_id() {
        let pa = ra.path();
        let ph = rh.path();
        if pa.starts_with(ph) || ph.starts_with(pa) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::{Affinity, Bind};
    use editor_macros::state;
    use editor_model::NodeId;

    use super::*;

    #[test]
    fn collapsed_roundtrip_yields_equal_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let sel = state.selection.unwrap();
        let stable = StableSelection::capture(&sel, &state.doc);
        let back = stable.restore(&state.doc);
        assert_eq!(back, sel);
        assert!(back.is_collapsed());
        assert_eq!(back.anchor, back.head);
    }

    #[test]
    fn is_preserved_returns_true_when_cursor_endpoint_is_live() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let sel = state.selection.unwrap();
        let stable = StableSelection::capture(&sel, &state.doc);

        assert!(stable.is_preserved(&state.doc));
        assert_eq!(stable.restore(&state.doc), sel);
    }

    #[test]
    fn non_collapsed_roundtrip_preserves_endpoints() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = StableSelection::capture(&sel, &state.doc);
        let back = stable.restore(&state.doc);
        assert_eq!(back, sel);
    }

    #[test]
    fn invariant_violation_collapses_to_head() {
        let (state, p1, _, t2) = state! {
            doc {
                root {
                    p1: paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
            }
            selection: (t1, 2) -> (t2, 3)
        };
        let stable = StableSelection::capture(state.selection.as_ref().unwrap(), &state.doc);

        let dead_state = remove_paragraph(&state, p1);
        let back = stable.restore(&dead_state.doc);
        assert!(back.is_collapsed());
        assert_eq!(back.head.node_id, t2);
    }

    #[test]
    fn is_preserved_rejects_missing_child_fallback() {
        use crate::stable_position::ChainLink;
        use editor_crdt::Dot;

        let (state, _p1, p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                }
            }
            selection: none
        };
        let root_chain = vec![ChainLink {
            node_id: NodeId::ROOT,
            child_dot: Dot::new(0, 0),
        }];
        let missing_anchor = StablePosition::Child {
            chain: root_chain.clone(),
            child_dot: Dot::new(999, 999),
            offset: 0,
            bind: Bind::Right,
            affinity: Affinity::Downstream,
        };
        let p2_dot = state
            .doc
            .get_entry(NodeId::ROOT)
            .unwrap()
            .children
            .dot_for(&p2)
            .unwrap();
        let live_head = StablePosition::Child {
            chain: root_chain,
            child_dot: p2_dot,
            offset: 1,
            bind: Bind::Right,
            affinity: Affinity::Downstream,
        };
        let stable = StableSelection {
            anchor: missing_anchor,
            head: live_head,
        };

        assert!(stable.restore(&state.doc).resolve(&state.doc).is_some());
        assert!(!stable.is_preserved(&state.doc));
    }

    fn remove_paragraph(state: &crate::State, p_id: NodeId) -> crate::State {
        use editor_crdt::{OrMapOp, RgaOp};
        use editor_model::DocOp;

        let root_id = NodeId::ROOT;
        let node_dot = state
            .doc
            .get_entry(root_id)
            .unwrap()
            .children
            .iter_with_dot()
            .find(|(_, v)| **v == p_id)
            .map(|(d, _)| d)
            .expect("paragraph must be a root child");

        let mut to_remove: Vec<NodeId> = Vec::new();
        let mut stack = vec![p_id];
        while let Some(id) = stack.pop() {
            to_remove.push(id);
            if let Some(entry) = state.doc.get_entry(id) {
                for child in entry.children.iter() {
                    stack.push(*child);
                }
            }
        }

        let (next, _ops) = state
            .batch_with_ops::<_, crate::StateError>(|b| {
                // Tombstone descendants before unlinking the subtree root, or
                // Doc::verify's reachability check rejects the resulting doc.
                for id in to_remove.iter().rev() {
                    let presence_dots: Vec<_> = b.doc.nodes_tags_for(id).copied().collect();
                    b.apply(DocOp::Presence {
                        node_id: *id,
                        op: OrMapOp::Unset {
                            observed: presence_dots,
                        },
                    })?;
                }
                b.apply(DocOp::Children {
                    node_id: root_id,
                    op: RgaOp::Remove { observed: node_dot },
                })?;
                Ok(())
            })
            .unwrap();
        next
    }
}

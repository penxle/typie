use editor_macros::ffi;
use editor_model::Doc;
use serde::{Deserialize, Serialize};

use crate::normalize::enclosing_unit_at_subtree_overlap;
use crate::selection::Selection;
use crate::stable_position::{StablePosition, freeze_position, thaw_position};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StableSelection {
    pub anchor: StablePosition,
    pub head: StablePosition,
}

impl StableSelection {
    pub fn freeze(sel: &Selection, doc: &Doc) -> Self {
        let anchor = freeze_position(sel.anchor, doc);
        let head = if sel.is_collapsed() {
            anchor.clone()
        } else {
            freeze_position(sel.head, doc)
        };
        StableSelection { anchor, head }
    }

    /// Thaws and normalizes, returning `Some` only when the range still locates
    /// to a real (non-empty) span. Returns `None` when the covered text was
    /// deleted.
    pub fn locate(&self, doc: &Doc) -> Option<Selection> {
        let sel = self.thaw(doc);
        let sel = sel.normalize(doc).unwrap_or(sel);
        (!sel.is_collapsed()).then_some(sel)
    }

    pub fn thaw(&self, doc: &Doc) -> Selection {
        let was_collapsed = self.anchor == self.head;
        let a = thaw_position(&self.anchor, doc);
        if was_collapsed {
            return Selection { anchor: a, head: a };
        }
        let h = thaw_position(&self.head, doc);
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
        let stable = StableSelection::freeze(&sel, &state.doc);
        let back = stable.thaw(&state.doc);
        assert_eq!(back, sel);
        assert!(back.is_collapsed());
        assert_eq!(back.anchor, back.head);
    }

    #[test]
    fn non_collapsed_roundtrip_preserves_endpoints() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let sel = state.selection.unwrap();
        let stable = StableSelection::freeze(&sel, &state.doc);
        let back = stable.thaw(&state.doc);
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
        let stable = StableSelection::freeze(state.selection.as_ref().unwrap(), &state.doc);

        let dead_state = remove_paragraph(&state, p1);
        let back = stable.thaw(&dead_state.doc);
        assert!(back.is_collapsed());
        assert_eq!(back.head.node_id, t2);
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

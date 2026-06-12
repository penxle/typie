use editor_macros::ffi;
use editor_model::{Doc, Node, NodeId};
use serde::{Deserialize, Serialize};

use crate::normalize::enclosing_unit_at_subtree_overlap;
use crate::position::Position;
use crate::selection::Selection;
use crate::stable_position::{StablePosition, build_chain, freeze_position, thaw_position};
use crate::{Affinity, Bind};

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

    pub fn freeze_covered_range(sel: &Selection, doc: &Doc) -> Option<Self> {
        let resolved = sel.resolve(doc)?;
        if resolved.is_collapsed() {
            return None;
        }

        let start = Position::from(resolved.from());
        let end = Position::from(resolved.to());
        let covered_entries = covered_entry_endpoints(&resolved);

        let anchor = match covered_entries.first {
            Some(endpoint) => freeze_text_endpoint(
                endpoint.node_id,
                endpoint.entry_dot,
                endpoint.boundary_offset,
                Bind::Left,
                start.affinity,
                doc,
            ),
            None => freeze_position(start, doc),
        };
        let head = match covered_entries.last {
            Some(endpoint) => freeze_text_endpoint(
                endpoint.node_id,
                endpoint.entry_dot,
                endpoint.boundary_offset,
                Bind::Right,
                end.affinity,
                doc,
            ),
            None => freeze_position(end, doc),
        };

        Some(StableSelection { anchor, head })
    }

    /// Thaws and normalizes, returning `Some` only when the selection locates
    /// to a non-empty span.
    // TODO: cursor thaw와 range locate를 분리한다.
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

#[derive(Clone, Copy)]
struct CoveredEntryEndpoint {
    node_id: NodeId,
    boundary_offset: usize,
    entry_dot: editor_crdt::EntryDot,
}

#[derive(Default)]
struct CoveredEntryEndpoints {
    first: Option<CoveredEntryEndpoint>,
    last: Option<CoveredEntryEndpoint>,
}

fn covered_entry_endpoints(resolved: &crate::ResolvedSelection<'_>) -> CoveredEntryEndpoints {
    let mut endpoints = CoveredEntryEndpoints::default();
    resolved.for_each_text_node(|node, span| {
        let Node::Text(text_node) = node.node() else {
            return;
        };
        let first_index = span.start;
        let last_index = span.end - 1;
        let first_entry_dot = text_node
            .text
            .entry_dot_at(first_index)
            .expect("covered span start must be in text bounds");
        let last_entry_dot = text_node
            .text
            .entry_dot_at(last_index)
            .expect("covered span end must be in text bounds");

        endpoints.first.get_or_insert(CoveredEntryEndpoint {
            node_id: node.id(),
            boundary_offset: span.start,
            entry_dot: first_entry_dot,
        });
        endpoints.last = Some(CoveredEntryEndpoint {
            node_id: node.id(),
            boundary_offset: span.end,
            entry_dot: last_entry_dot,
        });
    });
    endpoints
}

fn freeze_text_endpoint(
    node_id: NodeId,
    entry_dot: editor_crdt::EntryDot,
    boundary_offset: usize,
    bind: Bind,
    affinity: Affinity,
    doc: &Doc,
) -> StablePosition {
    StablePosition::Char {
        chain: build_chain(node_id, doc),
        char_dot: entry_dot.0,
        offset: boundary_offset,
        bind,
        affinity,
    }
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

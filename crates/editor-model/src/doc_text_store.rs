use editor_crdt::{CrdtError, Dot, EntryDot, PlacementId, Rga, RgaOp, TextPlacement};

#[cfg(any(test, debug_assertions))]
use crate::doc::Doc;
use crate::id::NodeId;
use crate::text_entry_store::TextEntryStore;
use crate::text_index::{CurrentTextLocation, TextIndex};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextPlacementRecord {
    entry_dot: EntryDot,
    ch: char,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredTextPlacement {
    placement: TextPlacement,
    alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryBoundary {
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct DocTextStore {
    entries: TextEntryStore,
    index: TextIndex,
    placements_by_node: imbl::HashMap<NodeId, Rga<TextPlacementRecord>>,
}

impl DocTextStore {
    pub(crate) fn register_insert(
        &mut self,
        entry_dot: EntryDot,
        birth_node: NodeId,
        initial_placement: PlacementId,
        after: Option<PlacementId>,
        ch: char,
    ) -> Result<(), CrdtError> {
        self.insert_placement_record(birth_node, initial_placement, entry_dot, after, ch)?;
        self.entries
            .register_insert(entry_dot, birth_node, initial_placement, ch);
        Ok(())
    }

    pub(crate) fn insert_placement(
        &mut self,
        owner_text_node: NodeId,
        placement: PlacementId,
        entry_dot: EntryDot,
        after: Option<PlacementId>,
        ch: char,
    ) -> Result<(), CrdtError> {
        self.insert_placement_record(owner_text_node, placement, entry_dot, after, ch)
    }

    pub(crate) fn mark_deleted(&mut self, entry_dot: EntryDot, remove_dot: Dot) {
        self.entries.mark_deleted(entry_dot, remove_dot);
        self.index.clear_moved_location(entry_dot);
    }

    pub(crate) fn record_current_location(
        &mut self,
        entry_dot: EntryDot,
        current: CurrentTextLocation,
    ) {
        if self.entries.is_deleted(entry_dot) {
            self.index.clear_moved_location(entry_dot);
            return;
        }
        self.index.set_current_location(
            entry_dot,
            self.entries.birth_location_for(entry_dot),
            current,
        );
    }

    pub(crate) fn current_location(&self, entry_dot: EntryDot) -> Option<CurrentTextLocation> {
        if self.entries.is_deleted(entry_dot) {
            return None;
        }
        self.index.moved_location(entry_dot).or_else(|| {
            let (owner_text_node, placement) = self.entries.birth_location_for(entry_dot)?;
            Some(CurrentTextLocation {
                owner_text_node,
                placement,
            })
        })
    }

    pub(crate) fn is_deleted(&self, entry_dot: EntryDot) -> bool {
        self.entries.is_deleted(entry_dot)
    }

    pub(crate) fn char_for(&self, entry_dot: EntryDot) -> Option<char> {
        self.entries.char_for(entry_dot)
    }

    pub(crate) fn contains_placement(&self, node_id: NodeId, placement: PlacementId) -> bool {
        self.placements_by_node
            .get(&node_id)
            .is_some_and(|placements| placements.contains_dot(placement.0))
    }

    pub(crate) fn visible_offset_before_entry(
        &self,
        entry_dot: EntryDot,
    ) -> Option<(NodeId, usize)> {
        self.visible_offset_near_entry(entry_dot, EntryBoundary::Before)
    }

    pub(crate) fn visible_offset_after_entry(
        &self,
        entry_dot: EntryDot,
    ) -> Option<(NodeId, usize)> {
        self.visible_offset_near_entry(entry_dot, EntryBoundary::After)
    }

    #[cfg(any(test, debug_assertions))]
    pub(crate) fn all_placements_for_node(&self, node_id: NodeId) -> Vec<TextPlacement> {
        self.stored_placements_for_node(node_id)
            .into_iter()
            .map(|stored| stored.placement)
            .collect()
    }

    fn stored_placements_for_node(&self, node_id: NodeId) -> Vec<StoredTextPlacement> {
        self.placements_by_node
            .get(&node_id)
            .into_iter()
            .flat_map(|placements| {
                placements
                    .iter_all_in_order()
                    .map(|(placement_id, entry, alive)| StoredTextPlacement {
                        placement: TextPlacement {
                            placement_id: PlacementId(placement_id),
                            entry_dot: entry.entry_dot,
                            ch: entry.ch,
                        },
                        alive,
                    })
            })
            .collect()
    }

    pub(crate) fn visible_placements_for_node(&self, node_id: NodeId) -> Vec<TextPlacement> {
        self.stored_placements_for_node(node_id)
            .into_iter()
            .filter_map(|stored| {
                self.stored_placement_is_visible(node_id, stored)
                    .then_some(stored.placement)
            })
            .collect()
    }

    fn stored_placement_is_visible(&self, node_id: NodeId, stored: StoredTextPlacement) -> bool {
        let placement = stored.placement;
        stored.alive
            && !self.is_deleted(placement.entry_dot)
            && self
                .current_location(placement.entry_dot)
                .is_some_and(|current| {
                    current.owner_text_node == node_id
                        && current.placement == placement.placement_id
                })
    }

    fn latest_placement_for_entry(
        &self,
        entry_dot: EntryDot,
        horizon: Option<PlacementId>,
    ) -> Option<(NodeId, PlacementId)> {
        self.placements_by_node
            .iter()
            .flat_map(|(node_id, placements)| {
                placements
                    .iter_all_in_order()
                    .map(move |(placement_id, record, _)| {
                        (*node_id, PlacementId(placement_id), record.entry_dot)
                    })
            })
            .filter(|(_, _, candidate)| *candidate == entry_dot)
            .filter(|(_, placement_id, _)| {
                horizon.is_none_or(|horizon| placement_id.0 <= horizon.0)
            })
            .max_by_key(|(_, placement_id, _)| placement_id.0)
            .map(|(node_id, placement_id, _)| (node_id, placement_id))
    }

    fn visible_offset_near_entry(
        &self,
        entry_dot: EntryDot,
        boundary: EntryBoundary,
    ) -> Option<(NodeId, usize)> {
        let horizon = self.entries.delete_horizon_for(entry_dot).map(PlacementId);
        let (owner, target_placement) = self.latest_placement_for_entry(entry_dot, horizon)?;
        let placements = self.placements_by_node.get(&owner)?;

        let mut passed_target = false;
        let mut visible_offset = 0;
        for (placement_id, record, alive) in placements.iter_all_in_order() {
            let placement = TextPlacement {
                placement_id: PlacementId(placement_id),
                entry_dot: record.entry_dot,
                ch: record.ch,
            };

            if placement.placement_id == target_placement {
                match boundary {
                    EntryBoundary::Before => {
                        return Some((owner, visible_offset));
                    }
                    EntryBoundary::After => {
                        passed_target = true;
                        continue;
                    }
                }
            }

            // A deleted entry's stable boundary is tied to the target placement's
            // historical position. Later fresh inserts are not revivals of that
            // EntryDot, even if RGA sibling ordering places them before the
            // tombstoned placement.
            let within_horizon =
                horizon.is_none_or(|horizon| placement.placement_id.0 <= horizon.0);
            if within_horizon
                && self.stored_placement_is_visible(owner, StoredTextPlacement { placement, alive })
            {
                if passed_target {
                    return Some((owner, visible_offset));
                }
                visible_offset += 1;
            }
        }

        passed_target.then_some((owner, visible_offset))
    }

    fn insert_placement_record(
        &mut self,
        node_id: NodeId,
        placement: PlacementId,
        entry_dot: EntryDot,
        after: Option<PlacementId>,
        ch: char,
    ) -> Result<(), CrdtError> {
        let placements = self
            .placements_by_node
            .get(&node_id)
            .cloned()
            .unwrap_or_default();
        let next = placements.apply(
            placement.0,
            RgaOp::Insert {
                after: after.map(Into::into),
                value: TextPlacementRecord { entry_dot, ch },
            },
        )?;
        self.placements_by_node.insert(node_id, next);
        Ok(())
    }

    #[cfg(any(test, debug_assertions))]
    pub(crate) fn birth_location_for(&self, entry_dot: EntryDot) -> Option<(NodeId, PlacementId)> {
        self.entries.birth_location_for(entry_dot)
    }

    #[cfg(test)]
    pub(crate) fn moved_location(&self, entry_dot: EntryDot) -> Option<CurrentTextLocation> {
        self.index.moved_location(entry_dot)
    }

    #[cfg(any(test, debug_assertions))]
    pub(crate) fn index_matches_rebuild(&self, doc: &Doc) -> bool {
        self.index == TextIndex::rebuild(doc)
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{EntryDot, LwwRegOp, OpGraph, OrMapOp, RgaOp, TextOp};

    use crate::{Doc, DocOp, ModelError, NodeId, NodeType, apply_doc_op};

    fn apply(
        graph: &mut OpGraph<DocOp>,
        doc: Doc,
        payload: DocOp,
    ) -> (Doc, editor_crdt::Op<DocOp>) {
        let (next_graph, op) = graph.clone().add(payload).unwrap();
        *graph = next_graph;
        let doc = apply_doc_op(doc, &op).unwrap();
        (doc, op)
    }

    #[test]
    fn verify_rejects_stale_text_index() {
        let mut graph = OpGraph::<DocOp>::new();
        let root = NodeId::ROOT;
        let t1 = NodeId::new();
        let t2 = NodeId::new();

        let (doc, _) = apply(
            &mut graph,
            Doc::empty(),
            DocOp::Presence {
                node_id: root,
                op: OrMapOp::Set {
                    key: root,
                    value: NodeType::Root,
                },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Presence {
                node_id: t1,
                op: OrMapOp::Set {
                    key: t1,
                    value: NodeType::Text,
                },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Presence {
                node_id: t2,
                op: OrMapOp::Set {
                    key: t2,
                    value: NodeType::Text,
                },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Parent {
                node_id: t1,
                op: LwwRegOp::Set { value: Some(root) },
            },
        );
        let (doc, t1_child) = apply(
            &mut graph,
            doc,
            DocOp::Children {
                node_id: root,
                op: RgaOp::Insert {
                    after: None,
                    value: t1,
                },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Parent {
                node_id: t2,
                op: LwwRegOp::Set { value: Some(root) },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Children {
                node_id: root,
                op: RgaOp::Insert {
                    after: Some(t1_child.id),
                    value: t2,
                },
            },
        );
        let (doc, insert) = apply(
            &mut graph,
            doc,
            DocOp::Text {
                node_id: t1,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            },
        );
        let (mut doc, _) = apply(
            &mut graph,
            doc,
            DocOp::MoveText {
                entry: EntryDot(insert.id),
                to_node_id: t2,
                after: None,
            },
        );

        doc.text.index.clear_moved_location(EntryDot(insert.id));

        assert_eq!(doc.verify(), Err(ModelError::TextIndexDesync));
    }
}

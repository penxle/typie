use editor_crdt::{EntryDot, PlacementId};

#[cfg(any(test, debug_assertions))]
use crate::doc::Doc;
use crate::id::NodeId;
#[cfg(any(test, debug_assertions))]
use crate::nodes::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CurrentTextLocation {
    pub(crate) owner_text_node: NodeId,
    pub(crate) placement: PlacementId,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct TextIndex {
    moved_locations: imbl::HashMap<EntryDot, CurrentTextLocation>,
}

impl TextIndex {
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn rebuild(doc: &Doc) -> Self {
        let mut current_locations = imbl::HashMap::new();
        for (node_id, entry) in doc.entries.iter() {
            let Node::Text(_) = &entry.node else {
                continue;
            };
            for placement in doc.text.all_placements_for_node(*node_id) {
                if doc.text.is_deleted(placement.entry_dot) {
                    continue;
                }
                let candidate = CurrentTextLocation {
                    owner_text_node: *node_id,
                    placement: placement.placement_id,
                };
                let should_replace = current_locations.get(&placement.entry_dot).is_none_or(
                    |current: &CurrentTextLocation| candidate.placement.0 > current.placement.0,
                );
                if should_replace {
                    current_locations.insert(placement.entry_dot, candidate);
                }
            }
        }

        let mut moved_locations = imbl::HashMap::new();
        for (entry_dot, current) in current_locations {
            let Some((birth_node, initial_placement)) = doc.text.birth_location_for(entry_dot)
            else {
                continue;
            };
            if current.owner_text_node != birth_node || current.placement != initial_placement {
                moved_locations.insert(entry_dot, current);
            }
        }

        Self { moved_locations }
    }

    pub(crate) fn moved_location(&self, entry_dot: EntryDot) -> Option<CurrentTextLocation> {
        self.moved_locations.get(&entry_dot).copied()
    }

    pub(crate) fn set_current_location(
        &mut self,
        entry_dot: EntryDot,
        birth_location: Option<(NodeId, PlacementId)>,
        current: CurrentTextLocation,
    ) {
        let Some((birth_node, initial_placement)) = birth_location else {
            return;
        };

        if current.owner_text_node == birth_node && current.placement == initial_placement {
            self.clear_moved_location(entry_dot);
        } else {
            self.moved_locations.insert(entry_dot, current);
        }
    }

    pub(crate) fn clear_moved_location(&mut self, entry_dot: EntryDot) {
        self.moved_locations.remove(&entry_dot);
    }
}

use editor_crdt::{EntryDot, PlacementId, TextPlacement};

use crate::doc::Doc;
use crate::id::NodeId;
use crate::nodes::{Node, TextNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextEntryLocation {
    pub node_id: NodeId,
    pub placement_id: PlacementId,
}

#[derive(Clone, Copy, Debug)]
pub struct TextView<'a> {
    id: NodeId,
    node: &'a TextNode,
}

impl<'a> TextView<'a> {
    pub(crate) fn new(id: NodeId, node: &'a TextNode) -> Self {
        Self { id, node }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn node(&self) -> &'a TextNode {
        self.node
    }

    pub fn text(&self) -> String {
        self.node.text.to_string()
    }

    pub fn len(&self) -> usize {
        self.node.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.node.text.is_empty()
    }

    pub fn visible_entries(&self) -> impl Iterator<Item = (EntryDot, char)> + '_ {
        self.node.text.iter_visible_entries()
    }

    pub fn visible_placements(&self) -> impl Iterator<Item = TextPlacement> + '_ {
        self.node.text.iter_visible_placements()
    }

    pub fn last_visible_placement(&self) -> Option<TextPlacement> {
        self.visible_placements().last()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextIdentityView<'a> {
    doc: &'a Doc,
}

impl<'a> TextIdentityView<'a> {
    pub(crate) fn new(doc: &'a Doc) -> Self {
        Self { doc }
    }

    pub fn current_location(&self, entry_dot: EntryDot) -> Option<TextEntryLocation> {
        self.doc
            .text
            .current_location(entry_dot)
            .map(|location| TextEntryLocation {
                node_id: location.owner_text_node,
                placement_id: location.placement,
            })
    }

    pub fn is_alive(&self, entry_dot: EntryDot) -> bool {
        self.current_location(entry_dot).is_some_and(|location| {
            self.visible_rank_of_placement(location.node_id, location.placement_id)
                .is_some()
        })
    }

    pub fn is_deleted(&self, entry_dot: EntryDot) -> bool {
        self.doc.text.is_deleted(entry_dot)
    }

    pub fn visible_rank_of_placement(
        &self,
        node_id: NodeId,
        placement_id: PlacementId,
    ) -> Option<usize> {
        let entry = self.doc.get_entry(node_id)?;
        let Node::Text(text_node) = &entry.node else {
            return None;
        };
        text_node.text.visible_rank_of_placement(placement_id)
    }

    pub fn visible_offset_before_deleted_entry(
        &self,
        entry_dot: EntryDot,
    ) -> Option<(NodeId, usize)> {
        let (node_id, offset) = self.doc.text.visible_offset_before_entry(entry_dot)?;
        self.live_text_offset(node_id, offset)
    }

    pub fn visible_offset_after_deleted_entry(
        &self,
        entry_dot: EntryDot,
    ) -> Option<(NodeId, usize)> {
        let (node_id, offset) = self.doc.text.visible_offset_after_entry(entry_dot)?;
        self.live_text_offset(node_id, offset)
    }

    pub fn char_for(&self, entry_dot: EntryDot) -> Option<char> {
        self.doc.text.char_for(entry_dot)
    }

    fn live_text_offset(&self, node_id: NodeId, offset: usize) -> Option<(NodeId, usize)> {
        let entry = self.doc.get_entry(node_id)?;
        let Node::Text(text_node) = &entry.node else {
            return None;
        };
        Some((node_id, offset.min(text_node.text.len())))
    }
}

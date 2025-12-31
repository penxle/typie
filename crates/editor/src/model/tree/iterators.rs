use crate::model::tree::Doc;
use crate::model::{Mark, NodeId, NodeType};
use crate::state::BlockTraverser;

pub struct BlockTextIterator<'a> {
    doc: &'a Doc,
    traverser: BlockTraverser<'a>,
}

impl<'a> BlockTextIterator<'a> {
    pub fn new(doc: &'a Doc) -> Self {
        let traverser = BlockTraverser::new(doc, NodeId::ROOT)
            .unwrap_or_else(|_| BlockTraverser::new(doc, NodeId::ROOT).unwrap());

        Self { doc, traverser }
    }
}

impl<'a> Iterator for BlockTextIterator<'a> {
    type Item = (NodeId, String);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(block_id) = self.traverser.next() {
            if self.doc.get_node_type(block_id) != Some(NodeType::Paragraph) {
                continue;
            }
            let text = self.doc.get_block_text(block_id);
            return Some((block_id, text));
        }
        None
    }
}

pub struct TextSegmentIterator<'a> {
    doc: &'a Doc,
    block_iter: BlockTextIterator<'a>,
    current_block_id: Option<NodeId>,
    current_child_ids: std::vec::IntoIter<NodeId>,
    current_segments: std::vec::IntoIter<(String, Vec<Mark>)>,
    current_offset: usize,
}

impl<'a> TextSegmentIterator<'a> {
    pub fn new(doc: &'a Doc) -> Self {
        Self {
            doc,
            block_iter: BlockTextIterator::new(doc),
            current_block_id: None,
            current_child_ids: Vec::new().into_iter(),
            current_segments: Vec::new().into_iter(),
            current_offset: 0,
        }
    }

    fn advance_block(&mut self) -> bool {
        while let Some((block_id, _)) = self.block_iter.next() {
            self.current_block_id = Some(block_id);
            self.current_child_ids = self.doc.get_children_ids(block_id).into_iter();
            self.current_offset = 0;
            return true;
        }
        false
    }
}

impl<'a> Iterator for TextSegmentIterator<'a> {
    type Item = (NodeId, usize, String, Vec<Mark>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((text, marks)) = self.current_segments.next() {
                let len = text.chars().count();
                let params = (
                    self.current_block_id.unwrap(),
                    self.current_offset,
                    text,
                    marks,
                );
                self.current_offset += len;
                return Some(params);
            }

            if let Some(child_id) = self.current_child_ids.next() {
                if self.doc.get_node_type(child_id) != Some(NodeType::Text) {
                    continue;
                }

                if let Some(segments) = self.doc.get_text_segments(child_id) {
                    self.current_segments = segments.into_iter();
                    continue;
                }
                continue;
            }

            if !self.advance_block() {
                return None;
            }
        }
    }
}

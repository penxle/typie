use crate::model::*;
use crate::runtime::Effect;
use crate::state::{Position, block_content_len, calculate_block_offsets, collect_blocks_in_range};
use crate::transaction::Transaction;
use anyhow::{Context, Result};

impl Transaction {
    pub fn add_annotation(&mut self, annotation: Annotation) -> Result<AnnotationId> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            anyhow::bail!("Cannot add annotation to collapsed selection");
        }

        let ann_type = annotation.as_type();
        let spec = self.doc().schema().annotation_spec(ann_type);
        if !spec.overlap {
            let (from, to) = selection.as_sorted(self.doc())?;
            if self.selection_has_annotation_type(from.clone(), to.clone(), ann_type)? {
                anyhow::bail!(
                    "Overlapping annotations of type {:?} are not allowed",
                    ann_type
                );
            }
        }

        let id = AnnotationId::new();

        self.doc().set_annotation(id, &annotation)?;

        let (from, to) = selection.as_sorted(self.doc())?;
        let ranges = collect_text_ranges_in_selection(self, from, to)?;
        for (text_node_id, start_offset, end_offset) in ranges {
            let allowed = self.doc().allowed_annotations_for(text_node_id);
            anyhow::ensure!(
                allowed.contains(&ann_type),
                "Annotation '{:?}' not allowed at node {}",
                ann_type,
                text_node_id,
            );

            let node = self.node_mut(text_node_id).context("Text node not found")?;
            if let Node::Text(text_node) = node.node() {
                text_node
                    .text
                    .apply_annotation(start_offset..end_offset, id)?;
                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }
        }

        Ok(id)
    }

    pub fn update_annotation(&mut self, id: AnnotationId, annotation: Annotation) -> Result<bool> {
        self.doc().set_annotation(id, &annotation)?;
        Ok(true)
    }

    pub fn remove_annotation(&mut self, id: AnnotationId) -> Result<bool> {
        let ranges = self.find_annotation_ranges(id);
        if ranges.is_empty() {
            return Ok(false);
        }

        for (text_node_id, start_offset, end_offset) in ranges {
            let node = self.node_mut(text_node_id).context("Text node not found")?;
            if let Node::Text(text_node) = node.node() {
                text_node
                    .text
                    .remove_annotation(start_offset..end_offset, id)?;
                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }
        }

        Ok(true)
    }

    fn find_annotation_ranges(&self, id: AnnotationId) -> Vec<(NodeId, usize, usize)> {
        let mut result = Vec::new();

        let root = match self.node(NodeId::ROOT) {
            Some(r) => r,
            None => return result,
        };

        for block in root.children() {
            self.find_annotation_in_block(block.node_id(), id, &mut result);
        }

        result
    }

    fn find_annotation_in_block(
        &self,
        block_id: NodeId,
        id: AnnotationId,
        result: &mut Vec<(NodeId, usize, usize)>,
    ) {
        let Some(block) = self.node(block_id) else {
            return;
        };

        for child in block.children() {
            match child.node() {
                Node::Text(text_node) => {
                    let segments = text_node.text.get_segments();
                    let mut current_offset = 0;
                    let mut range_start: Option<usize> = None;

                    for segment in &segments {
                        let segment_len = segment.text.chars().count();
                        let has_annotation = segment.annotations.contains(&id);

                        if has_annotation && range_start.is_none() {
                            range_start = Some(current_offset);
                        } else if !has_annotation && range_start.is_some() {
                            result.push((child.node_id(), range_start.unwrap(), current_offset));
                            range_start = None;
                        }
                        current_offset += segment_len;
                    }

                    if let Some(start) = range_start {
                        result.push((child.node_id(), start, current_offset));
                    }
                }
                _ => {
                    if !child.spec().inline {
                        self.find_annotation_in_block(child.node_id(), id, result);
                    }
                }
            }
        }
    }

    fn selection_has_annotation_type(
        &self,
        from: Position,
        to: Position,
        ann_type: AnnotationType,
    ) -> Result<bool> {
        let ranges = collect_text_ranges_in_selection(self, from, to)?;

        for (text_node_id, start_offset, end_offset) in ranges {
            let Some(node) = self.node(text_node_id) else {
                continue;
            };
            if let Node::Text(text_node) = node.node() {
                let segments = text_node.text.get_segments();
                let mut current_offset = 0;

                for segment in segments {
                    let segment_len = segment.text.chars().count();
                    let segment_end = current_offset + segment_len;
                    let overlap_start = current_offset.max(start_offset);
                    let overlap_end = segment_end.min(end_offset);

                    if overlap_start < overlap_end {
                        for ann_id in &segment.annotations {
                            if let Some(ann) = self.doc().get_annotation(*ann_id) {
                                if ann.as_type() == ann_type {
                                    return Ok(true);
                                }
                            }
                        }
                    }

                    current_offset = segment_end;
                }
            }
        }

        Ok(false)
    }
}

fn collect_text_ranges_in_selection(
    tr: &Transaction,
    from: Position,
    to: Position,
) -> Result<Vec<(NodeId, usize, usize)>> {
    let block_ids = collect_blocks_in_range(tr.doc(), from, to)?;
    let mut ranges = Vec::new();

    for block_id in block_ids {
        let block = tr
            .node(block_id)
            .with_context(|| format!("Block {block_id} not found"))?;

        if !block.spec().is_textblock(tr.doc().schema()) {
            continue;
        }

        let block_len = block_content_len(&block);
        let (start, end) = calculate_block_offsets(block_id, block_len, from, to);

        collect_ranges_in_textblock(&block, start, end, &mut ranges)?;
    }

    Ok(ranges)
}

fn collect_ranges_in_textblock(
    parent: &NodeRef,
    start_offset: usize,
    end_offset: usize,
    result: &mut Vec<(NodeId, usize, usize)>,
) -> Result<()> {
    let mut current_offset = 0;

    for child in parent.children() {
        match child.node() {
            Node::Text(text_node) => {
                let text_len = text_node.text.char_len();
                let child_end = current_offset + text_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = child_end.min(end_offset);

                if overlap_start < overlap_end {
                    let local_start = overlap_start - current_offset;
                    let local_end = overlap_end - current_offset;
                    result.push((child.node_id(), local_start, local_end));
                }

                current_offset = child_end;
            }
            Node::HardBreak(_) => {
                current_offset += 1;
            }
            _ => {}
        }
    }

    Ok(())
}

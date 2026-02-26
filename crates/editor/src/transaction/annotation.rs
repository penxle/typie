use crate::model::*;
use crate::runtime::Effect;
use crate::state::{Position, collect_text_ranges_in_selection};
use crate::transaction::Transaction;
use crate::utils::collect_codepoints;
use anyhow::{Context, Result};

impl Transaction {
    pub fn add_annotation(&mut self, annotation: Annotation) -> Result<()> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            anyhow::bail!("Cannot add annotation to collapsed selection");
        }

        let ruby_codepoints = match &annotation {
            Annotation::Ruby(ruby) => collect_codepoints(&ruby.text),
            _ => Vec::new(),
        };

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

        let (from, to) = selection.as_sorted(self.doc())?;
        let ranges = collect_text_ranges_in_selection(self.doc(), &selection, from, to)?;
        for (text_node_id, start_offset, end_offset) in ranges {
            let allowed = self.doc().allowed_annotations_for(text_node_id);
            anyhow::ensure!(
                allowed.contains(&ann_type),
                "Annotation '{:?}' not allowed at node {}",
                ann_type,
                text_node_id,
            );

            let node = self.node_mut(text_node_id).context("Text node not found")?;
            if let Some(Node::Text(text_node)) = node.node() {
                text_node
                    .text
                    .apply_annotation(start_offset..end_offset, &annotation)?;
                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }
        }

        if !ruby_codepoints.is_empty() {
            let defaults = self.doc().default_attrs();
            self.push_effect(Effect::FontDetected {
                family: defaults.font_family().to_string(),
                weight: defaults.font_weight(),
                codepoints: ruby_codepoints,
            });
        }

        Ok(())
    }

    pub fn update_annotation(
        &mut self,
        ann_type: AnnotationType,
        annotation: Annotation,
    ) -> Result<bool> {
        let ruby_codepoints = match &annotation {
            Annotation::Ruby(ruby) => collect_codepoints(&ruby.text),
            _ => Vec::new(),
        };

        let selection = self.selection().clone();
        let ranges = if selection.is_collapsed() {
            self.find_annotation_ranges_at_position(selection.anchor, ann_type)
        } else {
            self.find_annotation_ranges(ann_type)
        };
        if ranges.is_empty() {
            return Ok(false);
        }

        for (text_node_id, start_offset, end_offset) in ranges {
            let node = self.node_mut(text_node_id).context("Text node not found")?;
            if let Some(Node::Text(text_node)) = node.node() {
                text_node
                    .text
                    .remove_annotation(start_offset..end_offset, ann_type)?;
                text_node
                    .text
                    .apply_annotation(start_offset..end_offset, &annotation)?;
                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }
        }

        if !ruby_codepoints.is_empty() {
            let defaults = self.doc().default_attrs();
            self.push_effect(Effect::FontDetected {
                family: defaults.font_family().to_string(),
                weight: defaults.font_weight(),
                codepoints: ruby_codepoints,
            });
        }

        Ok(true)
    }

    pub fn remove_annotation(&mut self, ann_type: AnnotationType) -> Result<bool> {
        let selection = self.selection().clone();
        let ranges = if selection.is_collapsed() {
            self.find_annotation_ranges_at_position(selection.anchor, ann_type)
        } else {
            self.find_annotation_ranges(ann_type)
        };
        if ranges.is_empty() {
            return Ok(false);
        }

        for (text_node_id, start_offset, end_offset) in ranges {
            let node = self.node_mut(text_node_id).context("Text node not found")?;
            if let Some(Node::Text(text_node)) = node.node() {
                text_node
                    .text
                    .remove_annotation(start_offset..end_offset, ann_type)?;
                self.push_effect(Effect::NodeChanged {
                    node_id: text_node_id,
                });
            }
        }

        Ok(true)
    }

    fn find_annotation_ranges_at_position(
        &self,
        position: Position,
        ann_type: AnnotationType,
    ) -> Vec<(NodeId, usize, usize)> {
        let block_id = position.node_id;
        let block_offset = position.offset;

        let mut all_ranges = Vec::new();
        self.find_annotation_in_block(block_id, ann_type, &mut all_ranges);

        if all_ranges.is_empty() {
            return Vec::new();
        }

        let Some(block) = self.node(block_id) else {
            return Vec::new();
        };

        // Map block offset to (text_node_id, local_offset)
        let mut current_block_offset = 0;
        let mut cursor_text_node: Option<(NodeId, usize)> = None;

        for child in block.children() {
            let Some(child_node) = child.node() else {
                continue;
            };
            match child_node {
                Node::Text(text_node) => {
                    let text_len = text_node.text.char_len();
                    let child_end = current_block_offset + text_len;

                    if block_offset >= current_block_offset && block_offset <= child_end {
                        let local_offset = block_offset - current_block_offset;
                        cursor_text_node = Some((child.node_id(), local_offset));
                        break;
                    }

                    current_block_offset = child_end;
                }
                Node::HardBreak(_) => {
                    current_block_offset += 1;
                }
                _ => {}
            }
        }

        let Some((cursor_node_id, local_offset)) = cursor_text_node else {
            return Vec::new();
        };

        all_ranges
            .into_iter()
            .filter(|(node_id, start, end)| {
                *node_id == cursor_node_id && *start <= local_offset && local_offset < *end
            })
            .collect()
    }

    fn find_annotation_ranges(&self, ann_type: AnnotationType) -> Vec<(NodeId, usize, usize)> {
        let mut result = Vec::new();

        let root = match self.node(NodeId::ROOT) {
            Some(r) => r,
            None => return result,
        };

        for block in root.children() {
            self.find_annotation_in_block(block.node_id(), ann_type, &mut result);
        }

        result
    }

    fn find_annotation_in_block(
        &self,
        block_id: NodeId,
        ann_type: AnnotationType,
        result: &mut Vec<(NodeId, usize, usize)>,
    ) {
        let Some(block) = self.node(block_id) else {
            return;
        };

        for child in block.children() {
            let Some(child_node) = child.node() else {
                continue;
            };
            match child_node {
                Node::Text(text_node) => {
                    let segments = text_node.text.get_segments();
                    let mut current_offset = 0;
                    let mut range_start: Option<usize> = None;

                    for segment in &segments {
                        let segment_len = segment.text.chars().count();
                        let has_annotation =
                            segment.annotations.iter().any(|a| a.as_type() == ann_type);

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
                    if !child.spec().map_or(false, |s| s.inline) {
                        self.find_annotation_in_block(child.node_id(), ann_type, result);
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
        let ranges = collect_text_ranges_in_selection(self.doc(), self.selection(), from, to)?;

        for (text_node_id, start_offset, end_offset) in ranges {
            let Some(node) = self.node(text_node_id) else {
                continue;
            };
            if let Some(Node::Text(text_node)) = node.node() {
                let segments = text_node.text.get_segments();
                let mut current_offset = 0;

                for segment in segments {
                    let segment_len = segment.text.chars().count();
                    let segment_end = current_offset + segment_len;
                    let overlap_start = current_offset.max(start_offset);
                    let overlap_end = segment_end.min(end_offset);

                    if overlap_start < overlap_end {
                        if segment.annotations.iter().any(|a| a.as_type() == ann_type) {
                            return Ok(true);
                        }
                    }

                    current_offset = segment_end;
                }
            }
        }

        Ok(false)
    }
}

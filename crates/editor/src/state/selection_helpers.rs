use crate::model::{
    Annotation, AnnotationType, Attr, Doc, Node, NodeId, NodeRef, SelectionDecor, Style, StyleType,
    TextAlign,
};
use crate::state::position::Position;
use crate::state::position_helpers::compare_positions;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::table_helpers::{collect_cells_in_range, compute_table_selection};
use crate::state::{BlockTraverser, Selection};
use crate::types::Affinity;
use anyhow::{Context, Result};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Ordering;
use std::mem::discriminant;

#[derive(Debug, Clone)]
pub enum BlockAttr {
    TextAlign(TextAlign),
    LineHeight(u32),
}

impl PartialEq for BlockAttr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::TextAlign(a), Self::TextAlign(b)) => a == b,
            (Self::LineHeight(a), Self::LineHeight(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollectedBlockAttrs {
    pub values: Vec<BlockAttr>,
    pub has_absent: bool,
}

#[derive(Debug, Clone)]
pub struct SelectionAttributes {
    pub block_attrs: Vec<CollectedBlockAttrs>,
    pub style_values: FxHashMap<StyleType, Vec<Style>>,
    pub annotation_values: FxHashMap<AnnotationType, Vec<Annotation>>,
    pub absent_styles: FxHashSet<StyleType>,
    pub absent_annotations: FxHashSet<AnnotationType>,
    pub has_text_segments: bool,
}

fn extract_block_attrs(node: &NodeRef) -> Vec<BlockAttr> {
    let mut result = Vec::new();
    for ancestor in node.ancestors() {
        if let Some(Node::Paragraph(p)) = ancestor.node() {
            result.push(BlockAttr::TextAlign(p.align));
            result.push(BlockAttr::LineHeight(p.line_height));
        }
    }
    result
}

pub(crate) fn collect_blocks_in_range(
    doc: &Doc,
    from: Position,
    to: Position,
) -> Result<Vec<NodeId>> {
    let start_id = start_block_id(doc, from)?;

    if from == to {
        return Ok(vec![start_id]);
    }

    let end_exclusive = end_boundary_node(doc, to)?;

    let mut block_ids = Vec::new();
    block_ids.push(start_id);

    let mut traverser = BlockTraverser::new(doc, start_id)?;

    while let Some(node_id) = traverser.next() {
        if Some(node_id) == end_exclusive {
            break;
        }
        block_ids.push(node_id);
    }

    Ok(block_ids)
}

pub fn collect_top_level_blocks_in_range(
    doc: &Doc,
    from: Position,
    to: Position,
) -> Result<Vec<NodeId>> {
    let start_id = start_block_id(doc, from)?;

    if from == to {
        return Ok(vec![start_id]);
    }

    let end_exclusive = end_boundary_node(doc, to)?;
    let mut top_level_blocks = Vec::new();
    let mut current = start_id;

    loop {
        top_level_blocks.push(current);

        if let Some(end) = end_exclusive {
            if end == current || doc.is_ancestor(current, end) {
                break;
            }
        }

        let mut traverser = BlockTraverser::new_after_subtree(doc, current)
            .context("collect_top_level_blocks_in_range: Traverser init failed")?;
        let Some(next) = traverser.next() else {
            break;
        };

        if Some(next) == end_exclusive {
            break;
        }

        current = next;
    }

    Ok(top_level_blocks)
}

pub(crate) fn start_block_id(doc: &Doc, pos: Position) -> Result<NodeId> {
    let block_node = doc
        .node(pos.node_id)
        .context("start_block_id: Block node not found")?;

    if let Some((child_id, _)) = find_child_at_offset(&block_node, pos.offset) {
        let child_is_inline = doc
            .node(child_id)
            .context("start_block_id: Child node not found")?
            .is_inline();

        if !child_is_inline {
            return Ok(child_id);
        }
    }

    Ok(block_node.node_id())
}

pub(crate) fn end_boundary_node(doc: &Doc, pos: Position) -> Result<Option<NodeId>> {
    let block_node = doc
        .node(pos.node_id)
        .context("end_boundary_node: Block node not found")?;
    let block_id = block_node.node_id();

    if pos.offset == 0 && !matches!(block_node.node(), Some(Node::Root(_))) {
        return Ok(Some(block_id));
    }

    if let Some((child_id, local_offset)) = find_child_at_offset(&block_node, pos.offset) {
        let child_node = doc
            .node(child_id)
            .context("end_boundary_node: Child node not found")?;
        let child_is_inline = child_node.is_inline();

        if !child_is_inline && local_offset == 0 {
            return Ok(Some(child_id));
        }

        let target_block = if child_is_inline { block_id } else { child_id };

        if !child_is_inline && local_offset > 0 {
            let mut traverser = BlockTraverser::new_after_subtree(doc, target_block)
                .context("end_boundary_node: Traverser init failed")?;
            return Ok(traverser.next());
        }

        let mut traverser = BlockTraverser::new(doc, target_block)
            .context("end_boundary_node: Traverser init failed")?;
        return Ok(traverser.next());
    }

    let mut traverser =
        BlockTraverser::new(doc, block_id).context("end_boundary_node: Traverser init failed")?;
    Ok(traverser.next())
}

pub fn block_content_len(node: &NodeRef<'_>) -> usize {
    node.children()
        .map(|child| child.node().map_or(1, |n| n.len())) // undecodable nodes count as 1 (atomic)
        .sum()
}

// selection 범위 내 block의 start_offset과 end_offset을 계산
pub fn calculate_block_offsets(
    block_id: NodeId,
    block_len: usize,
    from: Position,
    to: Position,
) -> (usize, usize) {
    if block_id == from.node_id && block_id == to.node_id {
        (from.offset, to.offset)
    } else if block_id == from.node_id {
        (from.offset, block_len)
    } else if block_id == to.node_id {
        (0, to.offset)
    } else {
        (0, block_len)
    }
}

pub fn build_selection_decorations(
    doc: &Doc,
    selection: &Selection,
    block_ids: Option<&[NodeId]>,
) -> Vec<SelectionDecor> {
    let mut decorations = Vec::new();

    if selection.is_collapsed() {
        return Vec::new();
    }

    let (from, to) = match selection.as_sorted(doc) {
        Ok(sorted) => sorted,
        Err(_) => return Vec::new(),
    };

    let structure_selection_info = compute_structure_selection(doc, selection);
    let mut processed_structural_nodes = FxHashSet::default();

    decorations.extend(collect_structure_decorations(
        doc,
        &structure_selection_info,
        &mut processed_structural_nodes,
    ));

    let block_ids = match block_ids {
        Some(ids) => ids.to_vec(),
        None => collect_selected_block_ids(doc, selection, &structure_selection_info),
    };

    let block_id_set: FxHashSet<NodeId> = block_ids.iter().cloned().collect();

    for &block_id in &block_ids {
        if should_skip_block_decoration(doc, block_id, &processed_structural_nodes) {
            continue;
        }

        if let Some(decor) = build_block_selection_decoration(doc, block_id, from, to) {
            decorations.push(decor);
        }
    }

    add_ancestor_decorations(
        doc,
        from,
        to,
        &structure_selection_info,
        &block_id_set,
        &processed_structural_nodes,
        &mut decorations,
    );

    decorations
}

fn collect_structure_decorations(
    doc: &Doc,
    structure_selection: &StructureSelectionInfo,
    processed_structural_nodes: &mut FxHashSet<NodeId>,
) -> Vec<SelectionDecor> {
    let mut decorations = Vec::new();
    match structure_selection {
        StructureSelectionInfo::Rectangular { table_id, range } => {
            let cells = collect_cells_in_range(doc, *table_id, *range);
            for cell_id in cells {
                if processed_structural_nodes.insert(cell_id) {
                    decorations.push(SelectionDecor::Block { node_id: cell_id });
                }
            }
        }
        StructureSelectionInfo::Structural(block_ids) => {
            for &block_id in block_ids {
                let Some(node) = doc.node(block_id) else {
                    continue;
                };
                if should_skip_block_decoration(doc, block_id, processed_structural_nodes) {
                    continue;
                }

                if matches!(node.node(), Some(Node::Table(_))) {
                    for row in node.children() {
                        for cell in row.children() {
                            let cell_id = cell.node_id();
                            if processed_structural_nodes.insert(cell_id) {
                                decorations.push(SelectionDecor::Block { node_id: cell_id });
                            }
                        }
                    }
                } else if matches!(node.node(), Some(Node::Fold(_))) {
                    decorations.push(SelectionDecor::Block { node_id: block_id });
                    processed_structural_nodes.insert(block_id);
                }
            }
        }
        StructureSelectionInfo::None => {}
    }
    decorations
}

fn build_block_selection_decoration(
    doc: &Doc,
    block_id: NodeId,
    from: Position,
    to: Position,
) -> Option<SelectionDecor> {
    let block = doc.node(block_id)?;

    if matches!(block.node(), Some(Node::HorizontalRule(_))) {
        return Some(SelectionDecor::Block { node_id: block_id });
    }

    if !block.spec().map_or(false, |s| s.is_textblock(doc.schema())) {
        return None;
    }

    let block_len = block_content_len(&block).max(1);
    let (start_offset, end_offset) = calculate_block_offsets(block_id, block_len, from, to);
    Some(SelectionDecor::TextRange {
        node_id: block_id,
        start_offset,
        end_offset,
    })
}

fn should_skip_block_decoration(
    doc: &Doc,
    block_id: NodeId,
    processed_structural_nodes: &FxHashSet<NodeId>,
) -> bool {
    let mut current_id = Some(block_id);
    while let Some(id) = current_id {
        if processed_structural_nodes.contains(&id) {
            return true;
        }

        if let Some(node) = doc.node(id) {
            current_id = node.parent().map(|n| n.node_id());
        } else {
            break;
        }
    }
    false
}

fn add_ancestor_decorations(
    doc: &Doc,
    from: Position,
    to: Position,
    structure_selection: &StructureSelectionInfo,
    processed_blocks: &FxHashSet<NodeId>,
    processed_structural_nodes: &FxHashSet<NodeId>,
    decorations: &mut Vec<SelectionDecor>,
) {
    let Some(from_node) = doc.node(from.node_id) else {
        return;
    };
    let Some(to_node) = doc.node(to.node_id) else {
        return;
    };

    let from_path: Vec<_> = from_node.ancestors().map(|n| n.node_id()).collect();
    let to_path: Vec<_> = to_node.ancestors().map(|n| n.node_id()).collect();

    for (from_idx, &ancestor_id) in from_path.iter().enumerate() {
        if processed_blocks.contains(&ancestor_id)
            || processed_structural_nodes.contains(&ancestor_id)
        {
            continue;
        }

        let Some(to_idx) = to_path.iter().position(|&id| id == ancestor_id) else {
            continue;
        };

        if from_idx == 0 && to_idx == 0 && from.offset == to.offset {
            break;
        }

        let Some(ancestor) = doc.node(ancestor_id) else {
            break;
        };

        match structure_selection {
            StructureSelectionInfo::Rectangular { table_id, .. } if *table_id == ancestor_id => {
                break;
            }
            StructureSelectionInfo::Structural(block_ids) if block_ids.contains(&ancestor_id) => {
                break;
            }
            _ => {}
        }

        let start_child_id = if from_idx > 0 {
            Some(from_path[from_idx - 1])
        } else {
            find_child_at_offset(&ancestor, from.offset).map(|(id, _)| id)
        };

        let end_child_id = if to_idx > 0 {
            let direct_child_id = to_path[to_idx - 1];
            if to.offset == 0 {
                doc.node(direct_child_id)
                    .and_then(|node| node.prev_sibling().map(|n| n.node_id()))
            } else {
                Some(direct_child_id)
            }
        } else {
            if to.offset == 0 {
                None
            } else {
                find_child_at_offset(&ancestor, to.offset.saturating_sub(1)).map(|(id, _)| id)
            }
        };

        let (Some(start_child_id), Some(end_child_id)) = (start_child_id, end_child_id) else {
            break;
        };

        if start_child_id == end_child_id && (from_idx != 0 || to_idx != 0) {
            break;
        }

        let mut start_offset = 0usize;
        let mut end_offset = 0usize;
        let mut found_start = false;

        for child in ancestor.children() {
            let child_id = child.node_id();
            let child_len = child.node().map_or(1, |n| n.len()); // undecodable nodes count as 1 (atomic)

            if child_id == start_child_id {
                found_start = true;
            }

            if !found_start {
                start_offset += child_len;
            }

            end_offset += child_len;
            if child_id == end_child_id {
                break;
            }
        }

        if end_offset > start_offset {
            let decor = if ancestor
                .spec()
                .map_or(false, |s| s.is_textblock(doc.schema()))
            {
                SelectionDecor::TextRange {
                    node_id: ancestor_id,
                    start_offset,
                    end_offset,
                }
            } else {
                SelectionDecor::Block {
                    node_id: ancestor_id,
                }
            };
            decorations.push(decor);
        }

        break;
    }
}

pub fn collect_block_attrs_at(doc: &Doc, node_id: NodeId) -> Vec<CollectedBlockAttrs> {
    let Some(node) = doc.node(node_id) else {
        return Vec::new();
    };
    group_block_attrs(extract_block_attrs(&node))
}

fn group_block_attrs(attrs: Vec<BlockAttr>) -> Vec<CollectedBlockAttrs> {
    let mut map: Vec<(std::mem::Discriminant<BlockAttr>, Vec<BlockAttr>)> = Vec::new();
    for attr in attrs {
        let disc = discriminant(&attr);
        if let Some(entry) = map.iter_mut().find(|(d, _)| *d == disc) {
            if !entry.1.contains(&attr) {
                entry.1.push(attr);
            }
        } else {
            map.push((disc, vec![attr]));
        }
    }
    map.into_iter()
        .map(|(_, values)| CollectedBlockAttrs {
            values,
            has_absent: false,
        })
        .collect()
}

pub fn compute_selection_attrs(
    doc: &Doc,
    block_ids: &[NodeId],
    from: Position,
    to: Position,
) -> SelectionAttributes {
    let mut block_attr_map: FxHashMap<
        std::mem::Discriminant<BlockAttr>,
        (Vec<BlockAttr>, FxHashSet<NodeId>),
    > = FxHashMap::default();

    let mut style_values: FxHashMap<StyleType, Vec<Style>> = FxHashMap::default();
    let mut annotation_values: FxHashMap<AnnotationType, Vec<Annotation>> = FxHashMap::default();
    let mut segment_count: usize = 0;
    let mut style_segment_counts: FxHashMap<StyleType, usize> = FxHashMap::default();
    let mut annotation_segment_counts: FxHashMap<AnnotationType, usize> = FxHashMap::default();

    let mut effective_block_ids: Vec<NodeId> = block_ids.to_vec();
    if to.offset == 0 && from.node_id != to.node_id && !effective_block_ids.contains(&to.node_id) {
        if let Some(node) = doc.node(to.node_id) {
            if node.spec().map_or(false, |s| s.is_textblock(doc.schema()))
                && block_content_len(&node) == 0
            {
                effective_block_ids.push(to.node_id);
            }
        }
    }

    for &block_id in &effective_block_ids {
        let Some(node) = doc.node(block_id) else {
            continue;
        };

        let block_len = block_content_len(&node);

        for attr in extract_block_attrs(&node) {
            let entry = block_attr_map.entry(discriminant(&attr)).or_default();
            entry.1.insert(block_id);
            if !entry.0.contains(&attr) {
                entry.0.push(attr);
            }
        }

        let (start_offset, end_offset) = calculate_block_offsets(block_id, block_len, from, to);

        accumulate_block_attrs(
            &node,
            start_offset,
            end_offset,
            &mut style_values,
            &mut annotation_values,
            &mut segment_count,
            &mut style_segment_counts,
            &mut annotation_segment_counts,
        );

        if block_len == 0 {
            if let Some(cascade) = node.cascade_attrs() {
                let styles = Attr::extract_styles(&cascade);
                if !styles.is_empty() {
                    segment_count += 1;
                    for style in &styles {
                        let st = style.as_type();
                        *style_segment_counts.entry(st).or_default() += 1;
                        let values = style_values.entry(st).or_default();
                        if !values.iter().any(|v| v == style) {
                            values.push(style.clone());
                        }
                    }
                }
            }
        }
    }

    let block_count = effective_block_ids.len();

    let block_attrs: Vec<CollectedBlockAttrs> = block_attr_map
        .into_values()
        .filter(|(values, _)| !values.is_empty())
        .map(|(values, covered)| CollectedBlockAttrs {
            values,
            has_absent: covered.len() < block_count,
        })
        .collect();

    let mut absent_styles: FxHashSet<StyleType> = FxHashSet::default();
    if segment_count > 0 {
        for (&st, &count) in &style_segment_counts {
            if count < segment_count {
                absent_styles.insert(st);
            }
        }
    }

    let mut absent_annotations: FxHashSet<AnnotationType> = FxHashSet::default();
    if segment_count > 0 {
        for (&at, &count) in &annotation_segment_counts {
            if count < segment_count {
                absent_annotations.insert(at);
            }
        }
    }

    SelectionAttributes {
        block_attrs,
        style_values,
        annotation_values,
        absent_styles,
        absent_annotations,
        has_text_segments: segment_count > 0,
    }
}

fn accumulate_block_attrs(
    block: &NodeRef<'_>,
    start_offset: usize,
    end_offset: usize,
    style_values: &mut FxHashMap<StyleType, Vec<Style>>,
    annotation_values: &mut FxHashMap<AnnotationType, Vec<Annotation>>,
    segment_count: &mut usize,
    style_segment_counts: &mut FxHashMap<StyleType, usize>,
    annotation_segment_counts: &mut FxHashMap<AnnotationType, usize>,
) {
    let mut current_offset = 0;

    for child in block.children() {
        match child.node() {
            Some(Node::Text(text_node)) => {
                let text_len = text_node.text.char_len();
                let child_end = current_offset + text_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = child_end.min(end_offset);

                if overlap_start < overlap_end {
                    let local_start = overlap_start - current_offset;
                    let local_end = overlap_end - current_offset;

                    let segments = text_node.text.get_segments();
                    let mut seg_offset = 0;

                    for segment in segments {
                        let segment_len = segment.text.chars().count();
                        let seg_end = seg_offset + segment_len;

                        let seg_overlap_start = seg_offset.max(local_start);
                        let seg_overlap_end = seg_end.min(local_end);

                        if seg_overlap_start < seg_overlap_end {
                            *segment_count += 1;

                            for style in &segment.styles {
                                let st = style.as_type();
                                *style_segment_counts.entry(st).or_default() += 1;
                                let values = style_values.entry(st).or_default();
                                if !values.iter().any(|v| v == style) {
                                    values.push(style.clone());
                                }
                            }

                            for annotation in &segment.annotations {
                                let at = annotation.as_type();
                                *annotation_segment_counts.entry(at).or_default() += 1;
                                let values = annotation_values.entry(at).or_default();
                                if !values.iter().any(|v| v == annotation) {
                                    values.push(annotation.clone());
                                }
                            }
                        }

                        seg_offset = seg_end;
                    }
                }

                current_offset = child_end;
            }
            Some(Node::HardBreak(_)) => {
                current_offset += 1;
            }
            _ => {}
        }
    }
}

pub fn is_node_fully_selected(doc: &Doc, selection: &Selection, node_id: NodeId) -> Result<bool> {
    let (from, to) = selection.as_sorted(doc)?;

    let node = doc
        .node(node_id)
        .context("is_node_fully_selected: Node not found")?;
    let node_start = Position::new(node_id, 0, Affinity::default());
    let node_end = Position::new(node_id, block_content_len(&node), Affinity::default());

    let start_ok = compare_positions(doc, from, node_start)? != Ordering::Greater;
    let end_ok = compare_positions(doc, to, node_end)? != Ordering::Less;

    Ok(start_ok && end_ok)
}

pub fn collect_nodes_in_selection<F>(
    doc: &Doc,
    selection: &Selection,
    filter: F,
) -> Result<Vec<NodeId>>
where
    F: Fn(&Node) -> bool,
{
    let (from, to) = selection.as_sorted(doc)?;
    let blocks = collect_blocks_in_range(doc, from, to)?;

    Ok(blocks
        .into_iter()
        .filter(|&block_id| {
            doc.node(block_id)
                .and_then(|node| node.node().map(|n| filter(n)))
                .unwrap_or(false)
        })
        .collect())
}

fn collect_all_blocks_in_subtree(doc: &Doc, root_id: NodeId) -> Vec<NodeId> {
    let mut blocks = vec![root_id];
    let mut traverser = match BlockTraverser::new(doc, root_id) {
        Ok(t) => t,
        Err(_) => return blocks,
    };

    while let Some(node_id) = traverser.next() {
        if !doc.is_ancestor(root_id, node_id) {
            break;
        }
        blocks.push(node_id);
    }

    blocks
}

pub fn collect_selected_block_ids(
    doc: &Doc,
    selection: &Selection,
    cell_selection: &StructureSelectionInfo,
) -> Vec<NodeId> {
    if selection.is_collapsed() {
        return Vec::new();
    }

    let Ok((from, to)) = selection.as_sorted(doc) else {
        return Vec::new();
    };

    match cell_selection {
        StructureSelectionInfo::Rectangular { table_id, range } => {
            let mut ids = Vec::new();
            let cells = collect_cells_in_range(doc, *table_id, *range);
            for cell_id in cells {
                ids.extend(collect_all_blocks_in_subtree(doc, cell_id));
            }
            ids
        }
        StructureSelectionInfo::Structural(block_ids) => {
            let mut ids: FxHashSet<NodeId> = collect_blocks_in_range(doc, from, to)
                .unwrap_or_default()
                .into_iter()
                .collect();

            for &block_id in block_ids {
                ids.extend(collect_all_blocks_in_subtree(doc, block_id));
            }

            let mut result: Vec<NodeId> = ids.into_iter().collect();
            result.sort_by(|&a, &b| {
                let pos_a = Position::new(a, 0, Affinity::default());
                let pos_b = Position::new(b, 0, Affinity::default());
                compare_positions(doc, pos_a, pos_b).unwrap_or(Ordering::Equal)
            });
            result
        }
        StructureSelectionInfo::None => collect_blocks_in_range(doc, from, to).unwrap_or_default(),
    }
}

pub fn collect_text_target_blocks(
    doc: &Doc,
    selection: &Selection,
    from: Position,
    to: Position,
) -> Result<(Vec<NodeId>, bool)> {
    let structure_selection = compute_structure_selection(doc, selection);
    let is_rectangular = matches!(
        structure_selection,
        StructureSelectionInfo::Rectangular { .. }
    );

    if matches!(
        structure_selection,
        StructureSelectionInfo::Rectangular { .. } | StructureSelectionInfo::Structural(_)
    ) {
        return Ok((
            collect_selected_block_ids(doc, selection, &structure_selection),
            is_rectangular,
        ));
    }

    Ok((collect_blocks_in_range(doc, from, to)?, false))
}

pub fn collect_text_ranges_in_selection(
    doc: &Doc,
    selection: &Selection,
    from: Position,
    to: Position,
) -> Result<Vec<(NodeId, usize, usize)>> {
    let structure_selection = compute_structure_selection(doc, selection);
    let (block_ids, is_rectangular) = collect_text_target_blocks(doc, selection, from, to)?;
    let mut ranges = Vec::new();

    for block_id in block_ids {
        let block = doc
            .node(block_id)
            .with_context(|| format!("Block {block_id} not found"))?;

        if !block.spec().map_or(false, |s| s.is_textblock(doc.schema())) {
            continue;
        }

        let block_len = block_content_len(&block);
        let in_structural_selection = matches!(
            &structure_selection,
            StructureSelectionInfo::Structural(root_ids)
                if root_ids
                    .iter()
                    .any(|&root_id| root_id == block_id || doc.is_ancestor(root_id, block_id))
        );

        let (start, end) = if is_rectangular || in_structural_selection {
            (0, block_len)
        } else {
            calculate_block_offsets(block_id, block_len, from, to)
        };

        collect_text_ranges_in_textblock(&block, start, end, &mut ranges)?;
    }

    Ok(ranges)
}

fn collect_text_ranges_in_textblock(
    parent: &NodeRef,
    start_offset: usize,
    end_offset: usize,
    result: &mut Vec<(NodeId, usize, usize)>,
) -> Result<()> {
    let mut current_offset = 0;

    for child in parent.children() {
        match child.node() {
            Some(Node::Text(text_node)) => {
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
            Some(Node::HardBreak(_)) => {
                current_offset += 1;
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn selected_single_block_id(doc: &Doc, selection: &Selection) -> Option<NodeId> {
    if selection.is_collapsed() {
        return None;
    }

    let structure_selection = compute_structure_selection(doc, selection);
    let block_ids = collect_selected_block_ids(doc, selection, &structure_selection);
    if block_ids.len() == 1 {
        Some(block_ids[0])
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructureSelectionInfo {
    None,
    Rectangular {
        table_id: NodeId,
        range: ((usize, usize), (usize, usize)),
    },
    Structural(Vec<NodeId>),
}

pub fn compute_structure_selection(doc: &Doc, selection: &Selection) -> StructureSelectionInfo {
    if let Some((table_id, range)) = compute_table_selection(doc, selection) {
        return StructureSelectionInfo::Rectangular { table_id, range };
    }

    let blocks = collect_relevant_blocks(doc, selection).unwrap_or_default();

    if blocks.is_empty() {
        StructureSelectionInfo::None
    } else {
        StructureSelectionInfo::Structural(blocks)
    }
}

fn collect_relevant_blocks(doc: &Doc, selection: &Selection) -> Result<Vec<NodeId>> {
    let mut block_ids = FxHashSet::default();

    if let Ok(traversed) = collect_nodes_in_selection(doc, selection, |node| {
        doc.schema()
            .node_spec(node.as_type())
            .is_structural_root(doc.schema())
    }) {
        block_ids.extend(traversed);
    }

    for &node_id in &[selection.anchor.node_id, selection.head.node_id] {
        let mut current_id = Some(node_id);
        while let Some(id) = current_id {
            if let Some(node) = doc.node(id) {
                if node
                    .spec()
                    .map_or(false, |s| s.is_structural_root(doc.schema()))
                {
                    block_ids.insert(id);
                }
                current_id = node.parent().map(|n| n.node_id());
            } else {
                break;
            }
        }
    }

    let mut result: Vec<_> = block_ids
        .into_iter()
        .filter(|&id| {
            let fully_selected = is_node_fully_selected(doc, selection, id).unwrap_or(false);

            let contains_anchor =
                id == selection.anchor.node_id || doc.is_ancestor(id, selection.anchor.node_id);
            let contains_head =
                id == selection.head.node_id || doc.is_ancestor(id, selection.head.node_id);

            fully_selected || (contains_anchor != contains_head)
        })
        .collect();

    result.sort_by(|&a, &b| {
        let pos_a = Position::new(a, 0, Affinity::default());
        let pos_b = Position::new(b, 0, Affinity::default());
        compare_positions(doc, pos_a, pos_b).unwrap_or(Ordering::Equal)
    });

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FontWeightStyle;
    use crate::transaction::Transaction;

    #[test]
    fn test_compute_selection_attrs_with_list() {
        let mut p1 = id!();
        let state = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "bold" }
                        }
                    }
                }
            }
            selection { (p1, 0) -> (p1, 4) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let state = tr.commit().unwrap().0;

        let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
        let block_ids = collect_blocks_in_range(&state.doc, from, to).unwrap();

        let attrs = compute_selection_attrs(&state.doc, &block_ids, from, to);

        let has_font_weight = attrs.style_values.contains_key(&StyleType::FontWeight);

        assert!(
            has_font_weight,
            "Should detect font weight style in list item"
        );
    }

    #[test]
    fn test_build_selection_decorations_includes_fold_title() {
        let mut n1 = id!();
        let mut n2 = id!();

        let state = state! {
            doc {
                paragraph {
                    text { "123412312" }
                }
                fold {
                    @n1 fold_title {
                        text { "12123123123123" }
                    }
                    fold_content {
                        @n2 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 8) -> (n2, 0) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let fold_title_decor = decorations.iter().find(|d| d.node_id() == n1);
        assert!(
            fold_title_decor.is_some(),
            "FoldTitle should have a selection decoration"
        );

        let decor = fold_title_decor.unwrap();
        assert_eq!(decor.start_offset(), 8);
        assert_eq!(decor.end_offset(), 14);
    }

    #[test]
    fn test_build_selection_decorations_selecting_all() {
        let mut n1 = id!();
        let mut fold_id = id!();
        let mut n2 = id!();
        let mut n3 = id!();

        let state = state! {
            doc {
                @n1 paragraph { text { "1" } }
                @fold_id fold {
                    @n2 fold_title { text { "2" } }
                    fold_content { paragraph {} }
                }
                @n3 paragraph { text { "3" } }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 3) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let para_1_decor = decorations.iter().find(|d| d.node_id() == n1);
        assert!(
            para_1_decor.is_some(),
            "Paragraph 1 should have a selection decoration"
        );
        assert_eq!(para_1_decor.unwrap().start_offset(), 0);
        assert_eq!(para_1_decor.unwrap().end_offset(), 1);

        let fold_title_decor = decorations.iter().find(|d| d.node_id() == n2);
        assert!(
            fold_title_decor.is_none(),
            "FoldTitle should NOT have a separate decoration when Fold is selected"
        );

        let fold_decor = decorations
            .iter()
            .find(|d| matches!(d, SelectionDecor::Block { node_id } if *node_id == fold_id));
        assert!(fold_decor.is_some(), "Fold should have a Fold decoration");

        let para_3_decor = decorations.iter().find(|d| d.node_id() == n3);
        assert!(
            para_3_decor.is_some(),
            "Paragraph 3 should have a selection decoration"
        );
        assert_eq!(para_3_decor.unwrap().start_offset(), 0);
        assert_eq!(para_3_decor.unwrap().end_offset(), 1);
    }

    #[test]
    fn test_build_selection_decorations_single_horizontal_rule() {
        let mut hr = id!();
        let state = state! {
            doc {
                paragraph { text { "before" } }
                @hr horizontal_rule {}
                paragraph { text { "after" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let hr_decor = decorations
            .iter()
            .find(|d| matches!(d, SelectionDecor::Block { node_id } if *node_id == hr));
        assert!(
            hr_decor.is_some(),
            "HorizontalRule should have a dedicated selection decoration"
        );
    }

    #[test]
    fn test_compute_cell_selection_rectangular_reverse() {
        let mut t = id!();
        let mut r1 = id!();
        let mut c1_1 = id!();
        let mut c1_2 = id!();
        let mut r2 = id!();
        let mut c2_1 = id!();
        let mut c2_2 = id!();

        let mut p_anchor = id!();
        let mut p_head = id!();

        let state = state! {
            doc {
                @t table {
                    @r1 table_row {
                        @c1_1 table_cell { paragraph {} }
                        @c1_2 table_cell { @p_anchor paragraph { text { "A" } } }
                    }
                    @r2 table_row {
                        @c2_1 table_cell { @p_head paragraph { text { "B" } } }
                        @c2_2 table_cell { paragraph {} }
                    }
                    table_row {
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                    }
                }
            }
            selection { (p_anchor, 0) -> (p_head, 0) }
        };

        let cell_selection = compute_structure_selection(&state.doc, &state.selection);

        match cell_selection {
            StructureSelectionInfo::Rectangular { table_id, range } => {
                assert_eq!(table_id, t);
                assert_eq!(range.0, (0, 1), "Row range mismatch");
                assert_eq!(range.1, (0, 1), "Col range mismatch");
            }
            _ => panic!("Expected Rectangular selection, got {:?}", cell_selection),
        }
    }

    #[test]
    fn test_compute_cell_selection_full_table() {
        let mut t = id!();
        let mut p_before = id!();
        let mut p_after = id!();

        let state = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @t table {
                    table_row {
                        table_cell { paragraph { text { "cell" } } }
                    }
                }
                @p_after paragraph { text { "after" } }
            }
            selection { (p_before, 0) -> (p_after, 5) }
        };

        let cell_selection = compute_structure_selection(&state.doc, &state.selection);

        match cell_selection {
            StructureSelectionInfo::Structural(tables) => {
                assert_eq!(tables.len(), 1);
                assert_eq!(tables[0], t);
            }
            _ => panic!("Expected FullTables selection, got {:?}", cell_selection),
        }
    }

    #[test]
    fn test_compute_cell_selection_internal_to_external() {
        let mut t1 = id!();
        let mut c1 = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                @t1 table {
                    table_row {
                        @c1 table_cell {
                            @p1 paragraph { text { "Cell" } }
                        }
                    }
                }
                @p2 paragraph { text { "Outside" } }
            }
            selection { (p1, 0) -> (p2, 2) }
        };

        let cell_selection = compute_structure_selection(&state.doc, &state.selection);

        match cell_selection {
            StructureSelectionInfo::Structural(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], t1);
            }
            _ => panic!("Expected FullTables selection, got {:?}", cell_selection),
        }
    }

    #[test]
    fn test_collect_selected_block_ids_returns_empty_for_collapsed_selection() {
        let doc = doc! {
            image()
            paragraph { text { "hello" } }
        };

        let selection = Selection::collapsed(Position::new(NodeId::ROOT, 0, Affinity::Downstream));
        let structure_selection = compute_structure_selection(&doc, &selection);
        let block_ids = collect_selected_block_ids(&doc, &selection, &structure_selection);

        assert!(block_ids.is_empty());
    }

    #[test]
    fn test_reproduce_table_selection_bug() {
        let mut n1 = id!();
        let mut n2 = id!();
        let mut p_after = id!();

        let state = state! {
            doc {
                paragraph {}
                table {
                    table_row {
                        table_cell {
                            paragraph {
                                text { "1" }
                            }
                        }
                        table_cell {
                            @n1 paragraph {
                                text { "2" }
                            }
                        }
                    }
                    table_row {
                        table_cell {
                            paragraph {
                                text { "3" }
                            }
                        }
                        table_cell {
                            @n2 paragraph {
                                text { "4" }
                            }
                        }
                    }
                }
                @p_after paragraph {
                    text { "ㅁ" }
                }
            }
            selection { (n1, 1) -> (n2, 1) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let after_decor = decorations.iter().find(|d| d.node_id() == p_after);

        assert!(
            after_decor.is_none(),
            "Paragraph after table should NOT have selection decoration, but found: {:?}",
            after_decor
        );
    }

    #[test]
    fn test_compute_cell_selection_full_table_is_rectangular() {
        let mut t = id!();
        let mut p_start = id!();
        let mut p_end = id!();

        let state = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p_start paragraph { text { "A" } } }
                        table_cell { paragraph { text { "B" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "C" } } }
                        table_cell { @p_end paragraph { text { "D" } } }
                    }
                }
            }
            selection { (p_start, 0) -> (p_end, 1) }
        };

        let cell_selection = compute_structure_selection(&state.doc, &state.selection);

        match cell_selection {
            StructureSelectionInfo::Rectangular { table_id, range } => {
                assert_eq!(table_id, t);
                assert_eq!(range.0, (0, 1), "Row range mismatch");
                assert_eq!(range.1, (0, 1), "Col range mismatch");
            }
            _ => panic!("Expected Rectangular selection, got {:?}", cell_selection),
        }
    }

    #[test]
    fn test_reproduce_table_cell_partial_selection_decorated_fully() {
        let mut p1 = id!();
        let state = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            @p1 paragraph {
                                text { "Hello World" }
                            }
                        }
                    }
                }
            }
            selection { (p1, 2) -> (p1, 4) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let p1_decors: Vec<_> = decorations.iter().filter(|d| d.node_id() == p1).collect();

        assert_eq!(
            p1_decors.len(),
            1,
            "Should have exactly 1 decoration for paragraph"
        );
        let d = p1_decors[0];
        assert_eq!(d.start_offset(), 2, "Start offset mismatch");
        assert_eq!(d.end_offset(), 4, "End offset mismatch");
    }

    #[test]
    fn test_fold_structure_selection_skips_inner_text() {
        let mut fold_id = id!();
        let mut p1 = id!();
        let mut p_before = id!();
        let mut p_after = id!();

        let state = state! {
            doc {
                @p_before paragraph { text { "before" } }
                @fold_id fold {
                    fold_title { text { "Title" } }
                    fold_content {
                        @p1 paragraph { text { "Inside" } }
                    }
                }
                @p_after paragraph { text { "after" } }
            }
            selection { (p_before, 0) -> (p_after, 5) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let fold_decor = decorations
            .iter()
            .find(|d| matches!(d, SelectionDecor::Block { node_id } if *node_id == fold_id));
        assert!(fold_decor.is_some(), "Fold should have Fold decoration");

        let p1_decor = decorations.iter().find(|d| d.node_id() == p1);
        assert!(
            p1_decor.is_none(),
            "Inner paragraph should NOT have text decoration when Fold is structurally selected"
        );
    }

    #[test]
    fn test_build_selection_decorations_adds_container_block_for_cross_child_selection() {
        let mut callout_id = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let state = state! {
            doc {
                @callout_id callout {
                    @p1 paragraph { text { "A" } }
                    @p2 paragraph { text { "B" } }
                }
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let callout_decor = decorations
            .iter()
            .find(|d| matches!(d, SelectionDecor::Block { node_id } if *node_id == callout_id));
        assert!(
            callout_decor.is_some(),
            "Cross-child selection should add a Block decoration for the shared container"
        );
    }

    #[test]
    fn test_repro_outer_fold_selection_bug() {
        let mut n1 = id!();
        let mut n2 = id!();
        let mut inner_fold = id!();
        let mut outer_fold = id!();

        let state = state! {
            doc {
                @outer_fold fold {
                    fold_title { text { "Outer" } }
                    fold_content {
                        @n1 paragraph {
                            text { "1" }
                        }
                        @inner_fold fold {
                            @n2 fold_title {}
                            fold_content {
                                paragraph {
                                    text { "2" }
                                }
                            }
                        }
                        paragraph {
                            text { "3" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (n1, 1) -> (n2, 0) }
        };

        let blocks = collect_relevant_blocks(&state.doc, &state.selection).unwrap();

        assert!(
            blocks.contains(&inner_fold),
            "Inner Fold SHOULD be collected"
        );
        assert!(
            !blocks.contains(&outer_fold),
            "Outer Fold SHOULD NOT be collected"
        );
    }

    #[test]
    fn test_reproduce_fold_list_selection_bug() {
        let mut n1 = id!();
        let mut p_nested = id!();
        let mut p_first = id!();

        let state = state! {
            doc {
                fold {
                    fold_title {}
                    @n1 fold_content {
                        @p_first paragraph {
                            text { "1" }
                        }
                        bullet_list {
                            list_item {
                                @p_nested paragraph {
                                    text { "2" }
                                }
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) -> (n1, 2) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let p_first_decor = decorations.iter().find(|d| d.node_id() == p_first);
        assert!(
            p_first_decor.is_some(),
            "First paragraph should have decoration"
        );

        let p_nested_decor = decorations.iter().find(|d| d.node_id() == p_nested);
        assert!(
            p_nested_decor.is_some(),
            "Nested paragraph in list should have decoration, but found: {:?}",
            decorations
        );
    }

    #[test]
    fn compute_selection_attrs_includes_cascade_attrs_from_empty_textblocks() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph { text { "hello" } }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let state = tr.commit().unwrap().0;

        let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
        let block_ids = collect_blocks_in_range(&state.doc, from, to).unwrap();
        let attrs = compute_selection_attrs(&state.doc, &block_ids, from, to);

        let weights = attrs.style_values.get(&StyleType::FontWeight);
        assert!(
            weights.is_some(),
            "Selection attrs should contain FontWeight"
        );
        assert!(
            weights
                .unwrap()
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Selection attrs should contain FontWeight(700)"
        );
        assert!(
            !attrs.absent_styles.contains(&StyleType::FontWeight),
            "FontWeight should not be absent"
        );
    }

    #[test]
    fn compute_selection_attrs_includes_cascade_attrs_from_all_empty_textblocks() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph {}
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let mut tr = Transaction::new(&state);
        tr.set_style(Style::FontWeight(FontWeightStyle { weight: 700 }))
            .unwrap();
        let state = tr.commit().unwrap().0;

        let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
        let block_ids = collect_blocks_in_range(&state.doc, from, to).unwrap();
        let attrs = compute_selection_attrs(&state.doc, &block_ids, from, to);

        let weights = attrs.style_values.get(&StyleType::FontWeight);
        assert!(
            weights.is_some(),
            "Selection attrs should contain FontWeight from cascade_attrs"
        );
        assert!(
            weights
                .unwrap()
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
            "Selection attrs should contain FontWeight(700) from cascade_attrs"
        );
    }
}

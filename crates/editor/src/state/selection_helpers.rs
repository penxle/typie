use crate::model::{
    Doc, Mark, MarkType, Node, NodeId, NodeRef, NodeType, SelectionDecor, TextAlign,
};
use crate::state::position::Position;
use crate::state::position_helpers::compare_positions;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{BlockTraverser, Selection};
use anyhow::{Context, Result};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::{Ordering, max, min};

pub fn collect_blocks_in_range(doc: &Doc, from: Position, to: Position) -> Result<Vec<NodeId>> {
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
            if end == current || is_ancestor(doc, current, end) {
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

    if pos.offset == 0 && !matches!(block_node.node(), Node::Root(_)) {
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

        let mut traverser = BlockTraverser::new(doc, target_block)
            .context("end_boundary_node: Traverser init failed")?;
        return Ok(traverser.next());
    }

    let mut traverser =
        BlockTraverser::new(doc, block_id).context("end_boundary_node: Traverser init failed")?;
    Ok(traverser.next())
}

fn is_ancestor(doc: &Doc, ancestor: NodeId, node: NodeId) -> bool {
    let mut current = doc.get_parent_id(node);
    while let Some(parent) = current {
        if parent == ancestor {
            return true;
        }
        current = doc.get_parent_id(parent);
    }
    false
}

pub fn block_content_len(node: &NodeRef<'_>) -> usize {
    node.children().map(|child| child.node().len()).sum()
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

    let cell_selection_info = compute_cell_selection(doc, selection);
    let mut processed_cells = FxHashSet::default();

    decorations.extend(collect_cell_decorations(
        doc,
        &cell_selection_info,
        &mut processed_cells,
    ));

    let block_ids = match block_ids {
        Some(ids) => ids.to_vec(),
        None => collect_selected_block_ids(doc, selection, &cell_selection_info),
    };

    let block_id_set: FxHashSet<NodeId> = block_ids.iter().cloned().collect();

    for &block_id in &block_ids {
        let Some(block) = doc.node(block_id) else {
            continue;
        };

        if !block.spec().is_textblock(doc.schema()) {
            continue;
        }

        if should_skip_block_decoration(doc, block, &processed_cells) {
            continue;
        }

        let block_len = block_content_len(&doc.node(block_id).unwrap()).max(1);
        let (start_offset, end_offset) = calculate_block_offsets(block_id, block_len, from, to);

        decorations.push(SelectionDecor::Text {
            node_id: block_id,
            start_offset,
            end_offset,
        });
    }

    add_ancestor_decorations(
        doc,
        from,
        to,
        &cell_selection_info,
        &block_id_set,
        &mut decorations,
    );

    decorations
}

fn collect_cell_decorations(
    doc: &Doc,
    cell_selection: &CellSelectionInfo,
    processed_cells: &mut FxHashSet<NodeId>,
) -> Vec<SelectionDecor> {
    let mut decorations = Vec::new();
    match cell_selection {
        CellSelectionInfo::Rectangular { table_id, range } => {
            if let Some(table) = doc.node(*table_id) {
                for (r_idx, row) in table.children().enumerate() {
                    if r_idx < range.0.0 || r_idx > range.0.1 {
                        continue;
                    }

                    for (c_idx, cell) in row.children().enumerate() {
                        if c_idx < range.1.0 || c_idx > range.1.1 {
                            continue;
                        }

                        let cell_id = cell.node_id();
                        if processed_cells.insert(cell_id) {
                            decorations.push(SelectionDecor::Cell { node_id: cell_id });
                        }
                    }
                }
            }
        }
        CellSelectionInfo::FullTables(table_ids) => {
            for &table_id in table_ids {
                if let Some(table) = doc.node(table_id) {
                    for row in table.children() {
                        for cell in row.children() {
                            let cell_id = cell.node_id();
                            if processed_cells.insert(cell_id) {
                                decorations.push(SelectionDecor::Cell { node_id: cell_id });
                            }
                        }
                    }
                }
            }
        }
        CellSelectionInfo::None => {}
    }
    decorations
}

fn should_skip_block_decoration(
    doc: &Doc,
    block: NodeRef,
    processed_cells: &FxHashSet<NodeId>,
) -> bool {
    let mut current_id = Some(block.node_id());
    while let Some(id) = current_id {
        if processed_cells.contains(&id) {
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
    cell_selection: &CellSelectionInfo,
    processed_blocks: &FxHashSet<NodeId>,
    decorations: &mut Vec<SelectionDecor>,
) {
    let Some(from_node) = doc.node(from.node_id) else {
        return;
    };
    let Some(to_node) = doc.node(to.node_id) else {
        return;
    };

    let from_path: Vec<_> = std::iter::once(from.node_id)
        .chain(from_node.ancestors().map(|n| n.node_id()))
        .collect();
    let to_path: Vec<_> = std::iter::once(to.node_id)
        .chain(to_node.ancestors().map(|n| n.node_id()))
        .collect();

    for (from_idx, &ancestor_id) in from_path.iter().enumerate() {
        if processed_blocks.contains(&ancestor_id) {
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

        match cell_selection {
            CellSelectionInfo::Rectangular { table_id, .. } if *table_id == ancestor_id => break,
            CellSelectionInfo::FullTables(table_ids) if table_ids.contains(&ancestor_id) => break,
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
                // If to.offset is 0, the selection ends at the start of direct_child_id.
                // We should stop at the previous sibling.
                doc.node(direct_child_id)
                    .and_then(|node| node.prev_sibling().map(|n| n.node_id()))
            } else {
                Some(direct_child_id)
            }
        } else {
            // to_idx == 0 means 'to' is directly on ancestor.
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
            let child_len = child.node().len();

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
            decorations.push(SelectionDecor::Text {
                node_id: ancestor_id,
                start_offset,
                end_offset,
            });
        }

        break;
    }
}

pub fn compute_selection_aggregates(
    doc: &Doc,
    block_ids: &[NodeId],
    from: Position,
    to: Position,
) -> (
    usize,
    Option<TextAlign>,
    Option<f32>,
    Vec<Mark>,
    Vec<MarkType>,
) {
    let mut paragraph_count = 0usize;
    let mut uniform_align: Option<TextAlign> = None;
    let mut uniform_line_height: Option<f32> = None;
    let mut align_mixed = false;
    let mut line_height_mixed = false;

    let mut uniform_marks: Option<FxHashMap<MarkType, Mark>> = None;
    let mut all_types: FxHashSet<MarkType> = FxHashSet::default();

    for &block_id in block_ids {
        let Some(node) = doc.node(block_id) else {
            continue;
        };

        let block_len = block_content_len(&node);

        if let Node::Paragraph(p) = node.node() {
            paragraph_count += 1;

            if !align_mixed {
                if let Some(current) = uniform_align {
                    if current != p.align {
                        align_mixed = true;
                        uniform_align = None;
                    }
                } else {
                    uniform_align = Some(p.align);
                }
            }

            if !line_height_mixed {
                if let Some(current) = uniform_line_height {
                    if (current - p.line_height).abs() > f32::EPSILON {
                        line_height_mixed = true;
                        uniform_line_height = None;
                    }
                } else {
                    uniform_line_height = Some(p.line_height);
                }
            }
        }

        let (start_offset, end_offset) = calculate_block_offsets(block_id, block_len, from, to);

        accumulate_block_marks(
            &node,
            start_offset,
            end_offset,
            &mut uniform_marks,
            &mut all_types,
        );
    }

    let uniform_marks_vec: Vec<Mark> = uniform_marks
        .map(|u| u.into_values().collect())
        .unwrap_or_default();
    let uniform_types: FxHashSet<_> = uniform_marks_vec.iter().map(|m| m.as_type()).collect();
    let mixed_marks: Vec<_> = all_types.difference(&uniform_types).copied().collect();

    (
        paragraph_count,
        if align_mixed { None } else { uniform_align },
        if line_height_mixed {
            None
        } else {
            uniform_line_height
        },
        uniform_marks_vec,
        mixed_marks,
    )
}

fn accumulate_block_marks(
    block: &NodeRef<'_>,
    start_offset: usize,
    end_offset: usize,
    uniform: &mut Option<FxHashMap<MarkType, Mark>>,
    all_types: &mut FxHashSet<MarkType>,
) {
    let mut current_offset = 0;

    for child in block.children() {
        match child.node() {
            Node::Text(text_node) => {
                let text_len = text_node.text.char_len();
                let child_end = current_offset + text_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = child_end.min(end_offset);

                if overlap_start < overlap_end {
                    let local_start = overlap_start - current_offset;
                    let local_end = overlap_end - current_offset;

                    let rich_segments = text_node.text.get_rich_text_segments();
                    let mut seg_offset = 0;

                    for (segment_text, segment_marks) in rich_segments {
                        let segment_len = segment_text.chars().count();
                        let seg_end = seg_offset + segment_len;

                        let seg_overlap_start = seg_offset.max(local_start);
                        let seg_overlap_end = seg_end.min(local_end);

                        if seg_overlap_start < seg_overlap_end {
                            update_mark_sets(&segment_marks, uniform, all_types);
                        }

                        seg_offset = seg_end;
                    }
                }

                current_offset = child_end;
            }
            Node::HardBreak(_) => {
                current_offset += 1;
            }
            _ => {}
        }
    }
}

fn update_mark_sets(
    marks: &[Mark],
    uniform: &mut Option<FxHashMap<MarkType, Mark>>,
    all_types: &mut FxHashSet<MarkType>,
) {
    if let Some(u) = uniform {
        if marks.is_empty() {
            u.clear();
        } else {
            let segment_map: FxHashMap<MarkType, &Mark> =
                marks.iter().map(|m| (m.as_type(), m)).collect();

            u.retain(|mark_type, mark| segment_map.get(mark_type).map_or(false, |m| *m == mark));
        }
    } else {
        let mut initial = FxHashMap::default();
        for m in marks {
            initial.insert(m.as_type(), m.clone());
        }
        *uniform = Some(initial);
    }

    all_types.extend(marks.iter().map(|m| m.as_type()));
}

pub fn is_node_fully_selected(doc: &Doc, selection: &Selection, node_id: NodeId) -> Result<bool> {
    let (from, to) = selection.as_sorted(doc)?;

    let node = doc
        .node(node_id)
        .context("is_node_fully_selected: Node not found")?;
    let node_start = Position::new(node_id, 0, crate::types::Affinity::default());
    let node_end = Position::new(
        node_id,
        block_content_len(&node),
        crate::types::Affinity::default(),
    );

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
                .map(|node| filter(node.node()))
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
        if !is_ancestor(doc, root_id, node_id) {
            break;
        }
        blocks.push(node_id);
    }

    blocks
}

pub fn collect_selected_block_ids(
    doc: &Doc,
    selection: &Selection,
    cell_selection: &CellSelectionInfo,
) -> Vec<NodeId> {
    let Ok((from, to)) = selection.as_sorted(doc) else {
        return Vec::new();
    };

    match cell_selection {
        CellSelectionInfo::Rectangular { table_id, range } => {
            let mut ids = Vec::new();
            if let Some(table) = doc.node(*table_id) {
                for (r_idx, row) in table.children().enumerate() {
                    if r_idx < range.0.0 || r_idx > range.0.1 {
                        continue;
                    }
                    for (c_idx, cell) in row.children().enumerate() {
                        if c_idx < range.1.0 || c_idx > range.1.1 {
                            continue;
                        }
                        ids.extend(collect_all_blocks_in_subtree(doc, cell.node_id()));
                    }
                }
            }
            ids
        }
        CellSelectionInfo::FullTables(table_ids) => {
            let mut ids: FxHashSet<NodeId> = collect_blocks_in_range(doc, from, to)
                .unwrap_or_default()
                .into_iter()
                .collect();

            for &table_id in table_ids {
                ids.extend(collect_all_blocks_in_subtree(doc, table_id));
            }

            let mut result: Vec<NodeId> = ids.into_iter().collect();
            result.sort_by(|&a, &b| {
                let pos_a = Position::new(a, 0, crate::types::Affinity::default());
                let pos_b = Position::new(b, 0, crate::types::Affinity::default());
                compare_positions(doc, pos_a, pos_b).unwrap_or(Ordering::Equal)
            });
            result
        }
        CellSelectionInfo::None => collect_blocks_in_range(doc, from, to).unwrap_or_default(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellSelectionInfo {
    None,
    Rectangular {
        table_id: NodeId,
        range: ((usize, usize), (usize, usize)),
    },
    FullTables(Vec<NodeId>),
}

pub fn compute_cell_selection(doc: &Doc, selection: &Selection) -> CellSelectionInfo {
    let anchor_info = find_table_cell(doc, selection.anchor.node_id);
    let head_info = find_table_cell(doc, selection.head.node_id);

    match (anchor_info, head_info) {
        (Some((_, t1, r1, c1)), Some((_, t2, r2, c2))) if t1 == t2 => {
            if r1 == r2 && c1 == c2 {
                CellSelectionInfo::None
            } else {
                let start_row = min(r1, r2);
                let end_row = max(r1, r2);
                let start_col = min(c1, c2);
                let end_col = max(c1, c2);

                if let Some(table) = doc.node(t1) {
                    let num_rows = table.children().count();
                    let num_cols = table
                        .children()
                        .next()
                        .map(|row| row.children().count())
                        .unwrap_or(0);

                    if start_row == 0
                        && end_row == num_rows.saturating_sub(1)
                        && start_col == 0
                        && end_col == num_cols.saturating_sub(1)
                    {
                        return CellSelectionInfo::FullTables(vec![t1]);
                    }
                }

                CellSelectionInfo::Rectangular {
                    table_id: t1,
                    range: ((start_row, end_row), (start_col, end_col)),
                }
            }
        }
        _ => {
            let tables = collect_relevant_tables(doc, selection).unwrap_or_default();

            if tables.is_empty() {
                CellSelectionInfo::None
            } else {
                CellSelectionInfo::FullTables(tables)
            }
        }
    }
}

fn collect_relevant_tables(doc: &Doc, selection: &Selection) -> Result<Vec<NodeId>> {
    let mut table_ids = FxHashSet::default();

    if let Ok(traversed) =
        collect_nodes_in_selection(doc, selection, |node| matches!(node, Node::Table(_)))
    {
        table_ids.extend(traversed);
    }

    for &node_id in &[selection.anchor.node_id, selection.head.node_id] {
        let mut current_id = Some(node_id);
        while let Some(id) = current_id {
            if let Some(node) = doc.node(id) {
                if node.node_type() == NodeType::Table {
                    table_ids.insert(id);
                }
                current_id = node.parent().map(|n| n.node_id());
            } else {
                break;
            }
        }
    }

    let mut result: Vec<_> = table_ids
        .into_iter()
        .filter(|&id| {
            let fully_selected = is_node_fully_selected(doc, selection, id).unwrap_or(false);
            let contains_anchor =
                id == selection.anchor.node_id || is_ancestor(doc, id, selection.anchor.node_id);
            let contains_head =
                id == selection.head.node_id || is_ancestor(doc, id, selection.head.node_id);

            fully_selected || contains_anchor || contains_head
        })
        .collect();

    result.sort_by(|&a, &b| {
        let pos_a = Position::new(a, 0, crate::types::Affinity::default());
        let pos_b = Position::new(b, 0, crate::types::Affinity::default());
        compare_positions(doc, pos_a, pos_b).unwrap_or(Ordering::Equal)
    });

    Ok(result)
}

fn find_table_cell(doc: &Doc, node_id: NodeId) -> Option<(NodeId, NodeId, usize, usize)> {
    let mut current_id = node_id;

    loop {
        let Some(node) = doc.node(current_id) else {
            break;
        };

        if node.node_type() == NodeType::TableCell {
            let cell = node;
            let row = cell.parent()?;
            if row.node_type() != NodeType::TableRow {
                return None;
            }
            let table = row.parent()?;
            if table.node_type() != NodeType::Table {
                return None;
            }

            let row_idx = row.index()?;
            let col_idx = cell.index()?;

            return Some((cell.node_id(), table.node_id(), row_idx, col_idx));
        }
        if node.node_type() == NodeType::Table {
            break;
        }

        if let Some(parent) = node.parent() {
            current_id = parent.node_id();
        } else {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_selection_aggregates_with_list() {
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

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.add_mark(crate::model::Mark::FontWeight(
            crate::model::FontWeightMark { weight: 700 },
        ))
        .unwrap();
        let state = tr.commit().unwrap().0;

        let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
        let block_ids = collect_blocks_in_range(&state.doc, from, to).unwrap();

        let (_, _, _, uniform_marks, _) =
            compute_selection_aggregates(&state.doc, &block_ids, from, to);

        assert!(
            uniform_marks
                .iter()
                .any(|m| m.as_type() == crate::model::MarkType::FontWeight),
            "Should detect font weight mark in list item"
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
        let mut n2 = id!();
        let mut n3 = id!();

        let state = state! {
            doc {
                @n1 paragraph { text { "1" } }
                fold {
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
            fold_title_decor.is_some(),
            "FoldTitle should have a selection decoration"
        );
        assert_eq!(fold_title_decor.unwrap().start_offset(), 0);
        assert_eq!(fold_title_decor.unwrap().end_offset(), 1);

        let fold_content_decor = decorations.iter().find(|d| d.node_id() == n3);
        assert!(
            fold_content_decor.is_some(),
            "FoldContent should have a selection decoration"
        );
        assert_eq!(fold_content_decor.unwrap().start_offset(), 0);
        assert_eq!(fold_content_decor.unwrap().end_offset(), 1);
    }

    #[test]
    fn test_build_selection_decorations_single_horizontal_rule() {
        let state = state! {
            doc {
                paragraph { text { "before" } }
                horizontal_rule {}
                paragraph { text { "after" } }
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        let root_decor = decorations.iter().find(|d| d.node_id() == NodeId::ROOT);
        assert!(
            root_decor.is_some(),
            "ROOT should have a selection decoration when selecting a single HR"
        );
        let decor = root_decor.unwrap();
        assert_eq!(
            decor.start_offset(),
            1,
            "decoration should start at offset 1 (before HR)"
        );
        assert_eq!(
            decor.end_offset(),
            2,
            "decoration should end at offset 2 (after HR)"
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

        let cell_selection = compute_cell_selection(&state.doc, &state.selection);

        match cell_selection {
            CellSelectionInfo::Rectangular { table_id, range } => {
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

        let cell_selection = compute_cell_selection(&state.doc, &state.selection);

        match cell_selection {
            CellSelectionInfo::FullTables(tables) => {
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

        let cell_selection = compute_cell_selection(&state.doc, &state.selection);

        match cell_selection {
            CellSelectionInfo::FullTables(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], t1);
            }
            _ => panic!("Expected FullTables selection, got {:?}", cell_selection),
        }
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
    fn test_compute_cell_selection_rectangular_becomes_full_table() {
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

        let cell_selection = compute_cell_selection(&state.doc, &state.selection);

        match cell_selection {
            CellSelectionInfo::FullTables(tables) => {
                assert_eq!(tables.len(), 1);
                assert_eq!(tables[0], t);
            }
            _ => panic!("Expected FullTables selection, got {:?}", cell_selection),
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
}

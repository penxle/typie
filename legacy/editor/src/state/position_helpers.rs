use crate::model::{Doc, Node, NodeId, NodeRef, NodeType};
use crate::state::selection_helpers::{StructureSelectionInfo, compute_structure_selection};
use crate::state::{Position, Selection, block_content_len};
use crate::types::Affinity;
use anyhow::{Context, Result};
use std::cmp::Ordering;

pub fn find_child_at_offset(block: &NodeRef, offset: usize) -> Option<(NodeId, usize)> {
    let children: Vec<_> = block.children().collect();
    let mut current_offset = 0;

    for (i, child) in children.iter().enumerate() {
        let id = child.node_id();

        match child.node() {
            Some(Node::Text(text)) => {
                let text_len = text.text.char_len();
                if offset >= current_offset && offset < current_offset + text_len {
                    return Some((child.node_id(), offset - current_offset));
                }

                if offset == current_offset + text_len {
                    if let Some(next) = children.get(i + 1) {
                        return Some((next.node_id(), 0));
                    } else {
                        return Some((child.node_id(), text_len));
                    }
                }
                current_offset += text_len;
            }
            _ => {
                if offset == current_offset {
                    return Some((id, 0));
                } else if offset == current_offset + 1 {
                    if let Some(next) = children.get(i + 1) {
                        return Some((next.node_id(), 0));
                    } else if i == children.len() - 1 {
                        return Some((id, 1));
                    }
                }
                current_offset += 1;
            }
        }
    }

    None
}

pub fn is_inline_position(doc: &Doc, position: Position) -> bool {
    let Some(node) = doc.node(position.node_id) else {
        return true;
    };

    find_child_at_offset(&node, position.offset)
        .and_then(|(child_id, _)| doc.node(child_id))
        .map(|child| child.is_inline())
        .unwrap_or(true)
}

pub fn find_text_at_offset(
    doc: &Doc,
    block: &NodeRef,
    offset: usize,
) -> Option<(NodeId, usize, loro::LoroText)> {
    let (child_id, internal_offset) = find_child_at_offset(block, offset)?;
    let child = doc.node(child_id)?;
    match child.node() {
        Some(Node::Text(t)) => Some((child_id, internal_offset, t.text.into_loro_text())),
        _ => {
            // boundary case: text와 hard break 사이에서 text를 우선
            if offset > 0 {
                let (prev_child_id, prev_internal) = find_child_at_offset(block, offset - 1)?;
                let prev_child = doc.node(prev_child_id)?;
                match prev_child.node() {
                    Some(Node::Text(t)) => {
                        Some((prev_child_id, prev_internal + 1, t.text.into_loro_text()))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

pub fn is_block_position(doc: &Doc, position: Position) -> bool {
    !is_inline_position(doc, position)
}

pub fn calculate_offset_before_child(block: &NodeRef, target_child_id: NodeId) -> usize {
    let mut offset = 0;

    for child in block.children() {
        if child.node_id() == target_child_id {
            return offset;
        }

        match child.node() {
            Some(Node::Text(text)) => {
                offset += text.text.char_len();
            }
            _ => {
                offset += 1;
            }
        }
    }

    offset
}

pub fn eq_positions_ignoring_affinity(a: Position, b: Position) -> bool {
    a.node_id == b.node_id && a.offset == b.offset
}

pub fn compare_positions(doc: &Doc, a: Position, b: Position) -> Result<Ordering> {
    if eq_positions_ignoring_affinity(a, b) {
        return Ok(Ordering::Equal);
    }

    let path_a = position_path(doc, a)?;
    let path_b = position_path(doc, b)?;

    for (segment_a, segment_b) in path_a.iter().zip(path_b.iter()) {
        match segment_a.cmp(segment_b) {
            Ordering::Equal => continue,
            other => return Ok(other),
        }
    }

    Ok(path_a.len().cmp(&path_b.len()))
}

pub fn position_in_selection(doc: &Doc, pos: Position, selection: &Selection) -> bool {
    match compute_structure_selection(doc, selection) {
        StructureSelectionInfo::Rectangular { table_id, range } => {
            let ((r_start, r_end), (c_start, c_end)) = range;
            let mut current_id = Some(pos.node_id);

            while let Some(id) = current_id {
                let Some(node) = doc.node(id) else {
                    break;
                };

                if node.node_type() == Some(NodeType::TableCell) {
                    if let Some(row) = node.parent() {
                        if row.parent().map(|t| t.node_id()) == Some(table_id) {
                            let r_idx = row.index().unwrap_or(0);
                            let c_idx = node.index().unwrap_or(0);
                            return r_idx >= r_start
                                && r_idx <= r_end
                                && c_idx >= c_start
                                && c_idx <= c_end;
                        }
                    }
                } else if node.node_type() == Some(NodeType::Table) && id == table_id {
                    break;
                }

                current_id = node.parent().map(|n| n.node_id());
            }

            return false;
        }
        StructureSelectionInfo::Structural(block_ids) => {
            let mut current_id = Some(pos.node_id);
            while let Some(id) = current_id {
                if block_ids.contains(&id) {
                    return true;
                }

                let Some(node) = doc.node(id) else {
                    break;
                };
                current_id = node.parent().map(|n| n.node_id());
            }
        }
        _ => {}
    }

    let Ok((from, to)) = selection.as_sorted(doc) else {
        return false;
    };

    let after_start =
        compare_positions(doc, from, pos).map_or(false, |ord| ord != Ordering::Greater);
    let before_end = compare_positions(doc, pos, to).map_or(false, |ord| ord != Ordering::Greater);
    after_start && before_end
}

fn position_path(doc: &Doc, pos: Position) -> Result<Vec<usize>> {
    let mut path = doc
        .node(pos.node_id)
        .context("position_path: node not found")?
        .path();

    path.push(pos.offset);
    Ok(path)
}

pub fn leaf_block_start(node: &NodeRef<'_>) -> Option<Position> {
    if node.spec().map_or(false, |s| s.is_textblock(node.schema())) {
        return Some(Position::new(node.node_id(), 0, Affinity::Downstream));
    }

    if node.spec().map_or(false, |s| s.content.is_leaf()) || node.first_child().is_none() {
        let parent_id = node.parent_id()?;
        let idx = node.index()?;
        return Some(Position::new(parent_id, idx, Affinity::Downstream));
    }

    let child = node.first_child()?;
    leaf_block_start(&child)
}

pub fn leaf_block_end(node: &NodeRef<'_>) -> Option<Position> {
    if node.spec().map_or(false, |s| s.is_textblock(node.schema())) {
        return Some(Position::new(
            node.node_id(),
            block_content_len(node),
            Affinity::Upstream,
        ));
    }

    if node.spec().map_or(false, |s| s.content.is_leaf()) || node.last_child().is_none() {
        let parent_id = node.parent_id()?;
        let idx = node.index()?;
        return Some(Position::new(parent_id, idx + 1, Affinity::Upstream));
    }

    let child = node.last_child()?;
    leaf_block_end(&child)
}

pub fn move_from_block_position(doc: &Doc, position: Position, go_forward: bool) -> Position {
    let Some(node) = doc.node(position.node_id) else {
        return position;
    };

    if node.spec().map_or(false, |s| s.is_textblock(node.schema())) {
        return position;
    }

    let children: Vec<_> = node.children().collect();

    if go_forward {
        if position.offset < children.len() {
            if let Some(pos) = leaf_block_start(&children[position.offset]) {
                return pos;
            }
        }

        if let Some(next_pos) = find_next_cursor_position_forward(doc, node.node_id()) {
            return next_pos;
        }

        if let Some(last) = children.last() {
            if let Some(pos) = leaf_block_end(last) {
                return pos;
            }
        }

        if let Some(root) = doc.node(NodeId::ROOT) {
            if let Some(pos) = leaf_block_end(&root) {
                return pos;
            }
        }
        return position;
    } else {
        if position.offset > 0 && !children.is_empty() {
            let child_idx = (position.offset - 1).min(children.len() - 1);
            if let Some(pos) = leaf_block_end(&children[child_idx]) {
                return pos;
            }
        }

        if let Some(prev_pos) = find_prev_cursor_position_backward(doc, node.node_id()) {
            return prev_pos;
        }

        if let Some(first) = children.first() {
            if let Some(pos) = leaf_block_start(first) {
                return pos;
            }
        }

        if let Some(root) = doc.node(NodeId::ROOT) {
            if let Some(pos) = leaf_block_start(&root) {
                return pos;
            }
        }
        return position;
    }
}

fn find_next_cursor_position_forward(doc: &Doc, node_id: NodeId) -> Option<Position> {
    use crate::state::BlockTraverser;

    let mut traverser = BlockTraverser::new_after_subtree(doc, node_id).ok()?;
    let next_block = traverser.next()?;
    let node = doc.node(next_block)?;
    leaf_block_start(&node)
}

fn find_prev_cursor_position_backward(doc: &Doc, node_id: NodeId) -> Option<Position> {
    use crate::state::BlockTraverser;

    let mut traverser = BlockTraverser::new_before_subtree(doc, node_id).ok()?;
    let prev_block = traverser.prev()?;
    let node = doc.node(prev_block)?;
    leaf_block_end(&node)
}

pub fn get_surrounding_text(
    doc: &Doc,
    position: Position,
    preceding_count: usize,
    following_count: usize,
) -> (String, String) {
    let Some(block) = doc.node(position.node_id) else {
        return (String::new(), String::new());
    };

    let children: Vec<_> = block.children().collect();
    if children.is_empty() {
        return (String::new(), String::new());
    }

    // Build cumulative offset map (only calls char_len = O(1), not as_str)
    let mut child_lens: Vec<usize> = Vec::with_capacity(children.len());
    let mut child_starts: Vec<usize> = Vec::with_capacity(children.len());
    let mut cumulative = 0usize;
    for child in &children {
        child_starts.push(cumulative);
        let len = child.node().map_or(1, |n| n.len());
        child_lens.push(len);
        cumulative += len;
    }

    let pos_offset = position.offset.min(cumulative);

    // Find the child containing the cursor
    let cursor_idx = child_starts
        .iter()
        .rposition(|&start| start <= pos_offset)
        .unwrap_or(0);
    let internal_offset = pos_offset - child_starts[cursor_idx];

    // --- Preceding ---
    let preceding = if preceding_count > 0 {
        let mut chunks: Vec<String> = Vec::new();
        let mut remaining = preceding_count;

        // Portion from cursor child (chars before internal_offset)
        if internal_offset > 0 {
            if let Some(Node::Text(t)) = children[cursor_idx].node() {
                let s = t.text.as_str();
                if internal_offset <= remaining {
                    chunks.push(s.chars().take(internal_offset).collect());
                    remaining -= internal_offset;
                } else {
                    let skip = internal_offset - remaining;
                    chunks.push(s.chars().skip(skip).take(remaining).collect());
                    remaining = 0;
                }
            }
        }

        // Walk backwards through consecutive text nodes
        let mut i = cursor_idx;
        while i > 0 && remaining > 0 {
            i -= 1;
            match children[i].node() {
                Some(Node::Text(t)) => {
                    let char_len = child_lens[i];
                    if char_len <= remaining {
                        chunks.push(t.text.as_str());
                        remaining -= char_len;
                    } else {
                        let s = t.text.as_str();
                        let skip = char_len - remaining;
                        chunks.push(s.chars().skip(skip).collect());
                        remaining = 0;
                    }
                }
                _ => break,
            }
        }

        chunks.reverse();
        chunks.concat()
    } else {
        String::new()
    };

    // --- Following ---
    let following = if following_count > 0 {
        let mut result = String::new();
        let mut remaining = following_count;

        // Portion from cursor child (chars after internal_offset)
        if let Some(Node::Text(t)) = children[cursor_idx].node() {
            let char_len = child_lens[cursor_idx];
            let after = char_len - internal_offset;
            if after > 0 {
                let s = t.text.as_str();
                let take = after.min(remaining);
                let chunk: String = s.chars().skip(internal_offset).take(take).collect();
                remaining -= chunk.chars().count();
                result.push_str(&chunk);
            }
        } else {
            return (preceding, String::new());
        }

        // Walk forwards through consecutive text nodes
        let mut i = cursor_idx + 1;
        while i < children.len() && remaining > 0 {
            match children[i].node() {
                Some(Node::Text(t)) => {
                    let char_len = child_lens[i];
                    if char_len <= remaining {
                        result.push_str(&t.text.as_str());
                        remaining -= char_len;
                    } else {
                        let s = t.text.as_str();
                        let chunk: String = s.chars().take(remaining).collect();
                        remaining -= chunk.chars().count();
                        result.push_str(&chunk);
                    }
                }
                _ => break,
            }
            i += 1;
        }

        result
    } else {
        String::new()
    };

    (preceding, following)
}

#[cfg(test)]
mod tests {

    use crate::{
        model::NodeId,
        state::selection_helpers::{StructureSelectionInfo, compute_structure_selection},
        state::{Position, get_surrounding_text, leaf_block_end, leaf_block_start},
        types::Affinity,
    };

    #[test]
    fn test_leaf_block_helpers_atomic_node() {
        let mut img = id!();
        let doc = doc! {
            @img image(id: Some("test-image-id".to_string()),) {}
        };

        let root_id = NodeId::ROOT;
        let image = doc.node(img).unwrap();

        let start_pos = leaf_block_start(&image).unwrap();
        assert_eq!(start_pos, Position::new(root_id, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&image).unwrap();
        assert_eq!(end_pos, Position::new(root_id, 1, Affinity::Upstream));
    }

    #[test]
    fn test_leaf_block_helpers_empty_container() {
        let mut bq = id!();
        let doc = doc! {
            @bq blockquote {}
        };

        let root_id = NodeId::ROOT;
        let blockquote = doc.node(bq).unwrap();

        let start_pos = leaf_block_start(&blockquote).unwrap();
        assert_eq!(start_pos, Position::new(root_id, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&blockquote).unwrap();
        assert_eq!(end_pos, Position::new(root_id, 1, Affinity::Upstream));
    }

    #[test]
    fn test_leaf_block_helpers_empty_container_with_child() {
        let mut bq = id!();
        let mut p = id!();
        let doc = doc! {
            @bq blockquote {
                @p paragraph { text { "A" } }
            }
        };

        let blockquote = doc.node(bq).unwrap();

        let start_pos = leaf_block_start(&blockquote).unwrap();
        assert_eq!(start_pos, Position::new(p, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&blockquote).unwrap();
        assert_eq!(end_pos, Position::new(p, 1, Affinity::Upstream));
    }

    #[test]
    fn test_leaf_block_helpers_paragraph() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "Hello" } }
        };

        let paragraph = doc.node(p).unwrap();

        let start_pos = leaf_block_start(&paragraph).unwrap();
        assert_eq!(start_pos, Position::new(p, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&paragraph).unwrap();
        assert_eq!(end_pos, Position::new(p, 5, Affinity::Upstream));
    }

    #[test]
    fn test_leaf_block_helpers_nested_container() {
        let mut bq = id!();
        let mut list = id!();
        let mut item = id!();
        let mut p = id!();

        let doc = doc! {
            @bq blockquote {
                @list bullet_list {
                    @item list_item {
                        @p paragraph { text { "Nested" } }
                    }
                }
            }
        };

        let blockquote = doc.node(bq).unwrap();

        let start_pos = leaf_block_start(&blockquote).unwrap();
        assert_eq!(start_pos, Position::new(p, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&blockquote).unwrap();
        assert_eq!(end_pos, Position::new(p, 6, Affinity::Upstream));
    }

    #[test]
    fn test_leaf_block_helpers_container_with_atomic_child() {
        let mut bq = id!();
        let mut img = id!();

        let doc = doc! {
            @bq blockquote {
                @img image(id: Some("test-image-id".to_string()),) {}
            }
        };

        let blockquote = doc.node(bq).unwrap();

        let start_pos = leaf_block_start(&blockquote).unwrap();
        assert_eq!(start_pos, Position::new(bq, 0, Affinity::Downstream));

        let end_pos = leaf_block_end(&blockquote).unwrap();
        assert_eq!(end_pos, Position::new(bq, 1, Affinity::Upstream));
    }

    #[test]
    fn test_position_in_selection_structural_full_table_includes_table_node_position() {
        let mut t = id!();
        let mut p = id!();
        let mut p_start = id!();
        let mut p_end = id!();

        let state = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p paragraph { text { "A" } } }
                        table_cell { paragraph { text { "B" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "C" } } }
                        table_cell { @p_start paragraph { text { "D" } } }
                    }
                }
                @p_end paragraph {}
            }
            selection { (p_start, 0) -> (p_end, 1) }
        };

        let structure_selection = compute_structure_selection(&state.doc, &state.selection);
        assert!(
            matches!(structure_selection, StructureSelectionInfo::Structural(ref ids) if ids == &vec![t]),
            "Expected full-table Structural selection, got {structure_selection:?}"
        );

        let table_start = Position::new(p, 0, Affinity::Downstream);
        assert!(
            super::position_in_selection(&state.doc, table_start, &state.selection),
            "Table node position should be considered inside full-table Structural selection"
        );

        let after_table = Position::new(p_end, 0, Affinity::Downstream);
        assert!(
            super::position_in_selection(&state.doc, after_table, &state.selection),
            "Range position outside block_ids should still be considered inside selection"
        );
    }

    #[test]
    fn get_surrounding_text_single_text_node() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "Hello" } }
        };

        let pos = Position::new(p, 2, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 10, 10);
        assert_eq!(pre, "He");
        assert_eq!(fol, "llo");
    }

    #[test]
    fn get_surrounding_text_multiple_consecutive_text_nodes() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph {
                text { "AB" }
                text { "CD" }
                text { "EF" }
            }
        };

        let pos = Position::new(p, 3, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 2, 2);
        assert_eq!(pre, "BC");
        assert_eq!(fol, "DE");
    }

    #[test]
    fn get_surrounding_text_stops_at_hard_break() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph {
                text { "AB" }
                hard_break {}
                text { "CD" }
            }
        };

        // Cursor within "CD" at internal offset 1
        let pos = Position::new(p, 4, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 10, 10);
        assert_eq!(pre, "C");
        assert_eq!(fol, "D");
    }

    #[test]
    fn get_surrounding_text_cursor_at_block_boundaries() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "ABC" } }
        };

        // At start
        let (pre, fol) =
            get_surrounding_text(&doc, Position::new(p, 0, Affinity::Downstream), 5, 5);
        assert_eq!(pre, "");
        assert_eq!(fol, "ABC");

        // At end
        let (pre, fol) =
            get_surrounding_text(&doc, Position::new(p, 3, Affinity::Downstream), 5, 5);
        assert_eq!(pre, "ABC");
        assert_eq!(fol, "");
    }

    #[test]
    fn get_surrounding_text_count_limits_result() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "ABCDEF" } }
        };

        let pos = Position::new(p, 3, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 2, 1);
        assert_eq!(pre, "BC");
        assert_eq!(fol, "D");
    }

    #[test]
    fn get_surrounding_text_empty_block() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph {}
        };

        let pos = Position::new(p, 0, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 5, 5);
        assert_eq!(pre, "");
        assert_eq!(fol, "");
    }

    #[test]
    fn get_surrounding_text_korean() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "안녕하세요" } }
        };

        let pos = Position::new(p, 2, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 10, 10);
        assert_eq!(pre, "안녕");
        assert_eq!(fol, "하세요");
    }

    #[test]
    fn get_surrounding_text_korean_across_text_nodes() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph {
                text { "가나" }
                text { "다라" }
                text { "마바" }
            }
        };

        let pos = Position::new(p, 3, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 2, 2);
        assert_eq!(pre, "나다");
        assert_eq!(fol, "라마");
    }

    #[test]
    fn get_surrounding_text_korean_with_count_limit() {
        let mut p = id!();
        let doc = doc! {
            @p paragraph { text { "가나다라마바사" } }
        };

        let pos = Position::new(p, 4, Affinity::Downstream);
        let (pre, fol) = get_surrounding_text(&doc, pos, 2, 2);
        assert_eq!(pre, "다라");
        assert_eq!(fol, "마바");
    }
}

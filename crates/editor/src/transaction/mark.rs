use crate::runtime::Effect;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{Position, block_content_len, calculate_block_offsets, collect_blocks_in_range};
use crate::transaction::Transaction;
use crate::{model::*, state::Selection};
use anyhow::{Context, Result};

pub(crate) fn get_marks_at_cursor(tr: &Transaction, position: &Position) -> Vec<Mark> {
    let Some(node) = tr.node(position.node_id) else {
        return Vec::new();
    };

    let Some((child_id, local_offset)) = find_child_at_offset(&node, position.offset) else {
        return Vec::new();
    };

    let Some(child) = tr.node(child_id) else {
        return Vec::new();
    };

    if let Node::Text(text_node) = child.node() {
        let segments = text_node.text.get_rich_text_segments();
        let mut current_offset = 0;

        for (segment_text, segment_marks) in segments {
            let segment_len = segment_text.chars().count();
            if local_offset > current_offset && local_offset <= current_offset + segment_len {
                return segment_marks;
            }
            if local_offset == 0 && current_offset == 0 {
                return segment_marks;
            }
            current_offset += segment_len;
        }
    }

    Vec::new()
}

fn apply_mark_to_range<F>(
    tr: &mut Transaction,
    from: Position,
    to: Position,
    mut apply_fn: F,
) -> Result<()>
where
    F: FnMut(&Text, std::ops::Range<usize>) -> Result<()>,
{
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node_mut(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let range = start_offset..end_offset;
            apply_fn(&text_node.text, range)?;
            tr.push_effect(Effect::NodeChanged {
                node_id: text_node_id,
            });
        }
    }

    Ok(())
}

fn check_range_has_mark(
    tr: &Transaction,
    from: Position,
    to: Position,
    mark: &Mark,
) -> Result<bool> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node(text_node_id).context("Text node not found")?;
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_rich_text_segments();

            let mut current_offset = 0;
            for (segment_text, segment_marks) in segments {
                let segment_len = segment_text.chars().count();
                let segment_end = current_offset + segment_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end {
                    if !segment_marks.contains(mark) {
                        return Ok(false);
                    }
                }

                current_offset = segment_end;
            }
        }
    }

    Ok(true)
}

fn range_contains_mark_type(
    tr: &Transaction,
    from: Position,
    to: Position,
    mark_type: MarkType,
) -> Result<bool> {
    let ranges = collect_text_ranges_in_selection(tr, from, to)?;

    for (text_node_id, start_offset, end_offset) in ranges {
        let Some(node) = tr.node(text_node_id) else {
            continue;
        };
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_rich_text_segments();
            let mut current_offset = 0;

            for (segment_text, segment_marks) in segments {
                let segment_len = segment_text.chars().count();
                let segment_end = current_offset + segment_len;
                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end
                    && segment_marks.iter().any(|m| m.as_type() == mark_type)
                {
                    return Ok(true);
                }

                current_offset = segment_end;
            }
        }
    }

    Ok(false)
}

fn get_common_mark_in_range(
    tr: &Transaction,
    from: Position,
    to: Position,
    mark_type: MarkType,
) -> Option<Mark> {
    let ranges = collect_text_ranges_in_selection(tr, from, to).ok()?;
    let mut common_mark: Option<Mark> = None;

    for (text_node_id, start_offset, end_offset) in ranges {
        let node = tr.node(text_node_id)?;
        if let Node::Text(text_node) = node.node() {
            let segments = text_node.text.get_rich_text_segments();
            let mut current_offset = 0;

            for (segment_text, segment_marks) in segments {
                let segment_len = segment_text.chars().count();
                let segment_end = current_offset + segment_len;

                let overlap_start = current_offset.max(start_offset);
                let overlap_end = segment_end.min(end_offset);

                if overlap_start < overlap_end {
                    let found = segment_marks.iter().find(|m| m.as_type() == mark_type);
                    match (found, &common_mark) {
                        (None, _) => return None,
                        (Some(m), None) => common_mark = Some(m.clone()),
                        (Some(m), Some(existing)) if existing != m => return None,
                        _ => {}
                    }
                }

                current_offset = segment_end;
            }
        }
    }

    common_mark
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

impl Transaction {
    pub fn add_mark(&mut self, mark: Mark) -> Result<bool> {
        let selection = self.selection().clone();

        if selection.is_collapsed() {
            let mut current = self
                .state
                .pending_marks
                .clone()
                .unwrap_or_else(|| get_marks_at_cursor(self, &selection.head));

            let mark_type = mark.as_type();
            current.retain(|m| m.as_type() != mark_type);
            current.push(mark);

            self.state.pending_marks = Some(current);
            self.push_effect(Effect::PendingMarksChanged);
            return Ok(true);
        }

        let (from, to) = selection.as_sorted(self.doc())?;
        apply_mark_to_range(self, from, to, |text, range| text.mark(range, &mark))?;

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn remove_mark(&mut self, mark: Mark) -> Result<bool> {
        let selection = self.selection().clone();

        if selection.is_collapsed() {
            let mut current = self
                .state
                .pending_marks
                .clone()
                .unwrap_or_else(|| get_marks_at_cursor(self, &selection.head));

            let mark_type = mark.as_type();
            current.retain(|m| m.as_type() != mark_type);

            self.state.pending_marks = Some(current);
            self.push_effect(Effect::PendingMarksChanged);
            return Ok(true);
        }

        let (from, to) = selection.as_sorted(self.doc())?;
        let mark_type = mark.as_type();
        apply_mark_to_range(self, from, to, |text, range| text.unmark(range, mark_type))?;

        Ok(true)
    }

    pub fn toggle_mark(&mut self, mark: Mark) -> Result<bool> {
        let selection = self.selection().clone();

        match &mark {
            Mark::FontFamily(fm) => {
                let weights = crate::global::get_available_font_weights(&fm.family);
                let weight = if let Some(&first) = weights.first() {
                    weights.iter().fold(first, |prev, &curr| {
                        if (curr as i32 - 400).abs() < (prev as i32 - 400).abs() {
                            curr
                        } else {
                            prev
                        }
                    })
                } else {
                    400
                };

                self.push_effect(Effect::FontUsageChanged {
                    family: fm.family.clone(),
                    weight,
                });
            }
            Mark::FontWeight(fw) => {
                let family = match self.get_mark_attributes(MarkType::FontFamily) {
                    Some(Mark::FontFamily(fm)) => fm.family,
                    _ => FontFamilyMark::default().family,
                };
                self.push_effect(Effect::FontUsageChanged {
                    family,
                    weight: fw.weight,
                });
            }
            _ => {}
        }

        if selection.is_collapsed() {
            let current = self
                .state
                .pending_marks
                .clone()
                .unwrap_or_else(|| get_marks_at_cursor(self, &selection.head));

            let mark_type = mark.as_type();
            let has_exact_mark = current.contains(&mark);

            let mut new_pending: Vec<Mark> = current
                .into_iter()
                .filter(|m| m.as_type() != mark_type)
                .collect();

            if !has_exact_mark {
                new_pending.push(mark);
            }

            self.state.pending_marks = Some(new_pending);
            self.push_effect(Effect::PendingMarksChanged);

            return Ok(true);
        }

        let (from, to) = selection.as_sorted(self.doc())?;
        let all_have_mark = check_range_has_mark(self, from.clone(), to.clone(), &mark)?;

        let add = !all_have_mark;
        let mark_type = mark.as_type();
        apply_mark_to_range(self, from, to, |text, range| {
            if add {
                text.mark(range, &mark)
            } else {
                text.unmark(range, mark_type)
            }
        })?;

        Ok(true)
    }

    pub fn extend_mark_range(&mut self, mark_type: MarkType) -> Result<bool> {
        let selection = self.selection().clone();
        let (from, to) = selection.as_sorted(self.doc())?;

        let has_mark_type = |marks: &[Mark]| marks.iter().any(|m| m.as_type() == mark_type);

        let cursor_has_mark = if selection.is_collapsed() {
            let marks = get_marks_at_cursor(self, &from);
            has_mark_type(&marks)
        } else {
            range_contains_mark_type(self, from.clone(), to.clone(), mark_type)?
        };

        if !cursor_has_mark {
            return Ok(false);
        }

        let paragraph = self
            .node(from.node_id)
            .context("extend_mark_range: Paragraph not found")?;

        let mut all_segments = Vec::new();
        let mut current_offset = 0;

        for child in paragraph.children() {
            match child.node() {
                Node::Text(text_node) => {
                    for (text, marks) in text_node.text.get_rich_text_segments() {
                        let len = text.chars().count();
                        all_segments.push((current_offset, current_offset + len, marks));
                        current_offset += len;
                    }
                }
                Node::HardBreak(_) => {
                    all_segments.push((current_offset, current_offset + 1, Vec::new()));
                    current_offset += 1;
                }
                _ => {}
            }
        }

        let cursor_segment_idx = all_segments.iter().position(|(seg_start, seg_end, marks)| {
            if from.offset > *seg_start && from.offset <= *seg_end {
                return has_mark_type(marks);
            }
            if from.offset == *seg_start {
                return has_mark_type(marks);
            }
            false
        });

        let cursor_segment_idx = cursor_segment_idx.or_else(|| {
            all_segments
                .iter()
                .position(|(_seg_start, seg_end, marks)| {
                    from.offset == *seg_end && has_mark_type(marks)
                })
        });

        let Some(cursor_idx) = cursor_segment_idx else {
            return Ok(false);
        };

        let mut start_idx = cursor_idx;
        while start_idx > 0 {
            let (_, _, marks) = &all_segments[start_idx - 1];
            if !has_mark_type(marks) {
                break;
            }
            start_idx -= 1;
        }

        let mut end_idx = cursor_idx;
        while end_idx < all_segments.len() - 1 {
            let (_, _, marks) = &all_segments[end_idx + 1];
            if !has_mark_type(marks) {
                break;
            }
            end_idx += 1;
        }

        let (mark_start, _, _) = all_segments[start_idx];
        let (_, mark_end, _) = all_segments[end_idx];

        if mark_start != from.offset || mark_end != to.offset {
            self.set_selection(Selection::new(
                Position::new(from.node_id, mark_start, Default::default()),
                Position::new(from.node_id, mark_end, Default::default()),
            ));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear_pending_marks(&mut self) -> Result<bool> {
        if self.state.pending_marks.is_some() {
            self.state.pending_marks = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn unset_all_marks(&mut self) -> Result<bool> {
        let selection = self.selection().clone();
        if selection.is_collapsed() {
            return Ok(false);
        }

        let (from, to) = selection.as_sorted(self.doc())?;
        apply_mark_to_range(self, from, to, |text, range| {
            for mark_type in MarkType::all() {
                text.unmark(range.clone(), mark_type)?;
            }
            Ok(())
        })?;

        Ok(true)
    }

    pub fn toggle_bold(&mut self) -> Result<bool> {
        let current_weight = match self.get_mark_attributes(MarkType::FontWeight) {
            Some(Mark::FontWeight(mark)) => Some(mark.weight),
            _ => None,
        };

        let family_name = match self.get_mark_attributes(MarkType::FontFamily) {
            Some(Mark::FontFamily(mark)) => mark.family.clone(),
            _ => FontFamilyMark::default().family,
        };

        let weights = crate::global::get_available_font_weights(&family_name);

        if weights.is_empty() {
            return Ok(false);
        }

        let find_closest_weight = |target: u16| -> u16 {
            weights.iter().fold(weights[0], |prev, &curr| {
                if (curr as i32 - target as i32).abs() < (prev as i32 - target as i32).abs() {
                    curr
                } else {
                    prev
                }
            })
        };

        let normal_weight = find_closest_weight(400);
        let bold_weight = find_closest_weight(700);

        if normal_weight == bold_weight {
            return Ok(false);
        }

        let target_weight = if current_weight.unwrap_or(normal_weight) < bold_weight {
            bold_weight
        } else {
            normal_weight
        };

        let mark = Mark::FontWeight(FontWeightMark {
            weight: target_weight,
        });
        self.push_effect(Effect::FontUsageChanged {
            family: family_name,
            weight: target_weight,
        });
        self.add_mark(mark)
    }

    pub fn get_mark_attributes(&self, mark_type: MarkType) -> Option<Mark> {
        let selection = self.selection();

        if selection.is_collapsed() {
            if let Some(pending) = &self.state.pending_marks {
                if let Some(mark) = pending.iter().find(|m| m.as_type() == mark_type) {
                    return Some(mark.clone());
                }
            }

            let marks = get_marks_at_cursor(self, &selection.head);
            if let Some(mark) = marks.iter().find(|m| m.as_type() == mark_type) {
                return Some(mark.clone());
            }
        } else if let Ok((from, to)) = selection.as_sorted(self.doc()) {
            return get_common_mark_in_range(self, from, to, mark_type);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::*,
        runtime::{Effect, Message},
        types::Affinity,
    };

    #[test]
    fn add_mark_to_partial_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello" => [italic()],
                        " world"
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_to_full_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_across_two_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_across_three_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                    text(marks: [italic()]) { " beautiful" }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_with_slot_at_start() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text {
                        "hel" => [italic()],
                        "lo"
                    }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_with_slot_at_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_with_slots_at_both_ends() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_across_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "beautiful" }
                }
                @p3 paragraph {
                    text { "world" }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he",
                        "llo" => [italic()]
                    }
                }
                @p2 paragraph {
                    text(marks: [italic()]) { "beautiful" }
                }
                @p3 paragraph {
                    text {
                        "wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_to_already_marked_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_to_multiple_text_nodes_full() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn add_mark_to_all_list_items() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text { "A" }
                        }
                    }
                    list_item {
                        @p2 paragraph {
                            text { "B" }
                        }
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr.add_mark(Mark::Italic(ItalicMark)).unwrap());

        let expected = state! {
            doc {
                bullet_list {
                    list_item {
                        @p1 paragraph {
                            text(marks: [italic()]) { "A" }
                        }
                    }
                    list_item {
                        @p2 paragraph {
                            text(marks: [italic()]) { "B" }
                        }
                    }
                }
                @p2 paragraph {}
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_from_partial_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello",
                        " world" => [italic()]
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_from_full_text_node() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_across_two_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he" => [italic()],
                        "llo"
                    }
                    text {
                        " wor",
                        "ld" => [italic()]
                    }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_across_three_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he" => [italic()],
                        "llo"
                    }
                    text { " beautiful" }
                    text {
                        " wor",
                        "ld" => [italic()]
                    }
                }
            }
            selection { (p, 2) -> (p, 19) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_with_slot_at_start() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text {
                        "hel",
                        "lo" => [italic()]
                    }
                }
            }
            selection { (p1, 0) -> (p2, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_with_slot_at_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he" => [italic()],
                        "llo"
                    }
                }
                @p2 paragraph { }
            }
            selection { (p1, 2) -> (p2, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_with_slots_at_both_ends() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_across_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p2 paragraph {
                    text(marks: [italic()]) { "beautiful" }
                }
                @p3 paragraph {
                    text(marks: [italic()]) { "world" }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text {
                        "he" => [italic()],
                        "llo"
                    }
                }
                @p2 paragraph {
                    text { "beautiful" }
                }
                @p3 paragraph {
                    text {
                        "wor",
                        "ld" => [italic()]
                    }
                }
            }
            selection { (p1, 2) -> (p3, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_from_text_without_mark() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn remove_mark_from_multiple_text_nodes_full() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .remove_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_all_marked_to_unmarked() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_all_unmarked_to_marked() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_marked_adds_to_unmarked() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_selection_all_marked() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello",
                        " world" => [italic()]
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_selection_all_unmarked() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "hello" => [italic()],
                        " world"
                    }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_mixed_marks_across_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_all_marked_multiple_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text(marks: [italic()]) { " beautiful" }
                    text(marks: [italic()]) { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { " beautiful" }
                    text { " world" }
                }
            }
            selection { (p, 0) -> (p, 21) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_with_slot_positions() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p3 paragraph { }
            }
            selection { (p1, 0) -> (p3, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_across_multiple_paragraphs_mixed() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p2 paragraph {
                    text { "beautiful" }
                }
                @p3 paragraph {
                    text(marks: [italic()]) { "world" }
                }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text(marks: [italic()]) { "hello" }
                }
                @p2 paragraph {
                    text(marks: [italic()]) { "beautiful" }
                }
                @p3 paragraph {
                    text(marks: [italic()]) { "world" }
                }
            }
            selection { (p1, 0) -> (p3, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_with_split() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text { " world" }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                    text {
                        " wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 2) -> (p, 9) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_single_text_with_adjacent_text_position() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "wor" }
                    text { "ld" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text(marks: [italic()]) { "wor" }
                    text { "ld" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_split_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text {
                        "wor" => [italic()],
                        "ld"
                    }
                }
            }
            selection { (p, 5) -> (p, 8) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_partial_split_text_multiple() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                    text { "world" }
                }
            }
            selection { (p, 2) -> (p, 4) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "he",
                        "ll" => [italic()],
                        "o"
                    }
                    text { "world" }
                }
            }
            selection { (p, 2) -> (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "world" }
                }
            }
            selection { (p1, 5) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .toggle_mark(Mark::Italic(ItalicMark))
            .unwrap());

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text(marks: [italic()]) { "world" }
                }
            }
            selection { (p1, 5) -> (p2, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn toggle_mark_collapsed_adds_to_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.toggle_mark(Mark::Italic(ItalicMark)).unwrap();
        let (view, _) = tr.commit().unwrap();

        let pending = view.pending_marks.as_ref().unwrap();
        assert!(pending.iter().any(|m| matches!(m, Mark::Italic(_))));
    }

    #[test]
    fn toggle_mark_collapsed_removes_from_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.toggle_mark(Mark::Italic(ItalicMark)).unwrap();
        let (view, _) = tr.commit().unwrap();

        let pending = view.pending_marks.as_ref().unwrap();
        assert!(!pending.iter().any(|m| matches!(m, Mark::Italic(_))));
    }

    #[test]
    fn add_mark_collapsed_sets_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.add_mark(Mark::FontWeight(FontWeightMark { weight: 700 }))
            .unwrap();
        let (view, _) = tr.commit().unwrap();

        let pending = view.pending_marks.as_ref().unwrap();
        assert!(
            pending
                .iter()
                .any(|m| matches!(m, Mark::FontWeight(fw) if fw.weight == 700))
        );
    }

    #[test]
    fn remove_mark_collapsed_removes_from_pending_marks() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text(marks: [italic(), font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 2) }
        };

        let mut tr = crate::transaction::Transaction::new(&state);
        tr.remove_mark(Mark::Italic(ItalicMark)).unwrap();
        let (view, _) = tr.commit().unwrap();

        let pending = view.pending_marks.as_ref().unwrap();
        assert!(!pending.iter().any(|m| matches!(m, Mark::Italic(_))));
        assert!(pending.iter().any(|m| matches!(m, Mark::FontWeight(_))));
    }

    #[test]
    fn toggle_bold() {
        let mut p = id!();

        let font_family = FontFamilyMark::default().family;
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family.clone(), vec![400, 700]);
        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let (bold_state, effects) = transact_with_effect!(initial, |tr| tr.toggle_bold().unwrap());

        assert!(effects.contains(&Effect::FontUsageChanged {
            family: font_family.clone(),
            weight: 700,
        }));

        let expected_bold = state! {
            doc {
                @p paragraph {
                    text(marks: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(bold_state, expected_bold);

        let (normal_state, effects) =
            transact_with_effect!(bold_state, |tr| tr.toggle_bold().unwrap());

        assert!(effects.contains(&Effect::FontUsageChanged {
            family: font_family.clone(),
            weight: 400,
        }));

        let expected_normal = state! {
            doc {
                @p paragraph {
                    text(marks: [font_weight(400)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };
        assert_state_eq!(normal_state, expected_normal);
    }

    #[test]
    fn toggle_bold_backward_selection() {
        let mut p = id!();

        let font_family = FontFamilyMark::default().family;
        let mut fonts = std::collections::HashMap::new();
        fonts.insert(font_family.clone(), vec![400, 700]);
        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

        let initial = state! {
            doc {
                @p paragraph {
                    text {
                        "가가가",
                        "가" => [font_weight(700)]
                    }
                }
            }
            selection { (p, 4) -> (p, 3) }
        };

        let (result_state, _) = transact_with_effect!(initial, |tr| tr.toggle_bold().unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "가가가가" }
                }
            }
            selection { (p, 4) -> (p, 3) }
        };

        assert_state_eq!(result_state, expected);
    }

    #[test]
    fn ruby_mark_does_not_extend_after_typing() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [ruby("ルビ")]) { "漢字" }
                }
            }
            selection { (p, 2) }
        };

        let actual = transact!(initial, |tr| tr.insert_text("追加").unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "漢字" => [ruby("ルビ")],
                        "追加"
                    }
                }
            }
            selection { (p, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn ruby_mark_does_not_extend_in_pending_marks() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [ruby("ふりがな")]) { "文字" }
                }
            }
            selection { (p, 2) }
        };

        let tr = crate::transaction::Transaction::new(&initial);
        let pending = tr.state.pending_marks.clone();

        if let Some(marks) = pending {
            assert!(
                !marks
                    .iter()
                    .any(|m| matches!(m, crate::model::Mark::Ruby(_))),
                "Ruby mark should not be in pending marks due to Expand::None"
            );
        }
    }

    #[test]
    fn test_mark_application_invalidates_layout_cache() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello World" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        rt.update(Message::ToggleItalic);

        assert!(
            !rt.is_layout_cached(p),
            "paragraph layout cache should be invalidated after applying mark"
        );
    }

    #[test]
    fn test_mark_removal_invalidates_layout_cache() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(marks: [italic()]) { "Hello World" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        rt.update(Message::ToggleItalic);

        assert!(
            !rt.is_layout_cached(p),
            "paragraph layout cache should be invalidated after removing mark"
        );
    }

    #[test]
    fn toggle_mark_with_missing_400_weight() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("ThinFont".to_string(), vec![100]);

        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.toggle_mark(Mark::FontFamily(FontFamilyMark {
                family: "ThinFont".to_string(),
            }))
            .unwrap()
        });

        let effect = effects
            .iter()
            .find(|e| matches!(e, Effect::FontUsageChanged { .. }))
            .expect("Effect::FontUsageChanged not found");

        if let Effect::FontUsageChanged { family, weight } = effect {
            assert_eq!(family, "ThinFont");
            assert_eq!(*weight, 100);
        } else {
            panic!("Unexpected effect");
        }
    }
    #[test]
    fn toggle_font_weight_collapsed_100_to_900() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [font_weight(100)]) { "hello" }
                }
            }
            selection { (p, 2) -> (p, 2) }
        };

        let actual_state = transact!(initial, |tr| {
            tr.toggle_mark(Mark::FontWeight(FontWeightMark { weight: 900 }))
                .unwrap()
        });

        let pending = actual_state
            .pending_marks
            .expect("Pending marks should be set");

        let has_900 = pending.contains(&Mark::FontWeight(FontWeightMark { weight: 900 }));
        assert!(
            has_900,
            "Should have switched to 900, but pending marks are: {:?}",
            pending
        );

        let has_100 = pending.contains(&Mark::FontWeight(FontWeightMark { weight: 100 }));
        assert!(!has_100, "Should have removed 100");
    }

    #[test]
    fn toggle_font_weight_collapsed_900_to_100() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(marks: [font_weight(900)]) { "hello" }
                }
            }
            selection { (p, 2) -> (p, 2) }
        };

        let actual_state = transact!(initial, |tr| {
            tr.toggle_mark(Mark::FontWeight(FontWeightMark { weight: 100 }))
                .unwrap()
        });

        let pending = actual_state
            .pending_marks
            .expect("Pending marks should be set");

        assert!(
            pending.contains(&Mark::FontWeight(FontWeightMark { weight: 100 })),
            "Should have switched to 100, marks: {:?}",
            pending
        );
        assert!(
            !pending.contains(&Mark::FontWeight(FontWeightMark { weight: 900 })),
            "Should have removed 900"
        );
    }

    #[test]
    fn select_all_and_toggle_italic() {
        let mut p1 = id!();
        let mut rt = runtime! {
          viewport { 800, 600, 1.0 }
          doc {
            @p1 paragraph { text { "hello" } }
            paragraph { text { "world" } }
          }
          selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);
        rt.update(Message::ToggleItalic);

        let mut ep1 = id!();
        let mut ep2 = id!();
        let expected = state! {
          doc {
            @ep1 paragraph { text(marks: [italic()]) { "hello" } }
            @ep2 paragraph { text(marks: [italic()]) { "world" } }
          }
          selection { (ep1, 0) -> (ep2, 5, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }
}

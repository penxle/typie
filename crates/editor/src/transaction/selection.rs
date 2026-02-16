use crate::model::{Doc, Node, NodeId, NodeRef};
use crate::state::position_helpers::{calculate_offset_before_child, find_child_at_offset};
use crate::state::{BlockTraverser, Position, Selection};
use crate::transaction::Transaction;
use crate::types::Affinity;
use crate::utils::{compute_sentence_boundaries, compute_word_boundaries};
use anyhow::{Context, Result};

/// Collect consecutive text sibling nodes around `child_id` in `block`,
/// build the concatenated text, and compute text-offset-to-block-offset mappings.
/// Returns (full_text, node_char_offsets, global_target_offset).
fn collect_text_context(
    doc: &Doc,
    block: &NodeRef<'_>,
    child_id: NodeId,
    local_offset: usize,
) -> Option<(String, Vec<(NodeId, usize, usize, usize)>, usize)> {
    let child = doc.node(child_id)?;
    if !matches!(child.node(), Node::Text(_)) {
        return None;
    }

    let mut ordered_text_nodes = Vec::new();

    let mut current = child.node_id();
    while let Some(prev) = doc.node(current)?.prev_sibling() {
        if matches!(prev.node(), Node::Text(_)) {
            ordered_text_nodes.insert(0, prev.node_id());
            current = prev.node_id();
        } else {
            break;
        }
    }

    ordered_text_nodes.push(child.node_id());

    let mut current = child.node_id();
    while let Some(next) = doc.node(current)?.next_sibling() {
        if matches!(next.node(), Node::Text(_)) {
            ordered_text_nodes.push(next.node_id());
            current = next.node_id();
        } else {
            break;
        }
    }

    let mut node_char_offsets = Vec::new();
    let mut accumulated = 0;
    let mut full_text = String::new();

    for nid in &ordered_text_nodes {
        let n = doc.node(*nid)?;
        let Node::Text(text_node) = n.node() else {
            return None;
        };
        let len = text_node.text.char_len();
        let block_start = calculate_offset_before_child(block, *nid);
        full_text.push_str(&text_node.text.as_str());
        node_char_offsets.push((*nid, accumulated, accumulated + len, block_start));
        accumulated += len;
    }

    let global_target = node_char_offsets
        .iter()
        .find(|(id, _, _, _)| *id == child_id)
        .map(|(_, text_start, _, _)| text_start + local_offset)?;

    Some((full_text, node_char_offsets, global_target))
}

/// Convert a global text offset back to a block-relative offset.
fn global_to_block_offset(
    node_char_offsets: &[(NodeId, usize, usize, usize)],
    global_offset: usize,
) -> Option<usize> {
    node_char_offsets
        .iter()
        .find(|(_, text_start, text_end, _)| {
            global_offset >= *text_start && global_offset <= *text_end
        })
        .map(|(_, text_start, _, block_start)| block_start + (global_offset - text_start))
}

/// Compute word boundaries at `position` as block-relative (start, end) offsets.
/// Returns None if position is not in a text node.
pub fn word_range_at(doc: &Doc, position: Position) -> Option<(usize, usize)> {
    let block = doc.node(position.node_id)?;
    let (child_id, local_offset) = find_child_at_offset(&block, position.offset)?;
    let (full_text, offsets, global_target) =
        collect_text_context(doc, &block, child_id, local_offset)?;
    let (ws, we) = find_word_boundaries_in_text(&full_text, global_target)?;
    Some((
        global_to_block_offset(&offsets, ws)?,
        global_to_block_offset(&offsets, we)?,
    ))
}

/// Compute sentence boundaries at `position` as block-relative (start, end) offsets.
pub fn sentence_range_at(doc: &Doc, position: Position) -> Option<(usize, usize)> {
    let block = doc.node(position.node_id)?;
    let (child_id, local_offset) = find_child_at_offset(&block, position.offset)?;
    let (full_text, offsets, global_target) =
        collect_text_context(doc, &block, child_id, local_offset)?;
    let (ss, se) = find_sentence_boundaries_in_text(&full_text, global_target)?;
    Some((
        global_to_block_offset(&offsets, ss)?,
        global_to_block_offset(&offsets, se)?,
    ))
}

/// Compute paragraph boundaries at `position` as (node_id, start_offset, end_offset).
pub fn paragraph_range_at(doc: &Doc, position: Position) -> Option<(NodeId, usize, usize)> {
    let node = doc.node(position.node_id)?;
    let paragraph = node
        .ancestors()
        .find(|n| matches!(n.node(), Node::Paragraph(_)))?;
    let last_child = paragraph.last_child()?;
    let Node::Text(last_text) = last_child.node() else {
        return None;
    };
    let end =
        calculate_offset_before_child(&paragraph, last_child.node_id()) + last_text.text.char_len();
    Some((paragraph.node_id(), 0, end))
}

fn find_sentence_boundaries_in_text(text: &str, char_offset: usize) -> Option<(usize, usize)> {
    let boundaries = compute_sentence_boundaries(text);

    let start_offset = boundaries
        .iter()
        .rev()
        .find(|&&boundary| boundary <= char_offset)
        .copied()
        .unwrap_or(0);

    let end_offset = boundaries
        .iter()
        .find(|&&boundary| boundary > char_offset)
        .copied()
        .unwrap_or(text.chars().count());

    // ICU sentence segmentation includes trailing whitespace in the segment.
    // Trim it so the selection ends at the last non-whitespace character.
    let chars: Vec<char> = text.chars().collect();
    let mut trimmed_end = end_offset;
    while trimmed_end > start_offset
        && chars
            .get(trimmed_end - 1)
            .is_some_and(|c| c.is_whitespace())
    {
        trimmed_end -= 1;
    }

    Some((start_offset, trimmed_end))
}

fn find_word_boundaries_in_text(text: &str, char_offset: usize) -> Option<(usize, usize)> {
    let boundaries = compute_word_boundaries(text);

    let start_offset = boundaries
        .iter()
        .rev()
        .find(|&&boundary| boundary <= char_offset)
        .copied()
        .unwrap_or(0);

    let end_offset = boundaries
        .iter()
        .find(|&&boundary| boundary > char_offset)
        .copied()
        .unwrap_or(text.chars().count());

    Some((start_offset, end_offset))
}

impl Transaction {
    pub fn select_word_at(&mut self, position: Position) -> Result<bool> {
        let block = self.node(position.node_id).context("Node not found")?;

        if let Some((child_id, local_offset)) = find_child_at_offset(&block, position.offset) {
            let child = self.node(child_id).context("Child not found")?;

            match child.node() {
                Node::Text(_) => {
                    let this_id = child_id;
                    let this_offset = local_offset;
                    let this = child;

                    let mut ordered_text_nodes = Vec::new();

                    let mut current_node_id = this.node_id();
                    while let Some(prev_sibling) = self
                        .doc()
                        .node(current_node_id)
                        .context("Node not found")?
                        .prev_sibling()
                    {
                        if let Node::Text(_) = prev_sibling.node() {
                            ordered_text_nodes.insert(0, prev_sibling.node_id());
                            current_node_id = prev_sibling.node_id();
                        } else {
                            break;
                        }
                    }

                    ordered_text_nodes.push(this.node_id());

                    let mut current_node_id = this.node_id();
                    while let Some(next_sibling) = self
                        .doc()
                        .node(current_node_id)
                        .context("Node not found")?
                        .next_sibling()
                    {
                        if let Node::Text(_) = next_sibling.node() {
                            ordered_text_nodes.push(next_sibling.node_id());
                            current_node_id = next_sibling.node_id();
                        } else {
                            break;
                        }
                    }

                    let mut node_char_offsets = Vec::new();
                    let mut accumulated_text_offset = 0;
                    let mut full_text = String::new();

                    for node_id in ordered_text_nodes {
                        let node = self.node(node_id).context("Text node not found")?;
                        let Node::Text(text_node) = node.node() else {
                            return Ok(false);
                        };

                        let len = text_node.text.char_len();
                        let text_start = accumulated_text_offset;
                        let text_end = accumulated_text_offset + len;
                        let block_start = calculate_offset_before_child(&block, node_id);

                        full_text.push_str(&text_node.text.as_str());
                        node_char_offsets.push((node_id, text_start, text_end, block_start));

                        accumulated_text_offset = text_end;
                    }

                    let global_target_offset = node_char_offsets
                        .iter()
                        .find(|(id, _, _, _)| *id == this_id)
                        .map(|(_, text_start, _, _)| text_start + this_offset)
                        .context("Cannot find target node in node_char_offsets")?;

                    let mut target_word_offset = global_target_offset;
                    let mut word_boundaries =
                        find_word_boundaries_in_text(&full_text, target_word_offset);

                    if let Some((start, end)) = word_boundaries {
                        if start == end && target_word_offset > 0 {
                            target_word_offset = target_word_offset.saturating_sub(1);
                            word_boundaries =
                                find_word_boundaries_in_text(&full_text, target_word_offset);
                        }
                    }

                    // Skip whitespace-only segments by falling back to the previous word
                    if let Some((start, end)) = word_boundaries {
                        let is_whitespace = full_text
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .all(|c| c.is_whitespace());
                        if is_whitespace && start > 0 {
                            word_boundaries = find_word_boundaries_in_text(&full_text, start - 1);
                        }
                    }

                    let (word_start_global_offset, word_end_global_offset) = word_boundaries
                        .ok_or_else(|| anyhow::anyhow!("Failed to find word boundaries"))?;

                    let mut anchor_global_offset = None;
                    let mut head_global_offset = None;

                    for (_, text_start, text_end, block_start) in &node_char_offsets {
                        if anchor_global_offset.is_none()
                            && word_start_global_offset >= *text_start
                            && word_start_global_offset <= *text_end
                        {
                            anchor_global_offset =
                                Some(block_start + (word_start_global_offset - text_start));
                        }

                        if head_global_offset.is_none()
                            && word_end_global_offset >= *text_start
                            && word_end_global_offset <= *text_end
                        {
                            head_global_offset =
                                Some(block_start + (word_end_global_offset - text_start));
                        }
                    }

                    let anchor_pos = anchor_global_offset
                        .map(|offset| Position::new(position.node_id, offset, Affinity::Downstream))
                        .context("Anchor position not found")?;
                    let head_pos = head_global_offset
                        .map(|offset| Position::new(position.node_id, offset, Affinity::Upstream))
                        .context("Head position not found")?;

                    self.set_selection(Selection::new(anchor_pos, head_pos));
                    return Ok(true);
                }
                _ if child.is_inline() => {
                    if local_offset == 0 {
                        let child_offset_before = calculate_offset_before_child(&block, child_id);
                        self.set_selection(Selection::new(
                            Position::new(
                                position.node_id,
                                child_offset_before,
                                Affinity::Downstream,
                            ),
                            Position::new(
                                position.node_id,
                                child_offset_before + 1,
                                Affinity::Upstream,
                            ),
                        ));
                        return Ok(true);
                    } else {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    pub fn select_sentence_at(&mut self, position: Position) -> Result<bool> {
        let block = self.node(position.node_id).context("Node not found")?;

        if let Some((child_id, local_offset)) = find_child_at_offset(&block, position.offset) {
            let child = self.node(child_id).context("Child not found")?;

            match child.node() {
                Node::Text(_) => {
                    let this_id = child_id;
                    let this_offset = local_offset;
                    let this = child;

                    let mut ordered_text_nodes = Vec::new();

                    let mut current_node_id = this.node_id();
                    while let Some(prev_sibling) = self
                        .doc()
                        .node(current_node_id)
                        .context("Node not found")?
                        .prev_sibling()
                    {
                        if let Node::Text(_) = prev_sibling.node() {
                            ordered_text_nodes.insert(0, prev_sibling.node_id());
                            current_node_id = prev_sibling.node_id();
                        } else {
                            break;
                        }
                    }

                    ordered_text_nodes.push(this.node_id());

                    let mut current_node_id = this.node_id();
                    while let Some(next_sibling) = self
                        .doc()
                        .node(current_node_id)
                        .context("Node not found")?
                        .next_sibling()
                    {
                        if let Node::Text(_) = next_sibling.node() {
                            ordered_text_nodes.push(next_sibling.node_id());
                            current_node_id = next_sibling.node_id();
                        } else {
                            break;
                        }
                    }

                    let mut node_char_offsets = Vec::new();
                    let mut accumulated_text_offset = 0;
                    let mut full_text = String::new();

                    for node_id in ordered_text_nodes {
                        let node = self.node(node_id).context("Text node not found")?;
                        let Node::Text(text_node) = node.node() else {
                            return Ok(false);
                        };

                        let len = text_node.text.char_len();
                        let text_start = accumulated_text_offset;
                        let text_end = accumulated_text_offset + len;
                        let block_start = calculate_offset_before_child(&block, node_id);

                        full_text.push_str(&text_node.text.as_str());
                        node_char_offsets.push((node_id, text_start, text_end, block_start));

                        accumulated_text_offset = text_end;
                    }

                    let global_target_offset = node_char_offsets
                        .iter()
                        .find(|(id, _, _, _)| *id == this_id)
                        .map(|(_, text_start, _, _)| text_start + this_offset)
                        .context("Cannot find target node in node_char_offsets")?;

                    let sentence_boundaries =
                        find_sentence_boundaries_in_text(&full_text, global_target_offset);

                    let (sentence_start_global_offset, sentence_end_global_offset) =
                        sentence_boundaries
                            .ok_or_else(|| anyhow::anyhow!("Failed to find sentence boundaries"))?;

                    let mut anchor_global_offset = None;
                    let mut head_global_offset = None;

                    for (_, text_start, text_end, block_start) in &node_char_offsets {
                        if anchor_global_offset.is_none()
                            && sentence_start_global_offset >= *text_start
                            && sentence_start_global_offset <= *text_end
                        {
                            anchor_global_offset =
                                Some(block_start + (sentence_start_global_offset - text_start));
                        }

                        if head_global_offset.is_none()
                            && sentence_end_global_offset >= *text_start
                            && sentence_end_global_offset <= *text_end
                        {
                            head_global_offset =
                                Some(block_start + (sentence_end_global_offset - text_start));
                        }
                    }

                    let anchor_pos = anchor_global_offset
                        .map(|offset| Position::new(position.node_id, offset, Affinity::Downstream))
                        .context("Anchor position not found")?;
                    let head_pos = head_global_offset
                        .map(|offset| Position::new(position.node_id, offset, Affinity::Upstream))
                        .context("Head position not found")?;

                    self.set_selection(Selection::new(anchor_pos, head_pos));
                    return Ok(true);
                }
                _ => return Ok(false),
            }
        }

        Ok(false)
    }

    pub fn select_paragraph_at(&mut self, position: Position) -> Result<bool> {
        let this = self.node(position.node_id).context("Node not found")?;
        let paragraph = match this
            .ancestors()
            .find(|n| matches!(n.node(), Node::Paragraph(_)))
        {
            Some(p) => p,
            None => return Ok(false),
        };

        let last_child = match paragraph.last_child() {
            Some(c) => c,
            None => return Ok(false),
        };

        let Node::Text(last_child_text) = last_child.node() else {
            return Ok(false);
        };

        let anchor_offset = 0;
        let last_child_offset_before =
            calculate_offset_before_child(&paragraph, last_child.node_id());
        let head_offset = last_child_offset_before + last_child_text.text.char_len();

        let anchor = Position::new(paragraph.node_id(), anchor_offset, Affinity::Downstream);
        let head = Position::new(paragraph.node_id(), head_offset, Affinity::Upstream);

        self.set_selection(Selection::new(anchor, head));

        Ok(true)
    }

    pub fn move_to_next_block(&mut self, from_block_id: NodeId) -> Result<()> {
        self.ensure_paragraph_after_pagebreak()?;

        let mut traverser = BlockTraverser::new(self.doc(), from_block_id)
            .context("move_to_next_block: Traverser init failed")?;

        let next_block_id = traverser.next().context(
            "move_to_next_block: Next block not found after ensure_paragraph_after_pagebreak",
        )?;
        let next_block = self
            .node(next_block_id)
            .context("move_to_next_block: Next block not found")?;

        if next_block.spec().selectable {
            if let (Some(parent), Some(index)) = (next_block.parent(), next_block.index()) {
                self.set_selection(Selection::new(
                    Position::new(parent.node_id(), index, Affinity::Downstream),
                    Position::new(parent.node_id(), index + 1, Affinity::Downstream),
                ));
            } else {
                self.set_selection(Selection::collapsed(Position::new(
                    next_block_id,
                    0,
                    Affinity::Downstream,
                )));
            }
        } else {
            self.set_selection(Selection::collapsed(Position::new(
                next_block_id,
                0,
                Affinity::Downstream,
            )));
        }

        Ok(())
    }

    pub fn collapse_selection(&mut self) -> Result<()> {
        let selection = *self.selection();
        let (_, to) = selection.as_sorted(self.doc())?;
        self.set_selection(Selection::collapsed(to));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_word_at() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .select_word_at(Position::new(p, 1, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_sentence_at() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .select_sentence_at(Position::new(p, 3, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 0) -> (p, 12, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_sentence_at_second_sentence() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 15) }
        };

        let actual = transact!(initial, |tr| tr
            .select_sentence_at(Position::new(p, 15, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 13) -> (p, 25, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_sentence_at_across_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text { "world. How are you?" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .select_sentence_at(Position::new(p, 3, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text { "world. How are you?" }
                }
            }
            selection { (p, 0) -> (p, 12, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_paragraph_at_position() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .select_paragraph_at(Position::new(p, 3, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello world. How are you?" }
                }
            }
            selection { (p, 0) -> (p, 25, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_at_across_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "hel" }
                    text { "lo world" }
                }
            }
            selection { (p, 1) }
        };

        let actual = transact!(initial, |tr| tr
            .select_word_at(Position::new(p, 1, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hel" }
                    text { "lo world" }
                }
            }
            selection { (p, 0) -> (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_at_empty_paragraph() {
        let mut p = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                @p paragraph { }
                @p2 paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) }
        };

        let mut tr = Transaction::new(&initial);
        let result = tr
            .select_word_at(Position::new(p, 0, Affinity::Downstream))
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn select_word_at_last_empty_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .select_word_at(Position::new(p, 0, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph { }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_before_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    hard_break {}
                }
            }
            selection { (p, 1) }
        };

        let mut tr = Transaction::new(&initial);
        tr.select_word_at(tr.selection().anchor).unwrap();
        let (actual, _) = tr.commit().unwrap();

        let expected = state! {
            doc {
                @p paragraph {
                    text { "h" }
                    hard_break {}
                }
            }
            selection { (p, 1) -> (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_between_hard_breaks() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 1) }
        };

        let mut tr = Transaction::new(&initial);
        tr.select_word_at(tr.selection().anchor).unwrap();
        let (actual, _) = tr.commit().unwrap();

        let expected = state! {
            doc {
                @p paragraph {
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 1) -> (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_after_last_hard_break_not_applicable() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    hard_break {}
                }
            }
            selection { (p, 1) }
        };

        let mut tr = Transaction::new(&state);
        let result = tr.select_word_at(tr.selection().anchor).unwrap();
        assert!(!result);
    }

    #[test]
    fn select_paragraph_at_across_text_nodes() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text { "world!" }
                }
            }
            selection { (p, 3) }
        };

        let actual = transact!(initial, |tr| tr
            .select_paragraph_at(Position::new(p, 3, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text { "world!" }
                }
            }
            selection { (p, 0) -> (p, 12, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_word_at_whitespace_selects_previous_word() {
        // Cursor at "안녕하세요| 여러분" (offset 5, the space)
        // Should select "안녕하세요" (0-5), not the space.
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .select_word_at(Position::new(p, 5, Affinity::Downstream))
            .unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 0) -> (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn word_range_at_boundary_returns_adjacent_segment() {
        // "안녕하세요 여러분" — offset 5 is the space, which is outside "안녕하세요"
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 0) }
        };

        // At offset 4 (inside "안녕하세요"): should return the word range (0, 5)
        let range = word_range_at(&state.doc, Position::new(p, 4, Affinity::Downstream));
        assert_eq!(range, Some((0, 5)));

        // At offset 5 (the space): should return the space segment (5, 6)
        let range = word_range_at(&state.doc, Position::new(p, 5, Affinity::Downstream));
        assert_eq!(range, Some((5, 6)));

        // At offset 0 (start of "안녕하세요"): should return (0, 5)
        let range = word_range_at(&state.doc, Position::new(p, 0, Affinity::Downstream));
        assert_eq!(range, Some((0, 5)));
    }

    #[test]
    fn word_expansion_not_available_when_selection_is_exact_word() {
        // When "안녕하세요" is already selected (0-5), expanding both endpoints
        // to word boundaries should yield the same range — so word expansion is not possible.
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 0) -> (p, 5, Affinity::Upstream) }
        };

        let from = Position::new(p, 0, Affinity::Downstream);
        let to = Position::new(p, 5, Affinity::Upstream);
        let to_inner = Position::new(p, 4, Affinity::Downstream);

        let from_word = word_range_at(&state.doc, from);
        let to_word = word_range_at(&state.doc, to_inner);

        // Both should resolve to "안녕하세요" = (0, 5)
        assert_eq!(from_word, Some((0, 5)));
        assert_eq!(to_word, Some((0, 5)));

        // Expanded range = (0, 5), which equals current selection → NOT expandable
        let (ws, we) = match (from_word, to_word) {
            (Some((ws1, _)), Some((_, we2))) => (ws1, we2),
            _ => unreachable!(),
        };
        assert_eq!(ws, from.offset);
        assert_eq!(we, to.offset);
    }

    #[test]
    fn word_expansion_not_available_when_selection_spans_multiple_words() {
        // "안녕하세요 여러" selected (0-8) in "안녕하세요 여러분"
        // Endpoints are in different words, so word expansion should NOT be available.
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 0) -> (p, 8, Affinity::Upstream) }
        };

        let from = Position::new(p, 0, Affinity::Downstream);
        let to_inner = Position::new(p, 7, Affinity::Downstream);

        let from_word = word_range_at(&state.doc, from);
        let to_word = word_range_at(&state.doc, to_inner);

        // from is in "안녕하세요" (0,5), to is in "여러분" (6,9) — different words
        assert_eq!(from_word, Some((0, 5)));
        assert_eq!(to_word, Some((6, 9)));

        // Different words → word expansion should not be flagged
        let (ws1, we1) = from_word.unwrap();
        let (ws2, we2) = to_word.unwrap();
        assert!(ws1 != ws2 || we1 != we2);
    }

    #[test]
    fn word_expansion_available_for_partial_word_selection() {
        // When "녕하" is selected (1-3), expanding should yield "안녕하세요" (0-5)
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "안녕하세요 여러분" }
                }
            }
            selection { (p, 1) -> (p, 3, Affinity::Upstream) }
        };

        let from = Position::new(p, 1, Affinity::Downstream);
        let to_inner = Position::new(p, 2, Affinity::Downstream);

        let from_word = word_range_at(&state.doc, from);
        let to_word = word_range_at(&state.doc, to_inner);

        assert_eq!(from_word, Some((0, 5)));
        assert_eq!(to_word, Some((0, 5)));

        let (ws, we) = match (from_word, to_word) {
            (Some((ws1, _)), Some((_, we2))) => (ws1, we2),
            _ => unreachable!(),
        };
        // Expanded (0, 5) is larger than selection (1, 3) → expandable
        assert!(ws < from.offset || we > 3);
    }

    #[test]
    fn move_to_next_block_across_subtree() {
        let mut inner = id!();
        let mut after = id!();

        let initial = state! {
            doc {
                blockquote {
                    @inner paragraph {
                        text { "inside quote" }
                    }
                }
                @after paragraph {
                    text { "after quote" }
                }
            }
            selection { (inner, 0) }
        };

        let actual = transact!(initial, |tr| tr.move_to_next_block(inner).unwrap());

        let expected = state! {
            doc {
                blockquote {
                    @inner paragraph {
                        text { "inside quote" }
                    }
                }
                @after paragraph {
                    text { "after quote" }
                }
            }
            selection { (after, 0) }
        };

        assert_state_eq!(actual, expected);
    }
}

use editor_model::{Node, NodeId, PlainHardBreakNode, PlainNode, PlainTextNode, Subtree};
use editor_resource::{CompiledPattern, Resource, TextReplacementRule};
use editor_state::{Position, Selection};
use editor_transaction::{HistoryMeta, HistoryTag, Transaction};

use crate::{CommandError, CommandResult};

pub fn try_text_replacement(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let rules = &resource.text_replacement_rules;
    if rules.is_empty() {
        return Ok(false);
    }
    if tr.composition().is_some() {
        return Ok(false);
    }
    if !tr.selection().is_collapsed() {
        return Ok(false);
    }

    let mut replaced = false;
    let mut search_start_byte = 0usize;

    loop {
        let Some((_block_id, text_before)) = get_text_before_cursor(tr) else {
            break;
        };
        if text_before.is_empty() {
            break;
        }
        if search_start_byte >= text_before.len() {
            break;
        }
        while search_start_byte < text_before.len()
            && !text_before.is_char_boundary(search_start_byte)
        {
            search_start_byte += 1;
        }

        let matched = match_rule(rules, &text_before, search_start_byte);
        let Some((matched_start_byte, matched_text, substitute, suffix)) = matched else {
            break;
        };

        let next_search_start_byte = matched_start_byte.saturating_add(substitute.len());

        let original_offset_len = offset_len_for_text(&matched_text);
        let _replaced_offset_len = offset_len_for_text(&substitute);
        let suffix_offset_len = offset_len_for_text(&suffix);

        let delete_count = original_offset_len + suffix_offset_len;
        for _ in 0..delete_count {
            delete_one_backward(tr)?;
        }

        let full_insert = if suffix.is_empty() {
            substitute.clone()
        } else {
            format!("{substitute}{suffix}")
        };
        for (i, part) in full_insert.split('\n').enumerate() {
            if i > 0 {
                insert_hard_break(tr)?;
            }
            if !part.is_empty() {
                insert_text_at_cursor(tr, part)?;
            }
        }

        replaced = true;
        search_start_byte = next_search_start_byte;
    }

    if replaced {
        tr.update_meta(|m| {
            m.history = HistoryMeta::Tagged {
                tag: HistoryTag::AutoReplacement,
            }
        });
    }
    Ok(replaced)
}

fn match_rule(
    rules: &[TextReplacementRule],
    text_before: &str,
    search_start_byte: usize,
) -> Option<(usize, String, String, String)> {
    for rule in rules {
        match &rule.pattern {
            CompiledPattern::Plain(pattern) => {
                for (pos, _) in text_before.match_indices(pattern.as_str()) {
                    if pos < search_start_byte {
                        continue;
                    }
                    let match_end = pos + pattern.len();
                    if match_end == text_before.len() {
                        return Some((
                            pos,
                            pattern.clone(),
                            rule.substitute.clone(),
                            String::new(),
                        ));
                    }
                }
            }
            CompiledPattern::Regex(regex) => {
                for caps in regex.captures_iter(text_before).flatten() {
                    if let Some(m) = caps.get(0) {
                        if m.end() == text_before.len() && m.start() >= search_start_byte {
                            let matched_str = m.as_str().to_string();
                            let expanded = expand_substitute(&caps, &rule.substitute);
                            return Some((m.start(), matched_str, expanded, String::new()));
                        }
                    }
                }
            }
        }
    }
    None
}

fn expand_substitute(caps: &fancy_regex::Captures<'_>, template: &str) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '$' {
            result.push(c);
            continue;
        }

        match chars.peek() {
            Some(&'$') => {
                chars.next();
                result.push('$');
            }
            Some(&'{') => {
                chars.next();
                let mut name = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    name.push(c);
                }
                if let Some(m) = caps
                    .name(&name)
                    .or_else(|| name.parse::<usize>().ok().and_then(|n| caps.get(n)))
                {
                    result.push_str(m.as_str());
                }
            }
            Some(&c) if c.is_ascii_digit() => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(m) = num_str.parse::<usize>().ok().and_then(|n| caps.get(n)) {
                    result.push_str(m.as_str());
                }
            }
            Some(&c) if c.is_ascii_alphabetic() || c == '_' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(m) = caps
                    .name(&name)
                    .or_else(|| name.parse::<usize>().ok().and_then(|n| caps.get(n)))
                {
                    result.push_str(m.as_str());
                }
            }
            _ => {
                result.push('$');
            }
        }
    }

    result
}

fn offset_len_for_text(text: &str) -> usize {
    let mut len = 0;
    for (i, part) in text.split('\n').enumerate() {
        if i > 0 {
            len += 1;
        }
        len += part.chars().count();
    }
    len
}

fn get_text_before_cursor(tr: &Transaction) -> Option<(NodeId, String)> {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return None;
    }

    let head = selection.head;
    let doc = tr.doc();
    let cursor_node = doc.node(head.node_id)?;

    let block = cursor_node.ancestors().find(|n| n.spec().is_textblock())?;
    if cursor_node.id() == block.id() {
        return None;
    }
    let block_id = block.id();
    let cursor_offset = head.offset;
    let cursor_index = cursor_node.index()?;

    let mut text = String::new();
    for (i, child) in block.children().enumerate() {
        if i > cursor_index {
            break;
        }
        match child.node() {
            Node::Text(text_node) => {
                let full = text_node.text.to_string();
                if i == cursor_index {
                    let partial: String = full.chars().take(cursor_offset).collect();
                    text.push_str(&partial);
                } else {
                    text.push_str(&full);
                }
            }
            Node::HardBreak(_) => {
                if i < cursor_index || cursor_offset > 0 {
                    text.push('\n');
                }
            }
            _ => {}
        }
    }

    Some((block_id, text))
}

fn delete_one_backward(tr: &mut Transaction) -> CommandResult {
    let sel = tr.selection();
    let head = sel.head;
    let doc = tr.doc();
    let node = doc
        .node(head.node_id)
        .ok_or(CommandError::NodeNotFound(head.node_id))?;
    let Node::Text(text_node) = node.node() else {
        return Ok(false);
    };

    if head.offset > 0 {
        let new_offset = head.offset - 1;
        tr.remove_text(head.node_id, new_offset, 1)?;
        tr.set_selection(Selection::collapsed(Position::new(
            head.node_id,
            new_offset,
        )))?;
        return Ok(true);
    }

    let Some(prev) = node.prev_sibling() else {
        return Ok(false);
    };
    if !matches!(prev.node(), Node::HardBreak(_)) {
        return Ok(false);
    }

    let hard_break_id = prev.id();
    let current_text_id = head.node_id;
    let current_is_empty = text_node.text.is_empty();
    let target = prev.prev_sibling().and_then(|n| match n.node() {
        Node::Text(t) => Some((n.id(), t.text.len())),
        _ => None,
    });

    tr.remove_subtree(hard_break_id)?;
    if let Some((target_id, target_offset)) = target {
        if current_is_empty {
            tr.remove_subtree(current_text_id)?;
        }
        tr.set_selection(Selection::collapsed(Position::new(
            target_id,
            target_offset,
        )))?;
    }
    Ok(true)
}

fn insert_text_at_cursor(tr: &mut Transaction, text: &str) -> CommandResult {
    if text.is_empty() {
        return Ok(false);
    }
    let sel = tr.selection();
    let head = sel.head;
    let doc = tr.doc();
    let node = doc
        .node(head.node_id)
        .ok_or(CommandError::NodeNotFound(head.node_id))?;

    let (text_node_id, start_offset) = if matches!(node.node(), Node::Text(_)) {
        (head.node_id, head.offset)
    } else {
        let new_id = NodeId::new();
        let subtree = Subtree::leaf(
            new_id,
            PlainNode::Text(PlainTextNode {
                text: String::new(),
            }),
        );
        tr.insert_subtree(head.node_id, head.offset, subtree)?;
        (new_id, 0)
    };

    let insert_len = text.chars().count();
    tr.insert_text(text_node_id, start_offset, text)?;
    let new_offset = start_offset + insert_len;
    tr.set_selection(Selection::collapsed(Position::new(
        text_node_id,
        new_offset,
    )))?;
    Ok(true)
}

fn insert_hard_break(tr: &mut Transaction) -> CommandResult {
    let sel = tr.selection();
    let head = sel.head;

    let (node_id, text_node_exists, text_len) = {
        let doc = tr.doc();
        let node = doc
            .node(head.node_id)
            .ok_or(CommandError::NodeNotFound(head.node_id))?;
        match node.node() {
            Node::Text(t) => (head.node_id, true, t.text.len()),
            _ => (head.node_id, false, 0),
        }
    };

    let break_id = NodeId::new();
    let break_subtree = Subtree::leaf(
        break_id,
        PlainNode::HardBreak(PlainHardBreakNode::default()),
    );

    if text_node_exists {
        let (parent_id, node_index) = {
            let doc = tr.doc();
            let node = doc
                .node(node_id)
                .ok_or(CommandError::NodeNotFound(node_id))?;
            let parent = node.parent().ok_or(CommandError::NoParent(node_id))?;
            let index = node
                .index()
                .ok_or(CommandError::orphan_child(node_id, parent.id()))?;
            (parent.id(), index)
        };

        let hb_pos = head.offset;
        if hb_pos == 0 {
            tr.insert_subtree(parent_id, node_index, break_subtree)?;
            tr.set_selection(Selection::collapsed(Position::new(node_id, 0)))?;
        } else if hb_pos == text_len {
            tr.insert_subtree(parent_id, node_index + 1, break_subtree)?;
            let doc = tr.doc();
            let break_node = doc
                .node(break_id)
                .ok_or(CommandError::NodeNotFound(break_id))?;
            if let Some(next) = break_node.next_sibling() {
                if matches!(next.node(), Node::Text(_)) {
                    tr.set_selection(Selection::collapsed(Position::new(next.id(), 0)))?;
                } else {
                    let idx = next
                        .index()
                        .ok_or(CommandError::orphan_child(next.id(), parent_id))?;
                    tr.set_selection(Selection::collapsed(Position::new(parent_id, idx)))?;
                }
            } else {
                let break_idx = break_node
                    .index()
                    .ok_or(CommandError::orphan_child(break_id, parent_id))?;
                tr.set_selection(Selection::collapsed(Position::new(
                    parent_id,
                    break_idx + 1,
                )))?;
            }
        } else {
            let split_id = NodeId::new();
            tr.split_node(node_id, hb_pos, split_id)?;
            tr.insert_subtree(parent_id, node_index + 1, break_subtree)?;
            tr.set_selection(Selection::collapsed(Position::new(split_id, 0)))?;
        }
    } else {
        tr.insert_subtree(node_id, head.offset, break_subtree)?;
        tr.set_selection(Selection::collapsed(Position::new(
            node_id,
            head.offset + 1,
        )))?;
    }
    Ok(true)
}

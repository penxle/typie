use crate::model::{Node, NodeId};
use crate::runtime::{Effect, Runtime};
use serde::{Deserialize, Serialize};

pub fn expand_substitute(caps: &fancy_regex::Captures<'_>, template: &str) -> String {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTextReplacementRule {
    pub id: String,
    pub match_pattern: String,
    pub substitute: String,
    pub regex: bool,
}

pub enum CompiledPattern {
    Plain(String),
    Regex(fancy_regex::Regex),
}

#[allow(dead_code)]
pub struct TextReplacementRule {
    pub id: String,
    pub pattern: CompiledPattern,
    pub substitute: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementUndoState {
    pub node_id: NodeId,
    pub offset: usize,
    pub original_text: String,
    pub replaced_text: String,
    pub original_offset_len: usize,
    pub replaced_offset_len: usize,
}

pub fn offset_len_for_text(text: &str) -> usize {
    let mut len = 0;
    for part in text.split('\n') {
        if len > 0 {
            len += 1;
        }
        len += part.chars().count();
    }
    len
}

impl Runtime {
    pub(crate) fn try_undo_text_replacement(&mut self) -> Option<Vec<Effect>> {
        let undo = self.text_replacement_undo.as_ref()?;

        let selection = &self.state.selection;
        if !selection.is_collapsed() {
            return None;
        }

        if selection.head.node_id != undo.node_id || selection.head.offset != undo.offset {
            return None;
        }

        let undo = self.text_replacement_undo.take().unwrap();

        let delete_count = undo.replaced_offset_len;
        let original = undo.original_text.clone();

        let mut effects = self.transact(|tr| {
            for _ in 0..delete_count {
                tr.delete_text_backward()?;
            }
            Ok(true)
        });

        let parts: Vec<&str> = original.split('\n').collect();
        let insert_effects = self.transact(|tr| {
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    tr.insert_hard_break()?;
                }
                if !part.is_empty() {
                    tr.insert_text(part)?;
                }
            }
            Ok(true)
        });

        effects.extend(insert_effects);
        Some(effects)
    }

    pub(crate) fn get_text_before_cursor(&self) -> Option<(NodeId, String, usize)> {
        let selection = &self.state.selection;
        if !selection.is_collapsed() {
            return None;
        }

        let head = selection.head;
        let block = self.doc().node(head.node_id)?;

        if !block.is_block() {
            return None;
        }

        let mut text = String::new();
        let mut current_offset = 0;

        for child in block.children() {
            if current_offset >= head.offset {
                break;
            }

            match child.node() {
                Node::Text(text_node) => {
                    let char_len = text_node.text.char_len();
                    let remaining = head.offset - current_offset;
                    if remaining >= char_len {
                        text.push_str(&text_node.text.to_string());
                        current_offset += char_len;
                    } else {
                        let full = text_node.text.to_string();
                        let partial: String = full.chars().take(remaining).collect();
                        text.push_str(&partial);
                        current_offset += remaining;
                    }
                }
                Node::HardBreak(_) => {
                    text.push('\n');
                    current_offset += 1;
                }
                _ => {
                    current_offset += 1;
                }
            }
        }

        Some((head.node_id, text, head.offset))
    }
}

use editor_common::HistoryTag;
use editor_crdt::Dot;
use editor_model::{AtomLeaf, ChildView};
use editor_resource::{CompiledPattern, Resource, TextReplacementRule};
use editor_state::{Position, Selection};
use editor_transaction::{HistoryMeta, Transaction};

use crate::CommandResult;
use crate::helpers::insert_hard_break_at_caret;

pub fn try_text_replacement(tr: &mut Transaction, resource: &Resource) -> CommandResult {
    let rules = &resource.text_replacement_rules;
    if rules.is_empty() {
        return Ok(false);
    }
    if tr.composition().is_some() {
        return Ok(false);
    }
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let mut replaced = false;
    let mut search_start_byte = 0usize;

    while let Some((_block_id, text_before)) = get_text_before_cursor(tr) {
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
                insert_hard_break_at_caret(tr)?;
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

// Only matches ending exactly at the cursor are ever accepted, so a regex only
// needs to see a bounded tail of the paragraph text instead of the whole
// prefix — matches longer than this window are not recognized.
const REGEX_TAIL_WINDOW: usize = 256;

fn match_rule(
    rules: &[TextReplacementRule],
    text_before: &str,
    search_start_byte: usize,
) -> Option<(usize, String, String, String)> {
    for rule in rules {
        match &rule.pattern {
            CompiledPattern::Plain(pattern) => {
                let Some(pos) = text_before.len().checked_sub(pattern.len()) else {
                    continue;
                };
                if pos >= search_start_byte && text_before.ends_with(pattern.as_str()) {
                    return Some((pos, pattern.clone(), rule.substitute.clone(), String::new()));
                }
            }
            CompiledPattern::Regex(regex) => {
                let mut window_start = text_before.len().saturating_sub(REGEX_TAIL_WINDOW);
                while !text_before.is_char_boundary(window_start) {
                    window_start += 1;
                }
                let window = &text_before[window_start..];
                for caps in regex.captures_iter(window).flatten() {
                    if let Some(m) = caps.get(0)
                        && window_start + m.end() == text_before.len()
                        && window_start + m.start() >= search_start_byte
                    {
                        let matched_str = m.as_str().to_string();
                        let expanded = expand_substitute(&caps, &rule.substitute);
                        return Some((
                            window_start + m.start(),
                            matched_str,
                            expanded,
                            String::new(),
                        ));
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

fn get_text_before_cursor(tr: &Transaction) -> Option<(Dot, String)> {
    let selection = tr.selection()?;
    if selection.anchor != selection.head {
        return None;
    }

    let head = selection.head;
    let view = tr.view();
    let block = view.node(head.node)?;
    if !block.spec().is_textblock() {
        return None;
    }

    let mut text = String::new();
    for (i, child) in block.children().enumerate() {
        if i >= head.offset {
            break;
        }
        match child {
            ChildView::Leaf(l) => {
                if let Some(ch) = l.as_char() {
                    text.push(ch);
                } else if matches!(l.as_atom(), Some(AtomLeaf::HardBreak)) {
                    text.push('\n');
                }
            }
            ChildView::Block(_) => {}
        }
    }

    Some((head.node, text))
}

fn delete_one_backward(tr: &mut Transaction) -> CommandResult {
    let sel = tr.selection().expect("entry caller guaranteed selection");
    let head = sel.head;
    if head.offset == 0 {
        return Ok(false);
    }
    tr.remove_text(head.node, head.offset - 1, 1)?;
    tr.set_selection(Some(Selection::collapsed(Position::new(
        head.node,
        head.offset - 1,
    ))))?;
    Ok(true)
}

fn insert_text_at_cursor(tr: &mut Transaction, text: &str) -> CommandResult {
    if text.is_empty() {
        return Ok(false);
    }
    let sel = tr.selection().expect("entry caller guaranteed selection");
    let head = sel.head;
    let insert_len = text.chars().count();
    tr.insert_text(head.node, head.offset, text)?;
    tr.set_selection(Some(Selection::collapsed(Position::new(
        head.node,
        head.offset + insert_len,
    ))))?;
    Ok(true)
}

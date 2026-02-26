use crate::global::with_text_replacement_rules;
use crate::model::{Node, NodeId};
use crate::runtime::Effect;
use crate::runtime::text_replacement::{CompiledPattern, ReplacementUndoState};
use crate::transaction::Transaction;
use anyhow::Result;

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
    for part in text.split('\n') {
        if len > 0 {
            len += 1;
        }
        len += part.chars().count();
    }
    len
}

impl Transaction {
    pub fn try_text_replacement(&mut self, input_byte_len: usize) -> Result<bool> {
        if self.state.preedit.is_some() {
            return Ok(false);
        }

        if !self.selection().is_collapsed() {
            return Ok(false);
        }

        let mut replaced = false;
        let mut search_start_byte = 0usize;

        loop {
            let Some((block_id, text_before, cursor_offset)) = self.get_text_before_cursor() else {
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

            let input_start_byte = text_before.len().saturating_sub(input_byte_len);

            let matched = with_text_replacement_rules(|rules| {
                for rule in rules {
                    match &rule.pattern {
                        CompiledPattern::Plain(pattern) => {
                            for (pos, _) in text_before.match_indices(pattern.as_str()) {
                                if pos < search_start_byte {
                                    continue;
                                }
                                let match_end = pos + pattern.len();
                                if match_end > input_start_byte {
                                    let suffix = text_before[match_end..].to_string();
                                    return Some((
                                        pos,
                                        pattern.clone(),
                                        rule.substitute.clone(),
                                        suffix,
                                    ));
                                }
                            }
                        }
                        CompiledPattern::Regex(regex) => {
                            let try_start = (input_start_byte + 1).max(search_start_byte + 1);
                            for try_end in try_start..=text_before.len() {
                                if !text_before.is_char_boundary(try_end) {
                                    continue;
                                }
                                let truncated = &text_before[..try_end];
                                if let Ok(Some(caps)) = regex.captures(truncated) {
                                    if let Some(m) = caps.get(0) {
                                        if m.end() == truncated.len()
                                            && m.start() >= search_start_byte
                                        {
                                            let matched_str = m.as_str().to_string();
                                            let expanded =
                                                expand_substitute(&caps, &rule.substitute);
                                            let suffix = text_before[try_end..].to_string();
                                            return Some((
                                                m.start(),
                                                matched_str,
                                                expanded,
                                                suffix,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            });

            let Some((matched_start_byte, matched_text, substitute, suffix)) = matched else {
                break;
            };

            let next_search_start_byte = matched_start_byte.saturating_add(substitute.len());

            let original_offset_len = offset_len_for_text(&matched_text);
            let replaced_offset_len = offset_len_for_text(&substitute);
            let suffix_offset_len = offset_len_for_text(&suffix);

            let delete_count = original_offset_len + suffix_offset_len;
            for _ in 0..delete_count {
                self.delete_text_backward()?;
            }

            let full_insert = if suffix.is_empty() {
                substitute.clone()
            } else {
                format!("{}{}", substitute, suffix)
            };

            let parts: Vec<&str> = full_insert.split('\n').collect();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    self.insert_hard_break()?;
                }
                if !part.is_empty() {
                    self.insert_text(part)?;
                }
            }

            let new_offset =
                cursor_offset - original_offset_len - suffix_offset_len + replaced_offset_len;

            let undo_state = ReplacementUndoState {
                node_id: block_id,
                offset: new_offset,
                original_text: matched_text,
                replaced_text: substitute,
                original_offset_len,
                replaced_offset_len,
            };
            self.push_effect(Effect::TextReplacementApplied { undo_state });

            replaced = true;
            search_start_byte = next_search_start_byte;
        }

        Ok(replaced)
    }

    pub fn try_undo_text_replacement(&mut self, undo: &ReplacementUndoState) -> Result<bool> {
        let selection = self.selection();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        if selection.head.node_id != undo.node_id || selection.head.offset != undo.offset {
            return Ok(false);
        }

        for _ in 0..undo.replaced_offset_len {
            self.delete_text_backward()?;
        }

        let parts: Vec<&str> = undo.original_text.split('\n').collect();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                self.insert_hard_break()?;
            }
            if !part.is_empty() {
                self.insert_text(part)?;
            }
        }

        Ok(true)
    }

    fn get_text_before_cursor(&self) -> Option<(NodeId, String, usize)> {
        let selection = self.selection();
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

            let Some(child_data) = child.node() else {
                continue;
            };
            match child_data {
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

#[cfg(test)]
mod tests {
    use crate::global::{clear_text_replacement_rules, set_text_replacement_rules};
    use crate::runtime::text_replacement::RawTextReplacementRule;
    use crate::runtime::{Direction, Message};
    use crate::types::Affinity;

    fn set_rules(rules: Vec<RawTextReplacementRule>) {
        set_text_replacement_rules(rules);
    }

    fn clear_rules() {
        clear_text_replacement_rules();
    }

    #[test]
    fn plain_text_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(c)".into(),
            substitute: "\u{00A9}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "(c" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: ")".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{00A9}" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn backspace_undo_after_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(c)".into(),
            substitute: "\u{00A9}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "(c" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: ")".to_string(),
        });

        rt.update(Message::DeleteBackward);

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "(c)" }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn cursor_move_clears_undo() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(c)".into(),
            substitute: "\u{00A9}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "x(c" }
                }
            }
            selection { (p, 3) }
        };

        rt.update(Message::Input {
            text: ")".to_string(),
        });

        rt.layout();
        rt.update(Message::Navigate {
            direction: Direction::Left,
            extend: false,
        });

        rt.update(Message::DeleteBackward);

        let text = rt.doc().to_plain_text();
        assert!(
            !text.contains("(c)"),
            "After cursor move, backspace should do normal delete, not undo"
        );

        clear_rules();
    }

    #[test]
    fn first_match_wins() {
        set_rules(vec![
            RawTextReplacementRule {
                id: "1".into(),
                match_pattern: "abc".into(),
                substitute: "FIRST".into(),
                regex: false,
            },
            RawTextReplacementRule {
                id: "2".into(),
                match_pattern: "abc".into(),
                substitute: "SECOND".into(),
                regex: false,
            },
        ]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: "c".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "FIRST" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn regex_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: r#"-->"#.into(),
            substitute: "\u{2192}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "--" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: ">".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{2192}" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn preedit_skips_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "abc".into(),
            substitute: "X".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::CompositionStart {
            text: String::new(),
        });
        rt.update(Message::CompositionUpdate {
            text: "c".to_string(),
        });

        let text = rt.doc().to_plain_text();
        assert_eq!(text, "ab");

        clear_rules();
    }

    #[test]
    fn replacement_after_preedit_commit() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(?<=\u{201C}[^\u{201D}]*)\"".into(),
            substitute: "\u{201D}".into(),
            regex: true,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "\u{201C}안" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::CompositionStart {
            text: String::new(),
        });
        rt.update(Message::CompositionUpdate {
            text: "\u{B155}".to_string(),
        });

        rt.update(Message::Input {
            text: "\u{B155}\"".to_string(),
        });
        rt.update(Message::CompositionEnd);

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{201C}안\u{B155}\u{201D}" }
                }
            }
            selection { (p, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn same_match_substitute_filtered() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "abc".into(),
            substitute: "abc".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "ab" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: "c".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "abc" }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_plain_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(c)".into(),
            substitute: "\u{00A9}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        rt.update(Message::Input {
            text: "(c)def".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{00A9}def" }
                }
            }
            selection { (p, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_with_existing_text() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "-->".into(),
            substitute: "\u{2192}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "a-" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: "->bc".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "a\u{2192}bc" }
                }
            }
            selection { (p, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_no_false_match_on_old_text() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "abc".into(),
            substitute: "X".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "abc" }
                }
            }
            selection { (p, 3) }
        };

        rt.update(Message::Input {
            text: "def".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "abcdef" }
                }
            }
            selection { (p, 6, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_regex_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "-->".into(),
            substitute: "\u{2192}".into(),
            regex: true,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "-" }
                }
            }
            selection { (p, 1) }
        };

        rt.update(Message::Input {
            text: "->xyz".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{2192}xyz" }
                }
            }
            selection { (p, 4, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn commit_preedit_triggers_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "(c)".into(),
            substitute: "\u{00A9}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "(c" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::CompositionStart {
            text: String::new(),
        });
        rt.update(Message::CompositionUpdate {
            text: ")".to_string(),
        });
        rt.update(Message::CommitPreedit);

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{00A9}" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn regex_capture_group_substitute() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: r#""([^"]*)"$"#.into(),
            substitute: "\u{201C}$1\u{201D}".into(),
            regex: true,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "\"hello" }
                }
            }
            selection { (p, 6) }
        };

        rt.update(Message::Input {
            text: "\"".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{201C}hello\u{201D}" }
                }
            }
            selection { (p, 7, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn regex_named_capture_group_substitute() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: r#"(?P<num>\d+)/(?P<den>\d+)"#.into(),
            substitute: "$num\u{2044}$den".into(),
            regex: true,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "1/" }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: "2".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "1\u{2044}2" }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn regex_capture_group_with_unicode_substitute() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: r"asd(.+)f".into(),
            substitute: "안녕$1하세요".into(),
            regex: true,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "asd1" }
                }
            }
            selection { (p, 4) }
        };

        rt.update(Message::Input {
            text: "f".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "안녕1하세요" }
                }
            }
            selection { (p, 6, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_repeated_plain_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "...".into(),
            substitute: "\u{2026}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        rt.update(Message::Input {
            text: "......".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{2026}\u{2026}" }
                }
            }
            selection { (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn batched_input_seven_dots_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "...".into(),
            substitute: "\u{2026}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        rt.update(Message::Input {
            text: ".......".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{2026}\u{2026}." }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn growing_rule_applies_once_per_pass() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "a".into(),
            substitute: "aa".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        rt.update(Message::Input {
            text: "a".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "aa" }
                }
            }
            selection { (p, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }

    #[test]
    fn replace_backward_triggers_replacement() {
        set_rules(vec![RawTextReplacementRule {
            id: "1".into(),
            match_pattern: "...".into(),
            substitute: "\u{2026}".into(),
            regex: false,
        }]);

        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { ".." }
                }
            }
            selection { (p, 2) }
        };

        rt.update(Message::ReplaceBackward {
            length: 2,
            text: "...".to_string(),
        });

        let actual = rt.state();
        let expected = state! {
            doc {
                @p paragraph {
                    text { "\u{2026}" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
        clear_rules();
    }
}

use crate::global::with_text_replacement_rules;
use crate::runtime::text_replacement::{
    CompiledPattern, ReplacementUndoState, expand_substitute, offset_len_for_text,
};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn try_text_replacement(&mut self, input_byte_len: usize) -> Option<Vec<Effect>> {
        if self.state.preedit.is_some() {
            return None;
        }

        if !self.state.selection.is_collapsed() {
            return None;
        }

        let (block_id, text_before, cursor_offset) = self.get_text_before_cursor()?;

        if text_before.is_empty() {
            return None;
        }

        let input_start_byte = text_before.len().saturating_sub(input_byte_len);

        let matched = with_text_replacement_rules(|rules| {
            for rule in rules {
                match &rule.pattern {
                    CompiledPattern::Plain(pattern) => {
                        for (pos, _) in text_before.match_indices(pattern.as_str()) {
                            let match_end = pos + pattern.len();
                            if match_end > input_start_byte {
                                let suffix = text_before[match_end..].to_string();
                                return Some((pattern.clone(), rule.substitute.clone(), suffix));
                            }
                        }
                    }
                    CompiledPattern::Regex(regex) => {
                        for try_end in (input_start_byte + 1)..=text_before.len() {
                            if !text_before.is_char_boundary(try_end) {
                                continue;
                            }
                            let truncated = &text_before[..try_end];
                            if let Ok(Some(caps)) = regex.captures(truncated) {
                                if let Some(m) = caps.get(0) {
                                    if m.end() == truncated.len() {
                                        let matched_str = m.as_str().to_string();
                                        let expanded = expand_substitute(&caps, &rule.substitute);
                                        let suffix = text_before[try_end..].to_string();
                                        return Some((matched_str, expanded, suffix));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        });

        let (matched_text, substitute, suffix) = matched?;

        let original_offset_len = offset_len_for_text(&matched_text);
        let replaced_offset_len = offset_len_for_text(&substitute);
        let suffix_offset_len = offset_len_for_text(&suffix);

        let delete_count = original_offset_len + suffix_offset_len;

        let mut effects = self.transact(|tr| {
            for _ in 0..delete_count {
                tr.delete_text_backward()?;
            }
            Ok(true)
        });

        let full_insert = if suffix.is_empty() {
            substitute.clone()
        } else {
            format!("{}{}", substitute, suffix)
        };

        let parts: Vec<&str> = full_insert.split('\n').collect();
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

        effects.push(Effect::TextReplacementApplied { undo_state });

        Some(effects)
    }
}

#[cfg(test)]
mod tests {
    use crate::global::{clear_text_replacement_rules, set_text_replacement_rules};
    use crate::runtime::Message;
    use crate::runtime::message::Direction;
    use crate::runtime::text_replacement::RawTextReplacementRule;
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
}

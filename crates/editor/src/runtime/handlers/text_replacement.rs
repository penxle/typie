use crate::runtime::text_replacement::{ReplacementUndoState, offset_len_for_text};
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn try_text_replacement(&mut self) -> Option<Vec<Effect>> {
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

        let matched = crate::global::with_text_replacement_rules(|rules| {
            for rule in rules {
                match &rule.pattern {
                    crate::runtime::text_replacement::CompiledPattern::Plain(pattern) => {
                        if text_before.ends_with(pattern.as_str()) {
                            return Some((pattern.clone(), rule.substitute.clone(), pattern.len()));
                        }
                    }
                    crate::runtime::text_replacement::CompiledPattern::Regex(regex) => {
                        if let Ok(Some(m)) = regex.find(&text_before) {
                            if m.end() == text_before.len() {
                                let matched_str = m.as_str().to_string();
                                return Some((
                                    matched_str.clone(),
                                    rule.substitute.clone(),
                                    matched_str.len(),
                                ));
                            }
                        }
                    }
                }
            }
            None
        });

        let (matched_text, substitute, _matched_byte_len) = matched?;

        let original_offset_len = offset_len_for_text(&matched_text);
        let replaced_offset_len = offset_len_for_text(&substitute);

        let delete_count = original_offset_len;

        let mut effects = self.transact(|tr| {
            for _ in 0..delete_count {
                tr.delete_text_backward()?;
            }
            Ok(true)
        });

        let parts: Vec<&str> = substitute.split('\n').collect();
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

        let new_offset = cursor_offset - original_offset_len + replaced_offset_len;

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
    use crate::runtime::Message;
    use crate::runtime::message::Direction;
    use crate::runtime::text_replacement::RawTextReplacementRule;
    use crate::types::Affinity;

    fn set_rules(rules: Vec<RawTextReplacementRule>) {
        crate::global::set_text_replacement_rules(rules);
    }

    fn clear_rules() {
        crate::global::clear_text_replacement_rules();
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
}

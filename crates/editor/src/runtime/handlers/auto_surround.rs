use super::super::{Effect, Runtime};

const AUTO_SURROUND_PAIRS: &[(&str, &str, &str)] = &[
    ("(", "(", ")"),
    ("[", "[", "]"),
    ("{", "{", "}"),
    ("\"", "\u{201C}", "\u{201D}"),       // " → “ ”
    ("'", "\u{2018}", "\u{2019}"),        // ' → ‘ ’
    ("\u{201C}", "\u{201C}", "\u{201D}"), // “ → “ ”
    ("\u{2018}", "\u{2018}", "\u{2019}"), // ‘ → ‘ ’
    ("`", "`", "`"),
    ("<", "<", ">"),
    ("\u{300C}", "\u{300C}", "\u{300D}"), // 「 → 「 」
    ("\u{300E}", "\u{300E}", "\u{300F}"), // 『 → 『 』
    ("\u{300A}", "\u{300A}", "\u{300B}"), // 《 → 《 》
    ("\u{3008}", "\u{3008}", "\u{3009}"), // 〈 → 〈 〉
    ("\u{3010}", "\u{3010}", "\u{3011}"), // 【 → 【 】
    ("\u{3014}", "\u{3014}", "\u{3015}"), // 〔 → 〔 〕
    ("*", "*", "*"),
    ("_", "_", "_"),
    ("=", "=", "="),
    ("+", "+", "+"),
    ("-", "-", "-"),
    ("~", "~", "~"),
    ("|", "|", "|"),
    ("^", "^", "^"),
];

impl Runtime {
    pub(crate) fn try_auto_surround(&mut self, text: &str) -> Option<Vec<Effect>> {
        if !self.auto_surround_enabled {
            return None;
        }

        let selection = &self.state.selection;

        if selection.is_collapsed() {
            return None;
        }

        let pair = AUTO_SURROUND_PAIRS
            .iter()
            .find(|(trigger, _, _)| *trigger == text)?;
        let (_, left, right) = *pair;

        Some(self.transact(|tr| tr.surround_selection(left, right)))
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Message;
    use crate::types::Affinity;

    #[test]
    fn auto_surround_parentheses_single_block() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
                paragraph {}
            }
            selection { (p, 6) -> (p, 11) }
        };

        rt.update(Message::Input {
            text: "(".to_string(),
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello (world)" }
                }
                paragraph {}
            }
            selection { (p, 6) -> (p, 13) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn auto_surround_across_blocks() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "hello" }
                }
                @p2 paragraph {
                    text { "world" }
                }
                paragraph {}
            }
            selection { (p1, 2) -> (p2, 3) }
        };

        rt.update(Message::Input {
            text: "[".to_string(),
        });

        let expected = state! {
            doc {
                @p1 paragraph {
                    text { "he[llo" }
                }
                @p2 paragraph {
                    text { "wor]ld" }
                }
                paragraph {}
            }
            selection { (p1, 2) -> (p2, 4) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn no_auto_surround_on_collapsed_selection() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello" }
                }
                paragraph {}
            }
            selection { (p, 2) }
        };

        rt.update(Message::Input {
            text: "(".to_string(),
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "he(llo" }
                }
                paragraph {}
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn no_auto_surround_for_non_trigger() {
        let mut p = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
                paragraph {}
            }
            selection { (p, 6) -> (p, 11) }
        };

        rt.update(Message::Input {
            text: "x".to_string(),
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello x" }
                }
                paragraph {}
            }
            selection { (p, 7, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }
}

use editor_transaction::Transaction;

use crate::CommandResult;
use crate::commands::surround_selection;

const AUTO_SURROUND_PAIRS: &[(&str, &str, &str)] = &[
    ("(", "(", ")"),
    ("[", "[", "]"),
    ("{", "{", "}"),
    ("\"", "\u{201C}", "\u{201D}"),
    ("'", "\u{2018}", "\u{2019}"),
    ("\u{201C}", "\u{201C}", "\u{201D}"),
    ("\u{2018}", "\u{2018}", "\u{2019}"),
    ("`", "`", "`"),
    ("<", "<", ">"),
    ("\u{300C}", "\u{300C}", "\u{300D}"), // 「」
    ("\u{300E}", "\u{300E}", "\u{300F}"), // 『』
    ("\u{300A}", "\u{300A}", "\u{300B}"), // 《》
    ("\u{3008}", "\u{3008}", "\u{3009}"), // 〈〉
    ("\u{3010}", "\u{3010}", "\u{3011}"), // 【】
    ("\u{3014}", "\u{3014}", "\u{3015}"), // 〔〕
    ("*", "*", "*"),
    ("_", "_", "_"),
    ("=", "=", "="),
    ("+", "+", "+"),
    ("-", "-", "-"),
    ("~", "~", "~"),
    ("|", "|", "|"),
    ("^", "^", "^"),
];

pub fn auto_surround(tr: &mut Transaction, text: &str) -> CommandResult {
    let Some((_, left, right)) = AUTO_SURROUND_PAIRS
        .iter()
        .find(|(trigger, _, _)| *trigger == text)
    else {
        return Ok(false);
    };

    surround_selection(tr, left, right)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn parenthesis_surrounds_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0) -> (t1, 11)
        };
        let (actual, ..) = transact!(initial, |tr| auto_surround(&mut tr, "("));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("(hello world)") } } }
            selection: (t1, 0) -> (t1, 13)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn ascii_quote_produces_curly_quotes() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| auto_surround(&mut tr, "\""));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("\u{201C}hello\u{201D}") } } }
            selection: (t1, 0) -> (t1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_match_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        transact_fail!(initial, |tr| auto_surround(&mut tr, "x"));
    }

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| auto_surround(&mut tr, "("));
    }

    #[test]
    fn cjk_bracket_surrounds_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("일이삼") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| auto_surround(&mut tr, "\u{300C}"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("\u{300C}일이삼\u{300D}") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }
}

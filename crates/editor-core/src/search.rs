use editor_model::{Doc, Node};
use editor_state::{Position, Selection};

use crate::message::SearchOptions;

pub fn find_matches(doc: &Doc, query: &str, options: &SearchOptions) -> Vec<Selection> {
    let Some(root) = doc.root() else {
        return Vec::new();
    };
    let query_chars: Vec<char> = query.chars().collect();
    let m = query_chars.len();
    if m == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for desc in root.descendants() {
        let Node::Text(text_node) = desc.node() else {
            continue;
        };
        let chars: Vec<char> = text_node.text.to_string().chars().collect();
        let n = chars.len();
        if m > n {
            continue;
        }
        let mut i = 0;
        while i + m <= n {
            if chars[i..i + m] == query_chars[..] {
                let passes_word = if options.match_whole_word {
                    let before_ok = i == 0 || !is_word_char(chars[i - 1]);
                    let after_ok = i + m == n || !is_word_char(chars[i + m]);
                    before_ok && after_ok
                } else {
                    true
                };
                if passes_word {
                    out.push(Selection::new(
                        Position::new(desc.id(), i),
                        Position::new(desc.id(), i + m),
                    ));
                    i += m;
                    continue;
                }
            }
            i += 1;
        }
    }
    out
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    fn matches_text(state: &editor_state::State, query: &str, whole_word: bool) -> Vec<String> {
        let opts = SearchOptions {
            match_whole_word: whole_word,
        };
        find_matches(&state.doc, query, &opts)
            .into_iter()
            .filter_map(|sel| sel.resolve(&state.doc).map(|r| r.collect_text()))
            .collect()
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };
        assert!(matches_text(&state, "", false).is_empty());
    }

    #[test]
    fn finds_multiple_occurrences_in_single_text_node() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("ab ab ab") } } }
            selection: (t, 0)
        };
        assert_eq!(matches_text(&state, "ab", false), vec!["ab", "ab", "ab"]);
    }

    #[test]
    fn does_not_cross_text_node_boundary() {
        let (state, ..) = state! {
            doc { root { paragraph { a: text("foo") b: text("bar") } } }
            selection: (a, 0)
        };
        assert!(matches_text(&state, "oob", false).is_empty());
    }

    #[test]
    fn whole_word_excludes_partial_matches() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("cat catalog cat") } } }
            selection: (t, 0)
        };
        assert_eq!(matches_text(&state, "cat", true), vec!["cat", "cat"]);
    }

    #[test]
    fn whole_word_includes_punctuation_boundary() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("cat. cat,") } } }
            selection: (t, 0)
        };
        assert_eq!(matches_text(&state, "cat", true), vec!["cat", "cat"]);
    }
}

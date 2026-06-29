use editor_model::{ChildView, DocView};
use editor_state::{Position, Selection};

use crate::message::SearchOptions;

pub fn find_matches(view: &DocView, query: &str, options: &SearchOptions) -> Vec<Selection> {
    let Some(root) = view.root() else {
        return Vec::new();
    };
    let query_chars: Vec<char> = query.chars().collect();
    let m = query_chars.len();
    if m == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for desc in root.descendants() {
        let ChildView::Block(block) = desc else {
            continue;
        };
        let chars: Vec<char> = block.inline_text().chars().collect();
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
                        Position::new(block.id(), i),
                        Position::new(block.id(), i + m),
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
        let view = state.view();
        find_matches(&view, query, &opts)
            .into_iter()
            .filter_map(|sel| sel.resolve(&view).map(|r| r.collect_text()))
            .collect()
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        assert!(matches_text(&state, "", false).is_empty());
    }

    #[test]
    fn finds_multiple_occurrences_in_single_text_node() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab ab ab") } } }
            selection: (p1, 0)
        };
        assert_eq!(matches_text(&state, "ab", false), vec!["ab", "ab", "ab"]);
    }

    #[test]
    fn does_not_cross_block_boundary() {
        // In the eg-walker model there are no text-node boundaries: adjacent runs
        // in one block flatten into a single continuous string, so a query may
        // span them. The boundary search must still not cross is the *block*
        // boundary — matching is per-block over `inline_text`.
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("foo") }
                p2: paragraph { text("bar") }
            } }
            selection: (p1, 0)
        };
        assert!(matches_text(&state, "oob", false).is_empty());
    }

    #[test]
    fn whole_word_excludes_partial_matches() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("cat catalog cat") } } }
            selection: (p1, 0)
        };
        assert_eq!(matches_text(&state, "cat", true), vec!["cat", "cat"]);
    }

    #[test]
    fn whole_word_includes_punctuation_boundary() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("cat. cat,") } } }
            selection: (p1, 0)
        };
        assert_eq!(matches_text(&state, "cat", true), vec!["cat", "cat"]);
    }
}

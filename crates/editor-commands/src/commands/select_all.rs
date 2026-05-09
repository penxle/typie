use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{find_first_cursor_position, find_last_cursor_position};

pub fn select_all(tr: &mut Transaction) -> CommandResult {
    let doc = tr.doc();
    let root = doc.root().expect("root must exist");

    let start = find_first_cursor_position(&root);
    let end = find_last_cursor_position(&root);

    match (start, end) {
        (Some(start), Some(end)) => {
            tr.set_selection(Selection::new(start, end))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::Position;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn select_all_single_paragraph() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Selection::new(Position::new(t, 0), Position::new(t, 5))
        );
    }

    #[test]
    fn select_all_multiple_paragraphs() {
        let (state, t1, t3) = state! {
            doc {
                root {
                    paragraph { t1: text("hello") }
                    paragraph { text("world") }
                    paragraph { t3: text("!") }
                }
            }
            selection: (t1, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(actual.selection.anchor, Position::new(t1, 0));
        assert_eq!(actual.selection.head, Position::new(t3, 1));
    }

    #[test]
    fn select_all_images_only() {
        let (state, r) = state! {
            doc { r: root { image image image } }
            selection: (r, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(actual.selection.anchor, Position::new(r, 0));
        assert_eq!(actual.selection.head, Position::new(r, 3));
    }

    #[test]
    fn select_all_empty_paragraph() {
        let (state, p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Selection::new(Position::new(p, 0), Position::new(p, 0))
        );
    }
}

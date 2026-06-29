use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::CommandResult;

pub fn select_all(tr: &mut Transaction) -> CommandResult {
    let (root_id, children_count) = {
        let view = tr.view();
        let root = view.root().expect("root must exist");
        (root.id(), root.children().count())
    };

    if children_count == 0 {
        tr.set_selection(Some(Selection::collapsed(Position::new(root_id, 0))))?;
    } else {
        tr.set_selection(Some(Selection::new(
            Position {
                node: root_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: root_id,
                offset: children_count,
                affinity: Affinity::Upstream,
            },
        )))?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Affinity, Position};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn select_all_from_none_selects_entire_doc() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") } } }
            selection: none
        };
        let (actual, ..) = transact!(initial, |tr| select_all(&mut tr));
        assert!(actual.selection.is_some());
        let sel = actual.selection.unwrap();
        assert!(sel.anchor != sel.head, "select_all must produce a range");
    }

    #[test]
    fn select_all_single_paragraph() {
        let (state, r) = state! {
            doc { r: root { paragraph { text("hello") } } }
            selection: (r, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(r, 0),
                Position {
                    node: r,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_all_multiple_paragraphs() {
        let (state, r) = state! {
            doc {
                r: root {
                    paragraph { text("hello") }
                    paragraph { text("world") }
                    paragraph { text("!") }
                }
            }
            selection: (r, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        let sel = actual.selection.unwrap();
        assert_eq!(sel.anchor, Position::new(r, 0));
        assert_eq!(
            sel.head,
            Position {
                node: r,
                offset: 3,
                affinity: Affinity::Upstream,
            },
        );
    }

    #[test]
    fn select_all_images_only() {
        // Root content requires a trailing Paragraph, so projection appends a
        // derived paragraph after the three images → 4 children total.
        let (state, r) = state! {
            doc { r: root { image image image } }
            selection: (r, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        let sel = actual.selection.unwrap();
        assert_eq!(sel.anchor, Position::new(r, 0));
        assert_eq!(
            sel.head,
            Position {
                node: r,
                offset: 4,
                affinity: Affinity::Upstream,
            },
        );
    }

    #[test]
    fn select_all_empty_paragraph() {
        let (state, r) = state! {
            doc { r: root { paragraph {} } }
            selection: (r, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(r, 0),
                Position {
                    node: r,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}

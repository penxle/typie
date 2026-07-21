use editor_transaction::Transaction;

use crate::CommandResult;
use crate::judgments::judge_expand_all;
use crate::types::Verdict;

pub fn select_all(tr: &mut Transaction) -> CommandResult {
    let verdict = {
        let view = tr.view();
        judge_expand_all(&view, tr.selection())
    };
    match verdict {
        Verdict::NotApplicable => Ok(false),
        Verdict::AbsorbOnly => Ok(false),
        Verdict::Change(resolved) => {
            tr.set_selection(Some(resolved))?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::ChildView;
    use editor_state::{Affinity, Position, Selection, State};

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
        let (state, paragraph) = state! {
            doc { root { paragraph: paragraph { text("hello") } } }
            selection: (paragraph, 2)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(paragraph, 0),
                Position {
                    node: paragraph,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_all_multiple_paragraphs() {
        let (state, first, last) = state! {
            doc {
                root {
                    first: paragraph { text("hello") }
                    paragraph { text("world") }
                    last: paragraph { text("!") }
                }
            }
            selection: (first, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        let sel = actual.selection.unwrap();
        assert_eq!(sel.anchor, Position::new(first, 0));
        assert_eq!(
            sel.head,
            Position {
                node: last,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
    }

    #[test]
    fn select_all_descends_through_list_edges() {
        let (state, first, trailing) = state! {
            doc { root {
                bullet_list { list_item { first: paragraph { text("first") } } }
                trailing: paragraph {}
            } }
            selection: (trailing, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(first, 0),
                Position {
                    node: trailing,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_all_uses_derived_trailing_paragraph_cursor() {
        let (state, root) = state! {
            doc { root: root { image image image } }
            selection: (root, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));
        let trailing = {
            let view = actual.view();
            let root = view.node(root).expect("root");
            assert_eq!(root.children().count(), 4);
            let Some(ChildView::Block(trailing)) = root.last_child() else {
                panic!("projection must append a trailing paragraph");
            };
            trailing.id()
        };

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(root, 0),
                Position {
                    node: trailing,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    fn assert_leading_unit_selection(state: State, root: Dot, trailing: Dot) {
        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));
        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(root, 0),
                Position {
                    node: trailing,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_all_uses_parent_boundary_for_leading_units() {
        let (image_state, image_root, image_trailing) = state! {
            doc { image_root: root { image image_trailing: paragraph {} } }
            selection: (image_trailing, 0)
        };
        assert_leading_unit_selection(image_state, image_root, image_trailing);

        let (blockquote_state, blockquote_root, blockquote_trailing) = state! {
            doc { blockquote_root: root {
                blockquote { paragraph { text("quote") } }
                blockquote_trailing: paragraph {}
            } }
            selection: (blockquote_trailing, 0)
        };
        assert_leading_unit_selection(blockquote_state, blockquote_root, blockquote_trailing);

        let (callout_state, callout_root, callout_trailing) = state! {
            doc { callout_root: root {
                callout { paragraph { text("callout") } }
                callout_trailing: paragraph {}
            } }
            selection: (callout_trailing, 0)
        };
        assert_leading_unit_selection(callout_state, callout_root, callout_trailing);

        let (fold_state, fold_root, fold_trailing) = state! {
            doc { fold_root: root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                fold_trailing: paragraph {}
            } }
            selection: (fold_trailing, 0)
        };
        assert_leading_unit_selection(fold_state, fold_root, fold_trailing);

        let (table_state, table_root, table_trailing) = state! {
            doc { table_root: root {
                table { table_row { table_cell { paragraph { text("cell") } } } }
                table_trailing: paragraph {}
            } }
            selection: (table_trailing, 0)
        };
        assert_leading_unit_selection(table_state, table_root, table_trailing);
    }

    #[test]
    fn select_all_empty_paragraph() {
        let (state, paragraph) = state! {
            doc { root { paragraph: paragraph {} } }
            selection: (paragraph, 0)
        };

        let (actual, ..) = transact!(state, |tr| select_all(&mut tr));

        assert_eq!(
            actual.selection,
            Some(Selection::collapsed(Position {
                node: paragraph,
                offset: 0,
                affinity: Affinity::Upstream,
            }))
        );
    }

    #[test]
    fn select_all_is_idempotent_for_canonical_selection() {
        let (state, paragraph) = state! {
            doc { root { paragraph: paragraph { text("hello") } } }
            selection: (paragraph, 0)
        };
        let state = State {
            selection: Some(Selection::new(
                Position::new(paragraph, 0),
                Position {
                    node: paragraph,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            )),
            ..state
        };
        let mut tr = Transaction::new(&state);

        assert!(!select_all(&mut tr).expect("select all succeeds"));
        assert!(!tr.selection_changed());
        assert_eq!(tr.selection(), state.selection);
    }
}

use editor_resource::Resource;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::judgments::judge_expand_word;
use crate::types::Verdict;

pub fn select_word_at(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    let verdict = {
        let view = tr.view();
        judge_expand_word(&view, Some(selection), resource)
    };
    match verdict {
        Verdict::NotApplicable => Ok(false),
        Verdict::AbsorbOnly => Ok(true),
        Verdict::Change(resolved) => {
            tr.set_selection(Some(resolved))?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_resource::Resource;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn select_word_at_sets_resolved_selection() {
        let resource = Resource::new_test();
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(p1, 0),
                Position {
                    node: p1,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_word_at_expands_range_within_same_word() {
        let resource = Resource::new_test();
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 5)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(p1, 0),
                Position {
                    node: p1,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_word_at_rejects_range_across_words() {
        let resource = Resource::new_test();
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 8)
        };
        let before = initial.selection;

        let (actual, ..) = transact_fail!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(actual.selection, before);
    }
}

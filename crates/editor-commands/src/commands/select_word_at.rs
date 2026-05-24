use editor_resource::Resource;
use editor_state::{Selection, resolve_word_selection_expansion};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::set_selection_if_changed;

pub fn select_word_at(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    let doc = tr.doc();
    set_selection_if_changed(
        tr,
        resolve_word_selection_expansion(&doc, selection, resource),
    )
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 2)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(t, 0),
                Position {
                    node_id: t,
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_word_at_expands_range_within_same_word() {
        let resource = Resource::new_test();
        let (initial, t1, t2) = state! {
            doc { root { paragraph { t1: text("hel") t2: text("lo world") } } }
            selection: (t1, 1) -> (t2, 2)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(t1, 0),
                Position {
                    node_id: t2,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_word_at_rejects_range_across_words() {
        let resource = Resource::new_test();
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 8)
        };
        let before = initial.selection;

        let (actual, ..) = transact_fail!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_word_at(&mut tr, selection, &resource)
        });

        assert_eq!(actual.selection, before);
    }
}

use editor_resource::Resource;
use editor_state::{Selection, resolve_sentence_selection_expansion};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::set_selection_if_changed;

pub fn select_sentence_at(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    let doc = tr.doc();
    set_selection_if_changed(
        tr,
        resolve_sentence_selection_expansion(&doc, selection, resource),
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
    fn select_sentence_at_sets_resolved_selection() {
        let resource = Resource::new_test();
        let (initial, t) = state! {
            doc { root { paragraph { t: text("Hello.  Next.") } } }
            selection: (t, 1)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_sentence_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(t, 0),
                Position {
                    node_id: t,
                    offset: 6,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}

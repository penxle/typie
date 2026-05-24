use editor_state::{Selection, resolve_paragraph_selection_expansion};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::set_selection_if_changed;

pub fn select_paragraph_at(tr: &mut Transaction, selection: Selection) -> CommandResult {
    let doc = tr.doc();
    set_selection_if_changed(tr, resolve_paragraph_selection_expansion(&doc, selection))
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn select_paragraph_at_sets_resolved_selection() {
        let (initial, t1, t2) = state! {
            doc { root { paragraph { t1: text("Hello ") t2: text("world!") } } }
            selection: (t1, 3)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_paragraph_at(&mut tr, selection)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(t1, 0),
                Position {
                    node_id: t2,
                    offset: 6,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}

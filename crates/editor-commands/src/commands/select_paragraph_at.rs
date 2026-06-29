use editor_state::Selection;
use editor_state::resolve_paragraph_selection_expansion;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::set_selection_if_changed;

pub fn select_paragraph_at(tr: &mut Transaction, selection: Selection) -> CommandResult {
    let resolved = {
        let view = tr.view();
        resolve_paragraph_selection_expansion(&selection, &view)
    };
    set_selection_if_changed(tr, resolved)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn select_paragraph_at_sets_resolved_selection() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello world!") } } }
            selection: (p1, 3)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_paragraph_at(&mut tr, selection)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(p1, 0),
                Position {
                    node: p1,
                    offset: 12,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}

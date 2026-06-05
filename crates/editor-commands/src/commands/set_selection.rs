use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;

pub fn set_selection(tr: &mut Transaction, selection: Selection) -> CommandResult {
    tr.set_selection(Some(selection))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Affinity, Position};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn set_selection_from_none_creates_selection() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: none
        };
        let new_sel = Selection::collapsed(Position::new(t1, 0));
        let (actual, ..) = transact!(initial, |tr| set_selection(&mut tr, new_sel));
        assert_eq!(actual.selection, Some(new_sel));
    }

    #[test]
    fn set_selection_collapsed() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };

        let target = Selection::collapsed(Position::new(t, 3));
        let (actual, ..) = transact!(state, |tr| set_selection(&mut tr, target));

        assert_eq!(actual.selection, Some(target));
    }

    #[test]
    fn set_selection_range() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };

        let target = Selection::new(Position::new(t, 2), Position::new(t, 8));
        let (actual, ..) = transact!(state, |tr| set_selection(&mut tr, target));

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(t, 2),
                Position {
                    node_id: t,
                    offset: 8,
                    affinity: Affinity::Downstream,
                },
            )),
        );
    }
}

use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;

pub fn set_selection(tr: &mut Transaction, selection: Selection) -> CommandResult {
    tr.set_selection(selection)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::Position;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn set_selection_collapsed() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };

        let target = Selection::collapsed(Position::new(t, 3));
        let (result, ..) = transact!(state, |tr| set_selection(&mut tr, target));

        assert_eq!(result.selection, target);
    }

    #[test]
    fn set_selection_range() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0)
        };

        let target = Selection::new(Position::new(t, 2), Position::new(t, 8));
        let (result, ..) = transact!(state, |tr| set_selection(&mut tr, target));

        assert_eq!(result.selection, target);
    }
}

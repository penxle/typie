use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::judgments::judge_expand_paragraph;
use crate::types::Verdict;

pub fn select_paragraph_at(tr: &mut Transaction, selection: Selection) -> CommandResult {
    let verdict = {
        let view = tr.view();
        judge_expand_paragraph(&view, Some(selection))
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

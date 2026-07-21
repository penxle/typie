use editor_resource::Resource;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::judgments::judge_expand_sentence;
use crate::types::Verdict;

pub fn select_sentence_at(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    let verdict = {
        let view = tr.view();
        judge_expand_sentence(&view, Some(selection), resource)
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
    fn select_sentence_at_sets_resolved_selection() {
        let resource = Resource::new_test();
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello.  Next.") } } }
            selection: (p1, 1)
        };

        let (actual, ..) = transact!(initial, |tr| {
            let selection = tr.selection().unwrap();
            select_sentence_at(&mut tr, selection, &resource)
        });

        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position::new(p1, 0),
                Position {
                    node: p1,
                    offset: 6,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }
}

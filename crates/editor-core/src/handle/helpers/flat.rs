use editor_commands::{self as commands, CommandError, CommandResult};
use editor_schema::ResolvedPositionFlatExt;
use editor_state::{ResolvedPosition, Selection};
use editor_transaction::Transaction;

/// Replace content in flat offset range `[start, end)` with `text`.
///
/// **Precondition:** `start`/`end` must have been validated (both resolvable, within same block).
/// On resolve failure returns `Corrupted` — caller is expected to pre-validate.
pub(crate) fn replace_flat_range(
    tr: &mut Transaction,
    start: usize,
    end: usize,
    text: &str,
) -> CommandResult {
    let doc = tr.doc();
    let start_pos = ResolvedPosition::from_flat(&doc, start).ok_or(CommandError::Corrupted(
        "flat start unresolvable after validation".into(),
    ))?;
    let end_pos = ResolvedPosition::from_flat(&doc, end).ok_or(CommandError::Corrupted(
        "flat end unresolvable after validation".into(),
    ))?;
    let selection = Selection::new((&start_pos).into(), (&end_pos).into());

    commands::chain!(
        tr,
        commands::set_selection(selection),
        commands::optional!(commands::delete_selection()),
        commands::when!(!text.is_empty(), commands::insert_text(text)),
    )
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn insert_at_position() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        replace_flat_range(&mut tr, 5, 5, "!").unwrap();
        let (new_state, ..) = tr.commit();
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello!") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(&new_state, &expected);
    }

    #[test]
    fn delete_only() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        replace_flat_range(&mut tr, 2, 4, "").unwrap();
        let (new_state, ..) = tr.commit();
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("heo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&new_state, &expected);
    }

    #[test]
    fn replace_substring() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        replace_flat_range(&mut tr, 1, 4, "XY").unwrap();
        let (new_state, ..) = tr.commit();
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(&new_state, &expected);
    }

    #[test]
    fn noop_empty_range_empty_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        replace_flat_range(&mut tr, 2, 2, "").unwrap();
        let (new_state, ..) = tr.commit();
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&new_state, &expected);
    }
}

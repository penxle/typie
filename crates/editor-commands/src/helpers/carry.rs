use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType};
use editor_transaction::Transaction;

pub(crate) use editor_state::{block_accepts_carry_kind, end_touched_textblocks};

use crate::CommandError;

pub(crate) fn companion_set(
    tr: &mut Transaction,
    blocks: &[Dot],
    modifier: &Modifier,
) -> Result<(), CommandError> {
    if !modifier.as_type().is_carry_kind() {
        return Ok(());
    }
    for &block in blocks {
        tr.set_carry_modifier(block, modifier.clone())?;
    }
    Ok(())
}

pub(crate) fn companion_unset(
    tr: &mut Transaction,
    blocks: &[Dot],
    ty: ModifierType,
) -> Result<(), CommandError> {
    if !ty.is_carry_kind() {
        return Ok(());
    }
    for &block in blocks {
        tr.remove_carry_modifier(block, ty)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn carry_kind_judgment_excludes_fold_title_for_every_carry_kind() {
        let (initial, p1, ft1) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("body") } }
                }
            } }
            selection: (p1, 0)
        };
        let view = initial.view();
        for ty in ModifierType::iter().filter(|t| t.is_carry_kind()) {
            assert!(
                block_accepts_carry_kind(&view, p1, ty),
                "a paragraph is a carry target for {ty:?}"
            );
            assert!(
                !block_accepts_carry_kind(&view, ft1, ty),
                "a fold title is not a carry target for {ty:?}"
            );
        }
    }
}

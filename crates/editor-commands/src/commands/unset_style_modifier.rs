use editor_model::ModifierType;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::capture_style_entry;

pub fn unset_style_modifier(
    tr: &mut Transaction,
    style_id: String,
    ty: ModifierType,
) -> CommandResult {
    let Some(mut entry) = capture_style_entry(&tr.state().doc, &style_id) else {
        return Ok(false);
    };
    let before_len = entry.modifiers.len();
    entry.modifiers.retain(|m| m.as_type() != ty);
    if entry.modifiers.len() == before_len {
        return Ok(false);
    }
    tr.set_style(style_id, Some(entry))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Modifier, ModifierType};

    use super::*;
    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn removes_modifier_of_type() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![Modifier::Bold, Modifier::FontSize { value: 1600 }],
        ));
        let (after, ..) = transact!(defined, |tr| unset_style_modifier(
            &mut tr,
            "heading-1".into(),
            ModifierType::Bold
        ));
        let style = after.doc.style_entry("heading-1").unwrap();
        assert!(!style.modifiers.contains(&Modifier::Bold));
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 1600 })
        );
    }
}

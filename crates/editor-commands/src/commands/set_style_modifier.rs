use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::helpers::capture_style_entry;
use crate::{CommandError, CommandResult};

pub fn set_style_modifier(
    tr: &mut Transaction,
    style_id: String,
    modifier: Modifier,
) -> CommandResult {
    let Some(mut entry) = capture_style_entry(tr.state(), &style_id) else {
        return Err(CommandError::InvalidArgument(format!(
            "style {style_id:?} not defined"
        )));
    };
    let ty = modifier.as_type();
    let before_len = entry.modifiers.len();
    entry.modifiers.retain(|m| m.as_type() != ty);
    let removed_any = entry.modifiers.len() != before_len;
    let inserted = entry.modifiers.insert(modifier);
    if !inserted && !removed_any {
        return Ok(false);
    }
    tr.set_style(style_id, Some(entry))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn replaces_existing_modifier_of_same_type() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![Modifier::FontSize { value: 1600 }],
        ));
        let (after, ..) = transact!(defined, |tr| set_style_modifier(
            &mut tr,
            "heading-1".into(),
            Modifier::FontSize { value: 2400 }
        ));
        let style = after.projected.styles().style_entry("heading-1").unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );
        assert!(
            !style
                .modifiers
                .contains(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn adds_when_no_existing_of_type() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![],
        ));
        let (after, ..) = transact!(defined, |tr| set_style_modifier(
            &mut tr,
            "heading-1".into(),
            Modifier::Bold
        ));
        let style = after.projected.styles().style_entry("heading-1").unwrap();
        assert!(style.modifiers.contains(&Modifier::Bold));
    }
}

use editor_model::{Modifier, PlainStyleEntry};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn define_style(
    tr: &mut Transaction,
    style_id: String,
    name: String,
    modifiers: Vec<Modifier>,
) -> CommandResult {
    if style_id.is_empty() {
        return Err(CommandError::InvalidArgument(
            "style_id must not be empty".into(),
        ));
    }
    let entry = PlainStyleEntry {
        name,
        modifiers: modifiers.into_iter().collect(),
    };
    tr.set_style(style_id, Some(entry))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn registers_presence_name_and_modifiers() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![Modifier::Bold, Modifier::FontSize { value: 2400 }],
        ));

        assert!(actual.doc.style_present("heading-1"));

        let style = actual.doc.style_entry("heading-1").unwrap();
        assert_eq!(style.name.get(), "제목 1");
        assert!(style.modifiers.contains(&Modifier::Bold));
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontSize { value: 2400 })
        );
    }

    #[test]
    fn empty_style_id_errors() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| define_style(
            &mut tr,
            "".into(),
            "x".into(),
            vec![]
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}

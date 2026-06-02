use editor_transaction::Transaction;

use crate::helpers::capture_style_entry;
use crate::{CommandError, CommandResult};

pub fn rename_style(tr: &mut Transaction, style_id: String, new_name: String) -> CommandResult {
    let Some(mut entry) = capture_style_entry(&tr.state().doc, &style_id) else {
        return Err(CommandError::InvalidArgument(format!(
            "style {style_id:?} not defined"
        )));
    };
    if entry.name == new_name {
        return Ok(false);
    }
    entry.name = new_name;
    tr.set_style(style_id, Some(entry))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn updates_name() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![],
        ));
        let (renamed, ..) = transact!(defined, |tr| rename_style(
            &mut tr,
            "heading-1".into(),
            "Heading One".into()
        ));
        assert_eq!(
            renamed.doc.style_entry("heading-1").unwrap().name.get(),
            "Heading One"
        );
    }
}

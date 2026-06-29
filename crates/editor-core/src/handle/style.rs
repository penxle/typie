use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_style_op(editor: &mut Editor, op: StyleOp) -> Result<(), EditorError> {
    match op {
        StyleOp::ApplyToSelection { style_id } => editor.transact(|tr| {
            commands::apply_style_to_selection(tr, style_id)?;
            Ok(())
        }),
        StyleOp::UnsetInSelection => editor.transact(|tr| {
            commands::unset_style_in_selection(tr)?;
            Ok(())
        }),
        StyleOp::CreateFromSelection { style_id, name } => editor.transact(|tr| {
            commands::create_style_from_selection(tr, style_id, name)?;
            Ok(())
        }),
        StyleOp::UpdateFromSelection => editor.transact(|tr| {
            commands::update_style_from_selection(tr)?;
            Ok(())
        }),
        StyleOp::Define {
            style_id,
            name,
            modifiers,
        } => editor.transact(|tr| {
            commands::define_style(tr, style_id, name, modifiers)?;
            Ok(())
        }),
        StyleOp::Delete { style_id } => editor.transact(|tr| {
            commands::delete_style(tr, style_id)?;
            Ok(())
        }),
        StyleOp::Rename { style_id, name } => editor.transact(|tr| {
            commands::rename_style(tr, style_id, name)?;
            Ok(())
        }),
        StyleOp::SetModifier { style_id, modifier } => editor.transact(|tr| {
            commands::set_style_modifier(tr, style_id, modifier)?;
            Ok(())
        }),
        StyleOp::UnsetModifier {
            style_id,
            modifier_type,
        } => editor.transact(|tr| {
            commands::unset_style_modifier(tr, style_id, modifier_type)?;
            Ok(())
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::editor::Editor;
    use crate::event::EditorEvent;
    use crate::state_field::StateField;

    #[test]
    fn collapsed_apply_style_notifies_styles_field() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);

        let events = editor.apply(Message::Style {
            op: StyleOp::ApplyToSelection {
                style_id: "s1".to_string(),
            },
        });

        assert_eq!(
            editor.state().pending_style,
            Some(editor_state::PendingStyle::Set {
                style_id: "s1".to_string()
            })
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Styles)
            )),
            "collapsed apply_style must mark StateField::Styles dirty, got {:?}",
            events
        );
    }

    #[test]
    fn collapsed_unset_style_notifies_styles_field() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);

        let events = editor.apply(Message::Style {
            op: StyleOp::UnsetInSelection,
        });

        assert_eq!(
            editor.state().pending_style,
            Some(editor_state::PendingStyle::Unset)
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Styles)
            )),
            "collapsed unset_style must mark StateField::Styles dirty, got {:?}",
            events
        );
    }
}

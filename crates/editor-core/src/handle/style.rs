use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_style_op(editor: &mut Editor, op: StyleOp) -> Result<(), EditorError> {
    match op {
        StyleOp::Apply { node_id, style_id } => editor.transact(|tr| {
            commands::apply_style(tr, node_id, style_id)?;
            Ok(())
        }),
        StyleOp::Unapply { node_id, style_id } => editor.transact(|tr| {
            commands::unapply_style(tr, node_id, style_id)?;
            Ok(())
        }),
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

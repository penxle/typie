use std::sync::Arc;

use editor_commands::{self as commands};
use editor_model::Modifier;
use editor_resource::{Resource, match_weight};

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
        } => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            let modifiers = normalize_define_modifiers(modifiers, &resource);
            editor.transact(|tr| {
                commands::define_style(tr, style_id, name, modifiers)?;
                Ok(())
            })
        }
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

fn normalize_define_modifiers(modifiers: Vec<Modifier>, resource: &Resource) -> Vec<Modifier> {
    let Some(family) = modifiers.iter().find_map(|modifier| match modifier {
        Modifier::FontFamily { value } => Some(value.as_str()),
        _ => None,
    }) else {
        return modifiers;
    };
    let Some(weights) = resource
        .font_registry
        .weights(family)
        .filter(|w| !w.is_empty())
    else {
        return modifiers;
    };

    let Some(old_weight) = modifiers.iter().find_map(|modifier| match modifier {
        Modifier::FontWeight { value } => Some(*value),
        _ => None,
    }) else {
        return modifiers;
    };

    let Some(new_weight) = match_weight(weights, old_weight) else {
        return modifiers;
    };
    if new_weight == old_weight {
        return modifiers;
    }

    modifiers
        .into_iter()
        .map(|modifier| match modifier {
            Modifier::FontWeight { .. } => Modifier::FontWeight { value: new_weight },
            modifier => modifier,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use editor_macros::state;
    use editor_model::Modifier;
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource};

    use super::*;
    use crate::editor::Editor;
    use crate::event::EditorEvent;
    use crate::state_field::StateField;

    fn make_resource(families: impl IntoIterator<Item = (&'static str, Vec<u16>)>) -> Resource {
        let mut resource = Resource::new_test();
        resource.set_fonts(
            families
                .into_iter()
                .map(|(name, weights)| FontFamily {
                    name: name.to_string(),
                    source: FontFamilySource::Default,
                    weights: weights
                        .into_iter()
                        .map(|value| FontWeight {
                            value,
                            hash: format!("{name}-{value}"),
                            chunks: vec![vec![0x0000, 0xFFFF]],
                        })
                        .collect(),
                })
                .collect(),
        );
        resource
    }

    #[test]
    fn define_style_matches_font_weight_for_family() {
        let resource = Arc::new(Mutex::new(make_resource([("LightFont", vec![100, 300])])));
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.apply(Message::Style {
            op: StyleOp::Define {
                style_id: "heading".to_string(),
                name: "Heading".to_string(),
                modifiers: vec![
                    Modifier::FontFamily {
                        value: "LightFont".to_string(),
                    },
                    Modifier::FontWeight { value: 700 },
                ],
            },
        });

        let style = editor
            .state()
            .projected
            .styles()
            .style_entry("heading")
            .unwrap();
        assert!(style.modifiers.contains(&Modifier::FontFamily {
            value: "LightFont".to_string()
        }));
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontWeight { value: 300 })
        );
        assert!(!style.modifiers.contains(&Modifier::Bold));
        assert!(
            !style
                .modifiers
                .contains(&Modifier::FontWeight { value: 700 })
        );
    }

    #[test]
    fn define_style_preserves_explicit_bold_modifier() {
        let resource = Arc::new(Mutex::new(make_resource([("BoldFont", vec![400, 700])])));
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.apply(Message::Style {
            op: StyleOp::Define {
                style_id: "heading".to_string(),
                name: "Heading".to_string(),
                modifiers: vec![
                    Modifier::FontFamily {
                        value: "BoldFont".to_string(),
                    },
                    Modifier::FontWeight { value: 400 },
                    Modifier::Bold,
                ],
            },
        });

        let style = editor
            .state()
            .projected
            .styles()
            .style_entry("heading")
            .unwrap();
        assert!(
            style
                .modifiers
                .contains(&Modifier::FontWeight { value: 400 })
        );
        assert!(style.modifiers.contains(&Modifier::Bold));
        assert!(
            !style
                .modifiers
                .contains(&Modifier::FontWeight { value: 700 })
        );
    }

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

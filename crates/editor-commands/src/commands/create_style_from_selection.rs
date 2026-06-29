use editor_model::{ModifierType, PlainStyleEntry};
use editor_transaction::Transaction;

use crate::helpers::{
    clear_inline_modifier_types_in_selection, collect_uniform_text_modifiers_in_selection,
};
use crate::{CommandError, CommandResult};

pub fn create_style_from_selection(
    tr: &mut Transaction,
    style_id: String,
    name: String,
) -> CommandResult {
    if style_id.is_empty() {
        return Err(CommandError::InvalidArgument(
            "style_id must not be empty".into(),
        ));
    }

    let run_dots = crate::helpers::collect_run_nodes_in_selection(tr)?;
    if run_dots.is_empty() {
        return Ok(false);
    }

    let modifiers = collect_uniform_text_modifiers_in_selection(tr.state());
    let modifier_types: Vec<ModifierType> = modifiers.iter().map(|m| m.as_type()).collect();

    tr.set_style(
        style_id.clone(),
        Some(PlainStyleEntry {
            name,
            modifiers: modifiers.into_iter().collect(),
        }),
    )?;

    for elem in &run_dots {
        tr.set_node_style(*elem, Some(style_id.clone()))?;
    }

    clear_inline_modifier_types_in_selection(tr, &modifier_types)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::helpers::capture_style_entry;
    use crate::test_utils::*;

    #[test]
    fn captures_uniform_inline_modifiers_into_new_style() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800), text_color("#ff00ff".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "핑크임".into(),
        ));

        let style = capture_style_entry(&actual, "s1").unwrap();
        assert_eq!(style.name, "핑크임");
        assert!(style.modifiers.contains(&Modifier::FontSize { value: 800 }));
        assert!(style.modifiers.contains(&Modifier::TextColor {
            value: "#ff00ff".into()
        }));
    }

    #[test]
    fn ignores_zero_width_boundary_text_when_capturing_uniform_modifiers() {
        let (initial, ..) = state! {
            doc {
                root {
                    p: paragraph {
                        text("a") [bold]
                        text("b") [italic]
                    }
                }
            }
            selection: (p, 0) -> (p, 1)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into()
        ));

        let style = capture_style_entry(&actual, "s1").unwrap();
        assert!(style.modifiers.contains(&Modifier::Bold));
        assert!(!style.modifiers.contains(&Modifier::Italic));
    }

    #[test]
    fn sets_style_ref_on_selected_runs() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into()
        ));

        let (expected, ..) = state! {
            doc {
                styles { s1: "x" [font_size(800)] }
                root { p1: paragraph { text("Hello") @s1 } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn clears_inline_modifiers_moved_into_style() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [font_size(800), text_color("#ff00ff".to_string())] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));

        let (expected, ..) = state! {
            doc {
                styles { s1: "x" [font_size(800), text_color("#ff00ff".to_string())] }
                root { p1: paragraph { text("Hello") @s1 } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn drops_mixed_modifiers() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {
                text("Hello") [font_size(800)]
                text("World") [font_size(1600)]
            } } }
            selection: (p, 0) -> (p, 10)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));
        let style = capture_style_entry(&actual, "s1").unwrap();
        assert!(
            !style
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { .. })),
            "mixed font_size should be dropped from style"
        );
    }

    #[test]
    fn registers_style_presence_on_root() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| create_style_from_selection(
            &mut tr,
            "s1".into(),
            "x".into(),
        ));
        assert!(capture_style_entry(&actual, "s1").is_some());
    }

    #[test]
    fn empty_style_id_errors() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let err = transact_err!(initial, |tr| create_style_from_selection(
            &mut tr,
            "".into(),
            "x".into(),
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }
}

use std::collections::BTreeSet;

use editor_crdt::Dot;
use editor_model::{DocView, Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers};
use editor_transaction::Transaction;
use strum::IntoEnumIterator;

use crate::helpers::{is_text_applicable, span_dots};
use crate::{CommandError, CommandResult};

pub fn clear_all_modifiers(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        return clear_all_modifiers_collapsed(tr);
    }
    clear_all_modifiers_range(tr)
}

fn placeholder_modifier(ty: ModifierType) -> Modifier {
    match ty {
        ModifierType::Bold => Modifier::Bold,
        ModifierType::Italic => Modifier::Italic,
        ModifierType::Underline => Modifier::Underline,
        ModifierType::Strikethrough => Modifier::Strikethrough,
        ModifierType::FontSize => Modifier::FontSize { value: 0 },
        ModifierType::FontFamily => Modifier::FontFamily {
            value: String::new(),
        },
        ModifierType::FontWeight => Modifier::FontWeight { value: 0 },
        ModifierType::TextColor => Modifier::TextColor {
            value: String::new(),
        },
        ModifierType::BackgroundColor => Modifier::BackgroundColor {
            value: String::new(),
        },
        ModifierType::LetterSpacing => Modifier::LetterSpacing { value: 0 },
        ModifierType::Link => Modifier::Link {
            href: String::new(),
        },
        ModifierType::Ruby => Modifier::Ruby {
            text: String::new(),
        },
        other => unreachable!("{other:?} is not a text-applicable inline modifier"),
    }
}

fn caret_own_text_types(view: &DocView, pos_node: Dot, offset: usize) -> Vec<ModifierType> {
    let Some(node) = view.node(pos_node) else {
        return Vec::new();
    };
    let idx = offset.saturating_sub(1);
    match node.leaf_state_at(idx) {
        Some(st) => st
            .own
            .iter()
            .filter(|(_, o)| !o.from_style)
            .map(|(t, _)| *t)
            .filter(|&t| is_text_applicable(t))
            .collect(),
        None => Vec::new(),
    }
}

fn clear_all_modifiers_collapsed(tr: &mut Transaction) -> CommandResult {
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    let mut types: BTreeSet<ModifierType> = {
        let view = tr.view();
        caret_own_text_types(&view, pos.node, pos.offset)
            .into_iter()
            .collect()
    };

    for pm in tr.pending_modifiers() {
        match pm {
            PendingModifier::Set { modifier } if is_text_applicable(modifier.as_type()) => {
                types.insert(modifier.as_type());
            }
            PendingModifier::Unset { ty } if is_text_applicable(*ty) => {
                types.remove(ty);
            }
            _ => {}
        }
    }

    if types.is_empty() {
        return Ok(false);
    }

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| !types.contains(&pm.as_type()))
        .cloned()
        .collect();
    for ty in types {
        pending.push(PendingModifier::Unset { ty });
    }

    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

fn clear_all_modifiers_range(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");

    let (first, last) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        match span_dots(&view, &rs) {
            Some((first, last)) => (first, last),
            _ => return Ok(false),
        }
    };

    for ty in ModifierType::iter().filter(|&t| is_text_applicable(t)) {
        tr.remove_span_modifier(first, last, placeholder_modifier(ty))?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn range_clears_inline_on_single_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_preserves_block_modifiers() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph [line_height(200)] {
                        text("Hello") [italic, font_size(2400)]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph [line_height(200)] { text("Hello") }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_font_weight_falls_back_to_inherited() {
        let (initial, ..) = state! {
            doc {
                root [font_weight(400)] {
                    p1: paragraph { text("Hello") [font_weight(700)] }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_removes_link() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Click") [link(href: "https://example.com".to_string())]
                    }
                }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Click") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_partial_selection_clears_middle() {
        let (initial, ..) = state! {
            doc { root { p: paragraph { text("HelloWorld") [italic] } } }
            selection: (p, 2) -> (p, 7)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p: paragraph {
                        text("He") [italic]
                        text("lloWo")
                        text("rld") [italic]
                    }
                }
            }
            selection: (p, 2) -> (p, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_clears_across_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") [bold] }
                    p2: paragraph { text("World") [bold] }
                }
            }
            selection: (p1, 0) -> (p2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p1, 0) -> (p2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_backward_selection_clears() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 5) -> (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5) -> (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_partial_selection_no_modifiers_is_noop() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_clears_font_size_on_tab() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p: paragraph {
                        text("a") [font_size(2400)]
                        tab [font_size(2400)]
                        text("b") [font_size(2400)]
                    }
                }
            }
            selection: (p, 0) -> (p, 3)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p: paragraph {
                        text("a")
                        tab
                        text("b")
                    }
                }
            }
            selection: (p, 0) -> (p, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_clears_own_inline_into_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 2)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_plain_cursor_excludes_inherited_is_noop() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| clear_all_modifiers(&mut tr));
    }

    #[test]
    fn collapsed_clears_own_and_pending_inline_preserving_block() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph [line_height(200)] { text("Hello") [bold] }
                }
            }
            selection: (p1, 2)
            pending_modifiers: [underline]
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph [line_height(200)] { text("Hello") [bold] }
                }
            }
            selection: (p1, 2)
            pending_modifiers: [!bold, !underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_clears_pending_set_of_same_type() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }
}

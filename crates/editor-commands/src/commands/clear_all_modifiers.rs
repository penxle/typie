use std::collections::BTreeSet;

use editor_crdt::Dot;
use editor_model::{DocView, ModifierType};
use editor_state::{PendingModifier, PendingModifiers};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::clear_all_modifiers_range;

pub fn clear_all_modifiers(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        return clear_all_modifiers_collapsed(tr);
    }
    clear_all_modifiers_range(tr, selection)
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
            .map(|(t, _)| *t)
            .filter(|&t| t.is_carry_kind())
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
            PendingModifier::Set { modifier } if modifier.as_type().is_carry_kind() => {
                types.insert(modifier.as_type());
            }
            PendingModifier::Unset { ty } if ty.is_carry_kind() => {
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
    fn collapsed_clear_all_excludes_link_and_ruby_from_pending() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hi") [bold, link(href: "https://a.com".to_string()), ruby(text: "x".to_string())]
                    }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let types: Vec<ModifierType> = actual
            .pending_modifiers
            .iter()
            .map(|pm| pm.as_type())
            .collect();
        assert!(
            types.contains(&ModifierType::Bold),
            "carry-kind own is cleared into pending"
        );
        assert!(
            !types.contains(&ModifierType::Link),
            "link own must not enter pending"
        );
        assert!(
            !types.contains(&ModifierType::Ruby),
            "ruby own must not enter pending"
        );
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

    #[test]
    fn range_clear_removes_all_carry_kinds_at_paragraph_end() {
        let (initial, p1) = state! {
            doc { root {
                p1: paragraph carry([bold, italic, font_size(2400)]) {
                    text("Hello") [bold, italic]
                }
            } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        assert!(
            actual.projected.carry_modifiers(p1).is_empty(),
            "clear-all at the paragraph end wipes every carry-kind record"
        );
    }
}

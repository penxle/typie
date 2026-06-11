use editor_model::{Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers, Position};
use editor_transaction::Transaction;

use crate::helpers::{
    collect_text_nodes_in_range, compact_textblocks_for_nodes, is_text_applicable,
    resolve_effective_modifiers,
};
use crate::{CommandError, CommandResult};

pub fn clear_all_modifiers(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.is_collapsed() {
        return clear_all_modifiers_collapsed(tr);
    }
    clear_all_modifiers_range(tr)
}

fn clear_all_modifiers_collapsed(tr: &mut Transaction) -> CommandResult {
    let pos = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let effective = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    let types: Vec<ModifierType> = effective
        .iter()
        .map(|m| m.as_type())
        .filter(|&ty| is_text_applicable(ty))
        .collect();

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
    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;
    if node_ids.is_empty() {
        return Ok(false);
    }

    for &node_id in &node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        let to_remove: Vec<Modifier> = node
            .explicit_modifiers()
            .filter(|m| is_text_applicable(m.as_type()))
            .cloned()
            .collect();
        for modifier in to_remove {
            tr.remove_modifier(node_id, modifier)?;
        }
    }

    compact_textblocks_for_nodes(tr, &node_ids)?;
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
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_preserves_block_modifiers() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph [line_height(200)] {
                        t1: text("Hello") [italic, font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph [line_height(200)] { t1: text("Hello") }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_font_weight_falls_back_to_inherited() {
        let (initial, ..) = state! {
            doc {
                root [font_weight(400)] {
                    paragraph { t1: text("Hello") [font_weight(700)] }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root [font_weight(400)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_removes_link() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Click") [link(href: "https://example.com".to_string())]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Click") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_partial_selection_splits_and_clears_middle() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("HelloWorld") [italic] } } }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("He") [italic]
                        t1: text("lloWo")
                        t2: text("rld") [italic]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_clears_across_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") [bold] }
                    paragraph { t2: text("World") [bold] }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    // Backward selection keeps anchor/head direction after inline cleanup.
    #[test]
    fn range_backward_selection_clears() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 5) -> (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5) -> (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    // Regression guard: a partial selection over modifier-free text triggers the
    // boundary splits inside `collect_text_nodes_in_range`. Because the range
    // path always reaches `compact_textblocks_for_nodes` (no early return),
    // those splits are merged back and the document is left unchanged. If an
    // early-return ever skips compact, this fails with a fragmented text run.
    #[test]
    fn range_partial_selection_no_modifiers_does_not_fragment() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_clears_font_size_on_tab() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("a") [font_size(2400)]
                        tab [font_size(2400)]
                        t2: text("b") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));

        let tab_keeps_font_size = actual.doc.root().unwrap().descendants().any(|n| {
            matches!(n.node(), editor_model::Node::Tab(_))
                && n.explicit_modifiers()
                    .any(|m| matches!(m, Modifier::FontSize { .. }))
        });
        assert!(
            !tab_keeps_font_size,
            "clear_all must remove the tab's explicit font_size"
        );
    }

    #[test]
    fn collapsed_clears_own_inline_into_pending() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 2)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_plain_cursor_excludes_inherited_is_noop() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| clear_all_modifiers(&mut tr));
    }

    // own = [Bold]; pending Set{Underline} folds into effective, so both Bold and
    // Underline are text-applicable effective types → both get Unset, the pending
    // Set{Underline} is dropped. The paragraph's line_height is preserved because
    // the algorithm only mutates pending, never the doc tree.
    #[test]
    fn collapsed_clears_own_and_pending_inline_preserving_block() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph [line_height(200)] { t1: text("Hello") [bold] }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [underline]
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph [line_height(200)] { t1: text("Hello") [bold] }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [!bold, !underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_clears_pending_set_of_same_type() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| clear_all_modifiers(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }
}

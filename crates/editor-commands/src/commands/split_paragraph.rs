use editor_model::{Marker, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::carryable_modifiers_at;
use crate::{CommandError, CommandResult};

pub fn split_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (parent_id, block_index, carryable) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        let parent_id = parent.id();
        let block_index = parent
            .child_blocks()
            .position(|b| b.id() == pos.node)
            .ok_or_else(|| CommandError::orphan_child(pos.node, parent_id))?;
        let carryable = carryable_modifiers_at(&view, pos, tr.pending_modifiers());
        (parent_id, block_index, carryable)
    };

    tr.split_node(pos.node, pos.offset)?;

    let new_paragraph_id = {
        let view = tr.state().view();
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        parent
            .child_blocks()
            .nth(block_index + 1)
            .map(|b| b.id())
            .ok_or_else(|| CommandError::Corrupted("split produced no new sibling".into()))?
    };

    let marker = Marker {
        modifiers: carryable,
    };
    if !marker.is_empty() {
        tr.set_marker(new_paragraph_id, Some(marker))?;
    }

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_paragraph_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| split_paragraph(&mut tr));
    }

    #[test]
    fn returns_false_in_empty_fold_title() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title {}
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 0)
        };
        transact_fail!(initial, |tr| split_paragraph(&mut tr));
    }

    #[test]
    fn returns_false_in_fold_title_with_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title { text("Title") }
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 2)
        };
        transact_fail!(initial, |tr| split_paragraph(&mut tr));
    }

    #[test]
    fn split_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    p2: paragraph { text("Hello") }
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("He") }
                    p2: paragraph { text("llo") }
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_with_multiple_children() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_preserves_align() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph [alignment(Alignment::Center)] {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph [alignment(Alignment::Center)] { text("He") }
                    p2: paragraph [alignment(Alignment::Center)] { text("llo") }
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_with_hard_break() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        hard_break
                        text("World")
                    }
                }
            }
            selection: (p1, 6)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") hard_break }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn split_at_end_of_bold_text_attaches_marker_to_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let dot = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .nth(1)
                .expect("second paragraph exists")
                .dot()
                .unwrap()
        };
        let marker = actual
            .projected
            .node_markers()
            .value_of(dot)
            .expect("marker on new paragraph");
        assert!(marker.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(dot)
                .is_empty()
        );
    }

    #[test]
    fn split_in_middle_of_bold_text_attaches_marker_to_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let dot = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .nth(1)
                .expect("second paragraph exists")
                .dot()
                .unwrap()
        };
        let marker = actual
            .projected
            .node_markers()
            .value_of(dot)
            .expect("marker on new paragraph");
        assert!(marker.modifiers.iter().any(|m| matches!(m, Modifier::Bold)));
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(dot)
                .is_empty()
        );
    }

    #[test]
    fn split_at_start_of_bold_text_attaches_no_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let dot = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .next()
                .expect("first paragraph exists")
                .dot()
                .unwrap()
        };
        assert!(actual.projected.node_markers().value_of(dot).is_none());
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(dot)
                .is_empty()
        );
    }

    #[test]
    fn split_carries_font_family_and_weight() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("Arial".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let dot = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .nth(1)
                .expect("second paragraph exists")
                .dot()
                .unwrap()
        };
        let marker = actual
            .projected
            .node_markers()
            .value_of(dot)
            .expect("marker on new paragraph");
        assert!(marker.modifiers.iter().any(|m| matches!(
            m,
            Modifier::FontFamily { value } if value == "Arial"
        )));
        assert!(
            marker
                .modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn split_does_not_carry_link() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Click") [link(href: "https://e.com".to_string())] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let dot = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .nth(1)
                .expect("second paragraph exists")
                .dot()
                .unwrap()
        };
        let marker = actual.projected.node_markers().value_of(dot);
        assert!(marker.is_none_or(|m| {
            !m.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::Link { .. }))
        }));
    }
}

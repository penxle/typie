use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::carryable_modifiers_at;
use crate::{CommandError, CommandResult};

pub fn split_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;
    let carryable = carryable_modifiers_at(&doc, pos, tr.pending_modifiers());

    let new_paragraph_id = NodeId::new();

    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            if !matches!(parent.node(), Node::Paragraph(_)) {
                return Ok(false);
            }
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;
            let text_len = text_node.text.len();

            let split_index = if pos.offset == 0 {
                node_index
            } else if pos.offset == text_len {
                node_index + 1
            } else {
                let split_text_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_text_id)?;
                node_index + 1
            };

            tr.split_node(parent.id(), split_index, new_paragraph_id)?;
        }
        Node::Paragraph(_) => {
            tr.split_node(pos.node_id, pos.offset, new_paragraph_id)?;
        }
        _ => return Ok(false),
    }

    for m in carryable {
        tr.add_modifier(new_paragraph_id, m)?;
    }

    let doc = tr.doc();
    let new_paragraph = doc
        .node(new_paragraph_id)
        .ok_or(CommandError::NodeNotFound(new_paragraph_id))?;

    let new_selection = match new_paragraph.first_child() {
        Some(child) if matches!(child.node(), Node::Text(_)) => Selection::collapsed(Position {
            node_id: child.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        }),
        _ => Selection::collapsed(Position {
            node_id: new_paragraph_id,
            offset: 0,
            affinity: Affinity::Downstream,
        }),
    };
    tr.set_selection(Some(new_selection))?;

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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
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
                        fold_title { t1: text("Title") }
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| split_paragraph(&mut tr));
    }

    #[test]
    fn split_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("He") }
                    paragraph { t2: text("llo") }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
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
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_preserves_align() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph [alignment(Alignment::Center)] {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph [alignment(Alignment::Center)] { t1: text("He") }
                    paragraph [alignment(Alignment::Center)] { t2: text("llo") }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_with_hard_break() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                        hard_break
                        t2: text("World")
                    }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") hard_break }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn split_at_end_of_bold_text_attaches_marker_to_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") [bold] }
                    p2: paragraph [bold] {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_in_middle_of_bold_text_attaches_marker_to_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("He") [bold] }
                    paragraph [bold] { t2: text("llo") [bold] }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_at_start_of_bold_text_attaches_no_marker() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    paragraph { t1: text("Hello") [bold] }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_carries_font_family_and_weight() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello") [font_family("Arial".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let new_paragraph = actual
            .doc
            .root()
            .unwrap()
            .children()
            .nth(1)
            .expect("second paragraph exists");
        let mods: Vec<_> = new_paragraph.modifiers().cloned().collect();
        assert!(mods.iter().any(|m| matches!(
            m,
            Modifier::FontFamily { value } if value == "Arial"
        )));
        assert!(
            mods.iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn split_does_not_carry_link() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://e.com".to_string())] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let new_paragraph = actual
            .doc
            .root()
            .unwrap()
            .children()
            .nth(1)
            .expect("second paragraph exists");
        assert!(
            !new_paragraph
                .modifiers()
                .any(|m| matches!(m, Modifier::Link { .. }))
        );
    }
}

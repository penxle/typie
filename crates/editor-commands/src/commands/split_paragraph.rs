use editor_model::NodeType;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{continuation_paint_at, materialize_caret_block};
use crate::{CommandError, CommandResult};

pub fn split_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    {
        let view = tr.state().view();
        let node = view
            .node(selection.head.node)
            .ok_or(CommandError::NodeNotFound(selection.head.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
    }

    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let pos = selection.head;

    let (parent_id, block_index, paint) = {
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
        let paint = continuation_paint_at(&tr.state().projected, pos);
        (parent_id, block_index, paint)
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

    tr.replace_carry(pos.node, paint.clone())?;
    tr.replace_carry(new_paragraph_id, paint)?;

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
    use editor_model::{Modifier, ModifierType, NodeType};
    use editor_state::{Affinity, Position, Selection};
    use editor_transaction::Transaction;

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
    fn returns_false_in_synthetic_fold_title_without_materializing() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold
                    paragraph {}
                }
            }
            selection: none
        };
        let synth_title = {
            let view = initial.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldTitle)
                .map(|b| b.id())
                .expect("synthetic fold title")
        };
        assert!(synth_title.is_synthetic());

        let mut tr = Transaction::new(&initial);
        let selection = Selection::collapsed(Position {
            node: synth_title,
            offset: 0,
            affinity: Affinity::Downstream,
        });
        tr.set_selection(Some(selection)).unwrap();
        assert!(!split_paragraph(&mut tr).unwrap());
        let (actual, ..) = tr.commit();

        let mut expected = initial;
        expected.selection = Some(selection);
        assert_state_eq!(&actual, &expected);
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
    fn materializes_synthetic_trailing_paragraph_before_split() {
        let (initial, ..) = state! {
            doc { root { image } }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(
            synth_p.is_synthetic(),
            "trailing paragraph must be synthetic"
        );

        let mut tr = Transaction::new(&initial);
        tr.set_selection(Some(Selection::collapsed(Position {
            node: synth_p,
            offset: 0,
            affinity: Affinity::Downstream,
        })))
        .unwrap();
        assert!(split_paragraph(&mut tr).unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc {
                root {
                    image
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
    fn split_at_end_of_bold_text_attaches_carry_to_new_paragraph() {
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
        let carry = actual.projected.carry_modifiers(dot);
        assert!(
            carry.contains_key(&ModifierType::Bold),
            "new paragraph carries Bold"
        );
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(dot)
                .is_empty()
        );
    }

    #[test]
    fn split_in_middle_of_bold_text_attaches_carry_to_new_paragraph() {
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
        let carry = actual.projected.carry_modifiers(dot);
        assert!(
            carry.contains_key(&ModifierType::Bold),
            "new paragraph carries Bold"
        );
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(dot)
                .is_empty()
        );
    }

    #[test]
    fn split_at_start_of_bold_text_carries_right_paint_to_both() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (first, second) = both_paragraphs(&actual);
        assert!(
            actual
                .projected
                .carry_modifiers(first)
                .contains_key(&ModifierType::Bold)
        );
        assert!(
            actual
                .projected
                .carry_modifiers(second)
                .contains_key(&ModifierType::Bold)
        );
        assert!(
            actual
                .projected
                .block_modifiers()
                .modifiers_of(first)
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
        let carry = actual.projected.carry_modifiers(dot);
        assert!(carry.values().any(|m| matches!(
            m,
            Modifier::FontFamily { value } if value == "Arial"
        )));
        assert!(
            carry
                .values()
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
        let carry = actual.projected.carry_modifiers(dot);
        assert!(!carry.contains_key(&ModifierType::Link));
    }

    fn both_paragraphs(state: &editor_state::State) -> (editor_crdt::Dot, editor_crdt::Dot) {
        let view = state.view();
        let root = view.root().expect("root");
        let mut blocks = root.child_blocks();
        let first = blocks.next().expect("first paragraph").dot().unwrap();
        let second = blocks.next().expect("second paragraph").dot().unwrap();
        (first, second)
    }

    #[test]
    fn split_replaces_carry_on_both_paragraphs() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (first, second) = both_paragraphs(&actual);
        assert!(
            actual
                .projected
                .carry_modifiers(first)
                .contains_key(&ModifierType::Bold)
        );
        assert!(
            actual
                .projected
                .carry_modifiers(second)
                .contains_key(&ModifierType::Bold)
        );
    }

    #[test]
    fn split_at_start_clears_both_memories() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (first, second) = both_paragraphs(&actual);
        assert!(actual.projected.carry_modifiers(first).is_empty());
        assert!(actual.projected.carry_modifiers(second).is_empty());
    }

    #[test]
    fn split_empty_paragraph_preserves_carry() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph carry([font_size(2400), font_family("RIDIBatang".to_string())]) {
                        text("1") [font_size(2400), font_family("RIDIBatang".to_string())]
                    }
                    p1: paragraph carry([font_size(2400), font_family("RIDIBatang".to_string())]) {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph carry([font_size(2400), font_family("RIDIBatang".to_string())]) {
                        text("1") [font_size(2400), font_family("RIDIBatang".to_string())]
                    }
                    paragraph carry([font_size(2400), font_family("RIDIBatang".to_string())]) {}
                    p2: paragraph carry([font_size(2400), font_family("RIDIBatang".to_string())]) {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn split_carry_excludes_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (first, second) = both_paragraphs(&actual);
        assert!(actual.projected.carry_modifiers(first).is_empty());
        assert!(actual.projected.carry_modifiers(second).is_empty());
    }

    #[test]
    fn split_after_tab_carries_atom_paint_to_both() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("a") tab [font_size(2400)] text("b") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (first, second) = both_paragraphs(&actual);
        assert!(
            actual
                .projected
                .carry_modifiers(first)
                .contains_key(&ModifierType::FontSize)
        );
        assert!(
            actual
                .projected
                .carry_modifiers(second)
                .contains_key(&ModifierType::FontSize)
        );
    }

    #[test]
    fn split_does_not_copy_stale_carry_to_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| split_paragraph(&mut tr));
        let (_first, second) = both_paragraphs(&actual);
        let carry = actual.projected.carry_modifiers(second);
        assert!(carry.contains_key(&ModifierType::Bold));
        assert!(!carry.contains_key(&ModifierType::Italic));
    }
}

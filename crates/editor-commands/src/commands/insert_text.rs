use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_text_at_caret;

pub fn insert_text(tr: &mut Transaction, text: &str) -> CommandResult {
    insert_text_at_caret(tr, text)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::CommandError;
    use crate::test_utils::*;

    #[test]
    fn empty_text_returns_error() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, ""));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn newline_returns_error() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\nb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn carriage_return_returns_error() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\rb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| insert_text(&mut tr, "X"));
    }

    #[test]
    fn insert_into_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "XY"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HeXYllo") } } }
            selection: (p1, 4, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "AB"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ABHello") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello!") } } }
            selection: (p1, 6, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_unicode_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "한글"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello한글") } } }
            selection: (p1, 7, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_with_pending_bold_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        text("X") [bold]
                    }
                }
            }
            selection: (p1, 6, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_with_different_mods_creates_node_before() {
        // Bold has Expand::After → not inherited at start → effective = []
        // Current mods = [Bold] → mismatch → new node before
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("X")
                        text("Hello") [bold]
                    }
                }
            }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_with_pending_splits_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        // "He" [] → "X" [Bold] → "llo" []
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("He")
                        text("X") [bold]
                        text("llo")
                    }
                }
            }
            selection: (p1, 3, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_link_creates_node_after() {
        // Link has Expand::None → not inherited → new node after
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, " here"));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Click") [link(href: "https://a.com".to_string())]
                        text(" here")
                    }
                }
            }
            selection: (p1, 10, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_bold_stays_inline() {
        // Bold has Expand::After → inherited at end → match → Case 1
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello!") [bold] } } }
            selection: (p1, 6, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Hello"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_cleared_after_insert() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_into_non_textblock_returns_error() {
        let (initial, ..) = state! {
            doc { root { hr: horizontal_rule {} } }
            selection: (hr, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(matches!(
            err,
            CommandError::Step(_) | CommandError::NodeNotFound(_)
        ));
    }

    #[test]
    fn pending_unset_on_bold_text_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
            pending_modifiers: [!bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [bold]
                        text("X")
                    }
                }
            }
            selection: (p1, 6, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_into_empty_paragraph_with_marker_consumes_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph marker([bold]) {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Y") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_in_middle_split_state_coalesces_with_right_half() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph marker([bold]) { text("llo") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Yllo") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph_preserves_paragraph_only_modifier() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [line_height(220)] {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [line_height(220)] { text("X") } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph_with_mixed_markers_carries_only_text_applicable() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [line_height(220)] marker([bold]) {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph [line_height(220)] { text("Y") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_clears_paragraph_marker_even_if_text_already_styled() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph marker([bold]) { text("Hi") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HiX") [bold] } } }
            selection: (p1, 3, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn materializes_synthetic_trailing_paragraph_after_unit() {
        use editor_model::NodeType;
        use editor_state::{Affinity, Position, Selection};

        // Real ops = [HR] only; the Root schema derives a synthetic trailing
        // paragraph and the caret lands inside it. Typing must materialize the
        // scaffold into a real paragraph instead of erroring OffsetOutOfBounds.
        let (initial, _root) = state! {
            doc { r: root { horizontal_rule } }
            selection: (r, 0)
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
        assert!(insert_text(&mut tr, "x").unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                horizontal_rule
                p1: paragraph { text("x") }
            } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn materializes_synthetic_fold_content_chain_after_synthetic_title() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        let (initial, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    fold
                    paragraph {}
                }
            }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let fold = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .expect("fold");
            let title = fold
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldTitle)
                .expect("synthetic fold title");
            assert!(title.id().is_synthetic());
            let content = fold
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldContent)
                .expect("synthetic fold content");
            assert!(content.id().is_synthetic());
            let paragraph = content
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .expect("synthetic fold content paragraph");
            paragraph.id()
        };
        assert!(synth_p.is_synthetic());

        let mut tr = Transaction::new(&initial);
        tr.set_selection(Some(Selection::collapsed(Position::new(synth_p, 0))))
            .unwrap();
        assert!(insert_text(&mut tr, "x").unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("x") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn materializes_synthetic_empty_block_containers() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        let cases = [
            (NodeType::Blockquote, "blockquote"),
            (NodeType::Callout, "callout"),
            (NodeType::BulletList, "bullet_list"),
            (NodeType::OrderedList, "ordered_list"),
            (NodeType::Table, "table"),
        ];
        for (container_type, label) in cases {
            let (initial, ..) = match container_type {
                NodeType::Blockquote => state! {
                    doc { root { blockquote paragraph {} } }
                    selection: none
                },
                NodeType::Callout => state! {
                    doc { root { callout paragraph {} } }
                    selection: none
                },
                NodeType::BulletList => state! {
                    doc { root { bullet_list paragraph {} } }
                    selection: none
                },
                NodeType::OrderedList => state! {
                    doc { root { ordered_list paragraph {} } }
                    selection: none
                },
                NodeType::Table => state! {
                    doc { root { table paragraph {} } }
                    selection: none
                },
                _ => unreachable!(),
            };
            let synth_p = {
                let view = initial.view();
                let container = view
                    .root()
                    .unwrap()
                    .child_blocks()
                    .find(|b| b.node_type() == container_type)
                    .unwrap_or_else(|| panic!("{label} container"));
                let paragraph = container
                    .descendants()
                    .filter_map(|child| match child {
                        editor_model::ChildView::Block(block)
                            if block.node_type() == NodeType::Paragraph =>
                        {
                            Some(block.id())
                        }
                        _ => None,
                    })
                    .next()
                    .unwrap_or_else(|| panic!("{label} synthetic paragraph"));
                paragraph
            };
            assert!(synth_p.is_synthetic(), "{label} paragraph is synthetic");

            let mut tr = Transaction::new(&initial);
            tr.set_selection(Some(Selection::collapsed(Position::new(synth_p, 0))))
                .unwrap();
            assert!(insert_text(&mut tr, "x").unwrap(), "{label}");
            let (actual, ..) = tr.commit();
            let selection = actual.selection.expect("selection");
            assert_eq!(selection.anchor, selection.head, "{label}");
            assert!(
                selection.head.node.as_op_dot().is_some(),
                "{label} caret host must be real"
            );
            let view = actual.view();
            let paragraph = view.node(selection.head.node).expect("paragraph");
            assert_eq!(paragraph.node_type(), NodeType::Paragraph, "{label}");
            assert_eq!(paragraph.inline_text(), "x", "{label}");
        }
    }

    #[test]
    fn materializes_table_cell_after_synthetic_padded_cell() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        let (initial, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                            table_cell { paragraph { text("c") } }
                        }
                        table_row {
                            table_cell { paragraph { text("d") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let table = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Table)
                .expect("table");
            let row = table
                .child_blocks()
                .nth(1)
                .expect("short row padded by normalize_grid");
            let preceding_cell = row.child_blocks().nth(1).expect("first synthetic cell");
            assert!(preceding_cell.id().is_synthetic());
            let target_cell = row.child_blocks().nth(2).expect("second synthetic cell");
            assert!(target_cell.id().is_synthetic());
            let paragraph = target_cell
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .expect("synthetic table cell paragraph");
            paragraph.id()
        };
        assert!(synth_p.is_synthetic());

        let mut tr = Transaction::new(&initial);
        tr.set_selection(Some(Selection::collapsed(Position::new(synth_p, 0))))
            .unwrap();
        assert!(insert_text(&mut tr, "x").unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                            table_cell { paragraph { text("c") } }
                        }
                        table_row {
                            table_cell { paragraph { text("d") } }
                            table_cell {}
                            table_cell { p1: paragraph { text("x") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }
}

use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::delete_selection_range;

pub fn delete_selection(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    delete_selection_range(tr, selection)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn delete_selection_returns_false_when_no_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") } } }
            selection: none
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        let result = delete_selection(&mut tr);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| delete_selection(&mut tr));
    }

    #[test]
    fn deleting_all_plain_text_records_carry_tombstone() {
        use editor_model::EditOp;

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("X") } } }
            selection: (p1, 0) -> (p1, 1)
        };
        let (_state, _steps, recorded, _fx, _meta) =
            transact!(initial, |tr| delete_selection(&mut tr));
        let carry_ops = recorded
            .iter()
            .filter(|r| matches!(r.op.payload, EditOp::NodeCarry(_)))
            .count();
        assert_eq!(
            carry_ops, 10,
            "emptying a text block records a carry tombstone for all 10 carry kinds"
        );
    }

    #[test]
    fn delete_within_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 2) -> (p1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Heorld") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_entire_text_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("ABC") } } }
            selection: (p1, 1) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("AC") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Held") }
            } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_with_middle_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("Middle") }
                p3: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Held") }
            } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_range_into_synthetic_trailing_paragraph_materializes_endpoint() {
        use editor_model::NodeType;
        use editor_state::{Affinity, Position, Selection};

        let (mut initial, _r, p1) = state! {
            doc { r: root { p1: paragraph { text("Hello") } horizontal_rule } }
            selection: none
        };
        let synth = {
            let view = initial.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph && b.id().is_synthetic())
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        initial.selection = Some(Selection::new(
            Position {
                node: p1,
                offset: 2,
                affinity: Affinity::Downstream,
            },
            Position {
                node: synth,
                offset: 0,
                affinity: Affinity::Upstream,
            },
        ));

        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("He") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_range_between_synthetic_fold_scaffolds_materializes_endpoints() {
        use editor_model::NodeType;
        use editor_state::{Affinity, Position, Selection};

        let (mut initial, ..) = state! {
            doc { root { fold } }
            selection: none
        };
        let (title, trailing) = {
            let view = initial.view();
            let root = view.root().unwrap();
            let fold = root
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .unwrap();
            let title = fold
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldTitle)
                .unwrap()
                .id();
            let trailing = root
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .unwrap()
                .id();
            (title, trailing)
        };
        assert!(title.is_synthetic() && trailing.is_synthetic());
        initial.selection = Some(Selection::new(
            Position {
                node: title,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: trailing,
                offset: 0,
                affinity: Affinity::Upstream,
            },
        ));

        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let title_after = {
            let view = actual.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldTitle)
                .unwrap()
                .id()
        };
        assert!(!title_after.is_synthetic());
        let selection = actual.selection.unwrap();
        assert_eq!(selection.anchor, selection.head);
        assert_eq!(selection.head.node, title_after);
    }

    #[test]
    fn delete_across_blockquotes_merges_containers() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("Hello") }
                }
                blockquote {
                    p3: paragraph { text("World") }
                    p4: paragraph { text("B") }
                }
            } }
            selection: (p2, 2) -> (p3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("Held") }
                    p3: paragraph { text("B") }
                }
                paragraph {}
            } }
            selection: (p2, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_sole_content_leaves_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_inline_to() {
        let (initial, ..) = state! {
            doc { r: root { image p1: paragraph { text("Hello") } } }
            selection: (r, 0) -> (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("lo") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_inline_from_block_to() {
        let (initial, ..) = state! {
            doc { r: root { p1: paragraph { text("Hello") } image } }
            selection: (p1, 2) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("He") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_block_to_same_parent() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("Before") }
                image
                horizontal_rule
                p2: paragraph { text("After") }
            } }
            selection: (r, 1) -> (r, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Before") }
                p2: paragraph { text("After") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_inline_to_with_middle_nodes() {
        let (initial, ..) = state! {
            doc { r: root {
                image
                p1: paragraph { text("Middle") }
                p2: paragraph { text("Hello") }
            } }
            selection: (r, 0) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("lo") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_nodes_cursor_selects_adjacent_block() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
                p1: paragraph {}
            } }
            selection: (r, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_nodes_cursor_selects_remaining_hr() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("A") }
                image
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { r: root {
                p1: paragraph { text("A") }
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1) -> (r, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_single_block_cursor_to_textblock() {
        let (initial, ..) = state! {
            doc { r: root { image p1: paragraph { text("Hello") } } }
            selection: (r, 0) -> (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_from_does_not_merge_adjacent_paragraphs() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("Before") }
                image
                p2: paragraph { text("Hello") }
            } }
            selection: (r, 1) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Before") }
                p2: paragraph { text("lo") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn deleting_all_list_items_removes_the_emptied_list() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("Before") }
                bl: bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
                p2: paragraph { text("After") }
            } }
            selection: (bl, 0) -> (bl, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Before") }
                p2: paragraph { text("After") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn deleting_tail_list_items_materializes_schema_valid_item() {
        let (initial, ..) = state! {
            doc { root {
                bl: bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                    list_item { paragraph { text("c") } }
                }
                paragraph { text("After") }
            } }
            selection: (bl, 1) -> (bl, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { np: paragraph {} }
                }
                paragraph { text("After") }
            } }
            selection: (np, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfill_empty_container_after_deletion() {
        let (initial, ..) = state! {
            doc { r: root {
                fold {
                    ft1: fold_title { text("Title") }
                    fc: fold_content {
                        image
                        p1: paragraph { text("Content") }
                    }
                }
                p2: paragraph { text("Hello") }
            } }
            selection: (fc, 0) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    ft1: fold_title { text("Title") }
                    fold_content {
                        fp: paragraph {}
                    }
                }
                p1: paragraph { text("lo") }
            } }
            selection: (fp, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_from_empty_paragraph_merges() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph { text("asdf") }
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("asdf") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_both_texts_fully() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("asdf") }
                p2: paragraph { text("asdf") }
            } }
            selection: (p1, 0) -> (p2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_to_empty_paragraph_merges() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("asdf") }
                p2: paragraph {}
                p3: paragraph { text("asdf") }
            } }
            selection: (p1, 4) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("asdf") }
                p2: paragraph { text("asdf") }
            } }
            selection: (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_empty_paragraph_break_before_non_paragraph_removes_empty_owner() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph {}
                callout { p2: paragraph { text("callout") } }
                p3: paragraph { text("tail") }
            } }
            selection: (p1, 0) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                callout { p1: paragraph { text("callout") } }
                p2: paragraph { text("tail") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_empty_paragraph_break_before_paragraph_keeps_next_paragraph_identity() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph { text("next") }
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p2: paragraph { text("next") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_range_containing_empty_paragraph_break_keeps_non_empty_paragraph_identity() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph { text("next") }
            } }
            selection: (p1, 0) -> (p2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p2: paragraph { text("xt") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_selected_trailing_empty_paragraph_after_textblock_creates_new_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { r1: root {
                paragraph { text("ㅁ") }
                p2: paragraph {}
            } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("ㅁ") }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_selected_trailing_empty_paragraph_after_image_uses_fulfilled_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { r1: root {
                image
                paragraph {}
            } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { r1: root {
                image
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_selected_middle_empty_paragraph_moves_to_next_text_start() {
        let (initial, ..) = state! {
            doc { r1: root {
                paragraph { text("ㅁㄴㅇㄴㅁㅇ") }
                paragraph {}
                p1: paragraph { text("ㅁㄴㅇ") }
            } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("ㅁㄴㅇㄴㅁㅇ") }
                p1: paragraph { text("ㅁㄴㅇ") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_spanning_empty_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                p3: paragraph { text("asdf") }
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_hard_break() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {
                text("qwer")
                hard_break {}
                text("zxcv")
            } } }
            selection: (p, 2) -> (p, 7)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("qwcv") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_after_tab_from_tab_boundary_selection() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2, >) -> (p1, 3, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_image_and_full_text() {
        let (initial, ..) = state! {
            doc { r: root { image p1: paragraph { text("hello") } } }
            selection: (r, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_start_and_image() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("hello") }
                image
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
                p2: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_image_to_paragraph_start() {
        let (initial, ..) = state! {
            doc { r: root { image p1: paragraph { text("hello") } } }
            selection: (r, 0) -> (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_middle_image_cursor_to_prev_end() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("hello") }
                image
                p2: paragraph { text("world") }
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("hello") }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_to_first_hr_preserves_others() {
        let (initial, ..) = state! {
            doc { r: root {
                p1: paragraph { text("text1") }
                horizontal_rule
                horizontal_rule
                horizontal_rule
                p2: paragraph { text("text2") }
            } }
            selection: (p1, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
                horizontal_rule
                horizontal_rule
                p2: paragraph { text("text2") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_all_list_items_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("A") } }
                    list_item { p2: paragraph { text("B") } }
                }
                p3: paragraph {}
            } }
            selection: (p1, 0) -> (p3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph {} }
                }
                p3: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_two_list_items() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("asdf") } }
                    list_item { p2: paragraph { text("asdf") } }
                }
                paragraph {}
            } }
            selection: (p1, 2) -> (p2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("asdf") } }
                }
                paragraph {}
            } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_merge_adjacent_lists() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { p1: paragraph { text("1") } }
                    list_item { p2: paragraph { text("2") } }
                }
                ordered_list {
                    list_item { p3: paragraph { text("3") } }
                    list_item { p4: paragraph { text("4") } }
                }
            } }
            selection: (p2, 0) -> (p3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { p1: paragraph { text("1") } }
                    list_item { p2: paragraph { text("3") } }
                    list_item { p3: paragraph { text("4") } }
                }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_fold_boundary() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("11") }
                fold {
                    ft1: fold_title { text("22") }
                    fold_content {
                        p2: paragraph { text("33") }
                    }
                }
                p3: paragraph { text("44") }
            } }
            selection: (p1, 1) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // fold_title allows empty because its content is Text* (no required child).
        // The fold boundary blocks content merge, so p1 and p2 remain in separate blocks.
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("1") }
                fold { ft1: fold_title {} fold_content { p2: paragraph { text("3") } } }
                p3: paragraph { text("44") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_title_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("11") }
                fold {
                    ft1: fold_title { text("22") }
                    fold_content {
                        p2: paragraph { text("33") }
                    }
                }
                p3: paragraph { text("44") }
            } }
            selection: (ft1, 1) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // The fold boundary blocks content merge across it; fold structure is preserved.
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("11") }
                fold {
                    ft1: fold_title { text("2") }
                    fold_content { p2: paragraph {} }
                }
                p3: paragraph { text("4") }
            } }
            selection: (ft1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_containing_whole_fold() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("11") }
                fold {
                    ft1: fold_title { text("22") }
                    fold_content {
                        p2: paragraph { text("33") }
                    }
                }
                p3: paragraph { text("44") }
            } }
            selection: (p1, 1) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("14") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_with_non_textblock_content() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    ft1: fold_title { text("11") }
                    fold_content {
                        p1: paragraph { text("22") }
                        bullet_list {
                            list_item {
                                p2: paragraph { text("33") }
                            }
                        }
                    }
                }
            } }
            selection: (ft1, 1) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // The fold boundary blocks content merge; partially-selected non-textblock ancestors
        // (bullet_list/list_item) are preserved because they are not wholly contained.
        let (expected, ..) = state! {
            doc { root {
                fold {
                    ft1: fold_title { text("1") }
                    fold_content {
                        bullet_list { list_item { p1: paragraph { text("3") } } }
                    }
                }
            } }
            selection: (ft1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_from_blockquote_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph {}
                    p3: paragraph { text("ㅁㄴㅇㅁㄴㅇ") }
                }
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph {}
                }
                p2: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_last_paragraph_fulfills_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_list_items_with_both_sublists_combines_into_single_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("AAA") }
                            bullet_list {
                                list_item { p2: paragraph { text("a1") } }
                            }
                        }
                        list_item {
                            p3: paragraph { text("BBB") }
                            bullet_list {
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1) -> (p3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("AAA") }
                            bullet_list {
                                list_item { p2: paragraph { text("aBBB") } }
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_list_items_with_different_type_sublists_combines() {
        // Not reachable from keyboard input alone; assumes a paste/import
        // producing the mixed-type sublist state. merge_node preserves the
        // target's type, so the result is a single bullet sublist containing
        // both items.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("AAA") }
                            bullet_list {
                                list_item { p2: paragraph { text("a1") } }
                            }
                        }
                        list_item {
                            p3: paragraph { text("BBB") }
                            ordered_list {
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1) -> (p3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("AAA") }
                            bullet_list {
                                list_item { p2: paragraph { text("aBBB") } }
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cross_paragraph_range_delete_drops_trailing_page_break() {
        // `from` is the paragraph-anchored cursor at offset 2 (past p1's
        // trailing page_break), so delete_from's sibling sweep would leave
        // the carry in place. Without the fix, merging p2 into p1 produces
        // `[text("a"), page_break, text("c")]`, which violates the
        // trailing-only PageBreak rule.
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    p2: paragraph { text("bc") }
                }
            }
            selection: (p1, 2) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("ac") }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_selection_clears_1x1_cell_rect() {
        let (state, _, c00, c01) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("a") } }
                c01: table_cell { paragraph { text("b") } }
            } } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c00, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        let c00n = view.node(c00).expect("c00 survives");
        assert_eq!(c00n.children().count(), 1);
        assert_eq!(c00n.child_blocks().next().unwrap().children().count(), 0);
        let c01n = view.node(c01).expect("c01 survives");
        assert_eq!(c01n.child_blocks().next().unwrap().children().count(), 1); // "b" intact
    }

    #[test]
    fn cell_clear_undo_restores_inline_bold() {
        let (state, c00, _c01) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell { paragraph { text("b") } }
            } } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c00, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert_state_eq!(&restored, &initial);
    }

    #[test]
    fn delete_selection_clears_1xn_cell_rect() {
        let (state, c00, c01) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c01, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        for cid in [&c00, &c01] {
            let n = view.node(*cid).expect("cell survives");
            assert_eq!(n.children().count(), 1);
            assert_eq!(n.child_blocks().next().unwrap().children().count(), 0);
        }
    }

    #[test]
    fn delete_selection_through_rows_clears_cells_keeps_structure() {
        let (state, _, c00, c01, c10, c11, ..) = state! {
            doc { root {
                p1: paragraph { text("hello") }
                table {
                    table_row {
                        c00: table_cell { paragraph { text("a") } }
                        c01: table_cell { paragraph { text("b") } }
                    }
                    table_row {
                        c10: table_cell { paragraph { text("c") } }
                        c11: table_cell { p2: paragraph { text("d") } }
                    }
                }
                paragraph {}
            } }
            selection: (p1, 3) -> (p2, 0)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let view = after.view();
        for cid in [&c00, &c01, &c10, &c11] {
            assert!(view.node(*cid).is_some(), "cell {cid:?} must survive");
        }
        for cid in [&c00, &c01, &c10] {
            let cell = view.node(*cid).unwrap();
            let kids: Vec<_> = cell.children().collect();
            assert_eq!(kids.len(), 1, "cell {cid:?} one child");
            let editor_model::ChildView::Block(p) = &kids[0] else {
                panic!("expected paragraph block");
            };
            assert_eq!(p.node_type(), editor_model::NodeType::Paragraph);
            assert_eq!(p.children().count(), 0, "empty paragraph");
        }
    }

    #[test]
    fn delete_selection_whole_table_contained_removes_table_and_merges() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph { text("AB") }
                table { table_row { table_cell { paragraph { text("x") } } } }
                p2: paragraph { text("CD") }
            } }
            selection: (p1, 1) -> (p2, 1)
        };
        let (got, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("AD") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&got, &expected);
    }

    #[test]
    fn delete_selection_to_structural_container_offset_fulfills_cell() {
        let (state, _, c00, c01) = state! {
            doc { root {
                p1: paragraph { text("X") }
                table { table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                } }
                paragraph {}
            } }
            selection: (p1, 0) -> (c00, 1)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let view = after.view();
        let cell = view.node(c00).expect("structural cell must survive");
        assert_eq!(cell.children().count(), 1, "emptied cell must be fulfilled");
        let editor_model::ChildView::Block(p) = cell.first_child().unwrap() else {
            panic!("expected paragraph block");
        };
        assert_eq!(p.node_type(), editor_model::NodeType::Paragraph);
        assert_eq!(p.children().count(), 0);
        assert!(view.node(c01).is_some());
    }

    #[test]
    fn delete_selection_emptying_paragraph_records_carry() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold, font_weight(700)] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let dot = p1;
        let carry = actual.projected.carry_modifiers(dot);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn delete_selection_partial_text_no_carry_change() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 1) -> (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let dot = p1;
        assert!(actual.projected.carry_modifiers(dot).is_empty());
    }

    #[test]
    fn emptying_paragraph_records_first_charlike_tab_paint() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { tab [font_size(1600)] text("b") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let carry = actual.projected.carry_modifiers(p1);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontSize { value: 1600 })),
            "carry must come from the first charlike (the tab), got {carry:?}"
        );
    }

    #[test]
    fn emptying_unpainted_paragraph_clears_stale_carry() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph carry([bold]) { text("hi") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        assert!(
            actual.projected.carry_modifiers(p1).is_empty(),
            "emptying deletion must replace stale carry with the (empty) first charlike paint"
        );
    }

    #[test]
    fn cell_rect_replaces_each_cell_carry_with_own_first_charlike() {
        let (state, c00, c01) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") [bold] } }
                    c01: table_cell { paragraph { text("b") [italic] } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c01, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        let p00 = view.node(c00).unwrap().child_blocks().next().unwrap().id();
        let p01 = view.node(c01).unwrap().child_blocks().next().unwrap().id();
        assert!(
            after
                .projected
                .carry_modifiers(p00)
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
        assert!(
            after
                .projected
                .carry_modifiers(p01)
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic))
        );
    }

    #[test]
    fn cell_rect_multiblock_cell_carry_from_first_charlike_past_empty_first_paragraph() {
        let (state, c00, c01) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") [bold] } }
                    c01: table_cell {
                        paragraph {}
                        paragraph { text("b") [font_size(2000)] }
                    }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c01, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        let p00 = view.node(c00).unwrap().child_blocks().next().unwrap().id();
        let p01 = view.node(c01).unwrap().child_blocks().next().unwrap().id();
        assert!(
            after
                .projected
                .carry_modifiers(p00)
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "the single-block cell keeps its own first charlike paint, got {:?}",
            after.projected.carry_modifiers(p00)
        );
        assert!(
            after
                .projected
                .carry_modifiers(p01)
                .values()
                .any(|m| matches!(m, editor_model::Modifier::FontSize { value: 2000 })),
            "an empty first paragraph must not mask the cell's first charlike font size, got {:?}",
            after.projected.carry_modifiers(p01)
        );
    }

    #[test]
    fn select_all_scaffold_records_first_charlike_paint() {
        let (initial, _r) = state! {
            doc { r: root {
                paragraph { text("A") [bold] }
                paragraph { text("B") [italic] }
            } }
            selection: (r, 0) -> (r, 2)
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        let scaffold = view.root().unwrap().child_blocks().next().unwrap().id();
        let carry = after.projected.carry_modifiers(scaffold);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "scaffold must inherit the first block's first charlike paint, got {carry:?}"
        );
        assert!(
            !carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic)),
            "scaffold must not inherit a later block's paint, got {carry:?}"
        );
    }

    #[test]
    fn cross_boundary_emptied_survivor_records_own_carry() {
        let (initial, _p1, ft1, p2) = state! {
            doc { root {
                p1: paragraph { text("11") }
                fold {
                    ft1: fold_title carry([italic]) { text("22") }
                    fold_content {
                        p2: paragraph { text("33") [bold] }
                    }
                }
            } }
            selection: (p1, 1) -> (p2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let p2_carry = actual.projected.carry_modifiers(p2);
        assert!(
            p2_carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "the merge-blocked, emptied to-side block keeps its own first charlike paint, got {p2_carry:?}"
        );
        assert!(
            actual.projected.carry_modifiers(ft1).is_empty(),
            "the emptied intermediate fold_title must replace its stale carry with its (empty) own first charlike paint, got {:?}",
            actual.projected.carry_modifiers(ft1)
        );
    }

    #[test]
    fn emptying_delete_undo_restores_content_and_carry() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph carry([bold]) { text("hi") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.projected.carry_modifiers(p1).is_empty(),
            "emptying replaced the stale carry within the same transaction"
        );
        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert_state_eq!(&restored, &initial);
    }

    /// A `SeqItem::Unknown` leaf cannot be captured into a `Subtree` (no lossless
    /// `Plain` form), so its selection-delete must route to `Step::DeleteOpaque`
    /// (position-based `ListOp::Del`) rather than `Step::RemoveSubtree`. Undo must
    /// restore it via `Undel` — the SAME dot, not a freshly re-inserted carrier.
    #[test]
    fn unknown_leaf_delete_undo_restores_via_undel_same_dot() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (base, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let mut seeded = base.clone();
        let unknown = seeded
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            }))
            .unwrap()
            .id;
        let initial = editor_state::State {
            selection: Some(editor_state::Selection::new(
                editor_state::Position::new(p1, 1),
                editor_state::Position::new(p1, 2),
            )),
            ..seeded
        };

        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.view().leaf(unknown).is_none(),
            "unknown leaf must be deleted"
        );

        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert!(
            restored.view().leaf(unknown).is_some(),
            "undo must restore the unknown leaf under the SAME dot (Undel), not a re-inserted carrier"
        );
        assert_state_eq!(&restored, &initial);
    }

    /// A placeholder `SeqItem::Block { node_type: NodeType::Unknown, .. }` sitting
    /// among known blocks must also delete/undo through the opaque, dot-based
    /// path (not `RemoveSubtree`/`InsertSubtree`, which would need a lossless
    /// `Subtree` capture the placeholder cannot provide).
    #[test]
    fn unknown_block_range_delete_undo_round_trips_same_dot() {
        use editor_crdt::{Dot, ListOp};
        use editor_model::{EditOp, NodeType, SeqItem};

        let (base, p1, ..) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let mut seeded = base.clone();
        let unknown_block = seeded
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Block {
                    node_type: NodeType::Unknown,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        let initial = editor_state::State {
            selection: Some(editor_state::Selection::new(
                editor_state::Position::new(Dot::ROOT, 0),
                editor_state::Position::new(Dot::ROOT, 2),
            )),
            ..seeded
        };

        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.view().node(unknown_block).is_none(),
            "unknown placeholder block must be deleted"
        );
        assert!(after.view().node(p1).is_none(), "p1 must be deleted too");

        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert!(
            restored.view().node(unknown_block).is_some(),
            "undo must restore the placeholder block under the SAME dot (Undel)"
        );
        assert_state_eq!(&restored, &initial);
    }

    #[test]
    fn cross_boundary_merge_keeps_front_carry() {
        let (initial, p1, _p2) = state! {
            doc { root {
                p1: paragraph carry([bold]) { text("AB") }
                p2: paragraph carry([italic]) { text("CD") }
            } }
            selection: (p1, 2) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let carry = actual.projected.carry_modifiers(p1);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "the merged survivor keeps the front paragraph's carry, got {carry:?}"
        );
        assert!(
            !carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic)),
            "the merged-away paragraph's carry does not leak into the survivor, got {carry:?}"
        );
    }

    #[test]
    fn range_delete_to_paragraph_end_makes_no_merge() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph carry([bold]) { text("AB") }
                p2: paragraph carry([italic]) { text("CD") }
            } }
            selection: (p1, 0) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = actual.view();
        assert_eq!(
            view.root().unwrap().child_blocks().count(),
            2,
            "a range delete that stops at the paragraph end must not merge the two blocks"
        );
        let carry = actual.projected.carry_modifiers(p2);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic)),
            "the untouched following paragraph keeps its own carry, got {carry:?}"
        );
    }

    #[test]
    fn whole_block_delete_undo_restores_carry() {
        let (initial, _r, p1, _p2) = state! {
            doc { r: root {
                p1: paragraph carry([bold]) { text("A") }
                p2: paragraph { text("B") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.view().node(p1).is_none(),
            "the whole paragraph block is removed"
        );
        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert_state_eq!(&restored, &initial);
        let restored_first = restored
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();
        assert!(
            restored
                .projected
                .carry_modifiers(restored_first)
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "undo restores the removed block's carry through the captured subtree"
        );
    }

    #[test]
    fn cross_paragraph_delete_preserves_tail_bold() {
        use editor_model::Modifier;
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                p2: paragraph { text("World") [bold] }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = actual.view();
        let paras: Vec<_> = view.root().unwrap().child_blocks().collect();
        assert_eq!(paras.len(), 1, "the two paragraphs merge into one");
        let para = &paras[0];
        assert_eq!(para.inline_text(), "Held");
        assert!(para.leaf_own_modifiers_at(0).is_empty(), "'H' stays plain");
        assert!(para.leaf_own_modifiers_at(1).is_empty(), "'e' stays plain");
        assert!(
            para.leaf_own_modifiers_at(2).contains(&Modifier::Bold),
            "'l' from the bold tail keeps its bold"
        );
        assert!(
            para.leaf_own_modifiers_at(3).contains(&Modifier::Bold),
            "'d' from the bold tail keeps its bold"
        );
    }

    #[test]
    fn whole_block_delete_undo_restores_inline_bold() {
        let (initial, _r, p1, _p2) = state! {
            doc { r: root {
                p1: paragraph { text("A") [bold] }
                p2: paragraph { text("B") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.view().node(p1).is_none(),
            "the bold paragraph block is removed"
        );
        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert_state_eq!(&restored, &initial);
    }

    #[test]
    fn emptying_delete_undo_redo_roundtrips_carry() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph carry([bold]) { text("hi") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        assert!(delete_selection(&mut tr).unwrap());
        let (after, records, ..) = tr.commit();
        assert!(
            after.projected.carry_modifiers(p1).is_empty(),
            "emptying replaces the stale carry with the (empty) first charlike paint"
        );
        let undone = records.iter().rev().fold(after.clone(), |s, r| {
            r.step.inverse().apply(&s).unwrap().state
        });
        assert_state_eq!(&undone, &initial);
        let redone = records
            .iter()
            .fold(undone, |s, r| r.step.apply(&s).unwrap().state);
        assert_state_eq!(&redone, &after);
    }

    #[test]
    fn delete_selection_from_outside_into_cell_must_not_delete_cell() {
        let (state, _, c00, _, c01, c02, c03) = state! {
            doc { root {
                p1: paragraph {}
                table {
                    table_row {
                        c00: table_cell { p2: paragraph {} }
                        c01: table_cell { paragraph {} }
                        c02: table_cell { paragraph {} }
                        c03: table_cell { paragraph {} }
                    }
                    table_row {
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                    }
                    table_row {
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                        table_cell { paragraph {} }
                    }
                }
                paragraph {}
            } }
            selection: (p1, 0, >) -> (p2, 0, <)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let view = after.view();
        assert!(
            view.node(c00).is_some(),
            "first table_cell must not be deleted by backspace"
        );
        assert!(view.node(c01).is_some());
        assert!(view.node(c02).is_some());
        assert!(view.node(c03).is_some());
    }

    #[test]
    fn delete_selection_cell_to_cell_no_cross_merge() {
        let (state, ca, _, cb, ..) = state! {
            doc { root { table { table_row {
                ca: table_cell { p1: paragraph { text("hello") } }
                cb: table_cell { p2: paragraph { text("world") } }
            } } } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let view = after.view();
        assert!(view.node(ca).is_some());
        assert!(view.node(cb).is_some());
        assert!(view.node(ca).unwrap().children().count() >= 1);
        assert!(view.node(cb).unwrap().children().count() >= 1);
    }

    #[test]
    fn delete_selection_clears_mxn_cell_rect() {
        let (state, c00, c01, c10, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c11, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let view = after.view();
        for cid in [&c00, &c01, &c10, &c11] {
            let n = view.node(*cid).expect("cell survives");
            assert_eq!(n.children().count(), 1);
            assert_eq!(n.child_blocks().next().unwrap().children().count(), 0);
        }
    }

    #[test]
    fn delete_selection_full_table_cell_rect_removes_table() {
        let (state, tbl, c00, c11, _) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        c00: table_cell { paragraph { text("a") } }
                        table_cell { paragraph { text("b") } }
                    }
                    table_row {
                        table_cell { paragraph { text("c") } }
                        c11: table_cell { paragraph { text("d") } }
                    }
                }
                p1: paragraph { text("after") }
            } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(c00, c11, &state.view()).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };

        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));

        assert!(
            after.view().node(tbl).is_none(),
            "full-table selection should delete the table"
        );
    }

    fn assert_matches_cold_projection(state: &editor_state::State) {
        let cold = editor_state::ProjectedState::from_graph(state.projected.graph().clone())
            .expect("cold rebuild projects");
        assert_eq!(state.projected.projected(), cold.projected());
    }

    // Removing most of the root's children takes the bulk path (`delete_child_slots`
    // defers each step's projection and flushes them with one coverage-preserving
    // reprojection). The result must equal the expected doc AND a cold rebuild of
    // the op graph — the same equivalence the eager per-step path guarantees.
    #[test]
    fn bulk_select_all_delete_matches_cold_projection() {
        let (initial, _r) = state! {
            doc { r: root {
                paragraph { text("p00") }
                paragraph { text("p01") }
                paragraph { text("p02") }
                paragraph { text("p03") }
                paragraph { text("p04") }
                paragraph { text("p05") }
                paragraph { text("p06") }
                paragraph { text("p07") }
                paragraph { text("p08") }
                paragraph { text("p09") }
                paragraph { text("p10") }
                paragraph { text("p11") }
            } }
            selection: (r, 0) -> (r, 12)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_matches_cold_projection(&actual);
    }

    #[test]
    fn bulk_cross_node_delete_matches_cold_projection() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello") }
                paragraph { text("m00") }
                paragraph { text("m01") }
                paragraph { text("m02") }
                paragraph { text("m03") }
                paragraph { text("m04") }
                paragraph { text("m05") }
                paragraph { text("m06") }
                paragraph { text("m07") }
                paragraph { text("m08") }
                paragraph { text("m09") }
                p2: paragraph { text("World") }
            } }
            selection: (p1, 2) -> (p2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Held") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
        assert_matches_cold_projection(&actual);
    }

    // Root-level atoms (image/HR) and a nested list ride the same bulk run as the
    // paragraphs around them.
    #[test]
    fn bulk_select_all_delete_mixed_content_matches_cold_projection() {
        let (initial, _r) = state! {
            doc { r: root {
                image
                paragraph { text("aa") }
                paragraph { text("bb") }
                bullet_list {
                    list_item { paragraph { text("cc") } }
                    list_item { paragraph { text("dd") } }
                }
                horizontal_rule
                paragraph { text("ee") }
                paragraph { text("ff") }
            } }
            selection: (r, 0) -> (r, 7)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_matches_cold_projection(&actual);
    }

    #[test]
    fn tr_323_synthetic_trailing_paragraph_repro_deletes_selection() {
        use editor_model::NodeType;
        use editor_state::{Affinity, Position, Selection};

        let (state, p1) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    p1: paragraph {}
                    image
                    paragraph {}
                    embed
                }
            }
            selection: none
        };
        let synth_p = {
            let view = state.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .filter(|b| b.node_type() == NodeType::Paragraph)
                .find(|b| b.id().is_synthetic())
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        let initial = editor_state::State {
            selection: Some(Selection::new(
                Position {
                    node: synth_p,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node: p1,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
            )),
            ..state
        };

        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_matches_cold_projection(&actual);
    }

    // Sweep selection endpoints over a 12-paragraph doc: every shape — same-node
    // container ranges (bulk), cross-node ranges with bulk middles, and small
    // eager-path edges — must land on the projection a cold graph rebuild produces.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 96, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn delete_selection_multi_paragraph_matches_cold(
            a_para in 0usize..12,
            a_off in 0usize..=5,
            b_para in 0usize..12,
            b_off in 0usize..=5,
        ) {
            use editor_state::{Position, Selection};

            let (state, p00, p01, p02, p03, p04, p05, p06, p07, p08, p09, p10, p11) = state! {
                doc { root {
                    p00: paragraph { text("abcde") }
                    p01: paragraph { text("abcde") }
                    p02: paragraph { text("abcde") }
                    p03: paragraph { text("abcde") }
                    p04: paragraph { text("abcde") }
                    p05: paragraph { text("abcde") }
                    p06: paragraph { text("abcde") }
                    p07: paragraph { text("abcde") }
                    p08: paragraph { text("abcde") }
                    p09: paragraph { text("abcde") }
                    p10: paragraph { text("abcde") }
                    p11: paragraph { text("abcde") }
                } }
                selection: none
            };
            let paras = [p00, p01, p02, p03, p04, p05, p06, p07, p08, p09, p10, p11];
            let anchor = Position::new(paras[a_para], a_off);
            let head = Position::new(paras[b_para], b_off);
            proptest::prop_assume!(anchor != head);
            let initial = editor_state::State {
                selection: Some(Selection::new(anchor, head)),
                ..state
            };
            let mut tr = editor_transaction::Transaction::new(&initial);
            let changed = delete_selection(&mut tr).unwrap();
            proptest::prop_assume!(changed);
            let (actual, ..) = tr.commit();
            let cold = editor_state::ProjectedState::from_graph(actual.projected.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(actual.projected.projected(), cold.projected());
        }
    }
}

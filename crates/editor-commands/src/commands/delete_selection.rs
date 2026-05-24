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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| delete_selection(&mut tr));
    }

    #[test]
    fn delete_within_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Heorld") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_entire_text_node() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("A")
                t2: text("B")
                t3: text("C")
            } } }
            selection: (t2, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("A") t3: text("C") } } }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_two_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Held") }
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_with_middle_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("Middle") }
                paragraph { t3: text("World") }
            } }
            selection: (t1, 2) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Held") }
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_blockquotes_merges_containers() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("Hello") }
                }
                blockquote {
                    paragraph { t3: text("World") }
                    paragraph { t4: text("B") }
                }
            } }
            selection: (t2, 2) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("A") }
                    paragraph { t2: text("Held") }
                    paragraph { t4: text("B") }
                }
                paragraph {}
            } }
            selection: (t2, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_sole_content_leaves_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
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
            doc { r: root { image paragraph { t1: text("Hello") } } }
            selection: (r, 0) -> (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_inline_from_block_to() {
        let (initial, ..) = state! {
            doc { r: root { paragraph { t1: text("Hello") } image } }
            selection: (t1, 2) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("He") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_block_to_same_parent() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                image
                horizontal_rule
                paragraph { t2: text("After") }
            } }
            selection: (r, 1) -> (r, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("After") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_block_from_inline_to_with_middle_nodes() {
        let (initial, ..) = state! {
            doc { r: root {
                image
                paragraph { t1: text("Middle") }
                paragraph { t2: text("Hello") }
            } }
            selection: (r, 0) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t2: text("lo") } } }
            selection: (t2, 0)
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
                paragraph { t1: text("A") }
                image
                horizontal_rule
                paragraph {}
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { t1: text("A") }
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
            doc { r: root { image paragraph { t1: text("Hello") } } }
            selection: (r, 0) -> (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_from_does_not_merge_adjacent_paragraphs() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                image
                paragraph { t2: text("Hello") }
            } }
            selection: (r, 1) -> (t2, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("lo") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfill_empty_container_after_deletion() {
        let (initial, ..) = state! {
            doc { r: root {
                fold {
                    fold_title { t1: text("Title") }
                    fc: fold_content {
                        image
                        paragraph { t2: text("Content") }
                    }
                }
                paragraph { t3: text("Hello") }
            } }
            selection: (fc, 0) -> (t3, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content {
                        fp: paragraph {}
                    }
                }
                paragraph { t3: text("lo") }
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
                paragraph { t2: text("asdf") }
            } }
            selection: (p1, 0) -> (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t2: text("asdf") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_both_texts_fully() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("asdf") }
                paragraph { t2: text("asdf") }
            } }
            selection: (t1, 0) -> (t2, 4)
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
                paragraph { t1: text("asdf") }
                p2: paragraph {}
                paragraph { t3: text("asdf") }
            } }
            selection: (t1, 4) -> (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("asdf") }
                paragraph { t3: text("asdf") }
            } }
            selection: (t1, 4)
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
                paragraph { t3: text("ㅁㄴㅇ") }
            } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("ㅁㄴㅇㄴㅁㅇ") }
                paragraph { t3: text("ㅁㄴㅇ") }
            } }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_spanning_empty_paragraphs() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                paragraph { t1: text("asdf") }
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
            doc { root { paragraph {
                t1: text("qwer")
                hard_break {}
                t2: text("zxcv")
            } } }
            selection: (t1, 2) -> (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("qw")
                t2: text("cv")
            } } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_image_and_full_text() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { t1: text("hello") } } }
            selection: (r, 0) -> (t1, 5)
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
                paragraph { t1: text("hello") }
                image
                p2: paragraph {}
            } }
            selection: (t1, 0) -> (r, 2)
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
            doc { r: root { image paragraph { t1: text("hello") } } }
            selection: (r, 0) -> (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_middle_image_cursor_to_prev_end() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("hello") }
                image
                paragraph { t2: text("world") }
            } }
            selection: (r, 1) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_text_to_first_hr_preserves_others() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { t1: text("text1") }
                horizontal_rule
                horizontal_rule
                horizontal_rule
                paragraph { t2: text("text2") }
            } }
            selection: (t1, 0) -> (r, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph {}
                horizontal_rule
                horizontal_rule
                paragraph { t2: text("text2") }
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
                    list_item { paragraph { t1: text("A") } }
                    list_item { paragraph { t2: text("B") } }
                }
                p3: paragraph {}
            } }
            selection: (t1, 0) -> (p3, 0)
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
                    list_item { paragraph { t1: text("asdf") } }
                    list_item { paragraph { t2: text("asdf") } }
                }
                paragraph {}
            } }
            selection: (t1, 2) -> (t2, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { t1: text("asdf") } }
                }
                paragraph {}
            } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_merge_adjacent_lists() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { t1: text("1") } }
                    list_item { paragraph { t2: text("2") } }
                }
                ordered_list {
                    list_item { paragraph { t3: text("3") } }
                    list_item { paragraph { t4: text("4") } }
                }
            } }
            selection: (t2, 0) -> (t3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { paragraph { t1: text("1") } }
                    list_item { paragraph { t3: text("3") } }
                    list_item { paragraph { t4: text("4") } }
                }
                paragraph {}
            } }
            selection: (t3, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_across_fold_boundary() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1) -> (t3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // fold_title allows empty because its content is Text* (no required child).
        // The fold boundary blocks content merge, so t1 and t3 remain in separate blocks.
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("1") }
                fold { fold_title {} fold_content { paragraph { t3: text("3") } } }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_title_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t2, 1) -> (t4, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // The fold boundary blocks content merge across it; fold structure is preserved.
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("2") }
                    fold_content { paragraph {} }
                }
                paragraph { t4: text("4") }
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_containing_whole_fold() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { t1: text("11") }
                fold {
                    fold_title { t2: text("22") }
                    fold_content {
                        paragraph { t3: text("33") }
                    }
                }
                paragraph { t4: text("44") }
            } }
            selection: (t1, 1) -> (t4, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("14") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_fold_with_non_textblock_content() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("11") }
                    fold_content {
                        paragraph { t2: text("22") }
                        bullet_list {
                            list_item {
                                paragraph { t3: text("33") }
                            }
                        }
                    }
                }
            } }
            selection: (t1, 1) -> (t3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        // The fold boundary blocks content merge; partially-selected non-textblock ancestors
        // (bullet_list/list_item) are preserved because they are not wholly contained.
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("1") }
                    fold_content {
                        bullet_list { list_item { paragraph { t3: text("3") } } }
                    }
                }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_from_blockquote_to_outside() {
        let (initial, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph {}
                    paragraph { t1: text("ㅁㄴㅇㅁㄴㅇ") }
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
                            paragraph { t_a: text("AAA") }
                            bullet_list {
                                list_item { paragraph { t_sub_a: text("a1") } }
                            }
                        }
                        list_item {
                            paragraph { t_b: text("BBB") }
                            bullet_list {
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_sub_a, 1) -> (t_b, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { t_a: text("AAA") }
                            bullet_list {
                                list_item { paragraph { t_sub_a: text("aBBB") } }
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_sub_a, 1)
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
                            paragraph { t_a: text("AAA") }
                            bullet_list {
                                list_item { paragraph { t_sub_a: text("a1") } }
                            }
                        }
                        list_item {
                            paragraph { t_b: text("BBB") }
                            ordered_list {
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_sub_a, 1) -> (t_b, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { t_a: text("AAA") }
                            bullet_list {
                                list_item { paragraph { t_sub_a: text("aBBB") } }
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_sub_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cross_paragraph_range_delete_drops_trailing_page_break() {
        // `from` is the paragraph-anchored cursor at offset 2 (past p1's
        // trailing page_break), so delete_from's sibling sweep would leave
        // the marker in place. Without the fix, merging p2 into p1 produces
        // `[text("a"), page_break, text("c")]`, which violates the
        // trailing-only PageBreak rule.
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { t2: text("bc") }
                }
            }
            selection: (p1, 2) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("ac") }
                }
            }
            selection: (t1, 2)
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
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c00).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let c00n = after.doc.node(c00).expect("c00 survives");
        assert_eq!(c00n.children().count(), 1);
        assert_eq!(c00n.first_child().unwrap().children().count(), 0);
        let c01n = after.doc.node(c01).expect("c01 survives");
        assert_eq!(c01n.first_child().unwrap().children().count(), 1); // "b" intact
    }

    #[test]
    fn delete_selection_clears_1xn_cell_rect() {
        let (state, _, c00, c01) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("a") } }
                c01: table_cell { paragraph { text("b") } }
            } } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c01).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        for cid in [c00, c01] {
            let n = after.doc.node(cid).expect("cell survives");
            assert_eq!(n.children().count(), 1);
            assert_eq!(n.first_child().unwrap().children().count(), 0);
        }
    }

    #[test]
    fn delete_selection_through_rows_clears_cells_keeps_structure() {
        let (state, _, c00, c01, c10, c11, ..) = state! {
            doc { root {
                paragraph { pt: text("hello") }
                table {
                    table_row {
                        c00: table_cell { paragraph { text("a") } }
                        c01: table_cell { paragraph { text("b") } }
                    }
                    table_row {
                        c10: table_cell { paragraph { text("c") } }
                        c11: table_cell { paragraph { ct: text("d") } }
                    }
                }
                paragraph {}
            } }
            selection: (pt, 3) -> (ct, 0)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        for cid in [c00, c01, c10, c11] {
            assert!(after.doc.node(cid).is_some(), "cell {cid:?} must survive");
        }
        for cid in [c00, c01, c10] {
            let cell = after.doc.node(cid).unwrap();
            let kids: Vec<_> = cell.children().collect();
            assert_eq!(kids.len(), 1, "cell {cid:?} one child");
            assert!(matches!(kids[0].node(), editor_model::Node::Paragraph(_)));
            assert_eq!(kids[0].children().count(), 0, "empty paragraph");
        }
    }

    #[test]
    fn delete_selection_whole_table_contained_removes_table_and_merges() {
        let (state, ..) = state! {
            doc { root {
                paragraph { bt: text("AB") }
                table { table_row { table_cell { paragraph { text("x") } } } }
                paragraph { at: text("CD") }
            } }
            selection: (bt, 1) -> (at, 1)
        };
        let (got, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t: text("AD") } } }
            selection: (t, 1)
        };
        assert_state_eq!(&got, &expected);
    }

    #[test]
    fn delete_selection_to_structural_container_offset_fulfills_cell() {
        let (state, _, c00, c01) = state! {
            doc { root {
                paragraph { pt: text("X") }
                table { table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                } }
                paragraph {}
            } }
            selection: (pt, 0) -> (c00, 1)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        let cell = after.doc.node(c00).expect("structural cell must survive");
        assert_eq!(cell.children().count(), 1, "emptied cell must be fulfilled");
        assert!(matches!(
            cell.first_child().unwrap().node(),
            editor_model::Node::Paragraph(_)
        ));
        assert_eq!(cell.first_child().unwrap().children().count(), 0);
        assert!(after.doc.node(c01).is_some());
    }

    #[test]
    fn delete_selection_emptying_paragraph_lifts_marker() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") [bold, font_weight(700)] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let p = actual.doc.node(p1).unwrap();
        let mods: Vec<_> = p.modifiers().cloned().collect();
        assert!(
            mods.iter()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
        assert!(
            mods.iter()
                .any(|m| matches!(m, editor_model::Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn delete_selection_partial_text_no_lift() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 1) -> (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        let p = actual.doc.node(p1).unwrap();
        assert_eq!(p.modifiers().count(), 0);
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
        assert!(
            after.doc.node(c00).is_some(),
            "first table_cell must not be deleted by backspace"
        );
        assert!(after.doc.node(c01).is_some());
        assert!(after.doc.node(c02).is_some());
        assert!(after.doc.node(c03).is_some());
    }

    #[test]
    fn delete_selection_cell_to_cell_no_cross_merge() {
        let (state, ca, _, cb, ..) = state! {
            doc { root { table { table_row {
                ca: table_cell { paragraph { ta: text("hello") } }
                cb: table_cell { paragraph { tb: text("world") } }
            } } } }
            selection: (ta, 2) -> (tb, 3)
        };
        let (after, ..) = transact!(state, |tr| delete_selection(&mut tr));
        assert!(after.doc.node(ca).is_some());
        assert!(after.doc.node(cb).is_some());
        assert!(after.doc.node(ca).unwrap().children().count() >= 1);
        assert!(after.doc.node(cb).unwrap().children().count() >= 1);
    }

    #[test]
    fn delete_selection_clears_mxn_cell_rect() {
        let (state, _, c00, c01, _, c10, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = editor_state::cell_rect_selection(&state.doc, c00, c11).unwrap();
        let initial = editor_state::State {
            selection: Some(sel),
            ..state
        };
        let (after, ..) = transact!(initial, |tr| delete_selection(&mut tr));
        for cid in [c00, c01, c10, c11] {
            let n = after.doc.node(cid).expect("cell survives");
            assert_eq!(n.children().count(), 1);
            assert_eq!(n.first_child().unwrap().children().count(), 0);
        }
    }
}

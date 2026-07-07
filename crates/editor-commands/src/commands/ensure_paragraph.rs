use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::ensure_paragraph as ensure_paragraph_core;

pub fn ensure_paragraph(tr: &mut Transaction) -> CommandResult {
    ensure_paragraph_core(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn replaces_single_leaf_with_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unit_replaced_paragraph_has_empty_carry() {
        let (initial, _r) = state! {
            doc { r: root {
                paragraph carry([bold]) { text("a") }
                horizontal_rule
                paragraph carry([italic]) { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let new_para = actual.selection.unwrap().head.node;
        assert!(
            actual.projected.carry_modifiers(new_para).is_empty(),
            "a paragraph that replaces a deleted unit starts with no carry, got {:?}",
            actual.projected.carry_modifiers(new_para)
        );
    }

    #[test]
    fn collapsed_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_within_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn cross_textblock_range_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("b") }
            } }
            selection: (p1, 0) -> (p2, 1)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn range_with_text_endpoint_returns_false() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                p1: paragraph { text("hello") }
            } }
            selection: (r, 0, >) -> (p1, 3)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }

    #[test]
    fn replaces_multiple_leaves_with_single_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_mixed_blocks_and_textblock_with_single_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph { text("middle") }
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 3, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_single_textblock_selection() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("hello") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_all_children_then_fulfills() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 3, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_blockquote() {
        let (initial, ..) = state! {
            doc { root {
                bq: blockquote {
                    paragraph { text("x") }
                    paragraph { text("y") }
                }
            } }
            selection: (bq, 0, >) -> (bq, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph
                    paragraph { text("y") }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_fold_content() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fc: fold_content {
                        paragraph { text("x") }
                        horizontal_rule
                        paragraph { text("y") }
                    }
                }
            } }
            selection: (fc, 1, >) -> (fc, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("title") }
                    fold_content {
                        paragraph { text("x") }
                        p1: paragraph
                        paragraph { text("y") }
                    }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replaces_selection_inside_list_item() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    li: list_item {
                        paragraph { text("hello") }
                    }
                }
            } }
            selection: (li, 0, >) -> (li, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| ensure_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        p1: paragraph
                    }
                }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn parent_disallows_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                bl: bullet_list {
                    list_item { paragraph { text("a") } }
                    list_item { paragraph { text("b") } }
                }
            } }
            selection: (bl, 0, >) -> (bl, 2, <)
        };
        transact_fail!(initial, |tr| ensure_paragraph(&mut tr));
    }
}

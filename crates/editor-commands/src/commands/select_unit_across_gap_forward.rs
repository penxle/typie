use editor_crdt::Dot;
use editor_state::{Affinity, Position, Selection};
use editor_state::{GapCursor, as_gap_cursor};
use editor_transaction::Transaction;

use crate::CommandResult;

pub fn select_unit_across_gap_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let (parent_id, anchor_off, head_off): (Dot, usize, usize) = {
        let view = tr.state().view();
        let Some(rs) = selection.resolve(&view) else {
            return Ok(false);
        };
        match as_gap_cursor(&rs) {
            None => return Ok(false),
            Some(GapCursor::BetweenMonolithic { parent, index, .. }) => {
                (parent.id(), index, index + 1)
            }
            // A boundary gap has a single adjacent unit; select it in
            // either direction.
            Some(GapCursor::IsolatingBoundary { host, index, .. }) => {
                if index == 0 {
                    (host.id(), 0, 1)
                } else {
                    (host.id(), index - 1, index)
                }
            }
        }
    };

    tr.set_selection(Some(Selection::new(
        Position {
            node: parent_id,
            offset: anchor_off,
            affinity: Affinity::Downstream,
        },
        Position {
            node: parent_id,
            offset: head_off,
            affinity: Affinity::Upstream,
        },
    )))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn between_two_folds_at_root_selects_next_fold() {
        let (initial, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                    fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                    paragraph {}
                }
            }
            selection: (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                    fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                    paragraph {}
                }
            }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn between_two_folds_in_fold_content_selects_next_under_inner_parent() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("t") }
                        fc: fold_content {
                            fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                            fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (fc, 1)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("t") }
                        fc: fold_content {
                            fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                            fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (fc, 1, >) -> (fc, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn between_two_folds_in_table_cell_selects_next_under_cell() {
        let (initial, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            cell: table_cell {
                                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (cell, 1)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            cell: table_cell {
                                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (cell, 1, >) -> (cell, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn leading_image_gap_selects_image() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn leading_fold_gap_selects_fold() {
        let (initial, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                    paragraph {}
                }
            }
            selection: (r, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                    paragraph {}
                }
            }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn leading_boundary_gap_in_fold_content_selects_callout() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content { callout { paragraph { text("x") } } }
                }
                paragraph {}
            } }
            selection: (fc, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content { callout { paragraph { text("x") } } }
                }
                paragraph {}
            } }
            selection: (fc, 0, >) -> (fc, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn trailing_boundary_gap_in_fold_content_selects_callout() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content { callout { paragraph { text("x") } } }
                }
                paragraph {}
            } }
            selection: (fc, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content { callout { paragraph { text("x") } } }
                }
                paragraph {}
            } }
            selection: (fc, 0, >) -> (fc, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_text_selection_is_noop() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| select_unit_across_gap_forward(&mut tr));
    }

    #[test]
    fn non_collapsed_text_range_is_noop() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        transact_fail!(initial, |tr| select_unit_across_gap_forward(&mut tr));
    }

    #[test]
    fn paragraph_start_when_no_leading_unit_is_noop() {
        let (initial, ..) = state! {
            doc { root { p: paragraph { text("a") } } }
            selection: (p, 0)
        };
        transact_fail!(initial, |tr| select_unit_across_gap_forward(&mut tr));
    }

    #[test]
    fn between_two_folds_at_root_does_not_select_prev_fold() {
        // Direction-guard: forward must produce (r, 1, >) -> (r, 2, <),
        // never (r, 0, >) -> (r, 1, <) (which is the backward result).
        // If the implementation accidentally mirrored the backward offset
        // tuple `(index - 1, index)`, this assertion would fail.
        let (initial, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                    fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                    paragraph {}
                }
            }
            selection: (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| select_unit_across_gap_forward(&mut tr));
        let (backward_shape, ..) = state! {
            doc {
                r: root {
                    fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                    fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                    paragraph {}
                }
            }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_ne!(
            actual.selection, backward_shape.selection,
            "forward must select children[k], not children[k-1]"
        );
    }
}

use editor_model::NodeId;
use editor_state::{Affinity, GapCursor, Position, Selection};
use editor_transaction::Transaction;

use crate::CommandResult;

pub fn select_unit_across_gap_forward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    let doc = tr.doc();

    let (parent_id, anchor_off, head_off): (NodeId, usize, usize) = match selection
        .resolve(&doc)
        .and_then(|rs| rs.as_gap_cursor())
    {
        None => return Ok(false),
        Some(GapCursor::LeadingUnit { .. }) => (NodeId::ROOT, 0, 1),
        Some(GapCursor::BetweenMonolithic { parent, index, .. }) => (parent.id(), index, index + 1),
    };

    tr.set_selection(Selection::new(
        Position {
            node_id: parent_id,
            offset: anchor_off,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: parent_id,
            offset: head_off,
            affinity: Affinity::Upstream,
        },
    ))?;
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
    fn collapsed_text_selection_is_noop() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        transact_fail!(initial, |tr| select_unit_across_gap_forward(&mut tr));
    }

    #[test]
    fn non_collapsed_text_range_is_noop() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 1) -> (t, 4)
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

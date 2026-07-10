use editor_model::{NodeType, Subtree};
use editor_state::{Position, Selection, StableSelection};
use editor_transaction::Transaction;

use crate::helpers::{
    apply_fulfill, block_child_id_at, materialize_selected_block_run, promote_list_run,
    remap_slot_selection, resolve_selected_block_run_for_fold, restore_selection,
    validate_selected_block_wrap,
};
use crate::{CommandError, CommandResult};

pub fn wrap_selected_blocks_in_fold(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let Some(run) = resolve_selected_block_run_for_fold(tr)? else {
        return Ok(false);
    };
    let run = promote_list_run(tr, run)?;
    let Some(insert_index) =
        validate_selected_block_wrap(&tr.view(), &run, NodeType::Fold, NodeType::FoldContent)?
    else {
        return Ok(false);
    };
    let move_to_title = is_single_empty_paragraph(&tr.view(), &selection, &run);
    let stable_selection = StableSelection::capture(&selection, &tr.view());
    let slot_selection = if run.blocks.iter().all(|block| block.is_whole()) {
        remap_slot_selection(selection, run.parent_id, insert_index, run.blocks.len())
    } else {
        None
    };

    let mut inserted = None;
    tr.batch::<_, CommandError>(|tr| {
        let run = materialize_selected_block_run(tr, &run)?;
        let fold = Subtree::leaf(NodeType::Fold.into_node().to_plain()).with_children(vec![
            Subtree::leaf(NodeType::FoldTitle.into_node().to_plain()),
            Subtree::leaf(NodeType::FoldContent.into_node().to_plain()),
        ]);
        tr.insert_subtree(run.parent_id, insert_index, fold)?;
        let fold_id = block_child_id_at(tr, run.parent_id, insert_index)?;
        let title_id = block_child_id_at(tr, fold_id, 0)?;
        let content_id = block_child_id_at(tr, fold_id, 1)?;
        inserted = Some((fold_id, title_id, content_id));

        for (index, block) in run.blocks.iter().enumerate() {
            tr.move_node(block.id, content_id, index)?;
        }
        apply_fulfill(tr, &[content_id, fold_id, run.parent_id])?;
        Ok(())
    })?;

    let (_, title_id, content_id) =
        inserted.ok_or_else(|| CommandError::Corrupted("fold was not inserted".into()))?;
    if move_to_title {
        tr.set_selection(Some(Selection::collapsed(Position::new(title_id, 0))))?;
    } else if let Some(selection) = slot_selection {
        tr.set_selection(Some(selection.with_node(content_id)))?;
    } else {
        restore_selection(
            tr,
            stable_selection,
            "cannot restore folded block selection",
        )?;
    }
    Ok(true)
}

fn is_single_empty_paragraph(
    view: &editor_model::DocView,
    selection: &Selection,
    run: &crate::helpers::SelectedBlockRun,
) -> bool {
    selection.is_collapsed()
        && run.blocks.len() == 1
        && run.blocks[0].node_type == NodeType::Paragraph
        && view
            .node(run.blocks[0].id)
            .is_some_and(|paragraph| paragraph.children().next().is_none())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn wraps_collapsed_nonempty_paragraph_and_preserves_offset() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } paragraph {} } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content { p1: paragraph { text("Hello") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_selected_sibling_blocks() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("A") }
                            p2: paragraph { text("B") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn creates_nested_fold_inside_fold_content() {
        let (initial, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("Outer") }
                        fold_content { p1: paragraph { text("Body") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("Outer") }
                        fold_content {
                            fold {
                                fold_title {}
                                fold_content { p1: paragraph { text("Body") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_only_the_selected_list_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            bullet_list {
                                list_item { p1: paragraph { text("A") } }
                            }
                        }
                    }
                    bullet_list {
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_partial_list_boundaries_and_middle_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    middle: paragraph { text("M") }
                    bullet_list {
                        list_item { paragraph { text("D") } }
                        list_item { p5: paragraph { text("E") } }
                        list_item { paragraph { text("F") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p5, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                    }
                    fold {
                        fold_title {}
                        fold_content {
                            bullet_list {
                                list_item { p2: paragraph { text("B") } }
                                list_item { paragraph { text("C") } }
                            }
                            middle: paragraph { text("M") }
                            bullet_list {
                                list_item { paragraph { text("D") } }
                                list_item { p5: paragraph { text("E") } }
                            }
                        }
                    }
                    bullet_list {
                        list_item { paragraph { text("F") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p5, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_only_the_containing_outer_item_for_nested_list_selection() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item {
                            paragraph { text("B") }
                            bullet_list {
                                list_item { nested: paragraph { text("Nested") } }
                            }
                        }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (nested, 3)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                    }
                    fold {
                        fold_title {}
                        fold_content {
                            bullet_list {
                                list_item {
                                    paragraph { text("B") }
                                    bullet_list {
                                        list_item { nested: paragraph { text("Nested") } }
                                    }
                                }
                            }
                        }
                    }
                    bullet_list {
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (nested, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_inside_table_cell_without_promoting_table() {
        let (initial, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { p1: paragraph { text("Cell") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell {
                                fold {
                                    fold_title {}
                                    fold_content { p1: paragraph { text("Cell") } }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_whole_blockquote_for_collapsed_empty_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            blockquote {
                                paragraph { text("A") }
                                p1: paragraph {}
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_boundary_blockquote_when_every_direct_content_block_intersects() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("AB") }
                        paragraph { text("CD") }
                    }
                    p3: paragraph { text("E") }
                }
            }
            selection: (p1, 1) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            blockquote {
                                p1: paragraph { text("AB") }
                                paragraph { text("CD") }
                            }
                            p3: paragraph { text("E") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_whole_callout_for_collapsed_cursor() {
        let (initial, ..) = state! {
            doc {
                root {
                    callout {
                        paragraph { text("AB") }
                        p2: paragraph { text("CD") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            callout {
                                paragraph { text("AB") }
                                p2: paragraph { text("CD") }
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
    fn wraps_boundary_blockquote_when_some_direct_content_is_unselected() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p2: paragraph { text("BC") }
                    }
                    p3: paragraph { text("D") }
                }
            }
            selection: (p2, 1) -> (p3, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            blockquote {
                                paragraph { text("A") }
                                p2: paragraph { text("BC") }
                            }
                            p3: paragraph { text("D") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 1) -> (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_multiple_atomic_wrappers_and_outside_block_in_document_order() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("A") }
                        p2: paragraph { text("B") }
                    }
                    middle: paragraph { text("C") }
                    callout {
                        p4: paragraph { text("D") }
                        paragraph { text("E") }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p4, 1)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content {
                            blockquote {
                                paragraph { text("A") }
                                p2: paragraph { text("B") }
                            }
                            middle: paragraph { text("C") }
                            callout {
                                p4: paragraph { text("D") }
                                paragraph { text("E") }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0) -> (p4, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_explicitly_selected_whole_blockquote() {
        let (initial, _root, ..) = state! {
            doc {
                root: root {
                    blockquote { paragraph { text("A") } }
                    paragraph {}
                }
            }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, _root, ..) = state! {
            doc {
                root: root {
                    fold {
                        fold_title {}
                        content: fold_content { blockquote { paragraph { text("A") } } }
                    }
                    paragraph {}
                }
            }
            selection: (content, 0, >) -> (content, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn empty_paragraph_moves_cursor_to_fold_title() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} paragraph { text("After") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        title: fold_title {}
                        fold_content { paragraph {} }
                    }
                    paragraph { text("After") }
                }
            }
            selection: (title, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn whitespace_only_paragraph_keeps_cursor_in_content() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text(" ") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title {}
                        fold_content { p1: paragraph { text(" ") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn wraps_block_level_atom_selection() {
        let (initial, _root, ..) = state! {
            doc { root: root { image paragraph {} } }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| wrap_selected_blocks_in_fold(&mut tr));
        let (expected, _root, ..) = state! {
            doc {
                root: root {
                    fold {
                        fold_title {}
                        content: fold_content { image }
                    }
                    paragraph {}
                }
            }
            selection: (content, 0, >) -> (content, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }
}

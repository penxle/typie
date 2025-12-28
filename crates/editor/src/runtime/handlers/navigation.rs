use super::super::{Direction, Effect, Runtime};
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::model::{MarkType, NodeId};
use crate::state::position_helpers::{compare_positions, move_from_block_position};
use crate::state::{Position, Selection, block_content_len};
use std::cmp::Ordering;

impl Runtime {
    pub(crate) fn handle_navigate(
        &mut self,
        direction: Direction,
        extend_selection: bool,
    ) -> Vec<Effect> {
        let is_upward = matches!(direction, Direction::Up | Direction::PageUp);

        if is_upward && !extend_selection && self.is_at_document_start() {
            return vec![Effect::ExitedDocumentStart];
        }

        let invalidate_preferred_x = !matches!(
            direction,
            Direction::Up | Direction::Down | Direction::PageUp | Direction::PageDown
        );

        let ctx = NavigationContext::new(&self.state.doc);
        let cached_preferred_x = if matches!(
            direction,
            Direction::Up | Direction::Down | Direction::PageUp | Direction::PageDown
        ) {
            if let Some(x) = self.state.preferred_x {
                Some(x)
            } else {
                Cursor::bounds(&ctx, &self.pages, self.state.selection.head).map(|(_, rect)| rect.x)
            }
        } else {
            self.state.preferred_x
        };

        let (anchor, head) = self.compute_navigation(
            &ctx,
            &self.state.selection,
            direction,
            extend_selection,
            cached_preferred_x,
            self.viewport_height,
        );

        let new_preferred_x = if invalidate_preferred_x {
            None
        } else {
            cached_preferred_x
        };

        self.transact(move |tr| {
            tr.set_selection(Selection::new(anchor, head));
            tr.set_preferred_x(new_preferred_x);
            Ok(true)
        })
    }

    pub(crate) fn is_at_document_start(&self) -> bool {
        let selection = &self.state.selection;
        if !selection.is_collapsed() {
            return false;
        }

        let ctx = NavigationContext::new(&self.state.doc);
        let Some(doc_start_selection) = Cursor::move_to_document_start(&ctx, &self.pages) else {
            return false;
        };

        selection.head == doc_start_selection.head
    }

    pub(crate) fn handle_select_all(&mut self) -> Vec<Effect> {
        let head = &self.state.selection.head;

        if let Some(head_node) = self.state.doc.node(head.node_id) {
            let isolating_ancestor = head_node.ancestors().find(|a| a.spec().isolating);

            if let Some(isolating) = isolating_ancestor {
                let start =
                    Position::new(isolating.node_id(), 0, crate::types::Affinity::Downstream);
                let end = Position::new(
                    isolating.node_id(),
                    block_content_len(&isolating),
                    crate::types::Affinity::Downstream,
                );

                let (from, to) = self.state.selection.as_sorted(&self.state.doc).unwrap();

                if start != from || end != to {
                    return self.transact(move |tr| {
                        tr.set_selection(Selection::new(start, end));
                        Ok(true)
                    });
                }
            }
        }

        let root = self.state.doc.node(NodeId::ROOT).unwrap();
        let start = Position::new(root.node_id(), 0, crate::types::Affinity::Downstream);
        let end = Position::new(
            root.node_id(),
            block_content_len(&root),
            crate::types::Affinity::Downstream,
        );

        self.transact(move |tr| {
            tr.set_selection(Selection::new(start, end));
            Ok(true)
        })
    }

    pub(crate) fn handle_extend_mark_range(&mut self, mark_type: MarkType) -> Vec<Effect> {
        self.transact(|tr| tr.extend_mark_range(mark_type))
    }

    fn compute_navigation(
        &self,
        ctx: &NavigationContext,
        selection: &Selection,
        direction: Direction,
        extend_selection: bool,
        cached_preferred_x: Option<f32>,
        viewport_height: f32,
    ) -> (Position, Position) {
        let pages = &self.pages;

        let move_from = |position: Position| -> Selection {
            let span = match direction {
                Direction::DocumentStart => Cursor::move_to_document_start(ctx, pages),
                Direction::DocumentEnd => Cursor::move_to_document_end(ctx, pages),
                _ => {
                    let (_, rect) = match Cursor::bounds(ctx, pages, position.clone()) {
                        Some(r) => r,
                        None => {
                            let go_forward = matches!(
                                direction,
                                Direction::Right
                                    | Direction::Down
                                    | Direction::PageDown
                                    | Direction::LineEnd
                                    | Direction::WordRight
                                    | Direction::DocumentEnd
                            );
                            let resolved = move_from_block_position(ctx.doc, position, go_forward);
                            return Selection::collapsed(resolved);
                        }
                    };
                    let (preferred_x, preferred_y) = match direction {
                        Direction::Left | Direction::WordLeft => (rect.x, rect.y),
                        Direction::Right | Direction::WordRight => {
                            (rect.x + rect.width, rect.y + rect.height)
                        }
                        Direction::Up
                        | Direction::Down
                        | Direction::PageUp
                        | Direction::PageDown => {
                            let x = cached_preferred_x.unwrap_or(rect.x);
                            let y = if matches!(direction, Direction::Up | Direction::PageUp) {
                                rect.y
                            } else {
                                rect.y + rect.height
                            };
                            (x, y)
                        }
                        Direction::LineStart | Direction::LineEnd => (0.0, 0.0),
                        Direction::DocumentStart | Direction::DocumentEnd => (0.0, 0.0),
                    };

                    match direction {
                        Direction::Left => Cursor::move_left(ctx, pages, position, preferred_y),
                        Direction::Right => Cursor::move_right(ctx, pages, position, preferred_y),
                        Direction::Up => Cursor::move_up(ctx, pages, position, preferred_x),
                        Direction::Down => Cursor::move_down(ctx, pages, position, preferred_x),
                        Direction::PageUp => {
                            Cursor::move_page_up(ctx, pages, position, preferred_x, viewport_height)
                        }
                        Direction::PageDown => Cursor::move_page_down(
                            ctx,
                            pages,
                            position,
                            preferred_x,
                            viewport_height,
                        ),
                        Direction::LineStart => Cursor::move_to_line_start(ctx, pages, position),
                        Direction::LineEnd => Cursor::move_to_line_end(ctx, pages, position),
                        Direction::WordLeft => {
                            Cursor::move_word_left(ctx, pages, position, preferred_y)
                        }
                        Direction::WordRight => {
                            Cursor::move_word_right(ctx, pages, position, preferred_y)
                        }
                        Direction::DocumentStart | Direction::DocumentEnd => None,
                    }
                }
            };

            span.unwrap_or_else(|| Selection::collapsed(position))
        };

        if extend_selection {
            let new_span = move_from(selection.head);
            let (sorted_from, sorted_to) = selection.as_sorted(&self.state.doc).unwrap();

            let (anchor, new_head) = match direction {
                Direction::Up
                | Direction::PageUp
                | Direction::Left
                | Direction::WordLeft
                | Direction::LineStart
                | Direction::DocumentStart => {
                    match compare_positions(&self.state.doc, sorted_from, new_span.head) {
                        Ok(Ordering::Greater) => (sorted_to, new_span.anchor),
                        _ => (selection.anchor, new_span.head),
                    }
                }
                Direction::Down
                | Direction::PageDown
                | Direction::Right
                | Direction::WordRight
                | Direction::LineEnd
                | Direction::DocumentEnd => {
                    match compare_positions(&self.state.doc, sorted_to, new_span.anchor) {
                        Ok(Ordering::Less) => (sorted_from, new_span.head),
                        _ => (selection.anchor, new_span.anchor),
                    }
                }
            };
            (anchor, new_head)
        } else {
            let (from, to) = selection.as_sorted(&self.state.doc).unwrap();
            let base = match direction {
                Direction::Left
                | Direction::Up
                | Direction::PageUp
                | Direction::LineStart
                | Direction::WordLeft
                | Direction::DocumentStart => from,
                Direction::Right
                | Direction::Down
                | Direction::PageDown
                | Direction::LineEnd
                | Direction::WordRight
                | Direction::DocumentEnd => to,
            };
            let span = move_from(base);
            (span.anchor, span.head)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::NodeId;
    use crate::runtime::{Direction, Message};
    use crate::types::Affinity;

    #[test]
    fn test_horizontal_rule_shift_up_extends_selection() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        rt.layout();
        rt.update(Message::Navigate {
            direction: Direction::Up,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 2);
        assert_eq!(selection.anchor.affinity, Affinity::Upstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 0);
        assert_eq!(selection.head.affinity, Affinity::Downstream);
    }

    #[test]
    fn test_horizontal_rule_shift_down_extends_selection() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
            }
            selection { (NodeId::ROOT, 0, Affinity::Downstream) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        rt.layout();
        rt.update(Message::Navigate {
            direction: Direction::Down,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.anchor.affinity, Affinity::Downstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 2);
        assert_eq!(selection.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn test_horizontal_rule_shift_down_then_shift_up_restores_selection() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
            }
            selection { (NodeId::ROOT, 0, Affinity::Downstream) -> (NodeId::ROOT, 1, Affinity::Upstream) }
        };

        rt.layout();

        rt.update(Message::Navigate {
            direction: Direction::Down,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.offset, 2);

        rt.update(Message::Navigate {
            direction: Direction::Up,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.anchor.affinity, Affinity::Downstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 1);
        assert_eq!(selection.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn test_three_hrs_middle_selected_shift_up_extends_to_include_previous() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
                horizontal_rule {}
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        rt.layout();
        rt.update(Message::Navigate {
            direction: Direction::Up,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 2);
        assert_eq!(selection.anchor.affinity, Affinity::Upstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 0);
        assert_eq!(selection.head.affinity, Affinity::Downstream);
    }

    #[test]
    fn test_three_hrs_middle_selected_shift_down_extends_to_include_next() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
                horizontal_rule {}
            }
            selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        rt.layout();
        rt.update(Message::Navigate {
            direction: Direction::Down,
            extend: true,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 1);
        assert_eq!(selection.anchor.affinity, Affinity::Downstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 3);
        assert_eq!(selection.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn test_navigate_down_after_delete_forward_on_empty_paragraph_before_hrs() {
        let mut n1 = id!();
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @n1 paragraph {}
                horizontal_rule {}
                horizontal_rule {}
                horizontal_rule {}
                paragraph {}
            }
            selection { (n1, 0) }
        };

        rt.layout();
        rt.update(Message::DeleteForward);
        rt.update(Message::Navigate {
            direction: Direction::Down,
            extend: false,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 1);
        assert_eq!(selection.anchor.affinity, Affinity::Downstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 2);
        assert_eq!(selection.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn test_delete_backward_on_third_hr_selects_second_hr() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                horizontal_rule {}
                horizontal_rule {}
                horizontal_rule {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 2, Affinity::Downstream) -> (NodeId::ROOT, 3, Affinity::Upstream) }
        };

        rt.layout();
        rt.update(Message::DeleteBackward);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 1);
        assert_eq!(selection.anchor.affinity, Affinity::Downstream);

        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 2);
        assert_eq!(selection.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn select_all_in_isolating_node_selects_within_isolating_boundary() {
        let mut p1 = id!();
        let mut fc = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "before" }
                }
                fold {
                    fold_title {
                        text { "title" }
                    }
                    @fc fold_content {
                        @p1 paragraph {
                            text { "inside fold" }
                        }
                    }
                }
                paragraph {
                    text { "after" }
                }
            }
            selection { (p1, 3) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, fc);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.node_id, fc);
        assert_eq!(selection.head.offset, 1);
    }

    #[test]
    fn select_all_in_fold_with_multiple_paragraphs() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut fc = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "before" }
                }
                fold {
                    fold_title {
                        text { "title" }
                    }
                    @fc fold_content {
                        @p1 paragraph {
                            text { "first para" }
                        }
                        @p2 paragraph {
                            text { "second para" }
                        }
                    }
                }
                paragraph {
                    text { "after" }
                }
            }
            selection { (p1, 3) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, fc);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.node_id, fc);
        assert_eq!(selection.head.offset, 2);
    }

    #[test]
    fn select_all_in_fold_title_selects_within_fold_title() {
        let mut ft = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "before" }
                }
                fold {
                    @ft fold_title {
                        text { "title text" }
                    }
                    fold_content {
                        paragraph {
                            text { "inside fold" }
                        }
                    }
                }
                paragraph {
                    text { "after" }
                }
            }
            selection { (ft, 3) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, ft);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.node_id, ft);
        assert_eq!(selection.head.offset, 10);
    }

    #[test]
    fn shift_arrow_up_then_shift_arrow_down_restores_selection() {
        let mut n1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                paragraph {
                    text { "Subnecto advenio atrox ducimus ventus cometes. Aegrus capitulus iusto quasi. Rem arbustum valeo arcus advoco." }
                }
                @n1 paragraph {
                    text { "Vulnus conspergo attollo torrens aureus amor vulnus dolorum tot. Tutis curatio pel vitium territo. Conduco deleniti accendo avaritia sufficio uxor." }
                }
                paragraph {
                    text { "Tunc patruus decretum aliqua comparo bellum. Sublime succedo cui tutamen textilis. Conservo averto pecto coepi." }
                }
                paragraph {}
            }
            selection { (n1, 32) }
        };

        rt.layout();

        let initial_selection = rt.state().selection.clone();

        rt.update(Message::Navigate {
            direction: Direction::Up,
            extend: true,
        });

        rt.update(Message::Navigate {
            direction: Direction::Down,
            extend: true,
        });

        let final_selection = &rt.state().selection;

        assert_eq!(
            final_selection.anchor.node_id,
            initial_selection.anchor.node_id
        );
        assert_eq!(
            final_selection.anchor.offset,
            initial_selection.anchor.offset
        );
        assert_eq!(
            final_selection.anchor.affinity,
            initial_selection.anchor.affinity
        );
        assert_eq!(final_selection.head.node_id, initial_selection.head.node_id);
        assert_eq!(final_selection.head.offset, initial_selection.head.offset);
        assert_eq!(
            final_selection.head.affinity,
            initial_selection.head.affinity
        );
    }
    #[test]
    fn test_select_all_root_block_selection() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "First" } }
                @p2 paragraph { text { "Second" } }
            }
            selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        let selection = &rt.state().selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 0);
        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.head.offset, 2);
    }

    #[test]
    fn test_arrow_left_from_block_position_at_root_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "First" } }
                @p2 paragraph { text { "Second" } }
            }
            selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        rt.update(Message::Navigate {
            direction: Direction::Left,
            extend: false,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.head.node_id, p1);
        assert_eq!(selection.head.offset, 0);
    }

    #[test]
    fn test_arrow_right_from_block_position_at_root_start() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "First" } }
                @p2 paragraph { text { "Second" } }
            }
            selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        rt.update(Message::Navigate {
            direction: Direction::Right,
            extend: false,
        });

        let selection = &rt.state().selection;

        assert_eq!(selection.head.node_id, p2);
        assert_eq!(selection.head.offset, 6);
    }

    #[test]
    fn test_arrow_right_from_block_position_in_fold_end() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                fold {
                    fold_title {}
                    fold_content {
                        @p1 paragraph { text { "First" } }
                        paragraph { text { "Second" } }
                    }
                }
                @p2 paragraph { text { "Third" } }
            }
            selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);

        rt.update(Message::Navigate {
            direction: Direction::Right,
            extend: false,
        });

        let selection = &rt.state().selection;
        assert_eq!(selection.head.node_id, p2);
        assert_eq!(selection.head.offset, 0);
    }
}

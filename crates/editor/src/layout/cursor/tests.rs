use super::*;
use crate::layout::Element;
use crate::layout::elements::line::LineElement;
use crate::model::NodeId;
use crate::runtime::{Message, Modifier, PointerButton};
use crate::state::Selection;
use crate::types::Affinity;
use crate::utils::{byte_to_char_offset, char_to_byte_offset};

const PAGE_MARGIN: f32 = 20.0;

fn collect_lines_for_block<'a>(pages: &'a [Page], block_id: NodeId) -> Vec<&'a LineElement> {
    let mut lines = Vec::new();
    for page in pages {
        for entry in page.spatial_index().iter() {
            if let Element::Line(line) = entry.element() {
                if line.block_id == block_id {
                    lines.push(line);
                }
            }
        }
    }
    lines.sort_by_key(|l| l.line_idx);
    lines
}

fn line_text_slice(line: &LineElement) -> &str {
    let start = char_to_byte_offset(&line.text, line.metric.start_offset);
    let end = char_to_byte_offset(&line.text, line.metric.end_offset);
    &line.text[start..end]
}

fn ctx(state: &crate::runtime::State) -> NavigationContext<'_> {
    NavigationContext::new(&state.doc)
}

#[test]
fn test_cursor_move_right() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_right(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 1);
    assert_eq!(new_selection.head.affinity, Affinity::Upstream);
}

#[test]
fn test_cursor_move_left() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
            }
        }
        selection { (p, 1) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 0);
    assert_eq!(new_selection.head.affinity, Affinity::Downstream);
}

#[test]
fn test_cursor_move_down() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @p2 paragraph {
                text { "Line 2" }
            }
        }
        selection { (p1, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p2);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
fn test_cursor_move_down_from_selection_anchor_in_empty_paragraph() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @p2 paragraph { }
        }
        selection { (p2, 0) -> (p1, 0) }
    };

    let pages = rt.pages();
    let anchor = rt.selection().anchor;
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, anchor).unwrap();
    let new_selection = Cursor::move_down(&ctx(&rt.state()), &pages, anchor, rect.x).unwrap();

    assert_eq!(new_selection.head.node_id, p2);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
fn test_move_to_line_end_stays_on_hard_break_boundary() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break { }
            }
        }
        selection { (p, 1, Affinity::Upstream) }
    };

    let pages = rt.pages();
    let selection =
        Cursor::move_to_line_end(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    assert_eq!(
        selection,
        Selection::collapsed(Position::new(p, 1, Affinity::Upstream))
    );
}

#[test]
fn test_move_to_line_end_from_start_stops_at_hard_break() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break { }
            }
        }
        selection { (p, 0, Affinity::Downstream) }
    };

    let pages = rt.pages();
    let selection =
        Cursor::move_to_line_end(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    assert_eq!(
        selection,
        Selection::collapsed(Position::new(p, 1, Affinity::Upstream))
    );
}

#[test]
fn test_move_to_line_end_with_consecutive_hard_breaks_keeps_downstream() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break { }
                hard_break { }
            }
        }
        selection { (p, 2, Affinity::Downstream) }
    };

    let pages = rt.pages();
    let selection =
        Cursor::move_to_line_end(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    assert_eq!(
        selection,
        Selection::collapsed(Position::new(p, 2, Affinity::Downstream))
    );
}

#[test]
fn test_cursor_move_up() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @p2 paragraph {
                text { "Line 2" }
            }
        }
        selection { (p2, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p1);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
fn test_cursor_move_left_to_prev_para() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @p2 paragraph {
                text { "Line 2" }
            }
        }
        selection { (p2, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p1);
    assert_eq!(new_selection.head.offset, 6);
    assert_eq!(new_selection.head.affinity, Affinity::Upstream);
}

#[test]
fn test_cursor_move_right_to_next_para() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @p2 paragraph {
                text { "Line 2" }
            }
        }
        selection { (p1, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();

    assert_eq!(new_selection.head.node_id, p2);
    assert_eq!(new_selection.head.offset, 0);
    assert_eq!(new_selection.head.affinity, Affinity::Downstream);
}

fn get_first_line_end_offset(pages: &[Page], node_id: NodeId) -> usize {
    for page in pages {
        for entry in page.spatial_index().iter() {
            if let Element::Line(line) = entry.element() {
                if line.block_id == node_id && line.line_idx == 0 {
                    return line.metric.end_offset;
                }
            }
        }
    }
    panic!("Node not found or no lines");
}

#[test]
fn test_cursor_move_right_at_soft_wrap() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "asdfasdfasdfasdfasdfasdfasdfasdfasdfadsfasdfasdfasdfasdfasdfasdfasdfasdfasdfasdfadsfasdfasdf" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let wrap_offset = get_first_line_end_offset(&pages, p);
    let start_pos = Position::new(p, wrap_offset - 1, Affinity::Downstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();
    let new_selection =
        Cursor::move_right(&ctx(&rt.state()), &pages, start_pos, rect.y + rect.height).unwrap();
    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, wrap_offset);
    assert_eq!(new_selection.head.affinity, Affinity::Upstream);

    let (_, rect2) = Cursor::bounds(&ctx(&rt.state()), &pages, new_selection.head).unwrap();
    let new_selection_2 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        new_selection.head,
        rect2.y + rect2.height,
    )
    .unwrap();

    assert_eq!(new_selection_2.head.node_id, p);
    assert_eq!(new_selection_2.head.offset, wrap_offset + 1);
    assert_eq!(new_selection_2.head.affinity, Affinity::Upstream);
}

#[test]
fn test_cursor_move_left_at_soft_wrap() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "asdfasdfasdfasdfasdfasdfasdfasdfasdfadsfasdfasdfasdfasdfasdfasdfasdfasdfasdfasdfadsfasdfasdf" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let wrap_offset = get_first_line_end_offset(&pages, p);

    let start_pos = Position::new(p, wrap_offset + 1, Affinity::Downstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();
    let new_selection = Cursor::move_left(&ctx(&rt.state()), &pages, start_pos, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, wrap_offset);
    assert_eq!(new_selection.head.affinity, Affinity::Downstream);

    let (_, rect2) = Cursor::bounds(&ctx(&rt.state()), &pages, new_selection.head).unwrap();
    let new_selection_2 =
        Cursor::move_left(&ctx(&rt.state()), &pages, new_selection.head, rect2.y).unwrap();

    assert_eq!(new_selection_2.head.node_id, p);
    assert_eq!(new_selection_2.head.offset, wrap_offset - 1);
    assert_eq!(new_selection_2.head.affinity, Affinity::Downstream);
}

#[test]
fn test_cursor_bounds_in_empty_paragraph() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph { }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let cursor_result = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head);

    assert!(
        cursor_result.is_some(),
        "Cursor bounds should exist in empty paragraph"
    );

    let (page_idx, rect) = cursor_result.unwrap();
    assert_eq!(page_idx, 0, "Cursor should be on first page");
    assert!(rect.height > 0.0, "Cursor should have visible height");
}

#[test]
fn test_hard_break_navigation_right() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "World" }
            }
        }
        selection { (p, 5) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 6);
    assert_eq!(new_selection.head.affinity, Affinity::Downstream);
}

#[test]
fn test_hard_break_navigation_word_right() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "World" }
            }
        }
        selection { (p, 5) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection = Cursor::move_word_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 6);
    assert_eq!(new_selection.head.affinity, Affinity::Downstream);
}

#[test]
fn test_hard_break_navigation_left() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "World" }
            }
        }
        selection { (p, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 5);
}

#[test]
fn test_hard_break_navigation_down() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Line 1" }
                hard_break {}
                text { "Line 2" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 7);
}

#[test]
fn test_hard_break_navigation_up() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Line 1" }
                hard_break {}
                text { "Line 2" }
            }
        }
        selection { (p, 7) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
fn test_hard_break_cursor_visuals() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "World" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    let pos_5 = crate::state::Position::new(p, 5, Affinity::Upstream);
    let cursor_5 = Cursor::bounds(&ctx(&rt.state()), &pages, pos_5);
    assert!(
        cursor_5.is_some(),
        "Cursor should be visible at position 5 (end of Hello)"
    );
    let (_, rect_5) = cursor_5.unwrap();

    let pos_6 = crate::state::Position::new(p, 6, Affinity::Downstream);
    let cursor_6 = Cursor::bounds(&ctx(&rt.state()), &pages, pos_6);
    assert!(
        cursor_6.is_some(),
        "Cursor should be visible at position 6 (start of World)"
    );
    let (_, rect_6) = cursor_6.unwrap();

    assert!(
        (rect_5.height - rect_6.height).abs() < 1.0,
        "Cursor heights should be similar: {} vs {}",
        rect_5.height,
        rect_6.height
    );

    assert!(
        rect_6.y > rect_5.y,
        "Cursor after hard break should be on next line: y5={}, y6={}",
        rect_5.y,
        rect_6.y
    );

    assert!(
        rect_6.x < rect_5.x,
        "Cursor after hard break should be at line start: x5={}, x6={}",
        rect_5.x,
        rect_6.x
    );
}

#[test]
fn test_hard_break_click_at_line_end() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "World" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    let pos_at_5 = crate::state::Position::new(p, 5, Affinity::Upstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_at_5).unwrap();

    let selection = Cursor::hit_test(
        &ctx(&rt.state()),
        &pages[0],
        rect.x + 100.0,
        rect.y + rect.height / 2.0,
    )
    .unwrap();

    assert_eq!(selection.head.node_id, p);
    assert!(selection.head.offset >= 5 && selection.head.offset <= 6);
}

#[test]
fn test_consecutive_hard_breaks_navigation() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "A" }
                hard_break {}
                hard_break {}
                text { "B" }
            }
        }
        selection { (p, 1) }
    };

    let pages = rt.pages();

    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let sel1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(sel1.head.offset, 2);
    assert_eq!(sel1.head.affinity, Affinity::Downstream);

    let (_, rect2) = Cursor::bounds(&ctx(&rt.state()), &pages, sel1.head).unwrap();
    let sel2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, sel1.head, rect2.y + rect2.height).unwrap();
    assert_eq!(sel2.head.offset, 3);
}

#[test]
fn test_down_does_not_flip_affinity_at_last_line() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break {}
                hard_break {}
            }
        }
        selection { (p, 3, Affinity::Downstream) }
    };

    let pages = rt.pages();
    let start = rt.selection().head;
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start).expect("cursor bounds");

    let moved = Cursor::move_down(&ctx(&rt.state()), &pages, start, rect.x)
        .expect("cursor should stay when no lower line");

    assert_eq!(moved.head.node_id, start.node_id);
    assert_eq!(moved.head.offset, start.offset);
    assert_eq!(moved.head.affinity, start.affinity);
}

#[test]
fn test_line_end_after_hard_break_moves_to_next_line() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break {}
                text { "b" }
            }
        }
        selection { (p, 2, Affinity::Upstream) }
    };

    let pages = rt.pages();
    let start = rt.selection().head;
    let (_, start_rect) =
        Cursor::bounds(&ctx(&rt.state()), &pages, start).expect("cursor bounds exist");

    let moved = Cursor::move_to_line_end(&ctx(&rt.state()), &pages, start)
        .expect("should move to line end");
    let (_, moved_rect) =
        Cursor::bounds(&ctx(&rt.state()), &pages, moved.head).expect("cursor bounds after move");

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, 3);
    assert_eq!(moved.head.affinity, Affinity::Upstream);
    assert!(
        moved_rect.y >= start_rect.y,
        "Cursor should not jump to a higher line"
    );
}

#[test]
fn test_line_end_at_paragraph_end_after_consecutive_hard_breaks_stays_on_same_line() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a" }
                hard_break {}
                hard_break {}
            }
        }
        selection { (p, 3, Affinity::Downstream) }
    };

    let pages = rt.pages();
    let start = rt.selection().head;
    let (_, start_rect) =
        Cursor::bounds(&ctx(&rt.state()), &pages, start).expect("cursor bounds exist");

    let moved = Cursor::move_to_line_end(&ctx(&rt.state()), &pages, start)
        .expect("should move to line end");
    let (_, moved_rect) =
        Cursor::bounds(&ctx(&rt.state()), &pages, moved.head).expect("cursor bounds after move");

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, 3);
    assert_eq!(moved.head.affinity, Affinity::Downstream);
    assert!(
        moved_rect.y >= start_rect.y,
        "Cursor should not jump to a higher line"
    );
}

#[test]
fn test_consecutive_hard_breaks_visuals() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
                hard_break {}
                text { "Text" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let settings = rt.doc().settings();

    let pos_0 = crate::state::Position::new(p, 0, Affinity::Downstream);
    let (_, rect_0) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_0).unwrap();

    let pos_1 = crate::state::Position::new(p, 1, Affinity::Downstream);
    let (_, rect_1) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_1).unwrap();

    let pos_2 = crate::state::Position::new(p, 2, Affinity::Downstream);
    let (_, rect_2) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_2).unwrap();

    assert!(rect_1.y > rect_0.y, "Second line should be below first");
    assert!(rect_2.y > rect_1.y, "Third line should be below second");

    assert_eq!(rect_0.x, PAGE_MARGIN + settings.paragraph_indent * 16.0,);
    assert_eq!(
        rect_1.x, PAGE_MARGIN,
        "Second line start should be at left margin"
    );
    assert_eq!(
        rect_2.x, PAGE_MARGIN,
        "Third line start should be at left margin"
    );
}

#[test]
fn test_consecutive_hard_breaks_left_stays_on_second_line() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
                hard_break {}
            }
        }
        selection { (p, 2, Affinity::Downstream) }
    };

    let pages = rt.pages();
    let (_, rect_start) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    let moved =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect_start.y).unwrap();

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, 1);
    assert_eq!(moved.head.affinity, Affinity::Downstream);

    let (_, rect_after) = Cursor::bounds(&ctx(&rt.state()), &pages, moved.head).unwrap();
    let (_, rect_second_line) = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(p, 1, Affinity::Downstream),
    )
    .unwrap();
    assert!(
        (rect_after.y - rect_second_line.y).abs() < 0.1,
        "Cursor should stay on second line after moving left: expected y {}, got {}",
        rect_second_line.y,
        rect_after.y
    );
}

#[test]
fn test_hard_break_in_empty_paragraph() {
    let mut p = id!();

    let mut rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph { }
        }
        selection { (p, 0) }
    };

    rt.update(Message::InsertHardBreak);

    rt.layout();

    let pages = rt.pages();

    let pos_0 = crate::state::Position::new(p, 0, Affinity::Downstream);
    let cursor_0 = Cursor::bounds(&ctx(&rt.state()), &pages, pos_0);

    let pos_1 = crate::state::Position::new(p, 1, Affinity::Downstream);
    let cursor_1 = Cursor::bounds(&ctx(&rt.state()), &pages, pos_1);

    assert!(cursor_0.is_some(), "Cursor should be visible at position 0");
    assert!(cursor_1.is_some(), "Cursor should be visible at position 1");

    let (_, rect_0) = cursor_0.unwrap();
    let (_, rect_1) = cursor_1.unwrap();

    assert!(rect_0.height > 0.0, "Cursor 0 should have height");
    assert!(rect_1.height > 0.0, "Cursor 1 should have height");

    assert!(
        rect_1.y > rect_0.y,
        "Cursor after hard break should be on next line: y0={}, y1={}",
        rect_0.y,
        rect_1.y
    );
}

#[test]
fn test_hard_break_at_doc_end() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "End" }
                hard_break {}
            }
        }
        selection { (p, 3) }
    };

    let pages = rt.pages();

    let pos_4 = crate::state::Position::new(p, 4, Affinity::Downstream);
    let cursor_4 = Cursor::bounds(&ctx(&rt.state()), &pages, pos_4);

    assert!(
        cursor_4.is_some(),
        "Cursor should be visible after final hard break"
    );
    let (_, rect_4) = cursor_4.unwrap();

    let pos_3 = crate::state::Position::new(p, 3, Affinity::Upstream);
    let (_, rect_3) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_3).unwrap();

    assert!(rect_4.y > rect_3.y, "Final cursor should be on new line");
}

#[test]
fn test_hard_break_at_doc_start() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
                text { "Start" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    let pos_0 = crate::state::Position::new(p, 0, Affinity::Downstream);
    let (_, rect_0) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_0).unwrap();

    let pos_1 = crate::state::Position::new(p, 1, Affinity::Downstream);
    let (_, rect_1) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_1).unwrap();

    assert!(
        rect_1.y > rect_0.y,
        "Cursor after start hard break should be on next line"
    );
}

#[test]
fn test_affinity_after_insert_hard_break() {
    let mut p = id!();
    let initial = state! {
        doc {
            @p paragraph {
                text { "Hello" }
            }
        }
        selection { (p, 5, Affinity::Upstream) }
    };

    let actual = transact!(initial, |tr| tr.insert_hard_break().unwrap());

    let expected = state! {
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
            }
        }
        selection { (p, 6, Affinity::Downstream) }
    };

    assert_state_eq!(actual, expected);
}
#[test]
fn test_cursor_bounds_around_hard_break_in_empty_paragraph() {
    let mut p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    let cursor_result_0 = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(p, 0, Affinity::Downstream),
    );
    assert!(
        cursor_result_0.is_some(),
        "Cursor bounds should exist at offset 0 (before hard break)"
    );
    let (_, rect_0) = cursor_result_0.unwrap();

    let cursor_result_1 = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(p, 1, Affinity::Downstream),
    );
    assert!(
        cursor_result_1.is_some(),
        "Cursor bounds should exist at offset 1 (after hard break)"
    );
    let (_, rect_1) = cursor_result_1.unwrap();

    assert!(
        rect_1.y > rect_0.y,
        "Cursor after hard break should be on the next line"
    );
    assert!(rect_0.height > 0.0, "Cursor at 0 should have height > 0");
    assert!(rect_1.height > 0.0, "Cursor at 1 should have height > 0");
}

#[test]
fn test_cursor_respects_paragraph_indent_before_hard_break_in_empty_paragraph() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let settings = rt.doc().settings();

    let (_, rect) = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(p, 0, Affinity::Downstream),
    )
    .unwrap();

    assert_eq!(rect.x, PAGE_MARGIN + settings.paragraph_indent * 16.0);
}

#[test]
fn test_paragraph_indent_only_for_root_children() {
    let mut root_paragraph = id!();
    let mut quoted_paragraph = id!();
    let mut rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @root_paragraph paragraph {
                text { "Root paragraph" }
            }
            blockquote {
                @quoted_paragraph paragraph {
                    text { "Nested paragraph" }
                }
            }
        }
        selection { (root_paragraph, 0) }
    };

    rt.update(Message::SetParagraphIndent { indent: 2.0 });
    rt.layout();

    let pages = rt.pages();
    let settings = rt.doc().settings();

    let (_, root_rect) = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(root_paragraph, 0, Affinity::Downstream),
    )
    .unwrap();
    assert_eq!(root_rect.x, PAGE_MARGIN + settings.paragraph_indent * 16.0);

    let (_, quoted_rect) = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(quoted_paragraph, 0, Affinity::Downstream),
    )
    .unwrap();
    let blockquote_content_offset = 4.0 + 16.0;
    assert_eq!(quoted_rect.x, PAGE_MARGIN + blockquote_content_offset);
}

#[test]
fn test_cursor_bounds_at_end_of_text_with_mark() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text(marks: [italic()]) { "asdf" }
                hard_break {}
            }
        }
        selection { (p, 4, Affinity::Upstream) }
    };

    let pages = rt.pages();
    let cursor_at_end = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head);
    assert!(
        cursor_at_end.is_some(),
        "Marked line should show cursor at end of text before hard break"
    );

    let pos_after_break = Position::new(p, 5, Affinity::Downstream);
    let (_, rect_after_break) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_after_break)
        .expect("Cursor should exist after hard break");
    let moved = Cursor::move_left(
        &ctx(&rt.state()),
        &pages,
        pos_after_break,
        rect_after_break.y,
    )
    .unwrap();

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, 4);
    assert_eq!(
        moved.head.affinity,
        Affinity::Upstream,
        "Moving left across hard break should land upstream so cursor stays visible"
    );
    assert!(
        Cursor::bounds(&ctx(&rt.state()), &pages, moved.head).is_some(),
        "Cursor should remain visible after moving left from hard break"
    );
}

#[test]
fn test_emoji_left_right_navigation() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a👨‍👩‍👧‍👦b" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Move right from 'a'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let move1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(move1.head.offset, 1, "Should move to start of emoji");

    // Move right over emoji - should skip entire emoji sequence
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move1.head).unwrap();
    let move2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move1.head, rect.y + rect.height).unwrap();
    assert_eq!(move2.head.offset, 8, "Should skip entire family emoji");

    // Move right to 'b'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move2.head).unwrap();
    let move3 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move2.head, rect.y + rect.height).unwrap();
    assert_eq!(move3.head.offset, 9, "Should move to 'b'");

    // Move left from 'b'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move3.head).unwrap();
    let back1 = Cursor::move_left(&ctx(&rt.state()), &pages, move3.head, rect.y).unwrap();
    assert_eq!(back1.head.offset, 8, "Should move to end of emoji");

    // Move left over emoji
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, back1.head).unwrap();
    let back2 = Cursor::move_left(&ctx(&rt.state()), &pages, back1.head, rect.y).unwrap();
    assert_eq!(
        back2.head.offset, 1,
        "Should skip entire family emoji backwards"
    );

    // Move left to 'a'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, back2.head).unwrap();
    let back3 = Cursor::move_left(&ctx(&rt.state()), &pages, back2.head, rect.y).unwrap();
    assert_eq!(back3.head.offset, 0, "Should move back to 'a'");
}

#[test]
fn test_emoji_click_positioning_does_not_split_graphemes() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "👨‍👩‍👧‍👦" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Get the bounds of the start and end positions
    let pos_start = Position::new(p, 0, Affinity::Downstream);
    let (_, rect_start) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_start).unwrap();

    let pos_end = Position::new(p, 7, Affinity::Upstream);
    let (_, rect_end) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_end).unwrap();

    // Try clicking at various x positions throughout the emoji
    let test_positions = vec![
        rect_start.x + 1.0,
        rect_start.x + (rect_end.x - rect_start.x) * 0.25,
        rect_start.x + (rect_end.x - rect_start.x) * 0.5,
        rect_start.x + (rect_end.x - rect_start.x) * 0.75,
        rect_end.x - 1.0,
    ];

    for x in test_positions {
        let selection = Cursor::hit_test(
            &ctx(&rt.state()),
            &pages[0],
            x,
            rect_start.y + rect_start.height / 2.0,
        )
        .unwrap();
        assert!(
            selection.head.offset == 0 || selection.head.offset == 7,
            "Click at x={} should snap to either start (0) or end (7) of emoji, got {}",
            x,
            selection.head.offset
        );
    }
}

#[test]
fn test_emoji_vertical_navigation_maintains_position() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "👨‍👩‍👧‍👦" }
            }
            @p2 paragraph {
                text { "abc" }
            }
        }
        selection { (p2, 1) }
    };

    let pages = rt.pages();

    // Start at 'b' (offset 1 in second paragraph)
    let start_pos = rt.selection().head;
    let (_, start_rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();

    // Move up to first paragraph (with emoji)
    let moved_up = Cursor::move_up(&ctx(&rt.state()), &pages, start_pos, start_rect.x).unwrap();

    // The cursor should be at a valid grapheme boundary (0 or 7)
    assert!(
        moved_up.head.offset == 0 || moved_up.head.offset == 7,
        "After moving up, cursor should be at grapheme boundary, got {}",
        moved_up.head.offset
    );

    // Move back down
    let (_, up_rect) = Cursor::bounds(&ctx(&rt.state()), &pages, moved_up.head).unwrap();
    let moved_down =
        Cursor::move_down(&ctx(&rt.state()), &pages, moved_up.head, up_rect.x).unwrap();

    // Should be at a valid position in second paragraph
    assert_eq!(moved_down.head.node_id, p2);
    assert!(moved_down.head.offset <= 3, "Should be within text bounds");
}

#[test]
fn test_flag_emoji_navigation() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a🇺🇸b" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Move right from 'a' to flag start
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let move1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(move1.head.offset, 1, "Should move to start of flag");

    // Move right over flag (US flag is 2 regional indicator symbols, 1 grapheme cluster)
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move1.head).unwrap();
    let move2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move1.head, rect.y + rect.height).unwrap();
    assert_eq!(move2.head.offset, 3, "Should skip entire flag emoji");

    // Move right to 'b'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move2.head).unwrap();
    let move3 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move2.head, rect.y + rect.height).unwrap();
    assert_eq!(move3.head.offset, 4, "Should move to 'b'");
}

#[test]
fn test_skin_tone_emoji_navigation() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a👍🏽b" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Move right from 'a'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let move1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(move1.head.offset, 1, "Should move to start of thumbs up");

    // Move right over thumbs up with skin tone
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move1.head).unwrap();
    let move2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move1.head, rect.y + rect.height).unwrap();
    assert_eq!(
        move2.head.offset, 3,
        "Should skip entire emoji with skin tone modifier"
    );

    // Move right to 'b'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move2.head).unwrap();
    let move3 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move2.head, rect.y + rect.height).unwrap();
    assert_eq!(move3.head.offset, 4, "Should move to 'b'");
}

#[test]
fn test_combining_character_navigation() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "café" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Navigate through each character
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let move1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(move1.head.offset, 1, "Should move to 'a'");

    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move1.head).unwrap();
    let move2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move1.head, rect.y + rect.height).unwrap();
    assert_eq!(move2.head.offset, 2, "Should move to 'f'");

    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move2.head).unwrap();
    let move3 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move2.head, rect.y + rect.height).unwrap();
    // 'é' in "café" is just 1 char (precomposed), not a combining sequence
    assert_eq!(move3.head.offset, 3, "Should move to é");
}

#[test]
fn test_multiple_emoji_in_line() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "😀😃😄" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    let mut current_pos = rt.selection().head;
    let expected_offsets = vec![0, 1, 2, 3];

    for expected in expected_offsets {
        assert_eq!(
            current_pos.offset, expected,
            "Should be at offset {}",
            expected
        );

        if expected < 3 {
            let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, current_pos).unwrap();
            let next =
                Cursor::move_right(&ctx(&rt.state()), &pages, current_pos, rect.y + rect.height)
                    .unwrap();
            current_pos = next.head;
        }
    }
}

#[test]
fn test_emoji_at_line_boundaries() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "😀" }
                hard_break {}
                text { "😃" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();

    // Move right from start to emoji
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let move1 = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();
    assert_eq!(move1.head.offset, 1, "Should move to end of first emoji");

    // Move right to hard break
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move1.head).unwrap();
    let move2 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move1.head, rect.y + rect.height).unwrap();
    assert_eq!(move2.head.offset, 2, "Should move past hard break");

    // Move right to second emoji
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, move2.head).unwrap();
    let move3 =
        Cursor::move_right(&ctx(&rt.state()), &pages, move2.head, rect.y + rect.height).unwrap();
    assert_eq!(move3.head.offset, 3, "Should move to end of second emoji");
}

#[test]
fn test_vertical_nav_preserves_x_position_with_emoji() {
    // Test that navigating up from below an emoji at 3/4 position
    // goes to the right side of the emoji, not the left
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph {
                text { "👨‍👩‍👧‍👦" }  // Family emoji
            }
            @p2 paragraph {
                text { "iiii" }  // About same width as the emoji
            }
        }
        selection { (p2, 3) }  // At the 3rd 'i' (3/4 position)
    };

    let pages = rt.pages();

    // Get the x position of the cursor at the 3rd 'i'
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let preferred_x = rect.x;

    // Move up - should go to right side of emoji (offset 7) not left (offset 0)
    let moved_up =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, preferred_x).unwrap();

    assert_eq!(moved_up.head.node_id, p1);
    assert_eq!(
        moved_up.head.offset, 7,
        "Cursor should be at right side of emoji (offset 7), not left (offset 0). Got offset {}",
        moved_up.head.offset
    );
}

#[test]
fn test_heart_emoji_click_does_not_split() {
    // Test that clicking in the middle of ❤️ emoji doesn't place cursor in the middle
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "a❤️b" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let page = &pages[0];

    // Click at various positions and ensure we only get offset 1 (start) or 3 (end), never 2 (middle)
    for entry in page.spatial_index().iter() {
        if let Element::Line(line) = entry.element() {
            // Get the bounds of the emoji (should be around offset 1-3)
            for cluster in &line.metric.clusters {
                if cluster.start_offset >= 1 && cluster.end_offset <= 3 {
                    // Click at 25%, 50%, 75% of this cluster
                    for fraction in [0.25, 0.5, 0.75] {
                        let x =
                            entry.pos.x + line.metric.left + cluster.x + cluster.width * fraction;
                        let y = entry.pos.y + line.metric.top + line.metric.height / 2.0;

                        if let Some(selection) = Cursor::hit_test(&ctx(&rt.state()), page, x, y) {
                            assert!(
                                selection.head.offset == 1 || selection.head.offset == 3,
                                "Clicking at {}% of emoji cluster should give offset 1 or 3, not {}",
                                fraction * 100.0,
                                selection.head.offset
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
// 단어 단위 왼쪽 이동
fn test_cursor_move_word_left() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello world" }
            }
        }
        selection { (p, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
// 단어 단위 오른쪽 이동
fn test_cursor_move_word_right() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello world" }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_right(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 5);

    let (_, rect2) = Cursor::bounds(&ctx(&rt.state()), &pages, new_selection.head).unwrap();
    let new_selection_2 =
        Cursor::move_word_right(&ctx(&rt.state()), &pages, new_selection.head, rect2.y).unwrap();
    assert_eq!(new_selection_2.head.offset, 11);
}

#[test]
// soft wrap에서 이전 줄 끝 클릭하면 그 글자 뒤로 이동
fn test_soft_wrap_click_at_line_end() {
    let mut p = id!();
    let long_text = "aaaaaa bc dd";
    let rt = runtime! {
        viewport { paginated { width: 85.0, height: 400.0, margin: 0.0 } }
        doc { @p paragraph { text { long_text } } }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let lines = collect_lines_for_block(&pages, p);
    assert!(lines.len() >= 2, "wrap이 발생해야 합니다");

    let first_line = lines[0];
    let second_line = lines[1];

    let first_text = line_text_slice(first_line);
    let second_text = line_text_slice(second_line);

    assert!(
        first_text.trim_end().ends_with('b'),
        "첫 줄이 'b'로 끝나야 합니다: {}",
        first_text
    );
    assert!(
        second_text.starts_with('c'),
        "두 번째 줄은 다음 단어로 시작해야 합니다: {}",
        second_text
    );

    let b_index = long_text.find('b').unwrap();
    let b_pos = Position::new(p, b_index, Affinity::Downstream);

    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, b_pos).unwrap();

    let click_x = rect.x + rect.width + PAGE_MARGIN / 2.0;
    let click_y = rect.y + rect.height / 2.0;

    let selection = Cursor::hit_test(&ctx(&rt.state()), &pages[0], click_x, click_y).unwrap();

    assert_eq!(selection.head.node_id, p);
    assert_eq!(selection.head.offset, b_index + 1);
    assert_eq!(selection.head.affinity, Affinity::Upstream);
}

#[test]
// soft wrap에서 이전 줄 끝 단어가 한 글자일 때 그 글자 앞으로 이동
fn test_move_word_left_across_soft_wrap_single_char_end() {
    let mut p = id!();
    let long_text = "aaaaaa bc dd";
    let rt = runtime! {
        viewport { paginated { width: 85.0, height: 400.0, margin: 0.0 } }
        doc { @p paragraph { text { long_text } } }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let lines = collect_lines_for_block(&pages, p);
    assert!(lines.len() >= 2, "wrap이 발생해야 합니다");

    let first_line = lines[0];
    let second_line = lines[1];

    let first_text = line_text_slice(first_line);
    let second_text = line_text_slice(second_line);

    assert!(
        first_text.trim_end().ends_with('b'),
        "첫 줄이 'b'로 끝나야 합니다: {}",
        first_text
    );
    assert!(
        second_text.starts_with('c'),
        "두 번째 줄은 다음 단어로 시작해야 합니다: {}",
        second_text
    );

    let pos_at_second_line_start =
        Position::new(p, second_line.metric.start_offset, Affinity::Downstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_at_second_line_start).unwrap();

    let moved = Cursor::move_word_left(&ctx(&rt.state()), &pages, pos_at_second_line_start, rect.y)
        .unwrap();

    let b_byte = long_text.find('b').expect("텍스트에 'b'가 있어야 합니다");
    let b_offset = byte_to_char_offset(long_text, b_byte);

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, b_offset);
}

#[test]
// soft wrap에서 다음 줄 시작 단어가 한 글자일 때 그 글자 뒤로 이동
fn test_move_word_right_across_soft_wrap_single_char_end() {
    let mut p = id!();
    let long_text = "aaaaaa bc dd";
    let rt = runtime! {
        viewport { paginated { width: 85.0, height: 400.0, margin: 0.0 } }
        doc { @p paragraph { text { long_text } } }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let lines = collect_lines_for_block(&pages, p);
    assert!(lines.len() >= 2, "wrap이 발생해야 합니다");

    let first_line = lines[0];
    let second_line = lines[1];

    let first_text = line_text_slice(first_line);
    let second_text = line_text_slice(second_line);

    assert!(
        first_text.trim_end().ends_with('b'),
        "첫 줄이 'b'로 끝나야 합니다: {}",
        first_text
    );
    assert!(
        second_text.starts_with('c'),
        "두 번째 줄은 다음 단어로 시작해야 합니다: {}",
        second_text
    );

    let pos_at_first_line_end = Position::new(p, first_line.metric.end_offset, Affinity::Upstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, pos_at_first_line_end).unwrap();

    let moved = Cursor::move_word_right(
        &ctx(&rt.state()),
        &pages,
        pos_at_first_line_end,
        rect.y + rect.height,
    )
    .unwrap();

    let c_byte = long_text.find('c').expect("텍스트에 'c'가 있어야 합니다");
    let c_offset = byte_to_char_offset(long_text, c_byte);

    assert_eq!(moved.head.node_id, p);
    assert_eq!(moved.head.offset, c_offset + 1);
}

#[test]
// 단어 이동 시 이미지 노드를 선택함
fn test_move_word_select_image_node() {
    let mut p1 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph { text { "word1" } }
            image { }
            paragraph { text { "word2" } }
        }
        selection { (p1, 5) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let selection_right =
        Cursor::move_word_right(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(selection_right.head.node_id, NodeId::ROOT);
    assert_eq!(selection_right.anchor.offset, 1);
    assert_eq!(selection_right.head.offset, 2);
}

#[test]
#[ignore = "TODO: Fix after image node ID-based refactoring"]
fn test_move_word_across_image_node() {
    let mut p1 = id!();
    let mut img = id!();
    let mut rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph { text { "word1" } }
            @img image { }
        }
        selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
    };

    rt.update(Message::SetExternalElementHeight {
        node_id: img.to_string(),
        height: 100.0,
    });

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().anchor).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().anchor, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p1);
    assert_eq!(new_selection.head.offset, 5);
}

#[test]
// 줄 경계를 건너는 단어 이동
fn test_cursor_move_word_across_lines() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break {}
                text { "world" }
            }
        }
        selection { (p, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.offset, 5);
}

#[test]
// 하드 브레이크 직후에서 단어 왼쪽 이동
fn test_cursor_move_word_left_from_hard_break_start() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                hard_break {}
                text { "a" }
            }
        }
        selection { (p, 1) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
// 문단 끝에서 단어 오른쪽 이동 시 다음 문단 시작으로 이동
fn test_move_word_right_from_paragraph_end_to_next_paragraph_start() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph { text { "abc" } }
            @p2 paragraph { text { "def" } }
        }
        selection { (p1, 3) }
    };

    let pages = rt.pages();
    let start = rt.selection().head;
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start).unwrap();

    let moved =
        Cursor::move_word_right(&ctx(&rt.state()), &pages, start, rect.y + rect.height).unwrap();

    assert_eq!(moved.head.node_id, p2);
    assert_eq!(moved.head.offset, 0);
    assert_eq!(moved.head.affinity, Affinity::Downstream);
}
#[test]
fn test_word_nav_hard_break() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello" }
                hard_break { }
                text { "World" }
            }
        }
        selection { (p, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 5);
}

#[test]
fn test_word_nav_soft_wrap() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 50.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "Hello World" }
            }
        }
        selection { (p, 6) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
fn test_word_nav_soft_wrap_with_long_word() {
    let mut p = id!();
    let long_word = "Helloooooooooooooooooooo";
    let rt = runtime! {
        viewport { paginated { width: 50.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { long_word }
            }
        }
        selection { (p, long_word.chars().count()) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_word_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(new_selection.head.node_id, p);
    assert_eq!(new_selection.head.offset, 0);
}

#[test]
// 1페이지 하단 클릭은 Upstream, 그 다음 2페이지 상단 클릭은 Downstream
fn test_pagination_margin_clicks() {
    use crate::runtime::Effect;

    let mut p = id!();
    let text = "a".repeat(10000);
    let mut runtime = runtime! {
        viewport { paginated { width: 800.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { &text }
            }
        }
        selection { (p, 0) }
    };

    runtime.update(crate::runtime::Message::SetLayoutMode {
        mode: crate::model::LayoutMode::Paginated {
            page_width: 400.0,
            page_height: 400.0,
            page_margin_top: 20.0,
            page_margin_bottom: 20.0,
            page_margin_left: 20.0,
            page_margin_right: 20.0,
        },
    });

    runtime.layout();
    let pages = runtime.pages();
    assert!(pages.len() >= 2);

    let effects = runtime.handle_pointer_down(
        0,
        100.0,
        390.0,
        1,
        PointerButton::Primary,
        Modifier::default(),
    );
    assert!(effects.contains(&Effect::SelectionChanged));

    let sel1 = runtime.state().selection;
    assert_eq!(sel1.head.affinity, Affinity::Upstream);

    let effects = runtime.handle_pointer_down(
        1,
        100.0,
        10.0,
        1,
        PointerButton::Primary,
        Modifier::default(),
    );
    assert!(effects.contains(&Effect::SelectionChanged));

    let sel2 = runtime.state().selection;
    assert_eq!(sel2.head.affinity, Affinity::Downstream);
    assert_eq!(sel1.head.offset, sel2.head.offset);
    assert_ne!(sel1.head, sel2.head);
}

#[test]
fn test_document_end() {
    let mut n1 = id!();
    let mut n2 = id!();
    let rt = runtime! {
        viewport { continuous { width: 800.0 } }
        doc {
            @n1 paragraph {
                text { "asd" }
            }
            paragraph {
                text { "asd" }
            }
            paragraph {
                text { "asd" }
            }
            bullet_list {
                list_item {
                    paragraph {
                        text { "asd" }
                    }
                }
                list_item {
                    paragraph {
                        text { "asd" }
                    }
                }
                list_item {
                    paragraph {
                        text { "asdd" }
                    }
                    bullet_list {
                        list_item {
                            paragraph {
                                text { "ㅁㄴㅇㅁㄴㅇ" }
                            }
                            bullet_list {
                                list_item {
                                    paragraph {
                                        text { "ㅁㄴㅇ" }
                                    }
                                }
                            }
                        }
                    }
                }
                list_item {
                    paragraph {
                        text { "ㅁㄴ" }
                    }
                }
            }
            paragraph {}
            paragraph {
                text { "asd" }
            }
            paragraph {
                text { "asd" }
            }
            paragraph {
                text { "asd" }
            }
            @n2 paragraph {
                text { "asd" }
            }
        }
        selection { (n1, 0) }
    };

    rt.doc()
        .update_settings(|s| s.block_gap = 0.0)
        .expect("block gap 설정 실패");

    let pages = rt.pages();
    let new_selection = Cursor::move_to_document_end(&ctx(&rt.state()), &pages).unwrap();

    assert_eq!(new_selection.head.node_id, n2);
    assert_eq!(new_selection.head.offset, 3);
    assert_eq!(new_selection.head.affinity, Affinity::Upstream);
}

#[test]
fn test_document_start_in_continuous_mode() {
    let mut n1 = id!();
    let mut n2 = id!();
    let rt = runtime! {
        viewport { continuous { width: 800.0 } }
        doc {
            @n1 paragraph {
                text { "first" }
            }
            paragraph { }
            bullet_list {
                list_item {
                    paragraph { text { "middle" } }
                }
            }
            paragraph {
                text { "last" }
            }
            @n2 paragraph { text { "tail" } }
        }
        selection { (n2, 0) }
    };

    rt.doc()
        .update_settings(|s| s.block_gap = 0.0)
        .expect("block gap 설정 실패");

    let pages = rt.pages();
    let selection = Cursor::move_to_document_start(&ctx(&rt.state()), &pages).unwrap();

    assert_eq!(selection.head.node_id, n1);
    assert_eq!(selection.head.offset, 0);
    assert_eq!(selection.head.affinity, Affinity::Downstream);
}

#[test]
fn test_horizontal_rule_shift_up_extends_selection() {
    let rt = runtime! {
        viewport { paginated { width: 800.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            horizontal_rule {}
            horizontal_rule {}
        }
        selection { (NodeId::ROOT, 1, Affinity::Downstream) -> (NodeId::ROOT, 2, Affinity::Upstream) }
    };

    let pages = rt.pages();
    let ctx = ctx(&rt.state());

    let new_selection = Cursor::move_up(&ctx, &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(new_selection.anchor.node_id, NodeId::ROOT);
    assert_eq!(new_selection.anchor.offset, 0);
    assert_eq!(new_selection.anchor.affinity, Affinity::Downstream);

    assert_eq!(new_selection.head.node_id, NodeId::ROOT);
    assert_eq!(new_selection.head.offset, 1);
    assert_eq!(new_selection.head.affinity, Affinity::Upstream);
}

#[test]
fn test_dnd_between_paragraph_and_blockquote() {
    let mut para1 = id!();
    let mut blockquote_para = id!();

    let mut rt = runtime! {
        viewport { paginated { width: 800.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            @para1 paragraph {
                text { "Before blockquote" }
            }
            blockquote {
                @blockquote_para paragraph {
                    text { "Inside blockquote" }
                }
            }
        }
        selection { (para1, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para1_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para1, 0, Affinity::Downstream),
    );
    let blockquote_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(blockquote_para, 0, Affinity::Downstream),
    );

    let (_, para1_rect) = para1_bounds.expect("para1 should have bounds");
    let (_, bq_rect) = blockquote_bounds.expect("blockquote_para should have bounds");

    let gap_y = (para1_rect.y + para1_rect.height + bq_rect.y) / 2.0;
    let x = 100.0;

    println!(
        "Testing at x={}, y={} (gap between para1 bottom {} and bq top {})",
        x,
        gap_y,
        para1_rect.y + para1_rect.height,
        bq_rect.y
    );

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, gap_y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id,
        NodeId::ROOT,
        "Drop position should be at ROOT level, not inside blockquote"
    );
    assert_eq!(
        selection.head.offset, 1,
        "Drop position should be at offset 1 (before blockquote)"
    );
}

#[test]
fn test_dnd_between_blockquote_and_paragraph() {
    let mut blockquote_para = id!();
    let mut para_after = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            blockquote {
                @blockquote_para paragraph {
                    text { "Inside blockquote" }
                }
            }
            @para_after paragraph {
                text { "After blockquote" }
            }
        }
        selection { (blockquote_para, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let bq_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(blockquote_para, 0, Affinity::Downstream),
    );
    let para_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para_after, 0, Affinity::Downstream),
    );

    let (_, bq_rect) = bq_bounds.expect("blockquote_para should have bounds");
    let (_, para_rect) = para_bounds.expect("para_after should have bounds");

    let gap_y = (bq_rect.y + bq_rect.height + para_rect.y) / 2.0;
    let x = 100.0;

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, gap_y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id,
        NodeId::ROOT,
        "Should be at ROOT level"
    );
    assert_eq!(selection.head.offset, 1, "Should be at offset 1");
}

#[test]
fn test_dnd_between_paragraph_and_list() {
    let mut para = id!();
    let mut list_para = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @para paragraph {
                text { "Before list" }
            }
            bullet_list {
                list_item {
                    @list_para paragraph {
                        text { "List item" }
                    }
                }
            }
        }
        selection { (para, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para, 0, Affinity::Downstream),
    );
    let list_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(list_para, 0, Affinity::Downstream),
    );

    let (_, para_rect) = para_bounds.expect("para should have bounds");
    let (_, list_rect) = list_bounds.expect("list_para should have bounds");

    let gap_y = (para_rect.y + para_rect.height + list_rect.y) / 2.0;
    let x = 100.0;

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, gap_y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id,
        NodeId::ROOT,
        "Should be at ROOT level"
    );
    assert_eq!(selection.head.offset, 1, "Should be at offset 1");
}

#[test]
fn test_dnd_inline_position_in_paragraph() {
    let mut para = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @para paragraph {
                text { "Hello World" }
            }
        }
        selection { (para, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para, 0, Affinity::Downstream),
    );

    let (_, para_rect) = para_bounds.expect("para should have bounds");

    let x = para_rect.x + 50.0;
    let y = para_rect.y + para_rect.height / 2.0;

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id, para,
        "Drop should be inside paragraph, not at ROOT level"
    );
}

#[test]
fn test_dnd_inline_position_with_multiple_paragraphs() {
    let mut para1 = id!();
    let mut para2 = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @para1 paragraph {
                text { "First paragraph" }
            }
            @para2 paragraph {
                text { "Second paragraph" }
            }
        }
        selection { (para1, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para2_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para2, 0, Affinity::Downstream),
    )
    .expect("para2 should have bounds");
    let (_, para2_rect) = para2_bounds;

    let x = para2_rect.x + 50.0;
    let y = para2_rect.y + para2_rect.height / 2.0;

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id, para2,
        "Drop should be inside para2, not at ROOT level"
    );
}

#[test]
fn test_dnd_inline_position_in_first_paragraph() {
    let mut para1 = id!();
    let mut para2 = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @para1 paragraph {
                text { "First paragraph" }
            }
            @para2 paragraph {
                text { "Second paragraph" }
            }
        }
        selection { (para1, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para1_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para1, 0, Affinity::Downstream),
    )
    .expect("para1 should have bounds");
    let (_, para1_rect) = para1_bounds;

    let x = para1_rect.x + 50.0;
    let y = para1_rect.y + para1_rect.height / 2.0;

    let selection = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], x, y);

    let selection = selection.expect("Should find a drop position");
    assert_eq!(
        selection.head.node_id, para1,
        "Drop should be inside para1, not at ROOT level or para2"
    );
}
#[test]
fn test_dnd_page_margins() {
    let mut para1 = id!();
    let mut para2 = id!();

    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @para1 paragraph {
                text { "First paragraph" }
            }
            @para2 paragraph {
                text { "Last paragraph" }
            }
        }
        selection { (para1, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let para1_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para1, 0, Affinity::Downstream),
    )
    .expect("para1 should have bounds");
    let (_, para1_rect) = para1_bounds;

    let top_x = para1_rect.x + 10.0;
    let top_y = if para1_rect.y > 10.0 {
        para1_rect.y / 2.0
    } else {
        0.0
    };

    println!("Testing Top Margin at y={}", top_y);

    let selection_top = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], top_x, top_y)
        .expect("Should find selection at top margin");

    assert_eq!(
        selection_top.head.node_id,
        NodeId::ROOT,
        "Top drop should be ROOT"
    );
    assert_eq!(selection_top.head.offset, 0, "Top drop should be offset 0");

    let para2_bounds = Cursor::bounds(
        &ctx(&rt.state()),
        &pages,
        Position::new(para2, 0, Affinity::Downstream),
    )
    .expect("para2 should have bounds");
    let (_, para2_rect) = para2_bounds;

    let bottom_x = para2_rect.x + 10.0;
    let bottom_y = para2_rect.y + para2_rect.height + 50.0;

    println!("Testing Bottom Margin at y={}", bottom_y);

    let selection_bottom = Cursor::hit_test_dnd(&ctx(&rt.state()), &pages[0], bottom_x, bottom_y)
        .expect("Should find selection at bottom margin");

    assert_eq!(
        selection_bottom.head.node_id,
        NodeId::ROOT,
        "Bottom drop should be ROOT"
    );
    assert_eq!(
        selection_bottom.head.offset, 2,
        "Bottom drop should be offset 2"
    );
}

#[test]
fn test_hit_test_on_selected_image_preserves_selection() {
    let mut p1 = id!();
    let mut img = id!();
    let mut p2 = id!();
    let mut rt = runtime! {
        viewport { 800, 600, 1.0 }
        doc {
            @p1 paragraph {
                text { "Line 1" }
            }
            @img image(id: Some("test-image-id".to_string()),)
            @p2 paragraph {
                text { "Line 2" }
            }
        }
        selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2) }
    };

    rt.layout();
    let (page_idx, rect) = Cursor::bounds(
        &ctx(&rt.state()),
        &rt.pages(),
        Position::new(NodeId::ROOT, 1, Affinity::Downstream),
    )
    .unwrap();

    let click_x = rect.x + rect.width / 2.0;
    let click_y = rect.y + rect.height / 2.0;

    rt.update(Message::PointerDown {
        page_idx,
        x: click_x,
        y: click_y,
        click_count: 1,
        modifier: Modifier::default(),
        button: PointerButton::Primary,
    });

    rt.update(Message::PointerUp {
        page_idx,
        x: click_x,
        y: click_y,
        modifier: Modifier::default(),
        button: PointerButton::Primary,
    });

    assert_eq!(
        rt.state().selection,
        Selection::new(
            Position::new(NodeId::ROOT, 1, Affinity::Downstream),
            Position::new(NodeId::ROOT, 2, Affinity::Downstream)
        ),
        "Selection should remain unchanged"
    );
}

#[test]
fn test_hit_test_below_wrapped_text_goes_to_document_end() {
    let mut p = id!();
    let rt = runtime! {
        viewport { paginated { width: 200.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @p paragraph {
                text { "This is a long paragraph that will wrap into multiple lines when the viewport is narrow enough. We need enough text to ensure multiple lines." }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let lines = collect_lines_for_block(&pages, p);
    assert!(
        lines.len() >= 2,
        "Text should wrap into at least 2 lines, got {}",
        lines.len()
    );

    let last_line = lines.last().unwrap();
    let expected_end_offset = last_line.metric.end_offset;

    let mut last_line_bottom = 0.0f32;
    for entry in pages[0].spatial_index().iter() {
        if let Element::Line(line) = entry.element() {
            if line.block_id == p {
                let bottom = entry.pos.y + entry.size.height;
                last_line_bottom = last_line_bottom.max(bottom);
            }
        }
    }

    let click_x = 100.0;
    let click_y = last_line_bottom + 50.0;

    let selection = Cursor::hit_test(&ctx(&rt.state()), &pages[0], click_x, click_y).unwrap();

    assert_eq!(selection.head.node_id, p);
    assert_eq!(
        selection.head.offset, expected_end_offset,
        "Clicking below wrapped text should place cursor at document end (offset {}), not at an earlier line's end",
        expected_end_offset
    );
}

#[test]
fn test_table_navigate_right_to_next_cell() {
    let mut cell1_p = id!();
    let mut cell2_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @cell1_p paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        @cell2_p paragraph {
                            text { "B" }
                        }
                    }
                }
            }
        }
        selection { (cell1_p, 1) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();

    assert_eq!(
        new_selection.head.node_id, cell2_p,
        "Should move to next cell"
    );
    assert_eq!(
        new_selection.head.offset, 0,
        "Should be at start of next cell"
    );
}

#[test]
fn test_table_navigate_left_to_previous_cell() {
    let mut cell1_p = id!();
    let mut cell2_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @cell1_p paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        @cell2_p paragraph {
                            text { "B" }
                        }
                    }
                }
            }
        }
        selection { (cell2_p, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell1_p,
        "Should move to previous cell"
    );
    assert_eq!(
        new_selection.head.offset, 1,
        "Should be at end of previous cell"
    );
}

#[test]
fn test_table_navigate_right_wraps_to_next_row_first_cell() {
    let mut r1c2_p = id!();
    let mut r2c1_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        @r1c2_p paragraph {
                            text { "B" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @r2c1_p paragraph {
                            text { "C" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "D" }
                        }
                    }
                }
            }
        }
        selection { (r1c2_p, 1) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection = Cursor::move_right(
        &ctx(&rt.state()),
        &pages,
        rt.selection().head,
        rect.y + rect.height,
    )
    .unwrap();

    assert_eq!(
        new_selection.head.node_id, r2c1_p,
        "Should wrap to first cell of next row"
    );
    assert_eq!(new_selection.head.offset, 0, "Should be at start of cell");
}

#[test]
fn test_table_navigate_left_wraps_to_previous_row_last_cell() {
    let mut r1c2_p = id!();
    let mut r2c1_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        @r1c2_p paragraph {
                            text { "B" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @r2c1_p paragraph {
                            text { "C" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "D" }
                        }
                    }
                }
            }
        }
        selection { (r2c1_p, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y).unwrap();

    assert_eq!(
        new_selection.head.node_id, r1c2_p,
        "Should wrap to last cell of previous row"
    );
    assert_eq!(new_selection.head.offset, 1, "Should be at end of cell");
}

#[test]
fn test_table_navigate_down_to_cell_below() {
    let mut r1c1_p = id!();
    let mut r2c1_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @r1c1_p paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "B" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @r2c1_p paragraph {
                            text { "C" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "D" }
                        }
                    }
                }
            }
        }
        selection { (r1c1_p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, r2c1_p,
        "Should move to cell below in same column"
    );
}

#[test]
fn test_table_navigate_up_to_cell_above() {
    let mut r1c1_p = id!();
    let mut r2c1_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @r1c1_p paragraph {
                            text { "A" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "B" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @r2c1_p paragraph {
                            text { "C" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "D" }
                        }
                    }
                }
            }
        }
        selection { (r2c1_p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, r1c1_p,
        "Should move to cell above in same column"
    );
}

#[test]
fn test_cell_navigation_with_paragraphs_before_table() {
    let mut p1 = id!();
    let mut p2 = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {
                text { "Paragraph before table 1" }
            }
            paragraph {
                text { "Paragraph before table 2" }
            }
            table {
                table_row {
                    table_cell {
                        @p1 paragraph {
                            text { "Cell Line1" }
                        }
                        @p2 paragraph {
                            text { "Cell Line2" }
                        }
                        paragraph {
                            text { "Cell Line3" }
                        }
                        paragraph {
                            text { "Cell Line4" }
                        }
                    }
                }
            }
        }
        selection { (p2, 3) }
    };

    let pages = rt.pages();

    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, p1,
        "Up from Cell Line2 should go to Cell Line1 (with paragraphs before table)"
    );
}

#[test]
fn test_hard_break_navigation_down_in_table_cell() {
    let mut p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {
                text { "Before table" }
            }
            table {
                table_row {
                    table_cell {
                        @p paragraph {
                            text { "Line 1" }
                            hard_break {}
                            text { "Line 2" }
                        }
                    }
                }
            }
        }
        selection { (p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, p,
        "Should stay in same paragraph"
    );
    assert_eq!(new_selection.head.offset, 7, "Should be at start of Line 2");
}

#[test]
fn test_hard_break_navigation_up_in_table_cell() {
    let mut p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {
                text { "Before table" }
            }
            table {
                table_row {
                    table_cell {
                        @p paragraph {
                            text { "Line 1" }
                            hard_break {}
                            text { "Line 2" }
                        }
                    }
                }
            }
        }
        selection { (p, 10) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, p,
        "Should stay in same paragraph"
    );
    assert!(
        new_selection.head.offset < 7,
        "Should be in Line 1 (before hard_break)"
    );
}

#[test]
fn test_consecutive_hard_breaks_navigation_no_table() {
    let mut p = id!();

    let rt = runtime! {
        viewport { paginated { width: 600.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {
                text { "Before" }
            }
            @p paragraph {
                text { "Line1" }       // offset 0-4
                hard_break {}          // offset 5
                text { "L2" }          // offset 6-7
                hard_break {}          // offset 8
                text { "Line3" }       // offset 9-13
                hard_break {}          // offset 14
                hard_break {}          // offset 15
                text { "Line5" }       // offset 16-20
                hard_break {}          // offset 21
                text { "LastLine" }    // offset 22+
            }
        }
        selection { (p, 20) }  // At end of "Line5" (offset 20)
    };

    let pages = rt.pages();

    let sel1 = Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert!(
        sel1.head.offset >= 22,
        "Down from Line5 should reach LastLine (offset 22+), got {}",
        sel1.head.offset
    );
}

#[test]
fn test_consecutive_hard_breaks_navigation_in_table_cell() {
    let mut p = id!();

    let rt = runtime! {
        viewport { paginated { width: 600.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {
                text { "Before" }
            }
            paragraph {
                text { "Before2" }
            }
            table {
                table_row {
                    table_cell {
                        @p paragraph {
                            text { "Line1" }       // offset 0-4 (5 chars)
                            hard_break {}          // offset 5
                            text { "L2" }          // offset 6-7 (2 chars)
                            hard_break {}          // offset 8
                            text { "Line3" }       // offset 9-13 (5 chars)
                            hard_break {}          // offset 14
                            hard_break {}          // offset 15 (consecutive - empty line)
                            text { "Line5" }       // offset 16-20 (5 chars)
                            hard_break {}          // offset 21
                            text { "LastLine" }    // offset 22+ (shorter text)
                        }
                    }
                }
            }
        }
        selection { (p, 7) }  // At end of "L2"
    };

    let pages = rt.pages();

    let sel1 = Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();
    assert_eq!(sel1.head.node_id, p, "Should stay in paragraph");

    let sel2 = Cursor::move_down(&ctx(&rt.state()), &pages, sel1.head, 0.0).unwrap();
    let sel3 = Cursor::move_down(&ctx(&rt.state()), &pages, sel2.head, 0.0).unwrap();
    let sel4 = Cursor::move_down(&ctx(&rt.state()), &pages, sel3.head, 0.0).unwrap();
    let sel5 = Cursor::move_down(&ctx(&rt.state()), &pages, sel4.head, 0.0);
    let sel6 = if let Some(s5) = &sel5 {
        Cursor::move_down(&ctx(&rt.state()), &pages, s5.head, 0.0)
    } else {
        None
    };

    let all_offsets = [
        Some(sel1.head.offset),
        Some(sel2.head.offset),
        Some(sel3.head.offset),
        Some(sel4.head.offset),
        sel5.as_ref().map(|s| s.head.offset),
        sel6.as_ref().map(|s| s.head.offset),
    ];

    let reached_last_line = all_offsets
        .iter()
        .any(|o| o.map(|x| x >= 22).unwrap_or(false));
    assert!(
        reached_last_line,
        "Should reach last line (offset 22+), got offsets: {:?}",
        all_offsets
    );
}

#[test]
fn test_table_exit_up_from_first_row() {
    let mut para_before = id!();
    let mut table_cell_p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @para_before paragraph {
                text { "Before table" }
            }
            table {
                table_row {
                    table_cell {
                        @table_cell_p paragraph {
                            text { "Cell content" }
                        }
                    }
                }
            }
        }
        selection { (table_cell_p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, para_before,
        "Up from first row should exit table to paragraph above"
    );
}

#[test]
fn test_table_exit_down_from_last_row() {
    let mut table_cell_p = id!();
    let mut para_after = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @table_cell_p paragraph {
                            text { "Cell" }
                        }
                    }
                }
            }
            @para_after paragraph {
                text { "After table" }
            }
        }
        selection { (table_cell_p, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, rect.x).unwrap();

    assert_eq!(
        new_selection.head.node_id, para_after,
        "Down from last row should exit table to paragraph below"
    );
}

#[test]
fn test_table_entry_selects_correct_column() {
    let mut para_before = id!();
    let mut cell1_p = id!();
    let mut cell2_p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @para_before paragraph {
                text { "Before table - this text is long enough" }
            }
            table {
                table_row {
                    table_cell {
                        @cell1_p paragraph {
                            text { "Cell 1" }
                        }
                    }
                    table_cell {
                        @cell2_p paragraph {
                            text { "Cell 2" }
                        }
                    }
                }
            }
        }
        selection { (para_before, 30) }
    };

    let pages = rt.pages();

    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 300.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell2_p,
        "Entering table with preferred_x in second column should go to cell2"
    );
}

#[test]
fn test_table_exit_preserves_preferred_x() {
    let mut para_before = id!();
    let mut cell_p = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            @para_before paragraph {
                text { "Before table with some longer text here" }
            }
            table {
                table_row {
                    table_cell {
                        @cell_p paragraph {
                            text { "Cell content" }
                        }
                    }
                }
            }
        }
        selection { (cell_p, 5) }
    };

    let pages = rt.pages();

    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 100.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, para_before,
        "Up from first row should exit table to paragraph above"
    );
    assert!(
        new_selection.head.offset > 0,
        "Exit should maintain preferred_x, not go to start"
    );
}

#[test]
fn test_table_entry_down_goes_to_first_line_with_hard_breaks() {
    let mut para_above = id!();
    let mut cell_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            @para_above paragraph {
                text { "Above" }
            }
            table {
                table_row {
                    table_cell {
                        @cell_p paragraph {
                            text { "Line1" }
                            hard_break {}
                            text { "Line2" }
                            hard_break {}
                            text { "Line3" }
                        }
                    }
                    table_cell {
                        paragraph {}
                    }
                }
            }
        }
        selection { (para_above, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell_p,
        "Down from paragraph should enter table cell"
    );
    assert!(
        new_selection.head.offset <= 5,
        "Should be on first line (Line1), but offset is {}",
        new_selection.head.offset
    );
}

#[test]
fn test_table_to_table_up_goes_to_last_line_with_hard_breaks() {
    let mut table1_cell_p = id!();
    let mut table2_cell_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @table1_cell_p paragraph {
                            text { "Line1" }
                            hard_break {}
                            text { "Line2" }
                            hard_break {}
                            text { "Line3" }
                        }
                    }
                    table_cell {
                        paragraph {}
                    }
                }
            }
            table {
                table_row {
                    table_cell {
                        @table2_cell_p paragraph {
                            text { "TableB" }
                        }
                    }
                    table_cell {
                        paragraph {}
                    }
                }
            }
        }
        selection { (table2_cell_p, 0) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, table1_cell_p,
        "Up from Table2 should go to Table1 cell"
    );
    assert!(
        new_selection.head.offset >= 12,
        "Should be on last line (Line3), but offset is {}",
        new_selection.head.offset
    );
}

#[test]
fn test_click_in_short_cell_empty_space_stays_in_cell() {
    let mut left_p = id!();
    let mut right_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @left_p paragraph {
                            text { "Line1" }
                            hard_break {}
                            text { "Line2" }
                            hard_break {}
                            text { "Line3" }
                        }
                    }
                    table_cell {
                        @right_p paragraph {
                            text { "Short" }
                        }
                    }
                }
            }
        }
        selection { (left_p, 0) }
    };

    let pages = rt.pages();
    let test_x = 280.0;
    let test_y = 40.0;
    let selection = Cursor::hit_test(&ctx(&rt.state()), &pages[0], test_x, test_y);

    assert!(selection.is_some(), "Should find a selection");
    let sel = selection.unwrap();

    assert_eq!(
        sel.head.node_id, right_p,
        "Click in right cell's empty space should go to right cell paragraph, not left cell"
    );
}

#[test]
fn test_table_cell_up_stays_in_scope() {
    let mut n1 = id!();
    let mut n2 = id!();

    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {}
            table {
                table_row {
                    table_cell {
                        @n1 paragraph {
                            text { "1" }
                        }
                        @n2 paragraph {
                            text { "2" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "3" }
                        }
                    }
                }
            }
            paragraph {}
        }
        selection { (n2, 1, Affinity::Upstream) }
    };

    let pages = rt.pages();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_selection.head.node_id, n1,
        "Up from paragraph '2' should go to paragraph '1' within same cell, not exit table"
    );
}

#[test]
fn test_table_entry_from_above() {
    let mut above_para = id!();
    let mut cell_para = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            @above_para paragraph {
                text { "Above the table" }
            }
            table {
                table_row {
                    table_cell {
                        @cell_para paragraph {
                            text { "Cell content" }
                        }
                    }
                }
            }
        }
        selection { (above_para, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, rect.x).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell_para,
        "Down from paragraph above table should enter table's first cell"
    );
}

#[test]
fn test_table_entry_multi_column() {
    let mut above_para = id!();
    let mut cell1_para = id!();
    let mut cell2_para = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            @above_para paragraph {
                text { "Above the table" }
            }
            table {
                table_row {
                    table_cell {
                        @cell1_para paragraph {
                            text { "Cell 1" }
                        }
                    }
                    table_cell {
                        @cell2_para paragraph {
                            text { "Cell 2" }
                        }
                    }
                }
            }
        }
        selection { (above_para, 5) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, rect.x).unwrap();

    assert!(
        new_selection.head.node_id == cell1_para || new_selection.head.node_id == cell2_para,
        "Down from paragraph above table should enter a table cell, got {:?}",
        new_selection.head.node_id
    );
}

#[test]
fn test_table_entry_from_below() {
    let mut below_para = id!();
    let mut cell_para = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @cell_para paragraph {
                            text { "Cell content" }
                        }
                    }
                }
            }
            @below_para paragraph {
                text { "Below the table" }
            }
        }
        selection { (below_para, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_selection =
        Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, rect.x).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell_para,
        "Up from paragraph below table should enter table's cell"
    );
}

#[test]
fn test_down_in_single_line_cell_goes_to_cell_below() {
    let mut cell1_p = id!();
    let mut cell2_p = id!();
    let mut cell3_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 800.0, margin: PAGE_MARGIN } }
        doc {
            table {
                table_row {
                    table_cell {
                        @cell1_p paragraph {
                            text { "Short" }
                        }
                    }
                    table_cell {
                        @cell2_p paragraph {
                            text { "Line 1" }
                            hard_break {}
                            text { "Line 2" }
                            hard_break {}
                            text { "Line 3" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @cell3_p paragraph {
                            text { "Below" }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "Other" }
                        }
                    }
                }
            }
        }
        selection { (cell1_p, 0) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();

    let new_selection =
        Cursor::move_down(&ctx(&rt.state()), &pages, rt.selection().head, rect.x).unwrap();

    assert_eq!(
        new_selection.head.node_id, cell3_p,
        "Down in single-line cell should go to cell BELOW, not adjacent cell. Got {:?}",
        new_selection.head.node_id
    );
}

#[test]
fn test_table_cell_horizontal_navigation() {
    let mut p1 = id!();
    let mut p2 = id!();
    let mut rt = runtime! {
        doc {
            table {
                table_row {
                    table_cell {
                        @p1 paragraph { text { "Line 1" } }
                        @p2 paragraph { text { "Line 2" } }
                    }
                }
            }
        }
    };

    rt.layout();

    let pages = rt.pages();

    let start_pos = Position::new(p2, 0, Affinity::Downstream);

    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();
    let new_selection =
        Cursor::move_left(&ctx(&rt.state()), &pages, start_pos, rect.y).expect("Should move left");

    assert_eq!(
        new_selection.head.node_id, p1,
        "Should move to p1 line above, but moved to {:?}",
        new_selection.head.node_id
    );
    assert_eq!(
        new_selection.head.offset, 6,
        "Should be at the end of 'Line 1' (offset 6)"
    );
}

#[test]
fn test_table_cell_3para_horizontal_navigation() {
    let mut p1 = id!();
    let mut p2 = id!();
    let mut p3 = id!();
    let mut rt = runtime! {
        doc {
            paragraph {}
            table {
                table_row {
                    table_cell {
                        @p1 paragraph { text { "Para 1" } }
                        @p2 paragraph { text { "Para 2" } }
                        @p3 paragraph { text { "Para 3" } }
                    }
                }
            }
        }
    };

    rt.layout();

    let pages = rt.pages();

    let start_pos = Position::new(p2, 0, Affinity::Downstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();
    let new_sel =
        Cursor::move_left(&ctx(&rt.state()), &pages, start_pos, rect.y).expect("Should move left");

    assert_eq!(
        new_sel.head.node_id, p1,
        "Left from p2 start should go to p1, but went to {:?}",
        new_sel.head.node_id
    );
    assert_eq!(new_sel.head.offset, 6, "Should be at end of 'Para 1'");

    let start_pos2 = Position::new(p1, 6, Affinity::Upstream);
    let (_, rect2) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos2).unwrap();
    let new_sel2 = Cursor::move_right(&ctx(&rt.state()), &pages, start_pos2, rect2.y)
        .expect("Should move right");

    assert_eq!(
        new_sel2.head.node_id, p2,
        "Right from p1 end should go to p2, but went to {:?}",
        new_sel2.head.node_id
    );
    assert_eq!(new_sel2.head.offset, 0, "Should be at start of 'Para 2'");
}

#[test]
fn test_table_cell_up_stays_in_column() {
    let mut cell1_p = id!();
    let mut cell2_p = id!();
    let mut cell3_p = id!();
    let mut cell4_p = id!();

    let mut rt = runtime! {
        viewport { 800.0, 600.0, 1.0 }
        doc {
            table {
                table_row {
                    table_cell {
                        @cell1_p paragraph { text { "short" } }
                    }
                    table_cell {
                        @cell2_p paragraph {
                            text { "long" }
                            hard_break {}
                            text { "long" }
                            hard_break {}
                            text { "long" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        @cell3_p paragraph { text { "target" } }
                    }
                    table_cell {
                        @cell4_p paragraph { text { "dummy" } }
                    }
                }
            }
        }
        selection { (cell3_p, 0) }
    };

    rt.layout();
    let pages = rt.pages();

    let new_sel = Cursor::move_up(&ctx(&rt.state()), &pages, rt.selection().head, 0.0).unwrap();

    assert_eq!(
        new_sel.head.node_id, cell1_p,
        "Up from cell3 should go to cell1 (short), but went to {:?} (likely cell2 adj)",
        new_sel.head.node_id
    );
}

#[test]
fn test_page_boundary_navigation() {
    let mut p1 = id!();
    let mut filler = id!();
    let mut p2 = id!();
    let rt = runtime! {
        viewport { paginated { width: 500.0, height: 200.0, margin: PAGE_MARGIN } }
        doc {
             @p1 paragraph { text { "Page 1 Content" } }
             @filler paragraph {
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" } hard_break {}
                 text { "Filler" }
             }
             @p2 paragraph { text { "Page 2 Content" } }
        }
    };

    let pages = rt.pages();
    assert!(
        pages.len() >= 2,
        "Should have at least 2 pages. Got {}",
        pages.len()
    );

    let start_pos_1 = Position::new(p1, 0, Affinity::Downstream);
    let new_sel_down =
        Cursor::move_down(&ctx(&rt.state()), &pages, start_pos_1, 0.0).expect("Should move down");
    assert_eq!(
        new_sel_down.head.node_id, filler,
        "Down from p1 should go to filler"
    );

    let start_pos_2 = Position::new(p2, 0, Affinity::Downstream);
    let new_sel_up = Cursor::move_up(&ctx(&rt.state()), &pages, start_pos_2, 0.0)
        .expect("Should move up from p2");
    assert_eq!(
        new_sel_up.head.node_id, filler,
        "Up from p2 should go to filler"
    );
}

#[test]
fn test_table_entry_from_right() {
    let mut p1 = id!();
    let mut t1 = id!();
    let mut cell_p = id!();
    let rt = runtime! {
        viewport { paginated { width: 800.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph { text { "Above" } }
            @t1 table {
                table_row {
                    table_cell {
                         @cell_p paragraph { text { "Cell Content" } }
                    }
                }
            }
        }
    };

    let pages = rt.pages();
    let preferred_x = 2000.0;
    let new_sel = Cursor::move_down(
        &ctx(&rt.state()),
        &pages,
        Position::new(p1, 0, Affinity::Downstream),
        preferred_x,
    )
    .expect("Should move down");

    assert_eq!(
        new_sel.head.node_id, cell_p,
        "Should enter table cell even from far right"
    );
}

#[test]
fn test_hard_break_left_navigation() {
    let mut p1 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
             @p1 paragraph {
                 text { "Line 1" }
                 hard_break {}
                 text { "Line 2" }
             }
        }
        selection { (p1, 7) }
    };

    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_sel = Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y)
        .expect("Should move left");

    assert_eq!(new_sel.head.node_id, p1, "Should stay in paragraph");
    assert_eq!(new_sel.head.offset, 6, "Should move to end of Line 1");
}

#[test]
fn test_table_hard_break_left_navigation() {
    let mut p1 = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 400.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {}
            table {
                table_row {
                    table_cell {
                        @p1 paragraph {
                            text { "Line 1" } hard_break {} text { "Line 2" }
                        }
                    }
                    table_cell {
                         paragraph { text { "Col 2" } }
                    }
                }
            }
        }
        selection { (p1, 7) }
    };
    let pages = rt.pages();
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, rt.selection().head).unwrap();
    let new_sel = Cursor::move_left(&ctx(&rt.state()), &pages, rt.selection().head, rect.y)
        .expect("Should move left");

    assert_eq!(new_sel.head.node_id, p1, "Should stay in paragraph");
    assert_eq!(new_sel.head.offset, 6, "Should move to end of Line 1");
}

#[test]
fn test_table_at_start_navigation() {
    let mut p1 = id!();
    let mut p2 = id!();
    let rt = runtime! {
        doc {
            table {
                table_row {
                    table_cell { @p1 paragraph { text { "Col 1" } } }
                    table_cell { @p2 paragraph { text { "Col 2" } } }
                }
            }
        }
    };

    let pages = rt.pages();
    let start_pos = Position::new(p1, 0, Affinity::Downstream);

    let new_sel_up = Cursor::move_up(&ctx(&rt.state()), &pages, start_pos, 0.0);
    if let Some(sel) = new_sel_up {
        assert_eq!(
            sel.head.node_id, p1,
            "Up from start should stay in p1 (or noop)"
        );
    }

    let new_sel_left = Cursor::move_left(&ctx(&rt.state()), &pages, start_pos, 0.0);
    assert!(
        new_sel_left.is_none() || new_sel_left.unwrap().head.node_id == p1,
        "Left from start should stay in p1 (or None)"
    );
}

#[test]
fn test_page_boundary_navigation_repr() {
    let mut p1 = id!();
    let mut p2 = id!();
    let mut rt = runtime! {
        viewport { paginated { width: 400.0, height: 100.0, margin: PAGE_MARGIN } }
        doc {
            @p1 paragraph { text { "Page 1 Content" } }
            @p2 paragraph { text { "Page 2 Content" } }
        }
        selection { (p1, 0) }
    };

    rt.layout();
    let pages = rt.pages();
    assert_eq!(pages.len(), 2, "Should have 2 pages");

    let start_pos = Position::new(p1, 0, Affinity::Downstream);
    let (_, rect1) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();

    let sel_down = Cursor::move_down(&ctx(&rt.state()), &pages, start_pos, rect1.x)
        .expect("Should move down to next page");

    assert_eq!(
        sel_down.head.node_id, p2,
        "Down from Page 1 should go to Page 2"
    );

    let p1_end = Position::new(p1, 14, Affinity::Upstream);
    let (_, rect_end) = Cursor::bounds(&ctx(&rt.state()), &pages, p1_end).unwrap();

    let sel_right = Cursor::move_right(&ctx(&rt.state()), &pages, p1_end, rect_end.y)
        .expect("Should move right to next page");

    assert_eq!(
        sel_right.head.node_id, p2,
        "Right from Page 1 end should go to Page 2"
    );
    assert_eq!(sel_right.head.offset, 0, "Should be at start of Page 2");
}

#[test]
fn test_table_down_from_tall_cell_goes_to_next_row() {
    let mut n1 = id!();
    let mut target = id!();
    let rt = runtime! {
        viewport { paginated { width: 400.0, height: 600.0, margin: PAGE_MARGIN } }
        doc {
            paragraph {}
            table {
                table_row {
                    table_cell {
                        paragraph {}
                    }
                    table_cell {
                        bullet_list {
                            list_item {
                                paragraph {
                                    text { "ㅁㄴ" }
                                }
                            }
                            list_item {
                                paragraph {
                                    text { "ㅁㄴ" }
                                }
                            }
                            list_item {
                                @n1 paragraph {
                                    text { "ㅁㄴ" }
                                }
                            }
                        }
                    }
                    table_cell {
                        paragraph {
                            text { "ㅁㄴ" }
                        }
                        paragraph {
                            text { "ㅁㄴ" }
                        }
                    }
                }
                table_row {
                    table_cell {
                        paragraph {}
                    }
                    table_cell {
                        @target paragraph {}
                    }
                    table_cell {
                        paragraph {}
                    }
                }
            }
            paragraph {}
        }
        selection { (n1, 1) }
    };

    let pages = rt.pages();
    let start_pos = Position::new(n1, 1, Affinity::Downstream);
    let (_, rect) = Cursor::bounds(&ctx(&rt.state()), &pages, start_pos).unwrap();

    let sel_down = Cursor::move_down(&ctx(&rt.state()), &pages, start_pos, rect.x).unwrap();

    assert_eq!(sel_down.head.node_id, target);
}

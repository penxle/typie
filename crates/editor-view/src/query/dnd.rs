use editor_model::{Doc, NodeId};
use editor_state::{Position, Selection};

use crate::page::LayoutPage;
use crate::paginate::*;
use crate::{DropIndicator, DropTarget};

use super::hit_test::HitTester;

pub(crate) fn drop_target_at(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    doc: &Doc,
    page_idx: usize,
    x: f32,
    page_y: f32,
    source: Option<&Selection>,
) -> Option<DropTarget> {
    let page = pages.get(page_idx)?;
    let hit = HitTester::for_page(tree, page, x, page_y);
    let position = dnd_position(&hit, doc, source)?;
    let indicator = drop_indicator_from_position(tree, pages, doc, position)?;
    Some(DropTarget {
        position,
        indicator,
    })
}

fn dnd_position(hit: &HitTester<'_>, doc: &Doc, source: Option<&Selection>) -> Option<Position> {
    let position = if let Some(position) = hit.block_gap_position(doc) {
        promote_block_container_edge_position(doc, position).unwrap_or(position)
    } else {
        let target_x = hit.target_x();
        hit.exact_target()
            .or_else(|| hit.closest_target())
            .map(|target| target.selection(target_x).head)?
    };

    if let Some(source) = source
        && position_inside_selection(doc, position, source)
    {
        return None;
    }

    Some(position)
}

/// Convert an edge position inside a block container to the equivalent parent
/// boundary. This is not root-specific: leading/trailing padding inside a
/// blockquote, callout, list item, fold, or table means "before/after that
/// container" from the parent's point of view.
///
/// Structural containers such as fold_content and table_cell keep their own
/// boundary because they are confinement scopes rather than independently
/// movable siblings.
fn promote_block_container_edge_position(doc: &Doc, position: Position) -> Option<Position> {
    if position.node_id == NodeId::ROOT {
        return None;
    }

    let node = doc.node(position.node_id)?;
    if node.spec().structural {
        return None;
    }

    let child_count = node.children().count();
    if child_count == 0 {
        return None;
    }

    let parent = node.parent()?;
    let parent_offset = node.index()?;
    if position.offset == 0 {
        Some(Position::new(parent.id(), parent_offset))
    } else if position.offset >= child_count {
        Some(Position::new(parent.id(), parent_offset + 1))
    } else {
        None
    }
}

fn position_inside_selection(doc: &Doc, position: Position, selection: &Selection) -> bool {
    let Some(resolved_selection) = selection.resolve(doc) else {
        return false;
    };
    let Some(resolved_position) = position.resolve(doc) else {
        return false;
    };
    resolved_selection.contains(&resolved_position)
}

fn drop_indicator_from_position(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    doc: &Doc,
    position: Position,
) -> Option<DropIndicator> {
    let resolved = position.resolve(doc)?;
    if resolved.is_inline_position() {
        let metrics = super::cursor::cursor_metrics(tree, pages, &position, None)?;
        return Some(DropIndicator::Inline {
            page_idx: metrics.page_idx,
            x: metrics.caret.x,
            y: metrics.caret.y,
            height: metrics.caret.height,
        });
    }

    block_drop_indicator(tree, pages, position)
}

fn block_drop_indicator(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    position: Position,
) -> Option<DropIndicator> {
    let node = super::search::find_box_by_node_id(&tree.root, position.node_id)?;
    let LayoutContent::Box(b) = &node.content else {
        return None;
    };
    let children: Vec<_> = b
        .children
        .iter()
        .filter(|child| !matches!(child.content, LayoutContent::Spacing(_)))
        .collect();
    let (x, width) = children
        .first()
        .map(|child| (child.rect.x, child.rect.width))
        .unwrap_or((node.rect.x, node.rect.width));
    let y_abs = match (position.offset, children.get(position.offset)) {
        (0, Some(first)) => first.rect.y,
        (0, None) => node.rect.y,
        (offset, Some(next)) => {
            let prev = children.get(offset.saturating_sub(1))?;
            let next_page_idx = page_idx_for_y(pages, next.rect.y)?;
            let prev_page_idx = page_idx_for_y(pages, prev.rect.bottom())?;
            if prev_page_idx == next_page_idx {
                (prev.rect.bottom() + next.rect.y) * 0.5
            } else {
                next.rect.y
            }
        }
        (offset, None) => children
            .get(offset.saturating_sub(1))
            .map(|prev| prev.rect.bottom())
            .unwrap_or(node.rect.y),
    };
    let page_idx = page_idx_for_y(pages, y_abs)?;
    let page = &pages[page_idx];
    Some(DropIndicator::Block {
        page_idx,
        x,
        y: y_abs - page.y_start,
        width,
    })
}

fn page_idx_for_y(pages: &[LayoutPage], y_abs: f32) -> Option<usize> {
    pages
        .iter()
        .position(|page| y_abs >= page.y_start && y_abs <= page.y_end)
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_macros::doc;
    use editor_model::NodeId;
    use editor_state::Affinity;

    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::*;
    use crate::view::View;

    use super::*;

    fn make_line_node(id: NodeId, x: f32, y: f32, text: &str, char_w: f32) -> LayoutNode {
        let n = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(x, y, n as f32 * char_w, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(
                    id,
                    0,
                    text,
                    0.0,
                    vec![
                        GraphemeSpan {
                            advance: char_w,
                            codepoints: 1
                        };
                        n
                    ],
                )],
                ruby_annotations: vec![],
                text_indent: 0.0,
                child_range: None,
            }),
        }
    }

    fn make_box_node(
        id: NodeId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        make_box_node_with_style(
            id,
            Rect::from_xywh(x, y, w, h),
            Direction::Vertical,
            false,
            children,
        )
    }

    fn make_box_node_with_style(
        id: NodeId,
        rect: Rect,
        direction: Direction,
        scope: bool,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect,
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style: BoxStyle {
                    direction,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope,
                    decorations: vec![],
                    monolithic: false,
                },
                table_info: None,
                children,
                nav: None,
            }),
        }
    }

    fn make_page(y_start: f32, y_end: f32) -> LayoutPage {
        LayoutPage {
            y_start,
            y_end,
            size: editor_common::Size::new(440.0, y_end - y_start),
        }
    }

    #[test]
    fn dnd_hit_test_line_returns_inline_indicator() {
        let (doc, t) = doc! {
            root { paragraph { t: text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let caret = view
            .cursor_metrics(&doc, &Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        let target = view
            .drop_target_at(&doc, 0, caret.x, caret.y + caret.height * 0.5, None)
            .expect("valid dnd target");

        assert_eq!(target.position.node_id, t);
        assert_eq!(target.position.offset, 2);
        match target.indicator {
            crate::DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
            } => {
                assert_eq!(page_idx, 0);
                assert_eq!(x, caret.x);
                assert_eq!(y, caret.y);
                assert_eq!(height, caret.height);
            }
            other => panic!("expected inline indicator, got {other:?}"),
        }
    }

    #[test]
    fn dnd_hit_test_page_top_margin_returns_root_start_block_position() {
        let (doc,) = doc! {
            root { paragraph { text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let target = view
            .drop_target_at(&doc, 0, 40.0, 0.0, None)
            .expect("page top margin should be a root start drop target");

        assert_eq!(target.position, Position::new(NodeId::ROOT, 0));
        match target.indicator {
            crate::DropIndicator::Block { page_idx, y, .. } => {
                assert_eq!(page_idx, 0);
                assert_eq!(y, 20.0);
            }
            other => panic!("expected block indicator, got {other:?}"),
        }
    }

    #[test]
    fn dnd_hit_test_page_bottom_margin_returns_root_end_block_position() {
        let (doc,) = doc! {
            root { paragraph { text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let page_bottom = view.pages()[0].size.height;

        let target = view
            .drop_target_at(&doc, 0, 40.0, page_bottom, None)
            .expect("page bottom margin should be a root end drop target");

        assert_eq!(target.position, Position::new(NodeId::ROOT, 1));
        assert!(matches!(
            target.indicator,
            crate::DropIndicator::Block { page_idx: 0, .. }
        ));
    }

    #[test]
    fn dnd_hit_test_in_root_block_gap_returns_block_position() {
        let (doc, p1, p2) = doc! {
            root {
                p1: paragraph {}
                p2: paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                80.0,
                vec![
                    make_box_node(
                        p1,
                        20.0,
                        0.0,
                        180.0,
                        20.0,
                        vec![make_line_node(p1, 20.0, 0.0, "", 10.0)],
                    ),
                    LayoutNode {
                        rect: Rect::from_xywh(20.0, 20.0, 180.0, 40.0),
                        content: LayoutContent::Spacing(SpacingKind::Gap),
                    },
                    make_box_node(
                        p2,
                        20.0,
                        60.0,
                        180.0,
                        20.0,
                        vec![make_line_node(p2, 20.0, 60.0, "", 10.0)],
                    ),
                ],
            ),
        };
        let page = make_page(0.0, 100.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 30.0, 40.0, None)
            .expect("gap between root blocks must be a drop target");

        assert_eq!(
            target.position,
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Downstream,
            }
        );
        assert_eq!(
            target.indicator,
            crate::DropIndicator::Block {
                page_idx: 0,
                x: 20.0,
                y: 40.0,
                width: 180.0,
            }
        );
    }

    #[test]
    fn dnd_hit_test_in_nested_block_gap_returns_nested_block_position() {
        let (doc, fc, p1, p2) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fc: fold_content {
                        p1: paragraph {}
                        p2: paragraph {}
                    }
                }
            }
        };
        let fold = doc.node(fc).unwrap().parent().unwrap().id();
        let title = doc.node(fold).unwrap().first_child().unwrap().id();
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                100.0,
                vec![make_box_node(
                    fold,
                    0.0,
                    0.0,
                    220.0,
                    100.0,
                    vec![
                        make_box_node(title, 20.0, 0.0, 180.0, 20.0, vec![]),
                        make_box_node(
                            fc,
                            20.0,
                            20.0,
                            180.0,
                            80.0,
                            vec![
                                make_box_node(p1, 20.0, 20.0, 180.0, 20.0, vec![]),
                                LayoutNode {
                                    rect: Rect::from_xywh(20.0, 40.0, 180.0, 40.0),
                                    content: LayoutContent::Spacing(SpacingKind::Gap),
                                },
                                make_box_node(p2, 20.0, 80.0, 180.0, 20.0, vec![]),
                            ],
                        ),
                    ],
                )],
            ),
        };
        let page = make_page(0.0, 120.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 30.0, 60.0, None)
            .expect("gap inside fold_content must be a DnD target");

        assert_eq!(target.position, Position::new(fc, 1));
    }

    #[test]
    fn dnd_hit_test_in_table_cell_block_gap_stays_inside_cell_scope() {
        let (doc, table, row, cell, p1, p2) = doc! {
            root {
                table: table {
                    row: table_row {
                        cell: table_cell {
                            p1: paragraph {}
                            p2: paragraph {}
                        }
                    }
                }
                paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                240.0,
                140.0,
                vec![
                    make_box_node(
                        table,
                        0.0,
                        0.0,
                        220.0,
                        100.0,
                        vec![make_box_node_with_style(
                            row,
                            Rect::from_xywh(0.0, 0.0, 220.0, 100.0),
                            Direction::Horizontal,
                            false,
                            vec![make_box_node_with_style(
                                cell,
                                Rect::from_xywh(10.0, 10.0, 200.0, 80.0),
                                Direction::Vertical,
                                true,
                                vec![
                                    make_box_node(p1, 20.0, 20.0, 180.0, 20.0, vec![]),
                                    LayoutNode {
                                        rect: Rect::from_xywh(20.0, 40.0, 180.0, 30.0),
                                        content: LayoutContent::Spacing(SpacingKind::Gap),
                                    },
                                    make_box_node(p2, 20.0, 70.0, 180.0, 20.0, vec![]),
                                ],
                            )],
                        )],
                    ),
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 100.0, 0.0, 20.0),
                        content: LayoutContent::Spacing(SpacingKind::Gap),
                    },
                    make_box_node(NodeId::new(), 0.0, 120.0, 220.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 160.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 30.0, 55.0, None)
            .expect("gap inside table_cell scope must be a DnD target");

        assert_eq!(target.position, Position::new(cell, 1));
    }

    #[test]
    fn dnd_hit_test_in_table_cell_leading_padding_stays_inside_cell_scope() {
        let (doc, table, row, cell, p1) = doc! {
            root {
                table: table {
                    row: table_row {
                        cell: table_cell {
                            p1: paragraph {}
                        }
                    }
                }
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                80.0,
                vec![make_box_node(
                    table,
                    0.0,
                    0.0,
                    220.0,
                    80.0,
                    vec![make_box_node_with_style(
                        row,
                        Rect::from_xywh(0.0, 0.0, 220.0, 80.0),
                        Direction::Horizontal,
                        false,
                        vec![make_box_node_with_style(
                            cell,
                            Rect::from_xywh(10.0, 10.0, 200.0, 60.0),
                            Direction::Vertical,
                            true,
                            vec![make_box_node(p1, 20.0, 40.0, 180.0, 20.0, vec![])],
                        )],
                    )],
                )],
            ),
        };
        let page = make_page(0.0, 100.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 30.0, 20.0, None)
            .expect("table-cell leading padding should be a scoped cell drop target");

        assert_eq!(target.position, Position::new(cell, 0));
    }

    #[test]
    fn dnd_hit_test_in_table_row_side_margin_uses_nearest_cell_gap() {
        let (doc, table, row, cell, p1, p2, below_p) = doc! {
            root {
                table: table {
                    row: table_row {
                        cell: table_cell {
                            p1: paragraph {}
                            p2: paragraph {}
                        }
                    }
                }
                below_p: paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                300.0,
                140.0,
                vec![
                    make_box_node(
                        table,
                        0.0,
                        0.0,
                        100.0,
                        100.0,
                        vec![make_box_node_with_style(
                            row,
                            Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                            Direction::Horizontal,
                            false,
                            vec![make_box_node_with_style(
                                cell,
                                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                                Direction::Vertical,
                                true,
                                vec![
                                    make_box_node(p1, 10.0, 10.0, 80.0, 20.0, vec![]),
                                    LayoutNode {
                                        rect: Rect::from_xywh(10.0, 30.0, 80.0, 40.0),
                                        content: LayoutContent::Spacing(SpacingKind::Gap),
                                    },
                                    make_box_node(p2, 10.0, 70.0, 80.0, 20.0, vec![]),
                                ],
                            )],
                        )],
                    ),
                    make_line_node(below_p, 0.0, 110.0, "below", 60.0),
                ],
            ),
        };
        let page = make_page(0.0, 160.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 180.0, 50.0, None)
            .expect("same-row side margin should use the nearest table-cell gap");

        assert_eq!(target.position, Position::new(cell, 1));
    }

    #[test]
    fn dnd_hit_test_in_root_child_leading_padding_returns_root_boundary() {
        let (doc, before, bq, inner, after) = doc! {
            root {
                before: paragraph {}
                bq: blockquote { inner: paragraph {} }
                after: paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                140.0,
                vec![
                    make_box_node(before, 20.0, 0.0, 180.0, 20.0, vec![]),
                    make_box_node(
                        bq,
                        20.0,
                        40.0,
                        180.0,
                        60.0,
                        vec![make_box_node(inner, 40.0, 60.0, 140.0, 20.0, vec![])],
                    ),
                    make_box_node(after, 20.0, 120.0, 180.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 160.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 50.0, 50.0, None)
            .expect("leading padding before a root child container should be a root boundary");

        assert_eq!(target.position, Position::new(NodeId::ROOT, 1));
    }

    #[test]
    fn dnd_hit_test_in_root_child_trailing_padding_returns_root_boundary() {
        let (doc, before, bq, inner, after) = doc! {
            root {
                before: paragraph {}
                bq: blockquote { inner: paragraph {} }
                after: paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                140.0,
                vec![
                    make_box_node(before, 20.0, 0.0, 180.0, 20.0, vec![]),
                    make_box_node(
                        bq,
                        20.0,
                        40.0,
                        180.0,
                        60.0,
                        vec![make_box_node(inner, 40.0, 60.0, 140.0, 20.0, vec![])],
                    ),
                    make_box_node(after, 20.0, 120.0, 180.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 160.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 50.0, 90.0, None)
            .expect("trailing padding after a root child container should be a root boundary");

        assert_eq!(target.position, Position::new(NodeId::ROOT, 2));
    }

    #[test]
    fn dnd_hit_test_in_nested_container_edge_padding_returns_parent_boundary() {
        let (doc, fc, before, bq, inner, after) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fc: fold_content {
                        before: paragraph {}
                        bq: blockquote { inner: paragraph {} }
                        after: paragraph {}
                    }
                }
            }
        };
        let fold = doc.node(fc).unwrap().parent().unwrap().id();
        let title = doc.node(fold).unwrap().first_child().unwrap().id();
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                240.0,
                160.0,
                vec![make_box_node(
                    fold,
                    0.0,
                    0.0,
                    240.0,
                    160.0,
                    vec![
                        make_box_node(
                            title,
                            40.0,
                            0.0,
                            160.0,
                            20.0,
                            vec![make_line_node(title, 40.0, 0.0, "title", 10.0)],
                        ),
                        make_box_node(
                            fc,
                            20.0,
                            20.0,
                            200.0,
                            140.0,
                            vec![
                                make_box_node(before, 40.0, 40.0, 160.0, 20.0, vec![]),
                                make_box_node(
                                    bq,
                                    40.0,
                                    80.0,
                                    160.0,
                                    60.0,
                                    vec![make_box_node(inner, 60.0, 100.0, 120.0, 20.0, vec![])],
                                ),
                                make_box_node(after, 40.0, 140.0, 160.0, 20.0, vec![]),
                            ],
                        ),
                    ],
                )],
            ),
        };
        let pages = [make_page(0.0, 180.0)];

        let leading = drop_target_at(&tree, &pages, &doc, 0, 70.0, 90.0, None)
            .expect("leading padding before nested container should target parent boundary");
        let trailing = drop_target_at(&tree, &pages, &doc, 0, 70.0, 130.0, None)
            .expect("trailing padding after nested container should target parent boundary");

        assert_eq!(leading.position, Position::new(fc, 1));
        assert_eq!(trailing.position, Position::new(fc, 2));
    }

    #[test]
    fn dnd_hit_test_in_structural_container_edge_padding_keeps_structural_boundary() {
        let (doc, fc, p1) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fc: fold_content { p1: paragraph {} }
                }
            }
        };
        let fold = doc.node(fc).unwrap().parent().unwrap().id();
        let title = doc.node(fold).unwrap().first_child().unwrap().id();
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                240.0,
                100.0,
                vec![make_box_node(
                    fold,
                    0.0,
                    0.0,
                    240.0,
                    100.0,
                    vec![
                        make_box_node(
                            title,
                            40.0,
                            0.0,
                            160.0,
                            20.0,
                            vec![make_line_node(title, 40.0, 0.0, "title", 10.0)],
                        ),
                        make_box_node(
                            fc,
                            20.0,
                            20.0,
                            200.0,
                            80.0,
                            vec![make_box_node(p1, 40.0, 60.0, 160.0, 20.0, vec![])],
                        ),
                    ],
                )],
            ),
        };
        let page = make_page(0.0, 120.0);

        let target = drop_target_at(&tree, &[page], &doc, 0, 70.0, 40.0, None)
            .expect("fold_content leading padding should stay inside fold_content");

        assert_eq!(target.position, Position::new(fc, 0));
    }

    #[test]
    fn block_drop_indicator_between_siblings_uses_gap_midpoint_and_child_width() {
        let p1 = NodeId::new();
        let p2 = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                80.0,
                vec![
                    make_box_node(p1, 20.0, 0.0, 180.0, 20.0, vec![]),
                    LayoutNode {
                        rect: Rect::from_xywh(20.0, 20.0, 180.0, 40.0),
                        content: LayoutContent::Spacing(SpacingKind::Gap),
                    },
                    make_box_node(p2, 20.0, 60.0, 180.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 100.0);

        let indicator = block_drop_indicator(
            &tree,
            &[page],
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Downstream,
            },
        )
        .expect("block indicator");

        assert_eq!(
            indicator,
            crate::DropIndicator::Block {
                page_idx: 0,
                x: 20.0,
                y: 40.0,
                width: 180.0,
            }
        );
    }

    #[test]
    fn dnd_hit_test_rejects_internal_target_inside_source_selection() {
        let (doc, t) = doc! {
            root { paragraph { t: text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let caret = view
            .cursor_metrics(&doc, &Position::new(t, 2))
            .expect("cursor metrics")
            .caret;
        let source = Selection::new(Position::new(t, 1), Position::new(t, 4));

        let target = view.drop_target_at(
            &doc,
            0,
            caret.x,
            caret.y + caret.height * 0.5,
            Some(&source),
        );

        assert!(target.is_none());
    }
}

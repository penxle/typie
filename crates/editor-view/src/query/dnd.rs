use editor_common::Rect;
use editor_model::{Doc, Node, NodeId};
use editor_state::Position;

use crate::paginate::*;
use crate::style::Direction;
use crate::{DropIndicator, DropTarget};

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

pub(crate) fn drop_target_at(
    layout_index: &LayoutIndex,
    doc: &Doc,
    page_idx: usize,
    x: f32,
    page_y: f32,
) -> Option<DropTarget> {
    let point = layout_index.point(page_idx, x, page_y)?;
    let position = dnd_position(layout_index, doc, point)?;
    let position = promote_outer_edge_drop_position(doc, position).unwrap_or(position);
    let indicator = drop_indicator_from_position(layout_index, doc, position)?;
    Some(DropTarget {
        position,
        indicator,
    })
}

fn dnd_position(layout_index: &LayoutIndex, doc: &Doc, point: LayoutPoint) -> Option<Position> {
    let position = layout_index
        .exact_entry_with(point, |entry, node| {
            dnd_position_for_candidate(layout_index, doc, entry, node, point)
        })
        .or_else(|| {
            layout_index.closest_entry_with(point, |entry, node| {
                dnd_position_for_candidate(layout_index, doc, entry, node, point)
            })
        })
        .map(|(_, position)| position)?;

    position.resolve(doc).is_some().then_some(position)
}

fn is_dnd_entry(_entry: &LayoutEntry, node: &LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_)
            | LayoutContent::Atom(_)
            | LayoutContent::Box(_)
            | LayoutContent::Spacing(SpacingKind::Gap { .. })
    )
}

fn dnd_position_for_candidate(
    layout_index: &LayoutIndex,
    doc: &Doc,
    entry: &LayoutEntry,
    node: &LayoutNode,
    point: LayoutPoint,
) -> Option<Position> {
    is_dnd_entry(entry, node)
        .then(|| dnd_position_for_entry(layout_index, doc, entry, point))
        .flatten()
}

fn dnd_position_for_entry(
    layout_index: &LayoutIndex,
    doc: &Doc,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Position> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => Some(position_in_line(line, &entry.rect, point.x)),
        LayoutContent::Atom(atom) => Some(Position::new(
            atom.attachment.parent_id,
            atom.attachment.index + 1,
        )),
        LayoutContent::Box(b) => box_edge_position(layout_index, doc, b, point),
        LayoutContent::Spacing(SpacingKind::Gap { position }) => Some(*position),
        LayoutContent::Spacing(SpacingKind::Fill) => None,
    }
}

fn position_in_line(line: &LayoutLine, rect: &Rect, x: f32) -> Position {
    super::grapheme::position_at_x(line, x - rect.x)
}

#[derive(Debug, Clone, Copy)]
struct DropChild {
    offset: usize,
    rect: Rect,
}

fn box_edge_position(
    layout_index: &LayoutIndex,
    doc: &Doc,
    b: &LayoutBox,
    point: LayoutPoint,
) -> Option<Position> {
    if b.style.direction != Direction::Vertical {
        return None;
    }

    let page = layout_index.page(point.page_idx)?;
    if point.y < page.content_y_start {
        let first = drop_children_in_y_range(
            layout_index,
            doc,
            b.node_id,
            page.content_y_start,
            page.content_y_end,
        )
        .into_iter()
        .next()?;
        return Some(Position::new(b.node_id, first.offset));
    }
    if point.y > page.content_y_end {
        let last = drop_children_in_y_range(
            layout_index,
            doc,
            b.node_id,
            page.content_y_start,
            page.content_y_end,
        )
        .into_iter()
        .last()?;
        return Some(Position::new(b.node_id, last.offset + 1));
    }

    let children = drop_children(layout_index, doc, b.node_id);
    let first = children.first()?;
    if point.y < first.rect.y {
        return Some(Position::new(b.node_id, first.offset));
    }

    let last = children.last().expect("children is not empty");
    if point.y > last.rect.bottom() {
        return Some(Position::new(b.node_id, last.offset + 1));
    }

    None
}

fn drop_children(layout_index: &LayoutIndex, doc: &Doc, parent_id: NodeId) -> Vec<DropChild> {
    layout_index
        .direct_child_entries(parent_id)
        .filter_map(|entry| drop_child(layout_index, doc, parent_id, entry))
        .collect()
}

fn drop_children_in_y_range(
    layout_index: &LayoutIndex,
    doc: &Doc,
    parent_id: NodeId,
    y_start: f32,
    y_end: f32,
) -> Vec<DropChild> {
    layout_index
        .direct_child_entries_in_y_range(parent_id, y_start, y_end)
        .filter_map(|entry| drop_child(layout_index, doc, parent_id, entry))
        .collect()
}

fn drop_child(
    layout_index: &LayoutIndex,
    doc: &Doc,
    parent_id: NodeId,
    entry: &LayoutEntry,
) -> Option<DropChild> {
    match entry.content(layout_index)? {
        LayoutContent::Box(b) => {
            let child_ref = doc.node(b.node_id)?;
            (child_ref.parent()?.id() == parent_id).then(|| DropChild {
                offset: child_ref.index().unwrap_or(0),
                rect: entry.rect,
            })
        }
        LayoutContent::Atom(atom) => {
            (atom.attachment.parent_id == parent_id).then_some(DropChild {
                offset: atom.attachment.index,
                rect: entry.rect,
            })
        }
        LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
    }
}

fn promote_outer_edge_drop_position(doc: &Doc, position: Position) -> Option<Position> {
    if position.node_id == NodeId::ROOT {
        return None;
    }

    let node = doc.node(position.node_id)?;
    if !promotes_edge_drop_to_parent(node.node()) {
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

fn promotes_edge_drop_to_parent(node: &Node) -> bool {
    matches!(node, Node::Fold(_) | Node::Table(_) | Node::ListItem(_))
}

fn drop_indicator_from_position(
    layout_index: &LayoutIndex,
    doc: &Doc,
    position: Position,
) -> Option<DropIndicator> {
    let resolved = position.resolve(doc)?;
    if resolved.is_inline_position() {
        let metrics = super::cursor::cursor_metrics(layout_index, &position, None)?;
        return Some(DropIndicator::Inline {
            page_idx: metrics.page_idx,
            x: metrics.caret.x,
            y: metrics.caret.y,
            height: metrics.caret.height,
        });
    }

    block_drop_indicator(layout_index, position)
}

fn block_drop_indicator(layout_index: &LayoutIndex, position: Position) -> Option<DropIndicator> {
    let node_rect = layout_index.box_rect(position.node_id)?;
    let children: Vec<_> = layout_index
        .direct_child_entries(position.node_id)
        .filter(|entry| !matches!(entry.content(layout_index), Some(LayoutContent::Spacing(_))))
        .collect();
    let (x, width) = children
        .first()
        .map(|child| (child.rect.x, child.rect.width))
        .unwrap_or((node_rect.x, node_rect.width));
    let y_abs = match (position.offset, children.get(position.offset)) {
        (0, Some(first)) => first.rect.y,
        (0, None) => node_rect.y,
        (offset, Some(next)) => {
            let prev = children.get(offset.saturating_sub(1))?;
            let next_page_idx = layout_index.page_idx_for_y(next.rect.y)?;
            let prev_page_idx = layout_index.page_idx_for_y(prev.rect.bottom())?;
            if prev_page_idx == next_page_idx {
                (prev.rect.bottom() + next.rect.y) * 0.5
            } else {
                next.rect.y
            }
        }
        (offset, None) => children
            .get(offset.saturating_sub(1))
            .map(|prev| prev.rect.bottom())
            .unwrap_or(node_rect.y),
    };
    let page_idx = layout_index.page_idx_for_y(y_abs)?;
    let page_y_start = layout_index.page_y_start(page_idx)?;
    Some(DropIndicator::Block {
        page_idx,
        x,
        y: y_abs - page_y_start,
        width,
    })
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect};
    use editor_macros::doc;
    use editor_model::NodeId;
    use editor_state::Affinity;

    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::page::LayoutPage;
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
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
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
            children,
        )
    }

    fn make_box_node_with_style(
        id: NodeId,
        rect: Rect,
        direction: Direction,
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
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                attachment: None,
            }),
        }
    }

    fn make_page(y_start: f32, y_end: f32) -> LayoutPage {
        LayoutPage::new(
            y_start,
            y_end,
            editor_common::Size::new(440.0, y_end - y_start),
        )
    }

    fn drop_target_in_tree(
        tree: &LayoutTree,
        pages: &[LayoutPage],
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<DropTarget> {
        let layout_index = LayoutIndex::new(tree.clone(), pages);
        drop_target_at(&layout_index, doc, page_idx, x, y)
    }

    #[test]
    fn dnd_hit_test_line_returns_inline_indicator() {
        let (doc, t) = doc! {
            root { paragraph { t: text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = editor_state::State::new(doc, editor_crdt::OpGraph::new(), None);
        let caret = view
            .cursor_metrics(&state, &Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        let target = view
            .drop_target_at(&state.doc, 0, caret.x, caret.y + caret.height * 0.5)
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
            .drop_target_at(&doc, 0, 40.0, 0.0)
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
    fn dnd_hit_test_second_page_top_margin_returns_first_block_position_on_page() {
        let (doc, p1, t1, p2, t2) = doc! {
            root {
                p1: paragraph { t1: text("one") }
                p2: paragraph { t2: text("two") }
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                20.0,
                20.0,
                180.0,
                140.0,
                vec![
                    make_box_node(
                        p1,
                        20.0,
                        20.0,
                        180.0,
                        40.0,
                        vec![make_line_node(t1, 20.0, 20.0, "one", 10.0)],
                    ),
                    make_box_node(
                        p2,
                        20.0,
                        120.0,
                        180.0,
                        40.0,
                        vec![make_line_node(t2, 20.0, 120.0, "two", 10.0)],
                    ),
                ],
            ),
        };
        let pages = [
            LayoutPage::with_content(
                0.0,
                100.0,
                20.0,
                80.0,
                editor_common::Size::new(240.0, 100.0),
            ),
            LayoutPage::with_content(
                100.0,
                200.0,
                120.0,
                180.0,
                editor_common::Size::new(240.0, 100.0),
            ),
        ];

        let target = drop_target_in_tree(&tree, &pages, &doc, 1, 40.0, 0.0)
            .expect("second page top margin should be a page-local block boundary");

        assert_eq!(target.position, Position::new(NodeId::ROOT, 1));
        match target.indicator {
            crate::DropIndicator::Block { page_idx, y, .. } => {
                assert_eq!(page_idx, 1);
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
            .drop_target_at(&doc, 0, 40.0, page_bottom)
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
                        content: LayoutContent::Spacing(SpacingKind::Gap {
                            position: Position::new(NodeId::ROOT, 1),
                        }),
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

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 30.0, 40.0)
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
                                    content: LayoutContent::Spacing(SpacingKind::Gap {
                                        position: Position::new(fc, 1),
                                    }),
                                },
                                make_box_node(p2, 20.0, 80.0, 180.0, 20.0, vec![]),
                            ],
                        ),
                    ],
                )],
            ),
        };
        let page = make_page(0.0, 120.0);

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 30.0, 60.0)
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
                            vec![make_box_node_with_style(
                                cell,
                                Rect::from_xywh(10.0, 10.0, 200.0, 80.0),
                                Direction::Vertical,
                                vec![
                                    make_box_node(p1, 20.0, 20.0, 180.0, 20.0, vec![]),
                                    LayoutNode {
                                        rect: Rect::from_xywh(20.0, 40.0, 180.0, 30.0),
                                        content: LayoutContent::Spacing(SpacingKind::Gap {
                                            position: Position::new(cell, 1),
                                        }),
                                    },
                                    make_box_node(p2, 20.0, 70.0, 180.0, 20.0, vec![]),
                                ],
                            )],
                        )],
                    ),
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 100.0, 0.0, 20.0),
                        content: LayoutContent::Spacing(SpacingKind::Gap {
                            position: Position::new(NodeId::ROOT, 1),
                        }),
                    },
                    make_box_node(NodeId::new(), 0.0, 120.0, 220.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 160.0);

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 30.0, 55.0)
            .expect("gap inside table_cell must be a DnD target");

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
                        vec![make_box_node_with_style(
                            cell,
                            Rect::from_xywh(10.0, 10.0, 200.0, 60.0),
                            Direction::Vertical,
                            vec![make_box_node(p1, 20.0, 40.0, 180.0, 20.0, vec![])],
                        )],
                    )],
                )],
            ),
        };
        let page = make_page(0.0, 100.0);

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 30.0, 20.0)
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
                            vec![make_box_node_with_style(
                                cell,
                                Rect::from_xywh(0.0, 0.0, 100.0, 100.0),
                                Direction::Vertical,
                                vec![
                                    make_box_node(p1, 10.0, 10.0, 80.0, 20.0, vec![]),
                                    LayoutNode {
                                        rect: Rect::from_xywh(10.0, 30.0, 80.0, 40.0),
                                        content: LayoutContent::Spacing(SpacingKind::Gap {
                                            position: Position::new(cell, 1),
                                        }),
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

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 180.0, 50.0)
            .expect("same-row side margin should use the nearest table-cell gap");

        assert_eq!(target.position, Position::new(cell, 1));
    }

    #[test]
    fn dnd_hit_test_in_blockquote_leading_padding_stays_inside_blockquote() {
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

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 50.0, 50.0)
            .expect("blockquote leading padding should be an internal drop target");

        assert_eq!(target.position, Position::new(bq, 0));
    }

    #[test]
    fn dnd_hit_test_in_blockquote_trailing_padding_stays_inside_blockquote() {
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

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 50.0, 90.0)
            .expect("blockquote trailing padding should be an internal drop target");

        assert_eq!(target.position, Position::new(bq, 1));
    }

    #[test]
    fn dnd_hit_test_in_callout_edge_padding_stays_inside_callout() {
        let (doc, before, callout, inner, after) = doc! {
            root {
                before: paragraph {}
                callout: callout { inner: paragraph {} }
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
                        callout,
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
        let pages = [make_page(0.0, 160.0)];

        let leading = drop_target_in_tree(&tree, &pages, &doc, 0, 50.0, 50.0)
            .expect("callout leading padding should be an internal drop target");
        let trailing = drop_target_in_tree(&tree, &pages, &doc, 0, 50.0, 90.0)
            .expect("callout trailing padding should be an internal drop target");

        assert_eq!(leading.position, Position::new(callout, 0));
        assert_eq!(trailing.position, Position::new(callout, 1));
    }

    #[test]
    fn dnd_closest_fallback_includes_box_spanning_requested_page() {
        let (doc, before, callout, inner, after) = doc! {
            root {
                before: paragraph {}
                callout: callout { inner: paragraph {} }
                after: paragraph {}
            }
        };
        let tree = LayoutTree {
            root: make_box_node(
                NodeId::ROOT,
                0.0,
                0.0,
                220.0,
                200.0,
                vec![
                    make_box_node(before, 40.0, 0.0, 140.0, 20.0, vec![]),
                    make_box_node(
                        callout,
                        40.0,
                        50.0,
                        140.0,
                        100.0,
                        vec![make_box_node(inner, 60.0, 60.0, 100.0, 20.0, vec![])],
                    ),
                    make_box_node(after, 40.0, 170.0, 140.0, 20.0, vec![]),
                ],
            ),
        };
        let pages = [make_page(0.0, 100.0), make_page(100.0, 200.0)];

        let target = drop_target_in_tree(&tree, &pages, &doc, 1, 20.0, 20.0)
            .expect("closest fallback should include a box that started on the previous page");

        assert_eq!(target.position, Position::new(callout, 1));
    }

    #[test]
    fn dnd_hit_test_in_nested_blockquote_edge_padding_stays_inside_blockquote() {
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

        let leading = drop_target_in_tree(&tree, &pages, &doc, 0, 70.0, 90.0)
            .expect("nested blockquote leading padding should be an internal drop target");
        let trailing = drop_target_in_tree(&tree, &pages, &doc, 0, 70.0, 130.0)
            .expect("nested blockquote trailing padding should be an internal drop target");

        assert_eq!(leading.position, Position::new(bq, 0));
        assert_eq!(trailing.position, Position::new(bq, 1));
    }

    #[test]
    fn dnd_hit_test_in_list_item_leading_padding_returns_list_boundary() {
        let (doc, list, item, p1) = doc! {
            root {
                list: bullet_list {
                    item: list_item {
                        p1: paragraph {}
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
                    list,
                    20.0,
                    0.0,
                    180.0,
                    80.0,
                    vec![make_box_node(
                        item,
                        20.0,
                        0.0,
                        180.0,
                        60.0,
                        vec![make_box_node(p1, 40.0, 30.0, 140.0, 20.0, vec![])],
                    )],
                )],
            ),
        };
        let page = make_page(0.0, 100.0);

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 50.0, 10.0)
            .expect("list item leading padding should target the parent list boundary");

        assert_eq!(target.position, Position::new(list, 0));
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

        let target = drop_target_in_tree(&tree, &[page], &doc, 0, 70.0, 40.0)
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
                        content: LayoutContent::Spacing(SpacingKind::Gap {
                            position: Position::new(NodeId::ROOT, 1),
                        }),
                    },
                    make_box_node(p2, 20.0, 60.0, 180.0, 20.0, vec![]),
                ],
            ),
        };
        let page = make_page(0.0, 100.0);
        let pages = [page];
        let layout_index = LayoutIndex::new(tree.clone(), &pages);

        let indicator = block_drop_indicator(
            &layout_index,
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
    fn dnd_hit_test_returns_candidate_inside_source_selection() {
        let (doc, t) = doc! {
            root { paragraph { t: text("hello") } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = editor_state::State::new(doc, editor_crdt::OpGraph::new(), None);
        let caret = view
            .cursor_metrics(&state, &Position::new(t, 2))
            .expect("cursor metrics")
            .caret;

        let target = view.drop_target_at(&state.doc, 0, caret.x, caret.y + caret.height * 0.5);

        assert!(target.is_some());
    }
}

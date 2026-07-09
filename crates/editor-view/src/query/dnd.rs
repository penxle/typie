use editor_common::Rect;
use editor_crdt::Dot;
use editor_model::{DocView, Node};
use editor_state::Position;

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};
use crate::paginate::types::{LayoutContent, LayoutLine, SpacingKind};
use crate::style::Direction;
use crate::{DropIndicator, DropTarget};

pub(crate) fn drop_target_at(
    layout_index: &LayoutIndex,
    view: &DocView,
    page_idx: usize,
    x: f32,
    page_y: f32,
) -> Option<DropTarget> {
    let point = layout_index.point(page_idx, x, page_y)?;
    let position = dnd_position(layout_index, view, point)?;
    let position =
        promote_outer_edge_drop_position(layout_index, view, position).unwrap_or(position);
    let indicator = drop_indicator_from_position(layout_index, view, position)?;
    Some(DropTarget {
        position,
        indicator,
    })
}

fn dnd_position(
    layout_index: &LayoutIndex,
    view: &DocView,
    point: LayoutPoint,
) -> Option<Position> {
    let scope = layout_index.container_scope(point);
    let position = if let Some(scope) = scope {
        let in_scope_descendant = |entry: &LayoutEntry| {
            layout_index.entry_is_in_scope(entry, scope) && !std::ptr::eq(entry, scope)
        };
        let in_scope = |entry: &LayoutEntry| layout_index.entry_is_in_scope(entry, scope);
        exact_dnd_position(layout_index, view, point, &in_scope_descendant)
            .or_else(|| exact_dnd_position(layout_index, view, point, &in_scope))
            .or_else(|| closest_dnd_position(layout_index, view, point, &in_scope_descendant))
            .or_else(|| closest_dnd_position(layout_index, view, point, &in_scope))
    } else {
        let all_entries = |_: &LayoutEntry| true;
        exact_dnd_position(layout_index, view, point, &all_entries)
            .or_else(|| closest_dnd_position(layout_index, view, point, &all_entries))
    }?;

    position.resolve(view).is_some().then_some(position)
}

fn exact_dnd_position(
    layout_index: &LayoutIndex,
    view: &DocView,
    point: LayoutPoint,
    include: &impl Fn(&LayoutEntry) -> bool,
) -> Option<Position> {
    layout_index
        .exact_entry_with(point, |entry, node| {
            if !include(entry) {
                return None;
            }
            dnd_position_for_candidate(layout_index, view, entry, node, point)
        })
        .map(|(_, position)| position)
}

fn closest_dnd_position(
    layout_index: &LayoutIndex,
    view: &DocView,
    point: LayoutPoint,
    include: &impl Fn(&LayoutEntry) -> bool,
) -> Option<Position> {
    layout_index
        .closest_entry_with(point, |entry, node| {
            if !include(entry) {
                return None;
            }
            dnd_position_for_candidate(layout_index, view, entry, node, point)
        })
        .map(|(_, position)| position)
}

fn dnd_position_for_candidate(
    layout_index: &LayoutIndex,
    view: &DocView,
    entry: &LayoutEntry,
    node: &crate::paginate::types::LayoutNode,
    point: LayoutPoint,
) -> Option<Position> {
    is_dnd_entry(entry, node)
        .then(|| dnd_position_for_entry(layout_index, view, entry, point))
        .flatten()
}

fn is_dnd_entry(_entry: &LayoutEntry, node: &crate::paginate::types::LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_)
            | LayoutContent::Atom(_)
            | LayoutContent::Box(_)
            | LayoutContent::Spacing(SpacingKind::Gap { .. })
    )
}

fn dnd_position_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Position> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => Some(position_in_line(line, &entry.rect, point.x)),
        LayoutContent::Atom(atom) => Some(Position::new(
            atom.attachment.parent,
            atom.attachment.index + 1,
        )),
        LayoutContent::Box(b) => box_edge_position(layout_index, view, b, point),
        LayoutContent::Spacing(SpacingKind::Gap { position }) => Some(*position),
        LayoutContent::Spacing(SpacingKind::Fill) => None,
    }
}

fn position_in_line(line: &LayoutLine, rect: &Rect, x: f32) -> Position {
    crate::query::grapheme::position_at_x(line, x - rect.x)
}

#[derive(Debug, Clone, Copy)]
struct DropChild {
    offset: usize,
    rect: Rect,
}

fn box_edge_position(
    layout_index: &LayoutIndex,
    view: &DocView,
    b: &crate::paginate::types::LayoutBox,
    point: LayoutPoint,
) -> Option<Position> {
    if b.style.direction != Direction::Vertical {
        return None;
    }

    let page = layout_index.page(point.page_idx)?;
    if point.y < page.content_y_start {
        let first = drop_children_in_y_range(
            layout_index,
            view,
            &b.node,
            page.content_y_start,
            page.content_y_end,
        )
        .into_iter()
        .next()?;
        return Some(Position::new(b.node, first.offset));
    }
    if point.y > page.content_y_end {
        let last = drop_children_in_y_range(
            layout_index,
            view,
            &b.node,
            page.content_y_start,
            page.content_y_end,
        )
        .into_iter()
        .last()?;
        return Some(Position::new(b.node, last.offset + 1));
    }

    let children = drop_children(layout_index, view, &b.node);
    let first = children.first()?;
    if point.y < first.rect.y {
        return Some(Position::new(b.node, first.offset));
    }

    let last = children.last().expect("children is not empty");
    if point.y > last.rect.bottom() {
        return Some(Position::new(b.node, last.offset + 1));
    }

    None
}

fn drop_children(layout_index: &LayoutIndex, view: &DocView, parent: &Dot) -> Vec<DropChild> {
    layout_index
        .direct_child_entries(parent)
        .filter_map(|entry| drop_child(layout_index, view, parent, entry))
        .collect()
}

fn drop_children_in_y_range(
    layout_index: &LayoutIndex,
    view: &DocView,
    parent: &Dot,
    y_start: f32,
    y_end: f32,
) -> Vec<DropChild> {
    layout_index
        .direct_child_entries_in_y_range(parent, y_start, y_end)
        .filter_map(|entry| drop_child(layout_index, view, parent, entry))
        .collect()
}

fn drop_child(
    layout_index: &LayoutIndex,
    view: &DocView,
    parent: &Dot,
    entry: &LayoutEntry,
) -> Option<DropChild> {
    match entry.content(layout_index)? {
        LayoutContent::Box(b) => {
            let child_ref = view.node(b.node)?;
            (child_ref.parent()?.id() == *parent).then(|| DropChild {
                offset: child_ref.index().unwrap_or(0),
                rect: entry.rect,
            })
        }
        LayoutContent::Atom(atom) => (atom.attachment.parent == *parent).then_some(DropChild {
            offset: atom.attachment.index,
            rect: entry.rect,
        }),
        LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
    }
}

fn promote_outer_edge_drop_position(
    _layout_index: &LayoutIndex,
    view: &DocView,
    position: Position,
) -> Option<Position> {
    if view.root().map(|r| r.id()).as_ref() == Some(&position.node) {
        return None;
    }

    let node = view.node(position.node)?;
    let n = node.node();
    if !promotes_edge_drop_to_parent(&n) {
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
    view: &DocView,
    position: Position,
) -> Option<DropIndicator> {
    let resolved = position.resolve(view)?;
    if resolved.is_inline_position() {
        let metrics = crate::query::cursor::cursor_metrics(layout_index, &position, None)?;
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
    let node_rect = layout_index.box_rect(&position.node)?;
    let children: Vec<_> = layout_index
        .direct_child_entries(&position.node)
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
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, Modifier, ModifierAttrLog,
        ModifierAttrOp, NodeAttrLog, NodeType, SeqItem, SpanLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::LayoutContent;
    use crate::query::grapheme;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    fn build_index<'a>(
        _doc: &'a DocLogs,
        width: f32,
        pd: &'a editor_model::ProjectedDoc,
    ) -> (DocView<'a>, LayoutIndex) {
        let view = DocView::new(pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();
        let measured = measure_node(
            &mut crate::measure::Measurer::new(),
            &root_node,
            width,
            &MeasureContext::default(),
            &mut res,
        );
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (view, index)
    }

    fn para_doc_data(text: &str) -> DocLogs {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        logs(&items)
    }

    fn table_with_short_cell_and_wrapped_neighbor() -> DocLogs {
        let root = Dot::ROOT;
        let table = Dot::new(15, 1);
        let row = Dot::new(15, 2);
        let left_cell = Dot::new(15, 3);
        let left_para = Dot::new(15, 4);
        let right_cell = Dot::new(15, 20);
        let right_para = Dot::new(15, 21);

        let mut items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                left_cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                    attrs: vec![],
                },
            ),
            (
                left_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, left_cell],
                    attrs: vec![],
                },
            ),
            (Dot::new(15, 5), SeqItem::Char('x')),
            (
                right_cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                    attrs: vec![],
                },
            ),
            (
                right_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, right_cell],
                    attrs: vec![],
                },
            ),
        ];
        for (i, ch) in "right cell text wraps onto a second visual line"
            .chars()
            .enumerate()
        {
            items.push((Dot::new(15, 22 + i as u64), SeqItem::Char(ch)));
        }

        logs(&items)
    }

    #[test]
    fn drop_in_line_inline_indicator() {
        let doc = para_doc_data("Hello");
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 400.0, &pd);

        let root_node = view.root().unwrap();
        let root_id = root_node.id();
        let para_id = root_node
            .child_blocks()
            .next()
            .expect("para must exist")
            .id();

        let para_rect = index.box_rect(&para_id).expect("para box must exist");
        let mid_y = para_rect.y + para_rect.height * 0.5;

        let entry = index
            .direct_child_entries(&para_id)
            .find(|e| matches!(e.content(&index), Some(LayoutContent::Line(_))))
            .expect("line entry must exist");
        let line_rect = entry.rect;
        let test_x = line_rect.x + line_rect.width * 0.5;

        let result = drop_target_at(&index, &view, 0, test_x, mid_y - para_rect.y);
        let target = result.expect("drop target must be Some");

        assert_eq!(
            target.position.node, para_id,
            "position.node must be the paragraph Dot"
        );
        assert!(
            matches!(target.indicator, DropIndicator::Inline { .. }),
            "indicator must be Inline, got {:?}",
            target.indicator
        );

        let LayoutContent::Line(line) = &entry.content(&index).unwrap() else {
            panic!("must be a line");
        };
        let expected_pos = grapheme::position_at_x(line, test_x - line_rect.x);
        assert_eq!(
            target.position.offset, expected_pos.offset,
            "offset must match position_at_x"
        );

        let _ = root_id;
    }

    #[test]
    fn drop_target_side_gutter_routes_to_nearest_cell_scope_in_row() {
        let doc = table_with_short_cell_and_wrapped_neighbor();
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 180.0, &pd);

        let root = view.root().expect("root must exist");
        let table = root.child_blocks().next().expect("table must exist");
        let row = table.child_blocks().next().expect("row must exist");
        let mut cells = row.child_blocks();
        let left_cell = cells.next().expect("left cell must exist");
        let left_para = left_cell
            .child_blocks()
            .next()
            .expect("left paragraph must exist");
        let right_cell = cells.next().expect("right cell must exist");
        let right_para = right_cell
            .child_blocks()
            .next()
            .expect("right paragraph must exist");

        let left_cell_rect = index
            .box_rect(&left_cell.id())
            .expect("left cell must have a box");
        let right_lines: Vec<_> = index
            .entries()
            .filter_map(|entry| {
                let LayoutContent::Line(line) = entry.content(&index)? else {
                    return None;
                };
                (line.node == right_para.id() && line.offset_range.is_some()).then_some(entry)
            })
            .collect();
        assert!(
            right_lines.len() >= 2,
            "right cell text must wrap to at least two lines"
        );
        let second_right_line = right_lines[1];

        let x = left_cell_rect.x - 0.25;
        let page_y = second_right_line.rect.y - index.pages()[0].y_start
            + second_right_line.rect.height / 2.0;

        let target =
            drop_target_at(&index, &view, 0, x, page_y).expect("gutter drop in row must resolve");

        assert_eq!(
            target.position.node,
            left_para.id(),
            "row side gutter drop must route to the nearest cell scope"
        );
    }

    #[test]
    fn drop_target_cell_padding_uses_cell_block_boundary() {
        let doc = table_with_short_cell_and_wrapped_neighbor();
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 180.0, &pd);

        let root = view.root().expect("root must exist");
        let table = root.child_blocks().next().expect("table must exist");
        let row = table.child_blocks().next().expect("row must exist");
        let cell = row.child_blocks().next().expect("cell must exist");
        let para = cell
            .child_blocks()
            .next()
            .expect("cell paragraph must exist");

        let cell_rect = index.box_rect(&cell.id()).expect("cell must have a box");
        let para_rect = index
            .box_rect(&para.id())
            .expect("paragraph must have a box");
        let x = para_rect.x + para_rect.width * 0.5;

        let top_padding_y = (cell_rect.y + para_rect.y) * 0.5 - index.pages()[0].y_start;
        let top_target = drop_target_at(&index, &view, 0, x, top_padding_y)
            .expect("cell top padding drop must resolve");
        assert_eq!(top_target.position.node, cell.id());
        assert_eq!(top_target.position.offset, 0);
        assert!(
            matches!(top_target.indicator, DropIndicator::Block { .. }),
            "cell padding drop should be a block boundary, got {:?}",
            top_target.indicator
        );

        let bottom_padding_y =
            (para_rect.bottom() + cell_rect.bottom()) * 0.5 - index.pages()[0].y_start;
        let bottom_target = drop_target_at(&index, &view, 0, x, bottom_padding_y)
            .expect("cell bottom padding drop must resolve");
        assert_eq!(bottom_target.position.node, cell.id());
        assert_eq!(bottom_target.position.offset, 1);
        assert!(
            matches!(bottom_target.indicator, DropIndicator::Block { .. }),
            "cell padding drop should be a block boundary, got {:?}",
            bottom_target.indicator
        );
    }

    fn two_para_gap_doc() -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        let mut doc = logs(&items);
        doc.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::BlockGap { value: 100 },
                },
            )
            .unwrap();
        doc
    }

    #[test]
    fn box_edge_block_indicator() {
        let doc = two_para_gap_doc();
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 400.0, &pd);

        let root_node = view.root().unwrap();
        let root_id = root_node.id();

        let mut para_iter = root_node.child_blocks();
        let p1_id = para_iter.next().expect("first paragraph must exist").id();
        let p2_id = para_iter.next().expect("second paragraph must exist").id();
        let p1_rect = index.box_rect(&p1_id).expect("p1 box must exist");
        let p2_rect = index.box_rect(&p2_id).expect("p2 box must exist");

        let gap_mid_y = (p1_rect.bottom() + p2_rect.y) / 2.0;
        assert!(
            p2_rect.y > p1_rect.bottom(),
            "gap between paragraphs must exist (block gap modifier must produce spacing)"
        );

        let page = index.page(0).expect("page 0 must exist");

        let gap_page_y = gap_mid_y - page.y_start;
        let gap_target = drop_target_at(&index, &view, 0, 10.0, gap_page_y)
            .expect("drop_target_at must resolve at a block edge in the inter-paragraph gap");
        assert!(
            matches!(gap_target.indicator, DropIndicator::Block { .. }),
            "indicator in inter-paragraph gap must be Block, got {:?}",
            gap_target.indicator
        );
        assert_eq!(
            gap_target.position.node, root_id,
            "drop in gap must resolve to root node"
        );

        let gap_bottom_page_y = p2_rect.y - 1.0 - page.y_start;
        let gap_bottom_target = drop_target_at(&index, &view, 0, 10.0, gap_bottom_page_y)
            .expect("drop_target_at must resolve at a block edge near the bottom of the gap");
        assert!(
            matches!(gap_bottom_target.indicator, DropIndicator::Block { .. }),
            "indicator near bottom of gap must be Block, got {:?}",
            gap_bottom_target.indicator
        );
        assert_eq!(
            gap_bottom_target.position.node, root_id,
            "drop near bottom of gap must resolve to root node"
        );
    }

    #[test]
    fn root_edge_guard() {
        let doc = para_doc_data("Hello");
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 400.0, &pd);

        let root_id = view.root().expect("root must exist").id();

        let root_position = Position::new(root_id, 0);
        let result = promote_outer_edge_drop_position(&index, &view, root_position);
        assert!(
            result.is_none(),
            "promote_outer_edge_drop_position must return None for a root-node Position"
        );

        let root_end_position = Position::new(root_id, 1);
        let result_end = promote_outer_edge_drop_position(&index, &view, root_end_position);
        assert!(
            result_end.is_none(),
            "promote_outer_edge_drop_position must return None for root-node end Position"
        );
    }

    #[test]
    fn atom_drop() {
        let root = Dot::ROOT;
        let hr = Dot::new(1, 1);
        let p = Dot::new(1, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let (view, index) = build_index(&doc, 400.0, &pd);

        let root_id = view.root().expect("root must exist").id();

        let atom_entry = index
            .entries()
            .find(|e| matches!(e.content(&index), Some(LayoutContent::Atom(_))))
            .expect("atom entry must exist");
        let LayoutContent::Atom(atom) = atom_entry.content(&index).unwrap() else {
            panic!("must be atom");
        };
        let expected_parent = atom.attachment.parent;
        let expected_offset = atom.attachment.index + 1;
        let atom_rect = atom_entry.rect;

        let mid_x = atom_rect.x + atom_rect.width * 0.5;
        let mid_y_page =
            atom_rect.y + atom_rect.height * 0.5 - index.page_y_start(0).unwrap_or(0.0);

        let result = drop_target_at(&index, &view, 0, mid_x, mid_y_page);
        let target = result.expect("drop target on atom must be Some");

        assert_eq!(
            target.position.node, expected_parent,
            "position.node must be atom.attachment.parent (root Dot)"
        );
        assert_eq!(
            target.position.node, root_id,
            "atom attachment parent must be the root"
        );
        assert_eq!(
            target.position.offset, expected_offset,
            "offset must be attachment.index + 1"
        );
    }
}

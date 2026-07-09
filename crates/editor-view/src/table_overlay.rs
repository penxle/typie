use editor_common::Rect;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{Alignment, DocView, Modifier, ModifierType, Node, NodeType, TableBorderStyle};
use editor_state::ResolvedSelection;
use serde::{Deserialize, Serialize};

use crate::page_fragment::{PageFragmentBox, PageFragmentNode, PageFragmentTree};

const TABLE_BORDER_WIDTH: f32 = 1.0;
const MIN_CELL_WIDTH: f32 = 40.0;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlay {
    pub table_id: Dot,
    pub page_idx: usize,
    pub bounds: Rect,
    pub border_style: TableBorderStyle,
    pub align: Alignment,
    pub proportion: f32,
    pub content_width: f32,
    pub min_proportion_width: f32,
    pub max_proportion_width: f32,
    pub rows: Vec<TableOverlayRow>,
    pub columns: Vec<TableOverlayColumn>,
    pub row_count: usize,
    pub is_last_row_fragment: bool,
    pub is_focused: bool,
    pub focused_row_index: Option<usize>,
    pub focused_col_index: Option<usize>,
    pub cell_selection: Option<TableOverlayCellSelection>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlayCellSelection {
    pub anchor_row: usize,
    pub anchor_col: usize,
    pub head_row: usize,
    pub head_col: usize,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlayRow {
    pub index: usize,
    pub height: f32,
    pub position: f32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlayColumn {
    pub index: usize,
    pub width_as_px: f32,
    pub position: f32,
}

#[derive(Debug)]
struct OverlayCell {
    index: usize,
    rect: Rect,
}

#[derive(Debug)]
struct OverlayRow {
    index: usize,
    rect: Rect,
    cells: Vec<OverlayCell>,
}

pub(crate) fn page_table_overlays(
    fragment_tree: &PageFragmentTree,
    view: &DocView,
    selection: Option<&ResolvedSelection>,
    content_width: f32,
) -> Vec<TableOverlay> {
    let mut overlays = Vec::new();
    if let Some(root) = &fragment_tree.root {
        collect_table_overlays(
            root,
            fragment_tree.page_idx,
            view,
            selection,
            content_width,
            &mut overlays,
        );
    }
    overlays
}

fn collect_table_overlays(
    node: &PageFragmentNode,
    page_idx: usize,
    view: &DocView,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
    overlays: &mut Vec<TableOverlay>,
) {
    let Some(fragment_box) = node.as_box() else {
        return;
    };

    let node_view = view.node(fragment_box.node);
    match node_view.map(|nv| nv.node()) {
        Some(Node::Table(table_node)) => {
            if let Some(overlay) = build_table_overlay(
                node.rect,
                fragment_box,
                &table_node,
                page_idx,
                view,
                selection,
                content_width,
            ) {
                overlays.push(overlay);
            }
        }
        _ => {
            for child in &fragment_box.children {
                collect_table_overlays(child, page_idx, view, selection, content_width, overlays);
            }
        }
    }
}

fn build_table_overlay(
    table_rect: Rect,
    table_box: &PageFragmentBox,
    table_node: &editor_model::TableNode,
    page_idx: usize,
    view: &DocView,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
) -> Option<TableOverlay> {
    let table_id = table_box.node;
    let doc_node = view.node(table_id)?;
    let mut rows = visible_rows(table_box, view);
    rows.sort_by_key(|row| row.index);
    for row in &mut rows {
        row.cells.sort_by_key(|cell| cell.index);
    }

    let fragment_top = rows.first()?.rect.y;
    let fragment_bottom = rows.last()?.rect.bottom();
    let row_count = doc_node
        .child_blocks()
        .filter(|row| row.node_type() == NodeType::TableRow)
        .count();
    let is_last_row_fragment = rows
        .last()
        .is_some_and(|row| row.index.checked_add(1) == Some(row_count));

    let bounds = Rect::from_xywh(
        table_rect.x,
        fragment_top,
        table_rect.width,
        fragment_bottom - fragment_top,
    );

    let proportion = *table_node.proportion.get() as f32 / 100.0;
    let border_style = *table_node.border_style.get();
    let align = doc_node
        .block_modifier(ModifierType::Alignment)
        .and_then(|m| {
            if let Modifier::Alignment { value } = m {
                Some(*value)
            } else {
                None
            }
        })
        .unwrap_or(Alignment::Left);

    let columns: Vec<TableOverlayColumn> = rows
        .first()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| TableOverlayColumn {
                    index: cell.index,
                    width_as_px: (cell.rect.width - 2.0 * TABLE_BORDER_WIDTH).max(0.0),
                    position: cell.rect.right() - table_rect.x,
                })
                .collect()
        })
        .unwrap_or_default();
    let min_proportion_width = min_table_width(columns.len());
    let max_proportion_width = content_width.max(0.0);

    let overlay_rows = rows
        .iter()
        .map(|row| TableOverlayRow {
            index: row.index,
            height: (row.rect.height - 2.0 * TABLE_BORDER_WIDTH).max(0.0),
            position: row.rect.bottom() - fragment_top,
        })
        .collect::<Vec<_>>();

    let is_focused = selection
        .map(|sel| {
            is_inside_table(sel.anchor().node(), view, table_id)
                || is_inside_table(sel.head().node(), view, table_id)
        })
        .unwrap_or(false);

    let focused_row_index = if is_focused {
        selection
            .and_then(|sel| focused_row(sel.anchor().node(), view, table_id))
            .and_then(|row_idx| rows.iter().position(|row| row.index == row_idx))
    } else {
        None
    };

    let focused_col_index = if is_focused {
        selection.and_then(|sel| focused_col(sel.anchor().node(), view, table_id))
    } else {
        None
    };

    let cell_rect = selection.and_then(|sel| {
        let rect = sel.as_cell_rect()?;
        (rect.table_id() == table_id).then_some(rect)
    });

    let is_cross_boundary = cell_rect.is_none()
        && selection.is_some_and(|sel| is_table_boundary_selection(sel, view, table_id));

    let is_table_cell_selection = cell_rect.is_some() || is_cross_boundary;

    let (
        global_cell_selection_row_start,
        global_cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    ) = if is_cross_boundary {
        let max_cols = doc_node
            .child_blocks()
            .filter(|r| r.node_type() == NodeType::TableRow)
            .map(|r| {
                r.child_blocks()
                    .filter(|c| c.node_type() == NodeType::TableCell)
                    .count()
            })
            .max()
            .unwrap_or(0);
        (
            Some(0usize),
            row_count.checked_sub(1),
            Some(0usize),
            max_cols.checked_sub(1),
        )
    } else {
        (
            cell_rect.as_ref().map(|r| *r.rows().start()),
            cell_rect.as_ref().map(|r| *r.rows().end()),
            cell_rect.as_ref().map(|r| *r.cols().start()),
            cell_rect.as_ref().map(|r| *r.cols().end()),
        )
    };

    let visible_row_start = rows.first()?.index;
    let visible_row_end = rows.last()?.index;
    let cell_selection = match (
        is_table_cell_selection,
        global_cell_selection_row_start,
        global_cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    ) {
        (true, Some(row_start), Some(row_end), Some(col_start), Some(col_end))
            if row_start <= visible_row_end && row_end >= visible_row_start =>
        {
            if let Some(rect) = cell_rect.as_ref() {
                Some(TableOverlayCellSelection {
                    anchor_row: rect.anchor_cell.parent()?.index()?,
                    anchor_col: rect.anchor_cell.index()?,
                    head_row: rect.head_cell.parent()?.index()?,
                    head_col: rect.head_cell.index()?,
                })
            } else {
                Some(TableOverlayCellSelection {
                    anchor_row: row_start,
                    anchor_col: col_start,
                    head_row: row_end,
                    head_col: col_end,
                })
            }
        }
        _ => None,
    };

    Some(TableOverlay {
        table_id,
        page_idx,
        bounds,
        border_style,
        align,
        proportion,
        content_width,
        min_proportion_width,
        max_proportion_width,
        rows: overlay_rows,
        columns,
        row_count,
        is_last_row_fragment,
        is_focused,
        focused_row_index,
        focused_col_index,
        cell_selection,
    })
}

fn min_table_width(col_count: usize) -> f32 {
    if col_count == 0 {
        return TABLE_BORDER_WIDTH;
    }
    col_count as f32 * MIN_CELL_WIDTH + (col_count + 1) as f32 * TABLE_BORDER_WIDTH
}

fn visible_rows(table_box: &PageFragmentBox, view: &DocView) -> Vec<OverlayRow> {
    table_box
        .children
        .iter()
        .filter_map(|row_node| {
            let row_box = row_node.as_box()?;
            let row_view = view.node(row_box.node)?;
            if row_view.node_type() != NodeType::TableRow {
                return None;
            }

            let cells = row_box
                .children
                .iter()
                .filter_map(|cell_node| {
                    let cell_box = cell_node.as_box()?;
                    let cell_view = view.node(cell_box.node)?;
                    (cell_view.node_type() == NodeType::TableCell).then(|| OverlayCell {
                        index: cell_view.index().unwrap_or(0),
                        rect: cell_node.rect,
                    })
                })
                .collect();

            Some(OverlayRow {
                index: row_view.index().unwrap_or(0),
                rect: row_node.rect,
                cells,
            })
        })
        .collect()
}

fn is_table_boundary_selection(sel: &ResolvedSelection<'_>, view: &DocView, table_id: Dot) -> bool {
    let Some(table) = view.node(table_id) else {
        return false;
    };
    let Some(parent) = table.parent() else {
        return false;
    };
    let Some(table_idx) = table.index() else {
        return false;
    };
    let parent_id = parent.id();
    let from_pos = sel.from().position();
    let to_pos = sel.to().position();
    (from_pos.node == parent_id && from_pos.offset == table_idx)
        || (to_pos.node == parent_id && to_pos.offset == table_idx + 1)
}

fn is_inside_table(node_id: Dot, view: &DocView, table_id: Dot) -> bool {
    view.node(node_id)
        .is_some_and(|n| n.ancestors().any(|a| a.id() == table_id))
}

fn focused_row(node_id: Dot, view: &DocView, table_id: Dot) -> Option<usize> {
    let node = view.node(node_id)?;
    let row = node
        .ancestors()
        .find(|a| a.parent().is_some_and(|p| p.id() == table_id))?;
    view.node(table_id)?
        .child_blocks()
        .position(|c| c.id() == row.id())
}

fn focused_col(node_id: Dot, view: &DocView, table_id: Dot) -> Option<usize> {
    let node = view.node(node_id)?;
    let cell = node.ancestors().find(|a| {
        a.parent()
            .is_some_and(|p| p.parent().is_some_and(|gp| gp.id() == table_id))
    })?;
    let row = cell.parent()?;
    row.child_blocks().position(|c| c.id() == cell.id())
}

#[cfg(test)]
mod tests {
    use editor_common::{EdgeInsets, Rect, Size};
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, Alignment, DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp, NodeAttr,
        NodeAttrLog, NodeAttrOp, NodeType, ProjectedDoc, SeqItem, SpanLog, TableNodeAttr,
        project_document,
    };
    use editor_state::Affinity;
    use editor_state::{Position, ResolvedSelection, Selection};

    use crate::page::LayoutPage;
    use crate::page_fragment::build_page_fragment_tree;
    use crate::paginate::types::{LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree};
    use crate::style::{Alignment as StyleAlignment, BorderMode, BoxStyle, Direction};

    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_xywh(x, y, w, h)
    }

    fn elem(peer: u64, clock: u64) -> Dot {
        Dot::new(peer, clock)
    }

    fn page(y_start: f32, height: f32) -> LayoutPage {
        LayoutPage::new(y_start, y_start + height, Size::new(800.0, height))
    }

    fn empty_box_style() -> BoxStyle {
        BoxStyle {
            direction: Direction::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: StyleAlignment::Start,
            decorations: vec![],
            monolithic: false,
        }
    }

    fn box_node(
        node: Dot,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: rect(x, y, w, h),
            content: LayoutContent::Box(LayoutBox {
                node,
                style: empty_box_style(),
                children,
                attachment: None,
                scope: false,
            }),
        }
    }

    fn line_node(node: Dot, x: f32, y: f32, w: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: rect(x, y, w, h),
            content: LayoutContent::Line(LayoutLine {
                measured: std::sync::Arc::new(crate::measure::text::measure::MeasuredLine {
                    height: 0.0,
                    node,
                    baseline: h * 0.8,
                    ascent: h * 0.8,
                    descent: h * 0.2,
                    cursor_ascent: h * 0.8,
                    cursor_descent: h * 0.2,
                    glyph_runs: vec![],
                    ruby_annotations: vec![],
                    empty_caret_x: 0.0,
                    offset_range: None,
                    tab_gaps: vec![],
                    is_phantom: false,
                    content_edge_x: None,
                }),
            }),
        }
    }

    fn doc_logs(items: &[(Dot, SeqItem)]) -> DocLogs {
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

    // root(1,0) > table(1,1) > [row0(1,2) > [cell00(1,3) > para(1,8), cell01(1,4) > para(1,9)],
    //                            row1(1,5) > [cell10(1,6) > para(1,10), cell11(1,7) > para(1,11)]]
    fn two_by_two_table_doc() -> (ProjectedDoc, Dot, Dot, Dot, Dot, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row0 = Dot::new(1, 2);
        let cell00 = Dot::new(1, 3);
        let cell01 = Dot::new(1, 4);
        let row1 = Dot::new(1, 5);
        let cell10 = Dot::new(1, 6);
        let cell11 = Dot::new(1, 7);
        let para00 = Dot::new(1, 8);
        let para01 = Dot::new(1, 9);
        let para10 = Dot::new(1, 10);
        let para11 = Dot::new(1, 11);

        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row0,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell00,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                para00,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell00],
                    attrs: vec![],
                },
            ),
            (
                cell01,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                para01,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell01],
                    attrs: vec![],
                },
            ),
            (
                row1,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell10,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row1],
                    attrs: vec![],
                },
            ),
            (
                para10,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row1, cell10],
                    attrs: vec![],
                },
            ),
            (
                cell11,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row1],
                    attrs: vec![],
                },
            ),
            (
                para11,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row1, cell11],
                    attrs: vec![],
                },
            ),
        ];

        let mut logs = doc_logs(&items);

        // Set table alignment = Center
        logs.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                ModifierAttrOp::SetModifier {
                    target: table,
                    modifier: Modifier::Alignment {
                        value: Alignment::Center,
                    },
                },
            )
            .unwrap()
            // Set cell00 background = "#fff"
            .apply(
                Dot::new(2, 1),
                ModifierAttrOp::SetModifier {
                    target: cell00,
                    modifier: Modifier::BackgroundColor {
                        value: "#fff".into(),
                    },
                },
            )
            .unwrap();

        // Set table proportion = 100 via node_attrs
        logs.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::ROOT,
                NodeAttrOp {
                    target: table,
                    attr: NodeAttr::Table {
                        attr: TableNodeAttr::Proportion(100),
                    },
                },
            )
            .unwrap();

        let pd = project_document(&logs).unwrap();
        (pd, root, table, row0, cell00, cell01, row1, cell10, cell11)
    }

    fn resolved_sel<'a>(
        view: &'a DocView<'a>,
        anchor_node: Dot,
        anchor_off: usize,
        head_node: Dot,
        head_off: usize,
    ) -> ResolvedSelection<'a> {
        let a = Position {
            node: anchor_node,
            offset: anchor_off,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: head_node,
            offset: head_off,
            affinity: Affinity::Downstream,
        };
        Selection::new(a, h).resolve(view).unwrap()
    }

    // Build table LayoutTree:
    // table box (0,0,600,80) > [
    //   row0 box (0,0,600,40) > [cell00 box (0,0,300,40) > line, cell01 box (300,0,300,40) > line],
    //   row1 box (0,40,600,40) > [cell10 box (0,40,300,40) > line, cell11 box (300,40,300,40) > line]
    // ]
    #[allow(clippy::too_many_arguments)]
    fn table_layout_tree(
        table_id: Dot,
        row0_id: Dot,
        cell00_id: Dot,
        cell01_id: Dot,
        row1_id: Dot,
        cell10_id: Dot,
        cell11_id: Dot,
        para00_id: Dot,
        para01_id: Dot,
        para10_id: Dot,
        para11_id: Dot,
        root_id: Dot,
    ) -> LayoutTree {
        let line00 = line_node(para00_id, 0.0, 0.0, 300.0, 40.0);
        let line01 = line_node(para01_id, 300.0, 0.0, 300.0, 40.0);
        let line10 = line_node(para10_id, 0.0, 40.0, 300.0, 40.0);
        let line11 = line_node(para11_id, 300.0, 40.0, 300.0, 40.0);

        let cell00_node = box_node(cell00_id, 0.0, 0.0, 300.0, 40.0, vec![line00]);
        let cell01_node = box_node(cell01_id, 300.0, 0.0, 300.0, 40.0, vec![line01]);
        let cell10_node = box_node(cell10_id, 0.0, 40.0, 300.0, 40.0, vec![line10]);
        let cell11_node = box_node(cell11_id, 300.0, 40.0, 300.0, 40.0, vec![line11]);

        let row0_node = box_node(
            row0_id,
            0.0,
            0.0,
            600.0,
            40.0,
            vec![cell00_node, cell01_node],
        );
        let row1_node = box_node(
            row1_id,
            0.0,
            40.0,
            600.0,
            40.0,
            vec![cell10_node, cell11_node],
        );

        let table_node = box_node(table_id, 0.0, 0.0, 600.0, 80.0, vec![row0_node, row1_node]);

        let root_node = box_node(root_id, 0.0, 0.0, 600.0, 80.0, vec![table_node]);

        LayoutTree { root: root_node }
    }

    #[test]
    fn index_faithfulness_cell_column_ordinal() {
        let (pd, _root, _table, _row0, cell00, cell01, _row1, _cell10, cell11) =
            two_by_two_table_doc();
        let view = DocView::new(&pd);

        let cell00_view = view.node(cell00).unwrap();
        let cell01_view = view.node(cell01).unwrap();
        let cell11_view = view.node(cell11).unwrap();

        assert_eq!(cell00_view.index(), Some(0), "cell00 is at column 0");
        assert_eq!(cell01_view.index(), Some(1), "cell01 is at column 1");
        assert_eq!(
            cell11_view.index(),
            Some(1),
            "cell11 is at column 1 in row1"
        );
    }

    #[test]
    fn overlay_structure_two_by_two_table() {
        let (pd, root, table, row0, cell00, cell01, row1, cell10, cell11) = two_by_two_table_doc();
        let view = DocView::new(&pd);

        let para00_id = elem(1, 8);
        let para01_id = elem(1, 9);
        let para10_id = elem(1, 10);
        let para11_id = elem(1, 11);

        let tree = table_layout_tree(
            table, row0, cell00, cell01, row1, cell10, cell11, para00_id, para01_id, para10_id,
            para11_id, root,
        );

        let pg = page(0.0, 200.0);
        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        let overlays = page_table_overlays(&fragment, &view, None, 600.0);

        assert_eq!(overlays.len(), 1, "exactly one table overlay");

        let ov = &overlays[0];
        assert_eq!(ov.table_id, table, "table_id matches the table Dot");
        assert_eq!(ov.row_count, 2, "row_count is 2");
        assert_eq!(ov.rows.len(), 2, "two visible rows");
        assert_eq!(ov.columns.len(), 2, "two columns from first row");

        assert_eq!(ov.rows[0].index, 0);
        assert_eq!(ov.rows[1].index, 1);
        assert_eq!(ov.columns[0].index, 0);
        assert_eq!(ov.columns[1].index, 1);

        // bounds: x=0, y=0 (from_top fragment), width=600, height=80
        assert_eq!(ov.bounds.x, 0.0);
        assert_eq!(ov.bounds.width, 600.0);
        assert_eq!(ov.bounds.height, 80.0);
        assert_eq!(ov.min_proportion_width, 83.0);
        assert_eq!(ov.max_proportion_width, 600.0);

        // table alignment is Center (set via block_modifiers)
        assert_eq!(ov.align, Alignment::Center);

        assert!(!ov.is_focused);
        assert_eq!(ov.cell_selection, None);
    }

    #[test]
    fn cell_selection_single_cell_pins_row_and_col() {
        let (pd, root, table, row0, cell00, cell01, row1, cell10, cell11) = two_by_two_table_doc();
        let view = DocView::new(&pd);

        let para00_id = elem(1, 8);
        let para01_id = elem(1, 9);
        let para10_id = elem(1, 10);
        let para11_id = elem(1, 11);

        let tree = table_layout_tree(
            table, row0, cell00, cell01, row1, cell10, cell11, para00_id, para01_id, para10_id,
            para11_id, root,
        );

        let pg = page(0.0, 200.0);
        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        // cell selection: anchor=row0 offset 0, head=row0 offset 1 → selects cell00 only (col 0..=0, row 0..=0)
        let sel = resolved_sel(&view, row0, 0, row0, 1);
        let overlays = page_table_overlays(&fragment, &view, Some(&sel), 600.0);

        assert_eq!(overlays.len(), 1);
        let ov = &overlays[0];

        let cell_selection = ov
            .cell_selection
            .as_ref()
            .expect("single-cell selection is a cell selection");
        assert_eq!(cell_selection.anchor_row, 0);
        assert_eq!(cell_selection.anchor_col, 0);
        assert_eq!(cell_selection.head_row, 0);
        assert_eq!(cell_selection.head_col, 0);

        // anchor is inside table (row0 is an ancestor of table) → is_focused
        assert!(ov.is_focused);
        // anchor node is row0: focused_row finds row0 whose parent is table → index 0
        assert_eq!(ov.focused_row_index, Some(0));
        // anchor node is row0 (a row, not a cell): focused_col cannot find a cell ancestor → None
        assert_eq!(ov.focused_col_index, None);
    }

    #[test]
    fn cell_selection_full_row_pins_both_cols() {
        let (pd, root, table, row0, cell00, cell01, row1, cell10, cell11) = two_by_two_table_doc();
        let view = DocView::new(&pd);

        let para00_id = elem(1, 8);
        let para01_id = elem(1, 9);
        let para10_id = elem(1, 10);
        let para11_id = elem(1, 11);

        let tree = table_layout_tree(
            table, row0, cell00, cell01, row1, cell10, cell11, para00_id, para01_id, para10_id,
            para11_id, root,
        );

        let pg = page(0.0, 200.0);
        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        // anchor=row0 offset 0, head=row0 offset 2 → selects both cells in row0 (cols 0..=1)
        let sel = resolved_sel(&view, row0, 0, row0, 2);
        let overlays = page_table_overlays(&fragment, &view, Some(&sel), 600.0);

        assert_eq!(overlays.len(), 1);
        let ov = &overlays[0];

        let cell_selection = ov
            .cell_selection
            .as_ref()
            .expect("full-row selection is a cell selection");
        assert_eq!(cell_selection.anchor_row.min(cell_selection.head_row), 0);
        assert_eq!(cell_selection.anchor_row.max(cell_selection.head_row), 0);
        assert_eq!(cell_selection.anchor_col.min(cell_selection.head_col), 0);
        assert_eq!(cell_selection.anchor_col.max(cell_selection.head_col), 1);
    }

    #[test]
    fn cell_selection_reversed_carries_anchor_and_head_cells() {
        let (pd, root, table, row0, cell00, cell01, row1, cell10, cell11) = two_by_two_table_doc();
        let view = DocView::new(&pd);

        let para00_id = elem(1, 8);
        let para01_id = elem(1, 9);
        let para10_id = elem(1, 10);
        let para11_id = elem(1, 11);

        let tree = table_layout_tree(
            table, row0, cell00, cell01, row1, cell10, cell11, para00_id, para01_id, para10_id,
            para11_id, root,
        );

        let pg = page(0.0, 200.0);
        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        let sel = resolved_sel(&view, row1, 2, row0, 0);
        let overlays = page_table_overlays(&fragment, &view, Some(&sel), 600.0);

        assert_eq!(overlays.len(), 1);
        let ov = &overlays[0];

        let cell_selection = ov
            .cell_selection
            .as_ref()
            .expect("reversed selection is a cell selection");
        assert_eq!(cell_selection.anchor_row, 1);
        assert_eq!(cell_selection.anchor_col, 1);
        assert_eq!(cell_selection.head_row, 0);
        assert_eq!(cell_selection.head_col, 0);
    }

    #[test]
    fn table_boundary_selection_triggers_focused_path() {
        let (pd, root, table, row0, cell00, cell01, row1, cell10, cell11) = two_by_two_table_doc();
        let view = DocView::new(&pd);

        let para00_id = elem(1, 8);
        let para01_id = elem(1, 9);
        let para10_id = elem(1, 10);
        let para11_id = elem(1, 11);

        let tree = table_layout_tree(
            table, row0, cell00, cell01, row1, cell10, cell11, para00_id, para01_id, para10_id,
            para11_id, root,
        );

        let pg = page(0.0, 200.0);
        let fragment = build_page_fragment_tree(&tree, 0, &pg);

        // Table is the only child of root (index 0).
        // Boundary selection: anchor at root offset 0, head at root offset 1 → spans exactly the table.
        // is_table_boundary_selection checks:
        //   from_pos.node == parent_id (root) && from_pos.offset == table_idx (0)
        //   OR to_pos.node == parent_id (root) && to_pos.offset == table_idx + 1 (1)
        // anchor=root,0  head=root,1 satisfies BOTH conditions.
        let sel = resolved_sel(&view, root, 0, root, 1);
        let overlays = page_table_overlays(&fragment, &view, Some(&sel), 600.0);

        assert_eq!(overlays.len(), 1);
        let ov = &overlays[0];

        // anchor/head are both in root (not inside table), so is_focused = false
        assert!(!ov.is_focused);

        // is_cross_boundary → is_table_cell_selection = true → full table range
        let cell_selection = ov
            .cell_selection
            .as_ref()
            .expect("cross-boundary selection triggers cell selection");
        // cross-boundary: all rows 0..=row_count-1=1, all cols 0..=max_cols-1=1
        assert_eq!(cell_selection.anchor_row, 0);
        assert_eq!(cell_selection.anchor_col, 0);
        assert_eq!(cell_selection.head_row, 1);
        assert_eq!(cell_selection.head_col, 1);
    }
}

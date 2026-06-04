use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Alignment, Doc, Modifier, Node, NodeId, TableBorderStyle, TableNode};

fn cell_background_color(doc: &Doc, cell_id: NodeId) -> Option<String> {
    doc.node(cell_id)?
        .explicit_modifiers()
        .find_map(|m| match m {
            Modifier::BackgroundColor { value } => Some(value.clone()),
            _ => None,
        })
}
use editor_state::{Position, ResolvedSelection, Selection};
use serde::{Deserialize, Serialize};

use crate::page_fragment::{PageFragmentBox, PageFragmentNode, PageFragmentTree};

const TABLE_BORDER_WIDTH: f32 = 1.0;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlay {
    pub table_id: NodeId,
    pub page_idx: usize,
    pub bounds: Rect,
    pub border_style: TableBorderStyle,
    pub align: Alignment,
    pub proportion: f32,
    pub content_width: f32,
    pub rows: Vec<TableOverlayRow>,
    pub columns: Vec<TableOverlayColumn>,
    pub row_count: usize,
    pub is_last_row_fragment: bool,
    pub is_focused: bool,
    pub focused_row_index: Option<usize>,
    pub focused_col_index: Option<usize>,
    pub is_cell_selection: bool,
    pub cell_selection_background_color: Option<String>,
    pub cell_selection_row_start: Option<usize>,
    pub cell_selection_row_end: Option<usize>,
    pub cell_selection_col_start: Option<usize>,
    pub cell_selection_col_end: Option<usize>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlayRow {
    pub index: usize,
    pub height: f32,
    pub position: f32,
    pub background_color: Option<String>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TableOverlayColumn {
    pub index: usize,
    pub width_as_px: f32,
    pub position: f32,
    pub background_color: Option<String>,
}

pub(crate) fn table_overlays(
    page_fragments: &[PageFragmentTree],
    doc: &Doc,
    selection: Option<&Selection>,
    content_width: f32,
) -> Vec<TableOverlay> {
    let resolved = selection.and_then(|s| s.resolve(doc));
    let mut overlays = Vec::new();
    for fragment_tree in page_fragments {
        if let Some(root) = &fragment_tree.root {
            collect_table_overlays(
                root,
                fragment_tree.page_idx,
                doc,
                resolved.as_ref(),
                content_width,
                &mut overlays,
            );
        }
    }
    overlays
}

fn collect_table_overlays(
    node: &PageFragmentNode,
    page_idx: usize,
    doc: &Doc,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
    overlays: &mut Vec<TableOverlay>,
) {
    let Some(fragment_box) = node.as_box() else {
        return;
    };

    match doc.node(fragment_box.node_id).map(|n| n.node()) {
        Some(Node::Table(table_node)) => {
            if let Some(overlay) = build_table_overlay(
                node.rect,
                fragment_box,
                table_node,
                page_idx,
                doc,
                selection,
                content_width,
            ) {
                overlays.push(overlay);
            }
        }
        _ => {
            for child in &fragment_box.children {
                collect_table_overlays(child, page_idx, doc, selection, content_width, overlays);
            }
        }
    }
}

fn build_table_overlay(
    table_rect: Rect,
    table_box: &PageFragmentBox,
    table_node: &TableNode,
    page_idx: usize,
    doc: &Doc,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
) -> Option<TableOverlay> {
    let table_id = table_box.node_id;
    let doc_node = doc.node(table_id)?;
    let mut rows = visible_rows(table_box, doc);
    rows.sort_by_key(|row| row.index);
    for row in &mut rows {
        row.cells.sort_by_key(|cell| cell.index);
    }

    let fragment_top = rows.first()?.rect.y;
    let fragment_bottom = rows.last()?.rect.bottom();
    let row_count = doc_node
        .children()
        .filter(|row| matches!(row.node(), Node::TableRow(_)))
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
        .modifiers()
        .find_map(|m| {
            if let Modifier::Alignment { value } = m {
                Some(*value)
            } else {
                None
            }
        })
        .unwrap_or(Alignment::Left);

    let columns = rows
        .first()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| TableOverlayColumn {
                    index: cell.index,
                    width_as_px: (cell.rect.width - 2.0 * TABLE_BORDER_WIDTH).max(0.0),
                    position: cell.rect.right() - table_rect.x,
                    background_color: cell_background_color(doc, cell.node_id),
                })
                .collect()
        })
        .unwrap_or_default();

    let overlay_rows = rows
        .iter()
        .map(|row| TableOverlayRow {
            index: row.index,
            height: (row.rect.height - 2.0 * TABLE_BORDER_WIDTH).max(0.0),
            position: row.rect.bottom() - fragment_top,
            background_color: row
                .cells
                .first()
                .and_then(|cell| cell_background_color(doc, cell.node_id)),
        })
        .collect::<Vec<_>>();

    let is_focused = selection
        .map(|sel| {
            is_inside_table(sel.anchor().node_id(), doc, table_id)
                || is_inside_table(sel.head().node_id(), doc, table_id)
        })
        .unwrap_or(false);

    let focused_row_index = if is_focused {
        selection
            .and_then(|sel| focused_row(sel.anchor().node_id(), doc, table_id))
            .and_then(|row_idx| rows.iter().position(|row| row.index == row_idx))
    } else {
        None
    };

    let focused_col_index = if is_focused {
        selection.and_then(|sel| focused_col(sel.anchor().node_id(), doc, table_id))
    } else {
        None
    };

    let cell_rect = selection.and_then(|sel| {
        let rect = sel.as_cell_rect()?;
        (rect.table.id() == table_id).then_some(rect)
    });

    let is_cross_boundary = cell_rect.is_none()
        && selection.is_some_and(|sel| is_table_boundary_selection(sel, doc, table_id));

    let is_table_cell_selection = cell_rect.is_some() || is_cross_boundary;

    let cell_selection_background_color = cell_rect.as_ref().and_then(|rect| {
        let mut common: Option<Option<String>> = None;
        for cell in rect.cells() {
            let color = cell_background_color(doc, cell.id());
            match &common {
                None => common = Some(color),
                Some(c) if *c != color => return None,
                _ => {}
            }
        }
        common.flatten()
    });

    let (
        global_cell_selection_row_start,
        global_cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    ) = if is_cross_boundary {
        let max_cols = doc_node
            .children()
            .filter(|r| matches!(r.node(), Node::TableRow(_)))
            .map(|r| {
                r.children()
                    .filter(|c| matches!(c.node(), Node::TableCell(_)))
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
            cell_rect.as_ref().map(|r| *r.rows.start()),
            cell_rect.as_ref().map(|r| *r.rows.end()),
            cell_rect.as_ref().map(|r| *r.cols.start()),
            cell_rect.as_ref().map(|r| *r.cols.end()),
        )
    };

    let visible_row_start = rows.first()?.index;
    let visible_row_end = rows.last()?.index;
    let (cell_selection_row_start, cell_selection_row_end) = match (
        global_cell_selection_row_start,
        global_cell_selection_row_end,
    ) {
        (Some(row_start), Some(row_end))
            if row_start <= visible_row_end && row_end >= visible_row_start =>
        {
            let clipped_start = row_start.max(visible_row_start);
            let clipped_end = row_end.min(visible_row_end);
            (
                rows.iter().position(|row| row.index == clipped_start),
                rows.iter().position(|row| row.index == clipped_end),
            )
        }
        _ => (None, None),
    };
    let is_cell_selection = is_table_cell_selection && cell_selection_row_start.is_some();

    Some(TableOverlay {
        table_id,
        page_idx,
        bounds,
        border_style,
        align,
        proportion,
        content_width,
        rows: overlay_rows,
        columns,
        row_count,
        is_last_row_fragment,
        is_focused,
        focused_row_index,
        focused_col_index,
        is_cell_selection,
        cell_selection_background_color,
        cell_selection_row_start,
        cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    })
}

fn visible_rows(table_box: &PageFragmentBox, doc: &Doc) -> Vec<OverlayRow> {
    table_box
        .children
        .iter()
        .filter_map(|row_node| {
            let row_box = row_node.as_box()?;
            let row_doc = doc.node(row_box.node_id)?;
            if !matches!(row_doc.node(), Node::TableRow(_)) {
                return None;
            }

            let cells = row_box
                .children
                .iter()
                .filter_map(|cell_node| {
                    let cell_box = cell_node.as_box()?;
                    let cell_doc = doc.node(cell_box.node_id)?;
                    matches!(cell_doc.node(), Node::TableCell(_)).then(|| OverlayCell {
                        index: cell_doc.index().unwrap_or(0),
                        node_id: cell_box.node_id,
                        rect: cell_node.rect,
                    })
                })
                .collect();

            Some(OverlayRow {
                index: row_doc.index().unwrap_or(0),
                rect: row_node.rect,
                cells,
            })
        })
        .collect()
}

#[derive(Debug)]
struct OverlayCell {
    index: usize,
    node_id: NodeId,
    rect: Rect,
}

#[derive(Debug)]
struct OverlayRow {
    index: usize,
    rect: Rect,
    cells: Vec<OverlayCell>,
}

fn is_table_boundary_selection(sel: &ResolvedSelection<'_>, doc: &Doc, table_id: NodeId) -> bool {
    let Some(table) = doc.node(table_id) else {
        return false;
    };
    let Some(parent) = table.parent() else {
        return false;
    };
    let Some(table_idx) = table.index() else {
        return false;
    };
    let parent_id = parent.id();
    let from = Position::from(sel.from());
    let to = Position::from(sel.to());
    (from.node_id == parent_id && from.offset == table_idx)
        || (to.node_id == parent_id && to.offset == table_idx + 1)
}

fn is_inside_table(node_id: NodeId, doc: &Doc, table_id: NodeId) -> bool {
    doc.node(node_id)
        .is_some_and(|n| n.ancestors().any(|a| a.id() == table_id))
}

fn focused_row(node_id: NodeId, doc: &Doc, table_id: NodeId) -> Option<usize> {
    let node = doc.node(node_id)?;
    let row = node
        .ancestors()
        .find(|a| a.parent().is_some_and(|p| p.id() == table_id))?;
    doc.node(table_id)?
        .children()
        .position(|c| c.id() == row.id())
}

fn focused_col(node_id: NodeId, doc: &Doc, table_id: NodeId) -> Option<usize> {
    let node = doc.node(node_id)?;
    // Walk up to find the TableCell (whose parent is a TableRow child of the table)
    let cell = node.ancestors().find(|a| {
        a.parent()
            .is_some_and(|p| p.parent().is_some_and(|gp| gp.id() == table_id))
    })?;
    let row = cell.parent()?;
    row.children().position(|c| c.id() == cell.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::{MeasuredNode, MeasuredTree, Measurer};
    use crate::paginate::Paginator;
    use crate::view_state::ViewState;
    use editor_common::EdgeInsets;
    use editor_macros::doc;
    use std::sync::Arc;

    fn measured_tree(root: Arc<MeasuredNode>) -> MeasuredTree {
        MeasuredTree {
            root: Arc::unwrap_or_clone(root),
        }
    }

    #[test]
    fn paginated_table_overlay_splits_per_visible_page_content() {
        let (doc, table_id) = doc! {
            root {
                paragraph { text("before") }
                table_id: table {
                    table_row { table_cell { paragraph { text("A") } } }
                    table_row { table_cell { paragraph { text("B") } } }
                }
            }
        };
        let mut measurer = Measurer::new_test();
        let root = measurer.measure(&doc, NodeId::ROOT, 400.0, &ViewState::new());
        let paginated =
            Paginator::paginated(400.0, 130.0, EdgeInsets::all(10.0)).paginate(measured_tree(root));

        let overlays = table_overlays(&paginated.page_fragments, &doc, None, 380.0);
        let pages = paginated.pages;

        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0].page_idx, 0);
        assert_eq!(overlays[0].table_id, table_id);
        assert_eq!(overlays[0].rows[0].index, 0);
        assert_eq!(overlays[0].row_count, 2);
        assert!(!overlays[0].is_last_row_fragment);
        assert_eq!(overlays[1].page_idx, 1);
        assert_eq!(overlays[1].table_id, table_id);
        assert_eq!(overlays[1].rows[0].index, 1);
        assert_eq!(overlays[1].row_count, 2);
        assert!(overlays[1].is_last_row_fragment);

        for overlay in &overlays {
            let page = &pages[overlay.page_idx];
            let content_top = page.content_y_start - page.y_start;
            let content_bottom = page.content_y_end - page.y_start;
            assert!(overlay.bounds.y >= content_top);
            assert!(overlay.bounds.bottom() <= content_bottom);
            assert!(overlay.bounds.width > 0.0);
        }
    }
}

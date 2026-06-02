use editor_common::Rect;
use editor_macros::ffi;
use editor_model::{Alignment, Doc, Modifier, Node, NodeId, TableBorderStyle};

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

use crate::page::LayoutPage;
use crate::paginate::{LayoutContent, LayoutNode, LayoutTree};

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
    pub col_widths_as_px: Vec<f32>,
    pub col_positions: Vec<f32>,
    pub row_heights: Vec<f32>,
    pub row_positions: Vec<f32>,
    pub row_background_colors: Vec<Option<String>>,
    pub col_background_colors: Vec<Option<String>>,
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

pub(crate) fn table_overlays(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    doc: &Doc,
    selection: Option<&Selection>,
    content_width: f32,
) -> Vec<TableOverlay> {
    let resolved = selection.and_then(|s| s.resolve(doc));
    let mut overlays = Vec::new();
    collect_from_node(
        &tree.root,
        pages,
        doc,
        resolved.as_ref(),
        content_width,
        &mut overlays,
    );
    overlays
}

fn collect_from_node(
    node: &LayoutNode,
    pages: &[LayoutPage],
    doc: &Doc,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
    out: &mut Vec<TableOverlay>,
) {
    let LayoutContent::Box(b) = &node.content else {
        return;
    };

    if let Some(doc_node) = doc.node(b.node_id) {
        if let Node::Table(table) = doc_node.node() {
            if let Some(page_idx) = find_page_idx(node, pages) {
                let page = &pages[page_idx];
                let overlay = build_overlay(
                    node,
                    b.node_id,
                    &doc_node,
                    table,
                    page_idx,
                    page,
                    doc,
                    selection,
                    content_width,
                );
                out.push(overlay);
                // Don't recurse into table children — no nested tables
                return;
            }
        }
    }

    for child in &b.children {
        collect_from_node(child, pages, doc, selection, content_width, out);
    }
}

fn find_page_idx(node: &LayoutNode, pages: &[LayoutPage]) -> Option<usize> {
    let center_y = node.rect.y + node.rect.height / 2.0;
    pages
        .iter()
        .position(|p| center_y >= p.y_start && center_y < p.y_end)
}

fn build_overlay<'a>(
    node: &LayoutNode,
    table_id: NodeId,
    doc_node: &editor_model::NodeRef<'a>,
    table: &editor_model::TableNode,
    page_idx: usize,
    page: &LayoutPage,
    doc: &'a Doc,
    selection: Option<&ResolvedSelection<'_>>,
    content_width: f32,
) -> TableOverlay {
    let LayoutContent::Box(table_box) = &node.content else {
        unreachable!("table node must be a Box")
    };

    let bounds = Rect::from_xywh(
        node.rect.x,
        node.rect.y - page.y_start,
        node.rect.width,
        node.rect.height,
    );

    let proportion = *table.proportion.get() as f32 / 100.0;
    let border_style = *table.border_style.get();
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

    let mut col_widths_as_px = Vec::new();
    let mut col_positions = Vec::new();
    let mut row_heights = Vec::new();
    let mut row_positions = Vec::new();
    let mut row_background_colors: Vec<Option<String>> = Vec::new();
    let mut col_background_colors: Vec<Option<String>> = Vec::new();

    for (row_idx, row_node) in table_box
        .children
        .iter()
        .filter(|n| matches!(n.content, LayoutContent::Box(_)))
        .enumerate()
    {
        let row_height = (row_node.rect.height - 2.0 * TABLE_BORDER_WIDTH).max(0.0);
        row_heights.push(row_height);
        row_positions.push(row_node.rect.bottom() - node.rect.y);

        if let LayoutContent::Box(row_box) = &row_node.content {
            let first_cell_id = row_box
                .children
                .iter()
                .find(|n| matches!(n.content, LayoutContent::Box(_)))
                .and_then(|n| {
                    if let LayoutContent::Box(b) = &n.content {
                        Some(b.node_id)
                    } else {
                        None
                    }
                });
            let row_bg = first_cell_id.and_then(|id| cell_background_color(doc, id));
            row_background_colors.push(row_bg);

            if row_idx == 0 {
                for cell_node in row_box
                    .children
                    .iter()
                    .filter(|n| matches!(n.content, LayoutContent::Box(_)))
                {
                    let col_width = (cell_node.rect.width - 2.0 * TABLE_BORDER_WIDTH).max(0.0);
                    col_widths_as_px.push(col_width);
                    col_positions.push(cell_node.rect.right() - node.rect.x);

                    let cell_bg = if let LayoutContent::Box(cb) = &cell_node.content {
                        cell_background_color(doc, cb.node_id)
                    } else {
                        None
                    };
                    col_background_colors.push(cell_bg);
                }
            }
        }
    }

    let is_focused = selection
        .map(|sel| {
            is_inside_table(sel.anchor().node_id(), doc, table_id)
                || is_inside_table(sel.head().node_id(), doc, table_id)
        })
        .unwrap_or(false);

    let focused_row_index = if is_focused {
        selection.and_then(|sel| focused_row(sel.anchor().node_id(), doc, table_id))
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

    let is_cell_selection = cell_rect.is_some() || is_cross_boundary;

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
        cell_selection_row_start,
        cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    ) = if is_cross_boundary {
        let row_count = doc_node
            .children()
            .filter(|r| matches!(r.node(), Node::TableRow(_)))
            .count();
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

    TableOverlay {
        table_id,
        page_idx,
        bounds,
        border_style,
        align,
        proportion,
        content_width,
        col_widths_as_px,
        col_positions,
        row_heights,
        row_positions,
        row_background_colors,
        col_background_colors,
        is_focused,
        focused_row_index,
        focused_col_index,
        is_cell_selection,
        cell_selection_background_color,
        cell_selection_row_start,
        cell_selection_row_end,
        cell_selection_col_start,
        cell_selection_col_end,
    }
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

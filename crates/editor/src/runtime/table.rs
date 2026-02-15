use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::{Element, PositionedNode};
use crate::model::{Doc, NodeId, TABLE_BORDER_WIDTH, TableAlign, TableBorderStyle};
use crate::runtime::Runtime;
use crate::runtime::cmd::{self, TableOverlay};
use crate::state::Selection;
use crate::types::{Point, Rect};
use std::rc::Rc;

impl Runtime {
    pub fn build_table_overlays(&self) -> Vec<TableOverlay> {
        let mut overlays = Vec::new();
        let focused_page_idx =
            focused_cursor_page(&self.state.selection, &self.state.doc, &self.pages);

        for (page_idx, page) in self.pages.iter().enumerate() {
            collect_table_overlays_from_tree(
                &page.root,
                Point::zero(),
                page_idx,
                &self.state.selection,
                &self.state.doc,
                focused_page_idx,
                &mut overlays,
            );
        }

        overlays
    }
}

fn collect_table_overlays_from_tree(
    positioned: &PositionedNode,
    offset: Point,
    page_idx: usize,
    selection: &Selection,
    doc: &Rc<Doc>,
    focused_page_idx: Option<usize>,
    overlays: &mut Vec<TableOverlay>,
) {
    let abs_pos = Point::new(
        offset.x + positioned.position.x,
        offset.y + positioned.position.y,
    );

    if let Some(ref element) = positioned.node.element {
        if let Element::TableBorder(table_border) = element {
            let is_focused = is_cursor_in_table(selection.head.node_id, table_border.node_id, doc)
                && focused_page_idx == Some(page_idx);

            let mut col_positions = Vec::new();
            let mut x = TABLE_BORDER_WIDTH;
            for &col_width in &table_border.col_widths {
                x += col_width;
                col_positions.push(x);
                x += TABLE_BORDER_WIDTH;
            }

            let mut row_positions = Vec::new();
            let mut y = 0.0;
            for &row_height in &table_border.row_heights {
                y += row_height;
                row_positions.push(y);
            }

            let border_style = match table_border.border_style {
                TableBorderStyle::Solid => "solid",
                TableBorderStyle::Dashed => "dashed",
                TableBorderStyle::Dotted => "dotted",
                TableBorderStyle::None => "none",
            };

            overlays.push(cmd::TableOverlay {
                page_idx,
                table_id: table_border.node_id.to_string(),
                bounds: Rect {
                    x: abs_pos.x + table_border.x_offset,
                    y: abs_pos.y,
                    width: table_border.size.width,
                    height: table_border.size.height,
                },
                border_style: border_style.to_string(),
                align: match table_border.align {
                    TableAlign::Left => "left".to_string(),
                    TableAlign::Center => "center".to_string(),
                    TableAlign::Right => "right".to_string(),
                },
                col_widths: table_border.col_widths.clone(),
                col_positions,
                row_heights: table_border.row_heights.clone(),
                row_positions,
                start_row_index: table_border.start_row_index,
                total_rows: table_border.total_rows,
                is_focused,
            });
        }
    }

    if let Some(children) = &positioned.node.children {
        for child in children {
            collect_table_overlays_from_tree(
                child,
                abs_pos,
                page_idx,
                selection,
                doc,
                focused_page_idx,
                overlays,
            );
        }
    }
}

fn focused_cursor_page(
    selection: &Selection,
    doc: &Rc<Doc>,
    pages: &[crate::layout::Page],
) -> Option<usize> {
    let ctx = NavigationContext::new(doc);
    Cursor::bounds(&ctx, pages, selection.head).map(|(page_idx, _)| page_idx)
}

fn is_cursor_in_table(cursor_node_id: NodeId, table_id: NodeId, doc: &Rc<Doc>) -> bool {
    let Some(cursor_node) = doc.node(cursor_node_id) else {
        return false;
    };

    for ancestor in cursor_node.ancestors() {
        if ancestor.node_id() == table_id {
            return true;
        }
    }

    false
}

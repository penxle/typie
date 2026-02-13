use crate::layout::elements::TableCellElement;
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::model::nodes::table::TABLE_BORDER_WIDTH;
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TableRowNode {}

impl NodeHtmlCodec for TableRowNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(DomSpec::el("tr").hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("tr", |_| {
            Some(Node::TableRow(TableRowNode {}))
        })]
    }
}

impl Layout for TableRowNode {
    fn layout(&self, ctx: &LayoutContext, _constraints: BoxConstraints) -> LayoutNode {
        let cells: Vec<_> = ctx.node.children().collect();
        if cells.is_empty() {
            return LayoutNode {
                size: Size::new(0.0, 0.0),
                element: None,
                children: None,
                page_break_policy: PageBreakPolicy::Avoid,
                render_hints: Default::default(),
                scope_id: None,
            };
        }

        let col_widths: Vec<f32> = if let Some(table) = ctx.node.parent() {
            if let Some(first_row) = table.children().next() {
                first_row
                    .children()
                    .map(|cell| {
                        if let Node::TableCell(cell_node) = cell.node() {
                            cell_node
                                .col_width
                                .unwrap_or(crate::model::nodes::table::DEFAULT_CELL_WIDTH)
                        } else {
                            crate::model::nodes::table::DEFAULT_CELL_WIDTH
                        }
                    })
                    .collect()
            } else {
                vec![crate::model::nodes::table::DEFAULT_CELL_WIDTH; cells.len()]
            }
        } else {
            vec![crate::model::nodes::table::DEFAULT_CELL_WIDTH; cells.len()]
        };

        let mut cell_layouts = Vec::new();
        let mut max_height: f32 = 0.0;

        for (col_idx, cell) in cells.iter().enumerate() {
            let cell_width = col_widths
                .get(col_idx)
                .copied()
                .unwrap_or(crate::model::nodes::table::DEFAULT_CELL_WIDTH);

            let cell_constraints = BoxConstraints::new(cell_width, cell_width, 0.0, f32::MAX);
            let cell_layout = ctx.layout(cell, cell_constraints);
            max_height = max_height.max(cell_layout.size.height);
            cell_layouts.push((cell_layout, cell_width));
        }

        let mut children = Vec::new();
        let mut x = TABLE_BORDER_WIDTH;

        for (cell_layout, cell_width) in cell_layouts {
            let extended_cell = if cell_layout.size.height < max_height {
                let new_size = Size::new(cell_layout.size.width, max_height);
                let element = if let Some(Element::TableCell(cell_elem)) = &cell_layout.element {
                    Some(Element::TableCell(TableCellElement::new(
                        new_size,
                        cell_elem.node_id,
                    )))
                } else {
                    cell_layout.element.clone()
                };

                std::rc::Rc::new(LayoutNode {
                    size: new_size,
                    element,
                    children: cell_layout.children.clone(),
                    page_break_policy: cell_layout.page_break_policy,
                    render_hints: cell_layout.render_hints.clone(),
                    scope_id: cell_layout.scope_id,
                })
            } else {
                cell_layout
            };

            children.push(PositionedNode {
                position: Point::new(x, 0.0),
                node: extended_cell,
            });

            x += cell_width + TABLE_BORDER_WIDTH;
        }

        let actual_row_width = x;

        LayoutNode {
            size: Size::new(actual_row_width, max_height + TABLE_BORDER_WIDTH),
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

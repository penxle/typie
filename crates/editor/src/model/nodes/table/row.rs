use crate::layout::{Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use tsify::Tsify;

use super::{TABLE_BORDER_WIDTH, calculate_col_widths};

#[derive(Debug, Clone, Default, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
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
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;

        let cells: Vec<_> = ctx.node.children().collect();
        if cells.is_empty() {
            return LayoutNode {
                size: Size::new(max_width, 0.0),
                element: None,
                children: None,
                page_break_policy: PageBreakPolicy::Avoid,
                render_hints: Default::default(),
                scope_id: None,
            };
        }

        let col_count = cells.len();
        let default_col_widths = calculate_col_widths(max_width, col_count, None);
        let default_col_width = default_col_widths.first().copied().unwrap_or(0.0);

        let mut cell_layouts = Vec::new();
        let mut max_height: f32 = 0.0;

        for cell in &cells {
            let cell_width = if let Node::TableCell(cell_node) = cell.node() {
                cell_node.col_width.unwrap_or(default_col_width)
            } else {
                default_col_width
            };

            let cell_constraints = BoxConstraints::new(cell_width, cell_width, 0.0, f32::MAX);
            let cell_layout = ctx.layout(cell, cell_constraints);
            max_height = max_height.max(cell_layout.size.height);
            cell_layouts.push((cell_layout, cell_width));
        }

        let mut children = Vec::new();
        let mut x = TABLE_BORDER_WIDTH;

        for (cell_layout, cell_width) in cell_layouts {
            let extended_cell = if cell_layout.size.height < max_height {
                let inner_cell = std::rc::Rc::new(LayoutNode {
                    size: cell_layout.size,
                    element: cell_layout.element.clone(),
                    children: cell_layout.children.clone(),
                    page_break_policy: cell_layout.page_break_policy,
                    render_hints: cell_layout.render_hints.clone(),
                    scope_id: None,
                });
                std::rc::Rc::new(LayoutNode {
                    size: Size::new(cell_layout.size.width, max_height),
                    element: None,
                    children: Some(vec![PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: inner_cell,
                    }]),
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

        LayoutNode {
            size: Size::new(max_width, max_height + TABLE_BORDER_WIDTH),
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Avoid,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

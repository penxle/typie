use crate::layout::elements::TableBorderElement;
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::rc::Rc;
use tsify::Tsify;

use super::{TABLE_BORDER_WIDTH, calculate_col_widths};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Codec, Tsify,
)]
#[serde(rename_all = "snake_case")]
pub enum TableBorderStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    None,
}

impl TableBorderStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            TableBorderStyle::Solid => "solid",
            TableBorderStyle::Dashed => "dashed",
            TableBorderStyle::Dotted => "dotted",
            TableBorderStyle::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dashed" => TableBorderStyle::Dashed,
            "dotted" => TableBorderStyle::Dotted,
            "none" => TableBorderStyle::None,
            _ => TableBorderStyle::Solid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, Codec, Tsify)]
pub struct TableNode {
    #[serde(default)]
    pub border_style: TableBorderStyle,
}

impl Default for TableNode {
    fn default() -> Self {
        Self {
            border_style: TableBorderStyle::Solid,
        }
    }
}

impl NodeHtmlCodec for TableNode {
    fn to_dom(&self) -> Option<DomSpec> {
        Some(
            DomSpec::el("table")
                .attr("data-border-style", self.border_style.as_str())
                .hole(),
        )
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("table", |elem| {
            let border_style = elem
                .value()
                .attr("data-border-style")
                .map(TableBorderStyle::from_str)
                .unwrap_or_default();
            Some(Node::Table(TableNode { border_style }))
        })]
    }
}

impl Layout for TableNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        let max_width = constraints.max_width;
        let node_id = ctx.node.node_id();

        let rows: Vec<_> = ctx.node.children().collect();
        if rows.is_empty() {
            return LayoutNode {
                size: Size::new(max_width, 0.0),
                element: None,
                children: None,
                page_break_policy: PageBreakPolicy::Auto,
                render_hints: Default::default(),
                scope_id: None,
            };
        }

        let col_count = rows.first().map(|row| row.children().count()).unwrap_or(0);

        if col_count == 0 {
            return LayoutNode {
                size: Size::new(max_width, 0.0),
                element: None,
                children: None,
                page_break_policy: PageBreakPolicy::Auto,
                render_hints: Default::default(),
                scope_id: None,
            };
        }

        let custom_widths: Option<Vec<f32>> = rows.first().and_then(|first_row| {
            let widths: Vec<Option<f32>> = first_row
                .children()
                .map(|cell| {
                    if let Node::TableCell(cell_node) = cell.node() {
                        cell_node.col_width
                    } else {
                        None
                    }
                })
                .collect();

            if widths.iter().all(|w| w.is_some()) {
                Some(widths.into_iter().map(|w| w.unwrap()).collect())
            } else {
                None
            }
        });

        let col_widths = calculate_col_widths(col_count, custom_widths.as_deref());
        let actual_table_width =
            col_widths.iter().sum::<f32>() + TABLE_BORDER_WIDTH * (col_count as f32 + 1.0);

        let mut children = Vec::new();
        let mut row_heights = Vec::new();
        let mut y = TABLE_BORDER_WIDTH;

        for row in &rows {
            let row_constraints =
                BoxConstraints::new(actual_table_width, actual_table_width, 0.0, f32::MAX);
            let row_layout = ctx.layout(row, row_constraints);
            let row_height = row_layout.size.height;

            children.push(PositionedNode {
                position: Point::new(0.0, y),
                node: row_layout.clone(),
            });

            row_heights.push(row_height);
            y += row_height;
        }

        y += TABLE_BORDER_WIDTH;

        let table_size = Size::new(actual_table_width, y);

        let border_element = TableBorderElement::new(
            table_size,
            node_id,
            self.border_style,
            rows.len(),
            col_count,
            row_heights,
            col_widths,
        );

        let border_node = LayoutNode {
            size: table_size,
            element: Some(Element::TableBorder(border_element)),
            children: None,
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        };

        children.insert(
            0,
            PositionedNode {
                position: Point::new(0.0, 0.0),
                node: Rc::new(border_node),
            },
        );

        LayoutNode {
            size: table_size,
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

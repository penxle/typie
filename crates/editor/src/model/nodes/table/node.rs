use crate::layout::elements::{SplitEdges, TableBorderElement};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::Node;
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule};
use crate::model::nodes::table::{TABLE_BORDER_WIDTH, TableWidthModel};
use crate::types::{BoxConstraints, Point, Size};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl std::fmt::Display for TableAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TableAlign::Left => "left",
            TableAlign::Center => "center",
            TableAlign::Right => "right",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for TableAlign {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "center" => TableAlign::Center,
            "right" => TableAlign::Right,
            _ => TableAlign::Left,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TableNode {
    #[serde(default)]
    pub border_style: TableBorderStyle,
    #[serde(default)]
    pub align: TableAlign,
    #[serde(default = "default_proportion")]
    pub proportion: f32,
}

fn default_proportion() -> f32 {
    1.0
}

impl Default for TableNode {
    fn default() -> Self {
        Self {
            border_style: TableBorderStyle::Solid,
            align: TableAlign::Left,
            proportion: default_proportion(),
        }
    }
}

impl Hash for TableNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.border_style.hash(state);
        self.align.hash(state);
        self.proportion.to_bits().hash(state);
    }
}

impl NodeHtmlCodec for TableNode {
    fn to_dom(&self) -> Option<DomSpec> {
        let mut builder =
            DomSpec::el("table").attr("data-border-style", self.border_style.as_str());

        if self.align != TableAlign::Left {
            match self.align {
                TableAlign::Center => {
                    builder = builder.attr("data-align", "center");
                }
                TableAlign::Right => {
                    builder = builder.attr("data-align", "right");
                }
                _ => {}
            }
        }

        builder = builder.attr("data-proportion", self.proportion.to_string());

        Some(builder.hole())
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![NodeParseRule::simple("table", |elem| {
            let border_style = elem
                .value()
                .attr("data-border-style")
                .map(TableBorderStyle::from_str)
                .unwrap_or_default();

            let align = match elem.value().attr("data-align") {
                Some("center") => TableAlign::Center,
                Some("right") => TableAlign::Right,
                _ => TableAlign::Left,
            };
            let proportion = elem
                .value()
                .attr("data-proportion")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                .unwrap_or_else(default_proportion);

            Some(Node::Table(TableNode {
                border_style,
                align,
                proportion,
            }))
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

        let table_proportion = self.proportion.clamp(0.0, 1.0);
        let width_model = TableWidthModel::new(col_count, max_width);
        let table_width_floor = width_model.min_table_width().min(max_width.max(0.0));
        let table_width_constraint = width_model
            .target_table_width(table_proportion)
            .max(table_width_floor);
        let table_inner_width = width_model.inner_width_from_table_width(table_width_constraint);
        let col_widths =
            width_model.calculate_col_widths(custom_widths.as_deref(), table_inner_width);
        let actual_table_width = col_widths.iter().sum::<f32>() + width_model.border_width();

        let x_offset = if actual_table_width < max_width {
            match self.align {
                TableAlign::Center => (max_width - actual_table_width) / 2.0,
                TableAlign::Right => max_width - actual_table_width,
                TableAlign::Left => 0.0,
            }
        } else {
            0.0
        };

        let mut children = Vec::new();
        let mut row_heights = Vec::new();
        let mut y = TABLE_BORDER_WIDTH;

        for row in &rows {
            let row_constraints = BoxConstraints::new(
                table_width_constraint,
                table_width_constraint,
                0.0,
                f32::MAX,
            );
            let row_layout = ctx.layout(row, row_constraints);
            let row_height = row_layout.size.height;

            children.push(PositionedNode {
                position: Point::new(x_offset, y),
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
            self.align,
            rows.len(),
            col_count,
            row_heights,
            col_widths,
            SplitEdges::default(),
            0.0,
            x_offset,
            0,
            rows.len(),
        );

        LayoutNode {
            size: Size::new(max_width, y),
            element: Some(Element::TableBorder(border_element)),
            children: Some(children),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

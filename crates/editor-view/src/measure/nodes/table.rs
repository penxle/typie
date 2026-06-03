use std::sync::Arc;

use editor_common::EdgeInsets;

use crate::style::Alignment as LayoutAlignment;
use editor_model::{Alignment, Doc, Modifier, Node, NodeRef};

use crate::TableLayoutInfo;
use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::{MeasuredBox, MeasuredContent, MeasuredNode, PageBreakPolicy};
use crate::style::{BorderMode, BoxStyle, Direction};
use crate::view_state::ViewState;

const TABLE_BORDER_WIDTH: f32 = 1.0;
const TABLE_CELL_PADDING: f32 = 8.0;
const MIN_CELL_WIDTH: f32 = 40.0;

fn border_width(col_count: usize) -> f32 {
    (col_count + 1) as f32 * TABLE_BORDER_WIDTH
}

fn min_table_width(col_count: usize) -> f32 {
    if col_count == 0 {
        return border_width(0);
    }
    col_count as f32 * MIN_CELL_WIDTH + border_width(col_count)
}

/// Distributes available width across columns using ratio-based constraints.
fn calculate_col_widths(
    col_count: usize,
    custom_widths: Option<&[f32]>,
    available_width: f32,
) -> Vec<f32> {
    if col_count == 0 {
        return vec![];
    }

    // Use equal ratios when no custom widths are provided
    let ratios: Vec<f32> = match custom_widths {
        Some(cw) => cw
            .iter()
            .map(|&w| if w.is_finite() && w >= 0.0 { w } else { 0.0 })
            .collect(),
        None => vec![1.0 / col_count as f32; col_count],
    };

    // All columns at minimum when space is too tight
    let min_total = col_count as f32 * MIN_CELL_WIDTH;
    if available_width <= min_total {
        return vec![MIN_CELL_WIDTH; col_count];
    }

    let ratio_sum: f32 = ratios.iter().sum();
    if ratio_sum <= 1e-7 {
        let each = available_width / col_count as f32;
        return vec![each; col_count];
    }

    let mut widths = vec![MIN_CELL_WIDTH; col_count];

    // Sort by ratio ascending so smallest columns are constrained first
    let mut indexed_ratios: Vec<(usize, f32)> = ratios
        .iter()
        .enumerate()
        .filter(|(_, r)| **r > 0.0)
        .map(|(i, &r)| (i, r))
        .collect();
    indexed_ratios.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    if indexed_ratios.is_empty() {
        let each = available_width / col_count as f32;
        return vec![each; col_count];
    }

    let constrained_count = col_count - indexed_ratios.len();
    let mut remaining_width = available_width - constrained_count as f32 * MIN_CELL_WIDTH;
    let mut unconstrained_ratio_sum = ratio_sum;

    // Greedily constrain columns whose proportional share falls below MIN_CELL_WIDTH
    let mut constrained_end = 0;
    for (pos, &(_, ratio)) in indexed_ratios.iter().enumerate() {
        let scale = remaining_width / unconstrained_ratio_sum;
        if scale * ratio >= MIN_CELL_WIDTH {
            break;
        }
        remaining_width -= MIN_CELL_WIDTH;
        unconstrained_ratio_sum -= ratio;
        constrained_end = pos + 1;
    }

    // Distribute remaining width proportionally among unconstrained columns
    if unconstrained_ratio_sum > 1e-7 {
        let scale = remaining_width / unconstrained_ratio_sum;
        for &(idx, ratio) in &indexed_ratios[constrained_end..] {
            widths[idx] = scale * ratio;
        }
    }

    // Adjust last unconstrained column to absorb floating-point rounding error
    let total: f32 = widths.iter().sum();
    let diff = available_width - total;
    let tolerance = (1e-6 * available_width).max(1e-4);
    if diff.abs() > tolerance
        && let Some(&(last_idx, _)) = indexed_ratios.last()
        && (diff > 0.0 || widths[last_idx] + diff >= MIN_CELL_WIDTH)
    {
        widths[last_idx] += diff;
    }

    widths
}

pub fn measure_table_cell(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    layout_padded(
        measurer,
        doc,
        node,
        width,
        view_state,
        PaddedLayoutConfig {
            padding: EdgeInsets::all(TABLE_CELL_PADDING),
            border: EdgeInsets::all(TABLE_BORDER_WIDTH),
            scope: true,
            alignment: LayoutAlignment::Start,
            page_break_policy: PageBreakPolicy::Avoid,
        },
    )
}

pub fn measure_table(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let Node::Table(table_node) = node.node() else {
        unreachable!()
    };

    let rows: Vec<NodeRef<'_>> = node.children().collect();

    if rows.is_empty() {
        return MeasuredNode {
            width,
            height: 0.0,
            content: MeasuredContent::Box(MeasuredBox {
                node_id: node.id(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                    border_mode: BorderMode::Collapse,
                    alignment: LayoutAlignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: node.spec().monolithic,
                },
                table_info: None,
                children: vec![],
                page_break_policy: PageBreakPolicy::Auto,
            }),
        };
    }

    let first_row_cells: Vec<NodeRef<'_>> = rows[0].children().collect();
    let col_count = first_row_cells.len();

    // Extract custom widths: all Some -> use, any None -> None
    let custom_widths: Option<Vec<f32>> = {
        let mut widths = Vec::with_capacity(col_count);
        let mut all_some = true;
        for cell in &first_row_cells {
            if let Node::TableCell(tc) = cell.node() {
                if let Some(w) = *tc.col_width.get() {
                    widths.push(w as f32);
                } else {
                    all_some = false;
                    break;
                }
            } else {
                all_some = false;
                break;
            }
        }
        if all_some { Some(widths) } else { None }
    };

    let proportion = (*table_node.proportion.get() as f32 / 100.0).clamp(0.0, 1.0);
    let target_width = proportion * width;
    let floor = min_table_width(col_count).min(width);
    let table_width = target_width.max(floor);

    // Available width for cell content (excluding collapsed borders)
    let available_width = table_width - (col_count + 1) as f32 * TABLE_BORDER_WIDTH;
    let col_widths = calculate_col_widths(col_count, custom_widths.as_deref(), available_width);

    let actual_table_width =
        (col_count + 1) as f32 * TABLE_BORDER_WIDTH + col_widths.iter().sum::<f32>();

    let mut row_measurements: Vec<Arc<MeasuredNode>> = Vec::with_capacity(rows.len());

    for row in &rows {
        let cells: Vec<NodeRef<'_>> = row.children().collect();

        // 1st pass: measure each cell and collect heights
        let mut cell_measurements: Vec<Arc<MeasuredNode>> = Vec::with_capacity(cells.len());

        for (i, cell) in cells.iter().enumerate() {
            let col_width = col_widths.get(i).copied().unwrap_or(col_widths[0]);
            let cell_full_width = col_width + 2.0 * TABLE_BORDER_WIDTH;
            let m = measurer.measure(doc, cell.id(), cell_full_width, view_state);
            cell_measurements.push(m);
        }

        let max_height = cell_measurements
            .iter()
            .map(|m| m.height)
            .fold(0.0_f32, f32::max);

        // 2nd pass: adjust cells to max_height
        let row_children: Vec<Arc<MeasuredNode>> = cell_measurements
            .into_iter()
            .map(|m| {
                if (m.height - max_height).abs() > f32::EPSILON {
                    let mut adjusted = (*m).clone();
                    adjusted.height = max_height;
                    Arc::new(adjusted)
                } else {
                    m
                }
            })
            .collect();

        let row_node = MeasuredNode {
            width: actual_table_width,
            height: max_height,
            content: MeasuredContent::Box(MeasuredBox {
                node_id: row.id(),
                style: BoxStyle {
                    direction: Direction::Horizontal,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                    border_mode: BorderMode::Collapse,
                    alignment: LayoutAlignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: row.spec().monolithic,
                },
                table_info: None,
                children: row_children,
                page_break_policy: PageBreakPolicy::Avoid,
            }),
        };

        row_measurements.push(Arc::new(row_node));
    }

    let row_count = row_measurements.len();
    let table_row_inner_heights: Vec<f32> = row_measurements
        .iter()
        .map(|rm| (rm.height - 2.0 * TABLE_BORDER_WIDTH).max(0.0))
        .collect();
    let row_inner_heights_sum: f32 = table_row_inner_heights.iter().sum();
    let collapsed_height = (row_count + 1) as f32 * TABLE_BORDER_WIDTH + row_inner_heights_sum;

    let align = node
        .modifiers_with_style()
        .find_map(|m| match m {
            Modifier::Alignment { value } => Some(*value),
            _ => None,
        })
        .unwrap_or_default();

    let alignment = match align {
        Alignment::Left => LayoutAlignment::Start,
        Alignment::Center => LayoutAlignment::Center,
        Alignment::Right | Alignment::Justify => LayoutAlignment::End,
    };

    MeasuredNode {
        width: actual_table_width,
        height: collapsed_height,
        content: MeasuredContent::Box(MeasuredBox {
            node_id: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                border_mode: BorderMode::Collapse,
                alignment,
                scope: false,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            table_info: Some(Box::new(TableLayoutInfo {
                col_inner_widths: col_widths,
                row_inner_heights: table_row_inner_heights,
            })),
            children: row_measurements,
            page_break_policy: PageBreakPolicy::Auto,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn table_cell_has_padding_border_scope() {
        let (doc, c1) = doc! {
            root {
                table {
                    table_row {
                        c1: table_cell {
                            paragraph { text("Hello") }
                        }
                    }
                }
            }
        };

        let node = doc.node(c1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table_cell(&mut measurer, &doc, &node, 100.0, &ViewState::new());

        let MeasuredContent::Box(MeasuredBox { style, .. }) = &result.content else {
            panic!()
        };
        assert_eq!(style.padding.left, TABLE_CELL_PADDING);
        assert_eq!(style.padding.top, TABLE_CELL_PADDING);
        assert_eq!(style.border.left, TABLE_BORDER_WIDTH);
        assert_eq!(style.border.top, TABLE_BORDER_WIDTH);
        assert!(style.scope);
    }

    #[test]
    fn table_2x2_structure() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row {
                        table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Box(MeasuredBox {
            children, style, ..
        }) = &result.content
        else {
            panic!()
        };
        assert_eq!(children.len(), 2);
        assert_eq!(style.border_mode, BorderMode::Collapse);
        assert_eq!(style.direction, Direction::Vertical);
        assert_eq!(style.border.top, TABLE_BORDER_WIDTH);

        let MeasuredContent::Box(MeasuredBox {
            children: row_cells,
            style: row_style,
            ..
        }) = &children[0].content
        else {
            panic!()
        };
        assert_eq!(row_cells.len(), 2);
        assert_eq!(row_style.direction, Direction::Horizontal);
        assert_eq!(row_style.border_mode, BorderMode::Collapse);
    }

    #[test]
    fn table_align_center() {
        let (doc, t1) = doc! {
            root {
                t1: table [alignment(Alignment::Center)] {
                    table_row {
                        table_cell { paragraph }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Box(MeasuredBox { style, .. }) = &result.content else {
            panic!()
        };
        assert_eq!(style.alignment, LayoutAlignment::Center);
    }

    #[test]
    fn empty_table_returns_zero_height() {
        let (doc, t1) = doc! {
            root {
                t1: table
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn equal_col_distribution() {
        let widths = calculate_col_widths(3, None, 300.0);
        assert_eq!(widths, [100.0, 100.0, 100.0]);
    }

    #[test]
    fn border_width_formula() {
        assert_eq!(border_width(0), 1.0);
        assert_eq!(border_width(2), 3.0);
        assert_eq!(border_width(3), 4.0);
    }

    #[test]
    fn min_table_width_formula() {
        assert_eq!(min_table_width(0), 1.0);
        assert_eq!(min_table_width(2), 83.0);
    }

    #[test]
    fn custom_ratio_col_widths() {
        let widths = calculate_col_widths(2, Some(&[0.4, 0.6]), 500.0);
        assert_eq!(widths, [200.0, 300.0]);
    }

    #[test]
    fn min_col_width_enforcement() {
        let widths = calculate_col_widths(2, None, 60.0);
        assert_eq!(widths, [MIN_CELL_WIDTH, MIN_CELL_WIDTH]);
    }

    #[test]
    fn small_ratio_gets_min_col_width() {
        let widths = calculate_col_widths(2, Some(&[0.05, 0.95]), 500.0);
        assert_eq!(widths[0], MIN_CELL_WIDTH);
        assert_eq!(widths[0] + widths[1], 500.0);
    }

    #[test]
    fn zero_columns() {
        let widths = calculate_col_widths(0, None, 500.0);
        assert!(widths.is_empty());
    }

    #[test]
    fn table_col_inner_widths_len_matches_col_count() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row {
                        table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                        table_cell { paragraph { text("C") } }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };
        let info = b.table_info.as_ref().expect("table_info must be set");
        assert_eq!(info.col_inner_widths.len(), 3);
    }

    #[test]
    fn table_row_inner_heights_len_matches_row_count() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row { table_cell { paragraph { text("A") } } }
                    table_row { table_cell { paragraph { text("B") } } }
                    table_row { table_cell { paragraph { text("C") } } }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };
        let info = b.table_info.as_ref().expect("table_info must be set");
        assert_eq!(info.row_inner_heights.len(), 3);
    }

    #[test]
    fn table_col_inner_widths_sum_equals_available_width() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row {
                        table_cell { paragraph }
                        table_cell { paragraph }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_table(&mut measurer, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };
        let info = b.table_info.as_ref().expect("table_info must be set");
        let col_count = info.col_inner_widths.len();
        assert_eq!(col_count, 2);
        let expected_available = result.width - (col_count + 1) as f32 * TABLE_BORDER_WIDTH;
        let actual_sum: f32 = info.col_inner_widths.iter().sum();
        assert!((actual_sum - expected_available).abs() < 0.01);
    }
}

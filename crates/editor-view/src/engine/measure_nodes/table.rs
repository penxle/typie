use editor_common::{Alignment, EdgeInsets, Size};
use editor_model::{Doc, Node, NodeId, NodeRef, TableAlign};
use std::sync::Arc;

use super::super::LayoutEngine;
use super::super::resolve::resolve_gap_after;
use super::container::measure_padded_container;
use crate::measure::*;
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

/// Ratio-based column width calculation.
/// Algorithm (ported from legacy TableWidthModel::calculate_col_widths):
/// 1. If custom_widths is None, use equal ratios (1/col_count)
/// 2. If available_width <= col_count * MIN_CELL_WIDTH, return all MIN_CELL_WIDTH
/// 3. Sort columns by ratio ascending
/// 4. From smallest: if scale * ratio < MIN_CELL_WIDTH, constrain to MIN
/// 5. Remaining columns get: scale * ratio where scale = remaining_width / unconstrained_ratio_sum
/// 6. Float correction: adjust last column for rounding error
fn calculate_col_widths(
    col_count: usize,
    custom_widths: Option<&[f32]>,
    available_width: f32,
) -> Vec<f32> {
    if col_count == 0 {
        return vec![];
    }

    let ratios: Vec<f32> = match custom_widths {
        Some(cw) => cw
            .iter()
            .map(|&w| if w.is_finite() && w >= 0.0 { w } else { 0.0 })
            .collect(),
        None => vec![1.0 / col_count as f32; col_count],
    };

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

    if unconstrained_ratio_sum > 1e-7 {
        let scale = remaining_width / unconstrained_ratio_sum;
        for &(idx, ratio) in &indexed_ratios[constrained_end..] {
            widths[idx] = scale * ratio;
        }
    }

    let total: f32 = widths.iter().sum();
    let diff = available_width - total;
    let tolerance = (1e-6 * available_width).max(1e-4);
    if diff.abs() > tolerance {
        if let Some(&(last_idx, _)) = indexed_ratios.last() {
            if diff > 0.0 || widths[last_idx] + diff >= MIN_CELL_WIDTH {
                widths[last_idx] += diff;
            }
        }
    }

    widths
}

pub fn measure_table_cell(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    measure_padded_container(
        engine,
        doc,
        node,
        width,
        view_state,
        EdgeInsets::all(TABLE_CELL_PADDING),
        EdgeInsets::all(TABLE_BORDER_WIDTH),
        true,
        Alignment::Start,
    )
}

pub fn measure_table(
    engine: &mut LayoutEngine,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> Measurement {
    let Node::Table(table_node) = node.node() else {
        unreachable!()
    };

    let rows: Vec<NodeRef<'_>> = node.children().collect();

    if rows.is_empty() {
        return Measurement {
            size: Size { width, height: 0.0 },
            gap_after: 0.0,
            content: MeasuredContent::Container(ContainerContent {
                children: vec![],
                scope: false,
                direction: LayoutDirection::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                border_mode: BorderMode::Collapse,
                placeholders: vec![],
            }),
            alignment: Alignment::Start,
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
                if let Some(w) = tc.col_width {
                    widths.push(w);
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

    let proportion = table_node.proportion.clamp(0.0, 1.0);
    let target_width = proportion * width;
    let floor = min_table_width(col_count).min(width);
    let table_width = target_width.max(floor);

    // Available width for cell content (excluding collapsed borders)
    let available_width = table_width - (col_count + 1) as f32 * TABLE_BORDER_WIDTH;
    let col_widths = calculate_col_widths(col_count, custom_widths.as_deref(), available_width);

    let actual_table_width =
        (col_count + 1) as f32 * TABLE_BORDER_WIDTH + col_widths.iter().sum::<f32>();

    // Measure rows
    let mut row_measurements = Vec::with_capacity(rows.len());

    for row in &rows {
        let cells: Vec<NodeRef<'_>> = row.children().collect();

        // 1st pass: measure each cell and collect heights
        let mut cell_measurements: Vec<(NodeId, Arc<Measurement>)> =
            Vec::with_capacity(cells.len());

        for (i, cell) in cells.iter().enumerate() {
            let col_width = col_widths.get(i).copied().unwrap_or(col_widths[0]);
            let cell_full_width = col_width + 2.0 * TABLE_BORDER_WIDTH;
            let cell_id = cell.id();
            let m = engine.measure(doc, cell_id, cell_full_width, view_state);
            cell_measurements.push((cell_id, m));
        }

        let max_height = cell_measurements
            .iter()
            .map(|(_, m)| m.size.height)
            .fold(0.0_f32, f32::max);

        // 2nd pass: adjust cells to max_height
        let row_children: Vec<ChildMeasurement> = cell_measurements
            .into_iter()
            .map(|(cell_id, m)| {
                let measurement = if (m.size.height - max_height).abs() > f32::EPSILON {
                    let mut adjusted = (*m).clone();
                    adjusted.size.height = max_height;
                    Arc::new(adjusted)
                } else {
                    m
                };
                ChildMeasurement {
                    node_id: cell_id,
                    measurement,
                }
            })
            .collect();

        let row_measurement = Measurement {
            size: Size {
                width: actual_table_width,
                height: max_height,
            },
            gap_after: 0.0,
            content: MeasuredContent::Container(ContainerContent {
                children: row_children,
                scope: false,
                direction: LayoutDirection::Horizontal,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                border_mode: BorderMode::Collapse,
                placeholders: vec![],
            }),
            alignment: Alignment::Start,
        };

        row_measurements.push(ChildMeasurement {
            node_id: row.id(),
            measurement: Arc::new(row_measurement),
        });
    }

    // Calculate collapsed height
    let row_count = row_measurements.len();
    let row_inner_heights_sum: f32 = row_measurements
        .iter()
        .map(|rm| {
            // row_inner_height = max_cell_height - 2 * TABLE_BORDER_WIDTH
            rm.measurement.size.height - 2.0 * TABLE_BORDER_WIDTH
        })
        .sum();
    let collapsed_height = (row_count + 1) as f32 * TABLE_BORDER_WIDTH + row_inner_heights_sum;

    let alignment = match table_node.align {
        TableAlign::Left => Alignment::Start,
        TableAlign::Center => Alignment::Center,
        TableAlign::Right => Alignment::End,
    };

    Measurement {
        size: Size {
            width: actual_table_width,
            height: collapsed_height,
        },
        gap_after: resolve_gap_after(node),
        content: MeasuredContent::Container(ContainerContent {
            children: row_measurements,
            scope: false,
            direction: LayoutDirection::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::all(TABLE_BORDER_WIDTH),
            border_mode: BorderMode::Collapse,
            placeholders: vec![],
        }),
        alignment,
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
        let mut engine = LayoutEngine::new_test();
        let result = measure_table_cell(&mut engine, &doc, &node, 100.0, &ViewState::new());

        let MeasuredContent::Container(ContainerContent {
            padding,
            border,
            scope,
            ..
        }) = &result.content
        else {
            panic!()
        };
        assert_eq!(padding.left, TABLE_CELL_PADDING);
        assert_eq!(padding.top, TABLE_CELL_PADDING);
        assert_eq!(border.left, TABLE_BORDER_WIDTH);
        assert_eq!(border.top, TABLE_BORDER_WIDTH);
        assert!(*scope);
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
        let mut engine = LayoutEngine::new_test();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        let MeasuredContent::Container(ContainerContent {
            children,
            border,
            border_mode,
            direction,
            ..
        }) = &result.content
        else {
            panic!()
        };
        assert_eq!(children.len(), 2);
        assert_eq!(*border_mode, BorderMode::Collapse);
        assert_eq!(*direction, LayoutDirection::Vertical);
        assert_eq!(border.top, TABLE_BORDER_WIDTH);

        let MeasuredContent::Container(ContainerContent {
            children: row_cells,
            direction: row_dir,
            border_mode: row_bm,
            ..
        }) = &children[0].measurement.content
        else {
            panic!()
        };
        assert_eq!(row_cells.len(), 2);
        assert_eq!(*row_dir, LayoutDirection::Horizontal);
        assert_eq!(*row_bm, BorderMode::Collapse);
    }

    #[test]
    fn table_align_center() {
        let (doc, t1) = doc! {
            root {
                t1: table(align: TableAlign::Center) {
                    table_row {
                        table_cell { paragraph }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        assert_eq!(result.alignment, Alignment::Center);
    }

    #[test]
    fn empty_table_returns_zero_height() {
        let (doc, t1) = doc! {
            root {
                t1: table
            }
        };

        let node = doc.node(t1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        assert_eq!(result.size.height, 0.0);
    }

    #[test]
    fn table_custom_col_widths() {
        let (doc, t1) = doc! {
            root {
                t1: table {
                    table_row {
                        table_cell(col_width: Some(0.3)) { paragraph }
                        table_cell(col_width: Some(0.7)) { paragraph }
                    }
                }
            }
        };

        let node = doc.node(t1).unwrap();
        let mut engine = LayoutEngine::new_test();
        let result = measure_table(&mut engine, &doc, &node, 500.0, &ViewState::new());

        // inner_width = 500 - border_width(2) = 497
        // col_widths = [0.3 * 497, 0.7 * 497] = [149.1, 347.9]
        // cell full width = col_width + 2 * TABLE_BORDER_WIDTH
        let MeasuredContent::Container(ContainerContent { children, .. }) = &result.content else {
            panic!()
        };
        let MeasuredContent::Container(ContainerContent {
            children: cells, ..
        }) = &children[0].measurement.content
        else {
            panic!()
        };
        assert_eq!(cells[0].measurement.size.width, 0.3 * 497.0 + 2.0);
        assert_eq!(cells[1].measurement.size.width, 0.7 * 497.0 + 2.0);
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
    fn equal_col_distribution() {
        let widths = calculate_col_widths(3, None, 300.0);
        assert_eq!(widths, [100.0, 100.0, 100.0]);
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
}

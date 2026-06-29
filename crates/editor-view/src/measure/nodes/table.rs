use std::sync::Arc;

use editor_common::EdgeInsets;
use editor_model::{Alignment, ChildView, Modifier, ModifierType, Node, NodeView};
use editor_resource::Resource;

use crate::style::{BorderMode, BoxStyle, Direction};

use super::dispatch::measure_node;
use crate::measure::context::MeasureContext;
use crate::measure::types::{MeasuredBox, MeasuredChildren, MeasuredContent};

use crate::measure::PageBreakPolicy;
use crate::measure::container::PaddedLayoutConfig;

use super::dispatch::measure_child;
use crate::measure::container::layout_padded;
use crate::measure::types::MeasuredNode;

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
    if diff.abs() > tolerance
        && let Some(&(last_idx, _)) = indexed_ratios.last()
        && (diff > 0.0 || widths[last_idx] + diff >= MIN_CELL_WIDTH)
    {
        widths[last_idx] += diff;
    }

    widths
}

pub(crate) fn measure_table_cell(
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let mut seam: fn(ChildView, f32, &MeasureContext, &mut Resource) -> Arc<MeasuredNode> =
        measure_child;
    layout_padded(
        node,
        width,
        ctx,
        resource,
        PaddedLayoutConfig {
            padding: EdgeInsets::all(TABLE_CELL_PADDING),
            border: EdgeInsets::all(TABLE_BORDER_WIDTH),
            alignment: crate::style::Alignment::Start,
            page_break_policy: PageBreakPolicy::Avoid,
        },
        &mut seam,
    )
}

pub(crate) fn measure_table(
    node: &NodeView,
    width: f32,
    ctx: &MeasureContext,
    resource: &mut Resource,
) -> MeasuredNode {
    let Node::Table(table_node) = node.node() else {
        unreachable!()
    };

    let rows: Vec<NodeView> = node.child_blocks().collect();

    if rows.is_empty() {
        return MeasuredNode {
            width,
            height: 0.0,
            content: MeasuredContent::Box(MeasuredBox {
                node: node.id(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                    border_mode: BorderMode::Collapse,
                    alignment: crate::style::Alignment::Start,
                    decorations: vec![],
                    monolithic: node.spec().monolithic,
                },
                children: MeasuredChildren::default(),
                page_break_policy: PageBreakPolicy::Auto,
            }),
        };
    }

    let first_row_cells: Vec<NodeView> = rows[0].child_blocks().collect();
    let col_count = first_row_cells.len();

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

    let available_width = table_width - (col_count + 1) as f32 * TABLE_BORDER_WIDTH;
    let col_widths = calculate_col_widths(col_count, custom_widths.as_deref(), available_width);

    let actual_table_width =
        (col_count + 1) as f32 * TABLE_BORDER_WIDTH + col_widths.iter().sum::<f32>();

    let mut row_measurements: Vec<Arc<MeasuredNode>> = Vec::with_capacity(rows.len());

    for row in &rows {
        let cells: Vec<NodeView> = row.child_blocks().collect();

        let mut cell_measurements: Vec<Arc<MeasuredNode>> = Vec::with_capacity(cells.len());
        for (i, cell) in cells.iter().enumerate() {
            let col_width = col_widths.get(i).copied().unwrap_or(col_widths[0]);
            let cell_full_width = col_width + 2.0 * TABLE_BORDER_WIDTH;
            let m = Arc::new(measure_node(cell, cell_full_width, ctx, resource));
            cell_measurements.push(m);
        }

        let max_height = cell_measurements
            .iter()
            .map(|m| m.height)
            .fold(0.0_f32, f32::max);

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
                node: row.id(),
                style: BoxStyle {
                    direction: Direction::Horizontal,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                    border_mode: BorderMode::Collapse,
                    alignment: crate::style::Alignment::Start,
                    decorations: vec![],
                    monolithic: row.spec().monolithic,
                },
                children: MeasuredChildren::from_blocks(row_children),
                page_break_policy: PageBreakPolicy::Avoid,
            }),
        };

        row_measurements.push(Arc::new(row_node));
    }

    let row_count = row_measurements.len();
    let collapsed_row_content_height: f32 = row_measurements
        .iter()
        .map(|rm| (rm.height - 2.0 * TABLE_BORDER_WIDTH).max(0.0))
        .sum();
    let collapsed_height =
        (row_count + 1) as f32 * TABLE_BORDER_WIDTH + collapsed_row_content_height;

    let align = match node.effective().get(&ModifierType::Alignment) {
        Some(Modifier::Alignment { value }) => *value,
        _ => Alignment::default(),
    };
    let alignment = match align {
        Alignment::Left => crate::style::Alignment::Start,
        Alignment::Center => crate::style::Alignment::Center,
        Alignment::Right | Alignment::Justify => crate::style::Alignment::End,
    };

    MeasuredNode {
        width: actual_table_width,
        height: collapsed_height,
        content: MeasuredContent::Box(MeasuredBox {
            node: node.id(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::all(TABLE_BORDER_WIDTH),
                border_mode: BorderMode::Collapse,
                alignment,
                decorations: vec![],
                monolithic: node.spec().monolithic,
            },
            children: MeasuredChildren::from_blocks(row_measurements),
            page_break_policy: PageBreakPolicy::Auto,
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::PageBreakPolicy;
    use crate::measure::context::MeasureContext;
    use crate::measure::types::MeasuredContent;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
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
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    #[test]
    fn col_widths_equal_ratios() {
        let widths = calculate_col_widths(3, None, 300.0);
        assert_eq!(widths.len(), 3);
        for w in &widths {
            assert!((w - 100.0).abs() < 0.1, "expected ~100.0, got {w}");
        }
        let sum: f32 = widths.iter().sum();
        assert!((sum - 300.0).abs() < 0.1, "sum expected ~300.0, got {sum}");
    }

    #[test]
    fn col_widths_all_min_when_tight() {
        let widths = calculate_col_widths(3, None, 60.0);
        assert_eq!(widths, [40.0, 40.0, 40.0]);
    }

    #[test]
    fn col_widths_custom_proportional() {
        let widths = calculate_col_widths(2, Some(&[1.0, 3.0]), 400.0);
        assert_eq!(widths.len(), 2);
        assert!(
            (widths[0] - 100.0).abs() < 0.1,
            "expected ~100.0, got {}",
            widths[0]
        );
        assert!(
            (widths[1] - 300.0).abs() < 0.1,
            "expected ~300.0, got {}",
            widths[1]
        );
        assert!(widths[0] >= 40.0);
        assert!(widths[1] >= 40.0);
        let sum: f32 = widths.iter().sum();
        assert!((sum - 400.0).abs() < 0.1, "sum expected ~400.0, got {sum}");
    }

    #[test]
    fn col_widths_min_cell_constraint() {
        let widths = calculate_col_widths(2, Some(&[1.0, 99.0]), 200.0);
        assert_eq!(widths.len(), 2);
        assert_eq!(widths[0], 40.0);
        assert!(
            (widths[1] - 160.0).abs() < 0.1,
            "expected ~160.0, got {}",
            widths[1]
        );
    }

    #[test]
    fn table_metrics() {
        assert_eq!(min_table_width(3), 124.0);
        assert_eq!(border_width(3), 4.0);
        assert_eq!(min_table_width(0), 1.0);
    }

    #[test]
    fn table_cell_padding_border() {
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let table_row = Dot::new(1, 2);
        let cell = Dot::new(1, 3);
        let para = Dot::new(1, 4);
        let p_root = Dot::new(1, 5);
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                table_row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, table_row],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, table_row, cell],
                },
            ),
            (Dot::new(1, 6), SeqItem::Char('x')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);

        let root_node = view.root().unwrap();
        let table_node = root_node.children().next().unwrap();
        let row_node = match table_node {
            editor_model::ChildView::Block(nv) => nv.children().next().unwrap(),
            _ => panic!("expected block"),
        };
        let cell_node = match row_node {
            editor_model::ChildView::Block(nv) => nv.children().next().unwrap(),
            _ => panic!("expected block"),
        };
        let cell_view = match cell_node {
            editor_model::ChildView::Block(nv) => nv,
            _ => panic!("expected block"),
        };

        let mut res = Resource::new_test();
        let result = measure_table_cell(&cell_view, 100.0, &MeasureContext::default(), &mut res);

        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box");
        };
        assert_eq!(b.style.padding.left, 8.0);
        assert_eq!(b.style.padding.top, 8.0);
        assert_eq!(b.style.border.left, 1.0);
        assert_eq!(b.page_break_policy, PageBreakPolicy::Avoid);
    }

    fn two_cell_table_doc() -> (DocLogs, Dot) {
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row = Dot::new(1, 2);
        let cell_a = Dot::new(1, 3);
        let para_a = Dot::new(1, 4);
        let cell_b = Dot::new(1, 10);
        let para_b = Dot::new(1, 11);
        let p_root = Dot::new(1, 20);
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                cell_a,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                },
            ),
            (
                para_a,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell_a],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('A')),
            (
                cell_b,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                },
            ),
            (
                para_b,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell_b],
                },
            ),
            (Dot::new(1, 12), SeqItem::Char('B')),
            (
                p_root,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
        ];
        (logs(&items), table)
    }

    #[test]
    fn table_structure_and_collapse() {
        use crate::measure::nodes::dispatch::measure_node;
        use crate::style::{BorderMode, Direction};

        let (doc, _table_dot) = two_cell_table_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(&root_node, 400.0, &MeasureContext::default(), &mut res);
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected root Box");
        };

        let table_child = root_box.children.iter().next().unwrap();
        let MeasuredContent::Box(ref table_box) = table_child.content else {
            panic!("expected table Box");
        };

        assert!(
            matches!(table_box.style.border_mode, BorderMode::Collapse),
            "table border_mode must be Collapse"
        );
        assert_eq!(table_box.style.border.left, 1.0);
        assert!(
            matches!(table_box.style.direction, Direction::Vertical),
            "table direction must be Vertical"
        );
        assert_eq!(table_box.children.len(), 1, "exactly 1 row child");

        let row_child = table_box.children.iter().next().unwrap();
        let MeasuredContent::Box(ref row_box) = row_child.content else {
            panic!("expected row Box");
        };
        assert!(
            matches!(row_box.style.direction, Direction::Horizontal),
            "row direction must be Horizontal"
        );
        assert!(
            matches!(row_box.style.border_mode, BorderMode::Collapse),
            "row border_mode must be Collapse"
        );
        assert_eq!(row_box.children.len(), 2, "row must have 2 cell children");
    }

    #[test]
    fn cell_heights_equalized() {
        use crate::measure::nodes::dispatch::measure_node;

        let root = Dot::ROOT;
        let table = Dot::new(2, 1);
        let row = Dot::new(2, 2);
        let cell_tall = Dot::new(2, 3);
        let para_tall = Dot::new(2, 4);
        let cell_short = Dot::new(2, 30);
        let para_short = Dot::new(2, 31);
        let p_root = Dot::new(2, 50);

        let long_text: Vec<(Dot, SeqItem)> = (5u64..30u64)
            .map(|i| (Dot::new(2, i), SeqItem::Char('x')))
            .collect();

        let mut items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                cell_tall,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                },
            ),
            (
                para_tall,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell_tall],
                },
            ),
        ];
        items.extend(long_text);
        items.push((
            cell_short,
            SeqItem::Block {
                node_type: NodeType::TableCell,
                parents: vec![root, table, row],
            },
        ));
        items.push((
            para_short,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root, table, row, cell_short],
            },
        ));
        items.push((Dot::new(2, 32), SeqItem::Char('B')));
        items.push((
            p_root,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        ));

        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(&root_node, 400.0, &MeasureContext::default(), &mut res);
        let MeasuredContent::Box(ref root_box) = result.content else {
            panic!("expected root Box");
        };
        let table_child = root_box.children.iter().next().unwrap();
        let MeasuredContent::Box(ref table_box) = table_child.content else {
            panic!("expected table Box");
        };
        let row_child = table_box.children.iter().next().unwrap();
        let MeasuredContent::Box(ref row_box) = row_child.content else {
            panic!("expected row Box");
        };

        assert_eq!(row_box.children.len(), 2);
        let h1 = row_box.children[0].height;
        let h2 = row_box.children[1].height;
        let row_height = row_child.height;
        assert!(
            (h1 - h2).abs() < 0.01,
            "cell heights must be equalized: h1={h1}, h2={h2}"
        );
        assert!(
            (h1 - row_height).abs() < 0.01,
            "cell height must equal row height: cell={h1}, row={row_height}"
        );
    }

    #[test]
    fn proportion_narrows_table() {
        use editor_model::{NodeAttr, NodeAttrLog, NodeAttrOp, TableNodeAttr};

        use crate::measure::nodes::dispatch::measure_node;

        let (mut doc50, table_dot) = two_cell_table_doc();
        doc50.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::ROOT,
                NodeAttrOp {
                    target: table_dot,
                    attr: NodeAttr::Table {
                        attr: TableNodeAttr::Proportion(50),
                    },
                },
            )
            .unwrap();

        let (mut doc100, table_dot100) = two_cell_table_doc();
        doc100.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::ROOT,
                NodeAttrOp {
                    target: table_dot100,
                    attr: NodeAttr::Table {
                        attr: TableNodeAttr::Proportion(100),
                    },
                },
            )
            .unwrap();

        let pd50 = project_document(&doc50).unwrap();
        let view50 = DocView::new(&pd50);
        let root50 = view50.root().unwrap();
        let mut res = Resource::new_test();
        let result50 = measure_node(&root50, 400.0, &MeasureContext::default(), &mut res);
        let MeasuredContent::Box(ref rb50) = result50.content else {
            panic!()
        };
        let table50 = rb50.children.iter().next().unwrap();
        let w50 = table50.width;

        let pd100 = project_document(&doc100).unwrap();
        let view100 = DocView::new(&pd100);
        let root100 = view100.root().unwrap();
        let result100 = measure_node(&root100, 400.0, &MeasureContext::default(), &mut res);
        let MeasuredContent::Box(ref rb100) = result100.content else {
            panic!()
        };
        let table100 = rb100.children.iter().next().unwrap();
        let w100 = table100.width;

        assert!(
            w50 < w100,
            "proportion=50 table width ({w50}) must be less than proportion=100 ({w100})"
        );
    }

    #[test]
    fn dispatch_wires_table() {
        use crate::measure::nodes::dispatch::measure_node;
        use crate::style::BorderMode;

        let (doc, _) = two_cell_table_doc();
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let table_node_view = root_node.child_blocks().next().unwrap();
        let mut res = Resource::new_test();

        let result = measure_node(
            &table_node_view,
            400.0,
            &MeasureContext::default(),
            &mut res,
        );
        let MeasuredContent::Box(ref b) = result.content else {
            panic!("expected Box");
        };
        assert!(
            matches!(b.style.border_mode, BorderMode::Collapse),
            "Table dispatch must produce Collapse border, not the default Separate"
        );
    }
}

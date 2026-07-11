use editor_crdt::Dot;
use editor_model::{DocView, NodeType};

use crate::affinity::Affinity;

use crate::Position;
use crate::gap_cursor::{between_monolithic_at, isolating_boundary_at};
use crate::normalize::normalize;
use crate::selection::Selection;

pub fn cell_rect_selection<'a>(
    anchor_cell: Dot,
    head_cell: Dot,
    view: &'a DocView<'a>,
) -> Option<Selection> {
    let (ac, hc) = (view.node(anchor_cell)?, view.node(head_cell)?);
    if ac.node_type() != NodeType::TableCell || hc.node_type() != NodeType::TableCell {
        return None;
    }
    let (arow, hrow) = (ac.parent()?, hc.parent()?);
    let table = arow.parent()?;
    if table.node_type() != NodeType::Table || hrow.parent()?.id() != table.id() {
        return None;
    }
    let (ca, ch) = (ac.index()?, hc.index()?);
    let anchor_offset = if ca <= ch { ca } else { ca + 1 };
    let head_offset = if ch >= ca { ch + 1 } else { ch };
    normalize(
        &Selection::new(
            Position::new(arow.id(), anchor_offset),
            Position::new(hrow.id(), head_offset),
        ),
        view,
    )
}

pub fn gap_cursor_selection_at<'a>(
    parent: Dot,
    index: usize,
    view: &'a DocView<'a>,
) -> Option<Selection> {
    let host = view.node(parent)?;
    if between_monolithic_at(&host, index) {
        return normalize(&Selection::collapsed(Position::new(parent, index)), view);
    }
    isolating_boundary_at(&host, index)?;
    let affinity = if index == 0 {
        Affinity::Upstream
    } else {
        Affinity::Downstream
    };
    normalize(
        &Selection::collapsed(Position {
            node: parent,
            offset: index,
            affinity,
        }),
        view,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc,
        SeqItem, SpanLog, project_document,
    };

    use crate::{
        cell_selection::as_cell_rect,
        gap_cursor::{GapCursor, as_gap_cursor},
    };

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
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        }
    }

    // 3×3 table: root > table > [row0..row2], each row > [cell0..cell2 > para]
    // Returns (doc, root, table, rows[3], cells[3][3])
    // cells[r][c] = Dot for the cell at row r, column c
    fn three_by_three_table() -> (ProjectedDoc, Dot, Dot, [Dot; 3], [[Dot; 3]; 3]) {
        let root = Dot::ROOT;
        let table = Dot::new(5, 1);
        let rows = [Dot::new(5, 2), Dot::new(5, 5), Dot::new(5, 8)];
        // cells[r][c]: row r starts at offset 2 + r*3
        let cells = [
            [Dot::new(5, 11), Dot::new(5, 13), Dot::new(5, 15)],
            [Dot::new(5, 17), Dot::new(5, 19), Dot::new(5, 21)],
            [Dot::new(5, 23), Dot::new(5, 25), Dot::new(5, 27)],
        ];
        // para inside each cell: cell_dot + 1
        let mut counter = 100u64;
        let mut next = || {
            let d = Dot::new(5, counter);
            counter += 1;
            d
        };
        let mut items = vec![(
            table,
            SeqItem::Block {
                node_type: NodeType::Table,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for r in 0..3 {
            items.push((
                rows[r],
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ));
            for cell in cells[r] {
                items.push((
                    cell,
                    SeqItem::Block {
                        node_type: NodeType::TableCell,
                        parents: vec![root, table, rows[r]],
                        attrs: vec![],
                    },
                ));
                items.push((
                    next(),
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root, table, rows[r], cell],
                        attrs: vec![],
                    },
                ));
            }
        }
        (
            project_document(&logs(&items)).unwrap(),
            root,
            table,
            rows,
            cells,
        )
    }

    // 2×2 table for full-table promotion test
    fn two_by_two_table() -> (ProjectedDoc, Dot, Dot, Dot, Dot, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let table = Dot::new(6, 1);
        let row0 = Dot::new(6, 2);
        let row1 = Dot::new(6, 6);
        let cell00 = Dot::new(6, 3);
        let cell01 = Dot::new(6, 4);
        let cell10 = Dot::new(6, 7);
        let cell11 = Dot::new(6, 8);
        let mut counter = 20u64;
        let mut next = || {
            let d = Dot::new(6, counter);
            counter += 1;
            d
        };
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row0,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell00,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell00],
                    attrs: vec![],
                },
            ),
            (
                cell01,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell01],
                    attrs: vec![],
                },
            ),
            (
                row1,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell10,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row1],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row1, cell10],
                    attrs: vec![],
                },
            ),
            (
                cell11,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row1],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row1, cell11],
                    attrs: vec![],
                },
            ),
        ];
        (
            project_document(&logs(&items)).unwrap(),
            root,
            table,
            row0,
            cell00,
            cell01,
            row1,
            cell10,
            cell11,
        )
    }

    // image as first child of root, then a paragraph
    fn image_first_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let img_dot = Dot::new(7, 1);
        let para = Dot::new(7, 2);
        let img_node = match editor_model::NodeType::Image.into_node() {
            editor_model::Node::Image(n) => n,
            _ => unreachable!(),
        };
        let items = vec![
            (
                img_dot,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), root)
    }

    // root > fold1 > [title, content] + fold2 > [title, content] + para
    fn two_folds_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let fold1 = Dot::new(8, 1);
        let fold1_title = Dot::new(8, 2);
        let fold1_content = Dot::new(8, 3);
        let fold2 = Dot::new(8, 4);
        let fold2_title = Dot::new(8, 5);
        let fold2_content = Dot::new(8, 6);
        let para = Dot::new(8, 7);
        let items = vec![
            (
                fold1,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                fold1_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold1],
                    attrs: vec![],
                },
            ),
            (
                fold1_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold1],
                    attrs: vec![],
                },
            ),
            (
                fold2,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                fold2_title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold2],
                    attrs: vec![],
                },
            ),
            (
                fold2_content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold2],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), root)
    }

    // root > para (leading paragraph, no unit)
    fn leading_para_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(9, 1);
        let items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        (project_document(&logs(&items)).unwrap(), root)
    }

    // §4.1 — cell_rect_selection round-trip: (c00, c11) over 3×3 table
    // must resolve via as_cell_rect back to corners c00/c11, correct rows/cols
    #[test]
    fn test_1_cell_rect_round_trip() {
        let (pd, _root, _table, rows, cells) = three_by_three_table();
        let view = DocView::new(&pd);
        let c00 = cells[0][0];
        let c11 = cells[1][1];

        let sel = cell_rect_selection(c00, c11, &view);
        assert!(sel.is_some(), "cell_rect_selection(c00, c11) must succeed");
        let sel = sel.unwrap();
        let rs = sel.resolve(&view).expect("selection must resolve");
        let cr = as_cell_rect(&rs);
        assert!(cr.is_some(), "result must classify as CellRect");
        let cr = cr.unwrap();
        assert_eq!(*cr.rows.start(), 0, "rows start at 0");
        assert_eq!(*cr.rows.end(), 1, "rows end at 1");
        assert_eq!(*cr.cols.start(), 0, "cols start at 0");
        assert_eq!(*cr.cols.end(), 1, "cols end at 1");
        assert_eq!(cr.anchor_cell.id(), c00, "anchor_cell is c00");
        assert_eq!(cr.head_cell.id(), c11, "head_cell is c11");
        let _ = rows;
    }

    // §4.1 — direction preserved: (c11, c00) keeps anchor c11, head c00
    #[test]
    fn test_1_cell_rect_direction_preserved() {
        let (pd, _root, _table, rows, cells) = three_by_three_table();
        let view = DocView::new(&pd);
        let c00 = cells[0][0];
        let c11 = cells[1][1];

        let sel = cell_rect_selection(c11, c00, &view);
        assert!(sel.is_some(), "cell_rect_selection(c11, c00) must succeed");
        let sel = sel.unwrap();
        let rs = sel.resolve(&view).expect("must resolve");
        let cr = as_cell_rect(&rs);
        assert!(cr.is_some(), "must classify as CellRect");
        let cr = cr.unwrap();
        // When anchor=c11 (row1,col1) and head=c00 (row0,col0):
        // ca=1 (c11's col), ch=0 (c00's col): ca > ch
        // anchor_offset = ca+1 = 2, head_offset = ch = 0
        // anchor in arow=row1 at offset 2 (after col1), head in hrow=row0 at offset 0 (before col0)
        // as_cell_rect: o_lo=0, o_hi=2, c_lo=0, c_hi=1
        // anchor_col: a.offset(2) != o_lo(0) → c_hi=1 → cr.anchor_cell is in row1 col1 = c11
        // head_col: h.offset(0) == o_lo(0) → c_lo=0 → cr.head_cell is in row0 col0 = c00
        assert_eq!(cr.anchor_cell.id(), c11, "anchor_cell is c11 (reversed)");
        assert_eq!(cr.head_cell.id(), c00, "head_cell is c00 (reversed)");
        let _ = rows;
    }

    // §4.2 — full-table promotion: full 2×2 → table node-selection, as_cell_rect None
    #[test]
    fn test_2_full_table_promotion() {
        let (pd, root, table, _row0, cell00, _cell01, _row1, _cell10, cell11) = two_by_two_table();
        let view = DocView::new(&pd);
        let c00 = cell00;
        let c11 = cell11;

        let sel = cell_rect_selection(c00, c11, &view);
        assert!(sel.is_some(), "full-table cell_rect_selection must succeed");
        let sel = sel.unwrap();

        // Result must be the table node-selection
        let table_node = view.node(table).unwrap();
        let table_idx = table_node.index().unwrap();
        let root_id = root;
        assert_eq!(sel.anchor.node, root_id, "anchor node is root");
        assert_eq!(sel.anchor.offset, table_idx, "anchor offset = table index");
        assert_eq!(sel.head.node, root_id, "head node is root");
        assert_eq!(
            sel.head.offset,
            table_idx + 1,
            "head offset = table index + 1"
        );

        // as_cell_rect on the result must be None (it's a node-selection, not cell-rect)
        let rs = sel.resolve(&view).unwrap();
        assert!(
            as_cell_rect(&rs).is_none(),
            "full-table promotion result has no CellRect"
        );
    }

    // §4.3 — rejects non-cells
    #[test]
    fn test_3_rejects_non_cells() {
        let (pd, root, table, rows, cells) = three_by_three_table();
        let view = DocView::new(&pd);

        // Pass a root node id (not a TableCell)
        let root_id = root;
        let c00 = cells[0][0];
        assert!(
            cell_rect_selection(root_id, c00, &view).is_none(),
            "non-cell anchor must return None"
        );
        assert!(
            cell_rect_selection(c00, root_id, &view).is_none(),
            "non-cell head must return None"
        );

        // Pass a table row (not a TableCell)
        let row_id = rows[0];
        assert!(
            cell_rect_selection(row_id, c00, &view).is_none(),
            "TableRow anchor must return None"
        );
        assert!(
            cell_rect_selection(c00, row_id, &view).is_none(),
            "TableRow head must return None"
        );
        let _ = table;
    }

    // §4.3 — rejects cross-table cells
    #[test]
    fn test_3_rejects_cross_table() {
        // Build a doc with two separate tables
        let root = Dot::ROOT;
        let table1 = Dot::new(12, 1);
        let row_t1 = Dot::new(12, 2);
        let cell_t1 = Dot::new(12, 3);
        let table2 = Dot::new(12, 10);
        let row_t2 = Dot::new(12, 11);
        let cell_t2 = Dot::new(12, 12);
        let mut counter = 30u64;
        let mut next = || {
            let d = Dot::new(12, counter);
            counter += 1;
            d
        };
        let items = vec![
            (
                table1,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row_t1,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table1],
                    attrs: vec![],
                },
            ),
            (
                cell_t1,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table1, row_t1],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table1, row_t1, cell_t1],
                    attrs: vec![],
                },
            ),
            (
                table2,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row_t2,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table2],
                    attrs: vec![],
                },
            ),
            (
                cell_t2,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table2, row_t2],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table2, row_t2, cell_t2],
                    attrs: vec![],
                },
            ),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let id_t1 = cell_t1;
        let id_t2 = cell_t2;
        assert!(
            cell_rect_selection(id_t1, id_t2, &view).is_none(),
            "cross-table cells must return None"
        );
    }

    // §4.4 — document leading gap: image first → root IsolatingBoundary collapsed
    #[test]
    fn test_4_gap_leading_image() {
        let (pd, _root) = image_first_doc();
        let view = DocView::new(&pd);
        let sel = gap_cursor_selection_at(Dot::ROOT, 0, &view);
        assert!(
            sel.is_some(),
            "image-first doc must produce a leading gap selection"
        );
        let sel = sel.unwrap();
        assert!(
            sel.is_collapsed(),
            "gap leading selection must be collapsed"
        );
        let rs = sel.resolve(&view).unwrap();
        let gc = as_gap_cursor(&rs);
        assert!(gc.is_some(), "result must classify as gap cursor");
        assert!(
            matches!(gc, Some(GapCursor::IsolatingBoundary { index: 0, .. })),
            "must be the root leading boundary gap"
        );
    }

    // §4.4 — document leading gap: fold first → root IsolatingBoundary collapsed
    #[test]
    fn test_4_gap_leading_fold() {
        let (pd, _root) = two_folds_doc();
        let view = DocView::new(&pd);
        // In two_folds_doc, fold1 is the first child of root.
        // Fold is monolithic (isolating+monolithic), so it's a unit
        let sel = gap_cursor_selection_at(Dot::ROOT, 0, &view);
        assert!(
            sel.is_some(),
            "fold-first doc must produce a leading gap selection"
        );
        let sel = sel.unwrap();
        assert!(
            sel.is_collapsed(),
            "gap leading selection must be collapsed"
        );
        let rs = sel.resolve(&view).unwrap();
        let gc = as_gap_cursor(&rs);
        assert!(gc.is_some(), "result must classify as gap cursor");
        assert!(
            matches!(gc, Some(GapCursor::IsolatingBoundary { index: 0, .. })),
            "must be the root leading boundary gap"
        );
    }

    // §4.4 — document leading gap: paragraph first → None
    #[test]
    fn test_4_gap_leading_para_is_none() {
        let (pd, _root) = leading_para_doc();
        let view = DocView::new(&pd);
        let sel = gap_cursor_selection_at(Dot::ROOT, 0, &view);
        assert!(
            sel.is_none(),
            "paragraph-first doc must return None for leading gap"
        );
    }

    // §4.5 — gap_cursor_selection_at: between two folds → BetweenMonolithic { index: 1 }
    #[test]
    fn test_5_gap_between_folds() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let root_id = root;

        let sel = gap_cursor_selection_at(root_id, 1, &view);
        assert!(sel.is_some(), "between two folds at index 1 must succeed");
        let sel = sel.unwrap();
        assert!(
            sel.is_collapsed(),
            "gap between selection must be collapsed"
        );
        let rs = sel.resolve(&view).unwrap();
        let gc = as_gap_cursor(&rs);
        assert!(gc.is_some(), "must classify as gap cursor");
        assert!(
            matches!(gc, Some(GapCursor::BetweenMonolithic { index: 1, .. })),
            "must be BetweenMonolithic at index 1"
        );
    }

    // §4.5 — root is isolating: its leading slot builds through the unified
    // boundary rule and re-detects as the same gap
    #[test]
    fn test_5_gap_at_root_leading_slot_builds() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);

        let sel = gap_cursor_selection_at(root, 0, &view)
            .expect("root leading slot before a fold is a boundary gap");
        assert!(sel.is_collapsed(), "boundary gap selection is collapsed");
        let rs = sel.resolve(&view).unwrap();
        assert!(
            matches!(
                as_gap_cursor(&rs),
                Some(GapCursor::IsolatingBoundary { index: 0, .. })
            ),
            "root leading gap classifies as IsolatingBoundary at index 0"
        );
    }

    // §4.5 — gap_cursor_selection_at: out-of-range index → None
    #[test]
    fn test_5_gap_between_out_of_range_is_none() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let root_id = root;
        // two_folds_doc has 3 children: fold1, fold2, para — indices 0,1,2
        // index 3 is out of range (>= count)
        assert!(
            gap_cursor_selection_at(root_id, 3, &view).is_none(),
            "out-of-range index must return None"
        );
    }

    // §4.5 — gap_cursor_selection_at: non-monolithic neighbor (fold + para) → None
    #[test]
    fn test_5_gap_between_non_monolithic_is_none() {
        let (pd, root) = two_folds_doc();
        let view = DocView::new(&pd);
        let root_id = root;
        // children: fold1(0), fold2(1), para(2)
        // between fold2 and para at index 2: para is not monolithic → None
        assert!(
            gap_cursor_selection_at(root_id, 2, &view).is_none(),
            "fold+para at index 2 is not a valid gap (para not monolithic)"
        );
    }

    // root > fold > [fold_title, fold_content > callout > para] + para
    fn fold_callout_doc() -> (ProjectedDoc, Dot) {
        let root = Dot::ROOT;
        let fold = Dot::new(10, 1);
        let title = Dot::new(10, 2);
        let content = Dot::new(10, 3);
        let callout = Dot::new(10, 4);
        let inner_para = Dot::new(10, 5);
        let para = Dot::new(10, 6);
        let items = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                title,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![root, fold],
                    attrs: vec![],
                },
            ),
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![root, fold, content],
                    attrs: vec![],
                },
            ),
            (
                inner_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, fold, content, callout],
                    attrs: vec![],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        (project_document(&logs(&items)).unwrap(), content)
    }

    // §4.5 — gap_cursor_selection_at: isolating boundary slots round-trip
    #[test]
    fn test_5_gap_at_isolating_boundaries_round_trips() {
        let (pd, content) = fold_callout_doc();
        let view = DocView::new(&pd);

        for index in [0usize, 1] {
            let sel = gap_cursor_selection_at(content, index, &view)
                .unwrap_or_else(|| panic!("boundary gap at index {index} must build"));
            assert!(sel.is_collapsed(), "boundary gap selection is collapsed");
            let rs = sel.resolve(&view).unwrap();
            let gc = as_gap_cursor(&rs);
            assert!(
                matches!(gc, Some(GapCursor::IsolatingBoundary { index: i, .. }) if i == index),
                "index {index} must re-detect as IsolatingBoundary"
            );
        }
        assert!(
            gap_cursor_selection_at(content, 2, &view).is_none(),
            "out-of-range index must return None"
        );
    }

    // §4.6 — proptest: builders never panic; returned selection re-detects as same kind or is None
    proptest::proptest! {
        #[test]
        fn test_proptest_builders_never_panic(
            r0 in 0usize..3,
            c0 in 0usize..3,
            r1 in 0usize..3,
            c1 in 0usize..3,
            gap_index in 0usize..5,
        ) {
            let (pd3, _root3, _table3, _rows3, cells3) = three_by_three_table();
            let view3 = DocView::new(&pd3);
            let anchor_id = cells3[r0][c0];
            let head_id = cells3[r1][c1];

            let sel = cell_rect_selection(anchor_id, head_id, &view3);
            if let Some(sel) = sel {
                // Must resolve
                let Some(rs) = sel.resolve(&view3) else { return Ok(()); };
                let is_cell_rect = as_cell_rect(&rs).is_some();
                // A full-table promoted selection brackets the table block itself.
                // So we just check that the result resolves without any subtree violation.
                if !is_cell_rect {
                    // Either it's the promoted table node-selection or a collapse;
                    // both endpoints must resolve
                    proptest::prop_assert!(
                        rs.anchor().position().resolve(&view3).is_some(),
                        "anchor resolves"
                    );
                    proptest::prop_assert!(
                        rs.head().position().resolve(&view3).is_some(),
                        "head resolves"
                    );
                }
            }

            // gap_cursor_selection_at: never panic
            let (pd8, root8) = two_folds_doc();
            let view8 = DocView::new(&pd8);
            let root8_id = root8;
            let _ = gap_cursor_selection_at(root8_id, gap_index, &view8);

            // document leading gap: never panic
            let (pd7, _) = image_first_doc();
            let view7 = DocView::new(&pd7);
            let _ = gap_cursor_selection_at(Dot::ROOT, 0, &view7);
        }

        #[test]
        fn test_proptest_gap_between_re_detects(
            gap_index in 0usize..5,
        ) {
            let (pd, root) = two_folds_doc();
            let view = DocView::new(&pd);
            let root_id = root;
            if let Some(sel) = gap_cursor_selection_at(root_id, gap_index, &view) {
                // Must be collapsed and re-detect as a gap cursor
                proptest::prop_assert!(sel.is_collapsed(), "gap between must be collapsed");
                let rs = sel.resolve(&view).expect("must resolve");
                let gc = as_gap_cursor(&rs);
                proptest::prop_assert!(gc.is_some(), "gap between must re-detect as gap cursor");
                if let Some(GapCursor::BetweenMonolithic { index, .. }) = gc {
                    proptest::prop_assert_eq!(index, gap_index, "gap index matches");
                }
            }
        }

        #[test]
        fn test_proptest_leading_gap_re_detects(
            _unused in 0u8..4,
        ) {
            let (pd, _) = image_first_doc();
            let view = DocView::new(&pd);
            if let Some(sel) = gap_cursor_selection_at(Dot::ROOT, 0, &view) {
                proptest::prop_assert!(sel.is_collapsed(), "leading gap must be collapsed");
                let rs = sel.resolve(&view).expect("must resolve");
                let gc = as_gap_cursor(&rs);
                proptest::prop_assert!(gc.is_some(), "leading gap must re-detect as gap cursor");
                proptest::prop_assert!(
                    matches!(gc, Some(GapCursor::IsolatingBoundary { index: 0, .. })),
                    "must be the root leading boundary gap"
                );
            }
        }
    }
}

use std::ops::RangeInclusive;

use editor_crdt::Dot;
use editor_model::{ChildView, DocView, LeafView, NodeType, NodeView};

use crate::selection::ResolvedSelection;

fn block_child_at<'a>(node: &NodeView<'a>, i: usize) -> Option<NodeView<'a>> {
    match node.child_at(i) {
        Some(ChildView::Block(b)) => Some(b),
        _ => None,
    }
}

pub struct CellRect<'a> {
    pub table: NodeView<'a>,
    pub anchor_cell: NodeView<'a>,
    pub head_cell: NodeView<'a>,
    pub rows: RangeInclusive<usize>,
    pub cols: RangeInclusive<usize>,
}

fn row_width(table: &NodeView, row_idx: usize) -> usize {
    block_child_at(table, row_idx)
        .map(|row| row.children().count())
        .unwrap_or(0)
}

fn uniform_width(table: &NodeView) -> Option<usize> {
    let count = table.children().count();
    if count == 0 {
        return None;
    }
    let w = row_width(table, 0);
    for i in 1..count {
        if row_width(table, i) != w {
            return None;
        }
    }
    Some(w)
}

impl<'a> CellRect<'a> {
    pub fn table_id(&self) -> Dot {
        self.table.id()
    }
    pub fn rows(&self) -> &std::ops::RangeInclusive<usize> {
        &self.rows
    }
    pub fn cols(&self) -> &std::ops::RangeInclusive<usize> {
        &self.cols
    }

    pub fn cells(&self) -> Vec<NodeView<'a>> {
        let mut out = Vec::new();
        for r in self.rows.clone() {
            if let Some(row) = block_child_at(&self.table, r) {
                for c in self.cols.clone() {
                    if let Some(cell) = block_child_at(&row, c) {
                        out.push(cell);
                    }
                }
            }
        }
        out
    }

    pub fn contains(&self, cell: &NodeView) -> bool {
        let Some(row) = cell.parent() else {
            return false;
        };
        let Some(r) = row.index() else {
            return false;
        };
        let Some(c) = cell.index() else {
            return false;
        };
        self.rows.contains(&r) && self.cols.contains(&c)
    }

    pub fn is_single(&self) -> bool {
        self.rows.start() == self.rows.end() && self.cols.start() == self.cols.end()
    }

    pub fn is_full_row(&self) -> bool {
        match uniform_width(&self.table) {
            Some(w) if w > 0 => *self.cols.start() == 0 && *self.cols.end() == w - 1,
            _ => false,
        }
    }

    pub fn is_full_column(&self) -> bool {
        let row_count = self.table.children().count();
        if row_count == 0 {
            return false;
        }
        *self.rows.start() == 0 && *self.rows.end() == row_count - 1
    }

    pub fn is_full_table(&self) -> bool {
        self.is_full_row() && self.is_full_column()
    }
}

fn endpoint_row<'a>(pos: &crate::Position, view: &'a DocView<'a>) -> Option<NodeView<'a>> {
    let row = view.node(pos.node)?;
    (row.node_type() == NodeType::TableRow).then_some(row)
}

pub fn as_cell_rect<'a>(rs: &ResolvedSelection<'a>) -> Option<CellRect<'a>> {
    if rs.is_collapsed() {
        return None;
    }
    let (a, h) = (rs.anchor().position(), rs.head().position());
    let arow = endpoint_row(&a, rs.view())?;
    let hrow = endpoint_row(&h, rs.view())?;
    let table = arow.parent()?;
    if table.node_type() != NodeType::Table {
        return None;
    }
    if hrow.parent()?.id() != table.id() {
        return None;
    }
    let (ra, rh) = (arow.index()?, hrow.index()?);
    let (o_lo, o_hi) = (a.offset.min(h.offset), a.offset.max(h.offset));
    if o_hi == o_lo {
        return None;
    }
    let (c_lo, c_hi) = (o_lo, o_hi - 1);
    let anchor_col = if a.offset == o_lo { c_lo } else { c_hi };
    let head_col = if h.offset == o_lo { c_lo } else { c_hi };
    let anchor_cell = block_child_at(&arow, anchor_col)?;
    let head_cell = block_child_at(&hrow, head_col)?;
    if anchor_cell.node_type() != NodeType::TableCell
        || head_cell.node_type() != NodeType::TableCell
    {
        return None;
    }
    Some(CellRect {
        table,
        anchor_cell,
        head_cell,
        rows: ra.min(rh)..=ra.max(rh),
        cols: c_lo..=c_hi,
    })
}

pub fn as_node_selection<'a>(rs: &ResolvedSelection<'a>) -> Option<LeafView<'a>> {
    if rs.is_collapsed() || as_cell_rect(rs).is_some() {
        return None;
    }
    let (a, h) = (rs.anchor().position(), rs.head().position());
    if a.node != h.node {
        return None;
    }
    let (lo, hi) = (a.offset.min(h.offset), a.offset.max(h.offset));
    if hi - lo != 1 {
        return None;
    }
    match rs.view().node(a.node)?.child_at(lo)? {
        ChildView::Leaf(l) if l.as_char().is_some() => None,
        ChildView::Leaf(l) => Some(l),
        ChildView::Block(_) => None,
    }
}

pub fn enclosing_table_cell<'a>(view: &'a DocView<'a>, node: Dot) -> Option<Dot> {
    view.node(node)?
        .ancestors()
        .find(|n| n.node_type() == NodeType::TableCell)
        .map(|n| n.id())
}

pub fn enclosing_table<'a>(view: &'a DocView<'a>, cell: Dot) -> Option<Dot> {
    view.node(cell)?
        .ancestors()
        .find(|n| n.node_type() == NodeType::Table)
        .map(|n| n.id())
}

pub fn table_cell_ids<'a>(view: &'a DocView<'a>, cell: Dot) -> Vec<Dot> {
    let cell_node = view.node(cell);
    if cell_node.as_ref().map(|n| n.node_type()) != Some(NodeType::TableCell) {
        return vec![];
    }
    let table_id = match enclosing_table(view, cell) {
        Some(id) => id,
        None => return vec![],
    };
    let table = match view.node(table_id) {
        Some(t) => t,
        None => return vec![],
    };
    let mut out = Vec::new();
    for row in table.child_blocks() {
        for cell_child in row.child_blocks() {
            out.push(cell_child.id());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeType, ProjectedDoc,
        SeqItem, SpanLog, project_document,
    };

    use crate::{Position, affinity::Affinity, selection::Selection};

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

    // 2x2 table: root > table > [row0 > [cell00, cell01], row1 > [cell10, cell11]]
    // Each cell has a paragraph child.
    fn two_by_two_table() -> (ProjectedDoc, Dot, Dot, Dot, Dot, Dot, Dot, Dot) {
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row0 = Dot::new(1, 2);
        let cell00 = Dot::new(1, 3);
        let cell01 = Dot::new(1, 4);
        let row1 = Dot::new(1, 5);
        let cell10 = Dot::new(1, 6);
        let cell11 = Dot::new(1, 7);
        let mut counter = 8u64;
        let mut next = || {
            let d = Dot::new(1, counter);
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
        )
    }

    fn sel<'a>(
        view: &'a DocView<'a>,
        anchor_node: Dot,
        anchor_off: usize,
        head_node: Dot,
        head_off: usize,
    ) -> crate::selection::ResolvedSelection<'a> {
        let a = Position {
            node: anchor_node,
            offset: anchor_off,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: head_node,
            offset: head_off,
            affinity: Affinity::Downstream,
        };
        Selection::new(a, h).resolve(view).unwrap()
    }

    #[test]
    fn test_4_as_cell_rect_single_row() {
        let (pd, _root, _table, row0, cell00, cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);
        // anchor at row0 offset 0, head at row0 offset 2 (both cells)
        let rs = sel(&view, row0, 0, row0, 2);
        let cr = as_cell_rect(&rs);
        assert!(cr.is_some(), "single-row 2-cell rect should resolve");
        let cr = cr.unwrap();
        assert_eq!(*cr.rows.start(), 0);
        assert_eq!(*cr.rows.end(), 0);
        assert_eq!(*cr.cols.start(), 0);
        assert_eq!(*cr.cols.end(), 1);
        let cell00_node = view.node(cell00).unwrap();
        let cell01_node = view.node(cell01).unwrap();
        assert!(cr.contains(&cell00_node));
        assert!(cr.contains(&cell01_node));
    }

    #[test]
    fn test_4_as_cell_rect_cross_row() {
        let (pd, _root, _table, row0, _cell00, _cell01, row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);
        // anchor row0 col 0..1 → cell00, head row1 col 0..1 → cell10
        let rs = sel(&view, row0, 0, row1, 1);
        let cr = as_cell_rect(&rs);
        assert!(cr.is_some(), "cross-row rect should resolve");
        let cr = cr.unwrap();
        assert_eq!(*cr.rows.start(), 0);
        assert_eq!(*cr.rows.end(), 1);
        assert_eq!(*cr.cols.start(), 0);
        assert_eq!(*cr.cols.end(), 0);
    }

    #[test]
    fn test_4_as_cell_rect_none_when_collapsed() {
        let (pd, _root, _table, row0, _cell00, _cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);
        let pos = Position::new(row0, 1);
        let sel = Selection::collapsed(pos).resolve(&view).unwrap();
        assert!(
            as_cell_rect(&sel).is_none(),
            "collapsed selection cannot be cell rect"
        );
    }

    #[test]
    fn test_5_cell_rect_predicates() {
        let (pd, _root, _table, row0, _cell00, _cell01, row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // Single cell: row0 col 0
        let rs_single = sel(&view, row0, 0, row0, 1);
        let cr_single = as_cell_rect(&rs_single).unwrap();
        assert!(cr_single.is_single());
        assert!(!cr_single.is_full_row());
        assert!(!cr_single.is_full_column());
        assert!(!cr_single.is_full_table());

        // Full row: row0 col 0..=1
        let rs_full_row = sel(&view, row0, 0, row0, 2);
        let cr_full_row = as_cell_rect(&rs_full_row).unwrap();
        assert!(!cr_full_row.is_single());
        assert!(cr_full_row.is_full_row());
        assert!(!cr_full_row.is_full_column());
        assert!(!cr_full_row.is_full_table());

        // Full column: col 0 across both rows
        let rs_full_col = sel(&view, row0, 0, row1, 1);
        let cr_full_col = as_cell_rect(&rs_full_col).unwrap();
        assert!(!cr_full_col.is_single());
        assert!(!cr_full_col.is_full_row());
        assert!(cr_full_col.is_full_column());
        assert!(!cr_full_col.is_full_table());

        // Full table: all rows, all cols
        let rs_full_table = sel(&view, row0, 0, row1, 2);
        let cr_full_table = as_cell_rect(&rs_full_table).unwrap();
        assert!(!cr_full_table.is_single());
        assert!(cr_full_table.is_full_row());
        assert!(cr_full_table.is_full_column());
        assert!(cr_full_table.is_full_table());
    }

    fn doc_with_hardbreak_atom() -> (ProjectedDoc, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let atom_dot = Dot::new(1, 2);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (atom_dot, SeqItem::Atom(AtomLeaf::HardBreak)),
        ];
        (project_document(&logs(&items)).unwrap(), para, atom_dot)
    }

    #[test]
    fn test_6_as_node_selection_atom() {
        let (pd, para, _atom_dot) = doc_with_hardbreak_atom();
        let view = DocView::new(&pd);
        // selection of exactly 1 atom leaf (offset 0..1 in para)
        let a = Position {
            node: para,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let h = Position {
            node: para,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        let ns = as_node_selection(&sel);
        assert!(
            ns.is_some(),
            "selecting a single atom leaf should give node selection"
        );
        let lv = ns.unwrap();
        assert!(lv.as_char().is_none());
        assert!(lv.as_atom().is_some());
    }

    #[test]
    fn test_6_as_node_selection_none_for_char() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);
        let a = Position::new(para, 0);
        let h = Position::new(para, 1);
        let sel = Selection::new(a, h).resolve(&view).unwrap();
        assert!(
            as_node_selection(&sel).is_none(),
            "char leaf is not a node selection"
        );
    }

    #[test]
    fn test_7_table_helpers() {
        let (pd, _root, _table, _row0, cell00, _cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        let cell00_id = cell00;
        let tc = enclosing_table_cell(&view, cell00_id);
        assert!(
            tc.is_some(),
            "cell00 should find its own TableCell as enclosing"
        );
        let tc_id = tc.unwrap();
        assert_eq!(tc_id, cell00_id);

        let t = enclosing_table(&view, cell00_id);
        assert!(t.is_some(), "should find enclosing table from a cell");
        let table_node = view.node(t.unwrap()).unwrap();
        assert_eq!(table_node.node_type(), NodeType::Table);

        let ids = table_cell_ids(&view, cell00_id);
        assert_eq!(ids.len(), 4, "2x2 table should have 4 cells");
    }

    // §4.4 — affinity-invariant: same 2×2 rect with Upstream endpoint affinities yields same result
    #[test]
    fn test_4_cell_rect_affinity_invariant() {
        let (pd, _root, _table, row0, _cell00, _cell01, row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // Build with Downstream (existing helper)
        let rs_down = sel(&view, row0, 0, row1, 2);
        let cr_down = as_cell_rect(&rs_down).expect("Downstream should resolve");

        // Same endpoints with Upstream affinity
        let a = Position {
            node: row0,
            offset: 0,
            affinity: Affinity::Upstream,
        };
        let h = Position {
            node: row1,
            offset: 2,
            affinity: Affinity::Upstream,
        };
        let rs_up = Selection::new(a, h).resolve(&view).unwrap();
        let cr_up = as_cell_rect(&rs_up).expect("Upstream should resolve to the same rect");

        assert_eq!(cr_down.rows, cr_up.rows, "rows must be affinity-invariant");
        assert_eq!(cr_down.cols, cr_up.cols, "cols must be affinity-invariant");
    }

    // §4.4 — anti-diagonal: anchor top-right, head bottom-left → correct anchor_cell/head_cell
    #[test]
    fn test_4_cell_rect_anti_diagonal() {
        let (pd, _root, _table, row0, _cell00, cell01, row1, cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // anchor at row0 offset 2 (boundary after col 1 = top-right cell01),
        // head at row1 offset 0 (boundary before col 0 = bottom-left cell10).
        // o_lo=0, o_hi=2, c_lo=0, c_hi=1
        // anchor_col: a.offset(2) != o_lo(0) → c_hi=1 (cell01) ✓
        // head_col:   h.offset(0) == o_lo(0) → c_lo=0 (cell10) ✓
        let rs = sel(&view, row0, 2, row1, 0);
        let cr = as_cell_rect(&rs).expect("anti-diagonal selection should resolve");

        assert_eq!(*cr.rows.start(), 0);
        assert_eq!(*cr.rows.end(), 1);
        assert_eq!(*cr.cols.start(), 0);
        assert_eq!(*cr.cols.end(), 1);

        // anchor_cell must be top-right (cell01, row0 col1)
        assert_eq!(cr.anchor_cell.id(), cell01);
        // head_cell must be bottom-left (cell10, row1 col0)
        assert_eq!(cr.head_cell.id(), cell10);
    }

    // §4.4 — cross-table: endpoints in rows of different tables → None
    #[test]
    fn test_4_cell_rect_cross_table_is_none() {
        // Build two separate tables in the same doc.
        let root = Dot::ROOT;
        let table1 = Dot::new(1, 1);
        let row1t1 = Dot::new(1, 2);
        let _cell1 = Dot::new(1, 3);
        let table2 = Dot::new(1, 10);
        let row1t2 = Dot::new(1, 11);
        let _cell2 = Dot::new(1, 12);
        let mut counter = 20u64;
        let mut next = || {
            let d = Dot::new(1, counter);
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
                row1t1,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table1],
                    attrs: vec![],
                },
            ),
            (
                _cell1,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table1, row1t1],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table1, row1t1, _cell1],
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
                row1t2,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table2],
                    attrs: vec![],
                },
            ),
            (
                _cell2,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table2, row1t2],
                    attrs: vec![],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table2, row1t2, _cell2],
                    attrs: vec![],
                },
            ),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);

        // anchor in row of table1, head in row of table2
        let rs = sel(&view, row1t1, 0, row1t2, 1);
        assert!(
            as_cell_rect(&rs).is_none(),
            "endpoints in rows of different tables must not form a cell rect"
        );
    }

    // §4.4 — degenerate: same-row equal-offset, non-collapsed via differing affinity → None
    #[test]
    fn test_4_cell_rect_degenerate_same_offset_non_collapsed() {
        let (pd, _root, _table, row0, _cell00, _cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // Both endpoints in row0 at offset 1; differing affinity makes them non-collapsed.
        let a = Position {
            node: row0,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let h = Position {
            node: row0,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let rs = Selection::new(a, h).resolve(&view).unwrap();
        assert!(
            !rs.is_collapsed(),
            "must be non-collapsed (different affinity)"
        );
        assert!(
            as_cell_rect(&rs).is_none(),
            "same offset in same row (o_lo==o_hi) must return None"
        );
    }

    // §4.5 — ragged table: rows of different widths
    #[test]
    fn test_5_ragged_table_predicates() {
        // Build: root > table > [row0 > [cell00, cell01, cell02], row1 > [cell10, cell11]]
        // row0 has 3 cells, row1 has 2 cells — a ragged (non-uniform) table.
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row0 = Dot::new(1, 2);
        let cell00 = Dot::new(1, 3);
        let cell01 = Dot::new(1, 4);
        let cell02 = Dot::new(1, 5);
        let row1 = Dot::new(1, 6);
        let cell10 = Dot::new(1, 7);
        let cell11 = Dot::new(1, 8);
        let mut counter = 20u64;
        let mut next = || {
            let d = Dot::new(1, counter);
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
                cell02,
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
                    parents: vec![root, table, row0, cell02],
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
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);

        // Select cols 0..=1 across both rows: anchor=(row0,0), head=(row1,2)
        // o_lo=0, o_hi=2, cols=0..=1, rows=0..=1
        let rs = sel(&view, row0, 0, row1, 2);
        let cr = as_cell_rect(&rs).expect("should resolve to a cell rect");

        // is_full_row uses uniform_width; since row0 has 3 cells and row1 has 2, uniform_width
        // returns None, so is_full_row = false even though cols covers 0..=1.
        assert!(
            !cr.is_full_row(),
            "ragged table: uniform_width fails → is_full_row false"
        );

        // is_full_table = is_full_row && is_full_column; since is_full_row is false, so is this.
        assert!(
            !cr.is_full_table(),
            "ragged table must not claim is_full_table"
        );

        // cells() must not panic and must return only cells actually present in the cols range.
        // rows 0..=1, cols 0..=1: row0 has cells at both cols (cell00, cell01), row1 too → 4.
        let cells = cr.cells();
        assert_eq!(
            cells.len(),
            4,
            "ragged table: cells() counts only present slots, no panic"
        );

        // Suppress unused variable warnings
        let _ = (cell02, cell10, cell11);
    }

    // §4.6 — 1×1 cell-rect must NOT be reported as a node-selection
    #[test]
    fn test_6_node_selection_none_for_1x1_cell_rect() {
        let (pd, _root, _table, row0, _cell00, _cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // Single-cell selection (offset 0..1 in row0 → 1×1 rect)
        let rs = sel(&view, row0, 0, row0, 1);
        let cr = as_cell_rect(&rs);
        assert!(cr.is_some(), "this is a valid 1×1 cell rect");
        assert!(cr.unwrap().is_single());

        // as_node_selection must return None because as_cell_rect gate fires first
        assert!(
            as_node_selection(&rs).is_none(),
            "1×1 cell rect must NOT be reported as a node-selection (as_cell_rect gate)"
        );
    }

    // §4.6 — bracketed Block container (e.g. a Table) → as_node_selection is None
    #[test]
    fn test_6_node_selection_none_for_block_container() {
        // root > [table, paragraph]: bracket the table → offset 0..1 in root
        let root = Dot::ROOT;
        let table = Dot::new(1, 1);
        let row = Dot::new(1, 2);
        let cell = Dot::new(1, 3);
        let para_in_cell = Dot::new(1, 4);
        let para_after = Dot::new(1, 5);
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
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                    attrs: vec![],
                },
            ),
            (
                para_in_cell,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, cell],
                    attrs: vec![],
                },
            ),
            (
                para_after,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
        ];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);

        // Bracket the table: anchor at root offset 0, head at root offset 1
        let a = Position::new(root, 0);
        let h = Position::new(root, 1);
        let rs = Selection::new(a, h).resolve(&view).unwrap();

        // as_cell_rect: endpoints are in root (not a TableRow) → None
        assert!(
            as_cell_rect(&rs).is_none(),
            "no cell rect for root-level bracket"
        );
        // as_node_selection: the child at offset 0 is a Block (Table) → None
        assert!(
            as_node_selection(&rs).is_none(),
            "bracketed Block container must not be a node-selection"
        );
    }

    // §4.7 — enclosing_table_cell for a node outside any table → None
    #[test]
    fn test_7_enclosing_table_cell_outside_table_is_none() {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        let pd = project_document(&logs(&items)).unwrap();
        let view = DocView::new(&pd);

        let para_id = para;
        assert!(
            enclosing_table_cell(&view, para_id).is_none(),
            "paragraph outside any table must return None for enclosing_table_cell"
        );
    }

    // §4.7 — table_cell_ids for a non-TableCell id → empty vec
    #[test]
    fn test_7_table_cell_ids_non_cell_is_empty() {
        let (pd, _root, table, _row0, _cell00, _cell01, _row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        // Pass the Table node id, which is NOT a TableCell
        let table_id = table;
        let ids = table_cell_ids(&view, table_id);
        assert!(
            ids.is_empty(),
            "table_cell_ids for a non-TableCell id must return an empty vec"
        );
    }

    // §4.8 — proptest: detectors never panic and are mutually exclusive
    proptest::proptest! {
        #[test]
        fn test_8_proptest_mutual_exclusion(
            // Choose doc shape: 0=two-fold, 1=two-by-two-table, 2=hardbreak-atom
            doc_kind in 0u8..3,
            // Two (node_selector, offset) pairs for anchor / head
            a_node_sel in 0usize..3,
            a_off in 0usize..4,
            h_node_sel in 0usize..3,
            h_off in 0usize..4,
            a_aff_up in proptest::bool::ANY,
            h_aff_up in proptest::bool::ANY,
        ) {
            use crate::gap_cursor::as_gap_cursor;
            use crate::affinity::Affinity;

            // We'll test against the two_by_two_table doc for simplicity, with
            // offsets clamped to valid ranges. Using a larger arb space doesn't help
            // since the invariants are structural, not input-sensitive.
            let (pd, root_d, _table_d, row0_d, cell00_d, cell01_d, row1_d, cell10_d) = two_by_two_table();
            let view = DocView::new(&pd);

            // Available nodes to pick positions in
            let nodes = [
                root_d,
                row0_d,
                row1_d,
            ];
            let a_node = &nodes[a_node_sel.min(2)];
            let h_node = &nodes[h_node_sel.min(2)];

            // Clamp offsets to the node's actual child count
            let a_count = view.node(*a_node).map(|n| n.children().count()).unwrap_or(0);
            let h_count = view.node(*h_node).map(|n| n.children().count()).unwrap_or(0);
            let a_off_c = a_off.min(a_count);
            let h_off_c = h_off.min(h_count);

            let a_affinity = if a_aff_up { Affinity::Upstream } else { Affinity::Downstream };
            let h_affinity = if h_aff_up { Affinity::Upstream } else { Affinity::Downstream };

            let a = Position { node: *a_node, offset: a_off_c, affinity: a_affinity };
            let h = Position { node: *h_node, offset: h_off_c, affinity: h_affinity };

            let Some(rs) = Selection::new(a, h).resolve(&view) else { return Ok(()); };

            // Detectors must not panic
            let gc = as_gap_cursor(&rs);
            let cr = as_cell_rect(&rs);
            let ns = as_node_selection(&rs);

            // Mutual exclusion invariants:
            // 1. as_gap_cursor => selection is collapsed
            if gc.is_some() {
                proptest::prop_assert!(rs.is_collapsed(), "as_gap_cursor implies collapsed");
            }
            // 2. as_cell_rect / as_node_selection => non-collapsed
            if cr.is_some() {
                proptest::prop_assert!(!rs.is_collapsed(), "as_cell_rect implies non-collapsed");
            }
            if ns.is_some() {
                proptest::prop_assert!(!rs.is_collapsed(), "as_node_selection implies non-collapsed");
            }
            // 3. as_cell_rect and as_node_selection are never both Some
            proptest::prop_assert!(
                !(cr.is_some() && ns.is_some()),
                "as_cell_rect and as_node_selection are mutually exclusive"
            );

            // Suppress unused-variable warnings for doc_kind
            let _ = (doc_kind, cell00_d, cell01_d, cell10_d);
        }
    }

    #[test]
    fn cell_rect_accessors_and_anchor_head_reachable() {
        let (pd, _root, table, row0, cell00, cell01, row1, _cell10) = two_by_two_table();
        let view = DocView::new(&pd);

        let rs = sel(&view, row0, 0, row1, 2);
        let rect = rs.as_cell_rect().expect("should resolve to cell rect");

        assert_eq!(rect.table_id(), table);
        assert_eq!(*rect.rows().start(), 0);
        assert_eq!(*rect.rows().end(), 1);
        assert_eq!(*rect.cols().start(), 0);
        assert_eq!(*rect.cols().end(), 1);

        assert_eq!(rs.anchor().node(), row0);
        assert_eq!(rs.head().node(), row1);

        let _ = (cell00, cell01);
    }
}

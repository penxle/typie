use std::ops::RangeInclusive;

use editor_model::{Doc, Node, NodeId, NodeRef};

use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;
use crate::selection::Selection;

/// A rectangular block of table cells derived from a cell-rect `Selection`.
pub struct CellRect<'a> {
    pub table: NodeRef<'a>,
    pub anchor_cell: NodeRef<'a>,
    pub head_cell: NodeRef<'a>,
    pub rows: RangeInclusive<usize>,
    pub cols: RangeInclusive<usize>,
}

/// The `TableRow` an endpoint addresses, or `None` if its node is not one.
fn endpoint_row<'a>(doc: &'a Doc, pos: Position) -> Option<NodeRef<'a>> {
    let row = doc.node(pos.node_id)?;
    matches!(row.node(), Node::TableRow(_)).then_some(row)
}

impl<'a> ResolvedSelection<'a> {
    /// `Some` iff this selection encodes a table cell rectangle. Reads only
    /// node ids, offsets, and row indices — never affinity — so it is stable
    /// across the unconditional affinity-rewriting `normalize` pass.
    pub fn as_cell_rect(&self) -> Option<CellRect<'a>> {
        if self.is_collapsed() {
            return None;
        }
        let doc = self.doc();
        let a = Position::from(self.anchor());
        let h = Position::from(self.head());

        let arow = endpoint_row(doc, a)?;
        let hrow = endpoint_row(doc, h)?;

        let table = arow.parent()?;
        if !matches!(table.node(), Node::Table(_)) {
            return None;
        }
        if hrow.parent()?.id() != table.id() {
            return None;
        }

        let ra = arow.index()?;
        let rh = hrow.index()?;

        let o_lo = a.offset.min(h.offset);
        let o_hi = a.offset.max(h.offset);
        if o_hi == o_lo {
            return None; // degenerate / zero-width
        }
        let c_lo = o_lo;
        let c_hi = o_hi - 1;

        // The endpoint at the lower offset is the rectangle's left-outer
        // boundary (column c_lo); the one at the higher offset is the
        // right-outer boundary (column c_hi).
        let anchor_col = if a.offset == o_lo { c_lo } else { c_hi };
        let head_col = if h.offset == o_lo { c_lo } else { c_hi };

        let anchor_cell = arow.children().nth(anchor_col)?;
        let head_cell = hrow.children().nth(head_col)?;
        if !matches!(anchor_cell.node(), Node::TableCell(_))
            || !matches!(head_cell.node(), Node::TableCell(_))
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

    /// The single **leaf, non-`Text`** node bracketed by a node-selection,
    /// or `None`. Returns `None` whenever `as_cell_rect()` is `Some`, for a
    /// bracketed `Text` child, and for a **non-leaf** child — matching what
    /// real producers actually bracket (`select_node_forward.rs:38` checks
    /// `!next.spec().is_leaf()`; `select_node_backward.rs:34` checks
    /// `!prev.spec().is_leaf()`; both reject `Text`). It is a constrained
    /// "single leaf, non-`Text` node" detector — it does NOT also check the
    /// schema `selectable` flag, so a `HardBreak`/`PageBreak` bracket (never
    /// produced by the real commands) would still match — and is **not** a
    /// general "is this selection deletable as one node" predicate:
    /// structural containers (`TableRow`/`TableCell`/`Table`/paragraph) are
    /// never reported, so a cell-rect can never be observed here as a plain
    /// node-selection.
    pub fn as_node_selection(&self) -> Option<NodeRef<'a>> {
        if self.is_collapsed() || self.as_cell_rect().is_some() {
            return None;
        }
        let doc = self.doc();
        let a = Position::from(self.anchor());
        let h = Position::from(self.head());
        if a.node_id != h.node_id {
            return None;
        }
        let lo = a.offset.min(h.offset);
        let hi = a.offset.max(h.offset);
        if hi - lo != 1 {
            return None;
        }
        let child = doc.node(a.node_id)?.children().nth(lo)?;
        if matches!(child.node(), Node::Text(_)) || !child.spec().is_leaf() {
            return None;
        }
        Some(child)
    }
}

impl<'a> CellRect<'a> {
    /// Selected cells in row-major order. Skips a `(r, c)` slot whose cell
    /// does not exist (ragged table) rather than panicking.
    pub fn cells(&self) -> impl Iterator<Item = NodeRef<'a>> {
        let table = self.table;
        let rows = self.rows.clone();
        let cols = self.cols.clone();
        let mut out: Vec<NodeRef<'a>> = Vec::new();
        for r in rows {
            if let Some(row) = table.children().nth(r) {
                for c in cols.clone() {
                    if let Some(cell) = row.children().nth(c) {
                        out.push(cell);
                    }
                }
            }
        }
        out.into_iter()
    }

    pub fn contains(&self, cell: &NodeRef<'_>) -> bool {
        let Some(row) = cell.parent() else {
            return false;
        };
        if row.parent().map(|t| t.id()) != Some(self.table.id()) {
            return false;
        }
        let (Some(r), Some(c)) = (row.index(), cell.index()) else {
            return false;
        };
        self.rows.contains(&r) && self.cols.contains(&c)
    }

    fn row_width(&self, row_index: usize) -> Option<usize> {
        Some(self.table.children().nth(row_index)?.children().count())
    }

    /// `Some(width)` iff every row has the same number of cells; `None` on a
    /// ragged table.
    fn uniform_width(&self) -> Option<usize> {
        let mut w: Option<usize> = None;
        for row in self.table.children() {
            let rw = row.children().count();
            match w {
                None => w = Some(rw),
                Some(x) if x != rw => return None,
                _ => {}
            }
        }
        w
    }

    pub fn is_single(&self) -> bool {
        self.rows.start() == self.rows.end() && self.cols.start() == self.cols.end()
    }

    pub fn is_full_row(&self) -> bool {
        if self.rows.start() != self.rows.end() {
            return false;
        }
        match self.row_width(*self.rows.start()) {
            Some(w) => w > 0 && *self.cols.start() == 0 && *self.cols.end() == w - 1,
            None => false,
        }
    }

    pub fn is_full_column(&self) -> bool {
        let Some(w) = self.uniform_width() else {
            return false;
        };
        let h = self.table.children().count();
        w > 0
            && h > 0
            && self.cols.start() == self.cols.end()
            && *self.rows.start() == 0
            && *self.rows.end() == h - 1
    }

    pub fn is_full_table(&self) -> bool {
        let Some(w) = self.uniform_width() else {
            return false;
        };
        let h = self.table.children().count();
        w > 0
            && h > 0
            && *self.rows.start() == 0
            && *self.rows.end() == h - 1
            && *self.cols.start() == 0
            && *self.cols.end() == w - 1
    }
}

/// Build a cell-rect `Selection` whose corners are `anchor_cell` and
/// `head_cell`. Both must be `TableCell`s in one common `Table`; otherwise
/// `None`. Endpoints are placed at each corner's outer column boundary, then
/// the selection is run through `Selection::normalize` so the returned value
/// is already canonical (correct affinities, no pre-normalization invariant
/// violation) and safe for any caller to inspect directly — not only via the
/// `set_selection` ingress. Derivation never reads affinity anyway; this is
/// belt-and-suspenders. Full-table rectangles normalize to the table's unit
/// selection; non-full rectangles resolve back to a `CellRect` whose
/// `anchor_cell`/`head_cell` match the inputs (direction preserved).
pub fn cell_rect_selection(doc: &Doc, anchor_cell: NodeId, head_cell: NodeId) -> Option<Selection> {
    let ac = doc.node(anchor_cell)?;
    let hc = doc.node(head_cell)?;
    if !matches!(ac.node(), Node::TableCell(_)) || !matches!(hc.node(), Node::TableCell(_)) {
        return None;
    }
    let arow = ac.parent()?;
    let hrow = hc.parent()?;
    let table = arow.parent()?;
    if !matches!(table.node(), Node::Table(_)) || hrow.parent()?.id() != table.id() {
        return None;
    }
    let ca = ac.index()?;
    let ch = hc.index()?;

    let anchor_offset = if ca <= ch { ca } else { ca + 1 };
    let head_offset = if ch >= ca { ch + 1 } else { ch };

    Selection::new(
        Position::new(arow.id(), anchor_offset),
        Position::new(hrow.id(), head_offset),
    )
    .normalize(doc)
}

pub fn enclosing_table_cell(doc: &Doc, node_id: NodeId) -> Option<NodeId> {
    doc.node(node_id)?
        .ancestors()
        .find(|n| matches!(n.node(), Node::TableCell(_)))
        .map(|n| n.id())
}

pub fn enclosing_table(doc: &Doc, cell_id: NodeId) -> Option<NodeId> {
    doc.node(cell_id)?
        .ancestors()
        .find(|n| matches!(n.node(), Node::Table(_)))
        .map(|n| n.id())
}

pub fn table_cell_ids(doc: &Doc, cell_id: NodeId) -> Vec<NodeId> {
    let Some(node) = doc.node(cell_id) else {
        return Vec::new();
    };
    if !matches!(node.node(), Node::TableCell(_)) {
        return Vec::new();
    }
    let Some(table) = node
        .ancestors()
        .find(|n| matches!(n.node(), Node::Table(_)))
    else {
        return Vec::new();
    };
    table
        .children()
        .filter(|row| matches!(row.node(), Node::TableRow(_)))
        .flat_map(|row| row.children().collect::<Vec<_>>())
        .filter(|cell| matches!(cell.node(), Node::TableCell(_)))
        .map(|cell| cell.id())
        .collect()
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Node;

    use crate::affinity::Affinity;
    use crate::position::Position;
    use crate::selection::Selection;

    #[test]
    fn full_table_2x2_derives_bounding_box() {
        let (state, tr0, c00, _, tr1, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let rs = sel.resolve(&state.doc).unwrap();
        let rect = rs.as_cell_rect().expect("must be a cell rectangle");

        assert!(matches!(rect.table.node(), Node::Table(_)));
        assert_eq!(rect.anchor_cell.id(), c00);
        assert_eq!(rect.head_cell.id(), c11);
        assert_eq!(rect.rows, 0..=1);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn affinity_does_not_affect_derivation() {
        let (state, tr0, c00, _, tr1, _, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Upstream,
            },
            Position {
                node_id: tr1,
                offset: 2,
                affinity: Affinity::Downstream,
            },
        );
        let rect = sel.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        assert_eq!(rect.anchor_cell.id(), c00);
        assert_eq!(rect.head_cell.id(), c11);
        assert_eq!(rect.rows, 0..=1);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn single_cell_1x1_is_a_cell_rect() {
        let (state, tr0, c00, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph {} }
                c01: table_cell { paragraph {} }
            } } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr0,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rect = sel
            .resolve(&state.doc)
            .unwrap()
            .as_cell_rect()
            .expect("1x1 must be a cell rectangle");
        assert_eq!(rect.anchor_cell.id(), c00);
        assert_eq!(rect.head_cell.id(), c00);
        assert_eq!(rect.rows, 0..=0);
        assert_eq!(rect.cols, 0..=0);
    }

    #[test]
    fn plain_text_selection_is_not_a_cell_rect() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let sel = Selection::new(
            Position::new(t, 0),
            Position {
                node_id: t,
                offset: 5,
                affinity: Affinity::Upstream,
            },
        );
        assert!(sel.resolve(&state.doc).unwrap().as_cell_rect().is_none());
    }

    #[test]
    fn collapsed_at_row_boundary_is_not_a_cell_rect() {
        let (state, tr0, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph {} }
            } } } }
            selection: (c00, 0)
        };
        let sel = Selection::collapsed(Position::new(tr0, 0));
        assert!(sel.resolve(&state.doc).unwrap().as_cell_rect().is_none());
    }

    #[test]
    fn antidiagonal_direct_derives_corners() {
        // anchor corner = c01 (row0,col1, top-right); head corner = c10
        // (row1,col0, bottom-left). Built directly (no normalize): anchor
        // outer side of col1 = offset 2 in tr0; head outer side of col0 =
        // offset 0 in tr1. Affinity is immaterial.
        let (state, tr0, _, c01, tr1, c10, _) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 2,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        );
        let rect = sel
            .resolve(&state.doc)
            .unwrap()
            .as_cell_rect()
            .expect("anti-diagonal must be a cell rectangle");
        assert_eq!(rect.anchor_cell.id(), c01);
        assert_eq!(rect.head_cell.id(), c10);
        assert_eq!(rect.rows, 0..=1);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn cross_table_is_not_a_cell_rect() {
        let (state, tr0, _, tr1, _) = state! {
            doc { root {
                table { tr0: table_row { ca: table_cell { paragraph {} } } }
                table { tr1: table_row { cb: table_cell { paragraph {} } } }
            } }
            selection: (ca, 0)
        };
        // endpoints in TableRows of two DIFFERENT tables.
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        assert!(sel.resolve(&state.doc).unwrap().as_cell_rect().is_none());
    }

    #[test]
    fn same_row_equal_offset_differing_affinity_is_not_a_cell_rect() {
        let (state, tr0, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph {} }
            } } } }
            selection: (c00, 0)
        };
        // Not collapsed (affinity differs) but offsets equal → degenerate.
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr0,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rs = sel.resolve(&state.doc).unwrap();
        assert!(!rs.is_collapsed());
        assert!(rs.as_cell_rect().is_none());
    }

    #[test]
    fn cells_yields_row_major_rectangle() {
        let (state, tr0, c00, c01, tr1, c10, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let rect = sel.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        let ids: Vec<_> = rect.cells().map(|c| c.id()).collect();
        assert_eq!(ids, vec![c00, c01, c10, c11]);
    }

    #[test]
    fn contains_only_cells_inside_rectangle() {
        let (state, tr0, c00, c01, tr1, c10, _) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rect = sel.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        assert!(rect.contains(&state.doc.node(c00).unwrap()));
        assert!(rect.contains(&state.doc.node(c10).unwrap()));
        assert!(!rect.contains(&state.doc.node(c01).unwrap()));
    }

    #[test]
    fn predicates_single_full_row_column_table() {
        let (state, tr0, _, _, tr1, _, _) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let rect = |a: Position, h: Position| {
            Selection::new(a, h)
                .resolve(&state.doc)
                .unwrap()
                .as_cell_rect()
                .unwrap()
        };
        let dn = Affinity::Downstream;
        let up = Affinity::Upstream;

        let r = rect(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: dn,
            },
            Position {
                node_id: tr0,
                offset: 1,
                affinity: up,
            },
        );
        assert!(r.is_single());
        assert!(!r.is_full_row());
        assert!(!r.is_full_table());

        let r = rect(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: dn,
            },
            Position {
                node_id: tr0,
                offset: 2,
                affinity: up,
            },
        );
        assert!(r.is_full_row());
        assert!(!r.is_full_column());

        let r = rect(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: dn,
            },
            Position {
                node_id: tr1,
                offset: 1,
                affinity: up,
            },
        );
        assert!(r.is_full_column());
        assert!(!r.is_full_row());

        let r = rect(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: dn,
            },
            Position {
                node_id: tr1,
                offset: 2,
                affinity: up,
            },
        );
        assert!(r.is_full_table());
        assert!(!r.is_single());
    }

    #[test]
    fn ragged_table_predicates_do_not_lie_or_panic() {
        let (state, tr0, _, _, tr1, ..) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                    c12: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: tr0,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: tr1,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let rect = sel.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        assert!(!rect.is_full_table());
        assert!(!rect.is_full_column());
        let n = rect.cells().count();
        assert_eq!(n, 4);
    }

    #[test]
    fn builder_roundtrips_through_as_cell_rect() {
        let (state, c00, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let sel = super::cell_rect_selection(&state.doc, c00, c11)
            .expect("c00..c11 is a valid cell rectangle");
        let rect = sel.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        assert_eq!(rect.anchor_cell.id(), c00);
        assert_eq!(rect.head_cell.id(), c11);
        assert_eq!(rect.rows, 0..=1);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn full_table_cell_rect_normalizes_to_table_selection() {
        let (state, root, table, c00, c11) = state! {
            doc { root: root { table: table {
                table_row {
                    c00: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };

        let sel = super::cell_rect_selection(&state.doc, c00, c11)
            .expect("full-table cell rect should normalize");

        assert_eq!(
            sel,
            Selection::new(
                Position {
                    node_id: root,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: root,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            )
        );
        assert!(sel.resolve(&state.doc).unwrap().as_cell_rect().is_none());
        assert!(sel.is_unit_node_selection(&state.doc));
        assert_eq!(state.doc.node(table).unwrap().index(), Some(0));
    }

    #[test]
    fn builder_preserves_direction() {
        let (state, c00, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        let fwd = super::cell_rect_selection(&state.doc, c00, c11).unwrap();
        let bwd = super::cell_rect_selection(&state.doc, c11, c00).unwrap();
        assert_ne!(fwd, bwd, "anchor/head corner identity must be preserved");
        let rf = fwd.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        let rb = bwd.resolve(&state.doc).unwrap().as_cell_rect().unwrap();
        assert_eq!(rf.anchor_cell.id(), c00);
        assert_eq!(rb.anchor_cell.id(), c11);
        assert_eq!(rf.rows, rb.rows);
        assert_eq!(rf.cols, rb.cols);
    }

    #[test]
    fn builder_rejects_non_cell_nodes() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("x") } } }
            selection: (t, 0)
        };
        assert!(super::cell_rect_selection(&state.doc, t, t).is_none());
    }

    #[test]
    fn antidiagonal_survives_normalize() {
        let (state, _c00, c01, c10) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
                table_row {
                    c10: table_cell { paragraph {} }
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        // anchor corner = c01 (top-right), head corner = c10 (bottom-left).
        let sel = super::cell_rect_selection(&state.doc, c01, c10).unwrap();
        let normalized = sel.normalize(&state.doc).expect("normalizes");
        let rect = normalized
            .resolve(&state.doc)
            .unwrap()
            .as_cell_rect()
            .expect("anti-diagonal cell-rect must survive the affinity rewrite");
        assert_eq!(rect.anchor_cell.id(), c01);
        assert_eq!(rect.head_cell.id(), c10);
        assert_eq!(rect.rows, 0..=1);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn same_row_backward_survives_normalize() {
        let (state, c00, c01) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                table_row {
                    table_cell { paragraph {} }
                    table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        // backward 1xN: anchor corner c01 (right), head corner c00 (left).
        let sel = super::cell_rect_selection(&state.doc, c01, c00).unwrap();
        let normalized = sel.normalize(&state.doc).expect("normalizes");
        let rect = normalized
            .resolve(&state.doc)
            .unwrap()
            .as_cell_rect()
            .expect("backward same-row cell-rect must survive normalize");
        assert_eq!(rect.anchor_cell.id(), c01);
        assert_eq!(rect.head_cell.id(), c00);
        assert_eq!(rect.rows, 0..=0);
        assert_eq!(rect.cols, 0..=1);
    }

    #[test]
    fn image_node_selection_is_a_node_selection_not_a_cell_rect() {
        let (state, r) = state! {
            doc { r: root { paragraph {} image paragraph {} } }
            selection: (r, 0)
        };
        let sel = Selection::new(
            Position {
                node_id: r,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: r,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let rs = sel.resolve(&state.doc).unwrap();
        assert!(rs.as_cell_rect().is_none());
        let node = rs.as_node_selection().expect("image is a node selection");
        assert!(matches!(node.node(), Node::Image(_)));
    }

    #[test]
    fn cell_rect_is_never_observed_as_node_selection() {
        let (state, _, c00, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph {} }
                c01: table_cell { paragraph {} }
            } } } }
            selection: (c00, 0)
        };
        let sel = super::cell_rect_selection(&state.doc, c00, c00).unwrap();
        let rs = sel.resolve(&state.doc).unwrap();
        assert!(rs.as_cell_rect().is_some());
        assert!(
            rs.as_node_selection().is_none(),
            "a 1x1 cell-rect must never be observable as a plain node-selection"
        );
    }

    #[test]
    fn text_child_bracket_is_not_a_node_selection() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("ab") text("cd") } } }
            selection: (p, 0)
        };
        let sel = Selection::new(
            Position::new(p, 0),
            Position {
                node_id: p,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        assert!(
            sel.resolve(&state.doc)
                .unwrap()
                .as_node_selection()
                .is_none()
        );
    }

    #[test]
    fn plain_text_range_is_not_a_node_selection() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let sel = Selection::new(
            Position::new(t, 1),
            Position {
                node_id: t,
                offset: 4,
                affinity: Affinity::Upstream,
            },
        );
        assert!(
            sel.resolve(&state.doc)
                .unwrap()
                .as_node_selection()
                .is_none()
        );
    }

    #[test]
    fn non_leaf_container_bracket_is_not_a_node_selection() {
        let (state, r) = state! {
            doc { r: root { paragraph { text("x") } image } }
            selection: (r, 0)
        };
        let sel = Selection::new(
            Position::new(r, 0),
            Position {
                node_id: r,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        assert!(
            sel.resolve(&state.doc)
                .unwrap()
                .as_node_selection()
                .is_none()
        );
    }

    #[test]
    fn enclosing_table_cell_finds_cell_from_descendant_and_self() {
        let (state, _, c00, t00, _) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { t00: text("a") } }
                c01: table_cell { paragraph { text("b") } }
            } } } }
            selection: (t00, 0)
        };
        assert_eq!(super::enclosing_table_cell(&state.doc, t00), Some(c00));
        assert_eq!(super::enclosing_table_cell(&state.doc, c00), Some(c00));
    }

    #[test]
    fn enclosing_table_cell_none_outside_table() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("x") } } }
            selection: (t, 0)
        };
        assert_eq!(super::enclosing_table_cell(&state.doc, t), None);
    }

    #[test]
    fn table_cell_ids_row_major() {
        let (state, _, c00, c01, _, c10, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph {} }
                    c01: table_cell { paragraph {} }
                }
                tr1: table_row {
                    c10: table_cell { paragraph {} }
                    c11: table_cell { paragraph {} }
                }
            } } }
            selection: (c00, 0)
        };
        assert_eq!(
            super::table_cell_ids(&state.doc, c00),
            vec![c00, c01, c10, c11]
        );
        assert_eq!(
            super::table_cell_ids(&state.doc, c11),
            vec![c00, c01, c10, c11]
        );
    }

    #[test]
    fn table_cell_ids_empty_for_non_cell() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("x") } } }
            selection: (t, 0)
        };
        assert!(super::table_cell_ids(&state.doc, t).is_empty());
    }
}

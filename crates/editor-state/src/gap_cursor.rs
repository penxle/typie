use editor_model::{Doc, Node, NodeId, NodeRef, NodeType, Schema};

use crate::affinity::Affinity;
use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;
use crate::selection::Selection;

/// `Some(first_child)` iff the document's first child is a unit (atom leaf
/// or monolithic block). Used to detect a gap before a leading unit. The
/// container here is always the document root, whose content always admits
/// a leading paragraph, so no position-specific content check is needed.
pub(crate) fn leading_unit(doc: &Doc) -> Option<NodeRef<'_>> {
    let root = doc.node(NodeId::ROOT)?;
    let first = root.children().next()?;
    Schema::node_spec(first.as_type())
        .is_unit()
        .then_some(first)
}

/// True iff `(node_id, index)` is a boundary between two adjacent
/// monolithic block siblings where inserting a paragraph at `index` keeps
/// the parent's content model valid (checked position-specifically via
/// `ContentExpr::matches_sequence`). This is the single source of truth
/// for between-monolithic gap detection, so the normalization invariant
/// and this classification stay consistent.
pub(crate) fn between_monolithic_at(doc: &Doc, node_id: NodeId, index: usize) -> bool {
    let Some(node) = doc.node(node_id) else {
        return false;
    };
    if matches!(node.node(), Node::Text(_)) {
        return false;
    }
    let children: Vec<NodeRef<'_>> = node.children().collect();
    let count = children.len();
    if index == 0 || index >= count {
        return false;
    }
    if !Schema::node_spec(children[index - 1].as_type()).monolithic
        || !Schema::node_spec(children[index].as_type()).monolithic
    {
        return false;
    }
    let mut seq: Vec<NodeType> = children.iter().map(|c| c.as_type()).collect();
    seq.insert(index, NodeType::Paragraph);
    Schema::node_spec(node.as_type())
        .content
        .matches_sequence(&seq)
}

/// A gap cursor derived from a collapsed `Selection`. The document is not
/// mutated; this is a positional convention over the existing `Selection`,
/// classified here (same pattern as `cell_selection.rs`).
pub enum GapCursor<'a> {
    /// `(root, 0, Upstream)` when the document's first child is a unit.
    LeadingUnit { unit: NodeRef<'a> },
    /// `(parent, index)` between two `monolithic` siblings.
    BetweenMonolithic {
        parent: NodeRef<'a>,
        before: NodeRef<'a>,
        after: NodeRef<'a>,
        index: usize,
    },
}

pub(crate) fn gap_cursor_at(doc: &Doc, p: Position) -> Option<GapCursor<'_>> {
    if p.node_id == NodeId::ROOT && p.offset == 0 && p.affinity == Affinity::Upstream {
        return leading_unit(doc).map(|unit| GapCursor::LeadingUnit { unit });
    }

    if !between_monolithic_at(doc, p.node_id, p.offset) {
        return None;
    }
    let node = doc.node(p.node_id)?;
    let before = node.children().nth(p.offset - 1)?;
    let after = node.children().nth(p.offset)?;
    Some(GapCursor::BetweenMonolithic {
        parent: node,
        before,
        after,
        index: p.offset,
    })
}

impl<'a> ResolvedSelection<'a> {
    /// `Some` iff this collapsed selection encodes a gap cursor. A
    /// cell-rect / node-selection is by definition non-collapsed, so the
    /// `is_collapsed()` gate alone makes them mutually exclusive with a
    /// gap cursor; no extra `as_cell_rect`/`as_node_selection` check is
    /// needed here.
    pub fn as_gap_cursor(&self) -> Option<GapCursor<'a>> {
        if !self.is_collapsed() {
            return None;
        }
        let doc = self.doc();
        let p = Position::from(self.head());

        gap_cursor_at(doc, p)
    }
}

/// Build the leading-unit gap cursor selection, or `None` if the
/// document's first child is not a unit. Run through `normalize` so any
/// caller may inspect it directly (mirrors `cell_rect_selection`).
pub fn gap_cursor_selection_leading(doc: &Doc) -> Option<Selection> {
    leading_unit(doc)?;
    Selection::collapsed(Position {
        node_id: NodeId::ROOT,
        offset: 0,
        affinity: Affinity::Upstream,
    })
    .normalize(doc)
}

/// Build the between-monolithic gap cursor at `(parent, index)`, or
/// `None` if the shared structural predicate rejects it.
pub fn gap_cursor_selection_between(doc: &Doc, parent: NodeId, index: usize) -> Option<Selection> {
    if !between_monolithic_at(doc, parent, index) {
        return None;
    }
    Selection::collapsed(Position::new(parent, index)).normalize(doc)
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::NodeId;

    use super::*;
    use crate::cell_rect_selection;

    #[test]
    fn leading_unit_some_for_leading_image() {
        let (d, ..) = doc! { root { image paragraph { text("b") } } };
        assert!(leading_unit(&d).is_some());
    }

    #[test]
    fn leading_unit_some_for_leading_fold() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            }
        };
        assert!(leading_unit(&d).is_some());
    }

    #[test]
    fn leading_unit_none_for_leading_paragraph() {
        let (d, ..) = doc! { root { paragraph { text("a") } } };
        assert!(leading_unit(&d).is_none());
    }

    #[test]
    fn between_two_folds_in_root_is_true() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        assert!(between_monolithic_at(&d, NodeId::ROOT, 1));
    }

    #[test]
    fn between_two_folds_in_fold_content_is_true() {
        let (d, fc) = doc! {
            root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content {
                        fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                        fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                    }
                }
                paragraph {}
            }
        };
        assert!(between_monolithic_at(&d, fc, 1));
    }

    #[test]
    fn between_two_folds_in_table_cell_is_true() {
        let (d, cell) = doc! {
            root {
                table {
                    table_row {
                        cell: table_cell {
                            fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                            fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                        }
                    }
                }
                paragraph {}
            }
        };
        assert!(between_monolithic_at(&d, cell, 1));
    }

    #[test]
    fn between_image_and_fold_is_false_not_both_monolithic() {
        let (d, ..) = doc! {
            root {
                paragraph { text("p") }
                image
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                paragraph {}
            }
        };
        assert!(!between_monolithic_at(&d, NodeId::ROOT, 2));
    }

    #[test]
    fn between_two_horizontal_rules_is_false_hr_not_monolithic() {
        let (d, ..) = doc! {
            root { horizontal_rule horizontal_rule paragraph {} }
        };
        assert!(!between_monolithic_at(&d, NodeId::ROOT, 1));
    }

    #[test]
    fn between_two_paragraphs_is_false() {
        let (d, ..) = doc! { root { paragraph { text("a") } paragraph { text("b") } } };
        assert!(!between_monolithic_at(&d, NodeId::ROOT, 1));
    }

    #[test]
    fn index_zero_and_out_of_range_are_false() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        assert!(!between_monolithic_at(&d, NodeId::ROOT, 0));
        assert!(!between_monolithic_at(&d, NodeId::ROOT, 99));
    }

    #[test]
    fn leading_image_upstream_is_gap_cursor() {
        let (d, ..) = doc! { root { image paragraph { text("b") } } };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 0,
            affinity: Affinity::Upstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(matches!(
            rs.as_gap_cursor(),
            Some(GapCursor::LeadingUnit { .. })
        ));
    }

    #[test]
    fn leading_fold_upstream_is_gap_cursor() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph {}
            }
        };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 0,
            affinity: Affinity::Upstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(matches!(
            rs.as_gap_cursor(),
            Some(GapCursor::LeadingUnit { .. })
        ));
    }

    #[test]
    fn leading_unit_downstream_is_not_gap_cursor() {
        let (d, ..) = doc! { root { image paragraph { text("b") } } };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 0,
            affinity: Affinity::Downstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn leading_paragraph_is_not_gap_cursor() {
        let (d, ..) = doc! { root { paragraph { text("a") } } };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 0,
            affinity: Affinity::Upstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn between_two_folds_root_is_gap_cursor_both_affinities() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        for aff in [Affinity::Downstream, Affinity::Upstream] {
            let sel = Selection::collapsed(Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: aff,
            });
            let rs = sel.resolve(&d).unwrap();
            match rs.as_gap_cursor() {
                Some(GapCursor::BetweenMonolithic { index, .. }) => assert_eq!(index, 1),
                _ => panic!("expected BetweenMonolithic at affinity {:?}", aff),
            }
        }
    }

    #[test]
    fn between_two_folds_in_fold_content_is_gap_cursor() {
        let (d, fc) = doc! {
            root {
                fold {
                    fold_title { text("t") }
                    fc: fold_content {
                        fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                        fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                    }
                }
                paragraph {}
            }
        };
        let sel = Selection::collapsed(Position::new(fc, 1));
        let rs = sel.resolve(&d).unwrap();
        assert!(matches!(
            rs.as_gap_cursor(),
            Some(GapCursor::BetweenMonolithic { .. })
        ));
    }

    #[test]
    fn between_two_folds_in_table_cell_is_gap_cursor() {
        let (d, cell) = doc! {
            root {
                table {
                    table_row {
                        cell: table_cell {
                            fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                            fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                        }
                    }
                }
                paragraph {}
            }
        };
        let sel = Selection::collapsed(Position::new(cell, 1));
        let rs = sel.resolve(&d).unwrap();
        assert!(matches!(
            rs.as_gap_cursor(),
            Some(GapCursor::BetweenMonolithic { .. })
        ));
    }

    #[test]
    fn between_image_and_fold_is_not_gap_cursor() {
        let (d, ..) = doc! {
            root {
                paragraph { text("p") }
                image
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                paragraph {}
            }
        };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 2,
            affinity: Affinity::Downstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn between_two_paragraphs_is_not_gap_cursor() {
        let (d, ..) = doc! { root { paragraph { text("a") } paragraph { text("b") } } };
        let sel = Selection::collapsed(Position {
            node_id: NodeId::ROOT,
            offset: 1,
            affinity: Affinity::Downstream,
        });
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn collapsed_text_is_not_gap_cursor() {
        let (d, t) = doc! { root { paragraph { t: text("hi") } } };
        let sel = Selection::collapsed(Position::new(t, 1));
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn non_collapsed_text_range_is_not_gap_cursor() {
        let (d, t) = doc! { root { paragraph { t: text("hello") } } };
        let sel = Selection::new(
            Position::new(t, 1),
            Position {
                node_id: t,
                offset: 4,
                affinity: Affinity::Upstream,
            },
        );
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn node_selection_is_not_gap_cursor() {
        let (d, r) = doc! { r: root { paragraph {} image paragraph {} } };
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
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn cell_rect_is_not_gap_cursor() {
        let (d, c00, ..) = doc! {
            root {
                table {
                    table_row {
                        c00: table_cell { paragraph {} }
                        c01: table_cell { paragraph {} }
                    }
                }
                paragraph {}
            }
        };
        let sel = cell_rect_selection(&d, c00, c00).expect("1x1 cell-rect builds");
        let rs = sel.resolve(&d).unwrap();
        assert!(rs.as_cell_rect().is_some(), "precondition: is a cell-rect");
        assert!(rs.as_gap_cursor().is_none());
    }

    #[test]
    fn builder_leading_roundtrips_and_none_for_paragraph() {
        let (d1, ..) = doc! { root { image paragraph { text("b") } } };
        let sel = gap_cursor_selection_leading(&d1).expect("leading image is a gap");
        assert!(matches!(
            sel.resolve(&d1).unwrap().as_gap_cursor(),
            Some(GapCursor::LeadingUnit { .. })
        ));

        let (d2, ..) = doc! { root { paragraph { text("a") } } };
        assert!(gap_cursor_selection_leading(&d2).is_none());
    }

    #[test]
    fn builder_between_roundtrips_and_bounds_none() {
        let (d, ..) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        let sel =
            gap_cursor_selection_between(&d, NodeId::ROOT, 1).expect("between two folds is a gap");
        match sel.resolve(&d).unwrap().as_gap_cursor() {
            Some(GapCursor::BetweenMonolithic { index, .. }) => assert_eq!(index, 1),
            _ => panic!("between builder must roundtrip to BetweenMonolithic"),
        }
        assert!(gap_cursor_selection_between(&d, NodeId::ROOT, 0).is_none());
        assert!(gap_cursor_selection_between(&d, NodeId::ROOT, 99).is_none());
    }

    #[test]
    fn builder_between_none_for_non_monolithic() {
        let (d, ..) = doc! { root { paragraph { text("a") } paragraph { text("b") } } };
        assert!(gap_cursor_selection_between(&d, NodeId::ROOT, 1).is_none());
    }
}

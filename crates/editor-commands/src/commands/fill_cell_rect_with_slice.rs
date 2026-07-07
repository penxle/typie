use std::collections::BTreeMap;

use editor_clipboard::Slice;
use editor_crdt::Dot;
use editor_model::{Fragment, Modifier, PlainNode, PlainParagraphNode};
use editor_state::{PendingModifiers, Position, Selection, apply_pending};
use editor_transaction::Transaction;

use crate::helpers::{
    cell_first_charlike_block, consume_pending_modifiers, find_first_text_position,
    paint_block_uniformly, replace_cell_children, resolve_effective_modifiers,
};
use crate::types::SliceProvenance;
use crate::{CommandError, CommandResult};

/// Replace every cell in a cell-rect selection with the same slice content.
/// Inline-only slices are wrapped in a single paragraph so the cell stays
/// schema-valid; block slices are inserted verbatim.
pub fn fill_cell_rect_with_slice(
    tr: &mut Transaction,
    slice: Slice,
    provenance: SliceProvenance,
) -> CommandResult {
    let blocks = slice_to_cell_blocks(&slice);
    if blocks.is_empty() {
        return Ok(false);
    }

    let (anchor_cell_id, cell_ids) = {
        let Some(sel) = tr.selection() else {
            return Ok(false);
        };
        let view = tr.view();
        let Some(rs) = sel.resolve(&view) else {
            return Ok(false);
        };
        let Some(rect) = rs.as_cell_rect() else {
            return Ok(false);
        };
        let ids: Vec<Dot> = rect.cells().into_iter().map(|c| c.id()).collect();
        if ids.is_empty() {
            return Ok(false);
        }
        (ids[0], ids)
    };

    let pending = if provenance.is_plain() {
        let pending = tr.pending_modifiers().clone();
        consume_pending_modifiers(tr)?;
        Some(pending)
    } else {
        None
    };

    tr.batch::<_, CommandError>(|tr| {
        for cell_id in &cell_ids {
            let paint = pending
                .as_ref()
                .map(|pending| cell_plain_paint(tr, *cell_id, pending));
            replace_cell_children(tr, *cell_id, &blocks)?;
            if let Some(paint) = &paint {
                let block_ids: Vec<Dot> = {
                    let view = tr.view();
                    view.node(*cell_id)
                        .map(|cell| cell.child_blocks().map(|b| b.id()).collect())
                        .unwrap_or_default()
                };
                for block_id in block_ids {
                    paint_block_uniformly(tr, block_id, paint)?;
                }
            }
        }
        Ok(())
    })?;

    let cursor = find_first_text_position(&tr.view(), anchor_cell_id)
        .unwrap_or_else(|| Position::new(anchor_cell_id, 0));
    tr.set_selection(Some(Selection::collapsed(cursor)))?;
    Ok(true)
}

fn cell_plain_paint(tr: &Transaction, cell_id: Dot, pending: &PendingModifiers) -> Vec<Modifier> {
    let state = tr.state();
    let block = {
        let view = state.view();
        cell_first_charlike_block(&view, cell_id)
            .or_else(|| find_first_text_position(&view, cell_id).map(|p| p.node))
    };
    match block {
        Some(block) => resolve_effective_modifiers(&state.projected, block, 0, pending),
        None => {
            let mut map = BTreeMap::new();
            apply_pending(&mut map, pending);
            map.into_values().collect()
        }
    }
}

fn slice_to_cell_blocks(slice: &Slice) -> Vec<Fragment> {
    let top_children: Vec<&Fragment> = match &slice.fragment.node {
        PlainNode::Root(_) => slice.fragment.children.iter().collect(),
        _ => vec![&slice.fragment],
    };

    let mut out: Vec<Fragment> = Vec::new();
    let mut inline_run: Vec<Fragment> = Vec::new();
    let flush = |run: &mut Vec<Fragment>, out: &mut Vec<Fragment>| {
        if !run.is_empty() {
            out.push(Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: std::mem::take(run),
            });
        }
    };
    for child in top_children {
        match &child.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_) => {
                inline_run.push(child.clone())
            }
            _ => {
                flush(&mut inline_run, &mut out);
                out.push(child.clone());
            }
        }
    }
    flush(&mut inline_run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::NodeType;
    use editor_state::cell_rect_selection;

    use super::*;
    use crate::test_utils::*;

    fn with_cell_rect(initial: editor_state::State, anchor: Dot, head: Dot) -> editor_state::State {
        let sel = {
            let v = initial.view();
            cell_rect_selection(anchor, head, &v).unwrap()
        };
        editor_state::State {
            selection: Some(sel),
            ..initial
        }
    }

    fn cell_text(view: &editor_model::DocView, id: Dot) -> String {
        let mut s = String::new();
        if let Some(n) = view.node(id) {
            for d in n.descendants() {
                if let editor_model::ChildView::Leaf(l) = d
                    && let Some(c) = l.as_char()
                {
                    s.push(c);
                }
            }
        }
        s
    }

    fn cell_blocks(view: &editor_model::DocView, cell_id: Dot) -> Vec<Dot> {
        view.node(cell_id)
            .map(|c| c.child_blocks().map(|b| b.id()).collect())
            .unwrap_or_default()
    }

    fn block_all_inline_have(
        view: &editor_model::DocView,
        block: Dot,
        modifier: &Modifier,
    ) -> bool {
        let Some(node) = view.node(block) else {
            return false;
        };
        let mut count = 0;
        for (i, c) in node.children().enumerate() {
            if matches!(c, editor_model::ChildView::Leaf(_)) {
                count += 1;
                if !node.leaf_own_modifiers_at(i).iter().any(|m| m == modifier) {
                    return false;
                }
            }
        }
        count > 0
    }

    fn carry_of(state: &editor_state::State, block: Dot) -> Vec<Modifier> {
        state
            .projected
            .carry_modifiers(block)
            .into_values()
            .collect()
    }

    #[test]
    fn returns_false_when_selection_is_not_cell_rect() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        transact_fail!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            Slice::from_text("hi"),
            SliceProvenance::Formatted
        ));
    }

    #[test]
    fn fills_every_selected_cell_with_inline_slice() {
        let (state, c00, c01, c10, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c11);
        let slice = Slice::from_text("hi");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let v = after.view();
        for cid in [c00, c01, c10, c11] {
            assert_eq!(cell_text(&v, cid), "hi");
        }
    }

    #[test]
    fn keeps_table_structure_and_cell_ids() {
        let (state, tbl, c00, c01, c10, c11) = state! {
            doc { root { tbl: table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c11);
        let slice = Slice::from_text("X");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let v = after.view();
        let table = v.node(tbl).expect("table survives fill");
        assert_eq!(table.node_type(), NodeType::Table);
        assert_eq!(table.child_blocks().count(), 2);
        for cid in [c00, c01, c10, c11] {
            let cell = v.node(cid).expect("cell id stable");
            assert_eq!(cell.node_type(), NodeType::TableCell);
            assert_eq!(cell.child_blocks().count(), 1);
        }
    }

    #[test]
    fn fills_partial_rect_only() {
        let (state, _, c00, c01, c02) = state! {
            doc { root { table { tr0: table_row {
                c00: table_cell { paragraph { text("a") } }
                c01: table_cell { paragraph { text("b") } }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("Z");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let v = after.view();
        assert_eq!(cell_text(&v, c00), "Z");
        assert_eq!(cell_text(&v, c01), "Z");
        assert_eq!(cell_text(&v, c02), "c");
    }

    #[test]
    fn plain_fill_derives_paint_and_carry_per_cell() {
        let (state, c00, c01, _c02) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell { paragraph { text("b") [font_size(2000)] } }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("x\ny");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let v = after.view();
        let bold = Modifier::Bold;
        let fs = Modifier::FontSize { value: 2000 };

        let b00 = cell_blocks(&v, c00);
        assert_eq!(b00.len(), 2, "two pasted lines become two paragraphs");
        for b in &b00 {
            assert!(
                block_all_inline_have(&v, *b, &bold),
                "c00 chars use its own bold"
            );
            assert!(
                carry_of(&after, *b).iter().any(|m| *m == bold),
                "c00 carry bold"
            );
            assert!(
                !block_all_inline_have(&v, *b, &fs),
                "c00 does not inherit c01's font size"
            );
        }

        let b01 = cell_blocks(&v, c01);
        assert_eq!(b01.len(), 2);
        for b in &b01 {
            assert!(
                block_all_inline_have(&v, *b, &fs),
                "c01 chars use its own font size"
            );
            assert!(
                carry_of(&after, *b).iter().any(|m| *m == fs),
                "c01 carry font size"
            );
            assert!(
                !block_all_inline_have(&v, *b, &bold),
                "c01 does not inherit c00's bold"
            );
        }
    }

    #[test]
    fn plain_fill_overlays_pending_on_cell_carry_and_consumes_once() {
        let (state, c00, c01, _c02) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell { paragraph { text("b") } }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
            pending_modifiers: [italic]
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("x\ny");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let v = after.view();
        let bold = Modifier::Bold;
        let italic = Modifier::Italic;

        for b in cell_blocks(&v, c00) {
            assert!(block_all_inline_have(&v, b, &bold), "c00 keeps its bold");
            assert!(
                block_all_inline_have(&v, b, &italic),
                "pending italic overlays c00"
            );
            let carry = carry_of(&after, b);
            assert!(carry.iter().any(|m| *m == bold));
            assert!(carry.iter().any(|m| *m == italic));
        }
        for b in cell_blocks(&v, c01) {
            assert!(
                !block_all_inline_have(&v, b, &bold),
                "c01 has no bold to inherit"
            );
            assert!(
                block_all_inline_have(&v, b, &italic),
                "pending italic overlays c01"
            );
            let carry = carry_of(&after, b);
            assert!(!carry.iter().any(|m| *m == bold));
            assert!(carry.iter().any(|m| *m == italic));
        }

        assert!(
            after.pending_modifiers.is_empty(),
            "plain cell-rect paste consumes the pending format once"
        );
    }

    #[test]
    fn plain_fill_uses_existing_carry_for_empty_cell() {
        let (state, c00, c01, _c02) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell { paragraph carry([font_size(2000)]) {} }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("x\ny");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let v = after.view();
        let fs = Modifier::FontSize { value: 2000 };

        for b in cell_blocks(&v, c01) {
            assert!(
                block_all_inline_have(&v, b, &fs),
                "an empty cell derives paint from its existing carry record"
            );
            assert!(carry_of(&after, b).iter().any(|m| *m == fs));
        }
    }

    #[test]
    fn plain_fill_multiblock_cell_derives_from_first_charlike() {
        let (state, c00, c01, _c02) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell {
                    paragraph {}
                    paragraph { text("b") [font_size(2000)] }
                }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("x\ny");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let v = after.view();
        let fs = Modifier::FontSize { value: 2000 };

        let b01 = cell_blocks(&v, c01);
        assert_eq!(b01.len(), 2, "two pasted lines become two paragraphs");
        for b in &b01 {
            assert!(
                block_all_inline_have(&v, *b, &fs),
                "an empty first paragraph does not mask the cell's first charlike font size"
            );
            assert!(
                carry_of(&after, *b).iter().any(|m| *m == fs),
                "new paragraphs carry the first charlike font size"
            );
        }
    }

    #[test]
    fn plain_fill_leading_unit_atom_cell_derives_from_formatted_paragraph() {
        let (state, c00, c01, _c02) = state! {
            doc { root { table { table_row {
                c00: table_cell { paragraph { text("a") [bold] } }
                c01: table_cell {
                    horizontal_rule
                    paragraph { text("b") [font_size(2000)] }
                }
                c02: table_cell { paragraph { text("c") } }
            } } } }
            selection: (c00, 0)
        };
        let initial = with_cell_rect(state, c00, c01);
        let slice = Slice::from_text("x\ny");
        let (after, ..) = transact!(initial, |tr| fill_cell_rect_with_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let v = after.view();
        let fs = Modifier::FontSize { value: 2000 };

        let b01 = cell_blocks(&v, c01);
        assert_eq!(b01.len(), 2);
        for b in &b01 {
            assert!(
                block_all_inline_have(&v, *b, &fs),
                "a leading unit atom does not mask the cell's first charlike font size"
            );
            assert!(carry_of(&after, *b).iter().any(|m| *m == fs));
        }
    }
}

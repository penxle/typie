use crate::model::{Doc, Fragment, Node, NodeId};
use crate::state::{Position, Selection, SelectionKind, is_block_position, position_in_selection};
use crate::transaction::DeleteResult;
use crate::types::Affinity;
use anyhow::Result;

use super::Transaction;

impl Transaction {
    pub fn can_drop(&self, target: Position) -> bool {
        if position_in_selection(self.doc(), target, self.selection()) {
            return false;
        }

        if self.selection_contains_page_break() && !self.is_valid_page_break_drop_position(target) {
            return false;
        }

        true
    }

    pub fn drag_and_drop(&mut self, target: Position) -> Result<bool> {
        if !self.can_drop(target) {
            return Ok(false);
        }

        let selection = *self.selection();
        self.relocate_selection(selection, target)?;
        Ok(true)
    }

    pub fn drag_and_copy(&mut self, target: Position) -> Result<bool> {
        if !self.can_drop(target) {
            return Ok(false);
        }

        let selection = *self.selection();
        let selection_kind = selection.classify(self.doc())?;
        let anchor_before_head = selection.anchor_before_head(self.doc());

        let fragment = Fragment::new_from_selection(self.doc(), &selection)?;
        if fragment.is_empty() {
            return Ok(false);
        }

        let is_block_drop = is_block_position(self.doc(), target);
        let fragment = prepare_fragment(fragment, self.doc().schema(), is_block_drop);

        self.set_selection(Selection::collapsed(target));
        let result = self.insert_fragment(target, fragment)?;

        let selection = match selection_kind {
            SelectionKind::InlineRange => result.as_inline_range_selection(self.doc()),
            _ => result.as_range_selection(),
        };

        if let Some(selection) = selection {
            let selection = if anchor_before_head {
                selection
            } else {
                Selection::new(selection.head, selection.anchor)
            };
            self.set_selection(selection);
        }

        Ok(result.inserted())
    }

    fn selection_contains_page_break(&self) -> bool {
        let Ok(fragment) = self.selection().extract_fragment(self.doc()) else {
            return false;
        };

        fragment
            .iter()
            .any(|(_, node)| matches!(node.data(), Node::PageBreak(_)))
    }

    fn is_valid_page_break_drop_position(&self, position: Position) -> bool {
        let doc = self.doc();

        if is_block_position(doc, position) {
            return position.node_id == NodeId::ROOT;
        }

        let Some(node) = doc.node(position.node_id) else {
            return false;
        };

        if !matches!(node.node(), Node::Paragraph(_)) {
            return false;
        }

        node.parent()
            .map(|parent| parent.node_id() == NodeId::ROOT)
            .unwrap_or(false)
    }

    pub fn relocate_selection(&mut self, source: Selection, target: Position) -> Result<()> {
        let is_block_drop = is_block_position(self.doc(), target);

        let fragment = Fragment::new_from_selection(self.doc(), &source)?;
        if fragment.is_empty() {
            return Ok(());
        }

        let selection_kind = source.classify(self.doc())?;
        let anchor_before_head = source.anchor_before_head(self.doc());
        let children_before = collect_children(self.doc(), target.node_id);
        let fragment = prepare_fragment(fragment, self.doc().schema(), is_block_drop);

        let cell_selection =
            crate::state::selection_helpers::compute_cell_selection(self.doc(), &source);

        let delete_result = match cell_selection {
            crate::state::selection_helpers::CellSelectionInfo::Rectangular { .. } => {
                self.delete_cell_selection(&cell_selection)?;
                DeleteResult::None
            }
            crate::state::selection_helpers::CellSelectionInfo::FullTables(ref table_ids) => {
                let mut expanded_source = source;
                if let Ok((mut from, mut to)) = source.as_sorted(self.doc()) {
                    for &table_id in table_ids {
                        if let Some(table) = self.node(table_id) {
                            if let Some(parent) = table.parent() {
                                let index = table.index().unwrap_or(0);
                                let start_pos =
                                    Position::new(parent.node_id(), index, Affinity::Downstream);
                                let end_pos = Position::new(
                                    parent.node_id(),
                                    index + 1,
                                    Affinity::Downstream,
                                );

                                if crate::state::position_helpers::compare_positions(
                                    self.doc(),
                                    start_pos,
                                    from,
                                )
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .is_lt()
                                {
                                    from = start_pos;
                                }
                                if crate::state::position_helpers::compare_positions(
                                    self.doc(),
                                    end_pos,
                                    to,
                                )
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .is_gt()
                                {
                                    to = end_pos;
                                }
                            }
                        }
                    }
                    expanded_source = Selection::new(from, to);
                }

                self.set_selection(expanded_source);
                self.delete_selection_with_merge()?
            }
            _ => {
                self.set_selection(source);
                self.delete_selection_with_merge()?
            }
        };

        let insert_pos = compute_insert_position(
            self.doc(),
            target,
            is_block_drop,
            &children_before,
            &delete_result,
        );
        let fragment = fragment.flatten_for_merge_at(self.doc(), insert_pos);

        self.set_selection(Selection::collapsed(insert_pos));
        let result = self.insert_fragment(insert_pos, fragment)?;

        let selection = match selection_kind {
            SelectionKind::InlineRange => result.as_inline_range_selection(self.doc()),
            _ => result.as_range_selection(),
        };

        if let Some(selection) = selection {
            let selection = if anchor_before_head {
                selection
            } else {
                Selection::new(selection.head, selection.anchor)
            };
            self.set_selection(selection);
        }

        Ok(())
    }

    pub fn drop_external(&mut self, target: Position, fragment: Fragment) -> Result<bool> {
        if fragment.is_empty() {
            return Ok(false);
        }

        let is_block_drop = is_block_position(self.doc(), target);
        let fragment = prepare_fragment(fragment, self.doc().schema(), is_block_drop);

        self.set_selection(Selection::collapsed(target));
        let result = self.insert_fragment(target, fragment)?;

        if let Some(selection) = result.as_inline_range_selection(self.doc()) {
            self.set_selection(selection);
        }

        Ok(result.inserted())
    }
}

fn prepare_fragment(
    fragment: Fragment,
    schema: &crate::schema::Schema,
    is_block_drop: bool,
) -> Fragment {
    if is_block_drop {
        fragment.into_blocks(schema).closed().with_fresh_ids()
    } else {
        let has_page_break = fragment
            .iter()
            .any(|(_, n)| matches!(n.data(), Node::PageBreak(_)));
        if has_page_break {
            fragment.split_at_page_breaks(schema).with_fresh_ids()
        } else {
            fragment.with_fresh_ids()
        }
    }
}

fn compute_insert_position(
    doc: &Doc,
    target: Position,
    is_block_drop: bool,
    children_before: &[NodeId],
    delete_result: &DeleteResult,
) -> Position {
    if !is_block_drop {
        let mut pos = delete_result.remap_position(target);
        pos.affinity = Affinity::Downstream;
        return pos;
    }

    let index = children_before.iter().skip(target.offset).find_map(|id| {
        let node = doc.node(*id)?;
        if node.parent().map(|p| p.node_id()) == Some(target.node_id) {
            node.index()
        } else {
            None
        }
    });

    if let Some(index) = index {
        return Position::new(target.node_id, index, Affinity::Downstream);
    }

    let len = doc
        .node(target.node_id)
        .map(|n| n.children().count())
        .unwrap_or(0);
    Position::new(target.node_id, len, Affinity::Downstream)
}

fn collect_children(doc: &Doc, node_id: NodeId) -> Vec<NodeId> {
    doc.node(node_id)
        .map(|n| n.children().map(|c| c.node_id()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::state::Position;
    use crate::types::Affinity;

    #[test]
    fn test_drag_and_drop_rectangular_table_selection() {
        let mut t = id!();
        let mut p_out = id!();
        let mut cell_a = id!();
        let mut cell_b = id!();
        let mut cell_c = id!();
        let mut cell_d = id!();

        let mut para_a = id!();
        let mut para_b = id!();
        let mut para_c = id!();
        let mut para_d = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        @cell_a table_cell { @para_a paragraph { text { "A" } } }
                        @cell_b table_cell { @para_b paragraph { text { "B" } } }
                    }
                    table_row {
                        @cell_c table_cell { @para_c paragraph { text { "C" } } }
                        @cell_d table_cell { @para_d paragraph { text { "D" } } }
                    }
                }
                @p_out paragraph { text { "Target" } }
            }
            selection { (para_a, 0) -> (para_c, 1) }
        };

        let actual = transact!(initial, |tr| {
            tr.drag_and_drop(Position::new(p_out, 0, Affinity::Downstream))
                .unwrap();
        });

        let doc = actual.doc;

        let table = doc.node(t).unwrap();
        let row0 = table.first_child().unwrap();
        let row1 = table.last_child().unwrap();

        let cell00 = row0.first_child().unwrap();
        let cell01 = row0.last_child().unwrap();
        let cell10 = row1.first_child().unwrap();

        // Check 0,0 (A) - should be empty
        let p00 = cell00.first_child().unwrap();
        assert_eq!(p00.children().count(), 0, "Cell 0,0 should be empty");

        // Check 1,0 (C) - should be empty
        let p10 = cell10.first_child().unwrap();
        assert_eq!(p10.children().count(), 0, "Cell 1,0 should be empty");

        // Check 0,1 (B) - should RETAIN "B"
        let p01 = cell01.first_child().unwrap();
        let text_node = p01
            .first_child()
            .expect("Cell B should still have text node");
        if let crate::model::Node::Text(t) = text_node.node() {
            assert_eq!(
                t.text.to_string(),
                "B",
                "Cell B content should be preserved"
            );
        } else {
            panic!("Cell B should have text");
        }
    }
}

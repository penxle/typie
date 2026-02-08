mod blockquote;
mod callout;
mod clipboard;
mod document;
mod drop;
mod fold;
mod horizontal_rule;
mod list;
mod mark;
mod node;
mod paragraph;
mod preedit;
mod root;
mod selection;
mod table;
mod text;

#[allow(unused_imports)]
pub use document::InsertResult;
pub use text::DeleteResult;

use crate::model::{Doc, Node, NodeId, NodeRef, ParagraphNode};
use crate::runtime::{Effect, State};
use crate::schema::Schema;
use crate::state::position_helpers::find_child_at_offset;
use crate::state::{BlockTraverser, Position, Selection};
use crate::types::Affinity;
use anyhow::{Context, Result};

pub struct Transaction {
    initial: State,
    state: State,
    effects: Vec<Effect>,
}

impl Transaction {
    pub fn new(state: &State) -> Self {
        Self {
            initial: state.clone(),
            state: state.clone(),
            effects: Vec::new(),
        }
    }

    pub fn doc(&self) -> &Doc {
        &self.state.doc
    }

    pub fn selection(&self) -> &Selection {
        &self.state.selection
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.state.selection = selection;
    }

    pub fn set_preferred_x(&mut self, preferred_x: Option<f32>) {
        self.state.preferred_x = preferred_x;
    }

    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        self.doc().node(id)
    }

    pub fn node_mut(&self, id: NodeId) -> Option<NodeRef<'_>> {
        self.doc().node(id)
    }

    pub fn push_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    pub(crate) fn selection_codepoints(&self) -> Vec<u32> {
        self.selection()
            .to_plain_text(self.doc())
            .chars()
            .map(|c| c as u32)
            .collect()
    }

    pub(crate) fn current_font(&self) -> (String, u16) {
        use crate::model::{FontFamilyMark, FontWeightMark, Mark};

        let marks = self
            .state
            .pending_marks
            .clone()
            .unwrap_or_else(|| mark::get_marks_at_cursor(self, &self.selection().head));

        let mut family = FontFamilyMark::default().family;
        let mut weight = FontWeightMark::default().weight;

        for mark in &marks {
            match mark {
                Mark::FontFamily(f) => family = f.family.clone(),
                Mark::FontWeight(w) => weight = w.weight,
                _ => {}
            }
        }

        (family, weight)
    }

    pub fn commit(self) -> Result<(State, Vec<Effect>)> {
        self.commit_internal(true)
    }

    #[allow(unused)]
    pub fn commit_immediate(self) -> Result<(State, Vec<Effect>)> {
        self.commit_internal(false)
    }

    fn commit_internal(mut self, defer_loro_commit: bool) -> Result<(State, Vec<Effect>)> {
        self.normalize_to_schema()?;
        self.normalize_selection();
        self.validate()?;

        if self.state.doc.frontiers() != self.initial.frontiers {
            self.effects.push(Effect::DocChanged);
        }

        if self.state.selection != self.initial.selection {
            self.effects.push(Effect::SelectionChanged);
        }

        if self.state.preedit != self.initial.preedit {
            let node_id = self
                .state
                .preedit
                .as_ref()
                .or(self.initial.preedit.as_ref())
                .map(|preedit| preedit.node_id);
            self.effects.push(Effect::PreeditChanged { node_id });
        }

        if defer_loro_commit {
            self.state.pending_loro_commit = true;
        } else {
            self.state.doc.loro_doc().commit();
        }

        self.state.frontiers = self.state.doc.frontiers();

        Ok((self.state, self.effects))
    }

    fn normalize_selection(&mut self) {
        let selection = &self.state.selection;

        if !selection.is_collapsed() {
            return;
        }

        let pos = selection.head;

        let Some(node) = self.doc().node(pos.node_id) else {
            return;
        };

        if node.spec().is_textblock(self.doc().schema()) {
            return;
        }

        let Some((child_id, _)) = find_child_at_offset(&node, pos.offset) else {
            return;
        };

        let Some(child) = self.doc().node(child_id) else {
            return;
        };

        if child.spec().selectable {
            let anchor = Position::new(pos.node_id, pos.offset, Affinity::Downstream);
            let head = Position::new(pos.node_id, pos.offset + 1, Affinity::Downstream);
            self.state.selection = Selection::new(anchor, head);
            return;
        }

        let Ok(mut traverser) = BlockTraverser::new(self.doc(), child_id) else {
            return;
        };

        if child.spec().is_textblock(self.doc().schema()) {
            let new_pos = Position::new(child_id, 0, Affinity::Downstream);
            self.state.selection = Selection::collapsed(new_pos);
            return;
        }

        while let Some(block_id) = traverser.next() {
            let Some(block) = self.doc().node(block_id) else {
                continue;
            };

            if block.spec().is_textblock(self.doc().schema()) {
                let new_pos = Position::new(block_id, 0, Affinity::Downstream);
                self.state.selection = Selection::collapsed(new_pos);
                return;
            }
        }
    }

    pub fn rollback(&self) -> Result<()> {
        self.state.doc.revert_to(&self.initial.frontiers)?;
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        for effect in &self.effects {
            if let Effect::NodeChanged { node_id } = effect {
                if self.doc().node(*node_id).is_none() {
                    continue;
                }

                if let Err(e) = self.doc().validate_node(*node_id) {
                    return Err(e);
                }
            }
        }

        self.state
            .selection
            .validate(self.doc())
            .context("Selection validation failed")?;
        Ok(())
    }

    fn normalize_to_schema(&mut self) -> Result<()> {
        let structure_changed = self
            .effects
            .iter()
            .any(|e| matches!(e, Effect::StructureChanged));

        if !structure_changed {
            return Ok(());
        }

        let mut queue = vec![NodeId::ROOT];
        let mut modified = false;

        let schema = self.doc().schema().clone();

        while let Some(node_id) = queue.pop() {
            modified |= self.repair_node_children(node_id, &schema)?;

            let children = self.doc().get_children_ids(node_id);
            for child_id in children {
                if let Some(node_type) = self.doc().get_node_type(child_id) {
                    let spec = schema.node_spec(node_type);

                    if let Some(required_grandparent) = spec.grandparent_must_be {
                        let parent_id = self.doc().get_parent_id(child_id);
                        let grandparent_type = parent_id
                            .and_then(|pid| self.doc().get_parent_id(pid))
                            .and_then(|gpid| self.doc().get_node_type(gpid));

                        if grandparent_type != Some(required_grandparent) {
                            self.delete_node_recursive(child_id)?;
                            modified = true;
                            continue;
                        }
                    }

                    if !spec.content.is_leaf() {
                        queue.push(child_id);
                    }
                }
            }
        }

        self.ensure_paragraph_after_pagebreak()?;

        if modified {
            self.push_effect(Effect::StructureChanged);
        }

        Ok(())
    }

    fn repair_node_children(&mut self, node_id: NodeId, schema: &Schema) -> Result<bool> {
        use crate::schema::RepairAction;

        let node_type = self
            .doc()
            .get_node_type(node_id)
            .context("Node type not found")?;
        let spec = schema.node_spec(node_type);

        let child_ids = self.doc().get_children_ids(node_id);
        let child_types: Vec<_> = child_ids
            .iter()
            .filter_map(|id| self.doc().get_node_type(*id))
            .collect();

        let actions = spec.content.repair(&child_types);

        if actions.is_empty() {
            return Ok(false);
        }

        for action in actions.iter().rev() {
            if let RepairAction::Remove { index } = action {
                if *index < child_ids.len() {
                    self.delete_node_recursive(child_ids[*index])?;
                }
            }
        }

        for action in &actions {
            if let RepairAction::Insert { index, node_type } = action {
                let default_node = self.create_default_node(*node_type)?;
                let current_children = self.doc().get_children_ids(node_id).len();
                let insert_idx = (*index).min(current_children);
                let parent = self.node_mut(node_id).context("Parent not found")?;
                parent.as_mut().insert_child(insert_idx, default_node)?;
            }
        }

        Ok(true)
    }

    fn create_default_node(&self, node_type: crate::model::NodeType) -> Result<Node> {
        use crate::model::*;

        Ok(match node_type {
            NodeType::Paragraph => Node::Paragraph(ParagraphNode::default()),
            NodeType::Text => Node::Text(TextNode::default()),
            NodeType::Image => Node::Image(ImageNode::default()),
            NodeType::File => Node::File(FileNode::default()),
            NodeType::Embed => Node::Embed(EmbedNode::default()),
            NodeType::HardBreak => Node::HardBreak(HardBreakNode::default()),
            NodeType::PageBreak => Node::PageBreak(PageBreakNode::default()),
            NodeType::HorizontalRule => Node::HorizontalRule(HorizontalRuleNode::default()),
            NodeType::Blockquote => Node::Blockquote(BlockquoteNode::default()),
            NodeType::BulletList => Node::BulletList(BulletListNode::default()),
            NodeType::OrderedList => Node::OrderedList(OrderedListNode::default()),
            NodeType::ListItem => Node::ListItem(ListItemNode::default()),
            NodeType::Fold => Node::Fold(FoldNode::default()),
            NodeType::FoldTitle => Node::FoldTitle(FoldTitleNode::default()),
            NodeType::FoldContent => Node::FoldContent(FoldContentNode::default()),
            NodeType::Callout => Node::Callout(CalloutNode::default()),
            NodeType::Table => Node::Table(TableNode::default()),
            NodeType::TableRow => Node::TableRow(TableRowNode::default()),
            NodeType::TableCell => Node::TableCell(TableCellNode::default()),
            NodeType::Root => anyhow::bail!("Cannot create Root node"),
        })
    }

    fn ensure_paragraph_after_pagebreak(&mut self) -> Result<()> {
        let root = self.node(NodeId::ROOT).context("Root node not found")?;
        let last_child = root.last_child();

        let needs_paragraph = last_child
            .as_ref()
            .filter(|c| matches!(c.node(), Node::Paragraph(_)))
            .map(|c| {
                c.children()
                    .any(|gc| matches!(gc.node(), Node::PageBreak(_)))
            })
            .unwrap_or(false);

        if needs_paragraph {
            let children_count = root.children().count();
            let root = self.node_mut(NodeId::ROOT).context("Root node not found")?;
            root.as_mut()
                .insert_child(children_count, Node::Paragraph(ParagraphNode::default()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::model::NodeId;
    use crate::runtime::Effect;

    #[test]
    fn ensure_no_grandchild_page_break_removes_nested_page_breaks() {
        let mut p = id!();
        let mut bq = id!();

        let initial = state! {
            doc {
                @bq blockquote {
                    @p paragraph {
                        text { "nested" }
                        page_break {}
                    }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                @bq blockquote {
                    @p paragraph {
                        text { "nested" }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn ensure_no_grandchild_page_break_preserves_root_child_page_breaks() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "root level" }
                    page_break {}
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "root level" }
                    page_break {}
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_adds_trailing_paragraph_when_missing() {
        let initial = state! {
            doc {
                image()
            }
            selection { (NodeId::ROOT, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                image()
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_adds_trailing_paragraph_when_last_is_blockquote() {
        let initial = state! {
            doc {
                paragraph { text { "a" } }
                blockquote {
                    paragraph { text { "b" } }
                }
            }
            selection { (NodeId::ROOT, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                paragraph { text { "a" } }
                blockquote {
                    paragraph { text { "b" } }
                }
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_does_not_add_paragraph_when_already_trailing() {
        let mut p = id!();

        let initial = state! {
            doc {
                image()
                @p paragraph { text { "end" } }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                image()
                @p paragraph { text { "end" } }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_repairs_fold_missing_content() {
        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                }
            }
            selection { (NodeId::ROOT, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_selection_converts_block_position_to_inline() {
        let mut p = id!();

        let initial = state! {
            doc {
                paragraph { text { "first" } }
                @p paragraph { text { "second" } }
            }
            selection { (NodeId::ROOT, 1) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                paragraph { text { "first" } }
                @p paragraph { text { "second" } }
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn normalize_selection_skips_non_textblock_nodes() {
        let initial = state! {
            doc {
                image()
                paragraph { text { "hello" } }
            }
            selection { (NodeId::ROOT, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        let expected = state! {
            doc {
                image()
                paragraph { text { "hello" } }
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        assert_state_eq!(actual, expected);
    }
}

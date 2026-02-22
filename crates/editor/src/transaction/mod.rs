mod annotation;
mod blockquote;
mod callout;
mod clipboard;
mod document;
mod drop;
mod fold;
mod horizontal_rule;
mod list;
mod node;
mod paragraph;
mod preedit;
mod remark;
mod root;
mod selection;
mod style;
mod table;
mod text;
mod text_replacement;

pub use selection::{paragraph_range_at, sentence_range_at, word_range_at};
pub(crate) use style::{compute_styles_at_char_position, compute_styles_at_cursor};
pub use text::DeleteResult;

use crate::model::*;
use crate::runtime::{Effect, State};
use crate::schema::{RepairAction, Schema};
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
        self.recompute_pending_styles();
    }

    fn cursor_has_text_segment(&self, block_id: NodeId, offset: usize) -> bool {
        let Some(node) = self.node(block_id) else {
            return false;
        };
        let Some((child_id, _)) = find_child_at_offset(&node, offset) else {
            return false;
        };
        let Some(child) = self.node(child_id) else {
            return false;
        };
        matches!(child.node(), Node::Text(t) if !t.text.get_segments().is_empty())
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

    pub(crate) fn resolve_attr_cascade(&self, node_id: NodeId) -> Vec<Attr> {
        let mut result = Vec::new();
        let Some(node) = self.node(node_id) else {
            return result;
        };
        for ancestor in node.ancestors() {
            if let Some(attrs) = ancestor.cascade_attrs() {
                for attr in attrs {
                    if !result.iter().any(|a: &Attr| a.key() == attr.key()) {
                        result.push(attr);
                    }
                }
            }
        }
        result
    }

    pub(crate) fn resolve_style_cascade(&self, node_id: NodeId) -> Vec<Style> {
        Attr::extract_styles(&self.resolve_attr_cascade(node_id))
    }

    pub(crate) fn set_cascade_attrs(&self, node_id: NodeId, attrs: &[Attr]) -> Result<()> {
        let node = self.node_mut(node_id).context("Node not found")?;
        node.as_mut().set_cascade_attrs(attrs)?;
        Ok(())
    }

    pub(crate) fn selection_codepoints(&self) -> Vec<u32> {
        self.selection()
            .to_plain_text(self.doc())
            .chars()
            .map(|c| c as u32)
            .collect()
    }

    pub fn commit(self) -> Result<(State, Vec<Effect>)> {
        self.commit_internal(true)
    }

    pub fn commit_immediate(self) -> Result<(State, Vec<Effect>)> {
        self.commit_internal(false)
    }

    pub fn normalize(&mut self) -> Result<()> {
        self.normalize_to_schema()?;
        self.normalize_styles();
        self.normalize_selection();
        Ok(())
    }

    fn commit_internal(mut self, defer_loro_commit: bool) -> Result<(State, Vec<Effect>)> {
        let pre_normalize_selection = self.state.selection;
        self.normalize()?;
        if self.state.selection != pre_normalize_selection {
            self.recompute_pending_styles();
        }
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

        if !(selection.is_collapsed() || selection.is_collapsed_block_selection(self.doc())) {
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

    fn normalize_styles(&mut self) {
        let node_ids: Vec<NodeId> = self
            .effects
            .iter()
            .filter_map(|e| match e {
                Effect::NodeChanged { node_id } => Some(*node_id),
                _ => None,
            })
            .collect();

        for node_id in node_ids {
            let Some(node_ref) = self.doc().node(node_id) else {
                continue;
            };

            let Node::Text(text_node) = node_ref.node() else {
                continue;
            };

            let allowed = self.doc().allowed_styles_for(node_id);
            let segments = text_node.text.get_segments();
            let len = text_node.text.char_len();

            if len == 0 {
                continue;
            }

            let disallowed: Vec<StyleType> = segments
                .iter()
                .flat_map(|seg| seg.styles.iter().map(|s| s.as_type()))
                .filter(|st| !allowed.contains(st))
                .collect();

            for style_type in disallowed {
                let _ = text_node.text.remove_style(0..len, style_type);
            }
        }
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

        let mut queue: Vec<(NodeId, Vec<NodeType>)> = vec![(NodeId::ROOT, Vec::new())];
        let mut modified = false;

        let schema = self.doc().schema().clone();

        while let Some((node_id, inherited_forbidden)) = queue.pop() {
            modified |= self.repair_node_children(node_id, &schema)?;

            let node_type = self.doc().get_node_type(node_id);
            let own_forbidden: Vec<NodeType> = node_type
                .and_then(|nt| schema.node_spec(nt).forbidden_descendants)
                .map(|f| f.to_vec())
                .unwrap_or_default();

            let mut child_forbidden = inherited_forbidden.clone();
            for ft in &own_forbidden {
                if !child_forbidden.contains(ft) {
                    child_forbidden.push(*ft);
                }
            }

            let children = self.doc().get_children_ids(node_id);
            for &child_id in children.iter() {
                if let Some(child_type) = self.doc().get_node_type(child_id) {
                    if child_forbidden.contains(&child_type) {
                        self.delete_node_recursive(child_id)?;
                        modified = true;
                        continue;
                    }

                    let spec = schema.node_spec(child_type);

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
                        queue.push((child_id, child_forbidden.clone()));
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

    fn create_default_node(&self, node_type: NodeType) -> Result<Node> {
        Ok(match node_type {
            NodeType::Paragraph => Node::Paragraph(ParagraphNode::default()),
            NodeType::Text => Node::Text(TextNode::default()),
            NodeType::Image => Node::Image(ImageNode::default()),
            NodeType::File => Node::File(FileNode::default()),
            NodeType::Embed => Node::Embed(EmbedNode::default()),
            NodeType::Archived => Node::Archived(ArchivedNode::default()),
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
    use super::*;

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

    #[test]
    fn normalize_removes_forbidden_descendant_table_nested_in_table() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            fold {
                                fold_title { text { "title" } }
                                fold_content {
                                    @p1 paragraph { text { "before" } }
                                    table {
                                        table_row {
                                            table_cell {
                                                paragraph { text { "nested" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        // 내부 Table이 삭제되고, FoldContent의 content가 repair됨
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let table = root.first_child().unwrap();
        assert_eq!(table.node_type(), NodeType::Table);
        let row = table.first_child().unwrap();
        let cell = row.first_child().unwrap();
        let fold = cell.first_child().unwrap();
        assert_eq!(fold.node_type(), NodeType::Fold);
        let fold_content = fold.children().nth(1).unwrap();
        assert_eq!(fold_content.node_type(), NodeType::FoldContent);

        // FoldContent 안에 Table이 없어야 함
        for child in fold_content.children() {
            assert_ne!(
                child.node_type(),
                NodeType::Table,
                "Nested table should have been removed by normalization"
            );
        }
    }

    #[test]
    fn normalize_preserves_non_forbidden_table() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title { text { "title" } }
                    fold_content {
                        @p1 paragraph { text { "before" } }
                        table {
                            table_row {
                                table_cell {
                                    paragraph { text { "in table" } }
                                }
                            }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        // Table은 Table 안에 있는 게 아니므로 보존되어야 함
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let fold = root.first_child().unwrap();
        let fold_content = fold.children().nth(1).unwrap();

        let has_table = fold_content
            .children()
            .any(|c| c.node_type() == NodeType::Table);
        assert!(has_table, "Non-nested table should be preserved");
    }

    #[test]
    fn normalize_removes_deeply_nested_forbidden_table() {
        let mut p1 = id!();

        // Table > TableRow > TableCell > Fold > FoldContent > Fold > FoldContent > Table
        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            fold {
                                fold_title { text { "outer" } }
                                fold_content {
                                    fold {
                                        fold_title { text { "inner" } }
                                        fold_content {
                                            @p1 paragraph { text { "deep" } }
                                            table {
                                                table_row {
                                                    table_cell {
                                                        paragraph { text { "nested" } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.push_effect(Effect::StructureChanged);
        });

        // 깊이 중첩된 Table도 삭제되어야 함
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let table = root.first_child().unwrap();
        let row = table.first_child().unwrap();
        let cell = row.first_child().unwrap();
        let outer_fold = cell.first_child().unwrap();
        let outer_fc = outer_fold.children().nth(1).unwrap();
        let inner_fold = outer_fc.first_child().unwrap();
        let inner_fc = inner_fold.children().nth(1).unwrap();

        for child in inner_fc.children() {
            assert_ne!(
                child.node_type(),
                NodeType::Table,
                "Deeply nested table should have been removed"
            );
        }
    }

    #[test]
    fn insert_table_inside_table_is_rejected() {
        let mut p1 = id!();

        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            @p1 paragraph { text { "cell" } }
                        }
                    }
                }
            }
            selection { (p1, 0) }
        };

        let actual = transact!(initial, |tr| {
            let result = tr.insert_node(Node::Table(TableNode::default())).unwrap();
            assert!(!result, "Inserting table inside table should be rejected");
        });

        // Table이 하나만 있어야 함
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let table_count = root
            .children()
            .filter(|c| c.node_type() == NodeType::Table)
            .count();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn validate_exhaustive_rejects_forbidden_descendant() {
        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell {
                            fold {
                                fold_title { text { "title" } }
                                fold_content {
                                    table {
                                        table_row {
                                            table_cell {
                                                paragraph { text { "nested" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            selection { (NodeId::ROOT, 0) }
        };

        // state!로 만든 문서는 normalization 없이 직접 구성되므로
        // validate_exhaustive가 forbidden descendant를 잡아야 함
        let result = initial.doc.validate_exhaustive();
        assert!(
            result.is_err(),
            "validate_exhaustive should reject nested table"
        );
    }
}

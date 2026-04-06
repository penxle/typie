use editor_common::StrExt;
use editor_model::{Doc, DocumentAttrs, Modifier, Node, NodeId, Subtree};
use editor_state::{Composition, PendingModifiers, Selection, State};

use crate::{Effect, Step, StepError, TransactionMeta, Validation, validate};

struct Batch {
    validations: Vec<Validation>,
}

pub struct Transaction {
    state: State,
    steps: Vec<Step>,
    effects: Vec<Effect>,
    meta: TransactionMeta,
    batch: Option<Batch>,
}

#[derive(Clone)]
pub struct Savepoint {
    state: State,
    steps_len: usize,
    effects_len: usize,
    batch_validations_len: Option<usize>,
}

impl Transaction {
    pub fn new(state: &State) -> Self {
        Self {
            state: state.clone(),
            steps: Vec::new(),
            effects: Vec::new(),
            meta: TransactionMeta::default(),
            batch: None,
        }
    }

    pub fn doc(&self) -> Doc {
        self.state.doc.clone()
    }

    pub fn selection(&self) -> Selection {
        self.state.selection
    }

    pub fn pending_modifiers(&self) -> &PendingModifiers {
        &self.state.pending_modifiers
    }

    pub fn composition(&self) -> Option<&Composition> {
        self.state.composition.as_ref()
    }

    pub fn doc_changed(&self) -> bool {
        self.steps.iter().any(|s| s.is_doc_step())
    }

    pub fn selection_changed(&self) -> bool {
        self.steps.iter().any(|s| s.is_selection_step())
    }

    pub fn push_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    fn apply_step(&mut self, step: Step) -> Result<(), StepError> {
        let output = step.apply(&self.state)?;
        self.state = output.state;
        self.steps.push(step);
        if let Some(batch) = &mut self.batch {
            batch.validations.extend(output.validations);
        } else {
            self.run_validations(&output.validations)?;
        }
        Ok(())
    }

    pub fn savepoint(&self) -> Savepoint {
        Savepoint {
            state: self.state.clone(),
            steps_len: self.steps.len(),
            effects_len: self.effects.len(),
            batch_validations_len: self.batch.as_ref().map(|b| b.validations.len()),
        }
    }

    pub fn rollback(&mut self, sp: Savepoint) {
        self.state = sp.state;
        self.steps.truncate(sp.steps_len);
        self.effects.truncate(sp.effects_len);
        if let (Some(batch), Some(len)) = (&mut self.batch, sp.batch_validations_len) {
            batch.validations.truncate(len);
        }
    }

    pub fn insert_text(
        &mut self,
        node_id: NodeId,
        offset: usize,
        text: &str,
    ) -> Result<(), StepError> {
        self.apply_step(Step::InsertText {
            node_id,
            offset,
            text: text.to_string(),
        })
    }

    pub fn remove_text(
        &mut self,
        node_id: NodeId,
        offset: usize,
        len: usize,
    ) -> Result<(), StepError> {
        let entry = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;

        let text_node = match &entry.node {
            Node::Text(t) => t,
            _ => return Err(StepError::ExpectedTextNode(node_id)),
        };

        let byte_start = text_node.text.nth_char_byte_offset(offset);
        let byte_end = text_node.text.nth_char_byte_offset(offset + len);
        let text = text_node.text[byte_start..byte_end].to_string();

        self.apply_step(Step::RemoveText {
            node_id,
            offset,
            text,
        })
    }

    pub fn insert_subtree(
        &mut self,
        parent_id: NodeId,
        index: usize,
        subtree: Subtree,
    ) -> Result<(), StepError> {
        self.apply_step(Step::InsertSubtree {
            parent_id,
            index,
            subtree,
        })
    }

    pub fn remove_subtree(&mut self, node_id: NodeId) -> Result<(), StepError> {
        let entry = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let parent_id = entry.parent.ok_or(StepError::NodeNotFound(node_id))?;
        let parent_entry = self
            .state
            .doc
            .get_entry(parent_id)
            .ok_or(StepError::NodeNotFound(parent_id))?;
        let index = parent_entry
            .children
            .iter()
            .position(|&id| id == node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let subtree =
            Subtree::capture(&self.state.doc, node_id).ok_or(StepError::NodeNotFound(node_id))?;

        self.apply_step(Step::RemoveSubtree {
            parent_id,
            index,
            subtree,
        })
    }

    pub fn move_node(
        &mut self,
        node_id: NodeId,
        new_parent: NodeId,
        new_index: usize,
    ) -> Result<(), StepError> {
        let entry = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let old_parent = entry.parent.ok_or(StepError::NodeNotFound(node_id))?;
        let parent_entry = self
            .state
            .doc
            .get_entry(old_parent)
            .ok_or(StepError::NodeNotFound(old_parent))?;
        let old_index = parent_entry
            .children
            .iter()
            .position(|&id| id == node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;

        self.apply_step(Step::MoveNode {
            node_id,
            old_parent,
            old_index,
            new_parent,
            new_index,
        })
    }

    pub fn set_node(&mut self, node_id: NodeId, new_node: Node) -> Result<(), StepError> {
        let entry = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let old_node = entry.node.clone();

        self.apply_step(Step::SetNode {
            node_id,
            old_node,
            new_node,
        })
    }

    pub fn split_node(
        &mut self,
        node_id: NodeId,
        offset: usize,
        new_node_id: NodeId,
    ) -> Result<(), StepError> {
        self.apply_step(Step::SplitNode {
            node_id,
            offset,
            new_node_id,
        })
    }

    pub fn merge_node(&mut self, node_id: NodeId, target_id: NodeId) -> Result<(), StepError> {
        let target_entry = self
            .state
            .doc
            .get_entry(target_id)
            .ok_or(StepError::NodeNotFound(target_id))?;

        let offset = match &target_entry.node {
            Node::Text(t) => t.text.char_count(),
            _ => target_entry.children.len(),
        };

        self.apply_step(Step::MergeNode {
            node_id,
            target_id,
            offset,
        })
    }

    pub fn add_modifier(&mut self, node_id: NodeId, modifier: Modifier) -> Result<(), StepError> {
        self.apply_step(Step::AddModifier { node_id, modifier })
    }

    pub fn remove_modifier(
        &mut self,
        node_id: NodeId,
        modifier: Modifier,
    ) -> Result<(), StepError> {
        self.apply_step(Step::RemoveModifier { node_id, modifier })
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), StepError> {
        self.apply_step(Step::SetSelection {
            old: self.state.selection,
            new: selection,
        })
    }

    pub fn set_pending_modifiers(&mut self, modifiers: PendingModifiers) -> Result<(), StepError> {
        let old = self.state.pending_modifiers.clone();
        self.apply_step(Step::SetPendingModifiers {
            old,
            new: modifiers,
        })
    }

    pub fn set_modifiers(
        &mut self,
        node_id: NodeId,
        modifiers: Vec<Modifier>,
    ) -> Result<(), StepError> {
        let old_modifiers = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?
            .modifiers
            .clone();

        self.apply_step(Step::SetModifiers {
            node_id,
            old_modifiers,
            new_modifiers: modifiers,
        })
    }

    pub fn set_composition(&mut self, composition: Option<Composition>) -> Result<(), StepError> {
        let old = self.state.composition;
        self.apply_step(Step::SetComposition {
            old,
            new: composition,
        })
    }

    pub fn set_document_attrs(&mut self, attrs: DocumentAttrs) -> Result<(), StepError> {
        let old = self.state.doc.attrs().clone();
        self.apply_step(Step::SetDocumentAttrs { old, new: attrs })
    }

    pub fn batch<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(&mut Transaction) -> Result<(), E>,
        E: From<StepError>,
    {
        let sp = self.savepoint();
        self.batch = Some(Batch {
            validations: Vec::new(),
        });
        let result = f(self);
        let batch = self.batch.take().unwrap();

        match result {
            Ok(()) => {
                if let Err(e) = self.run_validations(&batch.validations) {
                    self.rollback(sp);
                    return Err(E::from(e));
                }
                Ok(())
            }
            Err(e) => {
                self.rollback(sp);
                Err(e)
            }
        }
    }

    pub fn apply_steps(&mut self, steps: Vec<Step>) -> Result<(), StepError> {
        for step in steps {
            self.apply_step(step)?;
        }
        Ok(())
    }

    fn run_validations(&self, validations: &[Validation]) -> Result<(), StepError> {
        for v in validations {
            match v {
                Validation::Node(node_id) => {
                    if self.state.doc.get_entry(*node_id).is_some() {
                        validate::validate_content(&self.state.doc, *node_id)?;
                    }
                }
                Validation::Subtree(node_id) => {
                    if self.state.doc.get_entry(*node_id).is_some() {
                        validate::validate_content(&self.state.doc, *node_id)?;
                        validate::validate_context_deep(&self.state.doc, *node_id)?;
                        if let Some(node_ref) = self.state.doc.node(*node_id) {
                            for desc in node_ref.descendants() {
                                validate::validate_content(&self.state.doc, desc.id())?;
                            }
                        }
                    }
                }
                Validation::Modifier(node_id, modifier_type) => {
                    if self.state.doc.get_entry(*node_id).is_some() {
                        validate::validate_modifier_context_by_type(
                            &self.state.doc,
                            *node_id,
                            *modifier_type,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn meta(&self) -> &TransactionMeta {
        &self.meta
    }

    pub fn update_meta(&mut self, f: impl FnOnce(&mut TransactionMeta)) {
        f(&mut self.meta);
    }

    pub fn commit(self) -> (State, Vec<Step>, Vec<Effect>, TransactionMeta) {
        (self.state, self.steps, self.effects, self.meta)
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;
    use editor_state::*;

    use super::*;
    use crate::HistoryMeta;
    use crate::test_utils::DocTestExt;

    #[test]
    fn new_transaction_reads_state() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let tr = Transaction::new(&state);

        assert_eq!(tr.selection(), Selection::collapsed(Position::new(t1, 0)));
        assert!(tr.doc().get_entry(t1).is_some());
    }

    #[test]
    fn finish_empty_transaction() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let tr = Transaction::new(&state);
        let (_, steps, effects, _) = tr.commit();

        assert!(steps.is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn insert_text_records_step() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.insert_text(t1, 5, " Beautiful").unwrap();

        assert_eq!(tr.doc().text(t1).text, "Hello Beautiful World");

        let (_, steps, _, _) = tr.commit();

        assert_eq!(steps.len(), 1);
        assert!(matches!(&steps[0], Step::InsertText { .. }));
    }

    #[test]
    fn remove_text_derives_content_from_state() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.remove_text(t1, 5, 6).unwrap();

        assert_eq!(tr.doc().text(t1).text, "Hello");

        let (_, steps, _, _) = tr.commit();

        assert_eq!(steps.len(), 1);
        match &steps[0] {
            Step::RemoveText { text, .. } => assert_eq!(text, " World"),
            _ => panic!("expected RemoveText"),
        }
    }

    #[test]
    fn insert_text_error_on_missing_node() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.insert_text(NodeId::new(), 0, "X");

        assert!(result.is_err());
        assert!(tr.steps.is_empty());
    }

    #[test]
    fn insert_subtree_records_step() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));
        tr.insert_subtree(NodeId::ROOT, 1, subtree).unwrap();

        assert!(tr.doc().get_entry(new_id).is_some());
        let doc = tr.doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.children.len(), 2);

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn remove_subtree_derives_subtree_from_state() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("Hello World") } p2: paragraph {} } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.remove_subtree(p1).unwrap();

        assert!(tr.doc().get_entry(p1).is_none());
        let doc = tr.doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0], p2);

        let (_, steps, _, _) = tr.commit();
        match &steps[0] {
            Step::RemoveSubtree { subtree, .. } => {
                assert!(matches!(subtree.node, Node::Paragraph(_)));
            }
            _ => panic!("expected RemoveSubtree"),
        }
    }

    #[test]
    fn move_node_derives_old_position_from_state() {
        let (state, p1, t1, p2) = state! {
            doc { root { p1: paragraph { t1: text("Hello World") } p2: paragraph {} } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.move_node(t1, p2, 0).unwrap();

        assert!(
            tr.doc()
                .get_entry(p1)
                .unwrap()
                .children
                .iter()
                .all(|&id| id != t1)
        );
        assert_eq!(tr.doc().get_entry(p2).unwrap().children[0], t1);

        let (_, steps, _, _) = tr.commit();
        match &steps[0] {
            Step::MoveNode {
                old_parent,
                old_index,
                ..
            } => {
                assert_eq!(*old_parent, p1);
                assert_eq!(*old_index, 0);
            }
            _ => panic!("expected MoveNode"),
        }
    }

    #[test]
    fn split_node_text() {
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let t2 = NodeId::new();
        tr.split_node(t1, 5, t2).unwrap();

        assert_eq!(tr.doc().text(t1).text, "Hello");
        assert_eq!(tr.doc().text(t2).text, " World");
        assert_eq!(tr.doc().get_entry(p1).unwrap().children.len(), 2);
    }

    #[test]
    fn merge_node_derives_offset_from_state() {
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let t2 = NodeId::new();
        tr.split_node(t1, 5, t2).unwrap();
        tr.merge_node(t2, t1).unwrap();

        assert_eq!(tr.doc().text(t1).text, "Hello World");
        assert!(tr.doc().get_entry(t2).is_none());
        assert_eq!(tr.doc().get_entry(p1).unwrap().children.len(), 1);

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 2);
        match &steps[1] {
            Step::MergeNode { offset, .. } => assert_eq!(*offset, 5),
            _ => panic!("expected MergeNode"),
        }
    }

    #[test]
    fn add_modifier_records_step() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.add_modifier(t1, Modifier::Bold).unwrap();

        let doc = tr.doc();
        let entry = doc.get_entry(t1).unwrap();
        assert_eq!(entry.modifiers, vec![Modifier::Bold]);

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn remove_modifier_records_step() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.add_modifier(t1, Modifier::Bold).unwrap();
        tr.remove_modifier(t1, Modifier::Bold).unwrap();

        let doc = tr.doc();
        let entry = doc.get_entry(t1).unwrap();
        assert!(entry.modifiers.is_empty());

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn set_selection_records_step() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_sel = Selection::collapsed(Position::new(t1, 5));
        tr.set_selection(new_sel).unwrap();

        assert_eq!(tr.selection(), new_sel);

        let (_, steps, _, _) = tr.commit();
        match &steps[0] {
            Step::SetSelection { old, new } => {
                assert_eq!(*old, Selection::collapsed(Position::new(t1, 0)));
                assert_eq!(*new, new_sel);
            }
            _ => panic!("expected SetSelection"),
        }
    }

    #[test]
    fn set_node_derives_old_node_from_state() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_node = Node::Paragraph(ParagraphNode {
            align: TextAlign::Center,
        });
        tr.set_node(p1, new_node.clone()).unwrap();

        assert_eq!(tr.doc().get_entry(p1).unwrap().node, new_node);

        let (_, steps, _, _) = tr.commit();
        match &steps[0] {
            Step::SetNode { old_node, .. } => {
                assert_eq!(*old_node, Node::Paragraph(ParagraphNode::default()));
            }
            _ => panic!("expected SetNode"),
        }
    }

    #[test]
    fn paragraph_split_via_transaction() {
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);

        let t2 = NodeId::new();
        tr.split_node(t1, 5, t2).unwrap();

        let p2 = NodeId::new();
        tr.split_node(p1, 1, p2).unwrap();

        tr.set_selection(Selection::collapsed(Position::new(t2, 0)))
            .unwrap();

        assert_eq!(tr.doc().text(t1).text, "Hello");
        assert_eq!(tr.doc().text(t2).text, " World");
        assert_eq!(tr.doc().get_entry(p1).unwrap().children.len(), 1);
        assert_eq!(tr.doc().get_entry(p2).unwrap().children.len(), 1);
        assert_eq!(tr.doc().get_entry(p2).unwrap().children[0], t2);

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn savepoint_rollback_preserves_earlier_steps() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);

        tr.insert_text(t1, 11, "!").unwrap();
        let sp = tr.savepoint();

        tr.remove_text(t1, 0, 5).unwrap();
        assert_eq!(tr.steps.len(), 2);

        tr.rollback(sp);
        assert_eq!(tr.steps.len(), 1);

        assert_eq!(tr.doc().text(t1).text, "Hello World!");

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn set_document_attrs_records_step() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 800.0 },
        };
        tr.set_document_attrs(new_attrs).unwrap();

        assert_eq!(
            tr.doc().attrs().layout_mode,
            LayoutMode::Continuous { max_width: 800.0 }
        );

        let (_, steps, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
        assert!(matches!(&steps[0], Step::SetDocumentAttrs { .. }));
    }

    #[test]
    fn steps_produce_valid_inverses() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);

        tr.insert_text(t1, 5, " Beautiful").unwrap();
        tr.set_selection(Selection::collapsed(Position::new(t1, 15)))
            .unwrap();

        let (_, steps, _, _) = tr.commit();

        let mut current = steps
            .iter()
            .fold(state.clone(), |s, step| step.apply(&s).unwrap().state);
        for step in steps.iter().rev() {
            current = step.inverse().apply(&current).unwrap().state;
        }

        assert_eq!(current.text(t1).text, "Hello World");
        assert_eq!(current.selection, state.selection);
    }

    #[test]
    fn batch_defers_validation() {
        let (state, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                    }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };

        let mut tr = Transaction::new(&state);
        let doc = tr.doc();
        let bq_id = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .id();
        let para_id = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .nth(1)
            .unwrap()
            .id();

        let fix_id = NodeId::new();
        tr.batch::<_, StepError>(|tr| {
            let target_children = tr.doc().node(bq_id).unwrap().children().count();
            tr.move_node(para_id, bq_id, target_children)?;
            tr.insert_subtree(
                NodeId::ROOT,
                1,
                Subtree::leaf(fix_id, Node::Paragraph(ParagraphNode::default())),
            )?;
            Ok(())
        })
        .unwrap();

        let doc = tr.doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.children.len(), 2);
        assert!(doc.get_entry(fix_id).is_some());
    }

    #[test]
    fn batch_rolls_back_on_invalid_final_state() {
        let (state, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                    }
                    paragraph { t2: text("B") }
                }
            }
            selection: (t2, 0)
        };

        let mut tr = Transaction::new(&state);
        let doc = tr.doc();
        let bq_id = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .id();
        let para_id = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .nth(1)
            .unwrap()
            .id();

        let result = tr.batch::<_, StepError>(|tr| {
            let target_children = tr.doc().node(bq_id).unwrap().children().count();
            tr.move_node(para_id, bq_id, target_children)?;
            Ok(())
        });

        assert!(result.is_err());
        let doc = tr.doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn apply_steps_executes_sequentially() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let p_id = NodeId::new();
        let steps = vec![Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree: Subtree::leaf(p_id, Node::Paragraph(ParagraphNode::default())),
        }];
        tr.apply_steps(steps).unwrap();

        assert!(tr.doc().get_entry(p_id).is_some());
    }

    #[test]
    fn new_transaction_has_default_meta() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let tr = Transaction::new(&state);
        assert!(matches!(tr.meta().history, HistoryMeta::Record));
    }

    #[test]
    fn update_meta_modifies_history() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        assert!(matches!(tr.meta().history, HistoryMeta::Skip));
    }

    #[test]
    fn commit_returns_meta() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        let (_, _, _, meta) = tr.commit();
        assert!(matches!(meta.history, HistoryMeta::Skip));
    }
}

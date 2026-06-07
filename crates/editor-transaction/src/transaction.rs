use editor_crdt::Op;
use editor_model::{
    Doc, DocOp, Marker, Modifier, ModifierType, Node, NodeId, PlainNode, PlainStyleEntry, Subtree,
};
use editor_state::{Composition, PendingModifiers, Selection, StableSelection, State};

use crate::{Effect, Step, StepError, TransactionMeta, Validation, validate};

struct Batch {
    validations: Vec<Validation>,
}

pub struct Transaction {
    state: State,
    // Stable form of the last selection set on this transaction. Tracked
    // separately from `state.selection` because `SetSelection.old` must
    // freeze against the doc where that selection was canonical, not against
    // the post-edit doc where its node may already be dead.
    selection_stable: Option<StableSelection>,
    steps: Vec<Step>,
    ops: Vec<Op<DocOp>>,
    effects: Vec<Effect>,
    meta: TransactionMeta,
    batch: Option<Batch>,
}

#[derive(Clone)]
pub struct Savepoint {
    state: State,
    selection_stable: Option<StableSelection>,
    steps_len: usize,
    ops_len: usize,
    effects_len: usize,
    batch_validations_len: Option<usize>,
}

impl Transaction {
    pub fn new(state: &State) -> Self {
        let selection_stable = state
            .selection
            .as_ref()
            .map(|s| StableSelection::freeze(s, &state.doc));
        Self {
            state: state.clone(),
            selection_stable,
            steps: Vec::new(),
            ops: Vec::new(),
            effects: Vec::new(),
            meta: TransactionMeta::default(),
            batch: None,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn doc(&self) -> Doc {
        self.state.doc.clone()
    }

    pub fn selection(&self) -> Option<Selection> {
        self.state.selection
    }

    pub fn pending_modifiers(&self) -> &PendingModifiers {
        &self.state.pending_modifiers
    }

    pub fn pending_style(&self) -> &Option<editor_state::PendingStyle> {
        &self.state.pending_style
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
        let mut validations: Vec<Validation> = Vec::new();
        let ops = self.state.batch_with_ops_mut::<_, StepError>(|batched| {
            step.apply_to(batched, &mut validations)
        })?;
        self.ops.extend(ops);
        if let Step::SetSelection { new, .. } = &step {
            self.selection_stable = new.clone();
        }
        self.steps.push(step);
        if validations.is_empty() {
            // No-op fast path. Lets non-doc steps (SetSelection, etc.) skip
            // the dedupe + match + per-validation cost entirely.
        } else if let Some(batch) = &mut self.batch {
            batch.validations.extend(validations);
        } else {
            self.run_validations(&validations)?;
        }
        Ok(())
    }

    pub fn savepoint(&self) -> Savepoint {
        Savepoint {
            state: self.state.clone(),
            selection_stable: self.selection_stable.clone(),
            steps_len: self.steps.len(),
            ops_len: self.ops.len(),
            effects_len: self.effects.len(),
            batch_validations_len: self.batch.as_ref().map(|b| b.validations.len()),
        }
    }

    pub fn rollback(&mut self, sp: Savepoint) {
        self.state = sp.state;
        self.selection_stable = sp.selection_stable;
        self.steps.truncate(sp.steps_len);
        self.ops.truncate(sp.ops_len);
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

        let text: String = text_node
            .text
            .to_string()
            .chars()
            .skip(offset)
            .take(len)
            .collect();

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
        let parent_id = (*entry.parent.get()).ok_or(StepError::NodeNotFound(node_id))?;
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
        let old_parent = (*entry.parent.get()).ok_or(StepError::NodeNotFound(node_id))?;
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

    pub fn set_node(&mut self, node_id: NodeId, new_node: PlainNode) -> Result<(), StepError> {
        let entry = self
            .state
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let old_node = entry.node.to_plain();

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
            Node::Text(t) => t.text.len(),
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

    pub fn set_node_style(
        &mut self,
        node_id: NodeId,
        style: Option<String>,
    ) -> Result<(), StepError> {
        let old = self
            .state
            .doc
            .get_entry(node_id)
            .map(|e| e.style.get().clone())
            .unwrap_or(None);
        if old == style {
            return Ok(());
        }
        self.apply_step(Step::SetNodeStyle {
            node_id,
            old,
            new: style,
        })
    }

    pub fn set_marker(&mut self, node_id: NodeId, marker: Option<Marker>) -> Result<(), StepError> {
        let old = self
            .state
            .doc
            .get_entry(node_id)
            .map(|e| e.marker.get().clone())
            .unwrap_or(None);
        if old == marker {
            return Ok(());
        }
        self.apply_step(Step::SetNodeMarker {
            node_id,
            old,
            new: marker,
        })
    }

    pub fn set_style(
        &mut self,
        style_id: String,
        entry: Option<PlainStyleEntry>,
    ) -> Result<(), StepError> {
        let old = capture_style_entry(&self.state.doc, &style_id);
        if old == entry {
            return Ok(());
        }
        self.apply_step(Step::SetStyle {
            style_id,
            old,
            new: entry,
        })
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) -> Result<(), StepError> {
        // Normalize through the current doc. If normalization fails (an endpoint
        // doesn't resolve against the live doc), drop the request rather than
        // panicking inside freeze. If the effective result matches the live
        // selection, the step is a noop.
        let new_effective = match selection.as_ref() {
            Some(s) => match s.normalize(&self.state.doc) {
                Some(n) => Some(n),
                None => return Ok(()),
            },
            None => None,
        };
        if new_effective == self.state.selection {
            return Ok(());
        }
        let old = self.selection_stable.clone();
        let new = new_effective
            .as_ref()
            .map(|s| StableSelection::freeze(s, &self.state.doc));
        self.apply_step(Step::SetSelection { old, new })
    }

    pub fn set_pending_modifiers(&mut self, modifiers: PendingModifiers) -> Result<(), StepError> {
        if self.state.pending_modifiers == modifiers {
            return Ok(());
        }
        let old = self.state.pending_modifiers.clone();
        self.apply_step(Step::SetPendingModifiers {
            old,
            new: modifiers,
        })
    }

    pub fn set_pending_style(
        &mut self,
        pending: Option<editor_state::PendingStyle>,
    ) -> Result<(), StepError> {
        if self.state.pending_style == pending {
            return Ok(());
        }
        let old = self.state.pending_style.clone();
        self.apply_step(Step::SetPendingStyle { old, new: pending })
    }

    pub fn set_composition(&mut self, composition: Option<Composition>) -> Result<(), StepError> {
        let old = self.state.composition;
        if old == composition {
            return Ok(());
        }
        self.apply_step(Step::SetComposition {
            old,
            new: composition,
        })
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
        // State-only steps (SetSelection/SetComposition/SetPendingModifiers)
        // write a single field nothing else reads during step replay, so only
        // the last of each kind affects the final state. Defer them until after
        // all doc steps have replayed: SetSelection.apply_to thaws against the
        // current doc, and during undo the relevant selection sits between doc
        // edits whose inverses are applied later in the same batch — running it
        // in place would resolve against a half-restored doc.
        let mut last_selection: Option<usize> = None;
        let mut last_composition: Option<usize> = None;
        let mut last_pending: Option<usize> = None;
        let mut last_pending_style: Option<usize> = None;
        for (i, step) in steps.iter().enumerate() {
            match step {
                Step::SetSelection { .. } => last_selection = Some(i),
                Step::SetComposition { .. } => last_composition = Some(i),
                Step::SetPendingModifiers { .. } => last_pending = Some(i),
                Step::SetPendingStyle { .. } => last_pending_style = Some(i),
                _ => {}
            }
        }

        let mut validations: Vec<Validation> = Vec::new();
        let ops = self.state.batch_with_ops_mut::<_, StepError>(|batched| {
            for step in steps.iter() {
                if !step.is_doc_step() {
                    continue;
                }
                step.apply_to(batched, &mut validations)?;
            }
            for i in [
                last_selection,
                last_composition,
                last_pending,
                last_pending_style,
            ]
            .into_iter()
            .flatten()
            {
                steps[i].apply_to(batched, &mut validations)?;
            }
            Ok(())
        })?;

        self.ops.extend(ops);

        for step in &steps {
            if let Step::SetSelection { new, .. } = step {
                self.selection_stable = new.clone();
            }
        }
        self.steps.extend(steps);

        if !validations.is_empty() {
            if let Some(batch) = &mut self.batch {
                batch.validations.extend(validations);
            } else {
                self.run_validations(&validations)?;
            }
        }

        Ok(())
    }

    fn run_validations(&self, validations: &[Validation]) -> Result<(), StepError> {
        let dedup = dedupe_validations(validations);
        for v in &dedup {
            match v {
                Validation::Node(node_id) => {
                    if self.state.doc.get_entry(*node_id).is_some() {
                        validate::validate_content(&self.state.doc, *node_id)?;
                        validate::validate_context(&self.state.doc, *node_id)?;
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

    pub fn commit(
        mut self,
    ) -> (
        State,
        Vec<Step>,
        Vec<Op<DocOp>>,
        Vec<Effect>,
        TransactionMeta,
    ) {
        self.state.graph.commit_mut();
        (self.state, self.steps, self.ops, self.effects, self.meta)
    }

    #[cfg(test)]
    pub(crate) fn ops_for_test(&self) -> &[Op<DocOp>] {
        &self.ops
    }
}

fn dedupe_validations(validations: &[Validation]) -> Vec<Validation> {
    use std::collections::HashSet;

    let mut subtree_ids: HashSet<NodeId> = HashSet::new();
    for v in validations {
        if let Validation::Subtree(id) = v {
            subtree_ids.insert(*id);
        }
    }

    let mut seen_node: HashSet<NodeId> = HashSet::new();
    let mut seen_subtree: HashSet<NodeId> = HashSet::new();
    let mut seen_modifier: HashSet<(NodeId, ModifierType)> = HashSet::new();
    let mut result: Vec<Validation> = Vec::new();

    for v in validations {
        match *v {
            Validation::Node(id) => {
                if subtree_ids.contains(&id) {
                    continue;
                }
                if seen_node.insert(id) {
                    result.push(*v);
                }
            }
            Validation::Subtree(id) => {
                if seen_subtree.insert(id) {
                    result.push(*v);
                }
            }
            Validation::Modifier(id, k) => {
                if seen_modifier.insert((id, k)) {
                    result.push(*v);
                }
            }
        }
    }
    result
}

fn capture_style_entry(doc: &Doc, style_id: &str) -> Option<PlainStyleEntry> {
    if !doc.style_present(style_id) {
        return None;
    }
    let entry = doc.style_entry(style_id)?;
    Some(PlainStyleEntry {
        name: entry.name.get().clone(),
        modifiers: entry.modifiers.iter().cloned().collect(),
    })
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

        assert_eq!(
            tr.selection(),
            Some(Selection::collapsed(Position::new(t1, 0)))
        );
        assert!(tr.doc().get_entry(t1).is_some());
    }

    #[test]
    fn finish_empty_transaction() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let tr = Transaction::new(&state);
        let (_, steps, _, effects, _) = tr.commit();

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

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello Beautiful World");

        let (_, steps, _, _, _) = tr.commit();

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

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello");

        let (_, steps, _, _, _) = tr.commit();

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
        let subtree = Subtree::leaf(new_id, PlainNode::Paragraph(PlainParagraphNode::default()));
        tr.insert_subtree(NodeId::ROOT, 1, subtree).unwrap();

        assert!(tr.doc().get_entry(new_id).is_some());
        let doc = tr.doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.children.len(), 2);

        let (_, steps, _, _, _) = tr.commit();
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
        assert_eq!(root.children.iter().next().copied().unwrap(), p2);

        let (_, steps, _, _, _) = tr.commit();
        match &steps[0] {
            Step::RemoveSubtree { subtree, .. } => {
                assert!(matches!(subtree.node, PlainNode::Paragraph(_)));
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
        assert_eq!(
            tr.doc()
                .get_entry(p2)
                .unwrap()
                .children
                .iter()
                .next()
                .copied()
                .unwrap(),
            t1
        );

        let (_, steps, _, _, _) = tr.commit();
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

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello");
        assert_eq!(tr.doc().text(t2).text.to_string(), " World");
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

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello World");
        assert!(tr.doc().get_entry(t2).is_none());
        assert_eq!(tr.doc().get_entry(p1).unwrap().children.len(), 1);

        let (_, steps, _, _, _) = tr.commit();
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
        let modifiers: Vec<Modifier> = entry.modifiers.iter().map(|(_, m)| m.clone()).collect();
        assert_eq!(modifiers, vec![Modifier::Bold]);

        let (_, steps, _, _, _) = tr.commit();
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

        let (_, steps, _, _, _) = tr.commit();
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
        tr.set_selection(Some(new_sel)).unwrap();

        assert_eq!(tr.selection(), Some(new_sel));

        let (_, steps, _, _, _) = tr.commit();
        match &steps[0] {
            Step::SetSelection { old, new } => {
                let old_sel = old.as_ref().expect("old must be Some");
                let new_sel_stable = new.as_ref().expect("new must be Some");
                assert_eq!(
                    old_sel.thaw(&state.doc),
                    Selection::collapsed(Position::new(t1, 0))
                );
                assert_eq!(new_sel_stable.thaw(&state.doc), new_sel);
            }
            _ => panic!("expected SetSelection"),
        }
    }

    #[test]
    fn set_node_derives_old_node_from_state() {
        let (state, c1) = state! {
            doc { root { c1: callout { paragraph { text("Hello World") } } } }
            selection: (c1, 0)
        };

        let mut tr = Transaction::new(&state);
        let new_node = PlainNode::Callout(PlainCalloutNode {
            variant: CalloutVariant::Warning,
        });
        tr.set_node(c1, new_node.clone()).unwrap();

        if let Node::Callout(n) = &tr.doc().get_entry(c1).unwrap().node {
            assert_eq!(*n.variant.get(), CalloutVariant::Warning);
        } else {
            panic!("expected Callout node");
        }

        let (_, steps, _, _, _) = tr.commit();
        match &steps[0] {
            Step::SetNode { old_node, .. } => {
                if let PlainNode::Callout(n) = old_node {
                    assert_eq!(n.variant, CalloutVariant::Info);
                } else {
                    panic!("expected Callout old_node");
                }
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

        tr.set_selection(Some(Selection::collapsed(Position::new(t2, 0))))
            .unwrap();

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello");
        assert_eq!(tr.doc().text(t2).text.to_string(), " World");
        assert_eq!(tr.doc().get_entry(p1).unwrap().children.len(), 1);
        assert_eq!(tr.doc().get_entry(p2).unwrap().children.len(), 1);
        assert_eq!(
            tr.doc()
                .get_entry(p2)
                .unwrap()
                .children
                .iter()
                .next()
                .copied()
                .unwrap(),
            t2
        );

        let (_, steps, _, _, _) = tr.commit();
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

        assert_eq!(tr.doc().text(t1).text.to_string(), "Hello World!");

        let (_, steps, _, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn steps_produce_valid_inverses() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);

        tr.insert_text(t1, 5, " Beautiful").unwrap();
        tr.set_selection(Some(Selection::collapsed(Position::new(t1, 15))))
            .unwrap();

        let (_, steps, _, _, _) = tr.commit();

        let mut current = steps
            .iter()
            .fold(state.clone(), |s, step| step.apply(&s).unwrap().state);
        for step in steps.iter().rev() {
            current = step.inverse().apply(&current).unwrap().state;
        }

        assert_eq!(current.text(t1).text.to_string(), "Hello World");
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
                Subtree::leaf(fix_id, PlainNode::Paragraph(PlainParagraphNode::default())),
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
            subtree: Subtree::leaf(p_id, PlainNode::Paragraph(PlainParagraphNode::default())),
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
        let (_, _, _, _, meta) = tr.commit();
        assert!(matches!(meta.history, HistoryMeta::Skip));
    }

    #[test]
    fn dedupe_subtree_subsumes_node() {
        let id = NodeId::new();
        let validations = vec![
            Validation::Node(id),
            Validation::Subtree(id),
            Validation::Node(id),
        ];
        let dedup = dedupe_validations(&validations);
        assert_eq!(dedup.len(), 1);
        assert!(matches!(dedup[0], Validation::Subtree(_)));
    }

    #[test]
    fn dedupe_collapses_duplicate_node() {
        let id = NodeId::new();
        let validations = vec![Validation::Node(id), Validation::Node(id)];
        let dedup = dedupe_validations(&validations);
        assert_eq!(dedup.len(), 1);
    }

    #[test]
    fn dedupe_collapses_duplicate_modifier() {
        let id = NodeId::new();
        let validations = vec![
            Validation::Modifier(id, ModifierType::Bold),
            Validation::Modifier(id, ModifierType::Bold),
            Validation::Modifier(id, ModifierType::Italic),
        ];
        let dedup = dedupe_validations(&validations);
        assert_eq!(dedup.len(), 2);
    }

    #[test]
    fn dedupe_preserves_distinct_kinds() {
        let id = NodeId::new();
        let validations = vec![
            Validation::Node(id),
            Validation::Modifier(id, ModifierType::Bold),
        ];
        let dedup = dedupe_validations(&validations);
        assert_eq!(dedup.len(), 2);
    }

    #[test]
    fn commit_seals_one_changeset() {
        let (state, t) = state! {
            doc { root { paragraph { t: text("") } } }
            selection: (t, 0)
        };
        let baseline = state.graph.changesets().len();

        let mut tr = Transaction::new(&state);
        tr.insert_text(t, 0, "a").unwrap();
        tr.insert_text(t, 1, "b").unwrap();
        let (new_state, _steps, _, _effects, _meta) = tr.commit();
        assert_eq!(
            new_state.graph.changesets().len(),
            baseline + 1,
            "1 transact = exactly 1 newly sealed cs (on top of seed)"
        );
        assert!(new_state.graph.pending().is_empty());
    }

    #[test]
    fn commit_with_no_steps_seals_no_changeset() {
        let (state, _) = state! {
            doc { root { paragraph { t: text("") } } }
            selection: (t, 0)
        };
        let baseline = state.graph.changesets().len();
        let tr = Transaction::new(&state);
        let (new_state, _, _, _, _) = tr.commit();
        assert_eq!(
            new_state.graph.changesets().len(),
            baseline,
            "no steps → commit is a no-op on changesets"
        );
        assert!(new_state.graph.pending().is_empty());
    }

    #[test]
    fn commit_returns_ops_alongside_steps() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.apply_step(Step::InsertText {
            node_id: t1,
            offset: 2,
            text: "!".to_string(),
        })
        .unwrap();
        let (_state, steps, ops, _effects, _meta) = tr.commit();
        assert!(!steps.is_empty(), "step recorded");
        assert!(!ops.is_empty(), "ops emitted by InsertText");
        assert!(
            ops.iter()
                .any(|op| matches!(op.payload, DocOp::Text { .. })),
            "ops must include DocOp::Text"
        );
    }

    #[test]
    fn savepoint_rollback_truncates_ops() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.apply_step(Step::InsertText {
            node_id: t1,
            offset: 2,
            text: "x".into(),
        })
        .unwrap();
        let sp = tr.savepoint();
        tr.apply_step(Step::InsertText {
            node_id: t1,
            offset: 3,
            text: "y".into(),
        })
        .unwrap();
        let ops_after_two = tr.ops_for_test().len();
        tr.rollback(sp);
        let ops_after_rollback = tr.ops_for_test().len();
        assert!(
            ops_after_two > ops_after_rollback,
            "rollback must truncate ops accumulated after savepoint"
        );
    }

    #[test]
    fn transaction_set_selection_none_roundtrip() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&s);
        tr.set_selection(None).unwrap();
        assert!(tr.selection().is_none());
        let (state_after, steps, _, _, _) = tr.commit();
        assert!(state_after.selection.is_none());
        assert_eq!(steps.len(), 1);
        assert!(matches!(&steps[0], Step::SetSelection { new: None, .. }));
    }

    #[test]
    fn transaction_none_to_some_to_none_via_undo() {
        let (s, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: none
        };
        let mut tr = Transaction::new(&s);
        let new_sel = Selection::collapsed(Position::new(t1, 0));
        tr.set_selection(Some(new_sel)).unwrap();
        let (state_after, steps, _, _, _) = tr.commit();
        assert_eq!(state_after.selection, Some(new_sel));

        let mut current = state_after;
        for step in steps.iter().rev() {
            current = step.inverse().apply(&current).unwrap().state;
        }
        assert!(current.selection.is_none());
    }
}

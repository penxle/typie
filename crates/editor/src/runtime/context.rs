use crate::runtime::State;
use loro::UndoManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ContextKey {
    RangeSelection,
    CanUndo,
    CanRedo,
    CanEdit,
    InComposition,
    HasParagraphTextInSelection,
    InParagraph,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum When {
    True,
    False,
    Key(ContextKey),
    Not(Box<When>),
    And(Vec<When>),
    Or(Vec<When>),
}

#[allow(dead_code)]
impl When {
    pub fn key(key: ContextKey) -> Self {
        When::Key(key)
    }

    pub fn not(self) -> Self {
        When::Not(Box::new(self))
    }

    pub fn and(self, other: When) -> Self {
        match (self, other) {
            (When::And(mut exprs), When::And(other_exprs)) => {
                exprs.extend(other_exprs);
                When::And(exprs)
            }
            (When::And(mut exprs), other) => {
                exprs.push(other);
                When::And(exprs)
            }
            (this, When::And(mut exprs)) => {
                exprs.insert(0, this);
                When::And(exprs)
            }
            (this, other) => When::And(vec![this, other]),
        }
    }

    pub fn or(self, other: When) -> Self {
        match (self, other) {
            (When::Or(mut exprs), When::Or(other_exprs)) => {
                exprs.extend(other_exprs);
                When::Or(exprs)
            }
            (When::Or(mut exprs), other) => {
                exprs.push(other);
                When::Or(exprs)
            }
            (this, When::Or(mut exprs)) => {
                exprs.insert(0, this);
                When::Or(exprs)
            }
            (this, other) => When::Or(vec![this, other]),
        }
    }

    pub fn evaluate(&self, ctx: &Context) -> bool {
        match self {
            When::True => true,
            When::False => false,
            When::Key(key) => ctx.get(*key),
            When::Not(expr) => !expr.evaluate(ctx),
            When::And(exprs) => exprs.iter().all(|e| e.evaluate(ctx)),
            When::Or(exprs) => exprs.iter().any(|e| e.evaluate(ctx)),
        }
    }
}

pub struct Context<'a> {
    state: &'a State,
    undo_manager: &'a UndoManager,
}

impl<'a> Context<'a> {
    pub fn new(state: &'a State, undo_manager: &'a UndoManager) -> Self {
        Self {
            state,
            undo_manager,
        }
    }

    pub fn get(&self, key: ContextKey) -> bool {
        match key {
            ContextKey::RangeSelection => !self.state.selection.is_collapsed(),
            ContextKey::CanUndo => self.undo_manager.can_undo(),
            ContextKey::CanRedo => self.undo_manager.can_redo(),
            ContextKey::CanEdit => !self.state.read_only,
            ContextKey::InComposition => self.state.preedit.is_some(),
            ContextKey::HasParagraphTextInSelection => self.has_paragraph_text_in_selection(),
            ContextKey::InParagraph => self.in_paragraph(),
        }
    }

    fn has_paragraph_text_in_selection(&self) -> bool {
        use crate::state::BlockTraverser;
        use crate::state::selection_helpers::{end_boundary_node, start_block_id};

        let selection = &self.state.selection;
        if selection.is_collapsed() {
            return false;
        }

        let Ok((from, to)) = selection.as_sorted(&self.state.doc) else {
            return false;
        };

        let Ok(start_id) = start_block_id(&self.state.doc, from.clone()) else {
            return false;
        };

        if let Some(block) = self.state.doc.node(start_id) {
            if block.spec().is_textblock(self.state.doc.schema()) {
                return true;
            }
        }

        if from == to {
            return false;
        }

        let end_exclusive = end_boundary_node(&self.state.doc, to).ok().flatten();

        let Ok(mut traverser) = BlockTraverser::new(&self.state.doc, start_id) else {
            return false;
        };

        while let Some(node_id) = traverser.next() {
            if Some(node_id) == end_exclusive {
                break;
            }
            if let Some(block) = self.state.doc.node(node_id) {
                if block.spec().is_textblock(self.state.doc.schema()) {
                    return true;
                }
            }
        }

        false
    }

    fn in_paragraph(&self) -> bool {
        let selection = &self.state.selection;
        if !selection.is_collapsed() {
            return false;
        }

        if let Some(container) = self.state.doc.node(selection.head.node_id) {
            return container.spec().is_textblock(self.state.doc.schema());
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Doc;
    use crate::state::Selection;
    use std::rc::Rc;

    use crate::model::NodeId;
    use crate::state::Position;
    use crate::types::Affinity;

    fn create_dummy_context<'a>(state: &'a State, undo_manager: &'a UndoManager) -> Context<'a> {
        Context::new(state, undo_manager)
    }

    #[test]
    fn test_when_logic() {
        let doc = Rc::new(Doc::new());
        let pos = Position::new(NodeId::ROOT, 0, Affinity::Downstream);
        let state = State::new(doc.clone(), Selection::collapsed(pos));
        let undo_manager = UndoManager::new(&doc.loro_doc());
        let ctx = create_dummy_context(&state, &undo_manager);

        assert_eq!(When::True.evaluate(&ctx), true);
        assert_eq!(When::False.evaluate(&ctx), false);
        assert_eq!(When::True.not().evaluate(&ctx), false);
        assert_eq!(When::True.and(When::True).evaluate(&ctx), true);
        assert_eq!(When::True.and(When::False).evaluate(&ctx), false);
        assert_eq!(When::False.or(When::True).evaluate(&ctx), true);
        assert_eq!(When::False.or(When::False).evaluate(&ctx), false);
    }
}

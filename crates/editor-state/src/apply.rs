use editor_crdt::{Changeset, CrdtError, Dot};
use editor_model::{DocOp, apply_doc_op};
use hashbrown::HashSet;

use crate::{Composition, PendingModifiers, Selection, State, StateError};

pub struct BatchedState<'a> {
    inner: &'a mut State,
}

impl<'a> std::ops::Deref for BatchedState<'a> {
    type Target = State;
    fn deref(&self) -> &State {
        self.inner
    }
}

impl<'a> BatchedState<'a> {
    pub fn apply(&mut self, payload: DocOp) -> Result<Dot, StateError> {
        self.inner.apply_internal(payload)
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.inner.selection = selection;
    }

    pub fn set_pending_modifiers(&mut self, pending: PendingModifiers) {
        self.inner.pending_modifiers = pending;
    }

    pub fn set_composition(&mut self, composition: Option<Composition>) {
        self.inner.composition = composition;
    }
}

impl State {
    fn apply_internal(&mut self, payload: DocOp) -> Result<Dot, StateError> {
        let (graph, op) = match self.graph.add(payload) {
            Ok(pair) => pair,
            Err(CrdtError::ClockOverflow { dot }) => {
                return Err(StateError::Crdt(CrdtError::ClockOverflow { dot }));
            }
            Err(other) => panic!("local create: {other:?}"),
        };
        let dot = op.id;
        let new_doc = apply_doc_op(self.doc.clone(), &op)?;

        self.graph = graph;
        self.doc = new_doc;
        Ok(dot)
    }
}

impl State {
    pub fn apply(&self, payload: DocOp) -> Result<(Self, Dot), StateError> {
        let mut next = self.clone();
        let dot = next.apply_internal(payload)?;
        next.verify()?;
        Ok((next, dot))
    }

    pub fn receive_remote_changeset(
        &self,
        changeset: Changeset<DocOp>,
    ) -> Result<Self, StateError> {
        let mut next = self.clone();
        next.graph = next
            .graph
            .receive_changeset(changeset.clone())
            .map_err(StateError::Crdt)?;
        for op in &changeset.ops {
            next.doc = apply_doc_op(next.doc.clone(), op).map_err(StateError::Model)?;
        }
        next.verify()?;
        Ok(next)
    }

    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<Changeset<DocOp>>, CrdtError> {
        self.graph.missing_changesets_for(remote_heads)
    }

    pub fn batch<F, E>(&self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        let mut next = self.clone();
        {
            let mut batched = BatchedState { inner: &mut next };
            f(&mut batched)?;
        }
        next.verify().map_err(E::from)?;
        Ok(next)
    }
}

impl State {
    pub fn verify(&self) -> Result<(), StateError> {
        self.doc.verify().map_err(StateError::Model)
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{LwwRegOp, OrMapOp, RgaOp};
    use editor_model::{
        Doc, ModelError, NodeId, NodeType, PlainDoc, PlainNode, PlainNodeEntry, PlainRootNode,
    };
    use std::collections::BTreeMap;

    use super::*;
    use crate::{Position, Selection};

    fn rooted_state() -> State {
        let root_id = NodeId::new();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: vec![],
                modifiers: BTreeMap::new(),
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        let plain = PlainDoc { nodes };
        let (doc, op_graph) = Doc::from_plain(plain);
        let sel = Selection::collapsed(Position::new(root_id, 0));
        State::new(doc, op_graph, sel)
    }

    #[test]
    fn apply_rolls_back_on_verify_failure() {
        let state = rooted_state();
        let id2 = NodeId::new();
        let result = state.apply(DocOp::Presence {
            node_id: id2,
            op: OrMapOp::Set {
                key: id2,
                value: NodeType::Root,
            },
        });
        assert!(result.is_err());
    }

    #[test]
    fn apply_rolls_back_on_kind_conflict() {
        let state = rooted_state();
        let root_id = state.doc.root().unwrap().id();
        let id = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            })?;
            b.apply(DocOp::Parent {
                node_id: id,
                op: LwwRegOp::Set {
                    value: Some(root_id),
                },
            })?;
            b.apply(DocOp::Children {
                node_id: root_id,
                op: RgaOp::Insert {
                    after: None,
                    value: id,
                },
            })?;
            Ok(())
        });
        let state = result.unwrap();
        let result = state.apply(DocOp::Presence {
            node_id: id,
            op: OrMapOp::Set {
                key: id,
                value: NodeType::Text,
            },
        });
        assert!(matches!(
            result,
            Err(StateError::Model(ModelError::PresenceKindConflict { .. }))
        ));
    }

    #[test]
    fn batch_runs_closure_then_verify_with_real_tree() {
        let state = rooted_state();
        let root_id = state.doc.root().unwrap().id();
        let child_id = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: child_id,
                op: OrMapOp::Set {
                    key: child_id,
                    value: NodeType::Paragraph,
                },
            })?;
            b.apply(DocOp::Parent {
                node_id: child_id,
                op: LwwRegOp::Set {
                    value: Some(root_id),
                },
            })?;
            b.apply(DocOp::Children {
                node_id: root_id,
                op: RgaOp::Insert {
                    after: None,
                    value: child_id,
                },
            })?;
            Ok(())
        });
        let new_state = result.unwrap();
        assert!(new_state.doc.get_entry(child_id).is_some());
    }

    #[test]
    fn batch_rolls_back_on_apply_internal_error() {
        let state = rooted_state();
        let id2 = NodeId::new();
        let id3 = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: id2,
                op: OrMapOp::Set {
                    key: id2,
                    value: NodeType::Paragraph,
                },
            })?;
            b.apply(DocOp::Presence {
                node_id: id3,
                op: OrMapOp::Set {
                    key: id3,
                    value: NodeType::Paragraph,
                },
            })?;
            b.apply(DocOp::Presence {
                node_id: id3,
                op: OrMapOp::Set {
                    key: id3,
                    value: NodeType::Text,
                },
            })?;
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn batch_rolls_back_on_closure_error() {
        let state = rooted_state();
        let id = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            })?;
            Err(StateError::Model(ModelError::AttrNodeKindMismatch))
        });
        assert!(result.is_err());
    }

    #[test]
    fn batch_rolls_back_on_verify_failure() {
        let state = rooted_state();
        let id2 = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: id2,
                op: OrMapOp::Set {
                    key: id2,
                    value: NodeType::Root,
                },
            })?;
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn verify_rejects_unreachable_node() {
        let state = rooted_state();
        let id = NodeId::new();
        let result: Result<State, StateError> = state.batch(|b| {
            b.apply(DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            })?;
            Ok(())
        });
        assert!(matches!(
            result,
            Err(StateError::Model(
                editor_model::ModelError::NodeUnreachable { .. }
            ))
        ));
    }

    #[test]
    fn verify_rejects_cycle() {
        let state = rooted_state();
        let a = NodeId::new();
        let b = NodeId::new();
        let result: Result<State, StateError> = state.batch(|bs| {
            bs.apply(DocOp::Presence {
                node_id: a,
                op: OrMapOp::Set {
                    key: a,
                    value: NodeType::Paragraph,
                },
            })?;
            bs.apply(DocOp::Presence {
                node_id: b,
                op: OrMapOp::Set {
                    key: b,
                    value: NodeType::Paragraph,
                },
            })?;
            bs.apply(DocOp::Parent {
                node_id: a,
                op: LwwRegOp::Set { value: Some(b) },
            })?;
            bs.apply(DocOp::Parent {
                node_id: b,
                op: LwwRegOp::Set { value: Some(a) },
            })?;
            bs.apply(DocOp::Children {
                node_id: b,
                op: RgaOp::Insert {
                    after: None,
                    value: a,
                },
            })?;
            bs.apply(DocOp::Children {
                node_id: a,
                op: RgaOp::Insert {
                    after: None,
                    value: b,
                },
            })?;
            Ok(())
        });
        assert!(result.is_err());
    }

    #[test]
    fn apply_does_not_seal_changeset() {
        let state = rooted_state();
        let baseline_changesets = state.graph.changesets().len();
        let baseline_pending = state.graph.pending().len();
        assert_eq!(
            baseline_pending, 0,
            "rooted_state must hand off a sealed graph"
        );

        let root_id = state.doc.root().unwrap().id();
        let child = NodeId::new();
        let new_state: State = state
            .batch(|b| {
                b.apply(DocOp::Presence {
                    node_id: child,
                    op: OrMapOp::Set {
                        key: child,
                        value: NodeType::Paragraph,
                    },
                })?;
                b.apply(DocOp::Parent {
                    node_id: child,
                    op: LwwRegOp::Set {
                        value: Some(root_id),
                    },
                })?;
                b.apply(DocOp::Children {
                    node_id: root_id,
                    op: RgaOp::Insert {
                        after: None,
                        value: child,
                    },
                })?;
                Ok::<(), StateError>(())
            })
            .expect("valid batch must succeed");

        assert_eq!(
            new_state.graph.changesets().len(),
            baseline_changesets,
            "batch must not seal a new changeset (no-seal contract)"
        );
        assert_eq!(
            new_state.graph.pending().len(),
            3,
            "all ops from the batch sit in pending until Transaction::commit"
        );
    }

    #[test]
    fn local_changesets_since_returns_vec() {
        let state = rooted_state();
        let result = state.local_changesets_since(&hashbrown::HashSet::new());
        let css = result.expect("API exists and returns Result<Vec, _>");
        assert_eq!(
            css.len(),
            state.graph.changesets().len(),
            "empty remote heads → all sealed cs are missing for remote"
        );
    }
}

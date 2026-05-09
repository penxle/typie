use editor_crdt::{Changeset, CrdtError, Dot, Op};
use editor_model::{DocOp, apply_doc_op};
use hashbrown::HashSet;

use crate::{Composition, PendingModifiers, Selection, State, StateError};

pub struct BatchedState<'a> {
    inner: &'a mut State,
    pub(crate) emitted_ops: Vec<Op<DocOp>>,
}

impl<'a> std::ops::Deref for BatchedState<'a> {
    type Target = State;
    fn deref(&self) -> &State {
        self.inner
    }
}

impl<'a> BatchedState<'a> {
    pub fn apply(&mut self, payload: DocOp) -> Result<Op<DocOp>, StateError> {
        let op = self.inner.apply_internal(payload)?;
        self.emitted_ops.push(op.clone());
        Ok(op)
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
    fn apply_internal(&mut self, payload: DocOp) -> Result<Op<DocOp>, StateError> {
        let (graph, op) = match self.graph.add(payload) {
            Ok(pair) => pair,
            Err(CrdtError::ClockOverflow { dot }) => {
                return Err(StateError::Crdt(CrdtError::ClockOverflow { dot }));
            }
            Err(other) => panic!("local create: {other:?}"),
        };
        let new_doc = apply_doc_op(self.doc.clone(), &op)?;

        self.graph = graph;
        self.doc = new_doc;
        Ok(op)
    }
}

impl State {
    pub fn apply(&self, payload: DocOp) -> Result<(Self, Op<DocOp>), StateError> {
        let mut next = self.clone();
        let op = next.apply_internal(payload)?;
        next.verify()?;
        Ok((next, op))
    }

    pub fn receive_remote_changeset(
        &self,
        changeset: Changeset<DocOp>,
    ) -> Result<(Self, Vec<Op<DocOp>>), StateError> {
        let prev_count = self.graph.changesets().len();
        let mut next = self.clone();
        next.graph = next
            .graph
            .receive_changeset(changeset.clone())
            .map_err(StateError::Crdt)?;
        let next_count = next.graph.changesets().len();
        let applied_ops = if next_count > prev_count {
            for op in &changeset.ops {
                next.doc = apply_doc_op(next.doc.clone(), op).map_err(StateError::Model)?;
            }
            changeset.ops
        } else {
            Vec::new()
        };
        next.verify()?;
        Ok((next, applied_ops))
    }

    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<Changeset<DocOp>>, CrdtError> {
        self.graph.local_changesets_since(remote_heads)
    }

    pub fn batch_with_ops<F, E>(&self, f: F) -> Result<(Self, Vec<Op<DocOp>>), E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        let mut next = self.clone();
        let ops = {
            let mut batched = BatchedState {
                inner: &mut next,
                emitted_ops: Vec::new(),
            };
            f(&mut batched)?;
            std::mem::take(&mut batched.emitted_ops)
        };
        next.verify().map_err(E::from)?;
        Ok((next, ops))
    }

    pub fn batch<F, E>(&self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        self.batch_with_ops(f).map(|(state, _ops)| state)
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
    fn batched_apply_returns_op() {
        let state = rooted_state();
        let root_id = state.doc.root().unwrap().id();
        let child_id = NodeId::new();
        let new_state: State = state
            .batch(|b| {
                let op = b.apply(DocOp::Presence {
                    node_id: child_id,
                    op: OrMapOp::Set {
                        key: child_id,
                        value: NodeType::Paragraph,
                    },
                })?;
                assert!(matches!(op.payload, DocOp::Presence { .. }));
                let _ = op.id.actor;
                let _ = op.id.clock;
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
                Ok::<(), StateError>(())
            })
            .expect("valid batch");
        assert!(new_state.doc.get_entry(child_id).is_some());
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

    #[test]
    fn duplicate_changeset_returns_empty_ops() {
        let state = rooted_state();
        let root_id = state.doc.root().unwrap().id();
        let child = NodeId::new();

        let baseline_heads: hashbrown::HashSet<_> = state.graph.current_heads().copied().collect();

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
            .unwrap();

        let committed = State {
            graph: new_state.graph.commit(),
            ..new_state
        };
        let css = committed.local_changesets_since(&baseline_heads).unwrap();
        assert!(
            !css.is_empty(),
            "batch must have produced at least one changeset"
        );
        let cs = css.into_iter().next().unwrap();

        let (after_first, ops1) = state.receive_remote_changeset(cs.clone()).unwrap();
        assert!(!ops1.is_empty(), "first delivery must yield ops");

        let (_after_second, ops2) = after_first.receive_remote_changeset(cs).unwrap();
        assert!(ops2.is_empty(), "duplicate delivery must yield empty ops");
    }

    #[test]
    fn local_changesets_since_excludes_remote_origin() {
        // Two peers share a baseline. Peer A authors a changeset and Peer B
        // ingests it via receive_remote_changeset. Peer B's
        // local_changesets_since must then return nothing — otherwise B would
        // echo A's changeset back to the server, producing the N-fold
        // duplication observed in production.
        let baseline = rooted_state();
        let baseline_css = baseline.graph.changesets().to_vec();
        let sel = baseline.selection;

        let replica_a = baseline;
        let replica_b = State::from_changesets(baseline_css.clone(), sel).unwrap();

        let baseline_b_heads: hashbrown::HashSet<_> =
            replica_b.graph.current_heads().copied().collect();

        let root_id = replica_a.doc.root().unwrap().id();
        let child = NodeId::new();
        let replica_a: State = replica_a
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
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };

        let css_from_a = replica_a.local_changesets_since(&baseline_b_heads).unwrap();
        assert!(
            !css_from_a.is_empty(),
            "replica_a must have authored a changeset"
        );

        let mut replica_b = replica_b;
        for cs in css_from_a {
            let (next, _ops) = replica_b.receive_remote_changeset(cs).unwrap();
            replica_b = next;
        }

        let echoed = replica_b
            .local_changesets_since(&hashbrown::HashSet::new())
            .unwrap();
        assert!(
            echoed.is_empty(),
            "replica_b authored nothing locally; receive must not turn remote-origin \
             changesets into push candidates (got {} cs)",
            echoed.len(),
        );
    }
}

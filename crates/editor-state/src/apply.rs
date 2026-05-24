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

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.inner.selection = selection.map(|sel| sel.normalize(&self.inner.doc).unwrap_or(sel));
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
        let op = match self.graph.add_mut(payload) {
            Ok(op) => op,
            Err(CrdtError::ClockOverflow { dot }) => {
                return Err(StateError::Crdt(CrdtError::ClockOverflow { dot }));
            }
            Err(other) => panic!("local create: {other:?}"),
        };
        let new_doc = apply_doc_op(self.doc.clone(), &op)?;
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

    pub fn would_receive_remote_changeset(
        &self,
        changeset: &Changeset<DocOp>,
    ) -> Result<bool, StateError> {
        if changeset.ops.iter().all(|op| self.graph.contains(&op.id)) {
            return Ok(false);
        }
        let snapshot = self.clone();
        let (next, ops) = snapshot.receive_remote_changeset(changeset.clone())?;
        use crate::state_observably_changed;
        Ok(state_observably_changed(self, &next) || !ops.is_empty())
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

    /// In-place variant of `batch_with_ops` for callers that already own a
    /// mutable State (e.g. inside `Transaction`). Skips the per-call
    /// `self.clone()` since the caller's state is already isolated from any
    /// other owner. On verify error the state is left in a mutated condition;
    /// callers are responsible for discarding it (Transaction is dropped
    /// without commit, so the editor's authoritative state is unaffected).
    pub fn batch_with_ops_mut<F, E>(&mut self, f: F) -> Result<Vec<Op<DocOp>>, E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        let ops = {
            let mut batched = BatchedState {
                inner: self,
                emitted_ops: Vec::new(),
            };
            f(&mut batched)?;
            std::mem::take(&mut batched.emitted_ops)
        };
        // Doc-invariant verification only matters when the closure mutated
        // the doc. State-only writes (set_selection/set_composition/etc.) emit
        // no ops, so the Doc is bit-identical to the pre-batch state and the
        // verify walk would re-check the same tree it just passed.
        if !ops.is_empty() {
            self.verify().map_err(E::from)?;
        }
        Ok(ops)
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
        State::new(doc, op_graph, Some(sel))
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
        let baseline_css = baseline.graph.changesets_as_vec();
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

    #[test]
    fn set_selection_canonicalizes_text_text_boundary_input() {
        use crate::Affinity;
        use editor_macros::state;

        let (state, ta, tb) = state! {
            doc {
                root { paragraph {
                    ta: text("Hello")
                    tb: text("World")
                } }
            }
            selection: (ta, 0)
        };

        let input = Selection::collapsed(Position {
            node_id: ta,
            offset: 5,
            affinity: Affinity::Downstream,
        });
        let result: Result<State, StateError> = state.batch(|b| {
            b.set_selection(Some(input));
            Ok(())
        });
        let next = result.unwrap();
        let sel = next.selection.as_ref().unwrap();
        assert_eq!(sel.anchor.node_id, tb);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.anchor.affinity, Affinity::Downstream);
        assert!(sel.is_collapsed());
    }

    #[test]
    fn set_selection_invalid_input_falls_back_to_raw() {
        use editor_macros::state;

        let (state, t) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 0)
        };

        let bad = Selection::collapsed(Position::new(t, 99));
        let result: Result<State, StateError> = state.batch(|b| {
            b.set_selection(Some(bad));
            Ok(())
        });
        let next = result.unwrap();
        assert_eq!(next.selection.as_ref().unwrap().anchor.offset, 99);
    }

    #[test]
    fn state_new_accepts_none_selection() {
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
        let state = State::new(doc, op_graph, None);
        assert!(state.selection.is_none());
    }

    #[test]
    fn batched_set_selection_none_clears_selection() {
        let state = rooted_state();
        let next: State = state
            .batch(|b| {
                b.set_selection(None);
                Ok::<_, StateError>(())
            })
            .unwrap();
        assert!(next.selection.is_none());
    }

    #[test]
    fn batched_set_selection_none_does_not_touch_composition_or_pending() {
        use crate::{Composition, PendingModifier, PendingModifiers};

        let state = rooted_state();
        let next: State = state
            .batch(|b| {
                b.set_composition(Some(Composition { start: 1, end: 3 }));
                b.set_pending_modifiers(PendingModifiers::from([PendingModifier::Set {
                    modifier: editor_model::Modifier::Bold,
                }]));
                b.set_selection(None);
                Ok::<_, StateError>(())
            })
            .unwrap();
        assert!(next.selection.is_none());
        assert!(
            next.composition.is_some(),
            "set_selection(None) must not touch composition"
        );
        assert!(
            !next.pending_modifiers.is_empty(),
            "set_selection(None) must not touch pending_modifiers"
        );
    }

    #[test]
    fn would_receive_remote_changeset_false_for_already_applied() {
        use editor_macros::state;

        let (replica_a, t1) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 1)
        };
        let css = replica_a.graph.changesets_as_vec();
        let replica_b = State::from_changesets(css, replica_a.selection).unwrap();

        let baseline: hashbrown::HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let (replica_a, _op) = replica_a
            .apply(DocOp::Text {
                node_id: t1,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'X',
                },
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let (replica_b, _ops) = replica_b.receive_remote_changeset(cs.clone()).unwrap();
        let probed = replica_b.would_receive_remote_changeset(&cs).unwrap();
        assert!(!probed);
    }

    #[test]
    fn would_receive_remote_changeset_true_for_new() {
        use editor_macros::state;

        let (replica_a, t1) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 1)
        };
        let css = replica_a.graph.changesets_as_vec();
        let replica_b = State::from_changesets(css, replica_a.selection).unwrap();

        let baseline: hashbrown::HashSet<_> = replica_a.graph.current_heads().copied().collect();
        let (replica_a, _op) = replica_a
            .apply(DocOp::Text {
                node_id: t1,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'X',
                },
            })
            .unwrap();
        let replica_a = State {
            graph: replica_a.graph.commit(),
            ..replica_a
        };
        let cs = replica_a
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0);

        let probed = replica_b.would_receive_remote_changeset(&cs).unwrap();
        assert!(probed);
    }
}

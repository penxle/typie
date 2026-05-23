//! Local apply variants of `CrdtError` other than `ClockOverflow` are
//! unreachable because `OpGraph::add` issues fresh dots for the local actor
//! and has no external input — verified by inspection of
//! `apply::apply_internal`'s match arms (see Notes at the bottom of this
//! file). The remote receive path is covered by the tests below.

use editor_crdt::{Changeset, CrdtError, Dot, Op, OrMapOp};
use editor_model::{
    Doc, DocOp, ModelError, NodeId, NodeType, PlainDoc, PlainNode, PlainNodeEntry, PlainRootNode,
};
use editor_state::{Position, Selection, State, StateError};
use std::collections::BTreeMap;

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
fn remote_receive_dot_conflict_is_result() {
    // Same dot, different payload — second op surfaces DotConflict from
    // `OpGraph::receive_changeset`'s Phase A, mapped to `StateError::Crdt`
    // before doc.apply or verify can run.
    let state = rooted_state();
    let some_node = NodeId::new();
    let dot = Dot::new(99, 0);

    let payload_a = DocOp::Presence {
        node_id: some_node,
        op: OrMapOp::Set {
            key: some_node,
            value: NodeType::Paragraph,
        },
    };
    let payload_b = DocOp::Presence {
        node_id: some_node,
        op: OrMapOp::Set {
            key: some_node,
            value: NodeType::Text,
        },
    };
    let op1 = Op {
        id: dot,
        parents: vec![],
        payload: payload_a,
    };
    let op2 = Op {
        id: dot,
        parents: vec![],
        payload: payload_b,
    };

    let result = state.receive_remote_changeset(Changeset {
        ops: vec![op1, op2],
    });
    assert!(matches!(
        result,
        Err(StateError::Crdt(CrdtError::DotConflict { .. }))
    ));
}

#[test]
fn remote_receive_self_reference_is_result() {
    let state = rooted_state();
    let dot = Dot::new(50, 5);
    let some_node = NodeId::new();
    let op = Op {
        id: dot,
        parents: vec![dot],
        payload: DocOp::Presence {
            node_id: some_node,
            op: OrMapOp::Set {
                key: some_node,
                value: NodeType::Paragraph,
            },
        },
    };
    let result = state.receive_remote_changeset(Changeset { ops: vec![op] });
    assert!(matches!(
        result,
        Err(StateError::Crdt(CrdtError::SelfReference { .. }))
    ));
}

#[test]
fn remote_receive_missing_parents_is_result() {
    let state = rooted_state();
    let some_node = NodeId::new();
    let op = Op {
        id: Dot::new(60, 1),
        parents: vec![Dot::new(60, 0)],
        payload: DocOp::Presence {
            node_id: some_node,
            op: OrMapOp::Set {
                key: some_node,
                value: NodeType::Paragraph,
            },
        },
    };
    let result = state.receive_remote_changeset(Changeset { ops: vec![op] });
    assert!(matches!(
        result,
        Err(StateError::Crdt(CrdtError::MissingParents { .. }))
    ));
}

#[test]
fn remote_receive_schema_root_uniqueness_is_model_error() {
    let state = rooted_state();
    let new_root = NodeId::new();
    let op = Op {
        id: Dot::new(80, 0),
        parents: vec![],
        payload: DocOp::Presence {
            node_id: new_root,
            op: OrMapOp::Set {
                key: new_root,
                value: NodeType::Root,
            },
        },
    };
    let result = state.receive_remote_changeset(Changeset { ops: vec![op] });
    assert!(matches!(
        result,
        Err(StateError::Model(
            ModelError::RootUniquenessViolation { .. }
        ))
    ));
}

#[test]
fn local_apply_root_uniqueness_is_model_error() {
    let state = rooted_state();
    let id2 = NodeId::new();
    let payload = DocOp::Presence {
        node_id: id2,
        op: OrMapOp::Set {
            key: id2,
            value: NodeType::Root,
        },
    };
    let result = state.apply(payload);
    assert!(matches!(
        result,
        Err(StateError::Model(
            ModelError::RootUniquenessViolation { .. }
        ))
    ));
}

#[test]
fn remote_receive_partial_changeset_rejects_whole() {
    // 2-op changeset: first op valid, second violates root uniqueness.
    // The fold rejects the whole changeset; the original `state` is never
    // mutated because `receive_remote_changeset` is `&self -> Result<Self, _>`.
    let state = rooted_state();

    let some_node = NodeId::new();
    let dup_root = NodeId::new();

    let op1 = Op {
        id: Dot::new(70, 0),
        parents: vec![],
        payload: DocOp::Presence {
            node_id: some_node,
            op: OrMapOp::Set {
                key: some_node,
                value: NodeType::Paragraph,
            },
        },
    };
    let op2 = Op {
        id: Dot::new(70, 1),
        parents: vec![Dot::new(70, 0)],
        payload: DocOp::Presence {
            node_id: dup_root,
            op: OrMapOp::Set {
                key: dup_root,
                value: NodeType::Root,
            },
        },
    };

    let result = state.receive_remote_changeset(Changeset {
        ops: vec![op1, op2],
    });
    assert!(matches!(
        result,
        Err(StateError::Model(
            ModelError::RootUniquenessViolation { .. }
        ))
    ));

    state
        .doc
        .verify()
        .expect("original state must remain valid");
}

#[test]
fn remote_receive_clock_overflow_is_result() {
    // `op.id.clock = u64::MAX` triggers the `checked_add(1)` overflow during
    // `OpGraph::receive_changeset` Phase A's clock-overflow check.
    let state = rooted_state();
    let some_node = NodeId::new();
    let op = Op {
        id: Dot::new(101, u64::MAX),
        parents: vec![],
        payload: DocOp::Presence {
            node_id: some_node,
            op: OrMapOp::Set {
                key: some_node,
                value: NodeType::Paragraph,
            },
        },
    };
    let result = state.receive_remote_changeset(Changeset { ops: vec![op] });
    assert!(matches!(
        result,
        Err(StateError::Crdt(CrdtError::ClockOverflow { .. }))
    ));
}

// Note: ClockOverflow's local-apply branch is reachable in principle
// (`State::apply_internal` maps it to `StateError::Crdt`), but trigger
// requires the actor's `next_clock` to advance to `u64::MAX` — 2^64 ops.
// Verified by inspection of `State::apply_internal`'s match arms.

// Note: `CrdtError::UnknownHeads` originates only in
// `OpGraph::missing_changesets_for`, not in `OpGraph::receive_changeset`,
// so it is not reachable through `State::receive_remote_changeset`.

// Note: The remaining four CrdtError variants (DotConflict / SelfReference /
// MissingParents / UnknownHeads) on the local-apply path collapse to a panic
// in `State::apply_internal` because `OpGraph::add` issues fresh dots for
// the local actor and has no external input that could trigger them.
// Verified by inspection.

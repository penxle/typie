//! Structural ops (Presence/Parent/Children) are intentionally outside the
//! generator: this codebase only provides single-replica naive
//! implementations for them, so concurrent multi-replica scenarios over
//! structural ops are not guaranteed to converge. The seed `PlainDoc`
//! establishes the tree shape; the random action stream only mutates
//! concurrent-safe payloads on top of it.

use std::collections::BTreeMap;

use editor_crdt::{Changeset, Dot, OpGraph, OrMapOp, TextOp};
use editor_model::{
    Doc, DocOp, LayoutMode, Modifier, ModifierType, Node, NodeAttr, NodeId, NodeType, PlainDoc,
    PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode, RootNodeAttr,
};
use editor_state::{Position, Selection, State};
use hashbrown::HashSet;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

fn rooted_plain_doc() -> (PlainDoc, NodeId) {
    let root_id = NodeId::new();
    let para_id = NodeId::new();
    let text_id = NodeId::new();

    let mut nodes = BTreeMap::new();
    nodes.insert(
        root_id,
        PlainNodeEntry {
            parent: None,
            children: vec![para_id],
            modifiers: BTreeMap::new(),
            style: None,
            node: PlainNode::Root(PlainRootNode::default()),
        },
    );
    nodes.insert(
        para_id,
        PlainNodeEntry {
            parent: Some(root_id),
            children: vec![text_id],
            modifiers: BTreeMap::new(),
            style: None,
            node: PlainNode::Paragraph(PlainParagraphNode {}),
        },
    );
    nodes.insert(
        text_id,
        PlainNodeEntry {
            parent: Some(para_id),
            children: vec![],
            modifiers: BTreeMap::new(),
            style: None,
            node: PlainNode::Text(PlainTextNode {
                text: String::new(),
            }),
        },
    );

    (
        PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        },
        text_id,
    )
}

fn first_text_node_id(state: &State) -> NodeId {
    state
        .doc
        .nodes_iter()
        .find(|(_, kind)| matches!(*kind, NodeType::Text))
        .map(|(id, _)| *id)
        .expect("seed PlainDoc must contain one text node")
}

fn root_node_id(state: &State) -> NodeId {
    state.doc.root().expect("seed has a root").id()
}

/// Replica 0 is built with `Doc::from_plain` (which assigns its own actor); the
/// rest replay the same changesets via `State::from_changesets` and pick fresh
/// actor ids of their own — every subsequent local emit stays unique by dot.
fn bootstrap_replicas(plain: PlainDoc, replica_count: usize) -> Vec<State> {
    assert!(replica_count >= 1);
    let (doc, graph) = Doc::from_plain(plain);
    let sel = Selection::collapsed(Position::new(doc.root().expect("seed has a root").id(), 0));
    let seed = State::new(doc, graph, Some(sel));

    // `Doc::from_plain` returns a committed graph, so the seed ops are already
    // sealed into changesets that can be replayed verbatim.
    let seed_css = seed.graph.changesets_as_vec();
    let mut replicas = Vec::with_capacity(replica_count);
    replicas.push(seed);
    for _ in 1..replica_count {
        let s = State::from_changesets(seed_css.clone(), Some(sel))
            .expect("from_changesets on bootstrap");
        replicas.push(s);
    }
    replicas
}

fn try_text_insert(state: &State, text_id: NodeId, ch: char, offset_hint: u8) -> Option<DocOp> {
    let entry = state.doc.get_entry(text_id)?;
    let Node::Text(t) = &entry.node else {
        return None;
    };
    let len = t.text.len();
    let offset = if len == 0 {
        0
    } else {
        (offset_hint as usize) % (len + 1)
    };
    let after = if offset == 0 {
        None
    } else {
        // dot_at(0) is None (before-first-char); dot_at(k) is the dot at the
        // boundary just after the k-th char — the natural anchor for an
        // insert at offset k.
        t.text.dot_at(offset).ok().flatten()
    };
    Some(DocOp::Text {
        node_id: text_id,
        op: TextOp::InsertChar { after, ch },
    })
}

fn try_text_remove(state: &State, text_id: NodeId, offset_hint: u8) -> Option<DocOp> {
    let entry = state.doc.get_entry(text_id)?;
    let Node::Text(t) = &entry.node else {
        return None;
    };
    let len = t.text.len();
    if len == 0 {
        return None;
    }
    let pick = (offset_hint as usize) % len;
    let dot = t.text.dot_at(pick + 1).ok().flatten()?;
    Some(DocOp::Text {
        node_id: text_id,
        op: TextOp::RemoveChar { observed: dot },
    })
}

fn modifier_bold_set(text_id: NodeId) -> DocOp {
    DocOp::Modifier {
        node_id: text_id,
        op: OrMapOp::Set {
            key: ModifierType::Bold,
            value: Modifier::Bold,
        },
    }
}

/// Filter out the empty-observed case — that would be a no-op and waste an
/// action slot.
fn try_modifier_bold_unset(state: &State, text_id: NodeId) -> Option<DocOp> {
    let entry = state.doc.get_entry(text_id)?;
    let observed: Vec<Dot> = entry
        .modifiers
        .tags_for(&ModifierType::Bold)
        .copied()
        .collect();
    if observed.is_empty() {
        return None;
    }
    Some(DocOp::Modifier {
        node_id: text_id,
        op: OrMapOp::Unset { observed },
    })
}

fn attr_set_root_layout(root_id: NodeId, hint: u16) -> DocOp {
    DocOp::Attr {
        node_id: root_id,
        op: NodeAttr::Root {
            attr: RootNodeAttr::LayoutMode(LayoutMode::Continuous {
                max_width: 600 + (hint as u32),
            }),
        },
    }
}

#[derive(Clone, Debug)]
enum ConcurrentSafeKind {
    TextInsert { ch: char, offset_hint: u8 },
    TextRemove { offset_hint: u8 },
    ModifierBoldSet,
    ModifierBoldUnset,
    AttrSetRootLayout { hint: u16 },
}

fn try_emit_concurrent(state: &State, kind: &ConcurrentSafeKind) -> Option<DocOp> {
    let text_id = first_text_node_id(state);
    match kind {
        ConcurrentSafeKind::TextInsert { ch, offset_hint } => {
            try_text_insert(state, text_id, *ch, *offset_hint)
        }
        ConcurrentSafeKind::TextRemove { offset_hint } => {
            try_text_remove(state, text_id, *offset_hint)
        }
        ConcurrentSafeKind::ModifierBoldSet => Some(modifier_bold_set(text_id)),
        ConcurrentSafeKind::ModifierBoldUnset => try_modifier_bold_unset(state, text_id),
        ConcurrentSafeKind::AttrSetRootLayout { hint } => {
            Some(attr_set_root_layout(root_node_id(state), *hint))
        }
    }
}

#[derive(Clone, Debug)]
enum Action {
    Emit {
        replica: u8,
        kind: ConcurrentSafeKind,
    },
    /// `prefix_hint` selects how many already-topo-sorted diff changesets to
    /// actually deliver — exercising partial sync.
    Sync { from: u8, to: u8, prefix_hint: u8 },
}

/// Collect the changesets that `from` has but `to` doesn't. Filters `to`'s
/// heads to only those `from` also knows, so `missing_changesets_for` never
/// sees an `UnknownHeads` error from `to`'s local-only ops.
fn sync_missing(
    from: &OpGraph<DocOp>,
    to: &OpGraph<DocOp>,
) -> Result<Vec<Changeset<DocOp>>, editor_crdt::CrdtError> {
    let to_known_heads: HashSet<Dot> = to
        .current_heads()
        .copied()
        .filter(|d| from.contains(d))
        .collect();
    from.missing_changesets_for(&to_known_heads)
}

#[test]
fn baseline_two_replicas_concurrent_text_insert_converges() {
    let (plain, text_id) = rooted_plain_doc();
    let mut replicas = bootstrap_replicas(plain, 2);

    let op0 = try_text_insert(&replicas[0], text_id, 'a', 0).unwrap();
    let (mut s0, _dot0) = replicas[0].apply(op0).unwrap();
    s0.graph = s0.graph.commit();
    replicas[0] = s0;

    let op1 = try_text_insert(&replicas[1], text_id, 'b', 0).unwrap();
    let (mut s1, _dot1) = replicas[1].apply(op1).unwrap();
    s1.graph = s1.graph.commit();
    replicas[1] = s1;

    for cs in sync_missing(&replicas[0].graph.clone(), &replicas[1].graph).unwrap() {
        replicas[1] = replicas[1].receive_remote_changeset(cs).unwrap().0;
    }
    for cs in sync_missing(&replicas[1].graph.clone(), &replicas[0].graph).unwrap() {
        replicas[0] = replicas[0].receive_remote_changeset(cs).unwrap().0;
    }

    assert!(replicas[0].graph.graph_state_eq(&replicas[1].graph));
    assert_eq!(replicas[0].doc.to_plain(), replicas[1].doc.to_plain());
    replicas[0].verify().unwrap();
    replicas[1].verify().unwrap();
}

fn action_strategy(replica_count: u8) -> impl Strategy<Value = Action> {
    let kind = prop_oneof![
        (any::<u8>(), 0u32..0x110000u32).prop_map(|(h, code)| {
            // Skip surrogate codepoints; fall back to ASCII 'a' if invalid.
            let ch = char::from_u32(code).unwrap_or('a');
            ConcurrentSafeKind::TextInsert { ch, offset_hint: h }
        }),
        any::<u8>().prop_map(|h| ConcurrentSafeKind::TextRemove { offset_hint: h }),
        Just(ConcurrentSafeKind::ModifierBoldSet),
        Just(ConcurrentSafeKind::ModifierBoldUnset),
        any::<u16>().prop_map(|h| ConcurrentSafeKind::AttrSetRootLayout { hint: h }),
    ];
    let emit =
        (0u8..replica_count, kind).prop_map(|(replica, kind)| Action::Emit { replica, kind });
    let sync = (0u8..replica_count, 0u8..replica_count, any::<u8>()).prop_map(
        |(from, to, prefix_hint)| Action::Sync {
            from,
            to,
            prefix_hint,
        },
    );
    prop_oneof![7 => emit, 3 => sync]
}

fn scenario_strategy() -> impl Strategy<Value = (u8, Vec<Action>)> {
    (2u8..=3u8).prop_flat_map(|count| {
        (
            Just(count),
            proptest::collection::vec(action_strategy(count), 1..=30),
        )
    })
}

/// Partial syncs interleaved with new emits can leave a replica behind in a
/// single pass, so the post-action cross-sync is iterated to a fixed point.
fn run_actions_and_converge(
    replicas: &mut [State],
    actions: Vec<Action>,
) -> Result<(), TestCaseError> {
    let count = replicas.len();
    for action in actions {
        match action {
            Action::Emit { replica, kind } => {
                let idx = (replica as usize) % count;
                if let Some(payload) = try_emit_concurrent(&replicas[idx], &kind) {
                    let (mut next, _dot) = replicas[idx]
                        .apply(payload)
                        .map_err(|e| TestCaseError::fail(format!("apply: {e:?}")))?;
                    next.graph = next.graph.commit();
                    replicas[idx] = next;
                }
            }
            Action::Sync {
                from,
                to,
                prefix_hint,
            } => {
                let from_idx = (from as usize) % count;
                let to_idx = (to as usize) % count;
                if from_idx == to_idx {
                    continue;
                }
                let missing =
                    sync_missing(&replicas[from_idx].graph.clone(), &replicas[to_idx].graph)
                        .map_err(|e| TestCaseError::fail(format!("sync_missing: {e:?}")))?;
                if missing.is_empty() {
                    continue;
                }
                // Partial delivery: deliver a prefix of the missing changesets.
                // Each changeset is self-contained so any prefix is safe.
                let take = ((prefix_hint as usize) % missing.len()) + 1;
                let mut to_state = replicas[to_idx].clone();
                for cs in missing.into_iter().take(take) {
                    to_state = to_state
                        .receive_remote_changeset(cs)
                        .map_err(|e| TestCaseError::fail(format!("receive: {e:?}")))?
                        .0;
                }
                replicas[to_idx] = to_state;
            }
        }
    }

    // Bounded by 2 * replica_count^2 rounds — each round strictly grows at
    // least one replica's op set or terminates.
    let max_rounds = 2 * count * count + 4;
    for _ in 0..max_rounds {
        let mut changed = false;
        for from in 0..count {
            for to in 0..count {
                if from == to {
                    continue;
                }
                let missing = sync_missing(&replicas[from].graph.clone(), &replicas[to].graph)
                    .map_err(|e| TestCaseError::fail(format!("converge sync_missing: {e:?}")))?;
                if !missing.is_empty() {
                    let mut to_state = replicas[to].clone();
                    for cs in missing {
                        to_state = to_state
                            .receive_remote_changeset(cs)
                            .map_err(|e| TestCaseError::fail(format!("converge receive: {e:?}")))?
                            .0;
                    }
                    replicas[to] = to_state;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    for s in replicas.iter() {
        s.verify()
            .map_err(|e| TestCaseError::fail(format!("verify: {e:?}")))?;
    }
    let plain0 = replicas[0].doc.to_plain();
    for (i, s) in replicas.iter().enumerate().skip(1) {
        let p = s.doc.to_plain();
        prop_assert!(
            p == plain0,
            "replica {} PlainDoc diverges:\n  r0={:?}\n  r{}={:?}",
            i,
            plain0,
            i,
            p
        );
        prop_assert!(
            replicas[0].graph.graph_state_eq(&s.graph),
            "replica {} OpGraph diverges from r0",
            i
        );
    }
    Ok(())
}

/// Same convergence loop as `run_actions_and_converge` but every `Changeset`
/// is round-tripped through `editor_crdt::wire::encode` → `wire::decode` before
/// `receive_remote_changeset` — exercises the full wire codec under the same
/// random action stream.
fn run_actions_and_converge_via_wire(
    replicas: &mut [State],
    actions: Vec<Action>,
) -> Result<(), TestCaseError> {
    let count = replicas.len();
    for action in actions {
        match action {
            Action::Emit { replica, kind } => {
                let idx = (replica as usize) % count;
                if let Some(payload) = try_emit_concurrent(&replicas[idx], &kind) {
                    let (mut next, _dot) = replicas[idx]
                        .apply(payload)
                        .map_err(|e| TestCaseError::fail(format!("apply: {e:?}")))?;
                    next.graph = next.graph.commit();
                    replicas[idx] = next;
                }
            }
            Action::Sync {
                from,
                to,
                prefix_hint,
            } => {
                let from_idx = (from as usize) % count;
                let to_idx = (to as usize) % count;
                if from_idx == to_idx {
                    continue;
                }
                let missing =
                    sync_missing(&replicas[from_idx].graph.clone(), &replicas[to_idx].graph)
                        .map_err(|e| TestCaseError::fail(format!("sync_missing: {e:?}")))?;
                if missing.is_empty() {
                    continue;
                }
                let take = ((prefix_hint as usize) % missing.len()) + 1;
                let mut to_state = replicas[to_idx].clone();
                for cs in missing.into_iter().take(take) {
                    let bytes = editor_crdt::wire::encode(std::slice::from_ref(&cs))
                        .map_err(|e| TestCaseError::fail(format!("encode: {e:?}")))?;
                    let decoded_vec: Vec<Changeset<DocOp>> = editor_crdt::wire::decode(&bytes)
                        .map_err(|e| TestCaseError::fail(format!("decode: {e:?}")))?;
                    assert_eq!(decoded_vec.len(), 1);
                    let decoded = decoded_vec.into_iter().next().unwrap();
                    to_state = to_state
                        .receive_remote_changeset(decoded)
                        .map_err(|e| TestCaseError::fail(format!("receive: {e:?}")))?
                        .0;
                }
                replicas[to_idx] = to_state;
            }
        }
    }

    let max_rounds = 2 * count * count + 4;
    for _ in 0..max_rounds {
        let mut changed = false;
        for from in 0..count {
            for to in 0..count {
                if from == to {
                    continue;
                }
                let missing = sync_missing(&replicas[from].graph.clone(), &replicas[to].graph)
                    .map_err(|e| TestCaseError::fail(format!("converge sync_missing: {e:?}")))?;
                if !missing.is_empty() {
                    let mut to_state = replicas[to].clone();
                    for cs in missing {
                        let bytes = editor_crdt::wire::encode(std::slice::from_ref(&cs))
                            .map_err(|e| TestCaseError::fail(format!("encode: {e:?}")))?;
                        let decoded_vec: Vec<Changeset<DocOp>> = editor_crdt::wire::decode(&bytes)
                            .map_err(|e| TestCaseError::fail(format!("decode: {e:?}")))?;
                        assert_eq!(decoded_vec.len(), 1);
                        let decoded = decoded_vec.into_iter().next().unwrap();
                        to_state = to_state
                            .receive_remote_changeset(decoded)
                            .map_err(|e| TestCaseError::fail(format!("converge receive: {e:?}")))?
                            .0;
                    }
                    replicas[to] = to_state;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    for s in replicas.iter() {
        s.verify()
            .map_err(|e| TestCaseError::fail(format!("verify: {e:?}")))?;
    }
    let plain0 = replicas[0].doc.to_plain();
    for (i, s) in replicas.iter().enumerate().skip(1) {
        let p = s.doc.to_plain();
        prop_assert!(
            p == plain0,
            "replica {} PlainDoc diverges:\n  r0={:?}\n  r{}={:?}",
            i,
            plain0,
            i,
            p
        );
        prop_assert!(
            replicas[0].graph.graph_state_eq(&s.graph),
            "replica {} OpGraph diverges from r0",
            i
        );
    }
    Ok(())
}

/// `SourceParallel` can't locate `lib.rs`/`main.rs` from an integration test
/// directory, so the default landing path becomes the test file's sibling
/// `.txt`. Pin regressions under `crates/editor-state/proptest-regressions/`
/// instead — matches the convention used elsewhere in the workspace.
fn config(cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases,
        max_shrink_iters: 10000,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            "proptest-regressions/multi_replica_convergence.txt",
        ))),
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn convergence_under_random_action_stream(
        scenario in scenario_strategy(),
    ) {
        let (count, actions) = scenario;
        let (plain, _text_id) = rooted_plain_doc();
        let mut replicas = bootstrap_replicas(plain, count as usize);
        run_actions_and_converge(&mut replicas, actions)?;
    }
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn duplicate_delivery_is_idempotent(
        scenario in scenario_strategy(),
    ) {
        let (count, actions) = scenario;
        let (plain, _text_id) = rooted_plain_doc();
        let mut replicas_a = bootstrap_replicas(plain.clone(), count as usize);
        run_actions_and_converge(&mut replicas_a, actions.clone())?;

        // Re-deliver all sealed changesets from replica 0 to replica 1. Each
        // changeset is already fully known by both replicas after convergence,
        // so `receive_remote_changeset` must accept every re-delivery without
        // mutation (idempotent).
        let all_css = replicas_a[0].graph.changesets_as_vec();
        let before_graph = replicas_a[1].graph.clone();
        let before_plain = replicas_a[1].doc.to_plain();
        let mut after = replicas_a[1].clone();
        for cs in all_css {
            after = after
                .receive_remote_changeset(cs)
                .map_err(|e| TestCaseError::fail(format!("dup receive: {e:?}")))?.0;
        }
        prop_assert!(
            before_graph.graph_state_eq(&after.graph),
            "duplicate delivery changed OpGraph"
        );
        prop_assert_eq!(before_plain, after.doc.to_plain(), "duplicate delivery changed PlainDoc");
    }
}

proptest! {
    #![proptest_config(config(256))]

    #[test]
    fn convergence_via_wire(
        scenario in scenario_strategy(),
    ) {
        let (count, actions) = scenario;
        let (plain, _text_id) = rooted_plain_doc();
        let mut replicas = bootstrap_replicas(plain, count as usize);
        run_actions_and_converge_via_wire(&mut replicas, actions)?;
    }
}

proptest! {
    #![proptest_config(config(256))]

    /// Multi-replica random transacts + random sync exchanges. After
    /// convergence, every replica must hold the same set of changeset
    /// boundaries (atomicity: a dot's boundary is fixed on first arrival
    /// across all replicas). Fingerprint by first-op `Dot` (since `DocOp`
    /// does not derive `Hash`, so `HashSet<Changeset<DocOp>>` won't
    /// compile); backstop with a sorted-Vec equality check on full content.
    #[test]
    fn boundary_preservation_under_sync(
        scenario in scenario_strategy(),
    ) {
        use std::collections::HashSet as StdSet;

        let (count, actions) = scenario;
        let (plain, _text_id) = rooted_plain_doc();
        let mut replicas = bootstrap_replicas(plain, count as usize);
        run_actions_and_converge(&mut replicas, actions)?;

        let fp = |s: &State| -> StdSet<Dot> {
            s.graph
                .changesets()
                .iter()
                .filter_map(|cs| cs.ops.first().map(|op| op.id))
                .collect()
        };
        for i in 1..replicas.len() {
            prop_assert_eq!(
                fp(&replicas[0]),
                fp(&replicas[i]),
                "replicas hold same cs boundaries"
            );
        }

        // Backstop: full-cs equality after canonical sort.
        let canon = |s: &State| -> Vec<Changeset<DocOp>> {
            let mut v = s.graph.changesets_as_vec();
            v.sort_by_key(|cs| cs.ops.first().map(|op| op.id));
            v
        };
        for i in 1..replicas.len() {
            prop_assert_eq!(
                canon(&replicas[0]),
                canon(&replicas[i]),
                "replicas agree on cs content"
            );
        }
    }
}

/// Atomicity violation simulation: deliver op_a from a real 2-op changeset
/// [op_a, op_b] to a fresh replica as a synthetic 1-op changeset, then
/// deliver op_b as a second 1-op changeset. Finally attempt to deliver the
/// original 2-op changeset — the receiver must reject it as
/// `PartialDuplicate` because op_a's dot is already present under a different
/// (1-op) boundary.
#[test]
fn partial_cs_delivery_rejected() {
    let (plain, text_id) = rooted_plain_doc();
    // Two replicas share the same bootstrap so remote ops resolve parents.
    let mut replicas = bootstrap_replicas(plain, 2);

    let op_a =
        try_text_insert(&replicas[0], text_id, 'a', 0).expect("text node must accept insert");
    let after_a: editor_state::State = replicas[0]
        .batch(|b| {
            b.apply(op_a)?;
            Ok::<(), editor_state::StateError>(())
        })
        .expect("first insert must succeed");

    let op_b = try_text_insert(&after_a, text_id, 'b', 1).expect("second insert must succeed");
    let after_ab = after_a
        .batch(|b| {
            b.apply(op_b)?;
            Ok::<(), editor_state::StateError>(())
        })
        .expect("second insert must succeed");

    let committed = editor_state::State {
        graph: after_ab.graph.commit(),
        ..after_ab
    };
    // The batch-sealed changeset is the last one; bootstrap occupies earlier slots.
    let cs = committed
        .graph
        .changesets()
        .last()
        .expect("commit must have sealed at least one changeset")
        .clone();
    assert!(
        cs.ops.len() >= 2,
        "batch must have produced a multi-op changeset, got {} ops",
        cs.ops.len()
    );

    let op_a_wrapped = Changeset {
        ops: vec![cs.ops[0].clone()],
    };
    let op_b_wrapped = Changeset {
        ops: vec![cs.ops[1].clone()],
    };

    let fresh = replicas.remove(1);
    let fresh = fresh
        .receive_remote_changeset(op_a_wrapped)
        .expect("op_a delivery must succeed")
        .0;
    let fresh = fresh
        .receive_remote_changeset(op_b_wrapped)
        .expect("op_b delivery must succeed")
        .0;

    // Delivering the original 2-op cs now must be rejected — op_a's dot is
    // already present under a different (1-op) boundary.
    let result = fresh.receive_remote_changeset(cs);
    assert!(
        matches!(
            result,
            Err(editor_state::StateError::Crdt(
                editor_crdt::CrdtError::PartialDuplicate { .. }
            ))
        ),
        "expected PartialDuplicate but got: {:?}",
        result
    );
}

proptest! {
    #![proptest_config(config(256))]

    /// Same set of changesets delivered in different causal-respecting orders
    /// to fresh replicas must yield identical doc projections. Any permutation
    /// that respects the causal partial-order (each changeset's op parents are
    /// present before the changeset itself) is a valid delivery order.
    #[test]
    fn causal_permutation_doc_projection_equal(
        scenario in scenario_strategy(),
        seed1 in any::<u64>(),
        seed2 in any::<u64>(),
    ) {
        let (count, actions) = scenario;
        let (plain, _text_id) = rooted_plain_doc();
        let mut source_replicas = bootstrap_replicas(plain, count as usize);
        run_actions_and_converge(&mut source_replicas, actions)?;

        let css: Vec<Changeset<DocOp>> = source_replicas[0].graph.changesets_as_vec();

        let perm1 = causal_permute_changesets(&css, seed1);
        let perm2 = causal_permute_changesets(&css, seed2);

        let r1 = replay_into_fresh(&perm1)
            .map_err(|e| TestCaseError::fail(format!("perm1 replay: {e:?}")))?;
        let r2 = replay_into_fresh(&perm2)
            .map_err(|e| TestCaseError::fail(format!("perm2 replay: {e:?}")))?;

        prop_assert!(r1.graph.graph_state_eq(&r2.graph));
        prop_assert_eq!(r1.doc.to_plain(), r2.doc.to_plain());
    }
}

/// Randomly permutes `css` while respecting the causal partial-order. At each
/// step any changeset whose op parents (the union of `op.parents` over all ops
/// in the cs, minus dots that belong to ops within the same cs) are all present
/// in the emitted set is considered "ready". A random one of the ready
/// changesets is selected using simple LCG steps over `seed`.
fn causal_permute_changesets(css: &[Changeset<DocOp>], seed: u64) -> Vec<Changeset<DocOp>> {
    use hashbrown::HashSet as HSet;

    let mut emitted_dots: HSet<Dot> = HSet::new();
    let mut remaining: Vec<&Changeset<DocOp>> = css.iter().collect();
    let mut result = Vec::with_capacity(css.len());
    let mut rng = seed;

    // LCG step (Knuth): produces a pseudo-random sequence from any seed.
    let next_rng = |r: u64| {
        r.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    };

    while !remaining.is_empty() {
        // Collect indices of changesets whose external parents are all known.
        let ready: Vec<usize> = remaining
            .iter()
            .enumerate()
            .filter_map(|(i, cs)| {
                let intra_cs_dots: HSet<Dot> = cs.ops.iter().map(|op| op.id).collect();
                let all_known = cs.ops.iter().all(|op| {
                    op.parents
                        .iter()
                        .all(|p| emitted_dots.contains(p) || intra_cs_dots.contains(p))
                });
                if all_known { Some(i) } else { None }
            })
            .collect();

        assert!(
            !ready.is_empty(),
            "causal cycle or missing bootstrap — should never happen"
        );

        rng = next_rng(rng);
        let pick = ready[rng as usize % ready.len()];
        let cs = remaining.remove(pick);
        for op in &cs.ops {
            emitted_dots.insert(op.id);
        }
        result.push(cs.clone());
    }

    result
}

/// Builds a fresh replica by replaying the given changesets in order via
/// `State::from_changesets`. Selection is irrelevant for graph/doc equality.
fn replay_into_fresh(css: &[Changeset<DocOp>]) -> Result<State, editor_state::StateError> {
    State::from_changesets(css.to_vec(), None)
}

use hashbrown::{HashMap, HashSet};
use std::collections::VecDeque;

use crate::{CrdtError, Dot, Op, OpGraph, SyncMessage};

/// Stable per-connection client identifier within a server's broadcast hub.
/// Distinct from `actor_id` — actor is ephemeral op-level identity, ClientId
/// is the simulator/server's connection bookkeeping.
pub type ClientId = u64;

/// `op_graph` and `pending_push` are private so `create_op` is the sole path
/// that mutates both — atomic lockstep is invariant. `inbox` is
/// `pub(crate)` for the simulator's fault-injection primitives.
#[derive(Debug)]
pub struct Replica<P> {
    pub actor_id: u64,
    pub(crate) inbox: VecDeque<SyncMessage<P>>,
    op_graph: OpGraph<P>,
    pending_push: HashSet<Dot>,
    /// Generation-layer invariant violations (DotConflict / SelfReference /
    /// ClockOverflow). MissingParents is a transient transport event and is
    /// not recorded here.
    pub receive_errors: Vec<CrdtError>,
}

impl<P: Clone> Replica<P> {
    pub fn new() -> Self {
        let actor_id = random_actor();
        Self {
            actor_id,
            inbox: VecDeque::new(),
            op_graph: OpGraph::with_actor(actor_id),
            pending_push: HashSet::new(),
            receive_errors: Vec::new(),
        }
    }

    pub fn with_actor(actor_id: u64) -> Self {
        Self {
            actor_id,
            inbox: VecDeque::new(),
            op_graph: OpGraph::with_actor(actor_id),
            pending_push: HashSet::new(),
            receive_errors: Vec::new(),
        }
    }

    pub fn op_graph(&self) -> &OpGraph<P> {
        &self.op_graph
    }

    pub fn pending_push(&self) -> &HashSet<Dot> {
        &self.pending_push
    }
}

impl<P: Clone> Default for Replica<P> {
    fn default() -> Self {
        Self::new()
    }
}

fn random_actor() -> u64 {
    let mut buf = [0u8; 8];
    getrandom::fill(&mut buf).expect("failed to generate random bytes");
    u64::from_le_bytes(buf)
}

impl<P: Clone + Eq> Replica<P> {
    /// Atomic update of `op_graph` and `pending_push`.
    pub fn create_op(&mut self, payload: P) -> Result<(Op<P>, SyncMessage<P>), CrdtError> {
        let (next, op) = self.op_graph.add(payload)?;
        let next = next.commit();
        self.op_graph = next;
        self.pending_push.insert(op.id);
        let cs = crate::Changeset {
            ops: vec![op.clone()],
        };
        let msg = SyncMessage::Changesets(vec![cs]);
        Ok((op, msg))
    }

    /// `None` when the inbox was empty; `Some(outgoing)` after processing
    /// one message — `outgoing` is non-empty only on the `ResendRequest`
    /// arm, where the server has explicitly asked the client to re-send
    /// sealed changesets from its local OpGraph.
    ///
    /// `MissingParents` on `Changesets` receive is a transient transport
    /// event (broadcast loss + child-arrived-first); silent reject.
    /// `DotConflict` / `SelfReference` / `ClockOverflow` are
    /// generation-layer violations and flow into `receive_errors`.
    pub fn process_one(&mut self) -> Option<Vec<SyncMessage<P>>> {
        let msg = self.inbox.pop_front()?;
        let mut outgoing = Vec::new();
        match msg {
            SyncMessage::Changesets(css) => {
                for cs in css {
                    match self.op_graph.receive_changeset(cs) {
                        Ok(next) => self.op_graph = next,
                        Err(CrdtError::MissingParents { .. }) => {}
                        Err(e) => self.receive_errors.push(e),
                    }
                }
            }
            SyncMessage::Dots(dots) => {
                for dot in dots {
                    self.pending_push.remove(&dot);
                }
            }
            SyncMessage::ResendRequest(dots) => {
                // Server-driven recovery: walk requested dots' full local
                // ancestry and re-send real sealed changesets, ancestry-first.
                let mut ancestry: HashSet<Dot> = HashSet::new();
                let mut walk: Vec<Dot> = dots
                    .into_iter()
                    .filter(|d| self.op_graph.contains(d))
                    .collect();
                while let Some(dot) = walk.pop() {
                    if ancestry.insert(dot)
                        && let Some(op) = self.op_graph.get(&dot)
                    {
                        walk.extend(op.parents.iter().copied());
                    }
                }
                let to_send: Vec<crate::Changeset<P>> = self
                    .op_graph
                    .changesets()
                    .iter()
                    .filter(|cs| cs.ops.iter().any(|op| ancestry.contains(&op.id)))
                    .map(|cs| cs.as_ref().clone())
                    .collect();
                if !to_send.is_empty() {
                    outgoing.push(SyncMessage::Changesets(to_send));
                }
            }
        }
        Some(outgoing)
    }

    /// Instance restart with a new actor. Models the editor instance
    /// terminating and a fresh instance opening the same doc — production
    /// hosts spin up a brand-new `OpGraph` with a freshly-minted ephemeral
    /// actor and replay the persisted changesets. This method mirrors that
    /// flow exactly: take the sealed changesets in order, build a fresh
    /// `OpGraph` with `new_actor`, replay each via `receive_changeset`.
    ///
    /// `pending_push` and `receive_errors` survive (host-side storage and
    /// observability outlive the connection). The inbox is implicitly gone
    /// — the new graph starts with no in-flight messages.
    pub fn restart_with_actor(&mut self, new_actor: u64) {
        self.actor_id = new_actor;
        let css = self.op_graph.changesets_as_vec();
        self.op_graph = css
            .into_iter()
            .try_fold(OpGraph::with_actor(new_actor), |g, cs| {
                g.receive_changeset(cs)
            })
            .expect("storage replay of own changesets never fails");
        self.inbox.clear();
    }

    /// Push side is `None` when `pending_push` is empty (avoids a redundant
    /// empty round-trip). Request side is always sent — an empty heads
    /// vector is the explicit "send me everything" signal a client uses on a
    /// fresh `OpGraph`.
    pub fn sync_messages(&self) -> (Option<SyncMessage<P>>, SyncMessage<P>) {
        let push = if self.pending_push.is_empty() {
            None
        } else {
            let to_send: Vec<crate::Changeset<P>> = self
                .op_graph
                .changesets()
                .iter()
                .filter(|cs| cs.ops.iter().any(|op| self.pending_push.contains(&op.id)))
                .map(|cs| cs.as_ref().clone())
                .collect();
            if to_send.is_empty() {
                None
            } else {
                Some(SyncMessage::Changesets(to_send))
            }
        };
        let heads: Vec<Dot> = self.op_graph.current_heads().copied().collect();
        let request = SyncMessage::Dots(heads);
        (push, request)
    }
}

/// Stateless broadcast hub: no cursor, no per-client state vector — every
/// missing-response is recomputed from the live `OpGraph`. `op_graph` is
/// private so external code cannot call `add` and mint ops under the
/// reserved actor=0 sentinel; read-only access is via `op_graph()`.
#[derive(Debug)]
pub struct Server<P> {
    op_graph: OpGraph<P>,
    pub(crate) inbound: HashMap<ClientId, VecDeque<SyncMessage<P>>>,
    pub(crate) outboxes: HashMap<ClientId, VecDeque<SyncMessage<P>>>,
    /// Generation-layer invariant violations only (DotConflict /
    /// SelfReference / ClockOverflow). MissingParents and UnknownHeads are
    /// transient transport events and are not recorded.
    pub receive_errors: Vec<CrdtError>,
}

impl<P: Clone> Server<P> {
    pub fn new() -> Self {
        Self {
            // Actor 0 reserved as a sentinel — server never calls `add` so
            // this value never mints ops; private op_graph + receive-only
            // accessor enforce this.
            op_graph: OpGraph::with_actor(0),
            inbound: HashMap::new(),
            outboxes: HashMap::new(),
            receive_errors: Vec::new(),
        }
    }

    /// Idempotent — re-registering a connected client is a no-op.
    pub fn register(&mut self, client: ClientId) {
        self.inbound.entry(client).or_default();
        self.outboxes.entry(client).or_default();
    }

    pub fn unregister(&mut self, client: ClientId) {
        self.inbound.remove(&client);
        self.outboxes.remove(&client);
    }

    pub fn op_graph(&self) -> &OpGraph<P> {
        &self.op_graph
    }
}

impl<P: Clone> Default for Server<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone + Eq> Server<P> {
    pub fn enqueue(&mut self, from: ClientId, msg: SyncMessage<P>) {
        self.inbound.entry(from).or_default().push_back(msg);
    }

    pub fn tick(&mut self, from: ClientId) -> bool {
        let Some(msg) = self.inbound.entry(from).or_default().pop_front() else {
            return false;
        };
        self.handle(from, msg);
        true
    }

    /// `Changesets` arm: accepted changesets ack back to the sender (`Dots`)
    /// and broadcast to all *other* registered clients (`Changesets`).
    /// `MissingParents` on receive is transient (push loss +
    /// child-arrived-first); silent reject. `DotConflict` /
    /// `SelfReference` / `ClockOverflow` flow into `receive_errors`.
    ///
    /// `Dots` arm: `client_heads` drives `missing_changesets_for`.
    /// `UnknownHeads` is transient (the matching push was lost); silent
    /// ignore. Empty `client_heads` is the explicit "send me everything"
    /// signal — clients opening a fresh doc rely on
    /// `missing_changesets_for(empty_set)` returning the full changeset log.
    fn handle(&mut self, from: ClientId, msg: SyncMessage<P>) {
        match msg {
            SyncMessage::Changesets(css) => {
                let mut accepted: Vec<crate::Changeset<P>> = Vec::new();
                let mut acked: Vec<Dot> = Vec::new();
                for cs in css {
                    match self.op_graph.receive_changeset(cs.clone()) {
                        Ok(next) => {
                            self.op_graph = next;
                            acked.extend(cs.ops.iter().map(|op| op.id));
                            accepted.push(cs);
                        }
                        Err(CrdtError::MissingParents { .. }) => {}
                        Err(e) => self.receive_errors.push(e),
                    }
                }
                if !acked.is_empty() {
                    self.outboxes
                        .entry(from)
                        .or_default()
                        .push_back(SyncMessage::Dots(acked));
                }
                if !accepted.is_empty() {
                    let other_ids: Vec<ClientId> = self
                        .outboxes
                        .keys()
                        .copied()
                        .filter(|&id| id != from)
                        .collect();
                    for id in other_ids {
                        self.outboxes
                            .entry(id)
                            .or_default()
                            .push_back(SyncMessage::Changesets(accepted.clone()));
                    }
                }
            }
            SyncMessage::Dots(client_heads) => {
                let heads_set: HashSet<Dot> = client_heads.into_iter().collect();
                match self.op_graph.missing_changesets_for(&heads_set) {
                    Ok(missing) => {
                        if !missing.is_empty() {
                            self.outboxes
                                .entry(from)
                                .or_default()
                                .push_back(SyncMessage::Changesets(missing));
                        }
                    }
                    Err(CrdtError::UnknownHeads { unknown }) => {
                        // Negative-ack: report the unknown dots so the
                        // client re-sends them with their local ancestry.
                        // Covers a push that hasn't arrived yet (the client
                        // re-pushes from `pending_push` on the next round)
                        // and server-side data loss after a prior ack (the
                        // client holds the op locally and the resend
                        // request is the only repair signal).
                        self.outboxes
                            .entry(from)
                            .or_default()
                            .push_back(SyncMessage::ResendRequest(unknown));
                    }
                    Err(_) => {
                        // Other CrdtError variants are unreachable from
                        // missing_changesets_for — it only emits UnknownHeads.
                        // Keep exhaustive on the Result so future error
                        // variants surface visibly.
                    }
                }
            }
            SyncMessage::ResendRequest(_) => {
                // Server-bound ResendRequest is not part of the protocol;
                // clients drive resend, not the other way around. Drop on
                // the floor rather than treat it as an error so a confused
                // peer cannot poison the server.
            }
        }
    }
}

impl<P: Clone + Eq> Server<P> {
    /// Test/bootstrap helper — directly receive an op into the server's
    /// OpGraph as a 1-op changeset. Production server only receives ops via
    /// enqueue + tick.
    pub(crate) fn debug_receive(&mut self, op: Op<P>) -> Result<(), CrdtError> {
        self.op_graph = self
            .op_graph
            .receive_changeset(crate::Changeset { ops: vec![op] })?;
        Ok(())
    }
}

const CLIENT_A: ClientId = 1;
const CLIENT_B: ClientId = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReplicaId {
    ClientA,
    ClientB,
    Server,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientReplicaId {
    ClientA,
    ClientB,
}

impl ClientReplicaId {
    fn as_client_id(self) -> ClientId {
        match self {
            ClientReplicaId::ClientA => CLIENT_A,
            ClientReplicaId::ClientB => CLIENT_B,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Action<P> {
    CreateOp {
        replica: ClientReplicaId,
        payload: P,
    },
    Tick {
        replica: ReplicaId,
    },
    /// Move one message from server's outbox to that client's inbox. Separate
    /// from `Tick(Client*)` so that `DropClientInbox` has a window to fire
    /// between the message arriving in the inbox and the client processing it.
    DrainOutbox {
        client: ClientReplicaId,
    },
    DropClientInbox {
        client: ClientReplicaId,
        msg_idx: usize,
    },
    DropServerInbound {
        client: ClientReplicaId,
        msg_idx: usize,
    },
    DropServerOutbox {
        client: ClientReplicaId,
        msg_idx: usize,
    },
    /// Restart the editor instance: terminate the current connection
    /// (flushing the client inbox, the server's inbound queue, and the
    /// server's outbox for this client) and open a fresh instance over the
    /// same doc with a new ephemeral actor. Models "user closes and reopens
    /// the doc" — the strongest disconnect shape; a pure connection drop
    /// without instance restart would keep the actor and is dominated by
    /// this case. *Initial sync NOT triggered automatically* — caller must
    /// emit a subsequent `FallbackSync` to resync. The gap models the
    /// post-restart-pre-fallback window.
    RestartInstance {
        client: ClientReplicaId,
    },
    /// Trigger one fallback sync round from the given client: emits
    /// `Changesets(...)` (if non-empty) followed by `Dots(client_heads)`
    /// into the server's inbound queue. Same-direction FIFO ordering means
    /// server processes the push before the request, ensuring
    /// `client_heads ⊆ server.OpGraph` before the missing-response walk.
    FallbackSync {
        client: ClientReplicaId,
    },
    /// Drop one op from the server's `OpGraph`. Models server-side data
    /// loss (replica failover with stale snapshot, point-in-time recovery,
    /// DB corruption). Recovery flows through the negative-ack path:
    /// the client's next fallback request hits `UnknownHeads`, the server
    /// reports the missing dots, and the client re-sends them with their
    /// local ancestry.
    ServerForgetOp {
        dot: Dot,
    },
}

#[derive(Debug)]
pub struct Simulator<P> {
    pub client_a: Replica<P>,
    pub client_b: Replica<P>,
    pub server: Server<P>,
    actor_counter: u64,
}

impl<P: Clone> Simulator<P> {
    pub fn new() -> Self {
        let mut sim = Self {
            client_a: Replica::with_actor(100),
            client_b: Replica::with_actor(200),
            server: Server::new(),
            actor_counter: 1000,
        };
        sim.server.register(CLIENT_A);
        sim.server.register(CLIENT_B);
        sim
    }

    fn next_actor(&mut self) -> u64 {
        self.actor_counter += 1;
        self.actor_counter
    }

    fn client(&self, id: ClientReplicaId) -> &Replica<P> {
        match id {
            ClientReplicaId::ClientA => &self.client_a,
            ClientReplicaId::ClientB => &self.client_b,
        }
    }

    fn client_mut(&mut self, id: ClientReplicaId) -> &mut Replica<P> {
        match id {
            ClientReplicaId::ClientA => &mut self.client_a,
            ClientReplicaId::ClientB => &mut self.client_b,
        }
    }
}

impl<P: Clone> Default for Simulator<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone + Eq> Simulator<P> {
    pub fn apply(&mut self, action: Action<P>) {
        match action {
            Action::CreateOp { replica, payload } => self.create_op(replica, payload),
            Action::Tick { replica } => self.tick(replica),
            Action::DrainOutbox { client } => self.drain_outbox(client),
            Action::DropClientInbox { client, msg_idx } => {
                self.drop_client_inbox(client, msg_idx);
            }
            Action::DropServerInbound { client, msg_idx } => {
                self.drop_server_inbound(client, msg_idx);
            }
            Action::DropServerOutbox { client, msg_idx } => {
                self.drop_server_outbox(client, msg_idx);
            }
            Action::RestartInstance { client } => self.restart_instance(client),
            Action::FallbackSync { client } => self.fallback_sync(client),
            Action::ServerForgetOp { dot } => {
                self.server.op_graph = self.server.op_graph.debug_remove(&dot);
            }
        }
    }

    /// `Some(dot)` only for `CreateOp` — used by per-op tracking proptests.
    pub fn apply_returning_dot(&mut self, action: Action<P>) -> Option<Dot> {
        match action {
            Action::CreateOp { replica, payload } => {
                let client_id = replica.as_client_id();
                let r = self.client_mut(replica);
                if let Ok((op, msg)) = r.create_op(payload) {
                    self.server.enqueue(client_id, msg);
                    Some(op.id)
                } else {
                    None
                }
            }
            other => {
                self.apply(other);
                None
            }
        }
    }

    fn create_op(&mut self, replica: ClientReplicaId, payload: P) {
        let client_id = replica.as_client_id();
        let r = self.client_mut(replica);
        if let Ok((_, msg)) = r.create_op(payload) {
            self.server.enqueue(client_id, msg);
        }
    }

    fn tick(&mut self, replica: ReplicaId) {
        match replica {
            ReplicaId::ClientA => self.tick_client(ClientReplicaId::ClientA),
            ReplicaId::ClientB => self.tick_client(ClientReplicaId::ClientB),
            ReplicaId::Server => {
                self.server.tick(CLIENT_A);
                self.server.tick(CLIENT_B);
            }
        }
    }

    fn tick_client(&mut self, client: ClientReplicaId) {
        let client_id = client.as_client_id();
        let outgoing = self.client_mut(client).process_one();
        if let Some(msgs) = outgoing {
            for msg in msgs {
                self.server.enqueue(client_id, msg);
            }
        }
    }

    fn drain_outbox(&mut self, client: ClientReplicaId) {
        let client_id = client.as_client_id();
        let outbox = self.server.outboxes.entry(client_id).or_default();
        if let Some(msg) = outbox.pop_front() {
            match client {
                ClientReplicaId::ClientA => self.client_a.inbox.push_back(msg),
                ClientReplicaId::ClientB => self.client_b.inbox.push_back(msg),
            }
        }
    }

    fn drop_client_inbox(&mut self, client: ClientReplicaId, msg_idx: usize) {
        let r = self.client_mut(client);
        if msg_idx < r.inbox.len() {
            r.inbox.remove(msg_idx);
        }
    }

    fn drop_server_inbound(&mut self, client: ClientReplicaId, msg_idx: usize) {
        let inbound = self
            .server
            .inbound
            .entry(client.as_client_id())
            .or_default();
        if msg_idx < inbound.len() {
            inbound.remove(msg_idx);
        }
    }

    fn drop_server_outbox(&mut self, client: ClientReplicaId, msg_idx: usize) {
        let outbox = self
            .server
            .outboxes
            .entry(client.as_client_id())
            .or_default();
        if msg_idx < outbox.len() {
            outbox.remove(msg_idx);
        }
    }

    fn restart_instance(&mut self, client: ClientReplicaId) {
        let client_id = client.as_client_id();
        let new_actor = self.next_actor();
        if let Some(inbound) = self.server.inbound.get_mut(&client_id) {
            inbound.clear();
        }
        if let Some(outbox) = self.server.outboxes.get_mut(&client_id) {
            outbox.clear();
        }
        let r = self.client_mut(client);
        r.restart_with_actor(new_actor);
    }

    fn fallback_sync(&mut self, client: ClientReplicaId) {
        let client_id = client.as_client_id();
        let r = self.client(client);
        let (push, request) = r.sync_messages();
        if let Some(msg) = push {
            self.server.enqueue(client_id, msg);
        }
        self.server.enqueue(client_id, request);
    }
}

impl<P: Clone + Eq> Simulator<P> {
    /// Drive the simulator to quiescence:
    /// 1. Drain all in-flight messages (server inbound, server outbox,
    ///    client inboxes) until empty.
    /// 2. Trigger a fallback sync from both clients.
    /// 3. Drain again — covers any messages newly enqueued by step 2.
    /// 4. If the snapshot diff shows no change and pending_push is empty
    ///    on both clients, stop. Otherwise loop.
    ///
    /// Termination relies on the server suppressing empty Changesets/Dots
    /// responses: once fully synced, step 2 enqueues nothing, so step 3
    /// is a no-op and the snapshot is stable.
    pub fn quiesce(&mut self) {
        for _ in 0..256 {
            self.drain_all_in_flight();
            let before = self.snapshot_queue_state();
            self.fallback_sync(ClientReplicaId::ClientA);
            self.fallback_sync(ClientReplicaId::ClientB);
            self.drain_all_in_flight();
            let after = self.snapshot_queue_state();
            if before == after
                && self.client_a.pending_push().is_empty()
                && self.client_b.pending_push().is_empty()
            {
                break;
            }
        }
    }

    fn drain_all_in_flight(&mut self) {
        for _ in 0..1024 {
            let mut progressed = false;
            for client in [CLIENT_A, CLIENT_B] {
                while self.server.tick(client) {
                    progressed = true;
                }
            }
            for client in [CLIENT_A, CLIENT_B] {
                while let Some(msg) = self.server.outboxes.entry(client).or_default().pop_front() {
                    match client {
                        CLIENT_A => self.client_a.inbox.push_back(msg),
                        CLIENT_B => self.client_b.inbox.push_back(msg),
                        _ => {}
                    }
                    progressed = true;
                }
            }
            for client in [ClientReplicaId::ClientA, ClientReplicaId::ClientB] {
                let client_id = client.as_client_id();
                loop {
                    let outgoing = self.client_mut(client).process_one();
                    let Some(msgs) = outgoing else { break };
                    progressed = true;
                    for msg in msgs {
                        self.server.enqueue(client_id, msg);
                    }
                }
            }
            if !progressed {
                break;
            }
        }
    }

    /// Snapshot of all queue lengths + OpGraph sizes for stable-state diff.
    /// Tuple order: (client_a.inbox, client_b.inbox, server_inbound,
    /// server_outbox, client_a.op_graph, client_b.op_graph,
    /// server.op_graph).
    fn snapshot_queue_state(&self) -> (usize, usize, usize, usize, usize, usize, usize) {
        let server_inbound: usize = self.server.inbound.values().map(|q| q.len()).sum();
        let server_outbox: usize = self.server.outboxes.values().map(|q| q.len()).sum();
        (
            self.client_a.inbox.len(),
            self.client_b.inbox.len(),
            server_inbound,
            server_outbox,
            self.client_a.op_graph().len(),
            self.client_b.op_graph().len(),
            self.server.op_graph().len(),
        )
    }

    pub fn converged(&self) -> bool {
        self.client_a
            .op_graph()
            .graph_state_eq(self.server.op_graph())
            && self
                .client_b
                .op_graph()
                .graph_state_eq(self.server.op_graph())
            && self.client_a.pending_push().is_empty()
            && self.client_b.pending_push().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_replica_has_empty_state() {
        let r: Replica<u32> = Replica::with_actor(1);
        assert_eq!(r.actor_id, 1);
        assert!(r.inbox.is_empty());
        assert!(r.op_graph.is_empty());
        assert!(r.pending_push.is_empty());
        assert!(r.receive_errors.is_empty());
    }

    #[test]
    fn create_op_adds_to_op_graph_and_pending_push() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        let (op, msg) = r.create_op(42).unwrap();
        assert_eq!(op.id.actor, 1);
        assert_eq!(op.payload, 42);
        assert!(r.pending_push.contains(&op.id));
        assert!(r.op_graph.contains(&op.id));
        match msg {
            SyncMessage::Changesets(css) => {
                assert_eq!(css.len(), 1);
                assert_eq!(css[0].ops.len(), 1);
                assert_eq!(css[0].ops[0].id, op.id);
            }
            _ => panic!("expected Changesets message"),
        }
    }

    #[test]
    fn process_ops_message_adds_to_op_graph() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        let foreign = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 99,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        r.inbox
            .push_back(SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![foreign.clone()],
            }]));
        let outgoing = r.process_one().expect("inbox had a message");
        assert!(outgoing.is_empty());
        assert!(r.op_graph.contains(&foreign.id));
        assert!(r.receive_errors.is_empty());
    }

    #[test]
    fn process_ops_message_silent_on_missing_parent() {
        // MissingParents = transient transport event (broadcast loss + child
        // arrived first). Silent reject — receive_errors stays empty.
        let mut r: Replica<u32> = Replica::with_actor(1);
        let orphan = Op {
            id: Dot::new(2, 0),
            parents: vec![Dot::new(99, 0)],
            payload: 1,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        r.inbox
            .push_back(SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![orphan.clone()],
            }]));
        r.process_one();
        assert!(!r.op_graph().contains(&orphan.id));
        assert!(r.receive_errors.is_empty());
    }

    #[test]
    fn process_ops_message_records_dot_conflict() {
        // DotConflict = generation-layer invariant violation — recorded.
        let mut r: Replica<u32> = Replica::with_actor(1);
        let original = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 1,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        r.inbox
            .push_back(SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![original],
            }]));
        r.process_one();
        let conflict = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 999,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        r.inbox
            .push_back(SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![conflict],
            }]));
        r.process_one();
        assert_eq!(r.receive_errors.len(), 1);
    }

    #[test]
    fn process_dots_message_removes_from_pending_push() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        let (op, _) = r.create_op(42).unwrap();
        assert!(r.pending_push.contains(&op.id));
        r.inbox.push_back(SyncMessage::Dots(vec![op.id]));
        r.process_one();
        assert!(!r.pending_push.contains(&op.id));
    }

    #[test]
    fn process_one_returns_none_when_inbox_empty() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        assert!(r.process_one().is_none());
    }

    #[test]
    fn process_resend_request_resends_full_ancestry() {
        // a → b chain, both already locally received. Server lost a (its
        // OpGraph snapshot rolled back) and asks the client to resend b;
        // the client must include the full sealed cs ancestry so a single
        // round repairs the gap.
        let mut r: Replica<u32> = Replica::with_actor(1);
        let (a, _) = r.create_op(1).unwrap();
        let (b, _) = r.create_op(2).unwrap();
        r.inbox.push_back(SyncMessage::ResendRequest(vec![b.id]));
        let mut outgoing = r.process_one().expect("inbox had a message");
        let resend = outgoing
            .pop()
            .expect("ResendRequest must produce Changesets");
        match resend {
            SyncMessage::Changesets(css) => {
                assert_eq!(css.len(), 2);
                assert_eq!(css[0].ops[0].id, a.id);
                assert_eq!(css[1].ops[0].id, b.id);
            }
            _ => panic!("expected Changesets"),
        }
    }

    #[test]
    fn restart_changes_actor_preserves_op_graph_and_pending_push() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        let (op, _) = r.create_op(1).unwrap();
        r.inbox.push_back(SyncMessage::Dots(vec![Dot::new(99, 0)]));
        r.restart_with_actor(2);
        assert_eq!(r.actor_id, 2);
        assert!(r.inbox.is_empty());
        assert!(r.op_graph.contains(&op.id));
        assert!(r.pending_push.contains(&op.id));
    }

    #[test]
    fn sync_messages_emits_pending_topo_sorted_and_heads() {
        let mut r: Replica<u32> = Replica::with_actor(1);
        let (a, _) = r.create_op(1).unwrap();
        let (b, _) = r.create_op(2).unwrap();
        let (push, request) = r.sync_messages();
        match push.expect("push present") {
            SyncMessage::Changesets(css) => {
                assert_eq!(css.len(), 2);
                assert_eq!(css[0].ops[0].id, a.id);
                assert_eq!(css[1].ops[0].id, b.id);
            }
            _ => panic!("expected Changesets"),
        }
        match request {
            SyncMessage::Dots(dots) => {
                assert_eq!(dots.len(), 1);
                assert_eq!(dots[0], b.id);
            }
            _ => panic!("expected Dots"),
        }
    }

    #[test]
    fn sync_messages_empty_replica_has_no_push_and_empty_dots() {
        // Empty replica: no pending → push None. Heads empty → Dots(empty)
        // sent (the "send me everything" signal).
        let r: Replica<u32> = Replica::with_actor(1);
        let (push, request) = r.sync_messages();
        assert!(push.is_none());
        match request {
            SyncMessage::Dots(dots) => assert!(dots.is_empty()),
            _ => panic!("expected Dots"),
        }
    }

    #[test]
    fn sync_messages_suppresses_empty_push_when_only_foreign_ops() {
        // OpGraph has ops (received from elsewhere) but pending_push is empty.
        let mut r: Replica<u32> = Replica::with_actor(1);
        let foreign = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 42,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        r.inbox
            .push_back(SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![foreign],
            }]));
        r.process_one();
        let (push, request) = r.sync_messages();
        assert!(push.is_none());
        match request {
            SyncMessage::Dots(dots) => assert!(!dots.is_empty()),
            _ => panic!("expected Dots"),
        }
    }

    #[test]
    fn server_handle_ops_accepts_and_broadcasts() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        s.register(2);
        let op = Op {
            id: Dot::new(10, 0),
            parents: vec![],
            payload: 99,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        s.enqueue(
            1,
            SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![op.clone()],
            }]),
        );
        s.tick(1);
        assert!(s.op_graph().contains(&op.id));
        let outbox_1 = s.outboxes.get(&1).unwrap();
        assert_eq!(outbox_1.len(), 1);
        assert!(matches!(&outbox_1[0], SyncMessage::Dots(d) if d == &vec![op.id]));
        let outbox_2 = s.outboxes.get(&2).unwrap();
        assert_eq!(outbox_2.len(), 1);
        assert!(matches!(
            &outbox_2[0],
            SyncMessage::Changesets(css) if css.len() == 1 && css[0].ops.len() == 1 && css[0].ops[0].id == op.id
        ));
        assert!(s.receive_errors.is_empty());
    }

    #[test]
    fn server_handle_dots_replies_with_missing() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        let a = Op {
            id: Dot::new(10, 0),
            parents: vec![],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(10, 1),
            parents: vec![a.id],
            payload: 2,
        };
        s.debug_receive(a.clone()).unwrap();
        s.debug_receive(b.clone()).unwrap();
        s.enqueue(1, SyncMessage::Dots(vec![a.id]));
        s.tick(1);
        let outbox = s.outboxes.get(&1).unwrap();
        assert_eq!(outbox.len(), 1);
        match &outbox[0] {
            SyncMessage::Changesets(css) => {
                let ids: Vec<Dot> = css
                    .iter()
                    .flat_map(|cs| cs.ops.iter().map(|op| op.id))
                    .collect();
                assert_eq!(ids, vec![b.id]);
            }
            _ => panic!("expected Changesets"),
        }
    }

    #[test]
    fn server_handle_dots_with_unknown_head_emits_resend_request() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        let unknown = Dot::new(99, 0);
        s.enqueue(1, SyncMessage::Dots(vec![unknown]));
        s.tick(1);
        assert!(s.receive_errors.is_empty());
        let outbox = s.outboxes.get(&1).unwrap();
        assert_eq!(outbox.len(), 1);
        match &outbox[0] {
            SyncMessage::ResendRequest(dots) => assert_eq!(dots, &vec![unknown]),
            _ => panic!("expected ResendRequest negative-ack"),
        }
    }

    #[test]
    fn server_handle_ops_silent_on_missing_parent() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        let orphan = Op {
            id: Dot::new(10, 0),
            parents: vec![Dot::new(99, 0)],
            payload: 1,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        s.enqueue(
            1,
            SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![orphan.clone()],
            }]),
        );
        s.tick(1);
        assert!(!s.op_graph().contains(&orphan.id));
        assert!(s.receive_errors.is_empty());
        assert_eq!(s.outboxes.get(&1).unwrap().len(), 0);
    }

    #[test]
    fn server_handle_ops_records_dot_conflict() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        let original = Op {
            id: Dot::new(10, 0),
            parents: vec![],
            payload: 1,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        s.enqueue(
            1,
            SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![original],
            }]),
        );
        s.tick(1);
        let conflict = Op {
            id: Dot::new(10, 0),
            parents: vec![],
            payload: 999,
        };
        // single-cs envelope: simulator-synthetic batch, no real boundary
        s.enqueue(
            1,
            SyncMessage::Changesets(vec![crate::Changeset {
                ops: vec![conflict],
            }]),
        );
        s.tick(1);
        assert_eq!(s.receive_errors.len(), 1);
    }

    #[test]
    fn server_handle_ops_dots_pair_does_not_emit_empty_messages() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        s.enqueue(1, SyncMessage::Changesets(vec![]));
        s.enqueue(1, SyncMessage::Dots(vec![]));
        s.tick(1);
        s.tick(1);
        assert_eq!(s.outboxes.get(&1).unwrap().len(), 0);
    }

    #[test]
    fn server_unregister_removes_inbound_and_outbox() {
        let mut s: Server<u32> = Server::new();
        s.register(1);
        s.unregister(1);
        assert!(s.inbound.get(&1).is_none());
        assert!(s.outboxes.get(&1).is_none());
    }

    #[test]
    fn empty_simulator_starts_converged() {
        let sim: Simulator<u32> = Simulator::new();
        assert!(sim.converged());
    }

    #[test]
    fn single_op_propagates_after_quiesce() {
        let mut sim: Simulator<u32> = Simulator::new();
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 42,
        });
        assert!(!sim.converged());
        sim.quiesce();
        assert!(sim.converged());
        assert_eq!(sim.client_b.op_graph().len(), 1);
    }

    #[test]
    fn op_created_after_restart_propagates_via_fallback() {
        let mut sim: Simulator<u32> = Simulator::new();
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 1,
        });
        sim.apply(Action::RestartInstance {
            client: ClientReplicaId::ClientA,
        });
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 2,
        });
        sim.quiesce();
        assert!(sim.converged());
        assert_eq!(sim.client_a.op_graph().len(), 2);
        assert_eq!(sim.client_b.op_graph().len(), 2);
    }

    #[test]
    fn drop_server_inbound_recovers_via_fallback() {
        let mut sim: Simulator<u32> = Simulator::new();
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 1,
        });
        sim.apply(Action::DropServerInbound {
            client: ClientReplicaId::ClientA,
            msg_idx: 0,
        });
        assert_eq!(sim.server.op_graph().len(), 0);
        sim.quiesce();
        assert!(sim.converged());
        assert_eq!(sim.server.op_graph().len(), 1);
    }

    #[test]
    fn server_op_loss_recovers_via_resend_request() {
        // Client creates an op, server acks it, then the server's storage
        // loses the op (modeled by `ServerForgetOp`). The client must
        // restore convergence on the next sync round through the
        // ResendRequest negative-ack path, even though `pending_push` is
        // already cleared.
        let mut sim: Simulator<u32> = Simulator::new();
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 7,
        });
        sim.quiesce();
        assert!(sim.converged());
        let lost = sim
            .client_a
            .op_graph()
            .current_heads()
            .copied()
            .next()
            .expect("client a should have one op");
        sim.apply(Action::ServerForgetOp { dot: lost });
        assert_eq!(sim.server.op_graph().len(), 0);
        sim.quiesce();
        assert!(sim.converged());
        assert!(sim.server.op_graph().contains(&lost));
    }

    #[test]
    fn server_non_head_op_loss_recovers_via_resend_request() {
        // a → b → c chain on client a; server drops the middle op b. The
        // server's frontier walk reaches b through c's parent reference,
        // reports b as unknown, and the client's resend covers a + b in
        // ancestry order so the server reconstructs the chain in one
        // round. Verifies the recovery path under non-head data loss.
        let mut sim: Simulator<u32> = Simulator::new();
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 1,
        });
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 2,
        });
        sim.apply(Action::CreateOp {
            replica: ClientReplicaId::ClientA,
            payload: 3,
        });
        sim.quiesce();
        assert!(sim.converged());

        let mut chain: Vec<Dot> = sim.client_a.op_graph().iter_all().map(|op| op.id).collect();
        chain.sort();
        let middle = chain[1];

        sim.apply(Action::ServerForgetOp { dot: middle });
        assert!(!sim.server.op_graph().contains(&middle));

        sim.quiesce();
        assert!(sim.converged());
        assert!(sim.server.op_graph().contains(&middle));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::SyncMessage;
    use proptest::prelude::*;

    fn arb_client_replica() -> impl Strategy<Value = ClientReplicaId> {
        prop_oneof![
            Just(ClientReplicaId::ClientA),
            Just(ClientReplicaId::ClientB),
        ]
    }

    fn arb_replica() -> impl Strategy<Value = ReplicaId> {
        prop_oneof![
            Just(ReplicaId::ClientA),
            Just(ReplicaId::ClientB),
            Just(ReplicaId::Server),
        ]
    }

    /// Generate one Action with bounded payload (u32) and bounded msg_idx.
    /// Weights bias toward CreateOp / Tick / DrainOutbox so that the
    /// protocol has actual work to do; drop / disconnect / fallback variants
    /// retain enough probability to exercise fault recovery.
    fn arb_action() -> impl Strategy<Value = Action<u32>> {
        prop_oneof![
            4 => (arb_client_replica(), any::<u32>())
                .prop_map(|(replica, payload)| Action::CreateOp { replica, payload }),
            3 => arb_replica()
                .prop_map(|replica| Action::Tick { replica }),
            3 => arb_client_replica()
                .prop_map(|client| Action::DrainOutbox { client }),
            1 => (arb_client_replica(), 0usize..4)
                .prop_map(|(client, msg_idx)| Action::DropClientInbox { client, msg_idx }),
            1 => (arb_client_replica(), 0usize..4)
                .prop_map(|(client, msg_idx)| Action::DropServerInbound { client, msg_idx }),
            1 => (arb_client_replica(), 0usize..4)
                .prop_map(|(client, msg_idx)| Action::DropServerOutbox { client, msg_idx }),
            1 => arb_client_replica()
                .prop_map(|client| Action::RestartInstance { client }),
            2 => arb_client_replica()
                .prop_map(|client| Action::FallbackSync { client }),
        ]
    }

    fn arb_action_sequence(max_len: usize) -> impl Strategy<Value = Vec<Action<u32>>> {
        proptest::collection::vec(arb_action(), 0..=max_len)
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            ..ProptestConfig::default()
        })]

        /// Eventual consistency under arbitrary fault sequences: applying any
        /// schedule of actions and then driving the simulator to quiescence
        /// must leave both clients and the server with identical OpGraphs and
        /// empty client `pending_push` queues.
        #[test]
        fn eventually_consistent_under_arbitrary_faults(
            actions in arb_action_sequence(40),
        ) {
            let mut sim: Simulator<u32> = Simulator::new();
            for action in actions {
                sim.apply(action);
            }
            sim.quiesce();
            prop_assert!(sim.converged(), "expected convergence after quiesce");
        }

        /// Generation-layer invariant: zero DotConflict / SelfReference /
        /// ClockOverflow errors across all replicas. MissingParents and
        /// UnknownHeads are transient transport events excluded from
        /// receive_errors.
        #[test]
        fn generation_invariant_preserved_under_arbitrary_faults(
            actions in arb_action_sequence(40)
        ) {
            let mut sim: Simulator<u32> = Simulator::new();
            for action in actions {
                sim.apply(action);
            }
            sim.quiesce();
            prop_assert!(
                sim.client_a.receive_errors.is_empty(),
                "client_a receive_errors: {:?}", sim.client_a.receive_errors
            );
            prop_assert!(
                sim.client_b.receive_errors.is_empty(),
                "client_b receive_errors: {:?}", sim.client_b.receive_errors
            );
            prop_assert!(
                sim.server.receive_errors.is_empty(),
                "server receive_errors: {:?}", sim.server.receive_errors
            );
        }

        /// Every op created by client_a reaches client_b after quiesce, and
        /// vice versa. Per-op tracking (not just final-state equality).
        #[test]
        fn cross_client_delivery(actions in arb_action_sequence(40)) {
            let mut sim: Simulator<u32> = Simulator::new();
            let mut client_a_dots: Vec<Dot> = Vec::new();
            let mut client_b_dots: Vec<Dot> = Vec::new();
            for action in actions {
                let was_create_a = matches!(
                    &action,
                    Action::CreateOp { replica: ClientReplicaId::ClientA, .. }
                );
                let was_create_b = matches!(
                    &action,
                    Action::CreateOp { replica: ClientReplicaId::ClientB, .. }
                );
                if let Some(dot) = sim.apply_returning_dot(action) {
                    if was_create_a {
                        client_a_dots.push(dot);
                    } else if was_create_b {
                        client_b_dots.push(dot);
                    }
                }
            }
            sim.quiesce();
            for dot in &client_a_dots {
                prop_assert!(
                    sim.client_b.op_graph().contains(dot),
                    "client_a's op {:?} not delivered to client_b", dot
                );
            }
            for dot in &client_b_dots {
                prop_assert!(
                    sim.client_a.op_graph().contains(dot),
                    "client_b's op {:?} not delivered to client_a", dot
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            ..ProptestConfig::default()
        })]

        /// Push message dropped, fallback recovers.
        #[test]
        fn recovers_from_push_drop(payload in any::<u32>()) {
            let mut sim: Simulator<u32> = Simulator::new();
            sim.apply(Action::CreateOp {
                replica: ClientReplicaId::ClientA,
                payload,
            });
            sim.apply(Action::DropServerInbound {
                client: ClientReplicaId::ClientA,
                msg_idx: 0,
            });
            sim.quiesce();
            prop_assert!(sim.client_a.pending_push().is_empty());
            prop_assert!(sim.converged());
        }

        /// Ack message dropped, fallback recovers.
        #[test]
        fn recovers_from_ack_drop(payload in any::<u32>()) {
            let mut sim: Simulator<u32> = Simulator::new();
            sim.apply(Action::CreateOp {
                replica: ClientReplicaId::ClientA,
                payload,
            });
            sim.apply(Action::Tick { replica: ReplicaId::Server });
            // Server has acked; drop the ack outbox message before client receives.
            sim.apply(Action::DropServerOutbox {
                client: ClientReplicaId::ClientA,
                msg_idx: 0,
            });
            sim.quiesce();
            prop_assert!(sim.client_a.pending_push().is_empty());
            prop_assert!(sim.converged());
        }

        /// Broadcast lost, fallback recovers (clientB pulls missing).
        #[test]
        fn recovers_from_broadcast_drop(payload in any::<u32>()) {
            let mut sim: Simulator<u32> = Simulator::new();
            sim.apply(Action::CreateOp {
                replica: ClientReplicaId::ClientA,
                payload,
            });
            sim.apply(Action::Tick { replica: ReplicaId::Server });
            // Drop the broadcast to clientB.
            sim.apply(Action::DropServerOutbox {
                client: ClientReplicaId::ClientB,
                msg_idx: 0,
            });
            sim.quiesce();
            prop_assert!(!sim.client_b.op_graph().is_empty());
            prop_assert!(sim.converged());
        }

        /// Drain-then-drop on client inbox: server broadcast moves into client
        /// inbox via DrainOutbox, then DropClientInbox drops it before
        /// Tick(Client) processes. Fallback must recover.
        #[test]
        fn recovers_from_drain_then_drop(payload in any::<u32>()) {
            let mut sim: Simulator<u32> = Simulator::new();
            sim.apply(Action::CreateOp {
                replica: ClientReplicaId::ClientA,
                payload,
            });
            sim.apply(Action::Tick { replica: ReplicaId::Server });
            // Server has placed broadcast Changesets into clientB outbox.
            sim.apply(Action::DrainOutbox {
                client: ClientReplicaId::ClientB,
            });
            // Broadcast now sits in client_b.inbox — drop before processing.
            sim.apply(Action::DropClientInbox {
                client: ClientReplicaId::ClientB,
                msg_idx: 0,
            });
            sim.quiesce();
            prop_assert!(sim.converged());
            prop_assert_eq!(sim.client_b.op_graph().len(), 1);
        }

        /// Generic: pending_push always empty after quiesce on arbitrary fault
        /// sequences (regression isolation for the convergence proptest's
        /// pending_push sub-property).
        #[test]
        fn pending_cleanup_generic(actions in arb_action_sequence(40)) {
            let mut sim: Simulator<u32> = Simulator::new();
            for action in actions {
                sim.apply(action);
            }
            sim.quiesce();
            prop_assert!(sim.client_a.pending_push().is_empty());
            prop_assert!(sim.client_b.pending_push().is_empty());
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            ..ProptestConfig::default()
        })]

        /// Re-delivering the same changesets yields identical OpGraph.
        /// Verifies that the sync layer's changeset re-delivery (e.g.
        /// duplicate broadcast or fallback resend) is absorbed by
        /// OpGraph::receive_changeset idempotency.
        #[test]
        fn redelivery_is_idempotent(actions in arb_action_sequence(40)) {
            let mut sim: Simulator<u32> = Simulator::new();
            for action in actions {
                sim.apply(action);
            }
            sim.quiesce();
            let snapshot_server = sim.server.op_graph().clone();
            let snapshot_a = sim.client_a.op_graph().clone();
            let snapshot_b = sim.client_b.op_graph().clone();

            // Force re-delivery: replay the server's sealed changesets to both
            // clients. After quiesce, each replica's OpGraph must remain unchanged.
            let all_css: Vec<crate::Changeset<u32>> = sim.server.op_graph().changesets_as_vec();
            sim.client_a.inbox.push_back(SyncMessage::Changesets(all_css.clone()));
            sim.client_b.inbox.push_back(SyncMessage::Changesets(all_css));
            sim.quiesce();

            prop_assert!(sim.server.op_graph().graph_state_eq(&snapshot_server));
            prop_assert!(sim.client_a.op_graph().graph_state_eq(&snapshot_a));
            prop_assert!(sim.client_b.op_graph().graph_state_eq(&snapshot_b));
            // Re-delivery must not trip generation-layer rejects — the
            // OpGraph::receive_changeset idempotency path absorbs duplicates.
            prop_assert!(sim.server.receive_errors.is_empty());
            prop_assert!(sim.client_a.receive_errors.is_empty());
            prop_assert!(sim.client_b.receive_errors.is_empty());
        }
    }
}

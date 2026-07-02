use crate::Dot;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, editor_macros::Wire)]
pub enum ListOp<P = char> {
    #[wire(n(0))]
    Ins {
        #[wire(n(0))]
        pos: usize,
        #[wire(n(1))]
        item: P,
    },
    #[wire(n(1))]
    Del {
        #[wire(n(0))]
        pos: usize,
        #[wire(n(1))]
        len: usize,
    },
    #[wire(n(2))]
    Undel {
        #[wire(n(0))]
        del: Dot,
    },
}

#[derive(Clone, Debug)]
pub struct InputEvent<P = char> {
    pub id: Dot,
    pub parents: Vec<Dot>,
    pub op: ListOp<P>,
}

/// Per-op parent list in local-version (lv) space. Inline capacity 2 keeps
/// the overwhelmingly common 0–2 parent case off the heap — a `Vec` here
/// costs one allocation per op across an entire history replay.
pub type LvParents = smallvec::SmallVec<[usize; 2]>;

/// One log entry, stored array-of-structs: the replay loop reads op, dot and
/// parents together, so one packed `Vector` costs a third of the chunk
/// allocations and cursor reads of three parallel ones.
#[derive(Clone, Debug)]
pub struct OpEntry<P> {
    pub op: ListOp<P>,
    pub dot: Dot,
    pub parents: LvParents,
}

#[derive(Clone, Debug)]
pub struct OpLog<P = char> {
    // Persistent (`imbl`) so cloning an `OpLog` — which every copy-on-write
    // `ProjectedState` mutation on the typing path does — shares storage by
    // pointer instead of deep-copying. On a large, heavily-edited document
    // a plain `Vec`/`HashMap` here grows to `O(total ops)` per clone and
    // dominates the per-keystroke cost.
    pub entries: imbl::Vector<OpEntry<P>>,
    pub lv_of: crate::DotMap<usize>,
}

impl<P: Clone> Default for OpLog<P> {
    fn default() -> Self {
        OpLog {
            entries: imbl::Vector::new(),
            lv_of: crate::DotMap::new(),
        }
    }
}

impl<P: Clone> OpLog<P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: Dot) -> bool {
        self.lv_of.contains_key(&id)
    }

    pub fn push(&mut self, ev: InputEvent<P>) -> usize {
        self.push_from(ev.id, &ev.parents, ev.op)
    }

    /// `push` without requiring an owned parent `Vec` — replay paths that
    /// already hold the parents as a slice skip one allocation per op.
    pub fn push_from(&mut self, id: Dot, parents: &[Dot], op: ListOp<P>) -> usize {
        let lv = self.entries.len();
        assert!(
            !self.lv_of.contains_key(&id),
            "duplicate Dot pushed to OpLog"
        );
        let parents: LvParents = parents.iter().map(|p| self.lv_of[p]).collect();
        self.entries.push_back(OpEntry {
            op,
            dot: id,
            parents,
        });
        self.lv_of.insert(id, lv);
        lv
    }

    pub fn extend(
        &mut self,
        evs: impl IntoIterator<Item = InputEvent<P>>,
    ) -> std::ops::Range<usize> {
        let start = self.entries.len();
        for ev in evs {
            self.push(ev);
        }
        start..self.entries.len()
    }
}

pub(crate) const NYI: i32 = -1;
pub(crate) fn item_width(state: i32) -> usize {
    if state == 0 { 1 } else { 0 }
}

fn topo_order<P>(events: &[InputEvent<P>]) -> Vec<Dot> {
    let by_dot: HashMap<Dot, &InputEvent<P>> = events.iter().map(|e| (e.id, e)).collect();
    enum F {
        Enter(Dot),
        Emit(Dot),
    }
    let mut roots: Vec<Dot> = events.iter().map(|e| e.id).collect();
    roots.sort();
    let mut stack: Vec<F> = roots.into_iter().rev().map(F::Enter).collect();
    let mut seen = HashSet::new();
    let mut order: Vec<Dot> = Vec::new();
    while let Some(f) = stack.pop() {
        match f {
            F::Enter(d) => {
                if !seen.insert(d) {
                    continue;
                }
                let e = by_dot[&d];
                stack.push(F::Emit(d));
                let mut ps = e.parents.clone();
                ps.sort();
                for p in ps.into_iter().rev() {
                    stack.push(F::Enter(p));
                }
            }
            F::Emit(d) => order.push(d),
        }
    }
    order
}

pub fn build_oplog<P: Clone>(events: &[InputEvent<P>]) -> OpLog<P> {
    let by_dot: HashMap<Dot, &InputEvent<P>> = events.iter().map(|e| (e.id, e)).collect();
    let mut log = OpLog::new();
    for d in topo_order(events) {
        log.push(by_dot[&d].clone());
    }
    log
}

pub(crate) fn lv_cmp<P: Clone>(log: &OpLog<P>, a: usize, b: usize) -> std::cmp::Ordering {
    let da = &log.entries[a].dot;
    let db = &log.entries[b].dot;
    da.actor.cmp(&db.actor).then(da.clock.cmp(&db.clock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dot;

    fn ins(a: u64, c: u64, parents: &[Dot], ch: char) -> InputEvent<char> {
        InputEvent {
            id: Dot::new(a, c),
            parents: parents.to_vec(),
            op: ListOp::Ins { pos: 0, item: ch },
        }
    }

    #[test]
    fn new_is_empty() {
        let log: OpLog<char> = OpLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert!(!log.contains(Dot::new(1, 0)));
    }

    #[test]
    fn push_assigns_sequential_lv_and_maps_parents() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let mut log: OpLog<char> = OpLog::new();
        let lv_a = log.push(ins(1, 0, &[], 'a'));
        let lv_b = log.push(ins(1, 1, &[a], 'b'));
        assert_eq!(lv_a, 0);
        assert_eq!(lv_b, 1);
        assert_eq!(log.len(), 2);
        assert!(log.contains(a) && log.contains(b));
        assert_eq!(
            log.entries.iter().map(|e| e.dot).collect::<Vec<_>>(),
            vec![a, b]
        );
        assert_eq!(log.entries[lv_b].parents.as_slice(), &[lv_a]);
        assert_eq!(log.lv_of[&a], 0);
        assert_eq!(log.lv_of[&b], 1);
    }

    #[test]
    fn extend_returns_range_and_resolves_intra_batch_parents() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let mut log: OpLog<char> = OpLog::new();
        let range = log.extend(vec![
            ins(1, 0, &[], 'a'),
            ins(1, 1, &[a], 'b'),
            ins(1, 2, &[b], 'c'),
        ]);
        assert_eq!(range, 0..3);
        assert_eq!(
            log.entries.iter().map(|e| e.dot).collect::<Vec<_>>(),
            vec![a, b, c]
        );
        assert_eq!(
            log.entries
                .iter()
                .map(|e| e.parents.to_vec())
                .collect::<Vec<_>>(),
            vec![Vec::<usize>::new(), vec![0usize], vec![1usize]]
        );
    }

    #[test]
    fn lv_is_append_stable() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let mut log: OpLog<char> = OpLog::new();
        log.push(ins(1, 0, &[], 'a'));
        let lv_a = log.lv_of[&a];
        log.push(ins(1, 1, &[a], 'b'));
        log.push(ins(1, 2, &[b], 'c'));
        assert_eq!(log.lv_of[&a], lv_a);
        assert_eq!(log.lv_of[&a], 0);
    }

    #[test]
    #[should_panic(expected = "duplicate Dot")]
    fn push_duplicate_dot_panics() {
        let mut log: OpLog<char> = OpLog::new();
        log.push(ins(1, 0, &[], 'a'));
        log.push(ins(1, 0, &[], 'a'));
    }

    #[test]
    fn build_oplog_linearization_stable_chain() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let ev = vec![
            ins(1, 0, &[], 'a'),
            ins(1, 1, &[a], 'b'),
            ins(1, 2, &[b], 'c'),
        ];
        let log = build_oplog(&ev);
        assert_eq!(
            log.entries.iter().map(|e| e.dot).collect::<Vec<_>>(),
            vec![a, b, c]
        );
        assert_eq!(
            log.entries
                .iter()
                .map(|e| e.parents.to_vec())
                .collect::<Vec<_>>(),
            vec![Vec::<usize>::new(), vec![0usize], vec![1usize]]
        );
        assert_eq!(log.lv_of[&a], 0);
        assert_eq!(log.lv_of[&c], 2);
    }

    #[test]
    fn build_oplog_linearization_stable_diamond() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(2, 0);
        let d = Dot::new(1, 2);
        let ev = vec![
            ins(1, 0, &[], 'a'),
            ins(1, 1, &[a], 'b'),
            ins(2, 0, &[a], 'c'),
            InputEvent {
                id: d,
                parents: vec![b, c],
                op: ListOp::Ins { pos: 0, item: 'd' },
            },
        ];
        let log = build_oplog(&ev);
        assert_eq!(
            log.entries.iter().map(|e| e.dot).collect::<Vec<_>>(),
            vec![a, c, b, d]
        );
        assert_eq!(
            log.entries
                .iter()
                .map(|e| e.parents.to_vec())
                .collect::<Vec<_>>(),
            vec![
                Vec::<usize>::new(),
                vec![0usize],
                vec![0usize],
                vec![2usize, 1usize]
            ]
        );
    }

    use crate::sequence::checkout_text;
    use proptest::prelude::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

    fn sub(events: &[InputEvent<char>], parents: &[Dot]) -> Vec<InputEvent<char>> {
        let map: HashMap<Dot, &InputEvent<char>> = events.iter().map(|e| (e.id, e)).collect();
        let mut anc: HashSet<Dot> = HashSet::new();
        let mut st = parents.to_vec();
        while let Some(d) = st.pop() {
            if anc.insert(d)
                && let Some(e) = map.get(&d)
            {
                st.extend(e.parents.iter().copied());
            }
        }
        events
            .iter()
            .filter(|e| anc.contains(&e.id))
            .cloned()
            .collect()
    }

    fn build_undel(raw: Vec<(u64, u8, u8, u8, char, bool)>) -> Vec<InputEvent<char>> {
        let mut clock: HashMap<u64, u64> = HashMap::new();
        let mut front: BTreeMap<u64, Dot> = BTreeMap::new();
        let mut del_authored: HashMap<u64, Vec<Dot>> = HashMap::new();
        let mut undeled: HashSet<Dot> = HashSet::new();
        let mut out: Vec<InputEvent<char>> = Vec::new();
        for (actor, action, target, del_len, ch, sync_other) in raw {
            let c = *clock.get(&actor).unwrap_or(&0);
            clock.insert(actor, c + 1);
            let id = Dot::new(actor, c);
            let mut parents: Vec<Dot> = Vec::new();
            if let Some(f) = front.get(&actor) {
                parents.push(*f);
            }
            if sync_other && let Some((_, f)) = front.iter().find(|(a, _)| **a != actor) {
                parents.push(*f);
            }
            let vis = checkout_text(&build_oplog(&sub(&out, &parents)))
                .chars()
                .count();
            let candidates: Vec<Dot> = del_authored
                .get(&actor)
                .map(|v| v.iter().copied().filter(|d| !undeled.contains(d)).collect())
                .unwrap_or_default();
            let op = if action % 3 == 2 && !candidates.is_empty() {
                let del = candidates[(target as usize) % candidates.len()];
                undeled.insert(del);
                ListOp::Undel { del }
            } else if action % 3 == 1 && vis > 0 {
                let pos = (target as usize) % vis;
                let len = 1 + (del_len as usize) % (vis - pos);
                del_authored.entry(actor).or_default().push(id);
                ListOp::Del { pos, len }
            } else {
                ListOp::Ins {
                    pos: (target as usize) % (vis + 1),
                    item: ch,
                }
            };
            out.push(InputEvent { id, parents, op });
            front.insert(actor, id);
        }
        out
    }

    fn arb_events_undel(max: usize, actors: u64) -> impl Strategy<Value = Vec<InputEvent<char>>> {
        proptest::collection::vec(
            (
                0u64..actors,
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
                any::<char>(),
                any::<bool>(),
            ),
            0..=max,
        )
        .prop_map(build_undel)
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]
        #[test]
        fn append_order_matches_dfs_linearization(events in arb_events_undel(40, 3)) {
            let cold = build_oplog(&events);
            let mut warm: OpLog<char> = OpLog::new();
            for e in &events {
                warm.push(e.clone());
            }
            prop_assert_eq!(checkout_text(&cold), checkout_text(&warm));
            let cold_dots: BTreeSet<Dot> = cold.entries.iter().map(|e| e.dot).collect();
            let warm_dots: BTreeSet<Dot> = warm.entries.iter().map(|e| e.dot).collect();
            prop_assert_eq!(cold_dots, warm_dots);
        }
    }
}

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

#[derive(Clone, Debug)]
pub struct OpLog<P = char> {
    // All four are persistent (`imbl`) so cloning an `OpLog` — which every
    // copy-on-write `ProjectedState` mutation on the typing path does — shares them
    // by pointer instead of deep-copying. On a large, heavily-edited document (a
    // 5MB op history) the `dots` bulk memcpy and, especially, the `lv_of` HashMap
    // rehash grow to `O(total ops)` and dominate `ProjectedState::clone` (~7ms/clone
    // measured). `imbl::HashMap`/`Vector` clone in `O(1)`; the reads that pay the
    // `O(log n)` structural-lookup tax are the same ones already indexing `ops`.
    pub ops: imbl::Vector<ListOp<P>>,
    pub dots: imbl::Vector<Dot>,
    pub parents: imbl::Vector<Vec<usize>>,
    pub lv_of: imbl::HashMap<Dot, usize>,
}

impl<P: Clone> Default for OpLog<P> {
    fn default() -> Self {
        OpLog {
            ops: imbl::Vector::new(),
            dots: imbl::Vector::new(),
            parents: imbl::Vector::new(),
            lv_of: imbl::HashMap::new(),
        }
    }
}

impl<P: Clone> OpLog<P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn contains(&self, id: Dot) -> bool {
        self.lv_of.contains_key(&id)
    }

    pub fn push(&mut self, ev: InputEvent<P>) -> usize {
        let lv = self.ops.len();
        assert!(
            !self.lv_of.contains_key(&ev.id),
            "duplicate Dot pushed to OpLog"
        );
        let parents: Vec<usize> = ev.parents.iter().map(|p| self.lv_of[p]).collect();
        self.ops.push_back(ev.op);
        self.dots.push_back(ev.id);
        self.parents.push_back(parents);
        self.lv_of.insert(ev.id, lv);
        lv
    }

    pub fn extend(
        &mut self,
        evs: impl IntoIterator<Item = InputEvent<P>>,
    ) -> std::ops::Range<usize> {
        let start = self.ops.len();
        for ev in evs {
            self.push(ev);
        }
        start..self.ops.len()
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

pub(crate) fn lv_cmp<P>(log: &OpLog<P>, a: usize, b: usize) -> std::cmp::Ordering {
    let da = &log.dots[a];
    let db = &log.dots[b];
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
        assert_eq!(log.dots.iter().copied().collect::<Vec<_>>(), vec![a, b]);
        assert_eq!(log.parents[lv_b], vec![lv_a]);
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
        assert_eq!(log.dots.iter().copied().collect::<Vec<_>>(), vec![a, b, c]);
        assert_eq!(
            log.parents.iter().cloned().collect::<Vec<_>>(),
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
        assert_eq!(log.dots.iter().copied().collect::<Vec<_>>(), vec![a, b, c]);
        assert_eq!(
            log.parents.iter().cloned().collect::<Vec<_>>(),
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
            log.dots.iter().copied().collect::<Vec<_>>(),
            vec![a, c, b, d]
        );
        assert_eq!(
            log.parents.iter().cloned().collect::<Vec<_>>(),
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
            let cold_dots: BTreeSet<Dot> = cold.dots.iter().copied().collect();
            let warm_dots: BTreeSet<Dot> = warm.dots.iter().copied().collect();
            prop_assert_eq!(cold_dots, warm_dots);
        }
    }
}

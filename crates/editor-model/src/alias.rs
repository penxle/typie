use editor_crdt::{Dot, FastMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasRun {
    pub old_start: Dot,
    pub len: u32,
    pub new_start: Dot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasOp {
    pub pairs: Vec<AliasRun>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AliasLog {
    ops: imbl::Vector<AliasOp>,
}

impl AliasLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, op: AliasOp) {
        self.ops.push_back(op);
    }

    pub fn iter(&self) -> impl Iterator<Item = &AliasOp> {
        self.ops.iter()
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AliasClasses {
    rep_of: FastMap<Dot, Dot>,
    members: FastMap<Dot, Vec<Dot>>,
}

impl AliasClasses {
    pub fn from_log(log: &AliasLog) -> Self {
        let mut classes = Self::default();
        for op in log.iter() {
            classes.apply(op);
        }
        classes
    }

    pub fn apply(&mut self, op: &AliasOp) {
        if !alias_op_is_valid(op) {
            debug_assert!(
                false,
                "invalid alias op: creation-boundary validation is the first line"
            );
            return;
        }
        for run in &op.pairs {
            for i in 0..run.len as u64 {
                self.union(
                    Dot::new(run.old_start.actor, run.old_start.clock + i),
                    Dot::new(run.new_start.actor, run.new_start.clock + i),
                );
            }
        }
    }
}

/// Single admission gate for `AliasOp`: every producer of a locally-generated
/// `AliasOp` (currently `ProjectedState::apply_op_warm`) must call this before the
/// op reaches `graph.add_mut`. `fragment_builder`/`load_builder` build their own
/// `OpGraph` outside `ProjectedState` and do not emit `EditOp::Alias` today; if they
/// ever do, route that emission through this same check.
pub fn alias_op_is_valid(op: &AliasOp) -> bool {
    let mut ranges: Vec<(u64, u64, u64)> = Vec::with_capacity(op.pairs.len() * 2);
    for run in &op.pairs {
        if run.len == 0 || run.old_start == run.new_start {
            return false;
        }
        let l = run.len as u64;
        let (Some(old_end), Some(new_end)) = (
            run.old_start.clock.checked_add(l - 1),
            run.new_start.clock.checked_add(l - 1),
        ) else {
            return false;
        };
        let old_last = Dot::new(run.old_start.actor, old_end);
        let new_last = Dot::new(run.new_start.actor, new_end);
        if run.old_start.is_synthetic()
            || old_last.is_synthetic()
            || run.new_start.is_synthetic()
            || new_last.is_synthetic()
        {
            return false;
        }
        ranges.push((run.old_start.actor, run.old_start.clock, old_end));
        ranges.push((run.new_start.actor, run.new_start.clock, new_end));
    }
    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (aa, as_, ae) = ranges[i];
            let (ba, bs, be) = ranges[j];
            if aa == ba && as_ <= be && bs <= ae {
                return false;
            }
        }
    }
    true
}

impl AliasClasses {
    pub fn contains(&self, d: Dot) -> bool {
        self.rep_of.contains_key(&d)
    }

    pub fn members_of(&self, d: Dot) -> Option<&[Dot]> {
        let rep = *self.rep_of.get(&d)?;
        self.members.get(&rep).map(|v| v.as_slice())
    }

    pub fn resolve_with(&self, d: Dot, is_visible: impl Fn(Dot) -> bool) -> Dot {
        let Some(rep) = self.rep_of.get(&d) else {
            return d;
        };
        if is_visible(d) {
            return d;
        }
        self.members[rep]
            .iter()
            .rev()
            .copied()
            .find(|m| is_visible(*m))
            .unwrap_or(d)
    }

    fn rep_of_or_create(&mut self, d: Dot) -> Dot {
        if let Some(r) = self.rep_of.get(&d) {
            return *r;
        }
        self.rep_of.insert(d, d);
        self.members.insert(d, vec![d]);
        d
    }

    fn union(&mut self, a: Dot, b: Dot) {
        if a == b {
            return;
        }
        let ra = self.rep_of_or_create(a);
        let rb = self.rep_of_or_create(b);
        if ra == rb {
            return;
        }
        let (keep, drop) = if ra < rb { (ra, rb) } else { (rb, ra) };
        let moved = self.members.remove(&drop).unwrap_or_default();
        for d in &moved {
            self.rep_of.insert(*d, keep);
        }
        let merged = self.members.get_mut(&keep).expect("keep class exists");
        merged.extend(moved);
        merged.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::Dot;

    fn run(old: (u64, u64), len: u32, new: (u64, u64)) -> AliasRun {
        AliasRun {
            old_start: Dot::new(old.0, old.1),
            len,
            new_start: Dot::new(new.0, new.1),
        }
    }

    #[test]
    fn unaliased_dot_resolves_to_itself_without_visibility_call() {
        let classes = AliasClasses::from_log(&AliasLog::new());
        let d = Dot::new(1, 5);
        let resolved = classes.resolve_with(d, |_| panic!("visibility must not be consulted"));
        assert_eq!(resolved, d);
    }

    #[test]
    fn dead_dot_resolves_to_visible_member() {
        let mut log = AliasLog::new();
        log.apply(AliasOp {
            pairs: vec![run((1, 10), 3, (2, 20))],
        });
        let classes = AliasClasses::from_log(&log);
        let visible = |d: Dot| d.actor == 2;
        assert_eq!(
            classes.resolve_with(Dot::new(1, 11), visible),
            Dot::new(2, 21)
        );
    }

    #[test]
    fn visible_input_short_circuits_to_itself() {
        let mut classes = AliasClasses::from_log(&AliasLog::new());
        classes.apply(&AliasOp {
            pairs: vec![run((1, 10), 1, (2, 20))],
        });
        assert_eq!(
            classes.resolve_with(Dot::new(1, 10), |_| true),
            Dot::new(1, 10)
        );
    }

    #[test]
    fn no_visible_member_returns_input() {
        let mut classes = AliasClasses::from_log(&AliasLog::new());
        classes.apply(&AliasOp {
            pairs: vec![run((1, 10), 1, (2, 20))],
        });
        assert_eq!(
            classes.resolve_with(Dot::new(1, 10), |_| false),
            Dot::new(1, 10)
        );
    }

    #[test]
    fn multiple_visible_members_pick_max_dot() {
        let mut classes = AliasClasses::from_log(&AliasLog::new());
        classes.apply(&AliasOp {
            pairs: vec![run((1, 10), 1, (2, 20))],
        });
        classes.apply(&AliasOp {
            pairs: vec![run((1, 10), 1, (3, 30))],
        });
        let visible = |d: Dot| d.actor != 1;
        assert_eq!(
            classes.resolve_with(Dot::new(1, 10), visible),
            Dot::new(3, 30)
        );
    }

    #[test]
    fn union_is_transitive_across_generations() {
        let mut classes = AliasClasses::from_log(&AliasLog::new());
        classes.apply(&AliasOp {
            pairs: vec![run((1, 0), 1, (2, 0))],
        });
        classes.apply(&AliasOp {
            pairs: vec![run((2, 0), 1, (3, 0))],
        });
        let visible = |d: Dot| d.actor == 3;
        assert_eq!(
            classes.resolve_with(Dot::new(1, 0), visible),
            Dot::new(3, 0)
        );
    }

    #[test]
    fn duplicate_alias_is_idempotent() {
        let mut a = AliasClasses::from_log(&AliasLog::new());
        a.apply(&AliasOp {
            pairs: vec![run((1, 0), 2, (2, 0))],
        });
        let mut b = a.clone();
        b.apply(&AliasOp {
            pairs: vec![run((1, 0), 2, (2, 0))],
        });
        assert_eq!(a, b);
    }

    #[test]
    fn self_alias_does_not_create_singleton_class() {
        let mut classes = AliasClasses::from_log(&AliasLog::new());
        classes.union(Dot::new(1, 0), Dot::new(1, 0));
        assert!(
            !classes.contains(Dot::new(1, 0)),
            "self-alias는 클래스를 만들지 않음 — fast path 보존"
        );
        assert_eq!(classes, AliasClasses::from_log(&AliasLog::new()));
    }

    #[test]
    fn op_wide_domain_validation_rejects_cross_run_pollution() {
        assert!(!alias_op_is_valid(&AliasOp {
            pairs: vec![run((1, 0), 1, (1, 1)), run((1, 1), 1, (1, 2))],
        }));
        assert!(!alias_op_is_valid(&AliasOp {
            pairs: vec![run((1, 10), 2, (1, 11))],
        }));
        assert!(!alias_op_is_valid(&AliasOp {
            pairs: vec![run((1, 0), 2, (2, 0)), run((1, 0), 2, (3, 0))],
        }));
        assert!(alias_op_is_valid(&AliasOp {
            pairs: vec![run((1, 0), 2, (9, 100)), run((2, 0), 2, (9, 102))],
        }));
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn apply_order_does_not_change_resolution(
            runs in proptest::collection::vec((0u64..4, 0u64..8, 1u32..4, 0u64..4, 0u64..8), 0..12),
            seed in any::<u64>(),
        ) {
            let ops: Vec<AliasOp> = runs
                .iter()
                .map(|(oa, oc, len, na, nc)| AliasOp {
                    pairs: vec![AliasRun {
                        old_start: Dot::new(*oa, *oc),
                        len: *len,
                        new_start: Dot::new(10 + *na, *nc),
                    }],
                })
                .collect();
            let mut fwd = AliasClasses::from_log(&AliasLog::new());
            for op in &ops {
                fwd.apply(op);
            }
            let mut idx: Vec<usize> = (0..ops.len()).collect();
            idx.sort_by_key(|i| (*i as u64).wrapping_mul(seed | 1).rotate_left(17));
            let mut shuffled = AliasClasses::from_log(&AliasLog::new());
            for i in idx {
                shuffled.apply(&ops[i]);
            }
            prop_assert_eq!(&fwd, &shuffled, "정준 구조: 적용 순서 무관 구조 동등");
            let visible = |d: Dot| d.clock.is_multiple_of(2);
            for (oa, oc, len, _, _) in &runs {
                for i in 0..*len as u64 {
                    let d = Dot::new(*oa, oc + i);
                    prop_assert_eq!(fwd.resolve_with(d, visible), shuffled.resolve_with(d, visible));
                }
            }
        }
    }
}

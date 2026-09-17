use editor_crdt::Dot;
use imbl::OrdMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    len: u64,
    block: Dot,
}

/// Leaf dot → containing block, stored as maximal runs of one actor's
/// consecutive clocks inside one block. Runs are always kept maximal, so two
/// indexes holding the same mapping compare equal structurally.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LeafBlocks {
    runs: OrdMap<(u64, u64), Run>,
}

impl LeafBlocks {
    fn run_of(&self, leaf: Dot) -> Option<(u64, Run)> {
        let (&(actor, start), &run) = self.runs.get_prev(&(leaf.actor, leaf.clock))?;
        (actor == leaf.actor && leaf.clock - start < run.len).then_some((start, run))
    }

    pub(crate) fn get(&self, leaf: Dot) -> Option<Dot> {
        self.run_of(leaf).map(|(_, run)| run.block)
    }

    pub(crate) fn contains(&self, leaf: Dot) -> bool {
        self.run_of(leaf).is_some()
    }

    pub(crate) fn insert(&mut self, leaf: Dot, block: Dot) {
        match self.run_of(leaf) {
            Some((_, run)) if run.block == block => return,
            Some(_) => self.remove(leaf),
            None => {}
        }
        self.insert_run(leaf.actor, leaf.clock, 1, block);
    }

    /// `[start, start + len)` must not overlap any stored leaf.
    pub(crate) fn insert_run(&mut self, actor: u64, start: u64, len: u64, block: Dot) {
        if len == 0 {
            return;
        }
        let mut start = start;
        let mut len = len;
        if let Some((&(prev_actor, prev_start), &prev)) = self.runs.get_prev(&(actor, start))
            && prev_actor == actor
            && prev.block == block
            && prev_start + prev.len == start
        {
            self.runs.remove(&(actor, prev_start));
            start = prev_start;
            len += prev.len;
        }
        if let Some(next_start) = start.checked_add(len)
            && let Some(&next) = self.runs.get(&(actor, next_start))
            && next.block == block
        {
            self.runs.remove(&(actor, next_start));
            len += next.len;
        }
        self.runs.insert((actor, start), Run { len, block });
    }

    pub(crate) fn remove(&mut self, leaf: Dot) {
        let Some((start, run)) = self.run_of(leaf) else {
            return;
        };
        self.runs.remove(&(leaf.actor, start));
        let before = leaf.clock - start;
        if before > 0 {
            self.runs.insert(
                (leaf.actor, start),
                Run {
                    len: before,
                    block: run.block,
                },
            );
        }
        let after = run.len - before - 1;
        if after > 0 {
            self.runs.insert(
                (leaf.actor, leaf.clock + 1),
                Run {
                    len: after,
                    block: run.block,
                },
            );
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (Dot, Dot)> + '_ {
        self.runs.iter().flat_map(|(&(actor, start), &run)| {
            (0..run.len).map(move |i| (Dot::new(actor, start + i), run.block))
        })
    }
}

/// Accumulates leaves in visiting order and hands maximal runs to
/// [`LeafBlocks::insert_run`], so a bulk build costs one map insert per run.
#[derive(Default)]
pub(crate) struct LeafBlocksBuilder {
    out: LeafBlocks,
    pending: Option<(u64, u64, u64, Dot)>,
}

impl LeafBlocksBuilder {
    pub(crate) fn push(&mut self, leaf: Dot, block: Dot) {
        if let Some((actor, start, len, pending_block)) = &mut self.pending {
            if *actor == leaf.actor
                && *pending_block == block
                && start.checked_add(*len) == Some(leaf.clock)
            {
                *len += 1;
                return;
            }
            self.out.insert_run(*actor, *start, *len, *pending_block);
        }
        self.pending = Some((leaf.actor, leaf.clock, 1, block));
    }

    pub(crate) fn finish(mut self) -> LeafBlocks {
        if let Some((actor, start, len, block)) = self.pending.take() {
            self.out.insert_run(actor, start, len, block);
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;

    #[derive(Clone, Debug)]
    enum Action {
        Insert(Dot, Dot),
        Remove(Dot),
    }

    fn arb_leaf() -> impl Strategy<Value = Dot> {
        (1u64..4, 0u64..24).prop_map(|(actor, clock)| Dot::new(actor, clock))
    }

    fn arb_block() -> impl Strategy<Value = Dot> {
        (0u64..3).prop_map(|clock| Dot::new(9, clock))
    }

    fn arb_action() -> impl Strategy<Value = Action> {
        prop_oneof![
            3 => (arb_leaf(), arb_block()).prop_map(|(l, b)| Action::Insert(l, b)),
            1 => arb_leaf().prop_map(Action::Remove),
        ]
    }

    fn rebuilt(model: &BTreeMap<(u64, u64), Dot>) -> LeafBlocks {
        let mut builder = LeafBlocksBuilder::default();
        for (&(actor, clock), &block) in model {
            builder.push(Dot::new(actor, clock), block);
        }
        builder.finish()
    }

    proptest! {
        #[test]
        fn matches_a_plain_map_and_stays_canonical(actions in proptest::collection::vec(arb_action(), 0..200)) {
            let mut model: BTreeMap<(u64, u64), Dot> = BTreeMap::new();
            let mut index = LeafBlocks::default();
            for action in actions {
                match action {
                    Action::Insert(leaf, block) => {
                        model.insert((leaf.actor, leaf.clock), block);
                        index.insert(leaf, block);
                    }
                    Action::Remove(leaf) => {
                        model.remove(&(leaf.actor, leaf.clock));
                        index.remove(leaf);
                    }
                }
                for actor in 1u64..4 {
                    for clock in 0u64..24 {
                        let leaf = Dot::new(actor, clock);
                        prop_assert_eq!(index.get(leaf), model.get(&(actor, clock)).copied());
                        prop_assert_eq!(index.contains(leaf), model.contains_key(&(actor, clock)));
                    }
                }
                let listed: Vec<((u64, u64), Dot)> =
                    index.iter().map(|(l, b)| ((l.actor, l.clock), b)).collect();
                let expected: Vec<((u64, u64), Dot)> = model.iter().map(|(k, v)| (*k, *v)).collect();
                prop_assert_eq!(listed, expected);
                prop_assert_eq!(&index, &rebuilt(&model));
            }
        }

        #[test]
        fn builder_accepts_any_visiting_order(
            (sorted, shuffled) in proptest::collection::btree_map((1u64..4, 0u64..24), arb_block(), 0..60)
                .prop_flat_map(|leaves| {
                    let entries: Vec<((u64, u64), Dot)> = leaves.into_iter().collect();
                    (Just(entries.clone()), Just(entries).prop_shuffle())
                })
        ) {
            let mut builder = LeafBlocksBuilder::default();
            for ((actor, clock), block) in shuffled {
                builder.push(Dot::new(actor, clock), block);
            }
            prop_assert_eq!(builder.finish(), rebuilt(&sorted.into_iter().collect()));
        }
    }
}

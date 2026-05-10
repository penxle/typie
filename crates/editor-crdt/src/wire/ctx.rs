use crate::Dot;
use crate::wire::WireError;
use hashbrown::HashMap;

#[derive(Debug, Default)]
pub struct CollectCtx {
    actors: HashMap<u64, u64>,
    /// Insertion order kept separately so `finalize` emits the actor table deterministically;
    /// HashMap iteration order is not stable.
    order: Vec<u64>,
}

impl CollectCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, dot: &Dot) {
        match self.actors.get_mut(&dot.actor) {
            Some(min) => *min = (*min).min(dot.clock),
            None => {
                self.actors.insert(dot.actor, dot.clock);
                self.order.push(dot.actor);
            }
        }
    }

    pub fn finalize(self) -> (Vec<u64>, Vec<u64>) {
        let baselines: Vec<u64> = self.order.iter().map(|a| self.actors[a]).collect();
        (self.order, baselines)
    }
}

#[derive(Debug)]
pub struct EncCtx {
    actor_to_idx: HashMap<u64, u32>,
    baselines: Vec<u64>,
}

impl EncCtx {
    pub fn from_table(actor_table: &[u64], baselines: Vec<u64>) -> Self {
        debug_assert_eq!(actor_table.len(), baselines.len());
        let actor_to_idx = actor_table
            .iter()
            .enumerate()
            .map(|(i, &a)| (a, i as u32))
            .collect();
        Self {
            actor_to_idx,
            baselines,
        }
    }

    pub fn actor_idx(&self, actor: u64) -> u32 {
        self.actor_to_idx[&actor]
    }

    pub fn baseline(&self, actor_idx: u32) -> u64 {
        self.baselines[actor_idx as usize]
    }
}

#[derive(Debug)]
pub struct DecCtx {
    pub actor_table: Vec<u64>,
    pub baselines: Vec<u64>,
}

impl DecCtx {
    pub fn lookup(&self, actor_idx: u32, clock_delta: u64) -> Result<Dot, WireError> {
        let idx = actor_idx as usize;
        if idx >= self.actor_table.len() {
            return Err(WireError::ActorIdxOutOfRange {
                idx: actor_idx as u64,
                table_len: self.actor_table.len(),
            });
        }
        let baseline = self.baselines[idx];
        let clock = baseline
            .checked_add(clock_delta)
            .ok_or(WireError::ClockOverflow {
                context: "Dot decode (baseline + delta)",
                base: baseline,
                delta: clock_delta,
            })?;
        Ok(Dot::new(self.actor_table[idx], clock))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_records_min_clock_per_actor() {
        let mut ctx = CollectCtx::new();
        ctx.observe(&Dot::new(7, 10));
        ctx.observe(&Dot::new(7, 3));
        ctx.observe(&Dot::new(7, 20));
        ctx.observe(&Dot::new(99, 5));
        let (table, baselines) = ctx.finalize();
        assert_eq!(table, vec![7, 99]);
        assert_eq!(baselines, vec![3, 5]);
    }

    #[test]
    fn enc_ctx_resolves_actor_idx_and_baseline() {
        let ec = EncCtx::from_table(&[7, 99], vec![3, 5]);
        assert_eq!(ec.actor_idx(7), 0);
        assert_eq!(ec.actor_idx(99), 1);
        assert_eq!(ec.baseline(0), 3);
        assert_eq!(ec.baseline(1), 5);
    }

    #[test]
    fn dec_ctx_lookup_within_range() {
        let dc = DecCtx {
            actor_table: vec![7, 99],
            baselines: vec![3, 5],
        };
        assert_eq!(dc.lookup(0, 17).unwrap(), Dot::new(7, 20));
        assert_eq!(dc.lookup(1, 0).unwrap(), Dot::new(99, 5));
    }

    #[test]
    fn dec_ctx_lookup_out_of_range_errors() {
        let dc = DecCtx {
            actor_table: vec![7],
            baselines: vec![3],
        };
        let err = dc.lookup(5, 0).unwrap_err();
        assert!(matches!(
            err,
            WireError::ActorIdxOutOfRange {
                idx: 5,
                table_len: 1
            }
        ));
    }
}

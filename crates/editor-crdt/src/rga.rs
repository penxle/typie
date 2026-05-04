use serde::{Deserialize, Serialize};

use crate::{CrdtError, Dot};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RgaOp<T> {
    Insert { after: Option<Dot>, value: T },
    Remove { observed: Dot },
}

/// **Standalone-POC representation — do not embed in an editor as-is.**
/// Without a child-index, `iter()` / `len()` are O(n²) over the document size.
/// Editor integration must replace this with a child-index or a cached projection
/// before exposing `Rga<T>` (or any wrapper of it) to user-facing operations on
/// large documents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rga<T> {
    entries: imbl::HashMap<Dot, Entry<T>>,
    pending_tombstones: imbl::HashSet<Dot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry<T> {
    value: T,
    after: Option<Dot>,
    alive: bool,
}

impl<T> Rga<T> {
    pub fn new() -> Self {
        Self {
            entries: imbl::HashMap::new(),
            pending_tombstones: imbl::HashSet::new(),
        }
    }

    /// Count of reachable + alive entries — guaranteed to equal
    /// `iter().count()` at any moment, including transient out-of-order
    /// states. Orphan entries (anchor not yet arrived) are not counted.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterative pre-order DFS over reachable+alive entries.
    /// Stack-based so recursion depth is irrelevant — no stack overflow on deep chains.
    /// Children are sorted asc and pushed; popping then yields desc Dot order.
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        VisibleIter::new(self)
    }

    /// Asc-sorted `Vec<Dot>` of `parent`'s children — popping yields desc order.
    fn children_asc(&self, parent: Option<Dot>) -> Vec<Dot> {
        let mut ids: Vec<Dot> = self
            .entries
            .iter()
            .filter(|(_, e)| e.after == parent)
            .map(|(id, _)| *id)
            .collect();
        ids.sort();
        ids
    }
}

impl<T> Default for Rga<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Eq> Rga<T> {
    /// Returns `Err` if the same `Dot` arrives with a different payload.
    ///
    /// Dot uniqueness with deterministic payload is a precondition the op-generation
    /// layer must guarantee. Wire integrations should reject malformed ops before
    /// reaching this API; this `Err` is a defense-in-depth signal that wire validation
    /// failed.
    ///
    /// `T: Eq` (not `PartialEq`) — non-reflexive payloads such as `f32::NAN` would
    /// make the same-Dot replay path (`existing.value != value`) return `true` for
    /// an exact-equal op, yielding a spurious `DotConflict` and breaking
    /// at-least-once delivery idempotency.
    pub fn apply(&self, id: Dot, op: RgaOp<T>) -> Result<Self, CrdtError> {
        match op {
            RgaOp::Insert { after, value } => self.apply_insert(id, after, value),
            RgaOp::Remove { observed } => Ok(self.apply_remove(observed)),
        }
    }

    fn apply_insert(&self, id: Dot, after: Option<Dot>, value: T) -> Result<Self, CrdtError> {
        if let Some(existing) = self.entries.get(&id) {
            if existing.value != value || existing.after != after {
                return Err(CrdtError::DotConflict { dot: id });
            }
            return Ok(self.clone());
        }
        let alive = !self.pending_tombstones.contains(&id);
        let entry = Entry {
            value,
            after,
            alive,
        };
        Ok(Self {
            entries: self.entries.update(id, entry),
            pending_tombstones: self.pending_tombstones.without(&id),
        })
    }

    fn apply_remove(&self, observed: Dot) -> Self {
        if let Some(entry) = self.entries.get(&observed) {
            if !entry.alive {
                return self.clone();
            }
            let new_entry = Entry {
                value: entry.value.clone(),
                after: entry.after,
                alive: false,
            };
            return Self {
                entries: self.entries.update(observed, new_entry),
                pending_tombstones: self.pending_tombstones.clone(),
            };
        }
        Self {
            entries: self.entries.clone(),
            pending_tombstones: self.pending_tombstones.update(observed),
        }
    }
}

struct VisibleIter<'a, T> {
    crdt: &'a Rga<T>,
    stack: Vec<Dot>,
}

impl<'a, T> VisibleIter<'a, T> {
    fn new(crdt: &'a Rga<T>) -> Self {
        Self {
            crdt,
            stack: crdt.children_asc(None),
        }
    }
}

impl<'a, T> Iterator for VisibleIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        while let Some(id) = self.stack.pop() {
            // Tombstones still act as anchors, so traverse regardless of alive.
            let children = self.crdt.children_asc(Some(id));
            self.stack.extend(children);

            let entry = self
                .crdt
                .entries
                .get(&id)
                .expect("popped id must be in entries");
            if entry.alive {
                return Some(&entry.value);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(crdt: &Rga<u32>) -> Vec<u32> {
        crdt.iter().copied().collect()
    }

    #[test]
    fn empty_state() {
        let crdt = Rga::<u32>::new();
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
        assert!(collect(&crdt).is_empty());
    }

    #[test]
    fn insert_single_value_at_start() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![1u32]);
        assert_eq!(crdt.len(), 1);
    }

    #[test]
    fn insert_then_remove_yields_empty() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        assert!(collect(&crdt).is_empty());
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
    }

    #[test]
    fn remove_insert_other_values_then_target_arrives() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(2, 0),
                },
            )
            .unwrap() // pending
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 0),
                RgaOp::Insert {
                    after: None,
                    value: 11u32,
                },
            )
            .unwrap();
        // children of None desc: (2,0) > (1,0) -> 11 subtree (dead, no emit) then 10 subtree.
        // Final: [10].
        assert_eq!(collect(&crdt), vec![10u32]);
    }

    #[test]
    fn permutation_remove_insert_vs_insert_remove_converges() {
        let id_i = Dot::new(1, 0);
        let id_r = Dot::new(u64::MAX, 0);
        let op_i = RgaOp::Insert {
            after: None,
            value: 1u32,
        };
        let op_r = RgaOp::<u32>::Remove {
            observed: Dot::new(1, 0),
        };
        let s1 = Rga::<u32>::new()
            .apply(id_i, op_i.clone())
            .unwrap()
            .apply(id_r, op_r.clone())
            .unwrap();
        let s2 = Rga::<u32>::new()
            .apply(id_r, op_r)
            .unwrap()
            .apply(id_i, op_i)
            .unwrap();
        assert_eq!(collect(&s1), collect(&s2));
    }

    #[test]
    fn remove_twice_pending() {
        let s1 = Rga::<u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        let s2 = s1
            .apply(
                Dot::new(u64::MAX, 1),
                RgaOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        let after_insert_1 = s1
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        let after_insert_2 = s2
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&after_insert_1), collect(&after_insert_2));
        assert!(collect(&after_insert_1).is_empty());
    }

    #[test]
    fn linear_chain_three_values() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 2),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 1)),
                    value: 3u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![1u32, 2, 3]);
        assert_eq!(crdt.len(), 3);
    }

    #[test]
    fn root_siblings_clock_desc_same_actor() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: None,
                    value: 11u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 2),
                RgaOp::Insert {
                    after: None,
                    value: 12u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![12u32, 11, 10]);
    }

    #[test]
    fn root_siblings_actor_tiebreak_on_equal_clock() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 0),
                RgaOp::Insert {
                    after: None,
                    value: 11u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                RgaOp::Insert {
                    after: None,
                    value: 12u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![12u32, 11, 10]);
    }

    #[test]
    fn clock_primary_dominates_actor() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(1, 5),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 3),
                RgaOp::Insert {
                    after: None,
                    value: 11u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(1, 7),
                RgaOp::Insert {
                    after: None,
                    value: 12u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![12u32, 10, 11]);
    }

    #[test]
    fn subtree_dfs_pre_order() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 2),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 3),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 2)),
                    value: 4u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 3u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![1u32, 2, 4, 3]);
    }

    #[test]
    fn tombstone_anchor_with_multiple_alive_children() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 2),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 3u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(0, 0),
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![3u32, 2]);
    }

    #[test]
    fn tombstone_mid_chain_preserves_descendants() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 2),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 1)),
                    value: 3u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(0, 1),
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![1u32, 3]);
    }

    #[test]
    fn out_of_order_insert_eventually_renders() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![1u32, 2]);
        assert_eq!(crdt.len(), 2);
    }

    #[test]
    fn orphan_entry_invisible_in_len_and_iter() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap();
        assert!(collect(&crdt).is_empty());
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
    }

    #[test]
    fn pending_tombstone_then_late_insert() {
        let s = Rga::<u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(0, 0),
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        assert!(collect(&s).is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn applying_same_insert_n_times() {
        let id = Dot::new(0, 0);
        let op = RgaOp::Insert {
            after: None,
            value: 1u32,
        };
        let s = Rga::<u32>::new()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op)
            .unwrap();
        assert_eq!(collect(&s), vec![1u32]);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn deep_chain_no_stack_overflow() {
        let chain_len = 1_000u64;
        let mut crdt = Rga::<u32>::new();
        for i in 0..chain_len {
            let after = if i == 0 {
                None
            } else {
                Some(Dot::new(0, i - 1))
            };
            crdt = crdt
                .apply(Dot::new(0, i), RgaOp::Insert { after, value: 1u32 })
                .unwrap();
        }
        assert_eq!(crdt.len() as u64, chain_len);
        assert_eq!(crdt.iter().count() as u64, chain_len);
    }

    #[test]
    fn complex_multi_actor_tree() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(1, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(1, 0)),
                    value: 2u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(1, 2),
                RgaOp::Insert {
                    after: Some(Dot::new(1, 1)),
                    value: 3u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(1, 3),
                RgaOp::Insert {
                    after: Some(Dot::new(1, 0)),
                    value: 4u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 0),
                RgaOp::Insert {
                    after: None,
                    value: 5u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(2, 0)),
                    value: 6u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![5u32, 6, 1, 4, 2, 3]);
    }

    #[test]
    fn duplicate_dot_different_value_returns_err() {
        let s = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap();
        let result = s.apply(
            Dot::new(0, 0),
            RgaOp::Insert {
                after: None,
                value: 2u32,
            },
        );
        assert_eq!(
            result,
            Err(CrdtError::DotConflict {
                dot: Dot::new(0, 0)
            })
        );
    }

    #[test]
    fn duplicate_dot_different_after_returns_err() {
        let s = Rga::<u32>::new()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 1u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 2u32,
                },
            )
            .unwrap();
        let result = s.apply(
            Dot::new(0, 1),
            RgaOp::Insert {
                after: None,
                value: 2u32,
            },
        );
        assert_eq!(
            result,
            Err(CrdtError::DotConflict {
                dot: Dot::new(0, 1)
            })
        );
    }

    #[test]
    fn pending_tombstoned_anchor_with_live_descendant() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(0, 0),
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 11u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![11u32]);
        assert_eq!(crdt.len(), 1);
    }

    #[test]
    fn pending_tombstoned_anchor_descendant_first_arrival() {
        let crdt = Rga::<u32>::new()
            .apply(
                Dot::new(0, 1),
                RgaOp::Insert {
                    after: Some(Dot::new(0, 0)),
                    value: 11u32,
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                RgaOp::Remove {
                    observed: Dot::new(0, 0),
                },
            )
            .unwrap()
            .apply(
                Dot::new(0, 0),
                RgaOp::Insert {
                    after: None,
                    value: 10u32,
                },
            )
            .unwrap();
        assert_eq!(collect(&crdt), vec![11u32]);
        assert_eq!(crdt.len(), 1);
    }

    // Catches accidental T: Copy bounds in apply / iter / Default by exercising
    // a non-Copy value type that satisfies the operation-level bound (Clone + Eq).
    #[test]
    fn smoke_non_copy_value_type() {
        let crdt: Rga<String> = Rga::new();
        assert_eq!(crdt.len(), 0);

        let crdt = crdt
            .apply(
                Dot::new(1, 0),
                RgaOp::Insert {
                    after: None,
                    value: "hello".to_string(),
                },
            )
            .unwrap();
        assert_eq!(crdt.len(), 1);
        assert_eq!(
            crdt.iter().cloned().collect::<Vec<String>>(),
            vec!["hello".to_string()]
        );

        let _: Rga<String> = Rga::default();
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::test_utils::permute;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a sequence of *causally-valid* ops:
    /// each Insert's `after` references a Dot already produced earlier in this sequence.
    /// `apply` handles out-of-order delivery, but the generator models a realistic editor
    /// session; out-of-order arrival is exercised separately by permuting the result.
    pub(super) fn arb_op_sequence(
        max_ops: usize,
        num_actors: u64,
    ) -> impl Strategy<Value = Vec<(Dot, RgaOp<u32>)>> {
        proptest::collection::vec(
            (0u64..num_actors, any::<bool>(), any::<u8>(), any::<u32>()),
            0..=max_ops,
        )
        .prop_map(build_ops)
    }

    fn build_ops(raw: Vec<(u64, bool, u8, u32)>) -> Vec<(Dot, RgaOp<u32>)> {
        let mut clocks: HashMap<u64, u64> = HashMap::new();
        let mut existing: Vec<Dot> = Vec::new();
        let mut ops: Vec<(Dot, RgaOp<u32>)> = Vec::new();
        let mut remove_counter: u64 = 0;

        // No drop path: even when `want_remove=true`, `existing.is_empty()` falls through
        // to do_remove=false and the insert path. Every raw entry produces exactly 1 op.
        for (actor, want_remove, target_byte, value) in raw {
            let do_remove = want_remove && !existing.is_empty();
            if do_remove {
                let observed = existing[(target_byte as usize) % existing.len()];
                let id = Dot::new(u64::MAX, remove_counter);
                remove_counter += 1;
                ops.push((id, RgaOp::Remove { observed }));
                continue;
            }
            let clock = clocks.entry(actor).or_insert(0);
            let id = Dot::new(actor, *clock);
            *clock += 1;
            let after = if existing.is_empty() {
                None
            } else {
                Some(existing[(target_byte as usize) % existing.len()])
            };
            ops.push((id, RgaOp::Insert { after, value }));
            existing.push(id);
        }
        ops
    }

    pub(super) fn apply_all(ops: &[(Dot, RgaOp<u32>)]) -> Rga<u32> {
        ops.iter()
            .cloned()
            .fold(Rga::<u32>::new(), |s, (id, op)| s.apply(id, op).unwrap())
    }

    /// Independent reference using a different code path (std HashMap, mutable, recursive
    /// DFS) than Rga (imbl HashMap, functional, iterative). Same RGA semantic, but
    /// disagreement on any input would indicate a bug in one path.
    fn reference_render(ops: &[(Dot, RgaOp<u32>)]) -> Vec<u32> {
        use std::collections::{HashMap, HashSet};

        let mut entries: HashMap<Dot, (u32, Option<Dot>, bool)> = HashMap::new();
        let mut pending: HashSet<Dot> = HashSet::new();
        for (id, op) in ops {
            match op {
                RgaOp::Insert { after, value } => {
                    if entries.contains_key(id) {
                        continue;
                    }
                    let alive = !pending.contains(id);
                    entries.insert(*id, (*value, *after, alive));
                    pending.remove(id);
                }
                RgaOp::Remove { observed } => {
                    if let Some(e) = entries.get_mut(observed) {
                        e.2 = false;
                    } else {
                        pending.insert(*observed);
                    }
                }
            }
        }
        let mut out = Vec::new();
        render_recursive(&entries, None, &mut out);
        out
    }

    fn render_recursive(
        entries: &std::collections::HashMap<Dot, (u32, Option<Dot>, bool)>,
        parent: Option<Dot>,
        out: &mut Vec<u32>,
    ) {
        let mut children: Vec<Dot> = entries
            .iter()
            .filter(|(_, e)| e.1 == parent)
            .map(|(id, _)| *id)
            .collect();
        children.sort_by(|a, b| b.cmp(a)); // desc

        for id in children {
            let (value, _, alive) = entries[&id];
            if alive {
                out.push(value);
            }
            render_recursive(entries, Some(id), out);
        }
    }

    #[test]
    fn build_ops_smoke() {
        let ops = build_ops(vec![
            (0, false, 0, 1u32),
            (0, false, 0, 2u32),
            (1, false, 0, 3u32),
        ]);
        assert_eq!(ops.len(), 3);
        let s = apply_all(&ops);
        // Every op has a valid `after` — graph stays connected.
        assert_eq!(s.len(), 3);
    }

    proptest! {
        /// Applying a *seed-derived permutation* of an arbitrary op set yields the
        /// same final state as the original. Not an exhaustive proof over all
        /// permutations — proptest's randomized sampling + shrink provides reasonable
        /// statistical evidence.
        /// Compares *full state equality* (entries + pending_tombstones), not just
        /// rendered output: comparing rendered values alone misses hidden state divergence
        /// that would surface only after subsequent ops are applied.
        #[test]
        fn convergence_under_permutation(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let s1 = apply_all(&ops);
            let s2 = apply_all(&permuted);
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// Applying a *seed-derived permutation* of a multiset that contains each op
        /// twice yields the same result as applying each op once. Models real
        /// at-least-once delivery where duplicates are interleaved with other ops
        /// rather than adjacent. Randomized sampling, not exhaustive over permutations.
        #[test]
        fn idempotency_under_permutation(
            ops in arb_op_sequence(20, 2),
            seed in any::<u64>(),
        ) {
            let single = apply_all(&ops);
            let doubled: Vec<(Dot, RgaOp<u32>)> = ops.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            let permuted_doubled = permute(&doubled, seed);
            let s2 = apply_all(&permuted_doubled);
            prop_assert_eq!(single, s2);
        }
    }

    proptest! {
        /// Multiple actors grow their own subtrees on the same non-root anchor;
        /// applying a seed-derived permutation must yield the same final state as
        /// the original (randomized sampling).
        #[test]
        fn concurrent_subtrees_at_non_root_anchor(
            num_actors in 2u64..6,
            depth in 1u64..5,
            seed in any::<u64>(),
        ) {
            let mut ops: Vec<(Dot, RgaOp<u32>)> = Vec::new();

            for i in 0..3u64 {
                let after = if i == 0 { None } else { Some(Dot::new(0, i - 1)) };
                ops.push((
                    Dot::new(0, i),
                    RgaOp::Insert {
                        after,
                        value: i as u32 + 1,
                    },
                ));
            }

            // Per-actor distinct value: a uniform constant would hurt
            // discriminability (state equality still catches divergence, but reduced
            // example readability suffers during debugging).
            let anchor = Dot::new(0, 1);
            for actor in 1..=num_actors {
                let mut prev = anchor;
                let actor_value = actor as u32 + 100;
                for d in 0..depth {
                    let id = Dot::new(actor, d);
                    ops.push((id, RgaOp::Insert { after: Some(prev), value: actor_value }));
                    prev = id;
                }
            }

            let s1 = apply_all(&ops);
            let s2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// Complex tombstone scenario:
        /// (1) tombstone several values in the chain,
        /// (2) add multiple children at each chain position (tombstoned anchors included),
        /// (3) seed-derived permutation triggers remove-before-insert at both anchor
        ///     and descendant.
        /// Final state must converge (randomized sampling).
        #[test]
        fn complex_tombstone(
            chain_len in 3usize..7,
            // At least 2 removes for the 'multiple tombstones' invariant; duplicates
            // are allowed (also exercises idempotency).
            remove_indices in proptest::collection::vec(0usize..7, 2..5),
            // At least 2 children per anchor for the 'multiple children under
            // tombstone' invariant.
            extras_per_anchor in 2u64..4,
            seed in any::<u64>(),
        ) {
            let mut ops: Vec<(Dot, RgaOp<u32>)> = Vec::new();
            let mut remove_counter: u64 = 0;

            for i in 0..chain_len as u64 {
                let after = if i == 0 { None } else { Some(Dot::new(0, i - 1)) };
                ops.push((
                    Dot::new(0, i),
                    RgaOp::Insert {
                        after,
                        value: (i as u32) + 1,
                    },
                ));
            }

            // Duplicate targets allowed — also exercises idempotency.
            for &idx in &remove_indices {
                let observed = Dot::new(0, (idx % chain_len) as u64);
                ops.push((Dot::new(u64::MAX, remove_counter), RgaOp::Remove { observed }));
                remove_counter += 1;
            }

            let mut child_clock: u64 = 0;
            for chain_idx in 0..chain_len as u64 {
                for _ in 0..extras_per_anchor {
                    ops.push((
                        Dot::new(1, child_clock),
                        RgaOp::Insert {
                            after: Some(Dot::new(0, chain_idx)),
                            value: 99u32,
                        },
                    ));
                    child_clock += 1;
                }
            }

            let s1 = apply_all(&ops);
            let s2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// After every op apply, `len() == iter().count()` must hold.
        /// Permutation forces transient out-of-order delivery scenarios.
        #[test]
        fn len_visible_consistency(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let mut state = Rga::<u32>::new();
            for (id, op) in &permuted {
                state = state.apply(*id, op.clone()).unwrap();
                prop_assert_eq!(
                    state.len(),
                    state.iter().count(),
                    "len/visible-values consistency broken after applying ({:?}, {:?})", id, op
                );
            }
        }
    }

    proptest! {
        /// Rga output matches an independent reference for both the original and a
        /// permuted op stream. Detects deterministic-but-wrong renderings that pure
        /// state-equality properties miss.
        #[test]
        fn render_matches_independent_reference(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            prop_assert_eq!(
                apply_all(&ops).iter().copied().collect::<Vec<u32>>(),
                reference_render(&ops),
            );
            prop_assert_eq!(
                apply_all(&permuted).iter().copied().collect::<Vec<u32>>(),
                reference_render(&permuted),
            );
        }
    }
}

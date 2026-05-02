use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{CrdtError, Dot};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextOp {
    InsertChar {
        id: Dot,
        after: Option<Dot>,
        ch: char,
    },
    RemoveChar {
        target: Dot,
    },
}

/// **Standalone-POC representation — do not embed in an editor as-is.**
/// Without a child-index, `to_string()` / `len()` are O(n²) over the document size.
/// Editor integration must replace this with a child-index or a cached projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextCrdt {
    entries: imbl::HashMap<Dot, Entry>,
    pending_tombstones: imbl::HashSet<Dot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    ch: char,
    after: Option<Dot>,
    alive: bool,
}

impl TextCrdt {
    pub fn new() -> Self {
        Self {
            entries: imbl::HashMap::new(),
            pending_tombstones: imbl::HashSet::new(),
        }
    }

    /// Count of reachable + alive entries — guaranteed to equal
    /// `to_string().chars().count()` at any moment, including transient out-of-order
    /// states. Orphan entries (anchor not yet arrived) are not counted.
    pub fn len(&self) -> usize {
        self.visible_chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterative pre-order DFS over reachable+alive entries.
    /// Stack-based so recursion depth is irrelevant — no stack overflow on deep chains.
    /// Children are sorted asc and pushed; popping then yields desc Dot order.
    fn visible_chars(&self) -> impl Iterator<Item = char> + '_ {
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

    /// Returns `Err` if the same `Dot` arrives with a different payload.
    ///
    /// Dot uniqueness with deterministic payload is a precondition the op-generation
    /// layer must guarantee. Wire integrations should reject malformed ops before
    /// reaching this API; this `Err` is a defense-in-depth signal that wire validation
    /// failed.
    pub fn apply(&self, op: TextOp) -> Result<Self, CrdtError> {
        match op {
            TextOp::InsertChar { id, after, ch } => self.apply_insert(id, after, ch),
            TextOp::RemoveChar { target } => Ok(self.apply_remove(target)),
        }
    }

    fn apply_insert(&self, id: Dot, after: Option<Dot>, ch: char) -> Result<Self, CrdtError> {
        if let Some(existing) = self.entries.get(&id) {
            // Two ops with the same Dot must have identical payloads — a violation
            // is a bug in the op generation layer (actor id collision, clock reuse,
            // etc.). Silent first-wins would diverge replicas.
            if existing.ch != ch || existing.after != after {
                return Err(CrdtError::DotPayloadConflict { dot: id });
            }
            return Ok(self.clone());
        }
        let alive = !self.pending_tombstones.contains(&id);
        let entry = Entry { ch, after, alive };
        Ok(Self {
            entries: self.entries.update(id, entry),
            pending_tombstones: self.pending_tombstones.without(&id),
        })
    }

    fn apply_remove(&self, target: Dot) -> Self {
        if let Some(entry) = self.entries.get(&target) {
            if !entry.alive {
                return self.clone();
            }
            let new_entry = Entry {
                ch: entry.ch,
                after: entry.after,
                alive: false,
            };
            return Self {
                entries: self.entries.update(target, new_entry),
                pending_tombstones: self.pending_tombstones.clone(),
            };
        }
        Self {
            entries: self.entries.clone(),
            pending_tombstones: self.pending_tombstones.update(target),
        }
    }
}

struct VisibleIter<'a> {
    crdt: &'a TextCrdt,
    // asc-sorted children — popping traverses in desc order
    stack: Vec<Dot>,
}

impl<'a> VisibleIter<'a> {
    fn new(crdt: &'a TextCrdt) -> Self {
        Self {
            crdt,
            stack: crdt.children_asc(None),
        }
    }
}

impl<'a> Iterator for VisibleIter<'a> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
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
                return Some(entry.ch);
            }
        }
        None
    }
}

impl Default for TextCrdt {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TextCrdt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for ch in self.visible_chars() {
            write!(f, "{ch}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let crdt = TextCrdt::new();
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
        assert_eq!(crdt.to_string(), "");
    }

    #[test]
    fn insert_single_char_at_start() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "a");
        assert_eq!(crdt.len(), 1);
    }

    #[test]
    fn insert_then_remove_yields_empty_string() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::RemoveChar {
                target: Dot::new(1, 0),
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "");
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
    }

    #[test]
    fn remove_insert_other_chars_then_target_arrives() {
        let crdt = TextCrdt::new()
            .apply(TextOp::RemoveChar {
                target: Dot::new(2, 0),
            })
            .unwrap() // pending
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'A',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(2, 0),
                after: None,
                ch: 'B',
            })
            .unwrap();
        // children of None desc: (2,0) > (1,0) -> 'B' subtree (dead, no emit) then 'A' subtree.
        // Final: "A".
        assert_eq!(crdt.to_string(), "A");
    }

    #[test]
    fn permutation_remove_insert_vs_insert_remove_converges() {
        let op_i = TextOp::InsertChar {
            id: Dot::new(1, 0),
            after: None,
            ch: 'x',
        };
        let op_r = TextOp::RemoveChar {
            target: Dot::new(1, 0),
        };
        let s1 = TextCrdt::new()
            .apply(op_i.clone())
            .unwrap()
            .apply(op_r.clone())
            .unwrap();
        let s2 = TextCrdt::new().apply(op_r).unwrap().apply(op_i).unwrap();
        assert_eq!(s1.to_string(), s2.to_string());
    }

    #[test]
    fn remove_twice_pending() {
        let s1 = TextCrdt::new()
            .apply(TextOp::RemoveChar {
                target: Dot::new(1, 0),
            })
            .unwrap();
        let s2 = s1
            .apply(TextOp::RemoveChar {
                target: Dot::new(1, 0),
            })
            .unwrap();
        let after_insert_1 = s1
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        let after_insert_2 = s2
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        assert_eq!(after_insert_1.to_string(), after_insert_2.to_string());
        assert_eq!(after_insert_1.to_string(), "");
    }

    #[test]
    fn linear_chain_three_chars() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 2),
                after: Some(Dot::new(0, 1)),
                ch: 'c',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "abc");
        assert_eq!(crdt.len(), 3);
    }

    #[test]
    fn root_siblings_clock_desc_same_actor() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'A',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: None,
                ch: 'B',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 2),
                after: None,
                ch: 'C',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "CBA");
    }

    #[test]
    fn root_siblings_actor_tiebreak_on_equal_clock() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'A',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(2, 0),
                after: None,
                ch: 'B',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(3, 0),
                after: None,
                ch: 'C',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "CBA");
    }

    #[test]
    fn clock_primary_dominates_actor() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 5),
                after: None,
                ch: 'A',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(2, 3),
                after: None,
                ch: 'B',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 7),
                after: None,
                ch: 'C',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "CAB");
    }

    #[test]
    fn subtree_dfs_pre_order() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 2),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 3),
                after: Some(Dot::new(0, 2)),
                ch: 'd',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'c',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "abdc");
    }

    #[test]
    fn tombstone_anchor_with_multiple_alive_children() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 2),
                after: Some(Dot::new(0, 0)),
                ch: 'c',
            })
            .unwrap()
            .apply(TextOp::RemoveChar {
                target: Dot::new(0, 0),
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "cb");
    }

    #[test]
    fn tombstone_mid_chain_preserves_descendants() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 2),
                after: Some(Dot::new(0, 1)),
                ch: 'c',
            })
            .unwrap()
            .apply(TextOp::RemoveChar {
                target: Dot::new(0, 1),
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "ac");
    }

    #[test]
    fn out_of_order_insert_eventually_renders() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "ab");
        assert_eq!(crdt.len(), 2);
    }

    #[test]
    fn orphan_entry_invisible_in_len_and_to_string() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "");
        assert_eq!(crdt.len(), 0);
        assert!(crdt.is_empty());
    }

    #[test]
    fn pending_tombstone_then_late_insert() {
        let s = TextCrdt::new()
            .apply(TextOp::RemoveChar {
                target: Dot::new(0, 0),
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        assert_eq!(s.to_string(), "");
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn applying_same_insert_n_times() {
        let op = TextOp::InsertChar {
            id: Dot::new(0, 0),
            after: None,
            ch: 'a',
        };
        let s = TextCrdt::new()
            .apply(op.clone())
            .unwrap()
            .apply(op.clone())
            .unwrap()
            .apply(op.clone())
            .unwrap()
            .apply(op)
            .unwrap();
        assert_eq!(s.to_string(), "a");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn deep_chain_no_stack_overflow() {
        let chain_len = 1_000u64;
        let mut crdt = TextCrdt::new();
        for i in 0..chain_len {
            let after = if i == 0 {
                None
            } else {
                Some(Dot::new(0, i - 1))
            };
            crdt = crdt
                .apply(TextOp::InsertChar {
                    id: Dot::new(0, i),
                    after,
                    ch: 'a',
                })
                .unwrap();
        }
        assert_eq!(crdt.len() as u64, chain_len);
        assert_eq!(crdt.to_string().chars().count() as u64, chain_len);
    }

    #[test]
    fn complex_multi_actor_tree() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 1),
                after: Some(Dot::new(1, 0)),
                ch: 'b',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 2),
                after: Some(Dot::new(1, 1)),
                ch: 'c',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(1, 3),
                after: Some(Dot::new(1, 0)),
                ch: 'd',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(2, 0),
                after: None,
                ch: 'e',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(2, 1),
                after: Some(Dot::new(2, 0)),
                ch: 'f',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "efadbc");
    }

    #[test]
    fn duplicate_dot_different_ch_returns_err() {
        let s = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap();
        let result = s.apply(TextOp::InsertChar {
            id: Dot::new(0, 0),
            after: None,
            ch: 'b',
        });
        assert_eq!(
            result,
            Err(CrdtError::DotPayloadConflict {
                dot: Dot::new(0, 0)
            })
        );
    }

    #[test]
    fn duplicate_dot_different_after_returns_err() {
        let s = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'a',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'b',
            })
            .unwrap();
        let result = s.apply(TextOp::InsertChar {
            id: Dot::new(0, 1),
            after: None,
            ch: 'b',
        });
        assert_eq!(
            result,
            Err(CrdtError::DotPayloadConflict {
                dot: Dot::new(0, 1)
            })
        );
    }

    #[test]
    fn pending_tombstoned_anchor_with_live_descendant() {
        let crdt = TextCrdt::new()
            .apply(TextOp::RemoveChar {
                target: Dot::new(0, 0),
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'A',
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'B',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "B");
        assert_eq!(crdt.len(), 1);
    }

    #[test]
    fn pending_tombstoned_anchor_descendant_first_arrival() {
        let crdt = TextCrdt::new()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 1),
                after: Some(Dot::new(0, 0)),
                ch: 'B',
            })
            .unwrap()
            .apply(TextOp::RemoveChar {
                target: Dot::new(0, 0),
            })
            .unwrap()
            .apply(TextOp::InsertChar {
                id: Dot::new(0, 0),
                after: None,
                ch: 'A',
            })
            .unwrap();
        assert_eq!(crdt.to_string(), "B");
        assert_eq!(crdt.len(), 1);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::test_utils::permute;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a sequence of *causally-valid* ops:
    /// each InsertChar's `after` references a Dot already produced earlier in this sequence.
    /// `apply` handles out-of-order delivery, but the generator models a realistic editor
    /// session; out-of-order arrival is exercised separately by permuting the result.
    pub(super) fn arb_op_sequence(
        max_ops: usize,
        num_actors: u64,
    ) -> impl Strategy<Value = Vec<TextOp>> {
        proptest::collection::vec(
            (0u64..num_actors, any::<bool>(), any::<u8>(), any::<char>()),
            0..=max_ops,
        )
        .prop_map(build_ops)
    }

    fn build_ops(raw: Vec<(u64, bool, u8, char)>) -> Vec<TextOp> {
        let mut clocks: HashMap<u64, u64> = HashMap::new();
        let mut existing: Vec<Dot> = Vec::new();
        let mut ops: Vec<TextOp> = Vec::new();

        // No ASCII filter: the full Unicode char is used as-is. The RGA algorithm depends
        // only on char identity, not on *which* char. An earlier version had an ASCII
        // filter, but `any::<char>()` samples from the full Unicode range, so only
        // ~0.009% passed the filter — most ops were dropped, leaving the proptest
        // properties below vacuous.
        //
        // No drop path: even when `want_remove=true`, `existing.is_empty()` falls through
        // to do_remove=false and the insert path. Every raw entry produces exactly 1 op.
        for (actor, want_remove, target_byte, ch) in raw {
            let do_remove = want_remove && !existing.is_empty();
            if do_remove {
                let target = existing[(target_byte as usize) % existing.len()];
                ops.push(TextOp::RemoveChar { target });
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
            ops.push(TextOp::InsertChar { id, after, ch });
            existing.push(id);
        }
        ops
    }

    pub(super) fn apply_all(ops: &[TextOp]) -> TextCrdt {
        ops.iter()
            .cloned()
            .fold(TextCrdt::new(), |s, op| s.apply(op).unwrap())
    }

    /// Independent reference using a different code path (std HashMap, mutable, recursive
    /// DFS) than TextCrdt (imbl HashMap, functional, iterative). Same RGA semantic, but
    /// disagreement on any input would indicate a bug in one path.
    fn reference_render(ops: &[TextOp]) -> String {
        use std::collections::{HashMap, HashSet};

        let mut entries: HashMap<Dot, (char, Option<Dot>, bool)> = HashMap::new();
        let mut pending: HashSet<Dot> = HashSet::new();
        for op in ops {
            match op {
                TextOp::InsertChar { id, after, ch } => {
                    if entries.contains_key(id) {
                        continue;
                    }
                    let alive = !pending.contains(id);
                    entries.insert(*id, (*ch, *after, alive));
                    pending.remove(id);
                }
                TextOp::RemoveChar { target } => {
                    if let Some(e) = entries.get_mut(target) {
                        e.2 = false;
                    } else {
                        pending.insert(*target);
                    }
                }
            }
        }
        let mut out = String::new();
        render_recursive(&entries, None, &mut out);
        out
    }

    fn render_recursive(
        entries: &std::collections::HashMap<Dot, (char, Option<Dot>, bool)>,
        parent: Option<Dot>,
        out: &mut String,
    ) {
        let mut children: Vec<Dot> = entries
            .iter()
            .filter(|(_, e)| e.1 == parent)
            .map(|(id, _)| *id)
            .collect();
        children.sort_by(|a, b| b.cmp(a)); // desc

        for id in children {
            let (ch, _, alive) = entries[&id];
            if alive {
                out.push(ch);
            }
            render_recursive(entries, Some(id), out);
        }
    }

    #[test]
    fn build_ops_smoke() {
        let ops = build_ops(vec![
            (0, false, 0, 'a'),
            (0, false, 0, 'b'),
            (1, false, 0, 'c'),
        ]);
        assert_eq!(ops.len(), 3);
        let s = apply_all(&ops);
        // Every op has a valid `after` — graph stays connected.
        assert_eq!(s.len(), 3);
    }

    /// Verifies the generator produces meaningful sequences. The only drop path is
    /// `want_remove=true && existing.is_empty()` (a few early ops). If an ASCII filter
    /// were re-introduced and dropped most ops, this test fails early — catching the
    /// case where the proptest properties below silently become vacuous.
    #[test]
    fn build_ops_with_unicode_chars_preserved() {
        let raw: Vec<(u64, bool, u8, char)> = vec![
            (0, false, 0, '한'),
            (0, false, 0, '글'),
            (1, false, 0, '🦀'),
            (2, false, 0, '\u{0}'),      // null char
            (0, false, 0, '\u{10FFFF}'), // max valid Unicode scalar
        ];
        let ops = build_ops(raw);
        assert_eq!(ops.len(), 5, "full Unicode char range must pass through");
        let s = apply_all(&ops);
        assert_eq!(s.len(), 5);
    }

    proptest! {
        /// Applying a *seed-derived permutation* of an arbitrary op set yields the
        /// same final state as the original. Not an exhaustive proof over all
        /// permutations — proptest's randomized sampling + shrink provides reasonable
        /// statistical evidence.
        /// Compares *full state equality* (entries + pending_tombstones), not just
        /// `to_string`: comparing rendered strings alone misses hidden state divergence
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
            let doubled: Vec<TextOp> = ops.iter().flat_map(|op| [op.clone(), op.clone()]).collect();
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
            let mut ops: Vec<TextOp> = Vec::new();

            for i in 0..3u64 {
                let after = if i == 0 { None } else { Some(Dot::new(0, i - 1)) };
                ops.push(TextOp::InsertChar {
                    id: Dot::new(0, i),
                    after,
                    ch: (b'a' + i as u8) as char,
                });
            }

            // Per-actor distinct char: a uniform 'x' would hurt string-level
            // discriminability (state equality still catches divergence, but reduced
            // clock-example readability suffers during debugging).
            let anchor = Dot::new(0, 1);
            for actor in 1..=num_actors {
                let mut prev = anchor;
                let actor_ch = (b'A' + (actor as u8 - 1)) as char;
                for d in 0..depth {
                    let id = Dot::new(actor, d);
                    ops.push(TextOp::InsertChar { id, after: Some(prev), ch: actor_ch });
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
        /// (1) tombstone several chars in the chain,
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
            let mut ops: Vec<TextOp> = Vec::new();

            for i in 0..chain_len as u64 {
                let after = if i == 0 { None } else { Some(Dot::new(0, i - 1)) };
                ops.push(TextOp::InsertChar {
                    id: Dot::new(0, i),
                    after,
                    ch: (b'a' + (i as u8 % 26)) as char,
                });
            }

            // Duplicate targets allowed — also exercises idempotency.
            for &idx in &remove_indices {
                let target = Dot::new(0, (idx % chain_len) as u64);
                ops.push(TextOp::RemoveChar { target });
            }

            let mut child_clock: u64 = 0;
            for chain_idx in 0..chain_len as u64 {
                for _ in 0..extras_per_anchor {
                    ops.push(TextOp::InsertChar {
                        id: Dot::new(1, child_clock),
                        after: Some(Dot::new(0, chain_idx)),
                        ch: 'X',
                    });
                    child_clock += 1;
                }
            }

            let s1 = apply_all(&ops);
            let s2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// After every op apply, `len() == to_string().chars().count()` must hold.
        /// Permutation forces transient out-of-order delivery scenarios.
        #[test]
        fn len_visible_consistency(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let mut state = TextCrdt::new();
            for op in &permuted {
                state = state.apply(op.clone()).unwrap();
                prop_assert_eq!(
                    state.len(),
                    state.to_string().chars().count(),
                    "len/visible-chars consistency broken after applying {:?}", op
                );
            }
        }
    }

    proptest! {
        /// TextCrdt output matches an independent reference for both the original and a
        /// permuted op stream. Detects deterministic-but-wrong renderings that pure
        /// state-equality properties miss.
        #[test]
        fn render_matches_independent_reference(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            prop_assert_eq!(apply_all(&ops).to_string(), reference_render(&ops));
            prop_assert_eq!(apply_all(&permuted).to_string(), reference_render(&permuted));
        }
    }
}

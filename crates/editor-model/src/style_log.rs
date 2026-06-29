use editor_crdt::{CrdtError, Dot, LwwRegOp, OrMap, OrMapOp, OrSetOp};
use serde::{Deserialize, Serialize};

use crate::{ModelError, Modifier, StyleEntry};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StyleOp {
    #[wire(n(0))]
    Name(#[wire(n(0))] LwwRegOp<String>),
    #[wire(n(1))]
    Modifiers(#[wire(n(0))] OrSetOp<Modifier>),
    #[wire(n(2))]
    Presence(#[wire(n(0))] OrMapOp<String, ()>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
pub struct StyleRegOp {
    #[wire(n(0))]
    pub style_id: String,
    #[wire(n(1))]
    pub op: StyleOp,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleLog {
    ops: imbl::HashMap<Dot, StyleRegOp>,
}

impl StyleLog {
    pub fn new() -> Self {
        Self {
            ops: imbl::HashMap::new(),
        }
    }

    pub fn apply(&self, id: Dot, op: StyleRegOp) -> Result<Self, ModelError> {
        if let Some(existing) = self.ops.get(&id) {
            if *existing != op {
                return Err(ModelError::Crdt(CrdtError::DotConflict { dot: id }));
            }
            return Ok(self.clone());
        }
        if let StyleOp::Presence(OrMapOp::Set { key, .. }) = &op.op
            && *key != op.style_id
        {
            return Err(ModelError::StylePresenceKeyMismatch {
                style_id: op.style_id.clone(),
                key: key.clone(),
            });
        }
        Ok(Self {
            ops: self.ops.update(id, op),
        })
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Dot, &StyleRegOp)> + '_ {
        self.ops.iter()
    }

    pub fn style_entry(&self, style_id: &str) -> Option<StyleEntry> {
        let mut entry: Option<StyleEntry> = None;
        for (op_dot, reg) in &self.ops {
            if reg.style_id != style_id {
                continue;
            }
            match &reg.op {
                StyleOp::Name(lww_op) => {
                    let e = entry.get_or_insert_with(StyleEntry::new);
                    let next = e.name.apply(*op_dot, lww_op.clone());
                    e.name = fold_crdt(&e.name, next);
                }
                StyleOp::Modifiers(orset_op) => {
                    let e = entry.get_or_insert_with(StyleEntry::new);
                    let next = e.modifiers.apply(*op_dot, orset_op.clone());
                    e.modifiers = fold_crdt(&e.modifiers, next);
                }
                StyleOp::Presence(_) => {}
            }
        }
        entry
    }

    pub fn registered_presence(&self) -> OrMap<String, ()> {
        let mut presence: OrMap<String, ()> = OrMap::new();
        for (op_dot, reg) in &self.ops {
            if let StyleOp::Presence(presence_op) = &reg.op {
                presence = fold_crdt(&presence, presence.apply(*op_dot, presence_op.clone()));
            }
        }
        presence
    }

    pub fn registered(&self, style_id: &str) -> bool {
        self.registered_presence()
            .contains_key(&style_id.to_string())
    }

    pub fn registered_entries(&self) -> imbl::HashMap<String, StyleEntry> {
        let mut presence: OrMap<String, ()> = OrMap::new();
        let mut entries: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        for (op_dot, reg) in &self.ops {
            match &reg.op {
                StyleOp::Name(lww_op) => {
                    let mut e = entries.get(&reg.style_id).cloned().unwrap_or_default();
                    let next = e.name.apply(*op_dot, lww_op.clone());
                    e.name = fold_crdt(&e.name, next);
                    entries.insert(reg.style_id.clone(), e);
                }
                StyleOp::Modifiers(orset_op) => {
                    let mut e = entries.get(&reg.style_id).cloned().unwrap_or_default();
                    let next = e.modifiers.apply(*op_dot, orset_op.clone());
                    e.modifiers = fold_crdt(&e.modifiers, next);
                    entries.insert(reg.style_id.clone(), e);
                }
                StyleOp::Presence(presence_op) => {
                    presence = fold_crdt(&presence, presence.apply(*op_dot, presence_op.clone()));
                }
            }
        }
        presence
            .iter()
            .map(|(id, _)| (id.clone(), entries.get(id).cloned().unwrap_or_default()))
            .collect()
    }
}

fn fold_crdt<C: Clone>(old: &C, applied: Result<C, CrdtError>) -> C {
    match applied {
        Ok(next) => next,
        Err(e) => {
            debug_assert!(false, "StyleLog fold: unexpected {e:?}");
            old.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Modifier;

    fn round_trip<T: editor_crdt::wire::Wire>(value: &T) -> editor_crdt::wire::WireResult<T> {
        use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, WireError};
        let mut cc = CollectCtx::new();
        value.collect(&mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };
        let mut buf = Vec::new();
        value.encode(&ec, &mut buf)?;
        let mut slice = &buf[..];
        let out = T::decode(&dc, &mut slice)?;
        if !slice.is_empty() {
            return Err(WireError::TrailingBytes {
                remaining: slice.len(),
            });
        }
        Ok(out)
    }

    fn name_op(sid: &str, value: &str) -> StyleRegOp {
        StyleRegOp {
            style_id: sid.to_string(),
            op: StyleOp::Name(editor_crdt::LwwRegOp::Set {
                value: value.to_string(),
            }),
        }
    }

    fn presence_unset_op(sid: &str, observed: Vec<Dot>) -> StyleRegOp {
        StyleRegOp {
            style_id: sid.to_string(),
            op: StyleOp::Presence(editor_crdt::OrMapOp::Unset { observed }),
        }
    }

    fn presence_set_op(sid: &str, key: &str) -> StyleRegOp {
        StyleRegOp {
            style_id: sid.to_string(),
            op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                key: key.to_string(),
                value: (),
            }),
        }
    }

    fn add_op(sid: &str, m: Modifier) -> StyleRegOp {
        StyleRegOp {
            style_id: sid.to_string(),
            op: StyleOp::Modifiers(editor_crdt::OrSetOp::Add { elem: m }),
        }
    }

    fn remove_op(sid: &str, observed: Dot) -> StyleRegOp {
        StyleRegOp {
            style_id: sid.to_string(),
            op: StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { observed }),
        }
    }

    #[test]
    fn registered_true_after_presence_set() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "s1"))
            .unwrap();
        assert!(log.registered("s1"));
        assert!(!log.registered("s2"));
    }

    #[test]
    fn registered_false_after_unset_observing_add() {
        let d1 = Dot::new(2, 0);
        let log = StyleLog::new()
            .apply(d1, presence_set_op("s1", "s1"))
            .unwrap()
            .apply(Dot::new(3, 0), presence_unset_op("s1", vec![d1]))
            .unwrap();
        assert!(!log.registered("s1"));
    }

    #[test]
    fn registered_entries_joins_registered_with_default() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "s1"))
            .unwrap();
        let entries = log.registered_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.get("s1").unwrap().name.get(), "");
    }

    #[test]
    fn registered_entries_includes_name() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "s1"))
            .unwrap()
            .apply(Dot::new(3, 0), name_op("s1", "Heading"))
            .unwrap();
        assert_eq!(
            log.registered_entries().get("s1").unwrap().name.get(),
            "Heading"
        );
    }

    #[test]
    fn registered_entries_excludes_entry_only_unregistered() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), name_op("s2", "x"))
            .unwrap();
        assert!(log.registered_entries().get("s2").is_none());
    }

    #[test]
    fn registered_entries_matches_style_entry() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "s1"))
            .unwrap()
            .apply(Dot::new(3, 0), name_op("s1", "H"))
            .unwrap();
        let joined = log.registered_entries();
        assert_eq!(
            joined.get("s1"),
            Some(&log.style_entry("s1").unwrap_or_default())
        );
    }

    #[test]
    fn registered_presence_add_wins_and_exposes_tags() {
        let d1 = Dot::new(2, 0);
        let d2 = Dot::new(2, 1);
        let log = StyleLog::new()
            .apply(d1, presence_set_op("s1", "s1"))
            .unwrap()
            .apply(d2, presence_set_op("s1", "s1"))
            .unwrap()
            .apply(Dot::new(3, 0), presence_unset_op("s1", vec![d1]))
            .unwrap();
        assert!(log.registered("s1"), "d2 생존 → 여전히 registered");
        let tags: Vec<Dot> = log
            .registered_presence()
            .tags_for(&"s1".to_string())
            .copied()
            .collect();
        assert_eq!(tags, vec![d2], "관측-제거된 d1은 빠지고 d2만 노출");
    }

    #[test]
    fn style_entry_name_lww_higher_dot_wins() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), name_op("s1", "a"))
            .unwrap()
            .apply(Dot::new(3, 0), name_op("s1", "b"))
            .unwrap();
        let e = log.style_entry("s1").unwrap();
        assert_eq!(e.name.get(), "b");
    }

    #[test]
    fn style_entry_modifiers_add() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), add_op("s1", Modifier::Bold))
            .unwrap();
        let e = log.style_entry("s1").unwrap();
        assert!(e.modifiers.contains(&Modifier::Bold));
    }

    #[test]
    fn style_entry_modifiers_observed_remove_keeps_survivor() {
        let d1 = Dot::new(2, 0);
        let log = StyleLog::new()
            .apply(d1, add_op("s1", Modifier::Bold))
            .unwrap()
            .apply(Dot::new(2, 1), add_op("s1", Modifier::Italic))
            .unwrap()
            .apply(Dot::new(3, 0), remove_op("s1", d1))
            .unwrap();
        let e = log.style_entry("s1").unwrap();
        assert!(!e.modifiers.contains(&Modifier::Bold));
        assert!(e.modifiers.contains(&Modifier::Italic));
    }

    #[test]
    fn style_entry_no_ops_is_none() {
        let log = StyleLog::new();
        assert_eq!(log.style_entry("s9"), None);
    }

    #[test]
    fn style_entry_presence_only_is_none() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "s1"))
            .unwrap();
        assert_eq!(log.style_entry("s1"), None);
    }

    #[test]
    fn style_entry_isolates_id() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), name_op("s1", "a"))
            .unwrap()
            .apply(Dot::new(2, 1), name_op("s2", "b"))
            .unwrap();
        assert_eq!(log.style_entry("s1").unwrap().name.get(), "a");
    }

    #[test]
    fn apply_stores_op() {
        let log = StyleLog::new()
            .apply(Dot::new(2, 0), name_op("s1", "a"))
            .unwrap();
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn apply_same_dot_same_op_idempotent() {
        let o = name_op("s1", "a");
        let a = StyleLog::new().apply(Dot::new(2, 0), o.clone()).unwrap();
        let b = a.apply(Dot::new(2, 0), o).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn apply_same_dot_diff_op_conflicts() {
        let d = Dot::new(2, 0);
        let a = StyleLog::new().apply(d, name_op("s1", "a")).unwrap();
        let err = a.apply(d, name_op("s1", "b")).unwrap_err();
        assert_eq!(
            err,
            crate::ModelError::Crdt(editor_crdt::CrdtError::DotConflict { dot: d })
        );
    }

    #[test]
    fn apply_presence_key_mismatch_rejected() {
        let err = StyleLog::new()
            .apply(Dot::new(2, 0), presence_set_op("s1", "other"))
            .unwrap_err();
        assert_eq!(
            err,
            crate::ModelError::StylePresenceKeyMismatch {
                style_id: "s1".to_string(),
                key: "other".to_string(),
            }
        );
    }

    #[test]
    fn style_reg_op_wire_round_trips_all_subcases() {
        let ops = [
            StyleRegOp {
                style_id: "s1".to_string(),
                op: StyleOp::Name(editor_crdt::LwwRegOp::Set {
                    value: "Heading".to_string(),
                }),
            },
            StyleRegOp {
                style_id: "s1".to_string(),
                op: StyleOp::Modifiers(editor_crdt::OrSetOp::Add {
                    elem: crate::Modifier::Bold,
                }),
            },
            StyleRegOp {
                style_id: "s1".to_string(),
                op: StyleOp::Modifiers(editor_crdt::OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                }),
            },
            StyleRegOp {
                style_id: "s1".to_string(),
                op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                    key: "s1".to_string(),
                    value: (),
                }),
            },
            StyleRegOp {
                style_id: "s1".to_string(),
                op: StyleOp::Presence(editor_crdt::OrMapOp::Unset {
                    observed: vec![Dot::new(2, 1)],
                }),
            },
        ];
        for op in &ops {
            assert_eq!(&round_trip(op).unwrap(), op);
        }
    }
}

#[cfg(test)]
mod proptests {
    use std::collections::HashSet;

    use editor_crdt::Dot;
    use proptest::prelude::*;

    use super::*;
    use crate::Modifier;

    fn permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
        let mut indexed: Vec<(u64, T)> = items
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut z = (i as u64).wrapping_add(seed.wrapping_mul(0x9E3779B97F4A7C15));
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
                z ^= z >> 31;
                (z, x.clone())
            })
            .collect();
        indexed.sort_by_key(|(k, _)| *k);
        indexed.into_iter().map(|(_, x)| x).collect()
    }

    fn arb_dot() -> impl Strategy<Value = Dot> {
        (any::<u64>(), any::<u64>()).prop_map(|(a, c)| Dot::new(a, c))
    }

    fn arb_modifier() -> impl Strategy<Value = Modifier> {
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            any::<u32>().prop_map(|value| Modifier::FontSize { value }),
        ]
    }

    fn arb_style_reg_op() -> impl Strategy<Value = StyleRegOp> {
        (0u8..4).prop_flat_map(|i| {
            let sid = format!("s{i}");
            let sid_for_op = sid.clone();
            let op = prop_oneof![
                any::<String>()
                    .prop_map(|v| StyleOp::Name(editor_crdt::LwwRegOp::Set { value: v })),
                arb_modifier()
                    .prop_map(|m| StyleOp::Modifiers(editor_crdt::OrSetOp::Add { elem: m })),
                arb_dot()
                    .prop_map(|d| StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { observed: d })),
                Just(StyleOp::Presence(editor_crdt::OrMapOp::Set {
                    key: sid_for_op.clone(),
                    value: (),
                })),
                prop::collection::vec(arb_dot(), 0..3).prop_map(|mut ds| {
                    ds.sort();
                    ds.dedup();
                    StyleOp::Presence(editor_crdt::OrMapOp::Unset { observed: ds })
                }),
            ];
            op.prop_map(move |op| StyleRegOp {
                style_id: sid.clone(),
                op,
            })
        })
    }

    fn apply_all(pairs: &[(Dot, StyleRegOp)]) -> StyleLog {
        let mut log = StyleLog::new();
        for (d, op) in pairs {
            log = log
                .apply(*d, op.clone())
                .expect("distinct dots + valid presence keys never error");
        }
        log
    }

    fn arb_name_ops() -> impl Strategy<Value = Vec<(Dot, String)>> {
        prop::collection::vec((any::<u64>(), any::<u64>(), any::<String>()), 0..24).prop_map(
            |raw| {
                let mut seen = HashSet::new();
                raw.into_iter()
                    .map(|(a, c, v)| (Dot::new(a, c), v))
                    .filter(|(d, _)| seen.insert(*d))
                    .collect()
            },
        )
    }

    proptest! {
        #[test]
        fn style_log_converges_under_permutation(
            ops in prop::collection::vec(arb_style_reg_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, StyleRegOp)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            prop_assert_eq!(apply_all(&pairs), apply_all(&permute(&pairs, seed)));
        }

        #[test]
        fn style_log_idempotent_under_permutation(
            ops in prop::collection::vec(arb_style_reg_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, StyleRegOp)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            let once = apply_all(&pairs);
            let doubled: Vec<(Dot, StyleRegOp)> =
                pairs.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            prop_assert_eq!(once, apply_all(&permute(&doubled, seed)));
        }

        #[test]
        fn name_matches_max_dot_reference(ops in arb_name_ops()) {
            let mut log = StyleLog::new();
            for (d, v) in &ops {
                log = log
                    .apply(*d, StyleRegOp {
                        style_id: "s".to_string(),
                        op: StyleOp::Name(editor_crdt::LwwRegOp::Set { value: v.clone() }),
                    })
                    .unwrap();
            }
            let reference = ops
                .iter()
                .max_by_key(|(d, _)| *d)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let got = log.style_entry("s").map(|e| e.name.get().clone()).unwrap_or_default();
            prop_assert_eq!(got, reference);
        }

        #[test]
        fn modifiers_match_alive_token_oracle(
            steps in prop::collection::vec(
                prop_oneof![
                    arb_modifier().prop_map(Ok::<Modifier, usize>),
                    (0usize..10).prop_map(Err::<Modifier, usize>),
                ],
                0..24,
            ),
        ) {
            use std::collections::HashSet;
            let mut log = StyleLog::new();
            let mut adds: Vec<(Dot, Modifier)> = Vec::new();
            let mut removed: HashSet<Dot> = HashSet::new();
            for (i, step) in steps.iter().enumerate() {
                let d = Dot::new(1, i as u64);
                match step {
                    Ok(m) => {
                        log = log
                            .apply(d, StyleRegOp {
                                style_id: "s".to_string(),
                                op: StyleOp::Modifiers(editor_crdt::OrSetOp::Add { elem: m.clone() }),
                            })
                            .unwrap();
                        adds.push((d, m.clone()));
                    }
                    Err(n) => {
                        if let Some((add_dot, _)) = adds.get(*n) {
                            let add_dot = *add_dot;
                            log = log
                                .apply(d, StyleRegOp {
                                    style_id: "s".to_string(),
                                    op: StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { observed: add_dot }),
                                })
                                .unwrap();
                            removed.insert(add_dot);
                        }
                    }
                }
            }
            let expected: HashSet<Modifier> = adds
                .iter()
                .filter(|(d, _)| !removed.contains(d))
                .map(|(_, m)| m.clone())
                .collect();
            let got: HashSet<Modifier> = log
                .style_entry("s")
                .map(|e| e.modifiers.iter().cloned().collect())
                .unwrap_or_default();
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn presence_match_alive_token_oracle(
            steps in prop::collection::vec(
                prop_oneof![Just(None::<usize>), (0usize..10).prop_map(Some)],
                0..24,
            ),
        ) {
            use std::collections::HashSet;
            let mut log = StyleLog::new();
            let mut sets: Vec<Dot> = Vec::new();
            let mut removed: HashSet<Dot> = HashSet::new();
            for (i, step) in steps.iter().enumerate() {
                let d = Dot::new(1, i as u64);
                match step {
                    None => {
                        log = log
                            .apply(d, StyleRegOp {
                                style_id: "s".to_string(),
                                op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                                    key: "s".to_string(),
                                    value: (),
                                }),
                            })
                            .unwrap();
                        sets.push(d);
                    }
                    Some(n) => {
                        if let Some(set_dot) = sets.get(*n) {
                            let set_dot = *set_dot;
                            log = log
                                .apply(d, StyleRegOp {
                                    style_id: "s".to_string(),
                                    op: StyleOp::Presence(editor_crdt::OrMapOp::Unset {
                                        observed: vec![set_dot],
                                    }),
                                })
                                .unwrap();
                            removed.insert(set_dot);
                        }
                    }
                }
            }
            let alive: HashSet<Dot> = sets.iter().filter(|d| !removed.contains(d)).copied().collect();
            prop_assert_eq!(log.registered("s"), !alive.is_empty());
            let got_tags: HashSet<Dot> = log
                .registered_presence()
                .tags_for(&"s".to_string())
                .copied()
                .collect();
            prop_assert_eq!(got_tags, alive);
        }
    }
}

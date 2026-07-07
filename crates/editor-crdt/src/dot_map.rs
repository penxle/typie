use crate::Dot;

/// `imbl` map with a fast non-cryptographic hasher — the default
/// `RandomState` (SipHash) shows up in profiles when every dot lookup hashes
/// through it. Keys here are u64 actors / 16-byte dots, not attacker-chosen
/// hash-flood surfaces.
type FastMap<K, V> =
    imbl::GenericHashMap<K, V, hashbrown::DefaultHashBuilder, imbl::shared_ptr::DefaultSharedPtr>;

/// Largest clock gap a lane will bridge with `None` slots. Anything further
/// out lands in the `spill` map so a hostile/corrupt clock can't force a
/// multi-gigabyte dense allocation.
const MAX_LANE_GAP: u64 = 1 << 16;

/// Dense per-actor storage keyed by `Dot`, exploiting that an actor's clocks
/// are (near-)sequential: `clock - base` indexes into a per-actor
/// `imbl::Vector` lane. Compared to `imbl::HashMap<Dot, V>` this stores
/// values packed into 64-wide chunks — one allocation per 64 sequential
/// inserts instead of one HAMT node (with hundreds of bytes of overhead) per
/// insert — while keeping the O(1) structural-sharing clone the
/// copy-on-write `OpGraph` relies on.
#[derive(Clone, Debug)]
pub struct DotMap<V> {
    lanes: FastMap<u64, Lane<V>>,
    spill: FastMap<Dot, V>,
    len: usize,
}

#[derive(Clone, Debug)]
struct Lane<V> {
    base: u64,
    slots: imbl::Vector<Option<V>>,
}

impl<V: Clone> Lane<V> {
    fn index_of(&self, clock: u64) -> Option<usize> {
        if clock < self.base {
            return None;
        }
        let idx = clock - self.base;
        if idx >= self.slots.len() as u64 {
            return None;
        }
        Some(idx as usize)
    }
}

impl<V: Clone> DotMap<V> {
    pub fn new() -> Self {
        DotMap {
            lanes: FastMap::default(),
            spill: FastMap::default(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, d: &Dot) -> Option<&V> {
        if let Some(lane) = self.lanes.get(&d.actor)
            && let Some(idx) = lane.index_of(d.clock)
            && let Some(v) = &lane.slots[idx]
        {
            return Some(v);
        }
        if self.spill.is_empty() {
            None
        } else {
            self.spill.get(d)
        }
    }

    pub fn get_mut(&mut self, d: &Dot) -> Option<&mut V> {
        let in_lane = self.lanes.get(&d.actor).and_then(|lane| {
            lane.index_of(d.clock)
                .map(|idx| (idx, lane.slots[idx].is_some()))
        });
        if let Some((idx, true)) = in_lane {
            let lane = self.lanes.get_mut(&d.actor).expect("lane exists");
            return lane.slots.get_mut(idx).and_then(|slot| slot.as_mut());
        }
        if self.spill.is_empty() {
            None
        } else {
            self.spill.get_mut(d)
        }
    }

    pub fn contains_key(&self, d: &Dot) -> bool {
        self.get(d).is_some()
    }

    pub fn insert(&mut self, d: Dot, v: V) -> Option<V> {
        // A dot that previously landed in `spill` must not gain a second home
        // in a lane that has since grown over its clock.
        if !self.spill.is_empty() && self.spill.contains_key(&d) {
            return self.spill.insert(d, v);
        }
        match self.lanes.get_mut(&d.actor) {
            None => {
                let mut slots = imbl::Vector::new();
                slots.push_back(Some(v));
                self.lanes.insert(
                    d.actor,
                    Lane {
                        base: d.clock,
                        slots,
                    },
                );
                self.len += 1;
                None
            }
            Some(lane) => {
                if let Some(idx) = lane.index_of(d.clock) {
                    let slot = lane.slots.get_mut(idx).expect("index in range");
                    let old = slot.replace(v);
                    if old.is_none() {
                        self.len += 1;
                    }
                    return old;
                }
                if d.clock >= lane.base {
                    let gap = d.clock - lane.base - lane.slots.len() as u64;
                    if gap > MAX_LANE_GAP {
                        let old = self.spill.insert(d, v);
                        if old.is_none() {
                            self.len += 1;
                        }
                        return old;
                    }
                    for _ in 0..gap {
                        lane.slots.push_back(None);
                    }
                    lane.slots.push_back(Some(v));
                } else {
                    let gap = lane.base - d.clock;
                    if gap > MAX_LANE_GAP {
                        let old = self.spill.insert(d, v);
                        if old.is_none() {
                            self.len += 1;
                        }
                        return old;
                    }
                    for _ in 1..gap {
                        lane.slots.push_front(None);
                    }
                    lane.slots.push_front(Some(v));
                    lane.base = d.clock;
                }
                self.len += 1;
                None
            }
        }
    }

    /// Ensure a slot for `d` exists (inserting `V::default()` when absent)
    /// and return a mutable reference to it.
    pub fn entry_or_default(&mut self, d: Dot) -> &mut V
    where
        V: Default,
    {
        if !self.contains_key(&d) {
            self.insert(d, V::default());
        }
        self.get_mut(&d).expect("slot just ensured")
    }

    // Production callers never remove ops; only the test-only data-loss
    // model (`debug_remove`) does.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn remove(&mut self, d: &Dot) -> Option<V> {
        if let Some(lane) = self.lanes.get_mut(&d.actor)
            && let Some(idx) = lane.index_of(d.clock)
        {
            let slot = lane.slots.get_mut(idx).expect("index in range");
            let old = slot.take();
            if old.is_some() {
                self.len -= 1;
                return old;
            }
        }
        if self.spill.is_empty() {
            return None;
        }
        let old = self.spill.remove(d);
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.lanes
            .values()
            .flat_map(|lane| lane.slots.iter().filter_map(|slot| slot.as_ref()))
            .chain(self.spill.values())
    }

    pub fn iter(&self) -> impl Iterator<Item = (Dot, &V)> + '_ {
        self.lanes
            .iter()
            .flat_map(|(actor, lane)| {
                lane.slots.iter().enumerate().filter_map(|(idx, slot)| {
                    slot.as_ref()
                        .map(|v| (Dot::new(*actor, lane.base + idx as u64), v))
                })
            })
            .chain(self.spill.iter().map(|(d, v)| (*d, v)))
    }
}

impl<V: Clone> Default for DotMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Live-entry equality: two maps are equal iff they hold the same dots with
/// equal values. Lane layout (trailing/leading `None` slots, spill vs lane
/// placement) is a storage detail and deliberately ignored.
impl<V: Clone + PartialEq> PartialEq for DotMap<V> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().all(|(d, v)| other.get(&d) == Some(v))
    }
}

impl<V: Clone + Eq> Eq for DotMap<V> {}

impl<V: Clone> std::ops::Index<&Dot> for DotMap<V> {
    type Output = V;

    fn index(&self, d: &Dot) -> &V {
        self.get(d).expect("no entry found for dot")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn insert_get_remove_roundtrip() {
        let mut m: DotMap<u32> = DotMap::new();
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        assert_eq!(m.insert(a, 10), None);
        assert_eq!(m.insert(b, 20), None);
        assert_eq!(m.insert(a, 11), Some(10));
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&a), Some(&11));
        assert_eq!(m.remove(&a), Some(11));
        assert_eq!(m.get(&a), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn far_clock_spills_without_dense_allocation() {
        let mut m: DotMap<u32> = DotMap::new();
        m.insert(Dot::new(1, 0), 1);
        m.insert(Dot::new(1, u64::MAX - 1), 2);
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&Dot::new(1, u64::MAX - 1)), Some(&2));
        assert_eq!(m.remove(&Dot::new(1, u64::MAX - 1)), Some(2));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn backward_clock_extends_lane_front() {
        let mut m: DotMap<u32> = DotMap::new();
        m.insert(Dot::new(1, 10), 1);
        m.insert(Dot::new(1, 3), 2);
        assert_eq!(m.get(&Dot::new(1, 3)), Some(&2));
        assert_eq!(m.get(&Dot::new(1, 10)), Some(&1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&Dot::new(1, 5)), None);
    }

    #[test]
    fn eq_ignores_storage_layout() {
        let mut a: DotMap<u32> = DotMap::new();
        a.insert(Dot::new(1, 0), 1);
        a.insert(Dot::new(1, 5), 2);
        a.remove(&Dot::new(1, 5));
        let mut b: DotMap<u32> = DotMap::new();
        b.insert(Dot::new(1, 0), 1);
        assert_eq!(a, b);
    }

    proptest! {
        // 무작위 연산 시퀀스에서 hashbrown::HashMap과 관측 동치.
        #[test]
        fn behaves_like_hashmap(ops in proptest::collection::vec(
            (0u64..3, 0u64..200, 0u32..1000, 0u8..3), 0..400,
        )) {
            let mut dut: DotMap<u32> = DotMap::new();
            let mut reference: std::collections::HashMap<Dot, u32> = std::collections::HashMap::new();
            for (actor, clock, value, kind) in ops {
                let d = Dot::new(actor, clock);
                match kind {
                    0 => prop_assert_eq!(dut.insert(d, value), reference.insert(d, value)),
                    1 => prop_assert_eq!(dut.remove(&d), reference.remove(&d)),
                    _ => prop_assert_eq!(dut.get(&d), reference.get(&d)),
                }
                prop_assert_eq!(dut.len(), reference.len());
            }
            let mut dut_entries: Vec<(Dot, u32)> = dut.iter().map(|(d, v)| (d, *v)).collect();
            let mut ref_entries: Vec<(Dot, u32)> = reference.into_iter().collect();
            dut_entries.sort();
            ref_entries.sort();
            prop_assert_eq!(dut_entries, ref_entries);
        }
    }
}

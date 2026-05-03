use crate::{Dot, Op};
use hashbrown::HashSet;

pub(crate) fn permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
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

/// Topological permutation: at each step, pick one op uniformly (under the
/// seed-driven RNG) among the ones whose `parents` are all already emitted.
/// Equivalent random shuffle for OpGraph's strict admission, which would
/// reject parents-out-of-order delivery.
pub(crate) fn causal_permute<P: Clone>(ops: &[Op<P>], seed: u64) -> Vec<Op<P>> {
    let mut emitted: HashSet<Dot> = HashSet::new();
    let mut remaining: Vec<&Op<P>> = ops.iter().collect();
    let mut out: Vec<Op<P>> = Vec::with_capacity(ops.len());
    let mut rng_state = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);

    while !remaining.is_empty() {
        let ready: Vec<usize> = remaining
            .iter()
            .enumerate()
            .filter(|(_, op)| op.parents.iter().all(|p| emitted.contains(p)))
            .map(|(i, _)| i)
            .collect();

        assert!(
            !ready.is_empty(),
            "causal_permute: input is not a valid DAG (no op has its parents satisfied — likely a cycle or missing parent)",
        );

        // splitmix-style step for deterministic per-step randomness
        rng_state = rng_state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;

        let pick = ready[(z as usize) % ready.len()];
        let op = remaining.swap_remove(pick);
        emitted.insert(op.id);
        out.push(op.clone());
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permute_keeps_all_items() {
        let items: Vec<u32> = (0..10).collect();
        let permuted = permute(&items, 12345);
        assert_eq!(permuted.len(), items.len());
        let mut a = items.clone();
        let mut b = permuted.clone();
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }
}

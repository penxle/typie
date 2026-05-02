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

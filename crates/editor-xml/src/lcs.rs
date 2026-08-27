/// How far a Myers search runs before it gives up. The memory it holds is the
/// square of this, so a rewrite that shares nothing must bail out rather than
/// finish.
pub const MAX_EDIT_DISTANCE: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Keep { a: usize, b: usize },
    Delete { a: usize },
    Insert { b: usize },
}

pub fn diff<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Edit> {
    diff_bounded(a, b, a.len() + b.len())
        .expect("myers diff converges within a.len() + b.len() rounds")
}

pub fn diff_bounded<T: PartialEq>(a: &[T], b: &[T], max_d: usize) -> Option<Vec<Edit>> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = n + m;
    let limit = if max_d >= max as usize {
        max
    } else {
        max_d as isize
    };
    let offset = limit + 1;

    let mut v: Vec<isize> = vec![0; (2 * limit + 3) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found: Option<isize> = None;

    'search: for d in 0..=limit {
        trace.push(v[(offset - d) as usize..=(offset + d) as usize].to_vec());

        let mut k = -d;
        while k <= d {
            let i = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) {
                v[i + 1]
            } else {
                v[i - 1] + 1
            };
            let mut y = x - k;

            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }

            v[i] = x;

            if x >= n && y >= m {
                found = Some(d);
                break 'search;
            }

            k += 2;
        }
    }

    let found = found?;

    let mut script: Vec<Edit> = Vec::new();
    let mut x = n;
    let mut y = m;

    for d in (1..=found).rev() {
        let round = &trace[d as usize];
        let k = x - y;
        let i = (k + d) as usize;
        let prev_k = if k == -d || (k != d && round[i - 1] < round[i + 1]) {
            k + 1
        } else {
            k - 1
        };
        let prev_x = round[(prev_k + d) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            script.push(Edit::Keep {
                a: x as usize,
                b: y as usize,
            });
        }

        if x == prev_x {
            y -= 1;
            script.push(Edit::Insert { b: y as usize });
        } else {
            x -= 1;
            script.push(Edit::Delete { a: x as usize });
        }
    }

    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        script.push(Edit::Keep {
            a: x as usize,
            b: y as usize,
        });
    }

    script.reverse();
    Some(script)
}

pub fn lcs_pairs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    keeps(diff(a, b))
}

pub fn lcs_pairs_bounded<T: PartialEq>(
    a: &[T],
    b: &[T],
    max_d: usize,
) -> Option<Vec<(usize, usize)>> {
    diff_bounded(a, b, max_d).map(keeps)
}

fn keeps(script: Vec<Edit>) -> Vec<(usize, usize)> {
    script
        .into_iter()
        .filter_map(|e| match e {
            Edit::Keep { a, b } => Some((a, b)),
            Edit::Delete { .. } | Edit::Insert { .. } => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply<T: Clone + PartialEq + std::fmt::Debug>(a: &[T], b: &[T], script: &[Edit]) -> Vec<T> {
        let mut out = Vec::new();
        for e in script {
            match e {
                Edit::Keep { a: i, b: j } => {
                    assert_eq!(a[*i], b[*j]);
                    out.push(a[*i].clone());
                }
                Edit::Delete { .. } => {}
                Edit::Insert { b: j } => out.push(b[*j].clone()),
            }
        }
        out
    }

    pub fn lcs_len<T: PartialEq>(a: &[T], b: &[T]) -> usize {
        let mut prev = vec![0usize; b.len() + 1];
        let mut cur = vec![0usize; b.len() + 1];
        for x in a {
            for (j, y) in b.iter().enumerate() {
                cur[j + 1] = if x == y {
                    prev[j] + 1
                } else {
                    prev[j + 1].max(cur[j])
                };
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[b.len()]
    }

    #[test]
    fn script_rebuilds_target_and_keeps_common_subsequence() {
        let a: Vec<char> = "안녕하세요 세상".chars().collect();
        let b: Vec<char> = "안녕, 세상아".chars().collect();
        let script = diff(&a, &b);
        assert_eq!(apply(&a, &b, &script), b);
        let kept = script
            .iter()
            .filter(|e| matches!(e, Edit::Keep { .. }))
            .count();
        assert_eq!(kept, 5);
        assert_eq!(kept, lcs_len(&a, &b));
    }

    #[test]
    fn edge_cases() {
        assert!(diff::<char>(&[], &[]).is_empty());
        assert_eq!(diff(&['a'], &[]), vec![Edit::Delete { a: 0 }]);
        assert_eq!(diff(&[], &['a']), vec![Edit::Insert { b: 0 }]);
        assert_eq!(lcs_pairs(&[1, 2, 3], &[3, 2, 1]).len(), 1);
    }

    #[test]
    fn empty_source_inserts_every_target_element() {
        let b: Vec<u8> = (0..16).collect();
        let script = diff::<u8>(&[], &b);
        assert_eq!(
            script,
            (0..16usize)
                .map(|j| Edit::Insert { b: j })
                .collect::<Vec<_>>()
        );
        assert_eq!(apply(&[], &b, &script), b);
        assert!(lcs_pairs::<u8>(&[], &b).is_empty());
    }

    #[test]
    fn empty_target_deletes_every_source_element() {
        let a: Vec<u8> = (0..16).collect();
        let script = diff::<u8>(&a, &[]);
        assert_eq!(
            script,
            (0..16usize)
                .map(|i| Edit::Delete { a: i })
                .collect::<Vec<_>>()
        );
        assert!(apply(&a, &[], &script).is_empty());
        assert!(lcs_pairs::<u8>(&a, &[]).is_empty());
    }

    #[test]
    fn identical_inputs_keep_everything() {
        let a: Vec<char> = "가나다라마바사".chars().collect();
        let script = diff(&a, &a);
        assert_eq!(
            script,
            (0..a.len())
                .map(|i| Edit::Keep { a: i, b: i })
                .collect::<Vec<_>>()
        );
        assert_eq!(
            lcs_pairs(&a, &a),
            (0..a.len()).map(|i| (i, i)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn lcs_pairs_are_increasing_and_equal() {
        let a: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7];
        let b: Vec<u8> = vec![0, 2, 9, 4, 6, 7, 8];
        let pairs = lcs_pairs(&a, &b);
        assert_eq!(pairs.len(), lcs_len(&a, &b));
        for w in pairs.windows(2) {
            assert!(w[0].0 < w[1].0);
            assert!(w[0].1 < w[1].1);
        }
        for &(i, j) in &pairs {
            assert_eq!(a[i], b[j]);
        }
    }

    #[test]
    fn single_replacement_in_long_input_is_cheap() {
        let a: Vec<char> = (0..5000u32)
            .map(|i| char::from(b'a' + (i % 26) as u8))
            .collect();
        let mut b = a.clone();
        b[2500] = '#';

        let started = std::time::Instant::now();
        let script = diff(&a, &b);
        let elapsed = started.elapsed();

        assert_eq!(apply(&a, &b, &script), b);
        let kept = script
            .iter()
            .filter(|e| matches!(e, Edit::Keep { .. }))
            .count();
        assert_eq!(kept, 4999);
        assert_eq!(script.len(), 5001);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "diff took {elapsed:?}"
        );
    }

    #[test]
    fn generous_bound_reproduces_the_unbounded_script() {
        let cases: Vec<(Vec<char>, Vec<char>)> = vec![
            (
                "안녕하세요 세상".chars().collect(),
                "안녕, 세상아".chars().collect(),
            ),
            (Vec::new(), Vec::new()),
            (vec!['a'], Vec::new()),
            (Vec::new(), vec!['a']),
            (
                "가나다라마바사".chars().collect(),
                "가나다라마바사".chars().collect(),
            ),
            (vec!['1', '2', '3'], vec!['3', '2', '1']),
        ];

        for (a, b) in cases {
            let script = diff(&a, &b);
            assert_eq!(
                diff_bounded(&a, &b, a.len() + b.len()),
                Some(script.clone())
            );
            assert_eq!(diff_bounded(&a, &b, usize::MAX), Some(script));
        }
    }

    #[test]
    fn bound_below_the_true_distance_gives_up() {
        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "xyz".chars().collect();

        assert_eq!(diff_bounded(&a, &b, 5), None);
        assert_eq!(diff_bounded(&a, &b, 6), Some(diff(&a, &b)));
    }

    #[test]
    fn total_rewrite_bails_out_within_the_bound() {
        let a: Vec<char> = (0..3000u32)
            .map(|i| char::from_u32(0xac00 + i).unwrap())
            .collect();
        let b: Vec<char> = (0..3000u32)
            .map(|i| char::from_u32(0xac00 + 3000 + i).unwrap())
            .collect();

        let started = std::time::Instant::now();
        let script = diff_bounded(&a, &b, 64);
        let elapsed = started.elapsed();

        assert_eq!(script, None);
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "diff_bounded took {elapsed:?}"
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::tests::lcs_len;
    use super::*;

    proptest! {
        #[test]
        fn script_is_a_minimal_front_to_back_rewrite(
            a in prop::collection::vec(0u8..6, 0..24),
            b in prop::collection::vec(0u8..6, 0..24),
        ) {
            let script = diff(&a, &b);

            let mut rebuilt = Vec::new();
            let mut next_a = 0usize;
            let mut next_b = 0usize;
            let mut kept = 0usize;
            let mut last: Option<(usize, usize)> = None;

            for e in &script {
                match *e {
                    Edit::Keep { a: i, b: j } => {
                        prop_assert_eq!(i, next_a);
                        prop_assert_eq!(j, next_b);
                        prop_assert_eq!(a[i], b[j]);
                        if let Some((pi, pj)) = last {
                            prop_assert!(pi < i);
                            prop_assert!(pj < j);
                        }
                        last = Some((i, j));
                        next_a += 1;
                        next_b += 1;
                        kept += 1;
                        rebuilt.push(a[i]);
                    }
                    Edit::Delete { a: i } => {
                        prop_assert_eq!(i, next_a);
                        next_a += 1;
                    }
                    Edit::Insert { b: j } => {
                        prop_assert_eq!(j, next_b);
                        next_b += 1;
                        rebuilt.push(b[j]);
                    }
                }
            }

            prop_assert_eq!(next_a, a.len());
            prop_assert_eq!(next_b, b.len());
            prop_assert_eq!(&rebuilt, &b);
            prop_assert_eq!(kept, lcs_len(&a, &b));
            prop_assert_eq!(lcs_pairs(&a, &b).len(), kept);

            prop_assert_eq!(diff_bounded(&a, &b, a.len() + b.len()), Some(script.clone()));

            let distance = script.len() - kept;
            prop_assert_eq!(diff_bounded(&a, &b, distance), Some(script));
            if distance > 0 {
                prop_assert_eq!(diff_bounded(&a, &b, distance - 1), None);
            }
        }
    }
}

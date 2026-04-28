use editor_model::NodeId;
use icu_segmenter::GraphemeClusterSegmenter;

use crate::sync::conflict::{
    BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};

fn graphemes<'a>(seg: &GraphemeClusterSegmenter, s: &'a str) -> Vec<&'a str> {
    let boundaries: Vec<usize> = seg.as_borrowed().segment_str(s).collect();
    boundaries.windows(2).map(|w| &s[w[0]..w[1]]).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOp {
    Equal,
    Insert,
    Delete,
}

#[derive(Debug, Clone)]
struct DiffEntry {
    op: DiffOp,
    base_index: usize,
    candidate_index: usize,
}

// Algorithm: Myers 1986 (greedy O(ND))
// Reference: https://blog.jcoglan.com/2017/02/12/the-myers-diff-algorithm-part-1/
fn myers_diff<T: Eq>(a: &[T], b: &[T]) -> Vec<DiffEntry> {
    let n = a.len();
    let m = b.len();
    let max = n + m;

    if max == 0 {
        return vec![];
    }

    // v[k + max] holds the furthest-reaching x on diagonal k.
    let mut v = vec![0isize; 2 * max + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    'outer: for d in 0..=(max as isize) {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let ki = (k + max as isize) as usize;
            let mut x = if k == -d || (k != d && v[ki - 1] < v[ki + 1]) {
                v[ki + 1]
            } else {
                v[ki - 1] + 1
            };
            let mut y = x - k;
            while x < n as isize && y < m as isize && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[ki] = x;
            if x >= n as isize && y >= m as isize {
                break 'outer;
            }
            k += 2;
        }
    }

    // Backtrack the trace to reconstruct the edit script.
    let mut script: Vec<DiffEntry> = Vec::new();
    let mut x = n as isize;
    let mut y = m as isize;

    for (d, v_prev) in trace.iter().enumerate().rev() {
        let d = d as isize;
        let k = x - y;
        let ki = (k + max as isize) as usize;

        let prev_k = if k == -d || (k != d && v_prev[(ki as isize - 1) as usize] < v_prev[ki + 1]) {
            k + 1
        } else {
            k - 1
        };
        let prev_ki = (prev_k + max as isize) as usize;
        let prev_x = v_prev[prev_ki];
        let prev_y = prev_x - prev_k;

        // Walk back through the snake (equal region).
        while x > prev_x + (x - prev_x - (y - prev_y)) && y > prev_y + (y - prev_y - (x - prev_x)) {
            script.push(DiffEntry {
                op: DiffOp::Equal,
                base_index: (x - 1) as usize,
                candidate_index: (y - 1) as usize,
            });
            x -= 1;
            y -= 1;
        }

        if d > 0 {
            if x == prev_x {
                script.push(DiffEntry {
                    op: DiffOp::Insert,
                    base_index: x as usize,
                    candidate_index: (y - 1) as usize,
                });
                y -= 1;
            } else {
                script.push(DiffEntry {
                    op: DiffOp::Delete,
                    base_index: (x - 1) as usize,
                    candidate_index: y as usize,
                });
                x -= 1;
            }
        }
    }

    script.reverse();
    script
}

struct Hunk {
    base_start: usize,
    base_end: usize,
    replacement: Vec<String>,
}

fn diff_to_hunks<T: Eq + ToString>(base: &[T], candidate: &[T]) -> Vec<Hunk> {
    let script = myers_diff(base, candidate);
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut current: Option<Hunk> = None;

    for entry in &script {
        match entry.op {
            DiffOp::Equal => {
                if let Some(h) = current.take() {
                    hunks.push(h);
                }
            }
            DiffOp::Delete => {
                let h = current.get_or_insert_with(|| Hunk {
                    base_start: entry.base_index,
                    base_end: entry.base_index,
                    replacement: vec![],
                });
                h.base_end = entry.base_index + 1;
            }
            DiffOp::Insert => {
                let h = current.get_or_insert_with(|| Hunk {
                    base_start: entry.base_index,
                    base_end: entry.base_index,
                    replacement: vec![],
                });
                h.replacement
                    .push(candidate[entry.candidate_index].to_string());
            }
        }
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }

    hunks
}

// Reconstruct the string that `side`'s hunks produce for base[range_start..range_end].
// Copies base graphemes verbatim between hunks, applies each hunk's replacement.
fn apply_hunks_to_range(
    base: &[&str],
    hunks: &[Hunk],
    range_start: usize,
    range_end: usize,
) -> String {
    let mut out = String::new();

    if range_start == range_end {
        // Zero-width conflict regions arise when both sides pure-insert at the same position;
        // the standard `>= range_end` break would skip those hunks entirely.
        for h in hunks {
            if h.base_start == h.base_end && h.base_start == range_start {
                for r in &h.replacement {
                    out.push_str(r);
                }
            }
        }
        return out;
    }

    let mut pos = range_start;
    for h in hunks {
        if h.base_start >= range_end {
            break;
        }
        if h.base_end <= range_start {
            continue;
        }
        let copy_from = pos.max(range_start);
        let copy_to = h.base_start.min(range_end);
        for &g in &base[copy_from..copy_to] {
            out.push_str(g);
        }
        if h.base_start < range_end {
            for r in &h.replacement {
                out.push_str(r);
            }
        }
        pos = h.base_end.max(pos);
    }
    for &g in &base[pos..range_end] {
        out.push_str(g);
    }
    out
}

fn same_position_pure_inserts(a: &Hunk, b: &Hunk) -> bool {
    a.base_start == a.base_end && b.base_start == b.base_end && a.base_start == b.base_start
}

fn integrate_hunks(
    base: &[&str],
    ours: &[Hunk],
    theirs: &[Hunk],
    node_id: NodeId,
) -> (Vec<String>, Vec<ConflictRecord>) {
    let mut result: Vec<String> = Vec::new();
    let mut conflicts: Vec<ConflictRecord> = Vec::new();
    let mut cursor = 0usize;
    let mut oi = 0usize;
    let mut ti = 0usize;

    while oi < ours.len() || ti < theirs.len() {
        let o = ours.get(oi);
        let t = theirs.get(ti);

        match (o, t) {
            (Some(o), Some(t)) => {
                // Same range, same replacement — apply once.
                if o.base_start == t.base_start
                    && o.base_end == t.base_end
                    && o.replacement == t.replacement
                {
                    for &g in &base[cursor..o.base_start] {
                        result.push(g.to_string());
                    }
                    result.extend(o.replacement.iter().cloned());
                    cursor = o.base_end;
                    oi += 1;
                    ti += 1;
                } else if o.base_end <= t.base_start
                    && !same_position_pure_inserts(o, t)
                    && ours
                        .get(oi + 1)
                        .is_none_or(|no| no.base_end <= t.base_start || no.base_start >= t.base_end)
                {
                    // Ours is fully before theirs — apply ours.
                    for &g in &base[cursor..o.base_start] {
                        result.push(g.to_string());
                    }
                    result.extend(o.replacement.iter().cloned());
                    cursor = o.base_end;
                    oi += 1;
                } else if t.base_end <= o.base_start
                    && !same_position_pure_inserts(o, t)
                    && theirs
                        .get(ti + 1)
                        .is_none_or(|nt| nt.base_end <= o.base_start || nt.base_start >= o.base_end)
                {
                    // Theirs is fully before ours — apply theirs.
                    for &g in &base[cursor..t.base_start] {
                        result.push(g.to_string());
                    }
                    result.extend(t.replacement.iter().cloned());
                    cursor = t.base_end;
                    ti += 1;
                } else {
                    // Overlapping — collect all hunks from both sides that touch
                    // the conflict region, expanding the range until stable.
                    let range_start = o.base_start.min(t.base_start);
                    let mut range_end = o.base_end.max(t.base_end);
                    let oi_start = oi;
                    let ti_start = ti;
                    oi += 1;
                    ti += 1;

                    loop {
                        let mut expanded = false;
                        while oi < ours.len() && ours[oi].base_start < range_end {
                            range_end = range_end.max(ours[oi].base_end);
                            oi += 1;
                            expanded = true;
                        }
                        while ti < theirs.len() && theirs[ti].base_start < range_end {
                            range_end = range_end.max(theirs[ti].base_end);
                            ti += 1;
                            expanded = true;
                        }
                        if !expanded {
                            break;
                        }
                    }

                    for &g in &base[cursor..range_start] {
                        result.push(g.to_string());
                    }

                    let base_slice = base[range_start..range_end].concat();
                    let ours_str =
                        apply_hunks_to_range(base, &ours[oi_start..oi], range_start, range_end);
                    let theirs_str =
                        apply_hunks_to_range(base, &theirs[ti_start..ti], range_start, range_end);

                    conflicts.push(ConflictRecord {
                        kind: ConflictKind::Text,
                        target: ConflictTarget::Text {
                            node_id,
                            range_start,
                            range_end,
                        },
                        base_value: Some(serde_json::Value::String(base_slice)),
                        branches: vec![
                            ConflictBranch {
                                side: BranchSide::Ours,
                                value: serde_json::Value::String(ours_str.clone()),
                            },
                            ConflictBranch {
                                side: BranchSide::Theirs,
                                value: serde_json::Value::String(theirs_str),
                            },
                        ],
                        auto_resolved: BranchSide::Ours,
                    });

                    // Ours wins: emit ours' version of the conflicting region.
                    result.push(ours_str);
                    cursor = range_end;
                }
            }
            (Some(o), None) => {
                for &g in &base[cursor..o.base_start] {
                    result.push(g.to_string());
                }
                result.extend(o.replacement.iter().cloned());
                cursor = o.base_end;
                oi += 1;
            }
            (None, Some(t)) => {
                for &g in &base[cursor..t.base_start] {
                    result.push(g.to_string());
                }
                result.extend(t.replacement.iter().cloned());
                cursor = t.base_end;
                ti += 1;
            }
            (None, None) => unreachable!(),
        }
    }

    for &g in &base[cursor..] {
        result.push(g.to_string());
    }

    (result, conflicts)
}

pub fn merge_text(
    segmenter: &GraphemeClusterSegmenter,
    node_id: NodeId,
    base: &str,
    ours: &str,
    theirs: &str,
) -> (String, Vec<ConflictRecord>) {
    let base_g = graphemes(segmenter, base);
    let ours_g = graphemes(segmenter, ours);
    let theirs_g = graphemes(segmenter, theirs);

    let ours_hunks = diff_to_hunks(&base_g, &ours_g);
    let theirs_hunks = diff_to_hunks(&base_g, &theirs_g);

    let (merged, conflicts) = integrate_hunks(&base_g, &ours_hunks, &theirs_hunks, node_id);
    (merged.concat(), conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::NodeId;
    use icu_segmenter::GraphemeClusterSegmenter;
    use std::collections::HashSet;

    fn segmenter() -> GraphemeClusterSegmenter {
        GraphemeClusterSegmenter::new().static_to_owned()
    }

    #[test]
    fn both_unchanged_returns_base() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "hello", "hello", "hello");
        assert_eq!(m, "hello");
        assert!(c.is_empty());
    }

    #[test]
    fn ours_only_changes() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "hello", "hi", "hello");
        assert_eq!(m, "hi");
        assert!(c.is_empty());
    }

    #[test]
    fn disjoint_inserts_auto_merge() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "ab", "Xab", "abY");
        assert_eq!(m, "XabY");
        assert!(c.is_empty());
    }

    #[test]
    fn same_region_different_replacement_creates_conflict() {
        let (m, c) = merge_text(
            &segmenter(),
            NodeId::new(),
            "hello world",
            "hello rust",
            "hello swift",
        );
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kind, ConflictKind::Text);
        assert_eq!(m, "hello rust"); // ours wins
    }

    #[test]
    fn same_region_same_replacement_no_conflict() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "abc", "aXc", "aXc");
        assert_eq!(m, "aXc");
        assert!(c.is_empty());
    }

    #[test]
    fn korean_grapheme_safe() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "한글", "한글a", "b한글");
        assert_eq!(m, "b한글a");
        assert!(c.is_empty());
    }

    #[test]
    fn emoji_zwj_safe() {
        let base = "👨‍👩‍👧";
        let (m, c) = merge_text(&segmenter(), NodeId::new(), base, base, base);
        assert_eq!(m, base);
        assert!(c.is_empty());
    }

    #[test]
    fn same_position_pure_inserts_create_conflict() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "abc", "aXbc", "aYbc");
        assert_eq!(c.len(), 1, "expected 1 conflict, got {:?}", c);
        assert_eq!(c[0].kind, ConflictKind::Text);
        assert_eq!(m, "aXbc", "ours wins by default");
    }

    #[test]
    fn pure_insert_at_boundary_with_adjacent_deletion_auto_merges() {
        let (m, c) = merge_text(&segmenter(), NodeId::new(), "a", "Xa", "");
        assert_eq!(m, "X");
        assert!(c.is_empty(), "boundary insert + deletion are disjoint");
    }

    #[test]
    fn conflict_targets_are_symmetric_for_interleaved_insert_delete_hunks() {
        let seg = segmenter();
        let node_id = NodeId::new();
        let (_, forward) = merge_text(&seg, node_id, "pu", "ap", "aapu가");
        let (_, swapped) = merge_text(&seg, node_id, "pu", "aapu가", "ap");

        let targets = |conflicts: Vec<ConflictRecord>| -> HashSet<ConflictTarget> {
            conflicts
                .into_iter()
                .map(|conflict| conflict.target)
                .collect()
        };

        assert_eq!(forward.len(), swapped.len());
        assert_eq!(targets(forward), targets(swapped));
    }
}

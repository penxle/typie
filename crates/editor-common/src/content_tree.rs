use std::collections::BTreeMap;

const B: usize = 4;
const MAX: usize = 2 * B;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sum {
    pub count: usize,
    pub cur: usize,
    pub end: usize,
}

impl std::ops::Add for Sum {
    type Output = Sum;

    fn add(self, o: Sum) -> Sum {
        Sum {
            count: self.count + o.count,
            cur: self.cur + o.cur,
            end: self.end + o.end,
        }
    }
}

impl std::ops::AddAssign for Sum {
    fn add_assign(&mut self, o: Sum) {
        self.count += o.count;
        self.cur += o.cur;
        self.end += o.end;
    }
}

pub trait Leaf: Sized {
    fn sum(&self) -> Sum;

    fn run_len(&self) -> usize;

    fn try_append(&mut self, other: &Self) -> bool;

    fn split_at(&mut self, offset: usize) -> Self;

    fn lv_start(&self) -> usize;

    fn contains_lv(&self, lv: usize) -> bool;

    fn offset_of_lv(&self, lv: usize) -> usize;
}

enum Kind<L> {
    Leaf(Vec<L>),
    Internal(Vec<usize>),
}

struct Node<L> {
    parent: Option<usize>,
    kind: Kind<L>,
    sum: Sum,
}

pub struct ContentTree<L> {
    nodes: Vec<Node<L>>,
    root: usize,
    lv_leaf: BTreeMap<usize, usize>,
}

#[derive(Clone, Copy)]
pub struct Cursor {
    pub leaf: usize,
    pub run: usize,
    pub off: usize,
    pub doc_idx: usize,
    pub end_pos: usize,
}

impl<L: Leaf> Default for ContentTree<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Leaf> ContentTree<L> {
    pub fn new() -> Self {
        let root = Node {
            parent: None,
            kind: Kind::Leaf(Vec::new()),
            sum: Sum::default(),
        };
        ContentTree {
            nodes: vec![root],
            root: 0,
            lv_leaf: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes[self.root].sum.count
    }

    pub fn cur_len(&self) -> usize {
        self.nodes[self.root].sum.cur
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn run_count(&self) -> usize {
        self.nodes
            .iter()
            .filter_map(|n| match &n.kind {
                Kind::Leaf(items) => Some(items.len()),
                Kind::Internal(_) => None,
            })
            .sum()
    }

    fn leaf_items(&self, id: usize) -> &Vec<L> {
        match &self.nodes[id].kind {
            Kind::Leaf(v) => v,
            Kind::Internal(_) => unreachable!("leaf_items on internal"),
        }
    }

    fn children(&self, id: usize) -> &Vec<usize> {
        match &self.nodes[id].kind {
            Kind::Internal(v) => v,
            Kind::Leaf(_) => unreachable!("children on leaf"),
        }
    }

    fn recompute_sum(&mut self, id: usize) {
        let s = match &self.nodes[id].kind {
            Kind::Leaf(items) => {
                let mut s = Sum::default();
                for it in items {
                    s += it.sum();
                }
                s
            }
            Kind::Internal(children) => {
                let mut s = Sum::default();
                for &c in children {
                    s += self.nodes[c].sum;
                }
                s
            }
        };
        self.nodes[id].sum = s;
    }

    fn update_path(&mut self, mut id: usize) {
        loop {
            self.recompute_sum(id);
            match self.nodes[id].parent {
                Some(p) => id = p,
                None => break,
            }
        }
    }

    fn descend_count(&self, mut i: usize) -> (usize, usize, usize) {
        let mut id = self.root;
        loop {
            match &self.nodes[id].kind {
                Kind::Leaf(items) => {
                    let mut run = 0usize;
                    while run < items.len() {
                        let rl = items[run].run_len();
                        if i < rl {
                            return (id, run, i);
                        }
                        i -= rl;
                        run += 1;
                    }
                    return (id, items.len(), 0);
                }
                Kind::Internal(children) => {
                    let mut acc = 0usize;
                    let mut chosen = *children.last().unwrap();
                    for &c in children {
                        let cc = self.nodes[c].sum.count;
                        if i <= acc + cc {
                            chosen = c;
                            i -= acc;
                            break;
                        }
                        acc += cc;
                    }
                    id = chosen;
                }
            }
        }
    }

    pub fn insert(&mut self, i: usize, item: L) {
        debug_assert_eq!(item.run_len(), 1, "insert expects a length-1 run");
        let (leaf, mut run, off) = self.descend_count(i);
        if off > 0 {
            self.split_run_in_leaf(leaf, run, off);
            run += 1;
        }
        match &mut self.nodes[leaf].kind {
            Kind::Leaf(items) => items.insert(run, item),
            Kind::Internal(_) => unreachable!(),
        }
        self.coalesce_around(leaf, run);
        self.rebuild_leaf_locator(leaf);
        self.update_path(leaf);
        self.maybe_split(leaf);
    }

    fn split_run_in_leaf(&mut self, leaf: usize, run: usize, offset: usize) {
        let right = match &mut self.nodes[leaf].kind {
            Kind::Leaf(items) => items[run].split_at(offset),
            Kind::Internal(_) => unreachable!(),
        };
        match &mut self.nodes[leaf].kind {
            Kind::Leaf(items) => items.insert(run + 1, right),
            Kind::Internal(_) => unreachable!(),
        }
    }

    fn try_merge_pair(items: &mut Vec<L>, left: usize) -> Option<usize> {
        if left + 1 >= items.len() {
            return None;
        }
        let (l, r) = items.split_at_mut(left + 1);
        if l[left].try_append(&r[0]) {
            let removed = items[left + 1].lv_start();
            items.remove(left + 1);
            Some(removed)
        } else {
            None
        }
    }

    fn coalesce_around(&mut self, leaf: usize, run: usize) {
        let mut removed_starts: [Option<usize>; 2] = [None, None];
        match &mut self.nodes[leaf].kind {
            Kind::Leaf(items) => {
                let merged_left = if run > 0 {
                    if let Some(s) = Self::try_merge_pair(items, run - 1) {
                        removed_starts[0] = Some(s);
                        removed_starts[1] = Self::try_merge_pair(items, run - 1);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !merged_left {
                    removed_starts[0] = Self::try_merge_pair(items, run);
                }
            }
            Kind::Internal(_) => unreachable!(),
        }
        for s in removed_starts.into_iter().flatten() {
            self.lv_leaf.remove(&s);
        }
    }

    fn rebuild_leaf_locator(&mut self, leaf: usize) {
        let starts: Vec<usize> = self
            .leaf_items(leaf)
            .iter()
            .map(|it| it.lv_start())
            .collect();
        for s in starts {
            self.lv_leaf.insert(s, leaf);
        }
    }

    fn slot_for_read(&self, i: usize) -> (usize, usize, usize) {
        let (mut leaf, mut run, off) = self.descend_count(i);
        while run == self.leaf_items(leaf).len() {
            match self.next_leaf(leaf) {
                Some(n) => {
                    leaf = n;
                    run = 0;
                }
                None => break,
            }
        }
        (leaf, run, off)
    }

    pub fn get(&self, i: usize) -> (&L, usize) {
        let (leaf, run, off) = self.slot_for_read(i);
        (&self.leaf_items(leaf)[run], off)
    }

    pub fn update_by_lv(&mut self, lv: usize, f: impl FnOnce(&mut L)) {
        let leaf = self.leaf_of_lv(lv);
        let run = self.run_index_in_leaf(leaf, lv);
        let offset = self.leaf_items(leaf)[run].offset_of_lv(lv);
        let rl = self.leaf_items(leaf)[run].run_len();
        let target_run = if rl == 1 {
            run
        } else if offset == 0 {
            self.split_run_in_leaf(leaf, run, 1);
            run
        } else {
            self.split_run_in_leaf(leaf, run, offset);
            let new_run = run + 1;
            let new_rl = self.leaf_items(leaf)[new_run].run_len();
            if new_rl > 1 {
                self.split_run_in_leaf(leaf, new_run, 1);
            }
            new_run
        };
        match &mut self.nodes[leaf].kind {
            Kind::Leaf(items) => f(&mut items[target_run]),
            Kind::Internal(_) => unreachable!(),
        }
        self.rebuild_leaf_locator(leaf);
        self.update_path(leaf);
        self.maybe_split(leaf);
    }

    fn leaf_of_lv(&self, lv: usize) -> usize {
        let (_, &leaf) = self
            .lv_leaf
            .range(..=lv)
            .next_back()
            .expect("leaf_of_lv: lv has a run-start <= it");
        leaf
    }

    fn run_index_in_leaf(&self, leaf: usize, lv: usize) -> usize {
        self.leaf_items(leaf)
            .iter()
            .position(|it| it.contains_lv(lv))
            .expect("run_index_in_leaf: lv in its leaf")
    }

    pub fn doc_index_of_lv(&self, lv: usize) -> usize {
        let leaf = self.leaf_of_lv(lv);
        let run = self.run_index_in_leaf(leaf, lv);
        let offset = self.leaf_items(leaf)[run].offset_of_lv(lv);
        let mut rank = offset;
        for it in &self.leaf_items(leaf)[..run] {
            rank += it.run_len();
        }
        let mut id = leaf;
        while let Some(p) = self.nodes[id].parent {
            for &c in self.children(p) {
                if c == id {
                    break;
                }
                rank += self.nodes[c].sum.count;
            }
            id = p;
        }
        rank
    }

    pub fn end_rank_at_doc_index(&self, i: usize) -> usize {
        debug_assert!(i <= self.len(), "end_rank_at_doc_index out of range");
        let mut id = self.root;
        let mut remaining = i;
        let mut end_rank = 0usize;
        loop {
            match &self.nodes[id].kind {
                Kind::Leaf(items) => {
                    let mut run = 0usize;
                    while run < items.len() {
                        let rl = items[run].run_len();
                        if remaining < rl {
                            break;
                        }
                        end_rank += items[run].sum().end;
                        remaining -= rl;
                        run += 1;
                    }
                    if run < items.len() && remaining > 0 {
                        let s = items[run].sum();
                        let per_end = s.end.checked_div(s.count).unwrap_or(0);
                        end_rank += per_end * remaining;
                    }
                    return end_rank;
                }
                Kind::Internal(children) => {
                    let mut chosen = *children.last().unwrap();
                    for &c in children {
                        let cc = self.nodes[c].sum.count;
                        if remaining <= cc {
                            chosen = c;
                            break;
                        }
                        end_rank += self.nodes[c].sum.end;
                        remaining -= cc;
                    }
                    id = chosen;
                }
            }
        }
    }

    fn normalize(&self, c: &mut Cursor) {
        loop {
            let len = self.leaf_items(c.leaf).len();
            if c.run < len {
                if c.off < self.leaf_items(c.leaf)[c.run].run_len() {
                    return;
                }
                c.off = 0;
                c.run += 1;
                continue;
            }
            match self.next_leaf(c.leaf) {
                Some(n) => {
                    c.leaf = n;
                    c.run = 0;
                    c.off = 0;
                }
                None => return,
            }
        }
    }

    pub fn cursor_at_cur_pos(&self, target: usize) -> Cursor {
        if target == 0 {
            let (leaf, run, off) = self.descend_count(0);
            let mut c = Cursor {
                leaf,
                run,
                off,
                doc_idx: 0,
                end_pos: 0,
            };
            self.normalize(&mut c);
            return c;
        }
        let mut id = self.root;
        let mut doc_idx = 0usize;
        let mut end_pos = 0usize;
        let mut remaining = target;
        loop {
            match &self.nodes[id].kind {
                Kind::Leaf(items) => {
                    let mut run = 0usize;
                    loop {
                        let s = items[run].sum();
                        if remaining <= s.cur {
                            break;
                        }
                        remaining -= s.cur;
                        doc_idx += s.count;
                        end_pos += s.end;
                        run += 1;
                    }
                    let s = items[run].sum();
                    let off = if s.cur == s.count {
                        remaining
                    } else {
                        debug_assert_eq!(s.cur, 0, "run must be homogeneous");
                        0
                    };
                    let per_end = s.end.checked_div(s.count).unwrap_or(0);
                    end_pos += per_end * off;
                    doc_idx += off;
                    let mut c = Cursor {
                        leaf: id,
                        run,
                        off,
                        doc_idx,
                        end_pos,
                    };
                    self.normalize(&mut c);
                    return c;
                }
                Kind::Internal(children) => {
                    let mut chosen = *children.last().unwrap();
                    for &c in children {
                        let s = &self.nodes[c].sum;
                        if remaining <= s.cur {
                            chosen = c;
                            break;
                        }
                        remaining -= s.cur;
                        doc_idx += s.count;
                        end_pos += s.end;
                    }
                    id = chosen;
                }
            }
        }
    }

    pub fn next_leaf(&self, leaf: usize) -> Option<usize> {
        let mut id = leaf;
        loop {
            let p = self.nodes[id].parent?;
            let ch = self.children(p);
            let pos = ch.iter().position(|&c| c == id).unwrap();
            if pos + 1 < ch.len() {
                let mut down = ch[pos + 1];
                loop {
                    match &self.nodes[down].kind {
                        Kind::Leaf(_) => return Some(down),
                        Kind::Internal(c2) => down = c2[0],
                    }
                }
            }
            id = p;
        }
    }

    pub fn cur_run(&self, c: &Cursor) -> Option<&L> {
        let items = self.leaf_items(c.leaf);
        if c.run < items.len() {
            Some(&items[c.run])
        } else {
            None
        }
    }

    pub fn run_remaining(&self, c: &Cursor) -> usize {
        match self.cur_run(c) {
            Some(r) => r.run_len() - c.off,
            None => 0,
        }
    }

    pub fn step(&self, c: &mut Cursor) {
        c.doc_idx += 1;
        let rl = self.leaf_items(c.leaf)[c.run].run_len();
        if c.off + 1 < rl {
            c.off += 1;
            return;
        }
        c.off = 0;
        let nruns = self.leaf_items(c.leaf).len();
        if c.run + 1 < nruns {
            c.run += 1;
        } else {
            match self.next_leaf(c.leaf) {
                Some(n) => {
                    c.leaf = n;
                    c.run = 0;
                }
                None => {
                    c.run = nruns;
                }
            }
        }
    }

    pub fn step_run(&self, c: &mut Cursor) -> usize {
        let rl = self.leaf_items(c.leaf)[c.run].run_len();
        let skipped = rl - c.off;
        c.doc_idx += skipped;
        c.off = 0;
        let nruns = self.leaf_items(c.leaf).len();
        if c.run + 1 < nruns {
            c.run += 1;
        } else {
            match self.next_leaf(c.leaf) {
                Some(n) => {
                    c.leaf = n;
                    c.run = 0;
                }
                None => {
                    c.run = nruns;
                }
            }
        }
        skipped
    }

    fn maybe_split(&mut self, id: usize) {
        let len = match &self.nodes[id].kind {
            Kind::Leaf(items) => items.len(),
            Kind::Internal(children) => children.len(),
        };
        if len <= MAX {
            return;
        }
        let mid = len / 2;
        let parent = self.nodes[id].parent;
        let new_id = self.nodes.len();
        match &mut self.nodes[id].kind {
            Kind::Leaf(items) => {
                let right: Vec<L> = items.split_off(mid);
                let new_node = Node {
                    parent,
                    kind: Kind::Leaf(right),
                    sum: Sum::default(),
                };
                self.nodes.push(new_node);
                let moved: Vec<usize> = self
                    .leaf_items(new_id)
                    .iter()
                    .map(|it| it.lv_start())
                    .collect();
                for s in moved {
                    self.lv_leaf.insert(s, new_id);
                }
            }
            Kind::Internal(children) => {
                let right: Vec<usize> = children.split_off(mid);
                let new_node = Node {
                    parent,
                    kind: Kind::Internal(right),
                    sum: Sum::default(),
                };
                self.nodes.push(new_node);
                let moved = self.children(new_id).clone();
                for c in moved {
                    self.nodes[c].parent = Some(new_id);
                }
            }
        }
        self.recompute_sum(id);
        self.recompute_sum(new_id);

        match parent {
            Some(p) => {
                let pos = self.children(p).iter().position(|&c| c == id).unwrap();
                match &mut self.nodes[p].kind {
                    Kind::Internal(children) => children.insert(pos + 1, new_id),
                    Kind::Leaf(_) => unreachable!(),
                }
                self.recompute_sum(p);
                self.maybe_split(p);
            }
            None => {
                let new_root_id = self.nodes.len();
                let s = self.nodes[id].sum + self.nodes[new_id].sum;
                let new_root = Node {
                    parent: None,
                    kind: Kind::Internal(vec![id, new_id]),
                    sum: s,
                };
                self.nodes.push(new_root);
                self.nodes[id].parent = Some(new_root_id);
                self.nodes[new_id].parent = Some(new_root_id);
                self.root = new_root_id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestRun {
        lv_start: usize,
        len: usize,
        cur_vis: bool,
        end_vis: bool,
    }

    impl TestRun {
        fn one(lv: usize) -> Self {
            TestRun {
                lv_start: lv,
                len: 1,
                cur_vis: true,
                end_vis: true,
            }
        }
    }

    impl Leaf for TestRun {
        fn sum(&self) -> Sum {
            Sum {
                count: self.len,
                cur: (self.cur_vis as usize) * self.len,
                end: (self.end_vis as usize) * self.len,
            }
        }
        fn run_len(&self) -> usize {
            self.len
        }
        fn try_append(&mut self, other: &Self) -> bool {
            if self.cur_vis == other.cur_vis
                && self.end_vis == other.end_vis
                && other.lv_start == self.lv_start + self.len
            {
                self.len += other.len;
                true
            } else {
                false
            }
        }
        fn split_at(&mut self, offset: usize) -> Self {
            assert!(offset > 0 && offset < self.len);
            let right = TestRun {
                lv_start: self.lv_start + offset,
                len: self.len - offset,
                cur_vis: self.cur_vis,
                end_vis: self.end_vis,
            };
            self.len = offset;
            right
        }
        fn lv_start(&self) -> usize {
            self.lv_start
        }
        fn contains_lv(&self, lv: usize) -> bool {
            lv >= self.lv_start && lv < self.lv_start + self.len
        }
        fn offset_of_lv(&self, lv: usize) -> usize {
            lv - self.lv_start
        }
    }

    fn check(tree: &ContentTree<TestRun>) -> (Vec<usize>, usize) {
        fn walk(
            tree: &ContentTree<TestRun>,
            id: usize,
            expect_parent: Option<usize>,
            out: &mut Vec<usize>,
        ) -> (Sum, usize) {
            assert_eq!(tree.nodes[id].parent, expect_parent, "parent link of {id}");
            match &tree.nodes[id].kind {
                Kind::Leaf(items) => {
                    assert!(
                        !items.is_empty() || id == tree.root,
                        "empty non-root leaf {id}"
                    );
                    assert!(items.len() <= MAX, "leaf {id} overfull: {}", items.len());
                    let mut s = Sum::default();
                    for it in items {
                        assert!(it.len >= 1, "empty run in leaf {id}");
                        s += it.sum();
                        for k in 0..it.len {
                            out.push(it.lv_start + k);
                        }
                    }
                    assert_eq!(tree.nodes[id].sum, s, "leaf {id} cached sum");
                    (s, 1)
                }
                Kind::Internal(children) => {
                    assert!(!children.is_empty(), "empty internal {id}");
                    assert!(children.len() <= MAX, "internal {id} overfull");
                    let mut s = Sum::default();
                    let mut height = None;
                    for &c in children {
                        let (cs, ch) = walk(tree, c, Some(id), out);
                        s += cs;
                        match height {
                            None => height = Some(ch),
                            Some(h) => assert_eq!(h, ch, "unbalanced height under {id}"),
                        }
                    }
                    assert_eq!(tree.nodes[id].sum, s, "internal {id} cached sum");
                    (s, height.unwrap() + 1)
                }
            }
        }
        let mut out = Vec::new();
        let (_, h) = walk(tree, tree.root, None, &mut out);
        (out, h)
    }

    #[test]
    fn empty_tree() {
        let tree: ContentTree<TestRun> = ContentTree::new();
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
        let (items, h) = check(&tree);
        assert!(items.is_empty());
        assert_eq!(h, 1);
    }

    #[test]
    fn forward_run_collapses_to_one_run() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..200 {
            tree.insert(lv, TestRun::one(lv));
        }
        assert_eq!(tree.len(), 200);
        let (items, h) = check(&tree);
        assert_eq!(items, (0..200).collect::<Vec<_>>());
        assert_eq!(tree.run_count(), 1, "forward typing must be a single run");
        assert_eq!(h, 1, "single run => single leaf, height 1");
    }

    #[test]
    fn front_inserts_force_runs_and_splits() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..50 {
            tree.insert(0, TestRun::one(lv));
        }
        let (items, h) = check(&tree);
        assert_eq!(items, (0..50).rev().collect::<Vec<_>>());
        assert!(h > 1);
        assert_eq!(tree.run_count(), 50, "front inserts never coalesce");
    }

    #[test]
    fn insert_bridges_two_runs() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in [0, 1, 2] {
            tree.insert(tree.len(), TestRun::one(lv));
        }
        for lv in [4, 5, 6] {
            tree.insert(tree.len(), TestRun::one(lv));
        }
        assert_eq!(tree.run_count(), 2, "gap at 3 keeps two runs");
        tree.insert(3, TestRun::one(3));
        let (items, _) = check(&tree);
        assert_eq!(items, (0..7).collect::<Vec<_>>());
        assert_eq!(tree.run_count(), 1, "bridging lv 3 coalesces into one run");
    }

    #[test]
    fn mid_run_insert_splits() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..6 {
            tree.insert(lv, TestRun::one(lv));
        }
        assert_eq!(tree.run_count(), 1);
        tree.insert(3, TestRun::one(100));
        let (items, _) = check(&tree);
        assert_eq!(items, vec![0, 1, 2, 100, 3, 4, 5]);
        assert_eq!(tree.run_count(), 3, "non-contiguous mid insert => 3 runs");
    }

    #[test]
    fn get_and_doc_index_are_inverse_with_runs() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        let mut expected: Vec<usize> = Vec::new();
        for lv in 0..80 {
            let pos = (lv * 7 + 3) % (expected.len() + 1);
            tree.insert(pos, TestRun::one(lv));
            expected.insert(pos, lv);
        }
        check(&tree);
        assert_eq!(tree.len(), expected.len());
        for (i, &lv) in expected.iter().enumerate() {
            let (run, off) = tree.get(i);
            assert_eq!(run.lv_start + off, lv, "get({i})");
            assert_eq!(tree.doc_index_of_lv(lv), i, "doc_index_of_lv({lv})");
        }
    }

    #[test]
    fn update_by_lv_splits_run() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..10 {
            tree.insert(lv, TestRun::one(lv));
        }
        assert_eq!(tree.run_count(), 1);
        tree.update_by_lv(4, |it| {
            it.cur_vis = false;
            it.end_vis = false;
        });
        let (items, _) = check(&tree);
        assert_eq!(items, (0..10).collect::<Vec<_>>());
        assert_eq!(tree.run_count(), 3, "isolating lv 4 splits into 3 runs");
        assert_eq!(tree.nodes[tree.root].sum.count, 10);
        assert_eq!(tree.nodes[tree.root].sum.cur, 9);
        assert_eq!(tree.nodes[tree.root].sum.end, 9);
        assert_eq!(tree.doc_index_of_lv(4), 4);
    }

    #[test]
    fn update_by_lv_head_of_run() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..6 {
            tree.insert(lv, TestRun::one(lv));
        }
        tree.update_by_lv(0, |it| it.cur_vis = false);
        let (items, _) = check(&tree);
        assert_eq!(items, (0..6).collect::<Vec<_>>());
        assert_eq!(tree.run_count(), 2, "head split => [0] [1..6]");
        assert_eq!(tree.nodes[tree.root].sum.cur, 5);
        assert_eq!(tree.nodes[tree.root].sum.end, 6);
    }

    #[test]
    fn update_by_lv_many_after_splits() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..70 {
            tree.insert(lv, TestRun::one(lv));
        }
        let mut hidden = 0;
        for lv in (0..70).step_by(3) {
            tree.update_by_lv(lv, |it| {
                it.cur_vis = false;
                it.end_vis = false;
            });
            hidden += 1;
        }
        check(&tree);
        assert_eq!(tree.nodes[tree.root].sum.cur, 70 - hidden);
        assert_eq!(tree.nodes[tree.root].sum.end, 70 - hidden);
        for lv in 0..70 {
            assert_eq!(tree.doc_index_of_lv(lv), lv);
        }
    }

    #[test]
    fn cursor_at_cur_pos_skips_invisible_runs() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..60 {
            tree.insert(lv, TestRun::one(lv));
        }
        for lv in (0..60).step_by(2) {
            tree.update_by_lv(lv, |it| it.cur_vis = false);
        }
        check(&tree);
        let elem_lv = |idx: usize| {
            let (run, off) = tree.get(idx);
            run.lv_start + off
        };
        let cur_vis_before = |idx: usize| (0..idx).filter(|i| elem_lv(*i) % 2 == 1).count();
        let total_vis = (0..60).filter(|lv| lv % 2 == 1).count();
        for target in 0..=total_vis {
            let c = tree.cursor_at_cur_pos(target);
            assert_eq!(
                cur_vis_before(c.doc_idx),
                target,
                "visible prefix at cursor({target})"
            );
            if target > 0 {
                assert!(
                    elem_lv(c.doc_idx - 1) % 2 == 1,
                    "cursor({target}) not leftmost"
                );
            }
        }
        let c = tree.cursor_at_cur_pos(total_vis);
        assert_eq!(c.doc_idx, tree.len());
    }

    #[test]
    fn cursor_end_pos_counts_end_visible_prefix() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..40 {
            tree.insert(lv, TestRun::one(lv));
        }
        for lv in 0..10 {
            tree.update_by_lv(lv, |it| it.cur_vis = false);
        }
        check(&tree);
        let c0 = tree.cursor_at_cur_pos(0);
        assert_eq!(c0.doc_idx, 0);
        assert_eq!(c0.end_pos, 0);
        let c1 = tree.cursor_at_cur_pos(1);
        assert_eq!(c1.doc_idx, 11);
        assert_eq!(c1.end_pos, 11);
        let end_vis = |idx: usize| {
            let (run, _) = tree.get(idx);
            run.end_vis
        };
        for target in 1..=30 {
            let c = tree.cursor_at_cur_pos(target);
            let end_before = (0..c.doc_idx).filter(|i| end_vis(*i)).count();
            assert_eq!(c.end_pos, end_before, "end_pos at cursor({target})");
        }
    }

    #[test]
    fn step_traverses_all_elements() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..55 {
            tree.insert(lv, TestRun::one(lv));
        }
        check(&tree);
        let mut c = tree.cursor_at_cur_pos(0);
        let mut seen = Vec::new();
        while let Some(run) = tree.cur_run(&c) {
            seen.push(run.lv_start + c.off);
            tree.step(&mut c);
        }
        assert_eq!(seen, (0..55).collect::<Vec<_>>());
        assert_eq!(c.doc_idx, 55);
    }

    #[test]
    fn step_run_jumps_whole_runs() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..30 {
            tree.insert(lv, TestRun::one(lv));
        }
        tree.update_by_lv(10, |it| it.cur_vis = false);
        tree.update_by_lv(20, |it| it.cur_vis = false);
        check(&tree);
        let mut c = tree.cursor_at_cur_pos(0);
        let mut total_skipped = 0;
        let mut end_acc = 0;
        while let Some(run) = tree.cur_run(&c) {
            let per_end = if run.run_len() == 0 {
                0
            } else {
                run.sum().end / run.run_len()
            };
            let before = c.off;
            let skipped = tree.step_run(&mut c);
            end_acc += per_end * skipped;
            total_skipped += skipped;
            assert_eq!(before, 0, "step_run should start a run at offset 0");
        }
        assert_eq!(total_skipped, 30, "every element visited once");
        assert_eq!(c.doc_idx, 30);
        assert_eq!(end_acc, 30, "all 30 are end-visible");
    }

    #[test]
    fn step_run_matches_step_end_pos() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..48 {
            tree.insert(lv, TestRun::one(lv));
        }
        for lv in (0..48).step_by(5) {
            tree.update_by_lv(lv, |it| it.end_vis = false);
        }
        check(&tree);
        let mut c1 = tree.cursor_at_cur_pos(0);
        let mut end1 = 0usize;
        while let Some(run) = tree.cur_run(&c1) {
            end1 += run.end_vis as usize;
            tree.step(&mut c1);
        }
        let mut c2 = tree.cursor_at_cur_pos(0);
        let mut end2 = 0usize;
        while let Some(run) = tree.cur_run(&c2) {
            let per_end = run.sum().end / run.run_len();
            let skipped = tree.step_run(&mut c2);
            end2 += per_end * skipped;
        }
        assert_eq!(end1, end2);
        assert_eq!(c1.doc_idx, c2.doc_idx);
    }

    #[test]
    fn end_rank_all_visible_is_index() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..50 {
            tree.insert(lv, TestRun::one(lv));
        }
        check(&tree);
        for i in 0..=tree.len() {
            assert_eq!(tree.end_rank_at_doc_index(i), i, "end_rank({i})");
        }
    }

    #[test]
    fn end_rank_matches_bruteforce_with_tombstones() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..80 {
            tree.insert(lv, TestRun::one(lv));
        }
        for lv in (0..80).step_by(3) {
            tree.update_by_lv(lv, |it| it.end_vis = false);
        }
        check(&tree);
        let end_vis = |idx: usize| {
            let (run, _) = tree.get(idx);
            run.end_vis
        };
        for i in 0..=tree.len() {
            let brute = (0..i).filter(|j| end_vis(*j)).count();
            assert_eq!(tree.end_rank_at_doc_index(i), brute, "end_rank({i})");
        }
        let total = (0..tree.len()).filter(|j| end_vis(*j)).count();
        assert_eq!(tree.end_rank_at_doc_index(tree.len()), total);
    }

    #[test]
    fn end_rank_leading_tombstones() {
        let mut tree: ContentTree<TestRun> = ContentTree::new();
        for lv in 0..30 {
            tree.insert(lv, TestRun::one(lv));
        }
        for lv in 0..10 {
            tree.update_by_lv(lv, |it| it.end_vis = false);
        }
        check(&tree);
        for i in 0..=10 {
            assert_eq!(tree.end_rank_at_doc_index(i), 0, "leading rank({i})");
        }
        for i in 10..=30 {
            assert_eq!(tree.end_rank_at_doc_index(i), i - 10, "suffix rank({i})");
        }
    }
}

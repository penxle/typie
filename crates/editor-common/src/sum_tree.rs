//! Persistent order-statistics balanced tree with cached subtree aggregates.
//!
//! A height-balanced (AVL) tree of items, each carrying a `size` of type `S`
//! (an additive measure — `u64` flat offsets, `f32` layout heights, …). Built
//! from reference-counted nodes so it is **persistent**: cloning the tree is
//! `O(1)` (a shared root pointer) and every mutation copies only the `O(log N)`
//! nodes on the root path, leaving clones untouched. That makes it the
//! structurally-correct editing primitive — available in `O(log N)` regardless
//! of edit position, and cheap to keep behind a doc's shared cache:
//!
//! - `len` / `total_size` — root aggregate, `O(1)`.
//! - `offset_before(index)` — cumulative size of items `[0, index)`, `O(log N)`.
//! - `find_by_offset(offset)` — the item spanning a cumulative offset, `O(log N)`.
//! - `set_size` / `insert` / `remove` — re-fold aggregates along the (copied)
//!   root path, `O(log N)` independent of position.
//!
//! Shared foundation for incremental flat layout (`S = u64`), the measured-tree
//! children, and pagination (`S = f32`). See the editing-pipeline design spec.

use std::sync::Arc;

/// An additive size measure cached by [`SumTree`]. Implemented for all numeric
/// types that are `Copy`, have a zero (`Default`), and support `+`/`-`/ordering.
pub trait SumSize:
    Copy + Default + std::ops::Add<Output = Self> + std::ops::Sub<Output = Self> + PartialOrd
{
}

impl<T> SumSize for T where
    T: Copy + Default + std::ops::Add<Output = T> + std::ops::Sub<Output = T> + PartialOrd
{
}

type Link<T, S> = Option<Arc<Node<T, S>>>;

#[derive(Clone)]
struct Node<T, S> {
    item: T,
    size: S,
    left: Link<T, S>,
    right: Link<T, S>,
    height: i32,
    count: usize,
    subtree_size: S,
}

fn height<T, S>(link: &Link<T, S>) -> i32 {
    link.as_ref().map_or(0, |n| n.height)
}

fn count<T, S>(link: &Link<T, S>) -> usize {
    link.as_ref().map_or(0, |n| n.count)
}

fn subtree_size<T, S: SumSize>(link: &Link<T, S>) -> S {
    link.as_ref().map_or_else(S::default, |n| n.subtree_size)
}

fn refresh<T, S: SumSize>(node: &mut Node<T, S>) {
    node.height = 1 + height(&node.left).max(height(&node.right));
    node.count = 1 + count(&node.left) + count(&node.right);
    node.subtree_size = node.size + subtree_size(&node.left) + subtree_size(&node.right);
}

fn balance_factor<T, S>(node: &Node<T, S>) -> i32 {
    height(&node.left) - height(&node.right)
}

/// Owned copy of the node (children `Arc`s are shared, so `O(1)`). Path-copying
/// these on the way down is what makes mutation persistent.
fn owned<T: Clone, S: Clone>(node: &Arc<Node<T, S>>) -> Node<T, S> {
    (**node).clone()
}

fn rotate_right<T: Clone, S: SumSize>(node: &Arc<Node<T, S>>) -> Arc<Node<T, S>> {
    let mut node = owned(node);
    let left = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    let mut left = owned(&left);
    node.left = left.right.take();
    refresh(&mut node);
    left.right = Some(Arc::new(node));
    refresh(&mut left);
    Arc::new(left)
}

fn rotate_left<T: Clone, S: SumSize>(node: &Arc<Node<T, S>>) -> Arc<Node<T, S>> {
    let mut node = owned(node);
    let right = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    let mut right = owned(&right);
    node.right = right.left.take();
    refresh(&mut node);
    right.left = Some(Arc::new(node));
    refresh(&mut right);
    Arc::new(right)
}

fn rebalance<T: Clone, S: SumSize>(mut node: Node<T, S>) -> Arc<Node<T, S>> {
    refresh(&mut node);
    let bf = balance_factor(&node);
    if bf > 1 {
        if balance_factor(node.left.as_ref().unwrap()) < 0 {
            let left = node.left.take().unwrap();
            node.left = Some(rotate_left(&left));
        }
        return rotate_right(&Arc::new(node));
    }
    if bf < -1 {
        if balance_factor(node.right.as_ref().unwrap()) > 0 {
            let right = node.right.take().unwrap();
            node.right = Some(rotate_right(&right));
        }
        return rotate_left(&Arc::new(node));
    }
    Arc::new(node)
}

fn insert_at<T: Clone, S: SumSize>(
    link: &Link<T, S>,
    index: usize,
    item: T,
    size: S,
) -> Arc<Node<T, S>> {
    let Some(node) = link else {
        return Arc::new(Node {
            item,
            size,
            left: None,
            right: None,
            height: 1,
            count: 1,
            subtree_size: size,
        });
    };
    let mut node = owned(node);
    let left_count = count(&node.left);
    if index <= left_count {
        node.left = Some(insert_at(&node.left, index, item, size));
    } else {
        node.right = Some(insert_at(&node.right, index - left_count - 1, item, size));
    }
    rebalance(node)
}

fn take_min<T: Clone, S: SumSize>(node: &Arc<Node<T, S>>) -> (T, S, Link<T, S>) {
    let mut node = owned(node);
    match node.left.take() {
        None => (node.item, node.size, node.right.take()),
        Some(left) => {
            let (item, size, rest) = take_min(&left);
            node.left = rest;
            (item, size, Some(rebalance(node)))
        }
    }
}

fn remove_at<T: Clone, S: SumSize>(
    link: &Link<T, S>,
    index: usize,
) -> (Link<T, S>, Option<(T, S)>) {
    let Some(node) = link else {
        return (None, None);
    };
    let left_count = count(&node.left);
    let mut node = owned(node);
    if index < left_count {
        let (new_left, removed) = remove_at(&node.left, index);
        node.left = new_left;
        (Some(rebalance(node)), removed)
    } else if index > left_count {
        let (new_right, removed) = remove_at(&node.right, index - left_count - 1);
        node.right = new_right;
        (Some(rebalance(node)), removed)
    } else {
        let removed = (node.item.clone(), node.size);
        let replacement = match (node.left.take(), node.right.take()) {
            (left, None) => left,
            (None, right) => right,
            (Some(left), Some(right)) => {
                let (succ_item, succ_size, rest) = take_min(&right);
                let succ = Node {
                    item: succ_item,
                    size: succ_size,
                    left: Some(left),
                    right: rest,
                    height: 1,
                    count: 1,
                    subtree_size: succ_size,
                };
                Some(rebalance(succ))
            }
        };
        (replacement, Some(removed))
    }
}

fn set_size_at<T: Clone, S: SumSize>(
    link: &Link<T, S>,
    index: usize,
    new_size: S,
) -> (Link<T, S>, bool) {
    let Some(node) = link else {
        return (None, false);
    };
    let mut node = owned(node);
    let left_count = count(&node.left);
    let ok = if index < left_count {
        let (new_left, ok) = set_size_at(&node.left, index, new_size);
        node.left = new_left;
        ok
    } else if index > left_count {
        let (new_right, ok) = set_size_at(&node.right, index - left_count - 1, new_size);
        node.right = new_right;
        ok
    } else {
        node.size = new_size;
        true
    };
    refresh(&mut node);
    (Some(Arc::new(node)), ok)
}

fn set_at<T: Clone, S: SumSize>(
    link: &Link<T, S>,
    index: usize,
    item: T,
    size: S,
) -> (Link<T, S>, bool) {
    let Some(node) = link else {
        return (None, false);
    };
    let mut node = owned(node);
    let left_count = count(&node.left);
    let ok = if index < left_count {
        let (new_left, ok) = set_at(&node.left, index, item, size);
        node.left = new_left;
        ok
    } else if index > left_count {
        let (new_right, ok) = set_at(&node.right, index - left_count - 1, item, size);
        node.right = new_right;
        ok
    } else {
        node.item = item;
        node.size = size;
        true
    };
    refresh(&mut node);
    (Some(Arc::new(node)), ok)
}

fn get_at<T, S>(link: &Link<T, S>, index: usize) -> Option<&T> {
    let node = link.as_ref()?;
    let left_count = count(&node.left);
    match index.cmp(&left_count) {
        std::cmp::Ordering::Less => get_at(&node.left, index),
        std::cmp::Ordering::Equal => Some(&node.item),
        std::cmp::Ordering::Greater => get_at(&node.right, index - left_count - 1),
    }
}

fn offset_before<T, S: SumSize>(link: &Link<T, S>, index: usize) -> S {
    let Some(node) = link.as_ref() else {
        return S::default();
    };
    let left_count = count(&node.left);
    if index <= left_count {
        offset_before(&node.left, index)
    } else {
        subtree_size(&node.left) + node.size + offset_before(&node.right, index - left_count - 1)
    }
}

fn find_by_offset<T, S: SumSize>(link: &Link<T, S>, offset: S) -> Option<(usize, S)> {
    let node = link.as_ref()?;
    let left_size = subtree_size(&node.left);
    if offset < left_size {
        return find_by_offset(&node.left, offset);
    }
    let here = offset - left_size;
    if here < node.size {
        return Some((count(&node.left), here));
    }
    find_by_offset(&node.right, here - node.size)
        .map(|(i, within)| (count(&node.left) + 1 + i, within))
}

/// `find_by_offset` on one projected `u64` dimension of a compound size.
/// `project` must be linear over `S`'s addition (a component read).
fn find_by_projected_offset<T, S: SumSize>(
    link: &Link<T, S>,
    offset: u64,
    project: &impl Fn(&S) -> u64,
) -> Option<(usize, u64)> {
    let node = link.as_ref()?;
    let left = project(&subtree_size(&node.left));
    if offset < left {
        return find_by_projected_offset(&node.left, offset, project);
    }
    let here = offset - left;
    let own = project(&node.size);
    if here < own {
        return Some((count(&node.left), here));
    }
    find_by_projected_offset(&node.right, here - own, project)
        .map(|(i, within)| (count(&node.left) + 1 + i, within))
}

fn for_each<T, S: Copy>(link: &Link<T, S>, f: &mut impl FnMut(&T, S)) {
    if let Some(node) = link {
        for_each(&node.left, f);
        f(&node.item, node.size);
        for_each(&node.right, f);
    }
}

fn for_each_in_range<T, S: SumSize>(
    link: &Link<T, S>,
    base: S,
    start: S,
    end: S,
    f: &mut impl FnMut(S, &T, S),
) {
    let Some(node) = link else {
        return;
    };
    if base >= end || base + node.subtree_size <= start {
        return;
    }
    for_each_in_range(&node.left, base, start, end, f);
    let item_start = base + subtree_size(&node.left);
    if item_start < end && item_start + node.size > start {
        f(item_start, &node.item, node.size);
    }
    for_each_in_range(&node.right, item_start + node.size, start, end, f);
}

fn build_balanced<T: Clone, S: SumSize>(items: &[(T, S)]) -> Link<T, S> {
    if items.is_empty() {
        return None;
    }
    let mid = items.len() / 2;
    let (item, size) = items[mid].clone();
    let left = build_balanced(&items[..mid]);
    let right = build_balanced(&items[mid + 1..]);
    let mut node = Node {
        item,
        size,
        left,
        right,
        height: 1,
        count: 1,
        subtree_size: size,
    };
    refresh(&mut node);
    Some(Arc::new(node))
}

/// A persistent order-statistics balanced tree; see the module docs. `Clone` is
/// `O(1)` (shares the root); mutations copy only the `O(log N)` root path and do
/// not affect existing clones. `S` is the cached size measure (defaults to `u64`).
pub struct SumTree<T, S = u64> {
    root: Link<T, S>,
}

impl<T, S> Clone for SumTree<T, S> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<T, S> Default for SumTree<T, S> {
    fn default() -> Self {
        Self { root: None }
    }
}

impl<T, S: SumSize + std::fmt::Debug> std::fmt::Debug for SumTree<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SumTree")
            .field("len", &count(&self.root))
            .field("total_size", &subtree_size(&self.root))
            .finish()
    }
}

impl<T: Clone, S: SumSize> SumTree<T, S> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a balanced tree from an already-ordered list in `O(n)`.
    pub fn from_items(items: Vec<(T, S)>) -> Self {
        Self {
            root: build_balanced(&items),
        }
    }

    pub fn len(&self) -> usize {
        count(&self.root)
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Total size cached at the root — `O(1)`.
    pub fn total_size(&self) -> S {
        subtree_size(&self.root)
    }

    /// Inserts `item` (with `size`) at position `index` (clamped to `len`).
    pub fn insert(&mut self, index: usize, item: T, size: S) {
        let index = index.min(self.len());
        self.root = Some(insert_at(&self.root, index, item, size));
    }

    /// Appends `item` at the end.
    pub fn push(&mut self, item: T, size: S) {
        let index = self.len();
        self.insert(index, item, size);
    }

    /// Removes the item at `index`, returning `(item, size)` if in range.
    pub fn remove(&mut self, index: usize) -> Option<(T, S)> {
        if index >= self.len() {
            return None;
        }
        let (new_root, removed) = remove_at(&self.root, index);
        self.root = new_root;
        removed
    }

    /// Updates the size of the item at `index`. Returns `false` if out of range.
    pub fn set_size(&mut self, index: usize, size: S) -> bool {
        let (new_root, ok) = set_size_at(&self.root, index, size);
        if ok {
            self.root = new_root;
        }
        ok
    }

    /// Replaces both the item and its size at `index`. Returns `false` if out of
    /// range. `O(log N)` (path copy), used to swap a re-measured child in place.
    pub fn set(&mut self, index: usize, item: T, size: S) -> bool {
        let (new_root, ok) = set_at(&self.root, index, item, size);
        if ok {
            self.root = new_root;
        }
        ok
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        get_at(&self.root, index)
    }

    /// Cumulative size of items `[0, index)` — `O(log N)`.
    pub fn offset_before(&self, index: usize) -> S {
        offset_before(&self.root, index)
    }

    /// `(index, offset_within_item)` for the item spanning cumulative `offset`,
    /// or `None` if `offset >= total_size`. A boundary belongs to the later item.
    pub fn find_by_offset(&self, offset: S) -> Option<(usize, S)> {
        find_by_offset(&self.root, offset)
    }

    /// `find_by_offset` on one projected `u64` dimension of a compound size —
    /// for a multi-component `S` (e.g. leaf count + flat width) where each
    /// dimension needs an independent descent. `S`'s own `PartialOrd` (if
    /// derived, lexicographic) is never used here.
    pub fn find_by_projected_offset(
        &self,
        offset: u64,
        project: impl Fn(&S) -> u64,
    ) -> Option<(usize, u64)> {
        find_by_projected_offset(&self.root, offset, &project)
    }

    /// In-order iteration, yielding `(item, size)`.
    pub fn for_each(&self, mut f: impl FnMut(&T, S)) {
        for_each(&self.root, &mut f);
    }

    /// Forward in-order iterator over `&item` — `O(1)` amortized per step.
    pub fn iter(&self) -> Iter<'_, T, S> {
        let mut stack = Vec::new();
        let mut cur = self.root.as_deref();
        while let Some(node) = cur {
            stack.push(node);
            cur = node.left.as_deref();
        }
        Iter {
            stack,
            remaining: count(&self.root),
        }
    }

    /// Visits every item overlapping the offset range `[start, end)`, passing
    /// `(item_start_offset, item, size)` — `O(visited + log N)`.
    pub fn for_each_in_range(&self, start: S, end: S, mut f: impl FnMut(S, &T, S)) {
        for_each_in_range(&self.root, S::default(), start, end, &mut f);
    }
}

/// Forward in-order iterator over a [`SumTree`]'s items.
pub struct Iter<'a, T, S> {
    stack: Vec<&'a Node<T, S>>,
    remaining: usize,
}

impl<'a, T, S> Iterator for Iter<'a, T, S> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        let node = self.stack.pop()?;
        self.remaining -= 1;
        let mut cur = node.right.as_deref();
        while let Some(n) = cur {
            self.stack.push(n);
            cur = n.left.as_deref();
        }
        Some(&node.item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T, S> ExactSizeIterator for Iter<'_, T, S> {}

impl<T: Clone, S: SumSize> FromIterator<(T, S)> for SumTree<T, S> {
    fn from_iter<I: IntoIterator<Item = (T, S)>>(iter: I) -> Self {
        let mut tree = SumTree::new();
        for (item, size) in iter {
            tree.push(item, size);
        }
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(tree: &SumTree<u32>) -> Vec<u32> {
        let mut out = Vec::new();
        tree.for_each(|item, _| out.push(*item));
        out
    }

    fn check_invariants(link: &Link<u32, u64>) -> (i32, usize, u64) {
        match link {
            None => (0, 0, 0),
            Some(node) => {
                let (lh, lc, ls) = check_invariants(&node.left);
                let (rh, rc, rs) = check_invariants(&node.right);
                assert!((lh - rh).abs() <= 1, "AVL balance violated");
                assert_eq!(node.height, 1 + lh.max(rh));
                assert_eq!(node.count, 1 + lc + rc);
                assert_eq!(node.subtree_size, node.size + ls + rs);
                (node.height, node.count, node.subtree_size)
            }
        }
    }

    #[test]
    fn push_keeps_order_and_aggregates() {
        let mut tree: SumTree<u32> = SumTree::new();
        for i in 0..100u32 {
            tree.push(i, i as u64 + 1);
        }
        check_invariants(&tree.root);
        assert_eq!(tree.len(), 100);
        assert_eq!(items(&tree), (0..100).collect::<Vec<_>>());
        assert_eq!(tree.total_size(), (1..=100).sum::<u64>());
    }

    #[test]
    fn insert_at_front_balances() {
        let mut tree: SumTree<u32> = SumTree::new();
        for i in 0..200u32 {
            tree.insert(0, i, 1);
        }
        check_invariants(&tree.root);
        assert_eq!(items(&tree), (0..200).rev().collect::<Vec<_>>());
        assert!(height(&tree.root) <= 16);
    }

    #[test]
    fn clone_is_independent_persistent() {
        let base: SumTree<u32> = (0..20u32).map(|i| (i, 1)).collect();
        let mut derived = base.clone();
        derived.set_size(5, 100);
        derived.insert(0, 999, 7);
        derived.remove(10);
        check_invariants(&base.root);
        assert_eq!(items(&base), (0..20).collect::<Vec<_>>());
        assert_eq!(base.total_size(), 20);
    }

    #[test]
    fn iter_yields_in_order() {
        let tree: SumTree<u32> = (0..64u32).map(|i| (i, 1)).collect();
        assert_eq!(
            tree.iter().copied().collect::<Vec<_>>(),
            (0..64).collect::<Vec<_>>()
        );
        assert!(tree.iter().any(|&x| x == 40));
        assert_eq!(tree.iter().position(|&x| x == 7), Some(7));
        assert_eq!(tree.iter().len(), 64);
    }

    #[test]
    fn from_items_builds_balanced_with_correct_aggregates() {
        let data: Vec<(u32, u64)> = (0..100u32).map(|i| (i, i as u64 + 1)).collect();
        let tree = SumTree::from_items(data);
        check_invariants(&tree.root);
        assert_eq!(items(&tree), (0..100).collect::<Vec<_>>());
        assert_eq!(tree.total_size(), (1..=100).sum::<u64>());
        assert!(height(&tree.root) <= 9);
    }

    #[test]
    fn for_each_in_range_visits_overlapping_items() {
        let tree: SumTree<u32> = (0..10u32).map(|i| (i, 2)).collect();
        let mut visited = Vec::new();
        tree.for_each_in_range(5, 11, |start, item, size| {
            visited.push((start, *item, size))
        });
        assert_eq!(visited, vec![(4, 2, 2), (6, 3, 2), (8, 4, 2), (10, 5, 2)]);
        let mut none = 0;
        tree.for_each_in_range(8, 8, |_, _, _| none += 1);
        assert_eq!(none, 0);
    }

    #[test]
    fn offset_before_and_find_by_offset() {
        let mut tree: SumTree<u32> = [(0u32, 2u64), (1, 3), (2, 5), (3, 1)].into_iter().collect();
        assert_eq!(tree.total_size(), 11);
        assert_eq!(tree.offset_before(2), 5);
        assert_eq!(tree.find_by_offset(2), Some((1, 0)));
        assert_eq!(tree.find_by_offset(4), Some((1, 2)));
        assert_eq!(tree.find_by_offset(11), None);
        assert!(tree.set_size(1, 10));
        assert_eq!(tree.total_size(), 18);
        assert_eq!(tree.offset_before(2), 12);
    }

    #[test]
    fn works_with_f32_sizes_for_layout_heights() {
        // The measured-tree / pagination use case: f32 height aggregates.
        let mut tree: SumTree<&'static str, f32> = SumTree::new();
        tree.push("a", 10.0);
        tree.push("b", 20.0);
        tree.push("c", 5.0);
        assert_eq!(tree.total_size(), 35.0);
        assert_eq!(tree.offset_before(2), 30.0);
        assert_eq!(tree.find_by_offset(15.0), Some((1, 5.0)));
        // Patch a child's height in O(log N); the total re-folds.
        assert!(tree.set_size(0, 100.0));
        assert_eq!(tree.total_size(), 125.0);
        assert_eq!(tree.offset_before(1), 100.0);
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
    struct Pair {
        a: u64,
        b: u64,
    }

    impl std::ops::Add for Pair {
        type Output = Self;
        fn add(self, o: Self) -> Self {
            Self {
                a: self.a + o.a,
                b: self.b + o.b,
            }
        }
    }
    impl std::ops::Sub for Pair {
        type Output = Self;
        fn sub(self, o: Self) -> Self {
            Self {
                a: self.a - o.a,
                b: self.b - o.b,
            }
        }
    }

    #[test]
    fn find_by_projected_offset_disagrees_with_lexicographic_find_by_offset() {
        let items: Vec<(u32, Pair)> = vec![
            (0, Pair { a: 1, b: 5 }),
            (1, Pair { a: 0, b: 3 }),
            (2, Pair { a: 2, b: 0 }),
        ];
        let tree: SumTree<u32, Pair> = SumTree::from_items(items);
        // A lexicographic `find_by_offset` on the raw pair would descend by `a`
        // first, then `b` — neither single-dimension answer below.
        assert_eq!(tree.find_by_projected_offset(1, |s| s.a), Some((2, 0)));
        assert_eq!(tree.find_by_projected_offset(6, |s| s.b), Some((1, 1)));
    }

    #[test]
    fn matches_naive_reference_under_random_ops() {
        let mut tree: SumTree<u32> = SumTree::new();
        let mut reference: Vec<(u32, u64)> = Vec::new();
        let mut state = 0x9e3779b97f4a7c15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for step in 0..2000u32 {
            let len = reference.len();
            let op = next() % 3;
            if op == 0 || len == 0 {
                let idx = (next() as usize) % (len + 1);
                let size = (next() % 9) + 1;
                tree.insert(idx, step, size);
                reference.insert(idx, (step, size));
            } else if op == 1 {
                let idx = (next() as usize) % len;
                let size = (next() % 9) + 1;
                tree.set_size(idx, size);
                reference[idx].1 = size;
            } else {
                let idx = (next() as usize) % len;
                assert_eq!(tree.remove(idx), Some(reference.remove(idx)));
            }
            check_invariants(&tree.root);
            assert_eq!(
                items(&tree),
                reference.iter().map(|(i, _)| *i).collect::<Vec<_>>()
            );
            let total: u64 = reference.iter().map(|(_, s)| s).sum();
            assert_eq!(tree.total_size(), total);
            let mut acc = 0u64;
            for (i, (_, s)) in reference.iter().enumerate() {
                assert_eq!(tree.offset_before(i), acc);
                assert_eq!(tree.find_by_offset(acc), Some((i, 0)));
                acc += s;
            }
            assert_eq!(tree.find_by_offset(acc), None);
        }
    }
}

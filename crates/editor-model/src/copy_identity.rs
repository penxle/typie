use crate::{AliasClasses, RawChild, RawNode, RawTree, SeqItem};
use editor_crdt::{Dot, FastMap, FastSet};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HiddenCopies {
    roots: FastMap<Dot, Vec<Dot>>,
    all: FastSet<Dot>,
}

impl HiddenCopies {
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn contains(&self, d: Dot) -> bool {
        self.all.contains(&d)
    }

    pub fn dots_of_root(&self, root: Dot) -> Option<&[Dot]> {
        self.roots.get(&root).map(|v| v.as_slice())
    }

    pub fn roots(&self) -> impl Iterator<Item = Dot> + '_ {
        self.roots.keys().copied()
    }

    pub fn insert_root(&mut self, root: Dot, dots: Vec<Dot>) {
        self.remove_root(root);
        debug_assert!(dots.contains(&root));
        debug_assert!(
            dots.iter().all(|d| !self.all.contains(d)),
            "hidden closures are disjoint"
        );
        for d in &dots {
            self.all.insert(*d);
        }
        self.roots.insert(root, dots);
    }

    pub fn remove_root(&mut self, root: Dot) {
        if let Some(dots) = self.roots.remove(&root) {
            for d in dots {
                self.all.remove(&d);
            }
        }
    }

    pub fn extend(&mut self, other: HiddenCopies) {
        for (root, dots) in other.roots {
            self.insert_root(root, dots);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    Absent,
    Present,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Escalate {
    pub dot: Dot,
}

fn is_unit_leaf(item: &SeqItem) -> bool {
    matches!(item, SeqItem::Atom(_) | SeqItem::BlockAtom { .. })
}

fn index_units<'a>(
    node: &'a RawNode,
    nearest: Option<Dot>,
    units: &mut Vec<Dot>,
    nodes: &mut FastMap<Dot, &'a RawNode>,
    parent: &mut FastMap<Dot, Option<Dot>>,
) {
    for c in &node.children {
        match c {
            RawChild::Block(b) => {
                units.push(b.id);
                nodes.insert(b.id, b);
                parent.insert(b.id, nearest);
                index_units(b, Some(b.id), units, nodes, parent);
            }
            RawChild::Leaf { id, item } if is_unit_leaf(item) => {
                units.push(*id);
                parent.insert(*id, nearest);
            }
            RawChild::Leaf { .. } => {}
        }
    }
}

fn collect_all_dots(node: &RawNode, out: &mut Vec<Dot>) {
    out.push(node.id);
    for c in &node.children {
        match c {
            RawChild::Block(b) => collect_all_dots(b, out),
            RawChild::Leaf { id, .. } => out.push(*id),
        }
    }
}

fn detach_losers(node: &mut RawNode, losers: &FastSet<Dot>, hidden: &mut HiddenCopies) {
    let children = std::mem::take(&mut node.children);
    for c in children {
        match c {
            RawChild::Block(mut b) => {
                if losers.contains(&b.id) {
                    let mut dots = Vec::new();
                    collect_all_dots(&b, &mut dots);
                    hidden.insert_root(b.id, dots);
                } else {
                    detach_losers(&mut b, losers, hidden);
                    node.children.push(RawChild::Block(b));
                }
            }
            RawChild::Leaf { id, item } => {
                if losers.contains(&id) {
                    hidden.insert_root(id, vec![id]);
                } else {
                    node.children.push(RawChild::Leaf { id, item });
                }
            }
        }
    }
}

fn is_settled(
    m: Dot,
    parent: &FastMap<Dot, Option<Dot>>,
    classes: &AliasClasses,
    decided: &FastMap<Dot, Dot>,
) -> bool {
    let mut cur = parent.get(&m).copied().flatten();
    while let Some(a) = cur {
        if let Some(members) = classes.members_of(a)
            && !decided.contains_key(&members[0])
        {
            return false;
        }
        cur = parent.get(&a).copied().flatten();
    }
    true
}

fn mark_loser(
    x: Dot,
    root_ids: &[Dot],
    nodes: &FastMap<Dot, &RawNode>,
    losers: &mut Vec<Dot>,
    present: &mut FastSet<Dot>,
) -> Result<(), Escalate> {
    if root_ids.contains(&x) {
        return Err(Escalate { dot: x });
    }
    losers.push(x);
    let mut dots = Vec::new();
    match nodes.get(&x) {
        Some(node) => collect_all_dots(node, &mut dots),
        None => dots.push(x),
    }
    for d in dots {
        present.remove(&d);
    }
    Ok(())
}

pub fn hide_losers(
    raw: &mut RawTree,
    classes: &AliasClasses,
    outside: &dyn Fn(Dot) -> Outside,
) -> Result<HiddenCopies, Escalate> {
    let mut hidden = HiddenCopies::default();
    if classes.is_empty() {
        return Ok(hidden);
    }
    let root_ids: Vec<Dot> = raw.roots.iter().map(|r| r.id).collect();
    let mut units: Vec<Dot> = Vec::new();
    let mut nodes: FastMap<Dot, &RawNode> = FastMap::default();
    let mut parent: FastMap<Dot, Option<Dot>> = FastMap::default();
    for root in &raw.roots {
        let nearest = if root.id.is_synthetic() {
            None
        } else {
            units.push(root.id);
            nodes.insert(root.id, root);
            parent.insert(root.id, None);
            Some(root.id)
        };
        index_units(root, nearest, &mut units, &mut nodes, &mut parent);
    }
    let in_forest: FastSet<Dot> = units.iter().copied().collect();
    let mut present: FastSet<Dot> = in_forest.clone();
    let mut decided: FastMap<Dot, Dot> = FastMap::default();
    let mut losers: Vec<Dot> = Vec::new();
    // A winner is chosen only under ancestors that are themselves winners, and every other
    // present member is marked at once, so no class ever loses its last visible copy.
    for &u in &units {
        if !present.contains(&u) {
            continue;
        }
        let Some(members) = classes.members_of(u) else {
            continue;
        };
        let rep = members[0];
        if let Some(&winner) = decided.get(&rep) {
            debug_assert_eq!(u, winner, "a decided class keeps only its winner present");
            continue;
        }
        let mut candidates: Vec<Dot> = Vec::new();
        for &m in members {
            if present.contains(&m) {
                if is_settled(m, &parent, classes, &decided) {
                    candidates.push(m);
                }
            } else if !in_forest.contains(&m) && outside(m) == Outside::Present {
                return Err(Escalate { dot: m });
            }
        }
        let winner = candidates
            .iter()
            .copied()
            .max()
            .expect("the visited unit is settled and present");
        decided.insert(rep, winner);
        for &m in members {
            if present.contains(&m) && m != winner {
                mark_loser(m, &root_ids, &nodes, &mut losers, &mut present)?;
            }
        }
    }
    drop(nodes);
    let losers: FastSet<Dot> = losers.into_iter().collect();
    for root in &mut raw.roots {
        detach_losers(root, &losers, &mut hidden);
    }
    Ok(hidden)
}

fn is_redirectable_leaf(item: &SeqItem) -> bool {
    matches!(item, SeqItem::Char(_) | SeqItem::Atom(_))
}

/// `in_dead_block` decides whether a candidate leaf was typed INSIDE a copy that is
/// gone: its enclosing block marker is dead or hidden. Without it a char whose origin
/// is an inline atom that merely moved within a live paragraph would chase the atom
/// out of the block it still belongs to.
pub fn redirect_anchors(
    elements: &[(Dot, SeqItem)],
    origins: &[Option<Dot>],
    classes: &AliasClasses,
    is_dead: &dyn Fn(Dot) -> bool,
    is_hidden: &dyn Fn(Dot) -> bool,
    in_dead_block: &dyn Fn(Dot) -> bool,
) -> Vec<(Dot, Dot)> {
    if classes.is_empty() {
        return Vec::new();
    }
    debug_assert_eq!(elements.len(), origins.len());
    let mut redirected: FastSet<Dot> = FastSet::default();
    let mut out = Vec::new();
    for ((dot, item), origin) in elements.iter().zip(origins) {
        if !is_redirectable_leaf(item) || classes.contains(*dot) {
            continue;
        }
        let Some(o) = origin else {
            continue;
        };
        let dead_or_hidden_member = classes.contains(*o) && (is_dead(*o) || is_hidden(*o));
        if (dead_or_hidden_member || redirected.contains(o)) && in_dead_block(*dot) {
            redirected.insert(*dot);
            out.push((*dot, *o));
        }
    }
    out
}

#[derive(Clone, Copy)]
enum Slot {
    After(Dot),
    FirstOf(Dot),
}

fn index_forest(node: &RawNode, blocks: &mut FastSet<Dot>, leaves: &mut FastSet<Dot>) {
    for c in &node.children {
        match c {
            RawChild::Block(b) => {
                blocks.insert(b.id);
                index_forest(b, blocks, leaves);
            }
            RawChild::Leaf { id, .. } => {
                leaves.insert(*id);
            }
        }
    }
}

fn take_leaves(node: &mut RawNode, wanted: &FastSet<Dot>, taken: &mut FastMap<Dot, SeqItem>) {
    let children = std::mem::take(&mut node.children);
    for c in children {
        match c {
            RawChild::Block(mut b) => {
                take_leaves(&mut b, wanted, taken);
                node.children.push(RawChild::Block(b));
            }
            RawChild::Leaf { id, item } if wanted.contains(&id) => {
                taken.insert(id, item);
            }
            other => node.children.push(other),
        }
    }
}

fn emit_followers(
    id: Dot,
    follow: &FastMap<Dot, Vec<Dot>>,
    taken: &FastMap<Dot, SeqItem>,
    out: &mut Vec<RawChild>,
) {
    let Some(first) = follow.get(&id) else {
        return;
    };
    // A chain is as long as the typing run behind a dead member, so it is walked, not recursed.
    let mut stack: Vec<Dot> = first.iter().rev().copied().collect();
    while let Some(f) = stack.pop() {
        let Some(item) = taken.get(&f) else {
            continue;
        };
        out.push(RawChild::Leaf {
            id: f,
            item: item.clone(),
        });
        if let Some(next) = follow.get(&f) {
            stack.extend(next.iter().rev().copied());
        }
    }
}

fn put_leaves(
    node: &mut RawNode,
    follow: &FastMap<Dot, Vec<Dot>>,
    first_of: &FastMap<Dot, Vec<Dot>>,
    taken: &FastMap<Dot, SeqItem>,
) {
    let children = std::mem::take(&mut node.children);
    let mut rebuilt: Vec<RawChild> = Vec::with_capacity(children.len());
    if let Some(firsts) = first_of.get(&node.id) {
        for &f in firsts {
            if let Some(item) = taken.get(&f) {
                rebuilt.push(RawChild::Leaf {
                    id: f,
                    item: item.clone(),
                });
                emit_followers(f, follow, taken, &mut rebuilt);
            }
        }
    }
    for c in children {
        let id = match &c {
            RawChild::Block(b) => b.id,
            RawChild::Leaf { id, .. } => *id,
        };
        match c {
            RawChild::Block(mut b) => {
                put_leaves(&mut b, follow, first_of, taken);
                rebuilt.push(RawChild::Block(b));
            }
            leaf => rebuilt.push(leaf),
        }
        emit_followers(id, follow, taken, &mut rebuilt);
    }
    node.children = rebuilt;
}

fn image_in_window(
    d: Dot,
    classes: &AliasClasses,
    in_forest: impl Fn(Dot) -> bool,
    outside: &dyn Fn(Dot) -> Outside,
) -> Result<Option<Dot>, Escalate> {
    let image = classes.resolve_with(d, &in_forest);
    // A copy that lives outside the window decides the slot when none is inside, and
    // outranks the one inside when it sorts higher — `resolve_with` takes the greatest
    // visible member, so a partial forest can otherwise settle on a different copy than
    // a whole-document rebuild.
    for &m in classes.members_of(d).unwrap_or(&[]) {
        if !in_forest(m) && outside(m) == Outside::Present && (image == d || m > image) {
            return Err(Escalate { dot: m });
        }
    }
    if image != d {
        return Ok(Some(image));
    }
    Ok(None)
}

/// Returns the `(leaf, slot target)` pairs actually moved — a leaf left in place is
/// not listed. A chained leaf's target is its own anchor leaf, not the image that
/// anchor resolved to; consumers read the key and value sets, not the relation.
pub fn place_redirects(
    raw: &mut RawTree,
    redirects: &[(Dot, Dot)],
    classes: &AliasClasses,
    prev: &dyn Fn(Dot) -> Option<(Dot, bool)>,
    outside: &dyn Fn(Dot) -> Outside,
) -> Result<Vec<(Dot, Dot)>, Escalate> {
    debug_assert!(
        {
            let mut seen: FastSet<Dot> = FastSet::default();
            redirects
                .iter()
                .all(|(leaf, _)| seen.insert(*leaf).is_none())
        },
        "a leaf is redirected at most once"
    );
    if redirects.is_empty() {
        return Ok(Vec::new());
    }
    let mut blocks: FastSet<Dot> = FastSet::default();
    let mut leaves: FastSet<Dot> = FastSet::default();
    for root in &raw.roots {
        if !root.id.is_synthetic() {
            blocks.insert(root.id);
        }
        index_forest(root, &mut blocks, &mut leaves);
    }
    let in_forest = |d: Dot| blocks.contains(&d) || leaves.contains(&d);
    let redirect_set: FastSet<Dot> = redirects.iter().map(|(leaf, _)| *leaf).collect();

    let mut follow: FastMap<Dot, Vec<Dot>> = FastMap::default();
    let mut first_of: FastMap<Dot, Vec<Dot>> = FastMap::default();
    let mut placed: FastSet<Dot> = FastSet::default();
    let mut moved: Vec<(Dot, Dot)> = Vec::new();
    for &(leaf, anchor) in redirects {
        let slot = if redirect_set.contains(&anchor) {
            // A follower moves only if its anchor did — an anchor left at its own
            // sequence slot already has the follower right behind it. And an anchor
            // this forest never shows would carry its followers out of the tree.
            if !placed.contains(&anchor) || !in_forest(anchor) {
                continue;
            }
            Slot::After(anchor)
        } else {
            let target = match image_in_window(anchor, classes, in_forest, outside)? {
                Some(image) => Some(image),
                None => {
                    let mut cur = anchor;
                    loop {
                        let Some((p, visible)) = prev(cur) else {
                            break None;
                        };
                        if visible && in_forest(p) {
                            break None;
                        }
                        if classes.contains(p)
                            && let Some(image) = image_in_window(p, classes, in_forest, outside)?
                        {
                            break Some(image);
                        }
                        cur = p;
                    }
                }
            };
            let Some(target) = target else {
                continue;
            };
            if blocks.contains(&target) {
                Slot::FirstOf(target)
            } else {
                Slot::After(target)
            }
        };
        placed.insert(leaf);
        match slot {
            Slot::After(t) => {
                moved.push((leaf, t));
                follow.entry(t).or_default().push(leaf);
            }
            Slot::FirstOf(b) => {
                moved.push((leaf, b));
                first_of.entry(b).or_default().push(leaf);
            }
        }
    }
    if placed.is_empty() {
        return Ok(moved);
    }
    let mut taken: FastMap<Dot, SeqItem> = FastMap::default();
    for root in &mut raw.roots {
        take_leaves(root, &placed, &mut taken);
    }
    for root in &mut raw.roots {
        put_leaves(root, &follow, &first_of, &taken);
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AliasLog, AliasOp, AliasRun, AtomLeaf, NodeType, RawChild, RawNode, RawTree, SeqItem,
    };

    fn run(old: (u64, u64), len: u32, new: (u64, u64)) -> AliasRun {
        AliasRun {
            old_start: Dot::new(old.0, old.1),
            len,
            new_start: Dot::new(new.0, new.1),
        }
    }

    fn classes(runs: Vec<AliasRun>) -> AliasClasses {
        let mut log = AliasLog::new();
        for r in runs {
            log.apply(AliasOp { pairs: vec![r] });
        }
        AliasClasses::from_log(&log)
    }

    fn para(id: Dot, chars: &[(Dot, char)]) -> RawNode {
        RawNode {
            id,
            node_type: NodeType::Paragraph,
            attrs: vec![],
            children: chars
                .iter()
                .map(|(d, c)| RawChild::Leaf {
                    id: *d,
                    item: SeqItem::Char(*c),
                })
                .collect(),
        }
    }

    fn root(children: Vec<RawNode>) -> RawTree {
        RawTree {
            roots: vec![RawNode {
                id: Dot::ROOT,
                node_type: NodeType::Root,
                attrs: vec![],
                children: children.into_iter().map(RawChild::Block).collect(),
            }],
        }
    }

    fn top_ids(raw: &RawTree) -> Vec<Dot> {
        raw.roots[0]
            .children
            .iter()
            .map(|c| match c {
                RawChild::Block(b) => b.id,
                RawChild::Leaf { id, .. } => *id,
            })
            .collect()
    }

    #[test]
    fn hidden_copies_track_roots_and_their_closure() {
        let mut h = HiddenCopies::default();
        assert!(h.is_empty());
        let root = Dot::new(2, 10);
        let c1 = Dot::new(2, 11);
        let c2 = Dot::new(2, 12);
        h.insert_root(root, vec![root, c1, c2]);
        assert!(!h.is_empty());
        assert!(h.contains(root) && h.contains(c1) && h.contains(c2));
        assert!(!h.contains(Dot::new(2, 13)));
        assert_eq!(h.dots_of_root(root), Some(&[root, c1, c2][..]));
        assert_eq!(h.roots().collect::<Vec<_>>(), vec![root]);

        h.remove_root(root);
        assert!(h.is_empty());
        assert!(!h.contains(c1));
    }

    #[test]
    fn extend_merges_roots() {
        let mut a = HiddenCopies::default();
        a.insert_root(Dot::new(1, 0), vec![Dot::new(1, 0)]);
        let mut b = HiddenCopies::default();
        b.insert_root(Dot::new(3, 0), vec![Dot::new(3, 0), Dot::new(3, 1)]);
        a.extend(b);
        assert!(a.contains(Dot::new(1, 0)) && a.contains(Dot::new(3, 1)));
        assert_eq!(a.roots().count(), 2);
    }

    #[test]
    fn reinserting_a_root_replaces_its_closure() {
        let mut h = HiddenCopies::default();
        let r = Dot::new(1, 0);
        let x = Dot::new(1, 1);
        h.insert_root(r, vec![r, x]);
        h.insert_root(r, vec![r]);
        assert!(!h.contains(x));
        assert_eq!(h.dots_of_root(r), Some(&[r][..]));
        h.remove_root(r);
        assert!(h.is_empty());
    }

    #[test]
    fn extend_replaces_a_colliding_root() {
        let r = Dot::new(1, 0);
        let x = Dot::new(1, 1);
        let y = Dot::new(1, 2);
        let mut a = HiddenCopies::default();
        a.insert_root(r, vec![r, x]);
        let mut b = HiddenCopies::default();
        b.insert_root(r, vec![r, y]);
        a.extend(b);
        assert!(a.contains(y));
        assert!(!a.contains(x));
        assert_eq!(a.dots_of_root(r), Some(&[r, y][..]));
        assert_eq!(a.roots().count(), 1);
    }

    #[test]
    fn remove_root_drops_only_that_roots_dots() {
        let mut h = HiddenCopies::default();
        let a = Dot::new(1, 0);
        let b = Dot::new(2, 0);
        h.insert_root(a, vec![a, Dot::new(1, 1)]);
        h.insert_root(b, vec![b]);
        h.remove_root(a);
        assert!(!h.contains(a) && !h.contains(Dot::new(1, 1)));
        assert!(h.contains(b));
        assert_eq!(h.dots_of_root(a), None);
        assert_eq!(h.roots().collect::<Vec<_>>(), vec![b]);
    }

    #[test]
    fn two_present_copies_keep_only_the_max_dot() {
        let p1 = Dot::new(2, 10);
        let p2 = Dot::new(3, 20);
        let mut raw = root(vec![
            para(p1, &[(Dot::new(2, 11), 'a')]),
            para(p2, &[(Dot::new(3, 21), 'a')]),
        ]);
        let cls = classes(vec![run((1, 0), 2, (2, 10)), run((1, 0), 2, (3, 20))]);
        let hidden = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(top_ids(&raw), vec![p2]);
        assert!(hidden.contains(p1) && hidden.contains(Dot::new(2, 11)));
        assert_eq!(hidden.dots_of_root(p1), Some(&[p1, Dot::new(2, 11)][..]));
        assert!(!hidden.contains(p2));
    }

    #[test]
    fn a_single_present_copy_is_untouched_and_empty_classes_cost_nothing() {
        let p1 = Dot::new(2, 10);
        let mut raw = root(vec![para(p1, &[(Dot::new(2, 11), 'a')])]);
        let before = raw.clone();
        let cls = classes(vec![run((1, 0), 2, (2, 10))]);
        assert!(
            hide_losers(&mut raw, &cls, &|_| Outside::Absent)
                .unwrap()
                .is_empty()
        );
        assert_eq!(raw, before);
        let empty = AliasClasses::from_log(&AliasLog::new());
        assert!(
            hide_losers(&mut raw, &empty, &|_| panic!(
                "empty classes must not consult outside"
            ))
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn a_member_under_a_hidden_ancestor_is_not_a_candidate() {
        let outer_a = Dot::new(2, 10);
        let outer_b = Dot::new(3, 30);
        let inner_in_a = Dot::new(2, 12);
        let inner_alone = Dot::new(4, 40);
        let mut raw = root(vec![
            RawNode {
                id: outer_a,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(inner_in_a, &[(Dot::new(2, 13), 'x')]))],
            },
            RawNode {
                id: outer_b,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(Dot::new(3, 31), &[]))],
            },
            para(inner_alone, &[(Dot::new(4, 41), 'x')]),
        ]);
        let cls = classes(vec![
            run((1, 0), 1, (2, 10)),
            run((1, 0), 1, (3, 30)),
            run((1, 5), 2, (2, 12)),
            run((1, 5), 2, (4, 40)),
        ]);
        let forest_dots = [outer_a, inner_in_a, outer_b, Dot::new(3, 31), inner_alone];
        let hidden = hide_losers(&mut raw, &cls, &|d| {
            assert!(
                !forest_dots.contains(&d),
                "이 포레스트가 가진 dot에는 outside를 묻지 않는다"
            );
            Outside::Absent
        })
        .unwrap();
        assert_eq!(top_ids(&raw), vec![outer_b, inner_alone]);
        assert!(hidden.contains(outer_a) && hidden.contains(inner_in_a));
        assert!(
            !hidden.contains(inner_alone),
            "조상 판정이 먼저 끝나 후보에서 빠진 뒤 남은 유일한 후보라서 보인다"
        );
    }

    #[test]
    fn a_present_member_outside_the_forest_escalates() {
        let p1 = Dot::new(2, 10);
        let mut raw = root(vec![para(p1, &[])]);
        let before = raw.clone();
        let cls = classes(vec![run((1, 0), 1, (2, 10)), run((1, 0), 1, (3, 20))]);
        let err = hide_losers(&mut raw, &cls, &|d| {
            if d == Dot::new(3, 20) {
                Outside::Present
            } else {
                Outside::Absent
            }
        })
        .unwrap_err();
        assert_eq!(
            err,
            Escalate {
                dot: Dot::new(3, 20)
            }
        );
        assert_eq!(raw, before);
    }

    #[test]
    fn hide_is_deterministic_for_the_same_input() {
        let mk = || {
            root(vec![
                para(Dot::new(2, 10), &[(Dot::new(2, 11), 'a')]),
                para(Dot::new(3, 20), &[(Dot::new(3, 21), 'a')]),
                para(Dot::new(4, 30), &[(Dot::new(4, 31), 'a')]),
            ])
        };
        let cls = classes(vec![
            run((1, 0), 2, (2, 10)),
            run((1, 0), 2, (3, 20)),
            run((1, 0), 2, (4, 30)),
        ]);
        let (mut a, mut b) = (mk(), mk());
        let ha = hide_losers(&mut a, &cls, &|_| Outside::Absent).unwrap();
        let hb = hide_losers(&mut b, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(a, b);
        assert_eq!(ha, hb);
        assert_eq!(top_ids(&a), vec![Dot::new(4, 30)]);
    }

    #[test]
    fn a_winner_nested_inside_another_classs_loser_keeps_its_class_visible() {
        let outer_a = Dot::new(2, 10);
        let outer_b = Dot::new(3, 30);
        let inner_hi = Dot::new(9, 50);
        let inner_alone = Dot::new(4, 40);
        let mut raw = root(vec![
            RawNode {
                id: outer_a,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(inner_hi, &[(Dot::new(9, 51), 'x')]))],
            },
            RawNode {
                id: outer_b,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(Dot::new(3, 31), &[]))],
            },
            para(inner_alone, &[(Dot::new(4, 41), 'x')]),
        ]);
        let cls = classes(vec![
            run((1, 0), 1, (2, 10)),
            run((1, 0), 1, (3, 30)),
            run((1, 5), 1, (9, 50)),
            run((1, 5), 1, (4, 40)),
        ]);
        let hidden = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(top_ids(&raw), vec![outer_b, inner_alone]);
        assert!(hidden.contains(outer_a) && hidden.contains(inner_hi));
        assert!(
            !hidden.contains(inner_alone),
            "조상을 먼저 판정해야 이 클래스가 마지막 사본까지 잃지 않는다"
        );
    }

    #[test]
    fn an_atom_leaf_loser_is_detached_as_a_single_dot_root() {
        let loser = Dot::new(2, 10);
        let winner = Dot::new(3, 20);
        let mut raw = root(vec![RawNode {
            id: Dot::new(5, 1),
            node_type: NodeType::Paragraph,
            attrs: vec![],
            children: vec![
                RawChild::Leaf {
                    id: loser,
                    item: SeqItem::Atom(AtomLeaf::HardBreak),
                },
                RawChild::Leaf {
                    id: winner,
                    item: SeqItem::Atom(AtomLeaf::HardBreak),
                },
            ],
        }]);
        let cls = classes(vec![run((1, 0), 1, (2, 10)), run((1, 0), 1, (3, 20))]);
        let hidden = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(hidden.dots_of_root(loser), Some(&[loser][..]));
        assert!(!hidden.contains(winner));
        let RawChild::Block(p) = &raw.roots[0].children[0] else {
            panic!("paragraph stays a block child");
        };
        assert_eq!(p.children.len(), 1);
        assert!(matches!(&p.children[0], RawChild::Leaf { id, .. } if *id == winner));
    }

    #[test]
    fn a_real_dot_forest_root_counts_as_present_without_consulting_outside() {
        let container = Dot::new(2, 10);
        let mut raw = RawTree {
            roots: vec![RawNode {
                id: container,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(Dot::new(2, 11), &[]))],
            }],
        };
        let before = raw.clone();
        let cls = classes(vec![run((1, 0), 1, (2, 10))]);
        let judged = std::cell::Cell::new(false);
        let hidden = hide_losers(&mut raw, &cls, &|d| {
            assert_ne!(
                d, container,
                "a real-dot forest root is present in the window"
            );
            if d == Dot::new(1, 0) {
                judged.set(true);
            }
            Outside::Absent
        })
        .unwrap();
        assert!(judged.get(), "the container's own class must be judged");
        assert!(hidden.is_empty());
        assert_eq!(raw, before);
    }

    #[test]
    fn a_forest_root_that_would_lose_escalates() {
        let losing_root = Dot::new(2, 10);
        let winning_root = Dot::new(3, 30);
        let mut raw = RawTree {
            roots: vec![
                RawNode {
                    id: losing_root,
                    node_type: NodeType::Blockquote,
                    attrs: vec![],
                    children: vec![RawChild::Block(para(Dot::new(2, 11), &[]))],
                },
                RawNode {
                    id: winning_root,
                    node_type: NodeType::Blockquote,
                    attrs: vec![],
                    children: vec![RawChild::Block(para(Dot::new(3, 31), &[]))],
                },
            ],
        };
        let before = raw.clone();
        let cls = classes(vec![run((1, 0), 1, (2, 10)), run((1, 0), 1, (3, 30))]);
        let err = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap_err();
        assert_eq!(err, Escalate { dot: losing_root });
        assert_eq!(raw, before);
    }

    #[test]
    fn a_winner_under_a_later_decided_losing_ancestor_is_never_chosen() {
        let p = Dot::new(5, 1);
        let v = Dot::new(1, 1);
        let q = Dot::new(2, 10);
        let w = Dot::new(9, 50);
        let qc = Dot::new(6, 60);
        let mut raw = root(vec![
            RawNode {
                id: p,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(v, &[]))],
            },
            RawNode {
                id: q,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![RawChild::Block(para(w, &[]))],
            },
            para(qc, &[]),
        ]);
        let cls = classes(vec![run((1, 1), 1, (9, 50)), run((2, 10), 1, (6, 60))]);
        let hidden = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(top_ids(&raw), vec![p, qc]);
        let RawChild::Block(kept) = &raw.roots[0].children[0] else {
            panic!("P stays a block child");
        };
        assert!(matches!(&kept.children[0], RawChild::Block(b) if b.id == v));
        assert!(hidden.contains(q) && hidden.contains(w));
        assert!(
            !hidden.contains(v),
            "조상이 나중에 져서 사라질 사본은 애초에 승자로 뽑히지 않는다"
        );
    }

    #[test]
    fn a_member_under_a_pending_loser_is_not_a_settled_candidate() {
        let d = Dot::new(2, 100);
        let b = Dot::new(3, 20);
        let u = Dot::new(5, 30);
        let a = Dot::new(1, 10);
        let w = Dot::new(6, 40);
        let mut raw = root(vec![
            para(d, &[]),
            RawNode {
                id: b,
                node_type: NodeType::Blockquote,
                attrs: vec![],
                children: vec![
                    RawChild::Block(para(u, &[])),
                    RawChild::Block(RawNode {
                        id: a,
                        node_type: NodeType::Blockquote,
                        attrs: vec![],
                        children: vec![RawChild::Block(para(w, &[]))],
                    }),
                ],
            },
        ]);
        let cls = classes(vec![
            run((1, 10), 1, (2, 100)),
            run((3, 20), 1, (4, 5)),
            run((5, 30), 1, (6, 40)),
        ]);
        let hidden = hide_losers(&mut raw, &cls, &|_| Outside::Absent).unwrap();
        assert_eq!(top_ids(&raw), vec![d, b]);
        assert!(hidden.contains(a) && hidden.contains(w));
        assert!(
            !hidden.contains(u),
            "미결 패자 아래의 사본은 후보가 아니므로 이 클래스는 마지막 사본을 잃지 않는다"
        );
        assert!(!hidden.contains(d) && !hidden.contains(b));
        let RawChild::Block(kept) = &raw.roots[0].children[1] else {
            panic!("B stays a block child");
        };
        assert_eq!(kept.children.len(), 1);
        assert!(matches!(&kept.children[0], RawChild::Block(p) if p.id == u));
    }

    fn elems_of(raw: &RawTree) -> Vec<(Dot, SeqItem)> {
        fn walk(n: &RawNode, out: &mut Vec<(Dot, SeqItem)>) {
            for c in &n.children {
                match c {
                    RawChild::Block(b) => {
                        out.push((
                            b.id,
                            SeqItem::Block {
                                node_type: b.node_type,
                                parents: vec![],
                                attrs: vec![],
                            },
                        ));
                        walk(b, out);
                    }
                    RawChild::Leaf { id, item } => out.push((*id, item.clone())),
                }
            }
        }
        let mut out = Vec::new();
        walk(&raw.roots[0], &mut out);
        out
    }

    fn text_of(raw: &RawTree, block: Dot) -> String {
        fn find(n: &RawNode, id: Dot) -> Option<&RawNode> {
            if n.id == id {
                return Some(n);
            }
            n.children.iter().find_map(|c| match c {
                RawChild::Block(b) => find(b, id),
                _ => None,
            })
        }
        let node = find(&raw.roots[0], block).expect("block present");
        node.children
            .iter()
            .filter_map(|c| match c {
                RawChild::Leaf {
                    item: SeqItem::Char(ch),
                    ..
                } => Some(*ch),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_leaf_whose_origin_is_a_dead_member_lands_after_the_image() {
        let old_p = Dot::new(1, 0);
        let old_a = Dot::new(1, 1);
        let old_b = Dot::new(1, 2);
        let new_p = Dot::new(2, 10);
        let new_a = Dot::new(2, 11);
        let new_b = Dot::new(2, 12);
        let x = Dot::new(3, 0);
        let y = Dot::new(3, 1);
        let prev_p = Dot::new(1, 20);
        let mut raw = root(vec![
            para(prev_p, &[(Dot::new(1, 21), 'p'), (x, 'x'), (y, 'y')]),
            para(new_p, &[(new_a, 'a'), (new_b, 'b')]),
        ]);
        let cls = classes(vec![run((1, 0), 3, (2, 10))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| match *d {
                d if d == x => Some(old_a),
                d if d == y => Some(x),
                _ => None,
            })
            .collect();
        let dead = |d: Dot| [old_p, old_a, old_b].contains(&d);
        let redirects = redirect_anchors(&elements, &origins, &cls, &dead, &|_| false, &|_| true);
        assert_eq!(redirects, vec![(x, old_a), (y, x)]);
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(text_of(&raw, new_p), "axyb");
        assert_eq!(text_of(&raw, prev_p), "p");
    }

    #[test]
    fn a_block_anchor_places_the_leaf_in_the_first_slot() {
        let old_p = Dot::new(1, 0);
        let new_p = Dot::new(2, 10);
        let x = Dot::new(3, 0);
        let mut raw = root(vec![
            para(Dot::new(1, 20), &[(x, 'x')]),
            para(new_p, &[(Dot::new(2, 11), 'a')]),
        ]);
        let cls = classes(vec![run((1, 0), 1, (2, 10))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x).then_some(old_p))
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|d| d == old_p,
            &|_| false,
            &|_| true,
        );
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(text_of(&raw, new_p), "xa");
    }

    #[test]
    fn a_missing_image_falls_back_to_the_left_scan_then_stays_put() {
        let old_a = Dot::new(1, 1);
        let old_b = Dot::new(1, 2);
        let new_p = Dot::new(2, 10);
        let new_a = Dot::new(2, 11);
        let x = Dot::new(3, 0);
        let host = Dot::new(1, 20);
        let mut raw = root(vec![para(host, &[(x, 'x')]), para(new_p, &[(new_a, 'a')])]);
        let cls = classes(vec![run((1, 1), 2, (2, 11))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x).then_some(old_b))
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|d| d == old_a || d == old_b,
            &|_| false,
            &|_| true,
        );
        let prev = |d: Dot| (d == old_b).then_some((old_a, false));
        place_redirects(&mut raw, &redirects, &cls, &prev, &|_| Outside::Absent).unwrap();
        assert_eq!(
            text_of(&raw, new_p),
            "ax",
            "b'는 지워졌으니 왼쪽 스캔이 a를 앵커로 잡는다"
        );

        let mut stay = root(vec![para(host, &[(x, 'x')]), para(new_p, &[])]);
        let elements = elems_of(&stay);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x).then_some(old_b))
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|d| d == old_a || d == old_b,
            &|_| false,
            &|_| true,
        );
        let prev = |d: Dot| (d == old_b).then_some((old_a, false));
        place_redirects(&mut stay, &redirects, &cls, &prev, &|_| Outside::Absent).unwrap();
        assert_eq!(
            text_of(&stay, host),
            "x",
            "이미지가 전무하면 현행 위치 유지"
        );
    }

    #[test]
    fn a_live_origin_is_never_redirected() {
        let x = Dot::new(3, 0);
        let live = Dot::new(1, 21);
        let raw = root(vec![para(Dot::new(1, 20), &[(live, 'p'), (x, 'x')])]);
        let cls = classes(vec![run((1, 0), 1, (2, 10))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x).then_some(live))
            .collect();
        assert!(
            redirect_anchors(&elements, &origins, &cls, &|_| false, &|_| false, &|_| true)
                .is_empty()
        );
    }

    #[test]
    fn an_image_outside_the_forest_escalates() {
        let old_a = Dot::new(1, 1);
        let x = Dot::new(3, 0);
        let mut raw = root(vec![para(Dot::new(1, 20), &[(x, 'x')])]);
        let cls = classes(vec![run((1, 1), 1, (2, 11))]);
        let redirects = vec![(x, old_a)];
        let err = place_redirects(&mut raw, &redirects, &cls, &|_| None, &|d| {
            if d == Dot::new(2, 11) {
                Outside::Present
            } else {
                Outside::Absent
            }
        })
        .unwrap_err();
        assert_eq!(
            err,
            Escalate {
                dot: Dot::new(2, 11)
            }
        );
    }

    #[test]
    fn a_chain_anchor_missing_from_the_forest_leaves_its_follower_in_place() {
        let old_a = Dot::new(1, 1);
        let new_a = Dot::new(2, 11);
        let new_p = Dot::new(2, 10);
        let host = Dot::new(1, 20);
        let gone = Dot::new(3, 0);
        let l = Dot::new(3, 1);
        let mut raw = root(vec![para(host, &[(l, 'l')]), para(new_p, &[(new_a, 'a')])]);
        let cls = classes(vec![run((1, 1), 1, (2, 11))]);
        let redirects = vec![(gone, old_a), (l, gone)];
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(
            text_of(&raw, host),
            "l",
            "포레스트에 없는 앵커를 따라 사라지지 않는다"
        );
        assert_eq!(text_of(&raw, new_p), "a");
    }

    #[test]
    fn the_left_scan_fails_at_live_context_even_when_it_is_a_class_member() {
        let old_b = Dot::new(1, 2);
        let live = Dot::new(1, 5);
        let twin = Dot::new(4, 5);
        let host = Dot::new(1, 20);
        let x = Dot::new(3, 0);
        let mut raw = root(vec![para(host, &[(live, 'p'), (x, 'x')])]);
        let cls = classes(vec![run((1, 2), 1, (2, 12)), run((1, 5), 1, (4, 5))]);
        let prev = |d: Dot| (d == old_b).then_some((live, true));
        place_redirects(&mut raw, &[(x, old_b)], &cls, &prev, &|d| {
            if d == twin {
                Outside::Present
            } else {
                Outside::Absent
            }
        })
        .unwrap();
        assert_eq!(
            text_of(&raw, host),
            "px",
            "살아 있는 맥락에서 스캔이 끝나므로 리프는 제자리에 남는다"
        );
    }

    #[test]
    fn a_long_chain_lands_in_seq_order() {
        let old_a = Dot::new(1, 1);
        let new_a = Dot::new(2, 11);
        let new_p = Dot::new(2, 10);
        let host = Dot::new(1, 20);
        let n: u64 = 2000;
        let chain: Vec<(Dot, char)> = (0..n)
            .map(|i| (Dot::new(3, i), char::from(b'a' + (i % 26) as u8)))
            .collect();
        let mut raw = root(vec![para(host, &chain), para(new_p, &[(new_a, 'z')])]);
        let cls = classes(vec![run((1, 1), 1, (2, 11))]);
        let mut redirects = vec![(Dot::new(3, 0), old_a)];
        redirects.extend((1..n).map(|i| (Dot::new(3, i), Dot::new(3, i - 1))));
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        let expected: String = std::iter::once('z')
            .chain(chain.iter().map(|(_, c)| *c))
            .collect();
        assert_eq!(text_of(&raw, new_p), expected);
        assert_eq!(text_of(&raw, host), "");
    }

    #[test]
    fn a_root_image_takes_the_leaf_as_its_first_child() {
        let old_r = Dot::new(1, 0);
        let new_r = Dot::new(2, 10);
        let kept = Dot::new(2, 11);
        let host = Dot::new(1, 20);
        let x = Dot::new(3, 0);
        let mut raw = RawTree {
            roots: vec![para(host, &[(x, 'x')]), para(new_r, &[(kept, 'a')])],
        };
        let cls = classes(vec![run((1, 0), 1, (2, 10))]);
        place_redirects(&mut raw, &[(x, old_r)], &cls, &|_| None, &|_| {
            Outside::Absent
        })
        .unwrap();
        assert!(raw.roots[0].children.is_empty());
        let ids: Vec<Dot> = raw.roots[1]
            .children
            .iter()
            .map(|c| match c {
                RawChild::Block(b) => b.id,
                RawChild::Leaf { id, .. } => *id,
            })
            .collect();
        assert_eq!(ids, vec![x, kept]);
    }

    #[test]
    fn a_hidden_origin_member_is_redirected() {
        let old_a = Dot::new(1, 1);
        let new_a = Dot::new(2, 11);
        let x = Dot::new(3, 0);
        let raw = root(vec![
            para(Dot::new(1, 20), &[(x, 'x')]),
            para(Dot::new(2, 10), &[(new_a, 'a')]),
        ]);
        let cls = classes(vec![run((1, 1), 1, (2, 11))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x).then_some(old_a))
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|_| false,
            &|d| d == old_a,
            &|_| true,
        );
        assert_eq!(redirects, vec![(x, old_a)]);
    }

    #[test]
    fn a_chain_leaf_follows_its_anchor_into_the_first_slot() {
        let old_p = Dot::new(1, 0);
        let new_p = Dot::new(2, 10);
        let x = Dot::new(3, 0);
        let y = Dot::new(3, 1);
        let mut raw = root(vec![
            para(Dot::new(1, 20), &[(x, 'x'), (y, 'y')]),
            para(new_p, &[(Dot::new(2, 11), 'a')]),
        ]);
        let cls = classes(vec![run((1, 0), 1, (2, 10))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| match *d {
                d if d == x => Some(old_p),
                d if d == y => Some(x),
                _ => None,
            })
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|d| d == old_p,
            &|_| false,
            &|_| true,
        );
        assert_eq!(redirects, vec![(x, old_p), (y, x)]);
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(text_of(&raw, new_p), "xya");
    }

    #[test]
    fn two_leaves_sharing_one_anchor_keep_seq_order() {
        let old_a = Dot::new(1, 1);
        let new_p = Dot::new(2, 10);
        let new_a = Dot::new(2, 11);
        let new_b = Dot::new(2, 12);
        let host = Dot::new(1, 20);
        let x = Dot::new(3, 0);
        let y = Dot::new(3, 1);
        let mut raw = root(vec![
            para(host, &[(x, 'x'), (y, 'y')]),
            para(new_p, &[(new_a, 'a'), (new_b, 'b')]),
        ]);
        let cls = classes(vec![run((1, 1), 2, (2, 11))]);
        let elements = elems_of(&raw);
        let origins: Vec<Option<Dot>> = elements
            .iter()
            .map(|(d, _)| (*d == x || *d == y).then_some(old_a))
            .collect();
        let redirects = redirect_anchors(
            &elements,
            &origins,
            &cls,
            &|d| d == old_a,
            &|_| false,
            &|_| true,
        );
        assert_eq!(redirects, vec![(x, old_a), (y, old_a)]);
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(text_of(&raw, new_p), "axyb");
        assert_eq!(text_of(&raw, host), "");
    }

    #[test]
    fn a_branching_chain_emits_each_branch_depth_first() {
        let old_a = Dot::new(1, 1);
        let new_p = Dot::new(2, 10);
        let new_a = Dot::new(2, 11);
        let host = Dot::new(1, 20);
        let x = Dot::new(3, 0);
        let z = Dot::new(3, 1);
        let y = Dot::new(4, 0);
        let mut raw = root(vec![
            para(host, &[(x, 'x'), (z, 'z'), (y, 'y')]),
            para(new_p, &[(new_a, 'a')]),
        ]);
        let cls = classes(vec![run((1, 1), 1, (2, 11))]);
        let redirects = vec![(x, old_a), (z, x), (y, old_a)];
        place_redirects(&mut raw, &redirects, &cls, &|_| None, &|_| Outside::Absent).unwrap();
        assert_eq!(text_of(&raw, new_p), "axzy");
        assert_eq!(text_of(&raw, host), "");
    }
}

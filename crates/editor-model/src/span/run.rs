use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use editor_crdt::Dot;
use strum::IntoEnumIterator;

use crate::projection::LeafEff;
use crate::seq::{BlockNode, BlockTree, Child};
use crate::{Modifier, ModifierType, NodeType, Schema};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub block: Dot,
    pub leaves: Vec<Dot>,
    pub modifiers: BTreeMap<ModifierType, Modifier>,
}

fn modifier_target_types() -> HashSet<NodeType> {
    ModifierType::iter()
        .flat_map(|m| Schema::modifier_spec(m).target.rightmost_node_types())
        .collect()
}

/// Whether a leaf of this type participates in mergeable runs (some modifier
/// targets it). Non-targets (atoms like HardBreak/Tab) form their own run — the
/// `is_atom` flag the splice path passes is exactly `!is_modifier_target(ty)`.
pub fn is_modifier_target(node_type: NodeType) -> bool {
    ModifierType::iter().any(|m| {
        Schema::modifier_spec(m)
            .target
            .rightmost_node_types()
            .contains(&node_type)
    })
}

/// Compare a run's shared modifiers against a leaf's effective entry: pointer
/// identity first (uniform stretches share one allocation), value equality as
/// the fallback. A missing entry is equivalent to an empty map.
fn modifiers_eq(a: &LeafEff, b: Option<&LeafEff>) -> bool {
    match b {
        Some(m) => Arc::ptr_eq(a, m) || a == m,
        None => a.is_empty(),
    }
}

pub fn derive_runs(tree: &BlockTree, effective: &imbl::HashMap<Dot, LeafEff>) -> Vec<Run> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        effective: &imbl::HashMap<Dot, LeafEff>,
        targets: &HashSet<NodeType>,
        out: &mut Vec<Run>,
    ) {
        let mut run: Option<(Run, LeafEff)> = None;
        for c in &node.children {
            match c {
                Child::Leaf { id: d, item } => {
                    let eff_ref = effective.get(d);
                    if targets.contains(&item.as_child_type()) {
                        match &mut run {
                            Some((r, eff)) if modifiers_eq(eff, eff_ref) => r.leaves.push(*d),
                            _ => {
                                if let Some((r, _)) = run.take() {
                                    out.push(r);
                                }
                                let eff = eff_ref.cloned().unwrap_or_default();
                                run = Some((
                                    Run {
                                        block: node.id,
                                        leaves: vec![*d],
                                        modifiers: (*eff).clone(),
                                    },
                                    eff,
                                ));
                            }
                        }
                    } else {
                        if let Some((r, _)) = run.take() {
                            out.push(r);
                        }
                        out.push(Run {
                            block: node.id,
                            leaves: vec![*d],
                            modifiers: eff_ref.map(|e| (**e).clone()).unwrap_or_default(),
                        });
                    }
                }
                Child::Block(id) => {
                    if let Some((r, _)) = run.take() {
                        out.push(r);
                    }
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, effective, targets, out);
                    }
                }
            }
        }
        if let Some((r, _)) = run {
            out.push(r);
        }
    }
    let targets = modifier_target_types();
    let mut out = Vec::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, effective, &targets, &mut out);
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSeg {
    // Persistent vector: a hot sequential splice (paste/typing) appends into one run,
    // and `clone`-on-`set` of the run must stay O(log K), not O(run length). A plain
    // `Vec` made each append clone the whole run, giving an O(N²) paste.
    pub leaves: imbl::Vector<Dot>,
    // Shared with the leaves' `effective` entries: merge checks are pointer
    // compares and re-segmentation clones Arcs, not maps.
    pub modifiers: LeafEff,
    pub mergeable: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BlockRuns {
    blocks: HashMap<Dot, editor_common::SumTree<RunSeg, usize>>,
}

/// Group an ordered `(leaf, mergeable)` list into run segments by equal
/// effective modifiers (atoms / non-mergeable leaves always stand alone).
pub fn segment_leaves(
    leaves: &[(Dot, bool)],
    effective: &imbl::HashMap<Dot, LeafEff>,
) -> Vec<RunSeg> {
    let mut out: Vec<RunSeg> = Vec::new();
    let mut cur: Option<RunSeg> = None;
    for &(id, mergeable) in leaves {
        let eff_ref = effective.get(&id);
        if mergeable {
            match &mut cur {
                Some(r) if r.mergeable && modifiers_eq(&r.modifiers, eff_ref) => {
                    r.leaves.push_back(id)
                }
                _ => {
                    if let Some(r) = cur.take() {
                        out.push(r);
                    }
                    cur = Some(RunSeg {
                        leaves: imbl::vector![id],
                        modifiers: eff_ref.cloned().unwrap_or_default(),
                        mergeable: true,
                    });
                }
            }
        } else {
            if let Some(r) = cur.take() {
                out.push(r);
            }
            out.push(RunSeg {
                leaves: imbl::vector![id],
                modifiers: eff_ref.cloned().unwrap_or_default(),
                mergeable: false,
            });
        }
    }
    if let Some(r) = cur {
        out.push(r);
    }
    out
}

pub fn resegment_block(node: &BlockNode, effective: &imbl::HashMap<Dot, LeafEff>) -> Vec<RunSeg> {
    let targets = modifier_target_types();
    let mut out: Vec<RunSeg> = Vec::new();
    let mut cur: Option<RunSeg> = None;
    for c in &node.children {
        match c {
            Child::Leaf { id, item } => {
                let eff_ref = effective.get(id);
                if targets.contains(&item.as_child_type()) {
                    match &mut cur {
                        Some(r) if r.mergeable && modifiers_eq(&r.modifiers, eff_ref) => {
                            r.leaves.push_back(*id)
                        }
                        _ => {
                            if let Some(r) = cur.take() {
                                out.push(r);
                            }
                            cur = Some(RunSeg {
                                leaves: imbl::vector![*id],
                                modifiers: eff_ref.cloned().unwrap_or_default(),
                                mergeable: true,
                            });
                        }
                    }
                } else {
                    if let Some(r) = cur.take() {
                        out.push(r);
                    }
                    out.push(RunSeg {
                        leaves: imbl::vector![*id],
                        modifiers: eff_ref.cloned().unwrap_or_default(),
                        mergeable: false,
                    });
                }
            }
            Child::Block(_) => {
                if let Some(r) = cur.take() {
                    out.push(r);
                }
            }
        }
    }
    if let Some(r) = cur {
        out.push(r);
    }
    out
}

impl BlockRuns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(tree: &BlockTree, effective: &imbl::HashMap<Dot, LeafEff>) -> Self {
        fn walk(
            tree: &BlockTree,
            node: &BlockNode,
            effective: &imbl::HashMap<Dot, LeafEff>,
            out: &mut BlockRuns,
        ) {
            out.set_block(node.id, resegment_block(node, effective));
            for c in &node.children {
                if let Child::Block(id) = c
                    && let Some(b) = tree.get(*id)
                {
                    walk(tree, b, effective, out);
                }
            }
        }
        let mut out = BlockRuns::new();
        if let Some(root) = tree.root_node() {
            walk(tree, root, effective, &mut out);
        }
        out
    }

    pub fn set_block(&mut self, block: Dot, segs: Vec<RunSeg>) {
        let tree: editor_common::SumTree<RunSeg, usize> = segs
            .into_iter()
            .map(|s| {
                let n = s.leaves.len();
                (s, n)
            })
            .collect();
        self.blocks.insert(block, tree);
    }

    pub fn remove_block(&mut self, block: Dot) {
        self.blocks.remove(&block);
    }

    /// The block's run segments as `(modifiers, leaf count)` groups, in leaf
    /// order. Lets a range query aggregate per uniform segment instead of per
    /// leaf, without materializing the segments' leaf lists.
    pub fn group_iter(
        &self,
        block: Dot,
    ) -> impl Iterator<Item = (&BTreeMap<ModifierType, Modifier>, usize)> + '_ {
        self.blocks
            .get(&block)
            .into_iter()
            .flat_map(|t| t.iter().map(|seg| (&*seg.modifiers, seg.leaves.len())))
    }

    pub fn iter_block(&self, block: Dot) -> Vec<RunSeg> {
        self.blocks
            .get(&block)
            .map(|t| t.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Re-segment a block in place from its current leaf order and updated
    /// `effective`, without consulting the tree. Sound because leaf-bearing
    /// blocks (Paragraph, Caption, …) never interleave block children, so the
    /// stored run leaves are the block's complete inline sequence — no run
    /// boundary is lost by flattening. O(block size).
    /// The block's leaves in order, paired with their mergeable flag, recovered
    /// from the stored run segments.
    pub fn block_leaves(&self, block: Dot) -> Vec<(Dot, bool)> {
        self.blocks
            .get(&block)
            .map(|t| {
                t.iter()
                    .flat_map(|seg| {
                        let m = seg.mergeable;
                        seg.leaves.iter().map(move |&id| (id, m))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_block_from_leaves(
        &mut self,
        block: Dot,
        leaves: &[(Dot, bool)],
        effective: &imbl::HashMap<Dot, LeafEff>,
    ) {
        self.set_block(block, segment_leaves(leaves, effective));
    }

    pub fn resegment_from_runs(&mut self, block: Dot, effective: &imbl::HashMap<Dot, LeafEff>) {
        let leaves = self.block_leaves(block);
        self.set_block(block, segment_leaves(&leaves, effective));
    }

    pub fn materialize(&self, tree: &BlockTree) -> Vec<Run> {
        fn walk(tree: &BlockTree, node: &BlockNode, runs: &BlockRuns, out: &mut Vec<Run>) {
            if let Some(t) = runs.blocks.get(&node.id) {
                for seg in t.iter() {
                    out.push(Run {
                        block: node.id,
                        leaves: seg.leaves.iter().copied().collect(),
                        modifiers: (*seg.modifiers).clone(),
                    });
                }
            }
            for c in &node.children {
                if let Child::Block(id) = c
                    && let Some(b) = tree.get(*id)
                {
                    walk(tree, b, runs, out);
                }
            }
        }
        let mut out = Vec::new();
        if let Some(root) = tree.root_node() {
            walk(tree, root, self, &mut out);
        }
        out
    }

    pub fn splice_insert(
        &mut self,
        block: Dot,
        offset: usize,
        leaf: Dot,
        eff: LeafEff,
        is_atom: bool,
    ) {
        let tree = self.blocks.entry(block).or_default();
        let fresh = RunSeg {
            leaves: imbl::vector![leaf],
            modifiers: eff.clone(),
            mergeable: !is_atom,
        };
        match tree.find_by_offset(offset) {
            None => {
                if !is_atom && !tree.is_empty() {
                    let li = tree.len() - 1;
                    let last = tree.get(li).expect("last exists");
                    if last.mergeable && modifiers_eq(&last.modifiers, Some(&eff)) {
                        let mut s = last.clone();
                        s.leaves.push_back(leaf);
                        let n = s.leaves.len();
                        tree.set(li, s, n);
                        return;
                    }
                }
                tree.push(fresh, 1);
            }
            Some((idx, intra)) => {
                let seg = tree.get(idx).expect("seg exists").clone();
                if intra == 0 {
                    if !is_atom && seg.mergeable && modifiers_eq(&seg.modifiers, Some(&eff)) {
                        let mut s = seg;
                        s.leaves.push_front(leaf);
                        let n = s.leaves.len();
                        tree.set(idx, s, n);
                    } else if !is_atom
                        && idx > 0
                        && tree
                            .get(idx - 1)
                            .is_some_and(|p| p.mergeable && modifiers_eq(&p.modifiers, Some(&eff)))
                    {
                        let mut p = tree.get(idx - 1).expect("prev exists").clone();
                        p.leaves.push_back(leaf);
                        let n = p.leaves.len();
                        tree.set(idx - 1, p, n);
                    } else {
                        tree.insert(idx, fresh, 1);
                    }
                } else if !is_atom && seg.mergeable && modifiers_eq(&seg.modifiers, Some(&eff)) {
                    let mut s = seg;
                    s.leaves.insert(intra, leaf);
                    let n = s.leaves.len();
                    tree.set(idx, s, n);
                } else {
                    let RunSeg {
                        mut leaves,
                        modifiers,
                        mergeable,
                    } = seg;
                    let right_leaves = leaves.split_off(intra);
                    let ln = leaves.len();
                    let rn = right_leaves.len();
                    let left = RunSeg {
                        leaves,
                        modifiers: modifiers.clone(),
                        mergeable,
                    };
                    let right = RunSeg {
                        leaves: right_leaves,
                        modifiers,
                        mergeable,
                    };
                    tree.set(idx, left, ln);
                    tree.insert(idx + 1, fresh, 1);
                    tree.insert(idx + 2, right, rn);
                }
            }
        }
    }

    pub fn splice_delete(&mut self, block: Dot, offset: usize) {
        let tree = match self.blocks.get_mut(&block) {
            Some(t) => t,
            None => return,
        };
        let (idx, intra) = match tree.find_by_offset(offset) {
            Some(x) => x,
            None => return,
        };
        let seg = tree.get(idx).expect("seg exists").clone();
        if seg.leaves.len() == 1 {
            tree.remove(idx);
            if idx > 0 && idx < tree.len() {
                let left = tree.get(idx - 1).expect("left exists").clone();
                let right = tree.get(idx).expect("right exists").clone();
                if left.mergeable
                    && right.mergeable
                    && modifiers_eq(&left.modifiers, Some(&right.modifiers))
                {
                    let mut merged = left;
                    merged.leaves.append(right.leaves);
                    let n = merged.leaves.len();
                    tree.set(idx - 1, merged, n);
                    tree.remove(idx);
                }
            }
        } else {
            let mut s = seg;
            s.leaves.remove(intra);
            let n = s.leaves.len();
            tree.set(idx, s, n);
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::sequence::checkout_with_resolver;
    use editor_crdt::{InputEvent, ListOp, build_oplog};

    use super::*;
    use crate::NodeType;
    use crate::seq::{SeqItem, normalize, project_blocks};
    use crate::span::{Anchor, Bias, SpanLog, SpanOp, derive_effective, derive_full_effective};
    use crate::{ModifierAttrLog, ModifierAttrOp};

    fn oplog(items: &[(Dot, SeqItem)]) -> editor_crdt::OpLog<SeqItem> {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        build_oplog(&ev)
    }

    fn eff_map(
        elems: &[(Dot, SeqItem)],
        tree: &BlockTree,
        resolver: &editor_crdt::sequence::BoundaryResolver,
        spans: &SpanLog,
    ) -> imbl::HashMap<Dot, LeafEff> {
        derive_effective(elems, tree, resolver, spans)
            .into_iter()
            .map(|(d, e)| (d, LeafEff::new(e)))
            .collect()
    }

    #[test]
    fn bold_splits_paragraph_into_two_runs() {
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 3),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 4),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &spans));
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].leaves, vec![Dot::new(1, 2)]);
        assert!(runs[0].modifiers.is_empty());
        assert_eq!(runs[1].leaves, vec![Dot::new(1, 3), Dot::new(1, 4)]);
        assert_eq!(
            runs[1].modifiers.get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
        assert_eq!(runs[0].block, runs[1].block);
    }

    #[test]
    fn runs_do_not_merge_across_block_boundary() {
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (
                Dot::new(1, 3),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 4), SeqItem::Char('b')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &SpanLog::new()));
        assert_eq!(runs.len(), 2, "다른 문단은 병합 안 됨");
        assert_eq!(runs[0].leaves, vec![Dot::new(1, 2)]);
        assert_eq!(runs[1].leaves, vec![Dot::new(1, 4)]);
        assert_ne!(runs[0].block, runs[1].block);
    }

    #[test]
    fn block_atom_and_text_block_yield_separate_runs() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::Line,
                    },
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 2),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('a')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &SpanLog::new()));
        assert_eq!(
            runs.len(),
            2,
            "HR(Root 직속) run + 'a'(Paragraph) run, 블록 경계로 분리"
        );
        assert_eq!(runs[0].leaves, vec![Dot::new(1, 1)]);
        assert_eq!(runs[1].leaves, vec![Dot::new(1, 3)]);
        assert_ne!(runs[0].block, runs[1].block);
    }

    #[test]
    fn non_target_atom_splits_run() {
        use crate::seq::AtomLeaf;
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::HardBreak)),
            (Dot::new(1, 4), SeqItem::Char('b')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &SpanLog::new()));
        assert_eq!(runs.len(), 3, "HardBreak(비대상 atom)가 run 분리");
        assert_eq!(runs[0].leaves, vec![Dot::new(1, 2)]);
        assert_eq!(runs[1].leaves, vec![Dot::new(1, 3)]);
        assert_eq!(runs[2].leaves, vec![Dot::new(1, 4)]);
    }

    #[test]
    fn consecutive_block_atoms_do_not_coalesce() {
        use crate::nodes::HorizontalRuleVariant;
        use crate::seq::AtomLeaf;
        let hr = || SeqItem::BlockAtom {
            leaf: AtomLeaf::HorizontalRule {
                variant: HorizontalRuleVariant::Line,
            },
            parents: vec![Dot::ROOT],
        };
        let elems = vec![
            (Dot::new(1, 1), hr()),
            (Dot::new(1, 2), hr()),
            (
                Dot::new(1, 3),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 4), SeqItem::Char('a')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &SpanLog::new()));
        assert_eq!(runs.len(), 3, "연속 비대상 atom은 병합 안 됨");
        assert_eq!(runs[0].leaves, vec![Dot::new(1, 1)]);
        assert_eq!(runs[1].leaves, vec![Dot::new(1, 2)]);
        assert_eq!(runs[2].leaves, vec![Dot::new(1, 4)]);
    }

    #[test]
    fn empty_document_has_no_runs() {
        let elems: Vec<(Dot, SeqItem)> = vec![];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let runs = derive_runs(&tree, &eff_map(&els, &tree, &resolver, &SpanLog::new()));
        assert!(runs.is_empty(), "빈 문서는 run 없음");
    }

    #[test]
    fn inherited_font_size_groups_into_one_run() {
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        use crate::span::{EffectiveSources, derive_explicit_effect};
        let explicit: HashMap<Dot, _> =
            derive_explicit_effect(&els, &tree, &resolver, &SpanLog::new())
                .into_iter()
                .collect();
        let node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        let styles: imbl::HashMap<String, crate::StyleEntry> = imbl::HashMap::new();
        let node_attrs: imbl::HashMap<Dot, crate::nodes::Node> = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &attrs,
            explicit_spans: &explicit,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let eff: imbl::HashMap<Dot, _> = derive_full_effective(&tree, &src).into_iter().collect();
        let runs = derive_runs(&tree, &eff);
        assert_eq!(runs.len(), 1, "상속 FontSize 동일 → 한 run");
        assert_eq!(
            runs[0].leaves,
            vec![Dot::new(1, 2), Dot::new(1, 3), Dot::new(1, 4)]
        );
        assert_eq!(
            runs[0].modifiers.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    fn arb_modifier() -> impl proptest::prelude::Strategy<Value = Modifier> {
        use proptest::prelude::*;
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            any::<u32>().prop_map(|v| Modifier::FontSize { value: v })
        ]
    }

    proptest::proptest! {
        #[test]
        fn runs_partition_all_leaves_in_order(
            ops in proptest::collection::vec(
                (0usize..5, 0usize..5,
                 proptest::prop_oneof![proptest::prelude::Just(Bias::Before), proptest::prelude::Just(Bias::After)],
                 proptest::prop_oneof![proptest::prelude::Just(Bias::Before), proptest::prelude::Just(Bias::After)],
                 arb_modifier(), proptest::prelude::any::<bool>()), 0..16),
        ) {
            let mut elems = vec![
                (Dot::new(1, 1), SeqItem::Block { node_type: NodeType::Paragraph, parents: vec![Dot::ROOT] }),
            ];
            for (i, ch) in "abcde".chars().enumerate() {
                elems.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
            }
            let log = oplog(&elems);
            let (els, resolver) = checkout_with_resolver(&log);
            let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
            let mut spans = SpanLog::new();
            for (i, (si, ei, sb, eb, m, is_rm)) in ops.into_iter().enumerate() {
                let start = Anchor { id: Dot::new(1, 2 + si as u64), bias: sb };
                let end = Anchor { id: Dot::new(1, 2 + ei as u64), bias: eb };
                let op = if is_rm { SpanOp::RemoveSpan { start, end, modifier_type: m.as_type() } }
                         else { SpanOp::AddSpan { start, end, modifier: m } };
                spans = spans.apply(Dot::new(9, i as u64), op).unwrap();
            }
            let eff = eff_map(&els, &tree, &resolver, &spans);
            let runs = derive_runs(&tree, &eff);

            let run_leaves: Vec<Dot> = runs.iter().flat_map(|r| r.leaves.iter().copied()).collect();
            let all_leaves: Vec<Dot> = crate::span::leaves_with_paths(&tree).into_iter().map(|(_, d)| d).collect();
            proptest::prop_assert_eq!(&run_leaves, &all_leaves);

            let empty = LeafEff::default();
            for r in &runs {
                for d in &r.leaves {
                    proptest::prop_assert_eq!(&r.modifiers, &**eff.get(d).unwrap_or(&empty));
                }
            }

            for w in runs.windows(2) {
                if w[0].block == w[1].block {
                    proptest::prop_assert_ne!(&w[0].modifiers, &w[1].modifiers);
                }
            }
        }
    }

    fn mods_for(c: u8) -> BTreeMap<ModifierType, Modifier> {
        let mut m = BTreeMap::new();
        match c {
            0 => {}
            1 => {
                m.insert(ModifierType::Bold, Modifier::Bold);
            }
            _ => {
                m.insert(ModifierType::Italic, Modifier::Italic);
            }
        }
        m
    }

    #[test]
    fn splice_atom_forces_split_and_delete_merges() {
        let block = Dot::new(7, 0);
        let mut runs = BlockRuns::new();
        let bold = LeafEff::new(mods_for(1));
        runs.splice_insert(block, 0, Dot::new(1, 0), bold.clone(), false);
        runs.splice_insert(block, 1, Dot::new(1, 1), bold.clone(), false);
        assert_eq!(runs.iter_block(block).len(), 1);
        runs.splice_insert(block, 1, Dot::new(1, 2), LeafEff::default(), true);
        let segs = runs.iter_block(block);
        assert_eq!(segs.len(), 3);
        assert!(!segs[1].mergeable);
        runs.splice_delete(block, 1);
        assert_eq!(runs.iter_block(block).len(), 1);
        assert_eq!(runs.iter_block(block)[0].leaves.len(), 2);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn run_splice_matches_resegment(
            steps in proptest::collection::vec(
                (proptest::prelude::any::<bool>(), proptest::prelude::any::<u8>(), 0u8..3u8),
                0..40),
        ) {
            let block = Dot::new(7, 0);
            let mut runs = BlockRuns::new();
            let mut model: Vec<(Dot, LeafEff)> = Vec::new();
            let mut next_leaf = 0u64;
            for (is_del, raw, modc) in steps {
                let count = model.len();
                if is_del && count > 0 {
                    let off = (raw as usize) % count;
                    model.remove(off);
                    runs.splice_delete(block, off);
                } else {
                    let off = (raw as usize) % (count + 1);
                    let leaf = Dot::new(1, next_leaf);
                    next_leaf += 1;
                    let eff = LeafEff::new(mods_for(modc));
                    model.insert(off, (leaf, eff.clone()));
                    runs.splice_insert(block, off, leaf, eff, false);
                }
                let node = BlockNode {
                    id: block,
                    node_type: NodeType::Paragraph,
                    children: model
                        .iter()
                        .map(|(d, _)| Child::Leaf {
                            id: *d,
                            item: SeqItem::Char('x'),
                        })
                        .collect(),
                };
                let effective: imbl::HashMap<Dot, LeafEff> =
                    model.iter().map(|(d, m)| (*d, m.clone())).collect();
                let expected = resegment_block(&node, &effective);
                proptest::prop_assert_eq!(runs.iter_block(block), expected);
            }
        }
    }
}

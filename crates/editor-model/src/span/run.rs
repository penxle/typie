use std::collections::{BTreeMap, HashMap, HashSet};

use editor_crdt::Dot;
use strum::IntoEnumIterator;

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

pub fn derive_runs(
    tree: &BlockTree,
    effective: &HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
) -> Vec<Run> {
    fn walk(
        node: &BlockNode,
        effective: &HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
        targets: &HashSet<NodeType>,
        out: &mut Vec<Run>,
    ) {
        let mut run: Option<Run> = None;
        for c in &node.children {
            match c {
                Child::Leaf { id: d, item } => {
                    let eff = effective.get(d).cloned().unwrap_or_default();
                    if targets.contains(&item.as_child_type()) {
                        match &mut run {
                            Some(r) if r.modifiers == eff => r.leaves.push(*d),
                            _ => {
                                if let Some(r) = run.take() {
                                    out.push(r);
                                }
                                run = Some(Run {
                                    block: node.id,
                                    leaves: vec![*d],
                                    modifiers: eff,
                                });
                            }
                        }
                    } else {
                        if let Some(r) = run.take() {
                            out.push(r);
                        }
                        out.push(Run {
                            block: node.id,
                            leaves: vec![*d],
                            modifiers: eff,
                        });
                    }
                }
                Child::Block(b) => {
                    if let Some(r) = run.take() {
                        out.push(r);
                    }
                    walk(b, effective, targets, out);
                }
            }
        }
        if let Some(r) = run {
            out.push(r);
        }
    }
    let targets = modifier_target_types();
    let mut out = Vec::new();
    for r in &tree.roots {
        walk(r, effective, &targets, &mut out);
    }
    out
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
    ) -> HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
        derive_effective(elems, tree, resolver, spans)
            .into_iter()
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let eff: HashMap<Dot, _> = derive_full_effective(&tree, &src).into_iter().collect();
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
            let tree = normalize(project_blocks(&els).unwrap());
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

            let empty: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
            for r in &runs {
                for d in &r.leaves {
                    proptest::prop_assert_eq!(&r.modifiers, eff.get(d).unwrap_or(&empty));
                }
            }

            for w in runs.windows(2) {
                if w[0].block == w[1].block {
                    proptest::prop_assert_ne!(&w[0].modifiers, &w[1].modifiers);
                }
            }
        }
    }
}

use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use editor_crdt::sequence::SeqResolve;

use super::{SpanLog, SpanOp};
use crate::seq::{BlockNode, BlockTree, Child, SeqItem, anchor_dot};
use crate::{Modifier, ModifierType, NodeType, Schema};

pub fn leaves_with_paths(tree: &BlockTree) -> Vec<(Vec<NodeType>, Dot)> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        path: &mut Vec<NodeType>,
        out: &mut Vec<(Vec<NodeType>, Dot)>,
    ) {
        path.push(node.node_type);
        for c in &node.children {
            match c {
                Child::Block(id) => {
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, path, out);
                    }
                }
                Child::Leaf { id, item } => {
                    if let Some(t) = item.as_child_type() {
                        let mut p = path.clone();
                        p.push(t);
                        out.push((p, *id));
                    }
                }
            }
        }
        path.pop();
    }
    let mut out = Vec::new();
    let mut path = Vec::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, &mut path, &mut out);
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafContext {
    pub block_path: Vec<(NodeType, Option<Dot>)>,
    pub leaf_type: NodeType,
    pub leaf_dot: Dot,
}

pub fn leaves_with_context(tree: &BlockTree) -> Vec<LeafContext> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        path: &mut Vec<(NodeType, Option<Dot>)>,
        out: &mut Vec<LeafContext>,
    ) {
        let dot = anchor_dot(node.id);
        path.push((node.node_type, dot));
        for c in &node.children {
            match c {
                Child::Block(id) => {
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, path, out);
                    }
                }
                Child::Leaf { id, item } => {
                    if let Some(t) = item.as_child_type() {
                        out.push(LeafContext {
                            block_path: path.clone(),
                            leaf_type: t,
                            leaf_dot: *id,
                        });
                    }
                }
            }
        }
        path.pop();
    }
    let mut out = Vec::new();
    let mut path = Vec::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, &mut path, &mut out);
    }
    out
}

/// Visit every leaf in document order, passing the *borrowed* block path (ancestors
/// including the leaf's parent block), the leaf's child type, and its dot. Unlike
/// `leaves_with_context`, this clones nothing per leaf — callers that only need the
/// path transiently avoid one allocation per character.
pub fn for_each_leaf(
    tree: &BlockTree,
    mut f: impl FnMut(&[(NodeType, Option<Dot>)], NodeType, Dot),
) {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        path: &mut Vec<(NodeType, Option<Dot>)>,
        f: &mut impl FnMut(&[(NodeType, Option<Dot>)], NodeType, Dot),
    ) {
        path.push((node.node_type, anchor_dot(node.id)));
        for c in &node.children {
            match c {
                Child::Block(id) => {
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, path, f);
                    }
                }
                Child::Leaf { id, item } => {
                    if let Some(t) = item.as_child_type() {
                        f(path, t, *id);
                    }
                }
            }
        }
        path.pop();
    }
    let mut path = Vec::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, &mut path, &mut f);
    }
}

/// The explicit modifier a span op contributes when it wins last-writer-wins for its
/// modifier type. `RemoveSpan` cancels back to absence (`None`): it masks the older
/// ops it beat but leaves no entry, so resolution falls through to node styles and
/// inheritance.
pub(crate) fn span_op_effect(op: &SpanOp) -> (ModifierType, Option<Modifier>) {
    match op {
        SpanOp::AddSpan { modifier, .. } => (modifier.as_type(), Some(modifier.clone())),
        SpanOp::RemoveSpan { modifier_type, .. } => (*modifier_type, None),
    }
}

pub fn derive_explicit_effect(
    elements: &[(Dot, SeqItem)],
    tree: &BlockTree,
    resolver: &impl SeqResolve,
    spans: &SpanLog,
) -> Vec<(Dot, BTreeMap<ModifierType, Modifier>)> {
    // No spans, or no visible leaves → every leaf's explicit effect is empty. Emit the
    // empty entries directly, skipping the `O(all spans)` resolution (which a reproject
    // over a select-all-deleted document would otherwise pay for nothing).
    if spans.is_empty() || elements.is_empty() {
        return leaves_with_paths(tree)
            .into_iter()
            .map(|(_, dot)| (dot, BTreeMap::new()))
            .collect();
    }
    let pos_of: HashMap<Dot, usize> = elements
        .iter()
        .enumerate()
        .map(|(i, (d, _))| (*d, i))
        .collect();

    struct Resolved {
        op_dot: Dot,
        ty: ModifierType,
        effect: Option<Modifier>,
        start: usize,
        end: usize,
    }
    let resolved: Vec<Resolved> = spans
        .iter()
        .filter_map(|(op_dot, op)| {
            let (sa, ea) = op.anchors();
            let s = resolver.resolve_boundary(sa.id, sa.bias.into())?.position;
            let e = resolver.resolve_boundary(ea.id, ea.bias.into())?.position;
            if s >= e {
                return None;
            }
            let (ty, effect) = span_op_effect(op);
            Some(Resolved {
                op_dot: *op_dot,
                ty,
                effect,
                start: s,
                end: e,
            })
        })
        .collect();

    leaves_with_paths(tree)
        .into_iter()
        .map(|(path, dot)| {
            let pos = pos_of[&dot];
            let mut by_type: HashMap<ModifierType, (Dot, Option<Modifier>)> = HashMap::new();
            for r in &resolved {
                if !(r.start <= pos && pos < r.end) {
                    continue;
                }
                if !Schema::modifier_spec(r.ty).target.matches(&path) {
                    continue;
                }
                let win = match by_type.get(&r.ty) {
                    Some((cur, _)) => r.op_dot > *cur,
                    None => true,
                };
                if win {
                    by_type.insert(r.ty, (r.op_dot, r.effect.clone()));
                }
            }
            let ex: BTreeMap<ModifierType, Modifier> = by_type
                .into_iter()
                .filter_map(|(t, (_, e))| e.map(|e| (t, e)))
                .collect();
            (dot, ex)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seq::project_blocks;

    fn para_chars(chars: &str) -> Vec<(Dot, SeqItem)> {
        let mut out = vec![(
            Dot::new(1, 1),
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::ROOT],
                attrs: vec![],
            },
        )];
        for (i, ch) in chars.chars().enumerate() {
            out.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        out
    }

    #[test]
    fn leaves_carry_full_path() {
        let elems = para_chars("ab");
        let tree = BlockTree::from_raw(&project_blocks(&elems).unwrap());
        let leaves = leaves_with_paths(&tree);
        assert_eq!(leaves.len(), 2);
        assert_eq!(
            leaves[0].0,
            vec![NodeType::Root, NodeType::Paragraph, NodeType::Text]
        );
        assert_eq!(leaves[0].1, Dot::new(1, 2));
        assert_eq!(leaves[1].1, Dot::new(1, 3));
    }

    use crate::seq::normalize;
    use editor_crdt::sequence::checkout_with_resolver;
    use editor_crdt::{InputEvent, ListOp, build_oplog};

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

    fn anc(id: Dot, bias: super::super::Bias) -> super::super::Anchor {
        super::super::Anchor { id, bias }
    }

    #[test]
    fn remove_span_cancels_winning_add_to_absent() {
        let elems = para_chars("ab");
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let a = Dot::new(1, 2);
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                SpanOp::RemoveSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier_type: ModifierType::Bold,
                },
            )
            .unwrap();
        let ex: BTreeMap<Dot, _> = derive_explicit_effect(&els, &tree, &resolver, &spans)
            .into_iter()
            .collect();
        assert!(
            !ex[&a].contains_key(&ModifierType::Bold),
            "winning RemoveSpan cancels back to absent, not Clear"
        );
    }

    #[test]
    fn leaf_context_has_block_dot_ancestry() {
        let elems = para_chars("a");
        let tree = BlockTree::from_raw(&normalize(project_blocks(&elems).unwrap()));
        let ctxs = leaves_with_context(&tree);
        assert_eq!(ctxs.len(), 1);
        let c = &ctxs[0];
        assert_eq!(c.leaf_type, NodeType::Text);
        assert_eq!(c.leaf_dot, Dot::new(1, 2));
        assert_eq!(
            c.block_path,
            vec![
                (NodeType::Root, Some(Dot::ROOT)),
                (NodeType::Paragraph, Some(Dot::new(1, 1))),
            ]
        );
    }

    use crate::{ModifierAttrLog, ModifierAttrOp};

    fn full(
        elems: &[(Dot, SeqItem)],
        spans: &SpanLog,
        attrs: &ModifierAttrLog,
    ) -> BTreeMap<Dot, BTreeMap<ModifierType, Modifier>> {
        use crate::span::{EffectiveSources, derive_explicit_effect};
        use std::collections::HashMap;
        let log = oplog(elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let explicit: HashMap<Dot, _> = derive_explicit_effect(&els, &tree, &resolver, spans)
            .into_iter()
            .collect();
        let node_attrs: imbl::HashMap<Dot, crate::nodes::Node> = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: attrs,
            explicit_spans: &explicit,
            node_attrs: &node_attrs,
        };
        let mut out = BTreeMap::new();
        for_each_leaf(&tree, |path, leaf_type, leaf_dot| {
            let e = crate::span::resolve_effective(path, Some(leaf_dot), leaf_type, true, &src);
            out.insert(leaf_dot, e);
        });
        out
    }

    fn full_blocks(
        elems: &[(Dot, SeqItem)],
        spans: &SpanLog,
        attrs: &ModifierAttrLog,
    ) -> BTreeMap<Dot, BTreeMap<ModifierType, Modifier>> {
        use crate::span::{EffectiveSources, derive_block_effective, derive_explicit_effect};
        use std::collections::HashMap;
        let log = oplog(elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let explicit: HashMap<Dot, _> = derive_explicit_effect(&els, &tree, &resolver, spans)
            .into_iter()
            .collect();
        let node_attrs: imbl::HashMap<Dot, crate::nodes::Node> = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: attrs,
            explicit_spans: &explicit,
            node_attrs: &node_attrs,
        };
        derive_block_effective(&tree, &src).into_iter().collect()
    }

    #[test]
    fn doc_default_inherits_to_all_chars() {
        let elems = para_chars("ab");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        assert_eq!(
            m[&Dot::new(1, 2)].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert_eq!(
            m[&Dot::new(1, 3)].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn explicit_span_overrides_inherited() {
        let elems = para_chars("ab");
        let a = Dot::new(1, 2);
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier: Modifier::FontSize { value: 1200 },
                },
            )
            .unwrap();
        let m = full(&elems, &spans, &attrs);
        assert_eq!(
            m[&a].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 })
        );
        assert_eq!(
            m[&Dot::new(1, 3)].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn remove_span_lets_inheritance_show_through() {
        let elems = para_chars("ab");
        let a = Dot::new(1, 2);
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier: Modifier::FontSize { value: 1200 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                SpanOp::RemoveSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier_type: ModifierType::FontSize,
                },
            )
            .unwrap();
        let m = full(&elems, &spans, &attrs);
        assert_eq!(
            m[&a].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 }),
            "cancelling the inline override falls back to the inherited value"
        );
    }

    #[test]
    fn invalid_source_context_not_inherited() {
        let elems = para_chars("a");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        assert!(
            !m[&Dot::new(1, 2)].contains_key(&ModifierType::Bold),
            "Bold-on-Root는 무효 source"
        );
    }

    #[test]
    fn root_text_color_record_not_inherited() {
        let elems = para_chars("a");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::TextColor {
                        value: "red".to_string(),
                    },
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        assert!(
            !m[&Dot::new(1, 2)].contains_key(&ModifierType::TextColor),
            "TextColor-on-Root는 out-of-context source(비상속·Root context 아님)"
        );
    }

    #[test]
    fn alignment_inherits_from_root_record() {
        let elems = para_chars("a");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let m = full_blocks(&elems, &SpanLog::new(), &attrs);
        assert_eq!(
            m[&Dot::new(1, 1)].get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: crate::Alignment::Center
            }),
            "the paragraph block inherits the Root's alignment; there is no consumer between the paragraph and the root"
        );
    }

    #[test]
    fn paragraph_indent_terminates_at_paragraph_consumer() {
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('가')),
            (
                Dot::new(2, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(2, 2), SeqItem::Char('나')),
        ];
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 1),
                    modifier: Modifier::ParagraphIndent { value: 200 },
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        assert!(
            !m[&Dot::new(1, 2)].contains_key(&ModifierType::ParagraphIndent),
            "ParagraphIndent's consumer is a root-direct Paragraph, so the paragraph's own record does not pass down to its text carrier"
        );
        assert!(
            !m[&Dot::new(2, 2)].contains_key(&ModifierType::ParagraphIndent),
            "기록·상속 없는 형제 문단은 sparse map에서 None"
        );
    }

    #[test]
    fn attr_clear_falls_back_to_ancestor() {
        let elems = para_chars("a");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(5, 1),
                ModifierAttrOp::ClearModifier {
                    target: Dot::new(1, 1),
                    key: ModifierType::FontSize,
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        assert_eq!(
            m[&Dot::new(1, 2)].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 }),
            "Paragraph attr clear는 own만, Root 상속 계속"
        );
    }

    #[test]
    fn nearest_ancestor_wins() {
        let elems = para_chars("a");
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::LineHeight { value: 200 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(5, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 1),
                    modifier: Modifier::LineHeight { value: 240 },
                },
            )
            .unwrap();
        let m = full_blocks(&elems, &SpanLog::new(), &attrs);
        assert_eq!(
            m[&Dot::new(1, 1)].get(&ModifierType::LineHeight),
            Some(&Modifier::LineHeight { value: 240 }),
            "the paragraph's own LineHeight wins over the Root's on the paragraph block (nearest ancestor)"
        );
    }

    #[test]
    fn inherited_font_size_reaches_tab_but_block_bold_record_does_not() {
        use crate::seq::AtomLeaf;
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::Tab)),
        ];
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(5, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 1),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let m = full(&elems, &SpanLog::new(), &attrs);
        let tab = Dot::new(1, 3);
        assert_eq!(
            m[&tab].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert!(
            !m[&tab].contains_key(&ModifierType::Bold),
            "Bold is non-inheritable; a Paragraph block Bold record does not reach carriers"
        );
        assert!(
            !m[&Dot::new(1, 2)].contains_key(&ModifierType::Bold),
            "Bold is non-inheritable; a Paragraph block Bold record does not reach its text"
        );
    }

    #[test]
    fn rootless_sequence_derives_without_panic() {
        let elems = vec![
            (
                Dot::new(1, 0),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 1), SeqItem::Char('a')),
        ];
        let attrs = ModifierAttrLog::new()
            .apply(
                Dot::new(5, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 0),
                    modifier: Modifier::LineHeight { value: 200 },
                },
            )
            .unwrap();
        let m = full_blocks(&elems, &SpanLog::new(), &attrs);
        assert_eq!(
            m[&Dot::new(1, 0)].get(&ModifierType::LineHeight),
            Some(&Modifier::LineHeight { value: 200 }),
            "real Paragraph 상속(Derived Root skip, panic 없음)"
        );
    }

    #[test]
    fn bold_span_covers_inner_chars() {
        let elems = para_chars("abc");
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let b = Dot::new(1, 3);
        let c = Dot::new(1, 4);
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(b, super::super::Bias::Before),
                    end: anc(c, super::super::Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let eff = derive_explicit_effect(&els, &tree, &resolver, &spans);
        let by_dot: BTreeMap<Dot, BTreeMap<ModifierType, Modifier>> = eff.into_iter().collect();
        assert!(by_dot[&Dot::new(1, 2)].is_empty(), "'a' bold 아님");
        assert_eq!(by_dot[&b].get(&ModifierType::Bold), Some(&Modifier::Bold));
        assert_eq!(by_dot[&c].get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn remove_span_wins_with_higher_dot() {
        let elems = para_chars("a");
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let a = Dot::new(1, 2);
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                SpanOp::RemoveSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(a, super::super::Bias::After),
                    modifier_type: ModifierType::Bold,
                },
            )
            .unwrap();
        let eff = derive_explicit_effect(&els, &tree, &resolver, &spans);
        let by_dot: BTreeMap<Dot, _> = eff.into_iter().collect();
        assert!(by_dot[&a].is_empty(), "higher-Dot Remove가 Bold 제거");
    }

    #[test]
    fn degenerate_span_covers_nothing() {
        let elems = para_chars("abc");
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let b = Dot::new(1, 3);
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(b, super::super::Bias::After),
                    end: anc(b, super::super::Bias::Before),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        for (_, eff) in derive_explicit_effect(&els, &tree, &resolver, &spans) {
            assert!(eff.is_empty(), "degenerate span은 아무 modifier도 안 입힘");
        }
    }

    #[test]
    fn font_size_and_bold_both_target_tab() {
        use crate::seq::AtomLeaf;
        let elems = vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Atom(AtomLeaf::Tab)),
        ];
        let log = oplog(&elems);
        let (els, resolver) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let a = Dot::new(1, 2);
        let tab = Dot::new(1, 3);
        let spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(tab, super::super::Bias::After),
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 1),
                SpanOp::AddSpan {
                    start: anc(a, super::super::Bias::Before),
                    end: anc(tab, super::super::Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let by_dot: BTreeMap<Dot, _> = derive_explicit_effect(&els, &tree, &resolver, &spans)
            .into_iter()
            .collect();
        assert_eq!(
            by_dot[&tab].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert_eq!(
            by_dot[&tab].get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "tabs carry the 10 non-link/ruby kinds, Bold included"
        );
        assert_eq!(
            by_dot[&a].get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert_eq!(by_dot[&a].get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    fn reference_effective(
        elements: &[(Dot, SeqItem)],
        tree: &BlockTree,
        resolver: &editor_crdt::sequence::BoundaryResolver,
        spans: &SpanLog,
    ) -> BTreeMap<Dot, BTreeMap<ModifierType, Modifier>> {
        let pos_of: BTreeMap<Dot, usize> = elements
            .iter()
            .enumerate()
            .map(|(i, (d, _))| (*d, i))
            .collect();
        let mut out = BTreeMap::new();
        for (path, dot) in leaves_with_paths(tree) {
            let pos = pos_of[&dot];
            let mut bucket: BTreeMap<ModifierType, Vec<(Dot, Option<Modifier>)>> = BTreeMap::new();
            for (op_dot, op) in spans.iter() {
                let (sa, ea) = op.anchors();
                let (Some(sb), Some(eb)) = (
                    resolver.resolve_boundary(sa.id, sa.bias.into()),
                    resolver.resolve_boundary(ea.id, ea.bias.into()),
                ) else {
                    continue;
                };
                if sb.position >= eb.position {
                    continue;
                }
                let covered: std::collections::BTreeSet<usize> =
                    (sb.position..eb.position).collect();
                if !covered.contains(&pos) {
                    continue;
                }
                let (ty, value) = match op {
                    SpanOp::AddSpan { modifier, .. } => {
                        (modifier.as_type(), Some(modifier.clone()))
                    }
                    SpanOp::RemoveSpan { modifier_type, .. } => (*modifier_type, None),
                };
                if !Schema::modifier_spec(ty).target.matches(&path) {
                    continue;
                }
                bucket.entry(ty).or_default().push((*op_dot, value));
            }
            let mut eff = BTreeMap::new();
            for (ty, mut covering) in bucket {
                covering.sort_by_key(|(d, _)| *d);
                if let Some((_, Some(m))) = covering.last() {
                    eff.insert(ty, m.clone());
                }
            }
            out.insert(dot, eff);
        }
        out
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
        fn derive_matches_reference(
            ops in proptest::collection::vec(
                (0usize..5, 0usize..5, proptest::prop_oneof![proptest::prelude::Just(super::super::Bias::Before), proptest::prelude::Just(super::super::Bias::After)],
                 proptest::prop_oneof![proptest::prelude::Just(super::super::Bias::Before), proptest::prelude::Just(super::super::Bias::After)], arb_modifier(), 0u8..2),
                0..16),
        ) {
            let elems = para_chars("abcde");
            let log = oplog(&elems);
            let (els, resolver) = checkout_with_resolver(&log);
            let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
            let mut spans = SpanLog::new();
            for (i, (si, ei, sb, eb, m, kind)) in ops.into_iter().enumerate() {
                let start = anc(Dot::new(1, 2 + si as u64), sb);
                let end = anc(Dot::new(1, 2 + ei as u64), eb);
                let op = match kind {
                    0 => SpanOp::AddSpan { start, end, modifier: m },
                    _ => SpanOp::RemoveSpan { start, end, modifier_type: m.as_type() },
                };
                spans = spans.apply(Dot::new(9, i as u64), op).unwrap();
            }
            let got: BTreeMap<Dot, _> = derive_explicit_effect(&els, &tree, &resolver, &spans).into_iter().collect();
            let want = reference_effective(&els, &tree, &resolver, &spans);
            proptest::prop_assert_eq!(got, want);
        }

        #[test]
        fn derive_is_deterministic(seed in proptest::prelude::any::<u64>()) {
            let _ = seed;
            let elems = para_chars("abc");
            let log = oplog(&elems);
            let (els, resolver) = checkout_with_resolver(&log);
            let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
            let spans = SpanLog::new().apply(Dot::new(2, 0), SpanOp::AddSpan {
                start: anc(Dot::new(1, 2), super::super::Bias::Before),
                end: anc(Dot::new(1, 4), super::super::Bias::After), modifier: Modifier::Italic }).unwrap();
            proptest::prop_assert_eq!(derive_explicit_effect(&els, &tree, &resolver, &spans),
                            derive_explicit_effect(&els, &tree, &resolver, &spans));
        }
    }
}

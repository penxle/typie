use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use strum::IntoEnumIterator;

use crate::nodes::Node;
use crate::seq::{BlockNode, BlockTree, Child, anchor_dot};
use crate::{Alignment, Modifier, ModifierAttrLog, ModifierType, NodeType, Schema};

pub struct EffectiveSources<'a> {
    pub block_modifiers: &'a ModifierAttrLog,
    pub explicit_spans: &'a HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub node_attrs: &'a imbl::HashMap<Dot, Node>,
}

pub(crate) fn is_table_justify(node_type: NodeType, m: &Modifier) -> bool {
    node_type == NodeType::Table
        && matches!(
            m,
            Modifier::Alignment {
                value: Alignment::Justify
            }
        )
}

pub(crate) fn own_effect(
    dot: Option<Dot>,
    node_type: NodeType,
    type_path: &[NodeType],
    is_leaf: bool,
    src: &EffectiveSources,
) -> BTreeMap<ModifierType, Modifier> {
    let mut out: BTreeMap<ModifierType, Modifier> = BTreeMap::new();

    if is_leaf && Schema::node_spec(node_type).inline {
        if let Some(d) = dot
            && let Some(ex) = src.explicit_spans.get(&d)
        {
            for (ty, m) in ex {
                if !m.is_valid() {
                    continue;
                }
                out.insert(*ty, m.clone());
            }
        }
    } else if let Some(d) = dot {
        for (ty, m) in src.block_modifiers.modifiers_of(d) {
            if Schema::modifier_spec(ty).context.matches(type_path)
                && m.is_valid()
                && !is_table_justify(node_type, &m)
            {
                out.entry(ty).or_insert(m);
            }
        }
    }

    let node = dot
        .and_then(|d| src.node_attrs.get(&d).cloned())
        .unwrap_or_else(|| node_type.into_node());
    for m in node.implicit_modifiers() {
        out.entry(m.as_type()).or_insert(m.clone());
    }

    out
}

pub fn resolve_effective(
    block_path: &[(NodeType, Option<Dot>)],
    self_dot: Option<Dot>,
    self_type: NodeType,
    is_leaf: bool,
    src: &EffectiveSources,
) -> BTreeMap<ModifierType, Modifier> {
    let mut self_path: Vec<NodeType> = block_path.iter().map(|(t, _)| *t).collect();
    self_path.push(self_type);
    let self_own = own_effect(self_dot, self_type, &self_path, is_leaf, src);

    let ancestors: Vec<_> = (0..block_path.len())
        .map(|i| {
            let (atype, adot) = block_path[i];
            let apath: Vec<NodeType> = block_path[0..=i].iter().map(|(t, _)| *t).collect();
            let own: BTreeMap<ModifierType, Modifier> = match adot {
                Some(d) => own_effect(Some(d), atype, &apath, false, src),
                None => BTreeMap::new(),
            };
            (adot, apath, own)
        })
        .collect();

    let inherited = |ty: ModifierType, inheritable: bool| -> Option<Modifier> {
        let target = &Schema::modifier_spec(ty).target;
        for (adot, apath, own) in ancestors.iter().rev() {
            if inheritable {
                if target.matches(apath) {
                    return None;
                }
                if adot.is_none() {
                    continue;
                }
                if let Some(m) = own.get(&ty) {
                    return Some(m.clone());
                }
            } else {
                if adot.is_none() {
                    continue;
                }
                if target.matches(apath) {
                    return own.get(&ty).cloned();
                }
            }
        }
        None
    };

    let ancestor_implicit = |ty: ModifierType| -> Option<Modifier> {
        if !is_leaf {
            return None;
        }
        for (atype, adot) in block_path.iter().rev() {
            let node = adot
                .and_then(|d| src.node_attrs.get(&d).cloned())
                .unwrap_or_else(|| (*atype).into_node());
            if let Some(m) = node.implicit_modifiers().iter().find(|m| m.as_type() == ty) {
                return Some(m.clone());
            }
        }
        None
    };

    let mut out = BTreeMap::new();
    for ty in ModifierType::iter() {
        let spec = Schema::modifier_spec(ty);
        let is_target_self = spec.target.matches(&self_path);
        let val = if spec.inheritable {
            self_own.get(&ty).cloned().or_else(|| inherited(ty, true))
        } else if is_target_self {
            self_own.get(&ty).cloned().or_else(|| ancestor_implicit(ty))
        } else {
            inherited(ty, false).or_else(|| ancestor_implicit(ty))
        };
        if let Some(m) = val {
            out.insert(ty, m);
        }
    }
    out
}

pub(crate) struct BlockContext {
    pub block_path: Vec<(NodeType, Option<Dot>)>,
    pub self_id: Dot,
    pub self_dot: Option<Dot>,
    pub self_type: NodeType,
}

pub(crate) fn blocks_with_context(tree: &BlockTree) -> Vec<BlockContext> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        path: &mut Vec<(NodeType, Option<Dot>)>,
        out: &mut Vec<BlockContext>,
    ) {
        out.push(BlockContext {
            block_path: path.clone(),
            self_id: node.id,
            self_dot: anchor_dot(node.id),
            self_type: node.node_type,
        });
        path.push((node.node_type, anchor_dot(node.id)));
        for c in &node.children {
            if let Child::Block(id) = c
                && let Some(b) = tree.get(*id)
            {
                walk(tree, b, path, out);
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

pub fn derive_block_effective(
    tree: &BlockTree,
    src: &EffectiveSources,
) -> imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
    blocks_with_context(tree)
        .into_iter()
        .map(|ctx| {
            let eff = resolve_effective(&ctx.block_path, ctx.self_dot, ctx.self_type, false, src);
            (ctx.self_id, eff)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnModifier {
    pub value: Modifier,
}

/// Own (non-inherited) modifiers of a single leaf: its explicit span effects.
/// Returns an empty map when the leaf carries none.
pub fn own_modifiers_for_leaf(
    leaf_dot: Dot,
    src: &EffectiveSources,
) -> BTreeMap<ModifierType, OwnModifier> {
    let mut own: BTreeMap<ModifierType, OwnModifier> = BTreeMap::new();
    if let Some(ex) = src.explicit_spans.get(&leaf_dot) {
        for (ty, m) in ex {
            if !m.is_valid() {
                continue;
            }
            own.insert(*ty, OwnModifier { value: m.clone() });
        }
    }
    own
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Per-leaf own-modifier maps for the whole tree (only non-empty entries),
    /// via `own_modifiers_for_leaf`.
    fn own_map(
        tree: &BlockTree,
        src: &EffectiveSources,
    ) -> HashMap<Dot, BTreeMap<ModifierType, OwnModifier>> {
        let mut out = HashMap::new();
        crate::span::for_each_leaf(tree, |_path, _leaf_type, leaf_dot| {
            let m = own_modifiers_for_leaf(leaf_dot, src);
            if !m.is_empty() {
                out.insert(leaf_dot, m);
            }
        });
        out
    }

    fn sources<'a>(
        bm: &'a ModifierAttrLog,
        spans: &'a HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
        node_attrs: &'a imbl::HashMap<Dot, Node>,
    ) -> EffectiveSources<'a> {
        EffectiveSources {
            block_modifiers: bm,
            explicit_spans: spans,
            node_attrs,
        }
    }

    #[test]
    fn own_effect_block_level_atom_reads_block_modifiers() {
        let img = Dot::new(1, 5);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: img,
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let own = own_effect(
            Some(img),
            NodeType::Image,
            &[NodeType::Root, NodeType::Image],
            true,
            &src,
        );
        assert!(matches!(
            own.get(&ModifierType::Alignment),
            Some(Modifier::Alignment { .. })
        ));
    }

    #[test]
    fn own_effect_inline_char_leaf_still_reads_spans_not_block_modifiers() {
        let leaf = Dot::new(1, 5);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: leaf,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(ModifierType::Bold, Modifier::Bold);
        spans.insert(leaf, e);
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let own = own_effect(Some(leaf), NodeType::Text, &path, true, &src);
        assert_eq!(
            own.get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "inline leaf reads explicit span"
        );
    }

    #[test]
    fn own_effect_block_modifier_is_context_filtered() {
        let root = Dot::new(1, 0);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root];
        let own = own_effect(Some(root), NodeType::Root, &path, false, &src);
        assert!(
            !own.contains_key(&ModifierType::Bold),
            "Bold is not context-valid on Root"
        );
    }

    #[test]
    fn own_effect_block_modifier_out_of_range_value_is_ignored() {
        let root = Dot::new(1, 0);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::FontSize { value: 399 },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root];
        let own = own_effect(Some(root), NodeType::Root, &path, false, &src);
        assert!(
            !own.contains_key(&ModifierType::FontSize),
            "out-of-range value is treated as no record"
        );
    }

    #[test]
    fn own_effect_table_justify_alignment_is_ignored() {
        let table = Dot::new(1, 1);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: table,
                    modifier: Modifier::Alignment {
                        value: Alignment::Justify,
                    },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root, NodeType::Table];
        let own = own_effect(Some(table), NodeType::Table, &path, false, &src);
        assert!(
            !own.contains_key(&ModifierType::Alignment),
            "Table Justify record is treated as no record"
        );
    }

    #[test]
    fn own_effect_leaf_explicit_span_out_of_range_value_is_ignored() {
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new();
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(ModifierType::FontSize, Modifier::FontSize { value: 50000 });
        spans.insert(leaf, e);
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let own = own_effect(Some(leaf), NodeType::Text, &path, true, &src);
        assert!(
            !own.contains_key(&ModifierType::FontSize),
            "out-of-range explicit span value is treated as no record"
        );
    }

    #[test]
    fn own_effect_block_implicit_from_node_attrs() {
        use crate::nodes::{BlockquoteVariant, Node};
        let blk = Dot::new(1, 1);
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let mut node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
        let mut bq = match NodeType::Blockquote.into_node() {
            Node::Blockquote(b) => b,
            _ => unreachable!(),
        };
        bq.variant = editor_crdt::LwwReg::with_value(BlockquoteVariant::MessageSent);
        node_attrs.insert(blk, Node::Blockquote(bq));
        let src = sources(&bm, &spans, &node_attrs);
        let path = [NodeType::Root, NodeType::Blockquote];
        let own = own_effect(Some(blk), NodeType::Blockquote, &path, false, &src);
        assert_eq!(
            own.get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "bright".to_string()
            })
        );
    }

    use proptest::prelude::*;

    fn arb_mod() -> impl Strategy<Value = Modifier> {
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            any::<u32>().prop_map(|v| Modifier::FontSize { value: v }),
            Just(Modifier::Alignment {
                value: crate::Alignment::Center
            }),
        ]
    }

    fn ref_own_ty(
        dot: Option<Dot>,
        nt: NodeType,
        path: &[NodeType],
        is_leaf: bool,
        ty: ModifierType,
        src: &EffectiveSources,
    ) -> Option<Modifier> {
        if is_leaf && Schema::node_spec(nt).inline {
            if let Some(d) = dot
                && let Some(ex) = src.explicit_spans.get(&d)
                && let Some(m) = ex.get(&ty)
                && m.is_valid()
            {
                return Some(m.clone());
            }
        } else if let Some(d) = dot {
            let bm = src.block_modifiers.modifiers_of(d);
            if let Some(m) = bm.get(&ty)
                && Schema::modifier_spec(ty).context.matches(path)
                && m.is_valid()
                && !is_table_justify(nt, m)
            {
                return Some(m.clone());
            }
        }
        let node = dot
            .and_then(|d| src.node_attrs.get(&d).cloned())
            .unwrap_or_else(|| nt.into_node());
        for m in node.implicit_modifiers() {
            if m.as_type() == ty {
                return Some(m.clone());
            }
        }
        None
    }

    fn ref_effective(
        block_path: &[(NodeType, Option<Dot>)],
        self_dot: Option<Dot>,
        self_type: NodeType,
        is_leaf: bool,
        src: &EffectiveSources,
    ) -> BTreeMap<ModifierType, Modifier> {
        let mut self_path: Vec<NodeType> = block_path.iter().map(|(t, _)| *t).collect();
        self_path.push(self_type);
        let mut out = BTreeMap::new();
        for ty in ModifierType::iter() {
            let spec = Schema::modifier_spec(ty);
            let is_target_self = spec.target.matches(&self_path);
            let inh = |ty: ModifierType| -> Option<Modifier> {
                let target = &Schema::modifier_spec(ty).target;
                for i in (0..block_path.len()).rev() {
                    let (atype, adot) = block_path[i];
                    let apath: Vec<NodeType> = block_path[0..=i].iter().map(|(t, _)| *t).collect();
                    if spec.inheritable {
                        if target.matches(&apath) {
                            return None;
                        }
                        if adot.is_none() {
                            continue;
                        }
                        if let Some(m) = ref_own_ty(adot, atype, &apath, false, ty, src) {
                            return Some(m);
                        }
                    } else {
                        if adot.is_none() {
                            continue;
                        }
                        if target.matches(&apath) {
                            return ref_own_ty(adot, atype, &apath, false, ty, src);
                        }
                    }
                }
                None
            };
            let ancestor_implicit = |ty: ModifierType| -> Option<Modifier> {
                if !is_leaf {
                    return None;
                }
                for (atype, adot) in block_path.iter().rev() {
                    let node = adot
                        .and_then(|d| src.node_attrs.get(&d).cloned())
                        .unwrap_or_else(|| (*atype).into_node());
                    if let Some(m) = node.implicit_modifiers().iter().find(|m| m.as_type() == ty) {
                        return Some(m.clone());
                    }
                }
                None
            };
            let val = if spec.inheritable {
                ref_own_ty(self_dot, self_type, &self_path, is_leaf, ty, src).or_else(|| inh(ty))
            } else if is_target_self {
                ref_own_ty(self_dot, self_type, &self_path, is_leaf, ty, src)
                    .or_else(|| ancestor_implicit(ty))
            } else {
                inh(ty).or_else(|| ancestor_implicit(ty))
            };
            if let Some(m) = val {
                out.insert(ty, m);
            }
        }
        out
    }

    proptest! {
        #[test]
        fn resolve_effective_matches_reference(
            root_mod in proptest::option::of(arb_mod()),
            para_mod in proptest::option::of(arb_mod()),
            leaf_span in proptest::option::of(arb_mod()),
        ) {
            let root = Dot::new(1, 0);
            let para = Dot::new(1, 1);
            let leaf = Dot::new(1, 2);
            let mut bm = ModifierAttrLog::new();
            let mut dc = 0u64;
            if let Some(m) = root_mod {
                bm = bm.apply(Dot::new(7, dc), crate::ModifierAttrOp::SetModifier { target: root, modifier: m }).unwrap();
                dc += 1;
            }
            if let Some(m) = para_mod {
                bm = bm.apply(Dot::new(7, dc), crate::ModifierAttrOp::SetModifier { target: para, modifier: m }).unwrap();
            }
            let mut spans: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
            if let Some(m) = leaf_span {
                let mut e = BTreeMap::new();
                e.insert(m.as_type(), m);
                spans.insert(leaf, e);
            }
            let node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
            let src = EffectiveSources { block_modifiers: &bm, explicit_spans: &spans, node_attrs: &node_attrs };
            let block_path = [(NodeType::Root, Some(root)), (NodeType::Paragraph, Some(para))];
            let got = resolve_effective(&block_path, Some(leaf), NodeType::Text, true, &src);
            let want = ref_effective(&block_path, Some(leaf), NodeType::Text, true, &src);
            prop_assert_eq!(got, want);
        }
    }

    #[test]
    fn resolve_effective_inherits_root_font_size() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Paragraph, Some(para)),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert_eq!(
            eff.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn paragraph_consumes_its_own_alignment_record() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let bp = [(NodeType::Root, Some(root))];
        let eff = resolve_effective(&bp, Some(para), NodeType::Paragraph, false, &src);
        assert_eq!(
            eff.get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: crate::Alignment::Center
            }),
            "a Paragraph consumes Alignment: its own record is its effective value, and does not pass down to its text carriers"
        );
    }

    fn table_alignment_src(bm: &ModifierAttrLog) -> EffectiveSources<'_> {
        static EMPTY_SPANS: std::sync::OnceLock<HashMap<Dot, BTreeMap<ModifierType, Modifier>>> =
            std::sync::OnceLock::new();
        static EMPTY_ATTRS: std::sync::OnceLock<imbl::HashMap<Dot, Node>> =
            std::sync::OnceLock::new();
        EffectiveSources {
            block_modifiers: bm,
            explicit_spans: EMPTY_SPANS.get_or_init(HashMap::new),
            node_attrs: EMPTY_ATTRS.get_or_init(imbl::HashMap::new),
        }
    }

    fn td(seq: u64) -> Dot {
        Dot::new(1, seq)
    }

    fn cell_para_path() -> [(NodeType, Option<Dot>); 4] {
        [
            (NodeType::Root, Some(td(0))),
            (NodeType::Table, Some(td(1))),
            (NodeType::TableRow, Some(td(2))),
            (NodeType::TableCell, Some(td(3))),
        ]
    }

    fn cell_leaf_path() -> [(NodeType, Option<Dot>); 5] {
        [
            (NodeType::Root, Some(td(0))),
            (NodeType::Table, Some(td(1))),
            (NodeType::TableRow, Some(td(2))),
            (NodeType::TableCell, Some(td(3))),
            (NodeType::Paragraph, Some(td(4))),
        ]
    }

    #[test]
    fn alignment_inheritance_terminates_at_table() {
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: td(1),
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Right,
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(7, 1),
                crate::ModifierAttrOp::SetModifier {
                    target: td(0),
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let src = table_alignment_src(&bm);
        let para_eff = resolve_effective(
            &cell_para_path(),
            Some(td(4)),
            NodeType::Paragraph,
            false,
            &src,
        );
        assert_eq!(
            para_eff.get(&ModifierType::Alignment),
            None,
            "Table consumes Alignment, so neither the Table's placement nor the Root's alignment passes into a cell paragraph — it falls back to DEFAULT_ALIGNMENT (Left)"
        );
    }

    #[test]
    fn table_placement_inherits_root_alignment() {
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: td(0),
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let src = table_alignment_src(&bm);
        let bp = [(NodeType::Root, Some(td(0)))];
        let table_eff = resolve_effective(&bp, Some(td(1)), NodeType::Table, false, &src);
        assert_eq!(
            table_eff.get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: crate::Alignment::Center
            }),
            "the Table has no consumer between itself and the Root, so its own placement inherits the Root alignment normally"
        );
    }

    #[test]
    fn cell_text_alignment_terminates_no_passthrough() {
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: td(0),
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(7, 1),
                crate::ModifierAttrOp::SetModifier {
                    target: td(1),
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Right,
                    },
                },
            )
            .unwrap();
        let src = table_alignment_src(&bm);
        let leaf_eff =
            resolve_effective(&cell_leaf_path(), Some(td(5)), NodeType::Text, true, &src);
        assert_eq!(
            leaf_eff.get(&ModifierType::Alignment),
            None,
            "cell text alignment terminates at its Paragraph consumer; no pass-through of Table or Root alignment → code default Left"
        );
    }

    #[test]
    fn root_font_size_reaches_cell_char() {
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: td(0),
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let src = table_alignment_src(&bm);
        let leaf_eff =
            resolve_effective(&cell_leaf_path(), Some(td(5)), NodeType::Text, true, &src);
        assert_eq!(
            leaf_eff.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 }),
            "FontSize has no block consumer, so an inheritable non-alignment kind flows through the Table into the cell char unchanged"
        );
    }

    #[test]
    fn table_justify_record_resolves_to_code_default_left() {
        let root = Dot::new(1, 0);
        let table = Dot::new(1, 1);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: table,
                    modifier: Modifier::Alignment {
                        value: Alignment::Justify,
                    },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let bp = [(NodeType::Root, Some(root))];
        let eff = resolve_effective(&bp, Some(table), NodeType::Table, false, &src);
        assert_eq!(
            eff.get(&ModifierType::Alignment),
            None,
            "a Table's own Justify record is treated as no record; consumers fall back to DEFAULT_ALIGNMENT (Left)"
        );
    }

    #[test]
    fn resolve_effective_inherits_through_none_ancestor() {
        let root = Dot::new(1, 0);
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let bp = [(NodeType::Root, Some(root)), (NodeType::Paragraph, None)];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert_eq!(
            eff.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 }),
            "inheritable FontSize flows through the None Paragraph ancestor"
        );
    }

    #[test]
    fn alignment_leaf_terminates_at_structural_paragraph_consumer() {
        let root = Dot::new(1, 0);
        let table = Dot::new(1, 1);
        let leaf = Dot::new(1, 5);
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: table,
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Right,
                    },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Table, Some(table)),
            (NodeType::Paragraph, None),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert_eq!(
            eff.get(&ModifierType::Alignment),
            None,
            "the enclosing Paragraph is an Alignment consumer even without its own record, so the walk terminates there — Table alignment does not pass through → code default Left"
        );
    }

    use crate::seq::{SeqItem, normalize, project_blocks};
    use editor_crdt::{InputEvent, ListOp, build_oplog, sequence::checkout_with_resolver};

    fn oplog_of(items: &[(Dot, SeqItem)]) -> editor_crdt::OpLog<SeqItem> {
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

    #[test]
    fn block_effective_empty_paragraph_inherits_font_size() {
        let para = Dot::new(1, 1);
        let elems = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::ROOT],
            },
        )];
        let log = oplog_of(&elems);
        let (els, _r) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let bm = ModifierAttrLog::new()
            .apply(
                Dot::new(7, 0),
                crate::ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let be = derive_block_effective(&tree, &src);
        assert_eq!(
            be.get(&para).and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert!(be.contains_key(&Dot::ROOT));
    }

    #[test]
    fn fold_title_implicit_text_color_reaches_its_chars() {
        let root = Dot::ROOT;
        let fold = Dot::new(1, 1);
        let title = Dot::new(1, 2);
        let a = Dot::new(1, 3);
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let eff = crate::span::resolve_effective(
            &[
                (NodeType::Root, Some(root)),
                (NodeType::Fold, Some(fold)),
                (NodeType::FoldTitle, Some(title)),
            ],
            Some(a),
            NodeType::Text,
            true,
            &src,
        );
        assert_eq!(
            eff.get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "gray".to_string()
            }),
            "the containing FoldTitle's implicit TextColor applies to its chars"
        );
        assert_eq!(
            eff.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1050 })
        );
    }

    #[test]
    fn message_sent_implicit_text_color_reaches_nested_chars() {
        use crate::nodes::{BlockquoteVariant, Node};
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let a = Dot::new(1, 3);
        let elems = vec![
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, bq],
                },
            ),
            (a, SeqItem::Char('a')),
        ];
        let log = oplog_of(&elems);
        let (els, _r) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let mut node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
        let mut bqn = match NodeType::Blockquote.into_node() {
            Node::Blockquote(n) => n,
            _ => unreachable!(),
        };
        bqn.variant = editor_crdt::LwwReg::with_value(BlockquoteVariant::MessageSent);
        node_attrs.insert(bq, Node::Blockquote(bqn));
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };
        let be = derive_block_effective(&tree, &src);
        assert!(
            be.get(&para)
                .map(|m| !m.contains_key(&ModifierType::TextColor))
                .unwrap_or(true),
            "block records stay sparse; the implicit walk is a carrier-resolution layer, not a block record"
        );
        assert!(
            be.keys().any(|id| id.is_synthetic() && *id != Dot::ROOT),
            "derived blocks get block_effective entries"
        );
        let eff = crate::span::resolve_effective(
            &[
                (NodeType::Root, Some(root)),
                (NodeType::Blockquote, Some(bq)),
                (NodeType::Paragraph, Some(para)),
            ],
            Some(a),
            NodeType::Text,
            true,
            &src,
        );
        assert_eq!(
            eff.get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "bright".to_string()
            }),
            "the ancestor implicit walk carries MessageSent bright to nested chars"
        );
    }

    #[test]
    fn own_modifiers_reports_set_spans_only() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (a, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
        ];
        let log = oplog_of(&elems);
        let (els, _r) = checkout_with_resolver(&log);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));

        let mut spans: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
        spans.insert(a, BTreeMap::from([(ModifierType::Bold, Modifier::Bold)]));
        let bm = ModifierAttrLog::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_attrs: &node_attrs,
        };

        let own = own_map(&tree, &src);
        assert_eq!(
            own.get(&a).and_then(|m| m.get(&ModifierType::Bold)),
            Some(&OwnModifier {
                value: Modifier::Bold
            })
        );
        assert!(
            own.get(&b)
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
    }

    #[test]
    fn own_modifiers_for_leaf_explicit_span_out_of_range_value_is_ignored() {
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new();
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(
            ModifierType::Link,
            Modifier::Link {
                href: String::new(),
            },
        );
        spans.insert(leaf, e);
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_attrs);
        let own = own_modifiers_for_leaf(leaf, &src);
        assert!(
            !own.contains_key(&ModifierType::Link),
            "out-of-range explicit span value is treated as no record"
        );
    }
}

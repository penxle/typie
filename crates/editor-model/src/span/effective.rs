use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use strum::IntoEnumIterator;

use crate::nodes::Node;
use crate::seq::{BlockNode, BlockTree, Child, anchor_dot};
use crate::span::{ExplicitEffect, leaves_with_context};
use crate::{Modifier, ModifierAttrLog, ModifierType, NodeType, Schema, StyleEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnEffect {
    Set(Modifier),
    Clear,
}

pub struct EffectiveSources<'a> {
    pub block_modifiers: &'a ModifierAttrLog,
    pub explicit_spans: &'a HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>>,
    pub node_styles: &'a imbl::HashMap<Dot, Option<String>>,
    pub styles: &'a imbl::HashMap<String, StyleEntry>,
    pub node_attrs: &'a imbl::HashMap<Dot, Node>,
}

pub(crate) fn style_modifiers(
    style: &StyleEntry,
    path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    let mut best: BTreeMap<ModifierType, (Dot, Modifier)> = BTreeMap::new();
    for m in style.modifiers.iter() {
        let ty = m.as_type();
        if !Schema::modifier_spec(ty).context.matches(path) {
            continue;
        }
        let Some(rep) = style.modifiers.tags_for(m).copied().max() else {
            continue;
        };
        match best.get(&ty) {
            Some((cur, _)) if *cur >= rep => {}
            _ => {
                best.insert(ty, (rep, m.clone()));
            }
        }
    }
    best.into_iter().map(|(t, (_, m))| (t, m)).collect()
}

pub(crate) fn own_effect(
    dot: Option<Dot>,
    node_type: NodeType,
    type_path: &[NodeType],
    is_leaf: bool,
    src: &EffectiveSources,
) -> BTreeMap<ModifierType, OwnEffect> {
    let mut out: BTreeMap<ModifierType, OwnEffect> = BTreeMap::new();

    if is_leaf && Schema::node_spec(node_type).inline {
        if let Some(d) = dot
            && let Some(ex) = src.explicit_spans.get(&d)
        {
            for (ty, e) in ex {
                out.insert(
                    *ty,
                    match e {
                        ExplicitEffect::Set(m) => OwnEffect::Set(m.clone()),
                        ExplicitEffect::Clear => OwnEffect::Clear,
                    },
                );
            }
        }
    } else if let Some(d) = dot {
        for (ty, m) in src.block_modifiers.modifiers_of(d) {
            if Schema::modifier_spec(ty).context.matches(type_path) {
                out.entry(ty).or_insert(OwnEffect::Set(m));
            }
        }
    }

    if let Some(d) = dot
        && let Some(Some(sid)) = src.node_styles.get(&d)
        && let Some(style) = src.styles.get(sid)
    {
        for (ty, m) in style_modifiers(style, type_path) {
            out.entry(ty).or_insert(OwnEffect::Set(m));
        }
    }

    let node = dot
        .and_then(|d| src.node_attrs.get(&d).cloned())
        .unwrap_or_else(|| node_type.into_node());
    for m in node.implicit_modifiers() {
        out.entry(m.as_type()).or_insert(OwnEffect::Set(m.clone()));
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
            let own: BTreeMap<ModifierType, OwnEffect> = match adot {
                Some(d) => own_effect(Some(d), atype, &apath, false, src),
                None => BTreeMap::new(),
            };
            (adot, apath, own)
        })
        .collect();

    let inherited = |ty: ModifierType, inheritable: bool| -> Option<Modifier> {
        for (adot, apath, own) in ancestors.iter().rev() {
            if adot.is_none() {
                continue;
            }
            if inheritable {
                if let Some(OwnEffect::Set(m)) = own.get(&ty) {
                    return Some(m.clone());
                }
            } else if Schema::modifier_spec(ty).target.matches(apath) {
                return match own.get(&ty) {
                    Some(OwnEffect::Set(m)) => Some(m.clone()),
                    _ => None,
                };
            }
        }
        None
    };

    let mut out = BTreeMap::new();
    for ty in ModifierType::iter() {
        let spec = Schema::modifier_spec(ty);
        let is_target_self = spec.target.matches(&self_path);
        let val = if spec.inheritable {
            match self_own.get(&ty) {
                Some(OwnEffect::Set(m)) => Some(m.clone()),
                Some(OwnEffect::Clear) => None,
                None => inherited(ty, true),
            }
        } else if is_target_self {
            match self_own.get(&ty) {
                Some(OwnEffect::Set(m)) => Some(m.clone()),
                _ => None,
            }
        } else {
            inherited(ty, false)
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
            if let Child::Block(b) = c {
                walk(b, path, out);
            }
        }
        path.pop();
    }
    let mut out = Vec::new();
    let mut path = Vec::new();
    for r in &tree.roots {
        walk(r, &mut path, &mut out);
    }
    out
}

pub fn derive_block_effective(
    tree: &BlockTree,
    src: &EffectiveSources,
) -> HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
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
    pub from_style: bool,
}

pub fn derive_own_modifiers(
    tree: &BlockTree,
    src: &EffectiveSources,
) -> HashMap<Dot, BTreeMap<ModifierType, OwnModifier>> {
    let mut out: HashMap<Dot, BTreeMap<ModifierType, OwnModifier>> = HashMap::new();
    for ctx in leaves_with_context(tree) {
        let leaf_path: Vec<NodeType> = ctx
            .block_path
            .iter()
            .map(|(t, _)| *t)
            .chain(std::iter::once(ctx.leaf_type))
            .collect();
        let mut own: BTreeMap<ModifierType, Option<OwnModifier>> = BTreeMap::new();
        if let Some(ex) = src.explicit_spans.get(&ctx.leaf_dot) {
            for (ty, e) in ex {
                own.insert(
                    *ty,
                    match e {
                        ExplicitEffect::Set(m) => Some(OwnModifier {
                            value: m.clone(),
                            from_style: false,
                        }),
                        ExplicitEffect::Clear => None,
                    },
                );
            }
        }
        if let Some(Some(sid)) = src.node_styles.get(&ctx.leaf_dot)
            && let Some(style) = src.styles.get(sid)
        {
            for (ty, m) in style_modifiers(style, &leaf_path) {
                own.entry(ty).or_insert(Some(OwnModifier {
                    value: m,
                    from_style: true,
                }));
            }
        }
        let map: BTreeMap<ModifierType, OwnModifier> = own
            .into_iter()
            .filter_map(|(t, o)| o.map(|m| (t, m)))
            .collect();
        if !map.is_empty() {
            out.insert(ctx.leaf_dot, map);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::OrSetOp;

    fn style_with(mods: &[(Dot, Modifier)]) -> StyleEntry {
        let mut s = StyleEntry::new();
        for (d, m) in mods {
            s.modifiers = s
                .modifiers
                .apply(*d, OrSetOp::Add { elem: m.clone() })
                .unwrap();
        }
        s
    }

    #[test]
    fn style_modifiers_picks_highest_dot_per_type() {
        let style = style_with(&[
            (Dot::new(1, 0), Modifier::FontSize { value: 1600 }),
            (Dot::new(2, 0), Modifier::FontSize { value: 1200 }),
        ]);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let got = style_modifiers(&style, &path);
        assert_eq!(
            got.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 }),
            "highest add-Dot (2,0) wins"
        );
    }

    fn sources<'a>(
        bm: &'a ModifierAttrLog,
        spans: &'a HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>>,
        node_styles: &'a imbl::HashMap<Dot, Option<String>>,
        styles: &'a imbl::HashMap<String, StyleEntry>,
        node_attrs: &'a imbl::HashMap<Dot, Node>,
    ) -> EffectiveSources<'a> {
        EffectiveSources {
            block_modifiers: bm,
            explicit_spans: spans,
            node_styles,
            styles,
            node_attrs,
        }
    }

    #[test]
    fn own_effect_leaf_span_clear_suppresses_style() {
        let leaf = Dot::new(1, 5);
        let bm = ModifierAttrLog::new();
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(ModifierType::Bold, ExplicitEffect::Clear);
        spans.insert(leaf, e);
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(leaf, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(9, 0), Modifier::Bold)]),
        );
        let node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_styles, &styles, &node_attrs);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let own = own_effect(Some(leaf), NodeType::Text, &path, true, &src);
        assert_eq!(own.get(&ModifierType::Bold), Some(&OwnEffect::Clear));
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_styles, &styles, &node_attrs);
        let own = own_effect(
            Some(img),
            NodeType::Image,
            &[NodeType::Root, NodeType::Image],
            true,
            &src,
        );
        assert!(matches!(
            own.get(&ModifierType::Alignment),
            Some(OwnEffect::Set(_))
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
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(ModifierType::Bold, ExplicitEffect::Set(Modifier::Bold));
        spans.insert(leaf, e);
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_styles, &styles, &node_attrs);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let own = own_effect(Some(leaf), NodeType::Text, &path, true, &src);
        assert_eq!(
            own.get(&ModifierType::Bold),
            Some(&OwnEffect::Set(Modifier::Bold)),
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = sources(&bm, &spans, &node_styles, &styles, &node_attrs);
        let path = [NodeType::Root];
        let own = own_effect(Some(root), NodeType::Root, &path, false, &src);
        assert!(
            !own.contains_key(&ModifierType::Bold),
            "Bold is not context-valid on Root"
        );
    }

    #[test]
    fn own_effect_block_implicit_from_node_attrs() {
        use crate::nodes::{BlockquoteVariant, Node};
        let blk = Dot::new(1, 1);
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let mut node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
        let mut bq = match NodeType::Blockquote.into_node() {
            Node::Blockquote(b) => b,
            _ => unreachable!(),
        };
        bq.variant = editor_crdt::LwwReg::with_value(BlockquoteVariant::MessageSent);
        node_attrs.insert(blk, Node::Blockquote(bq));
        let src = sources(&bm, &spans, &node_styles, &styles, &node_attrs);
        let path = [NodeType::Root, NodeType::Blockquote];
        let own = own_effect(Some(blk), NodeType::Blockquote, &path, false, &src);
        assert_eq!(
            own.get(&ModifierType::TextColor),
            Some(&OwnEffect::Set(Modifier::TextColor {
                value: "bright".to_string()
            }))
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
    ) -> Option<OwnEffect> {
        if is_leaf && Schema::node_spec(nt).inline {
            if let Some(d) = dot
                && let Some(ex) = src.explicit_spans.get(&d)
                && let Some(e) = ex.get(&ty)
            {
                return Some(match e {
                    ExplicitEffect::Set(m) => OwnEffect::Set(m.clone()),
                    ExplicitEffect::Clear => OwnEffect::Clear,
                });
            }
        } else if let Some(d) = dot {
            let bm = src.block_modifiers.modifiers_of(d);
            if let Some(m) = bm.get(&ty)
                && Schema::modifier_spec(ty).context.matches(path)
            {
                return Some(OwnEffect::Set(m.clone()));
            }
        }
        if let Some(d) = dot
            && let Some(Some(sid)) = src.node_styles.get(&d)
            && let Some(st) = src.styles.get(sid)
        {
            let mut best: Option<(Dot, Modifier)> = None;
            for m in st.modifiers.iter() {
                if m.as_type() == ty
                    && Schema::modifier_spec(ty).context.matches(path)
                    && let Some(rep) = st.modifiers.tags_for(m).copied().max()
                    && best.as_ref().is_none_or(|(c, _)| rep > *c)
                {
                    best = Some((rep, m.clone()));
                }
            }
            if let Some((_, m)) = best {
                return Some(OwnEffect::Set(m));
            }
        }
        let node = dot
            .and_then(|d| src.node_attrs.get(&d).cloned())
            .unwrap_or_else(|| nt.into_node());
        for m in node.implicit_modifiers() {
            if m.as_type() == ty {
                return Some(OwnEffect::Set(m.clone()));
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
                for i in (0..block_path.len()).rev() {
                    let (atype, adot) = block_path[i];
                    if adot.is_none() {
                        continue;
                    }
                    let apath: Vec<NodeType> = block_path[0..=i].iter().map(|(t, _)| *t).collect();
                    if spec.inheritable {
                        if let Some(OwnEffect::Set(m)) =
                            ref_own_ty(adot, atype, &apath, false, ty, src)
                        {
                            return Some(m);
                        }
                    } else if Schema::modifier_spec(ty).target.matches(&apath) {
                        return match ref_own_ty(adot, atype, &apath, false, ty, src) {
                            Some(OwnEffect::Set(m)) => Some(m),
                            _ => None,
                        };
                    }
                }
                None
            };
            let val = if spec.inheritable {
                match (
                    is_target_self,
                    ref_own_ty(self_dot, self_type, &self_path, is_leaf, ty, src),
                ) {
                    (true, Some(OwnEffect::Set(m))) => Some(m),
                    (true, Some(OwnEffect::Clear)) => None,
                    _ => inh(ty),
                }
            } else if is_target_self {
                match ref_own_ty(self_dot, self_type, &self_path, is_leaf, ty, src) {
                    Some(OwnEffect::Set(m)) => Some(m),
                    _ => None,
                }
            } else {
                inh(ty)
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
            leaf_style_mod in proptest::option::of(arb_mod()),
            leaf_span in proptest::option::of((arb_mod(), any::<bool>())),
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
            let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
            let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
            if let Some(m) = leaf_style_mod {
                node_styles.insert(leaf, Some("s".to_string()));
                styles.insert("s".to_string(), style_with(&[(Dot::new(8, 0), m)]));
            }
            let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
            if let Some((m, clear)) = leaf_span {
                let mut e = BTreeMap::new();
                e.insert(m.as_type(), if clear { ExplicitEffect::Clear } else { ExplicitEffect::Set(m) });
                spans.insert(leaf, e);
            }
            let node_attrs: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
            let src = EffectiveSources { block_modifiers: &bm, explicit_spans: &spans, node_styles: &node_styles, styles: &styles, node_attrs: &node_attrs };
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
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
    fn resolve_effective_folds_leaf_local_style() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(leaf, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(8, 0), Modifier::Bold)]),
        );
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Paragraph, Some(para)),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert_eq!(eff.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn resolve_effective_inherits_block_style_to_leaf() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let leaf = Dot::new(1, 2);
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(para, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(8, 0), Modifier::FontSize { value: 1600 })]),
        );
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
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
    fn resolve_effective_non_inheritable_alignment_reaches_text() {
        let root = Dot::new(1, 0);
        let para = Dot::new(1, 1);
        let leaf = Dot::new(1, 2);
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Paragraph, Some(para)),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert_eq!(
            eff.get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: crate::Alignment::Center
            })
        );
    }

    #[test]
    fn resolve_effective_non_inheritable_scoped_stops_at_nearest_target() {
        let root = Dot::new(1, 0);
        let table = Dot::new(1, 1);
        let row = Dot::new(1, 2);
        let cell = Dot::new(1, 3);
        let para = Dot::new(1, 4);
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Table, Some(table)),
            (NodeType::TableRow, Some(row)),
            (NodeType::TableCell, Some(cell)),
            (NodeType::Paragraph, Some(para)),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert!(
            !eff.contains_key(&ModifierType::Alignment),
            "scoped: stops at nearest Paragraph target"
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
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
    fn resolve_effective_non_inheritable_skips_none_target_ancestor() {
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
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
            Some(&Modifier::Alignment {
                value: crate::Alignment::Right
            }),
            "non-inheritable walk skips the None Paragraph target and reaches Table"
        );
    }

    #[test]
    fn resolve_effective_leaf_span_clear_is_barrier() {
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
        let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        let mut e = BTreeMap::new();
        e.insert(ModifierType::FontSize, ExplicitEffect::Clear);
        spans.insert(leaf, e);
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let bp = [
            (NodeType::Root, Some(root)),
            (NodeType::Paragraph, Some(para)),
        ];
        let eff = resolve_effective(&bp, Some(leaf), NodeType::Text, true, &src);
        assert!(
            !eff.contains_key(&ModifierType::FontSize),
            "leaf Clear blocks inheritance"
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
        let tree = normalize(project_blocks(&els).unwrap());
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
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
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
    fn leaf_inherits_ancestor_implicit_text_color() {
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
        let tree = normalize(project_blocks(&els).unwrap());
        let bm = ModifierAttrLog::new();
        let spans = HashMap::new();
        let node_styles = imbl::HashMap::new();
        let styles = imbl::HashMap::new();
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
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let be = derive_block_effective(&tree, &src);
        assert_eq!(
            be.get(&para).and_then(|m| m.get(&ModifierType::TextColor)),
            Some(&Modifier::TextColor {
                value: "bright".to_string()
            }),
            "Paragraph block_effective inherits the Blockquote's implicit TextColor"
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
            })
        );
    }

    #[test]
    fn own_modifiers_tags_source_and_tombstones_clear() {
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
        let tree = normalize(project_blocks(&els).unwrap());

        let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        spans.insert(
            a,
            BTreeMap::from([(ModifierType::Bold, ExplicitEffect::Set(Modifier::Bold))]),
        );
        spans.insert(
            b,
            BTreeMap::from([(ModifierType::Bold, ExplicitEffect::Clear)]),
        );
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(b, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(9, 0), Modifier::Bold)]),
        );
        let bm = ModifierAttrLog::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };

        let own = derive_own_modifiers(&tree, &src);
        assert_eq!(
            own.get(&a).and_then(|m| m.get(&ModifierType::Bold)),
            Some(&OwnModifier {
                value: Modifier::Bold,
                from_style: false
            })
        );
        assert!(
            own.get(&b)
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
    }

    #[test]
    fn own_modifiers_explicit_beats_same_type_style() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (a, SeqItem::Char('a')),
        ];
        let log = oplog_of(&elems);
        let (els, _r) = checkout_with_resolver(&log);
        let tree = normalize(project_blocks(&els).unwrap());

        let mut spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        spans.insert(
            a,
            BTreeMap::from([(
                ModifierType::FontSize,
                ExplicitEffect::Set(Modifier::FontSize { value: 1600 }),
            )]),
        );
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(a, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(9, 0), Modifier::FontSize { value: 1200 })]),
        );
        let bm = ModifierAttrLog::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };

        let own = derive_own_modifiers(&tree, &src);
        assert_eq!(
            own.get(&a).and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&OwnModifier {
                value: Modifier::FontSize { value: 1600 },
                from_style: false
            }),
            "explicit Set wins over a same-type style; style's 1200 loses and from_style is false"
        );
    }

    #[test]
    fn own_modifiers_style_only_is_from_style() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (a, SeqItem::Char('a')),
        ];
        let log = oplog_of(&elems);
        let (els, _r) = checkout_with_resolver(&log);
        let tree = normalize(project_blocks(&els).unwrap());
        let spans: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
        let mut node_styles: imbl::HashMap<Dot, Option<String>> = imbl::HashMap::new();
        node_styles.insert(a, Some("s".to_string()));
        let mut styles: imbl::HashMap<String, StyleEntry> = imbl::HashMap::new();
        styles.insert(
            "s".to_string(),
            style_with(&[(Dot::new(9, 0), Modifier::Bold)]),
        );
        let bm = ModifierAttrLog::new();
        let node_attrs = imbl::HashMap::new();
        let src = EffectiveSources {
            block_modifiers: &bm,
            explicit_spans: &spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let own = derive_own_modifiers(&tree, &src);
        assert_eq!(
            own.get(&a).and_then(|m| m.get(&ModifierType::Bold)),
            Some(&OwnModifier {
                value: Modifier::Bold,
                from_style: true
            })
        );
    }

    #[test]
    fn style_same_type_conflict_is_order_independent() {
        let a = Dot::new(1, 0);
        let b = Dot::new(2, 0);
        let forward = style_with(&[
            (a, Modifier::FontSize { value: 1600 }),
            (b, Modifier::FontSize { value: 1200 }),
        ]);
        let reverse = style_with(&[
            (b, Modifier::FontSize { value: 1200 }),
            (a, Modifier::FontSize { value: 1600 }),
        ]);
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        assert_eq!(
            style_modifiers(&forward, &path),
            style_modifiers(&reverse, &path)
        );
        assert_eq!(
            style_modifiers(&forward, &path).get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 }),
            "highest Dot b=(2,0) wins regardless of insertion order"
        );
    }
}

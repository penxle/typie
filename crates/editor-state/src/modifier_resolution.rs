use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, EffectiveSources, Expand, ExplicitEffect, LeafView, Modifier,
    ModifierAttrLog, ModifierType, Node, NodeType, NodeView, ProjectedDoc, Schema,
    resolve_effective,
};

use crate::Position;
use crate::affinity::Affinity;
use crate::pending_modifier::PendingModifier;
use crate::projected_state::ProjectedState;
use crate::state::State;

pub(crate) struct CaretCtx<'a> {
    pub(crate) view: &'a DocView<'a>,
    pub(crate) doc: &'a ProjectedDoc,
    pub(crate) block_modifiers: &'a ModifierAttrLog,
}

fn is_inline(ty: ModifierType) -> bool {
    Schema::modifier_spec(ty)
        .target
        .rightmost_node_types()
        .contains(&NodeType::Text)
}

fn parents_path(host: &NodeView) -> Vec<(NodeType, Option<Dot>)> {
    let mut v: Vec<(NodeType, Option<Dot>)> = host
        .ancestors()
        .skip(1)
        .map(|n| (n.node_type(), n.dot()))
        .collect();
    v.reverse();
    v
}

fn inherited_over(
    ancestors: &[(NodeType, Option<Dot>)],
    ctx: &CaretCtx,
) -> BTreeMap<ModifierType, Modifier> {
    let empty: HashMap<Dot, BTreeMap<ModifierType, ExplicitEffect>> = HashMap::new();
    let src = EffectiveSources {
        block_modifiers: ctx.block_modifiers,
        explicit_spans: &empty,
        node_styles: &ctx.doc.node_styles,
        styles: &ctx.doc.styles,
        node_attrs: &ctx.doc.node_attrs,
    };
    resolve_effective(ancestors, None, NodeType::Text, true, &src)
}

fn apply_pending(out: &mut BTreeMap<ModifierType, Modifier>, pending: &[PendingModifier]) {
    for pm in pending {
        match pm {
            PendingModifier::Set { modifier } => {
                out.insert(modifier.as_type(), modifier.clone());
            }
            PendingModifier::Unset { ty } => {
                out.remove(ty);
            }
        }
    }
}

/// Effective inline modifiers a caret at `pos` would carry (no pending overrides).
pub fn resolve_effective_modifiers_at(state: &State, pos: &Position) -> Vec<Modifier> {
    caret_modifiers(&state.projected, pos, &[])
        .into_values()
        .collect()
}

pub(crate) fn caret_modifiers(
    state: &ProjectedState,
    pos: &Position,
    pending: &[PendingModifier],
) -> BTreeMap<ModifierType, Modifier> {
    let view = state.view();
    let ctx = CaretCtx {
        view: &view,
        doc: state.projected(),
        block_modifiers: state.block_modifiers(),
    };
    resolve_caret_modifiers(pos, &ctx, pending)
}

fn self_path(host: &NodeView) -> Vec<(NodeType, Option<Dot>)> {
    let mut v: Vec<(NodeType, Option<Dot>)> =
        host.ancestors().map(|n| (n.node_type(), n.dot())).collect();
    v.reverse();
    v
}

fn char_leaf<'a>(c: &ChildView<'a>) -> Option<LeafView<'a>> {
    match c {
        ChildView::Leaf(l) if l.as_char().is_some() => Some(l.clone()),
        _ => None,
    }
}

pub(crate) fn resolve_caret_modifiers(
    pos: &Position,
    ctx: &CaretCtx,
    pending: &[PendingModifier],
) -> BTreeMap<ModifierType, Modifier> {
    let Some(host) = ctx.view.node(pos.node) else {
        return BTreeMap::new();
    };
    let children: Vec<ChildView> = host.children().collect();
    let left = pos
        .offset
        .checked_sub(1)
        .and_then(|i| children.get(i))
        .and_then(char_leaf);
    let right = children.get(pos.offset).and_then(char_leaf);

    let mut out = if left.is_some() || right.is_some() {
        non_empty(&host, pos, left, right, ctx)
    } else {
        empty_or_structural(&host, &children, ctx)
    };
    apply_pending(&mut out, pending);
    out
}

fn empty_or_structural(
    host: &NodeView,
    children: &[ChildView],
    ctx: &CaretCtx,
) -> BTreeMap<ModifierType, Modifier> {
    let inherited = inherited_over(&parents_path(host), ctx);
    if !Schema::node_spec(host.node_type()).is_textblock() {
        return inherited;
    }
    let mut out: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
    if let Some(d) = host.dot()
        && let Some(bm) = ctx.doc.block_modifiers.get(&d)
    {
        for (ty, m) in bm {
            out.entry(*ty).or_insert_with(|| m.clone());
        }
    }
    let node: Node = host.node();
    for m in node.implicit_modifiers() {
        out.entry(m.as_type()).or_insert_with(|| m.clone());
    }
    if children.is_empty()
        && let Some(d) = host.dot()
    {
        if let Some(Some(sid)) = ctx.doc.node_styles.get(&d)
            && let Some(style) = ctx.doc.styles.get(sid)
        {
            for m in style.modifiers.iter() {
                let ty = m.as_type();
                if is_inline(ty) {
                    out.entry(ty).or_insert_with(|| m.clone());
                }
            }
        }
        if let Some(Some(marker)) = ctx.doc.node_markers.get(&d) {
            for m in &marker.modifiers {
                let ty = m.as_type();
                if is_inline(ty) {
                    out.entry(ty).or_insert_with(|| m.clone());
                }
            }
            if let Some(sid) = &marker.style
                && let Some(style) = ctx.doc.styles.get(sid)
            {
                for m in style.modifiers.iter() {
                    let ty = m.as_type();
                    if is_inline(ty) {
                        out.entry(ty).or_insert_with(|| m.clone());
                    }
                }
            }
        }
    }
    for (ty, m) in &inherited {
        out.entry(*ty).or_insert_with(|| m.clone());
    }
    out
}

fn non_empty(
    host: &NodeView,
    pos: &Position,
    left: Option<LeafView>,
    right: Option<LeafView>,
    ctx: &CaretCtx,
) -> BTreeMap<ModifierType, Modifier> {
    if let (Some(l), Some(r)) = (&left, &right)
        && l.effective() == r.effective()
    {
        return r.effective().clone();
    }

    let inherited = inherited_over(&self_path(host), ctx);
    let downstream = pos.affinity == Affinity::Downstream;
    let (refleaf, start, carry): (LeafView, bool, Option<LeafView>) = match (right, left) {
        (Some(r), Some(l)) => {
            if downstream {
                (r, true, Some(l))
            } else {
                (l, false, None)
            }
        }
        (Some(r), None) => (r, true, None),
        (None, Some(l)) => (l, false, None),
        (None, None) => return inherited,
    };

    let mut out: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
    for (ty, om) in refleaf.own_modifiers() {
        let keep = match &Schema::modifier_spec(*ty).expand {
            Expand::Before => start,
            Expand::After => !start,
            Expand::Both => true,
            Expand::None => false,
        };
        if keep {
            out.entry(*ty).or_insert_with(|| om.value.clone());
        }
    }
    if let Some(c) = &carry {
        for (ty, om) in c.own_modifiers() {
            if matches!(
                &Schema::modifier_spec(*ty).expand,
                Expand::After | Expand::Both
            ) {
                out.entry(*ty).or_insert_with(|| om.value.clone());
            }
        }
    }
    let ref_eff = refleaf.effective();
    for (ty, m) in &inherited {
        if ref_eff.contains_key(ty) {
            out.entry(*ty).or_insert_with(|| m.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{InputEvent, ListOp, LwwRegOp, OrMapOp, OrSetOp, build_oplog};
    use editor_model::{
        Alignment, Anchor, AtomLeaf, Bias, DocLogs, Marker, ModifierAttrLog, ModifierAttrOp,
        NodeAttrLog, NodeLwwOp, NodeMarkerLog, NodeStyleLog, SeqItem, SpanLog, SpanOp, StyleLog,
        StyleOp, StyleRegOp, project_document,
    };

    fn block(node_type: NodeType, parents: Vec<Dot>) -> SeqItem {
        SeqItem::Block { node_type, parents }
    }

    fn ins_only(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
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
        ev
    }

    #[derive(Default)]
    struct Overlays {
        spans: SpanLog,
        block_modifiers: ModifierAttrLog,
        node_styles: NodeStyleLog,
        node_markers: NodeMarkerLog,
        styles: StyleLog,
    }

    fn doclogs(items: &[(Dot, SeqItem)], o: Overlays) -> DocLogs {
        DocLogs {
            seq: build_oplog(&ins_only(items)),
            spans: o.spans,
            block_modifiers: o.block_modifiers,
            node_attrs: NodeAttrLog::new(),
            node_styles: o.node_styles,
            node_markers: o.node_markers,
            styles: o.styles,
        }
    }

    fn style_log(id: &str, m: Modifier) -> StyleLog {
        StyleLog::new()
            .apply(
                Dot::new(5, 0),
                StyleRegOp {
                    style_id: id.to_string(),
                    op: StyleOp::Presence(OrMapOp::Set {
                        key: id.to_string(),
                        value: (),
                    }),
                },
            )
            .unwrap()
            .apply(
                Dot::new(5, 1),
                StyleRegOp {
                    style_id: id.to_string(),
                    op: StyleOp::Modifiers(OrSetOp::Add { elem: m }),
                },
            )
            .unwrap()
    }

    fn node_style(target: Dot, id: &str) -> NodeStyleLog {
        NodeStyleLog::new()
            .apply(
                Dot::new(6, 0),
                NodeLwwOp {
                    target,
                    op: LwwRegOp::Set {
                        value: Some(id.to_string()),
                    },
                },
            )
            .unwrap()
    }

    fn node_marker(target: Dot, marker: Marker) -> NodeMarkerLog {
        NodeMarkerLog::new()
            .apply(
                Dot::new(7, 0),
                NodeLwwOp {
                    target,
                    op: LwwRegOp::Set {
                        value: Some(marker),
                    },
                },
            )
            .unwrap()
    }

    fn block_mod(target: Dot, m: Modifier) -> ModifierAttrLog {
        ModifierAttrLog::new()
            .apply(
                Dot::new(8, 0),
                ModifierAttrOp::SetModifier {
                    target,
                    modifier: m,
                },
            )
            .unwrap()
    }

    fn span_set(leaf: Dot, m: Modifier) -> SpanLog {
        SpanLog::new()
            .apply(
                Dot::new(9, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: leaf,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: leaf,
                        bias: Bias::After,
                    },
                    modifier: m,
                },
            )
            .unwrap()
    }

    fn span_clear(leaf: Dot, ty: ModifierType) -> SpanLog {
        SpanLog::new()
            .apply(
                Dot::new(9, 0),
                SpanOp::RemoveSpan {
                    start: Anchor {
                        id: leaf,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: leaf,
                        bias: Bias::After,
                    },
                    modifier_type: ty,
                },
            )
            .unwrap()
    }

    fn project(logs: &DocLogs) -> ProjectedDoc {
        project_document(logs).unwrap()
    }

    fn para(leaves: &[SeqItem], o: Overlays) -> (DocLogs, Dot, Dot) {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let mut items = vec![(p, block(NodeType::Paragraph, vec![root]))];
        for (i, l) in leaves.iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), l.clone()));
        }
        (doclogs(&items, o), root, p)
    }

    fn ctx<'a>(pd: &'a ProjectedDoc, view: &'a DocView<'a>, logs: &'a DocLogs) -> CaretCtx<'a> {
        CaretCtx {
            view,
            doc: pd,
            block_modifiers: &logs.block_modifiers,
        }
    }

    #[test]
    fn is_inline_classifies_bold_vs_alignment() {
        assert!(is_inline(ModifierType::Bold));
        assert!(!is_inline(ModifierType::Alignment));
    }

    #[test]
    fn apply_pending_set_and_unset() {
        let mut m = BTreeMap::new();
        m.insert(ModifierType::Bold, Modifier::Bold);
        apply_pending(
            &mut m,
            &[
                PendingModifier::Set {
                    modifier: Modifier::Italic,
                },
                PendingModifier::Unset {
                    ty: ModifierType::Bold,
                },
            ],
        );
        assert!(m.contains_key(&ModifierType::Italic));
        assert!(!m.contains_key(&ModifierType::Bold));
    }

    #[test]
    fn inherited_over_inheritable_from_root() {
        let (logs, root, p) = para(
            &[],
            Overlays {
                block_modifiers: block_mod(Dot::ROOT, Modifier::FontSize { value: 1600 }),
                ..Default::default()
            },
        );
        let _ = root;
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let host = view.node(p).unwrap();
        let inh = inherited_over(&parents_path(&host), &c);
        assert_eq!(
            inh.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn inherited_over_excludes_own_block_alignment() {
        let (logs, _root, p) = para(
            &[],
            Overlays {
                block_modifiers: block_mod(
                    Dot::new(1, 1),
                    Modifier::Alignment {
                        value: Alignment::Center,
                    },
                ),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let host = view.node(p).unwrap();
        let inh = inherited_over(&parents_path(&host), &c);
        assert!(!inh.contains_key(&ModifierType::Alignment));
    }

    fn caret(pos: &Position, c: &CaretCtx) -> BTreeMap<ModifierType, Modifier> {
        resolve_caret_modifiers(pos, c, &[])
    }

    #[test]
    fn empty_paragraph_marker_surfaces() {
        let (logs, _root, p) = para(
            &[],
            Overlays {
                node_markers: node_marker(
                    Dot::new(1, 1),
                    Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    },
                ),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 0), &c);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn interior_run_has_all_modifiers() {
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let spans = SpanLog::new()
            .apply(
                Dot::new(9, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: a,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: b,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Char('b')],
            Overlays {
                spans,
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 1), &c);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn structural_container_caret_inherited_only() {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let pp = Dot::new(1, 2);
        let items = vec![
            (bq, block(NodeType::Blockquote, vec![root])),
            (pp, block(NodeType::Paragraph, vec![root, bq])),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let logs = doclogs(
            &items,
            Overlays {
                block_modifiers: block_mod(root, Modifier::FontSize { value: 1600 }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(bq, 0), &c);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn pending_overlay_applies_last() {
        let (logs, _root, p) = para(&[SeqItem::Char('a')], Overlays::default());
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position::new(p, 1),
            &c,
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        );
        assert_eq!(out.get(&ModifierType::Italic), Some(&Modifier::Italic));
    }

    #[test]
    fn empty_paragraph_pending_overlay_applies() {
        let (logs, _root, p) = para(&[], Overlays::default());
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position::new(p, 0),
            &c,
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        );
        assert_eq!(out.get(&ModifierType::Italic), Some(&Modifier::Italic));
    }

    #[test]
    fn boundary_expand_none_link_excluded_at_edge() {
        let a = Dot::new(1, 2);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Char('b')],
            Overlays {
                spans: span_set(a, Modifier::Link { href: "x".into() }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &c,
            &[],
        );
        assert!(!out.contains_key(&ModifierType::Link));
    }

    #[test]
    fn boundary_link_present_inside_run() {
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let spans = SpanLog::new()
            .apply(
                Dot::new(9, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: a,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: b,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Link { href: "x".into() },
                },
            )
            .unwrap();
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Char('b')],
            Overlays {
                spans,
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &c,
            &[],
        );
        assert!(out.contains_key(&ModifierType::Link));
    }

    #[test]
    fn clear_blocks_reinheritance_at_boundary() {
        let a = Dot::new(1, 2);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Char('b')],
            Overlays {
                spans: span_clear(a, ModifierType::FontSize),
                block_modifiers: block_mod(Dot::ROOT, Modifier::FontSize { value: 1600 }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            &c,
            &[],
        );
        assert!(!out.contains_key(&ModifierType::FontSize));
    }

    #[test]
    fn boundary_expand_excluded_own_falls_back_to_inherited() {
        let a = Dot::new(1, 2);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a')],
            Overlays {
                spans: span_set(a, Modifier::FontSize { value: 1200 }),
                block_modifiers: block_mod(Dot::ROOT, Modifier::FontSize { value: 1600 }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            &c,
            &[],
        );
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn carryover_char_only_not_atom() {
        let a = Dot::new(1, 2);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Atom(AtomLeaf::HardBreak)],
            Overlays {
                spans: span_set(a, Modifier::Bold),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 2,
                affinity: Affinity::Downstream,
            },
            &c,
            &[],
        );
        assert!(!out.contains_key(&ModifierType::Bold));
    }

    #[test]
    fn atom_only_textblock_no_marker() {
        let (logs, _root, p) = para(
            &[SeqItem::Atom(AtomLeaf::HardBreak)],
            Overlays {
                node_markers: node_marker(
                    Dot::new(1, 1),
                    Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    },
                ),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = resolve_caret_modifiers(&Position::new(p, 0), &c, &[]);
        assert!(!out.contains_key(&ModifierType::Bold));
    }

    #[test]
    fn empty_marker_style_inline_only() {
        let mut styles = style_log("ms", Modifier::Bold);
        styles = styles
            .apply(
                Dot::new(5, 2),
                StyleRegOp {
                    style_id: "ms".to_string(),
                    op: StyleOp::Modifiers(OrSetOp::Add {
                        elem: Modifier::Alignment {
                            value: Alignment::Center,
                        },
                    }),
                },
            )
            .unwrap();
        let (logs, _root, p) = para(
            &[],
            Overlays {
                node_markers: node_marker(
                    Dot::new(1, 1),
                    Marker {
                        modifiers: vec![],
                        style: Some("ms".to_string()),
                    },
                ),
                styles,
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 0), &c);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
        assert!(!out.contains_key(&ModifierType::Alignment));
    }

    #[test]
    fn empty_marker_beats_inherited() {
        let (logs, _root, p) = para(
            &[],
            Overlays {
                block_modifiers: block_mod(Dot::ROOT, Modifier::FontSize { value: 1600 }),
                node_markers: node_marker(
                    Dot::new(1, 1),
                    Marker {
                        modifiers: vec![Modifier::FontSize { value: 1200 }],
                        style: None,
                    },
                ),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 0), &c);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 })
        );
    }

    #[test]
    fn offset_extremes_resolve_to_edges() {
        let a = Dot::new(1, 2);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a')],
            Overlays {
                spans: span_set(a, Modifier::Bold),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let start = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 0,
                affinity: Affinity::Upstream,
            },
            &c,
            &[],
        );
        let end = resolve_caret_modifiers(
            &Position {
                node: p,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            &c,
            &[],
        );
        assert!(
            !start.contains_key(&ModifierType::Bold),
            "Bold (After) excluded at start"
        );
        assert!(
            end.contains_key(&ModifierType::Bold),
            "Bold (After) included at end"
        );
    }

    #[test]
    fn derived_empty_paragraph_inherited_only() {
        let root = Dot::ROOT;
        let bq = Dot::new(1, 1);
        let pp = Dot::new(1, 2);
        let items = vec![
            (bq, block(NodeType::Blockquote, vec![root])),
            (pp, block(NodeType::Paragraph, vec![root, bq])),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let logs = doclogs(
            &items,
            Overlays {
                block_modifiers: block_mod(root, Modifier::FontSize { value: 1600 }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let derived = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|b| b.id().is_synthetic())
            .map(|b| b.id())
            .expect("derived trailing paragraph");
        let out = caret(&Position::new(derived, 0), &c);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert!(!out.contains_key(&ModifierType::Bold));
    }

    #[test]
    fn interior_same_effective_different_source() {
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let (logs, _root, p) = para(
            &[SeqItem::Char('a'), SeqItem::Char('b')],
            Overlays {
                spans: span_set(a, Modifier::Bold),
                node_styles: node_style(b, "s"),
                styles: style_log("s", Modifier::Bold),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 1), &c);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn empty_block_own_beats_style() {
        let (logs, _root, p) = para(
            &[],
            Overlays {
                block_modifiers: block_mod(Dot::new(1, 1), Modifier::FontSize { value: 1600 }),
                node_styles: node_style(Dot::new(1, 1), "s"),
                styles: style_log("s", Modifier::FontSize { value: 1200 }),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let out = caret(&Position::new(p, 0), &c);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn empty_foldtitle_marker_surfaces() {
        let root = Dot::ROOT;
        let fold = Dot::new(1, 1);
        let ftitle = Dot::new(1, 2);
        let fcontent = Dot::new(1, 3);
        let fc_para = Dot::new(1, 4);
        let items = vec![
            (fold, block(NodeType::Fold, vec![root])),
            (ftitle, block(NodeType::FoldTitle, vec![root, fold])),
            (fcontent, block(NodeType::FoldContent, vec![root, fold])),
            (
                fc_para,
                block(NodeType::Paragraph, vec![root, fold, fcontent]),
            ),
        ];
        let logs = doclogs(
            &items,
            Overlays {
                node_markers: node_marker(
                    ftitle,
                    Marker {
                        modifiers: vec![Modifier::Bold],
                        style: None,
                    },
                ),
                ..Default::default()
            },
        );
        let pd = project(&logs);
        let view = DocView::new(&pd);
        let c = ctx(&pd, &view, &logs);
        let title = view.node(ftitle).expect("FoldTitle survives normalize");
        assert!(title.children().count() == 0, "FoldTitle is empty");
        let out = caret(&Position::new(ftitle, 0), &c);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    fn arb_para_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c']), 0..6)
    }

    proptest::proptest! {
        #[test]
        fn resolve_never_panics_and_interior_equals_run(chars in arb_para_chars()) {
            let mut o = Overlays::default();
            if !chars.is_empty() {
                o.spans = span_set(Dot::new(1, 2), Modifier::Bold);
            }
            let leaves: Vec<SeqItem> = chars.iter().map(|c| SeqItem::Char(*c)).collect();
            let (logs, _root, p) = para(&leaves, o);
            let pd = project(&logs);
            let view = DocView::new(&pd);
            let c = ctx(&pd, &view, &logs);
            for offset in 0..=chars.len() {
                for affinity in [Affinity::Downstream, Affinity::Upstream] {
                    let pos = Position { node: p, offset, affinity };
                    let _ = resolve_caret_modifiers(&pos, &c, &[]);
                }
            }
            let host = view.node(p).unwrap();
            let kids: Vec<ChildView> = host.children().collect();
            for i in 1..chars.len() {
                let l = char_leaf(&kids[i - 1]);
                let r = char_leaf(&kids[i]);
                if let (Some(l), Some(r)) = (l, r)
                    && l.effective() == r.effective()
                {
                    let out = resolve_caret_modifiers(
                        &Position { node: p, offset: i, affinity: Affinity::Downstream },
                        &c,
                        &[],
                    );
                    proptest::prop_assert_eq!(&out, r.effective());
                }
            }
        }
    }
}

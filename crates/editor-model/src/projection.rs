use std::collections::{BTreeMap, HashMap, HashSet};

use editor_crdt::Dot;
use editor_crdt::OpLog;
use editor_crdt::sequence::checkout_with_resolver;

use crate::{
    BlockNode, BlockTree, Child, Modifier, ModifierAttrLog, ModifierType, NodeType, OwnModifier,
    ProjectError, SchemaError, anchor_dot,
};
use crate::{
    Marker, Node, NodeAttrLog, NodeMarkerLog, NodeStyleLog, Run, SeqItem, SpanLog, StyleEntry,
    StyleLog, derive_full_effective, derive_runs, normalize, project_blocks, validate_block_tree,
};

#[derive(Debug)]
pub enum ProjectionError {
    Project(ProjectError),
    LeafTypedBlock { dot: Dot, node_type: NodeType },
    SchemaInvalid(SchemaError),
}

#[derive(Clone, Debug)]
pub struct DocLogs {
    pub seq: OpLog<SeqItem>,
    pub spans: SpanLog,
    pub block_modifiers: ModifierAttrLog,
    pub node_attrs: NodeAttrLog,
    pub node_styles: NodeStyleLog,
    pub node_markers: NodeMarkerLog,
    pub styles: StyleLog,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedDoc {
    pub tree: BlockTree,
    pub effective: HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub block_effective: HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub own_modifiers: HashMap<Dot, BTreeMap<ModifierType, OwnModifier>>,
    pub runs: Vec<Run>,
    pub block_modifiers: HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub node_attrs: imbl::HashMap<Dot, Node>,
    pub node_styles: imbl::HashMap<Dot, Option<String>>,
    pub node_markers: imbl::HashMap<Dot, Option<Marker>>,
    pub styles: imbl::HashMap<String, StyleEntry>,
}

fn collect_real_ids(tree: &BlockTree) -> HashMap<Dot, NodeType> {
    fn walk(node: &BlockNode, out: &mut HashMap<Dot, NodeType>) {
        if let Some(d) = anchor_dot(node.id) {
            out.insert(d, node.node_type);
        }
        for c in &node.children {
            match c {
                Child::Leaf { id, item } => {
                    out.insert(*id, item.as_child_type());
                }
                Child::Block(b) => walk(b, out),
            }
        }
    }
    let mut out = HashMap::new();
    for r in &tree.roots {
        walk(r, &mut out);
    }
    out
}

fn filter_live<T: Clone>(map: imbl::HashMap<Dot, T>, live: &HashSet<Dot>) -> imbl::HashMap<Dot, T> {
    map.into_iter().filter(|(d, _)| live.contains(d)).collect()
}

fn collect_block_modifiers(
    tree: &BlockTree,
    log: &ModifierAttrLog,
) -> HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
    fn walk(
        node: &BlockNode,
        log: &ModifierAttrLog,
        out: &mut HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    ) {
        if let Some(d) = anchor_dot(node.id) {
            let m = log.modifiers_of(d);
            if !m.is_empty() {
                out.insert(d, m);
            }
        }
        for c in &node.children {
            if let Child::Block(b) = c {
                walk(b, log, out);
            }
        }
    }
    let mut out = HashMap::new();
    for r in &tree.roots {
        walk(r, log, &mut out);
    }
    out
}

pub fn project_document(logs: &DocLogs) -> Result<ProjectedDoc, ProjectionError> {
    let (elements, resolver) = checkout_with_resolver(&logs.seq);

    for (d, item) in &elements {
        if let SeqItem::Block { node_type, .. } = item
            && node_type.spec().is_leaf()
        {
            return Err(ProjectionError::LeafTypedBlock {
                dot: *d,
                node_type: *node_type,
            });
        }
    }

    let raw_tree = project_blocks(&elements).map_err(ProjectionError::Project)?;
    let tree = normalize(raw_tree);
    validate_block_tree(&tree).map_err(ProjectionError::SchemaInvalid)?;

    let node_type_of = collect_real_ids(&tree);
    let live: HashSet<Dot> = node_type_of.keys().copied().collect();

    let node_attrs = logs.node_attrs.project(|d| node_type_of.get(&d).copied());
    let node_styles = filter_live(logs.node_styles.project(), &live);
    let node_markers = filter_live(logs.node_markers.project(), &live);
    let styles = logs.styles.registered_entries();
    let block_modifiers = collect_block_modifiers(&tree, &logs.block_modifiers);

    let explicit_spans: HashMap<Dot, BTreeMap<ModifierType, crate::span::ExplicitEffect>> =
        crate::span::derive_explicit_effect(&elements, &tree, &resolver, &logs.spans)
            .into_iter()
            .collect();
    let (effective, block_effective, own_modifiers) = {
        let src = crate::span::EffectiveSources {
            block_modifiers: &logs.block_modifiers,
            explicit_spans: &explicit_spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let effective: HashMap<Dot, BTreeMap<ModifierType, Modifier>> =
            derive_full_effective(&tree, &src).into_iter().collect();
        let block_effective = crate::span::derive_block_effective(&tree, &src);
        let own_modifiers = crate::span::derive_own_modifiers(&tree, &src);
        (effective, block_effective, own_modifiers)
    };
    let runs = derive_runs(&tree, &effective);

    Ok(ProjectedDoc {
        tree,
        effective,
        block_effective,
        own_modifiers,
        runs,
        block_modifiers,
        node_attrs,
        node_styles,
        node_markers,
        styles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomLeaf, SeqItem, project_blocks};

    fn elems_nested() -> Vec<(Dot, SeqItem)> {
        let bq = Dot::new(1, 5);
        vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (Dot::new(1, 4), SeqItem::Atom(AtomLeaf::HardBreak)),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 6),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 7), SeqItem::Char('y')),
            (Dot::new(1, 8), SeqItem::Char('o')),
        ]
    }

    #[test]
    fn collect_real_ids_covers_blocks_chars_atoms() {
        let tree = project_blocks(&elems_nested()).unwrap();
        let ids = collect_real_ids(&tree);
        assert_eq!(ids.get(&Dot::ROOT), Some(&NodeType::Root));
        assert_eq!(ids.get(&Dot::new(1, 1)), Some(&NodeType::Paragraph));
        assert_eq!(ids.get(&Dot::new(1, 2)), Some(&NodeType::Text));
        assert_eq!(ids.get(&Dot::new(1, 4)), Some(&NodeType::HardBreak));
        assert_eq!(ids.get(&Dot::new(1, 5)), Some(&NodeType::Blockquote));
    }

    #[test]
    fn filter_live_keeps_only_live_keys() {
        let mut m: imbl::HashMap<Dot, u32> = imbl::HashMap::new();
        m.insert(Dot::new(1, 1), 10);
        m.insert(Dot::new(9, 9), 20);
        let live: HashSet<Dot> = [Dot::new(1, 1)].into_iter().collect();
        let out = filter_live(m, &live);
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(&Dot::new(1, 1)), Some(&10));
    }

    use crate::{
        Anchor, Bias, CalloutNodeAttr, CalloutVariant, ImageNodeAttr, Modifier, ModifierAttrOp,
        ModifierType, NodeAttr, NodeAttrOp, NodeLwwOp, SpanOp, StyleOp, StyleRegOp,
    };
    use editor_crdt::{InputEvent, ListOp, LwwRegOp, build_oplog};

    fn events(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
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

    fn logs_of(items: &[(Dot, SeqItem)]) -> DocLogs {
        DocLogs {
            seq: build_oplog(&events(items)),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn para_abc() -> (Vec<(Dot, SeqItem)>, Dot, Dot) {
        let para = Dot::new(1, 1);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        (elems, Dot::ROOT, para)
    }

    #[test]
    fn empty_document_projects_ok() {
        let pd = project_document(&logs_of(&[])).unwrap();
        assert_eq!(pd.tree.roots.len(), 1);
        assert_eq!(pd.tree.roots[0].node_type, NodeType::Root);
        assert!(pd.effective.is_empty());
        assert!(pd.runs.is_empty());
        assert!(pd.node_attrs.is_empty());
    }

    #[test]
    fn projects_nested_blocks() {
        let pd = project_document(&logs_of(&elems_nested())).unwrap();
        assert_eq!(pd.tree.roots.len(), 1);
        assert_eq!(pd.tree.roots[0].node_type, NodeType::Root);
        assert_eq!(pd.effective.len(), 5);
    }

    #[test]
    fn bold_span_splits_runs() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 4),
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
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.effective
                .get(&Dot::new(1, 4))
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&Modifier::Bold)
        );
        assert!(
            pd.effective
                .get(&Dot::new(1, 2))
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
        assert!(pd.runs.len() >= 2);
    }

    #[test]
    fn runs_split_under_style_enriched_effective() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        let styled = Dot::new(1, 2);
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: styled,
                    op: LwwRegOp::Set {
                        value: Some("s1".to_string()),
                    },
                },
            )
            .unwrap();
        l.styles = StyleLog::new()
            .apply(
                Dot::new(2, 1),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                        key: "s1".to_string(),
                        value: (),
                    }),
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 2),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Modifiers(editor_crdt::OrSetOp::Add {
                        elem: Modifier::Bold,
                    }),
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.effective
                .get(&styled)
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&Modifier::Bold)
        );
        assert_eq!(pd.runs.len(), 2);
        assert!(pd.runs.iter().any(|r| {
            r.leaves == vec![styled]
                && r.modifiers.get(&ModifierType::Bold) == Some(&Modifier::Bold)
        }));
        assert!(pd.runs.iter().any(|r| {
            r.leaves == vec![Dot::new(1, 3), Dot::new(1, 4)]
                && !r.modifiers.contains_key(&ModifierType::Bold)
        }));
    }

    #[test]
    fn overlays_attr_style_marker() {
        let callout = Dot::new(1, 1);
        let elems = vec![
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 2),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, callout],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let mut l = logs_of(&elems);
        l.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target: callout,
                    attr: NodeAttr::Callout {
                        attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                    },
                },
            )
            .unwrap();
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 1),
                NodeLwwOp {
                    target: callout,
                    op: LwwRegOp::Set {
                        value: Some("s1".to_string()),
                    },
                },
            )
            .unwrap();
        l.node_markers = NodeMarkerLog::new()
            .apply(
                Dot::new(2, 2),
                NodeLwwOp {
                    target: callout,
                    op: LwwRegOp::Set {
                        value: Some(Marker {
                            modifiers: vec![],
                            style: Some("s1".to_string()),
                        }),
                    },
                },
            )
            .unwrap();
        l.styles = StyleLog::new()
            .apply(
                Dot::new(2, 3),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                        key: "s1".to_string(),
                        value: (),
                    }),
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(pd.node_attrs.contains_key(&callout));
        assert_eq!(pd.node_styles.get(&callout), Some(&Some("s1".to_string())));
        assert!(pd.node_markers.get(&callout).is_some());
        assert!(pd.styles.contains_key("s1"));
    }

    #[test]
    fn font_size_inheritable_double_source() {
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.block_modifiers
                .get(&para)
                .and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert_eq!(
            pd.effective
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn alignment_block_resolves_onto_descendant_text() {
        use crate::Alignment;
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::Alignment {
                        value: Alignment::Center,
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(
            pd.block_modifiers
                .get(&para)
                .is_some_and(|m| m.contains_key(&ModifierType::Alignment))
        );
        assert_eq!(
            pd.effective
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::Alignment)),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            })
        );
    }

    #[test]
    fn image_atom_attr_projected() {
        let image = Dot::new(1, 1);
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let elems = vec![
            (
                image,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
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
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let mut l = logs_of(&elems);
        l.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target: image,
                    attr: NodeAttr::Image {
                        attr: ImageNodeAttr::Proportion(150),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(pd.node_attrs.contains_key(&image));
    }

    #[test]
    fn structural_malformation_errors() {
        let elems = vec![(
            Dot::new(1, 1),
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::new(9, 9)],
            },
        )];
        assert!(matches!(
            project_document(&logs_of(&elems)),
            Err(ProjectionError::Project(_))
        ));
    }

    #[test]
    fn leaf_typed_block_errors() {
        for leaf_ty in [NodeType::Text, NodeType::Image] {
            let elems = vec![(
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: leaf_ty,
                    parents: vec![Dot::ROOT],
                },
            )];
            assert!(
                matches!(
                    project_document(&logs_of(&elems)),
                    Err(ProjectionError::LeafTypedBlock { .. })
                ),
                "leaf-typed block {leaf_ty:?} must fail-loud"
            );
        }
    }

    #[test]
    fn unknown_anchor_span_is_dropped() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(7, 7),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(8, 8),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(
            pd.effective
                .get(&Dot::new(1, 2))
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
    }

    #[test]
    fn deleted_anchor_span_no_panic() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let c = Dot::new(1, 4);
        let mut ev = events(&[
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (a, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (c, SeqItem::Char('c')),
        ]);
        ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![c],
            op: ListOp::Del { pos: 3, len: 1 },
        });
        let mut l = logs_of(&[]);
        l.seq = build_oplog(&ev);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
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
        assert!(project_document(&l).is_ok());
    }

    #[test]
    fn stale_overlay_does_not_leak() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: Dot::new(9, 9),
                    op: LwwRegOp::Set {
                        value: Some("ghost".to_string()),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_styles.contains_key(&Dot::new(9, 9)));
    }

    #[test]
    fn duplicate_fixed_slot_loser_overlay_no_leak() {
        let fold = Dot::new(1, 1);
        let title1 = Dot::new(1, 2);
        let loser = Dot::new(1, 3);
        let content = Dot::new(1, 4);
        let elems = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                title1,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                loser,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
        ];
        let mut l = logs_of(&elems);
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: loser,
                    op: LwwRegOp::Set {
                        value: Some("x".to_string()),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_styles.contains_key(&loser));
    }

    #[test]
    fn effective_matches_module_reference() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 3),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Italic,
                },
            )
            .unwrap();
        let (els, resolver) = checkout_with_resolver(&l.seq);
        let tree = normalize(project_blocks(&els).unwrap());
        let node_type_of = collect_real_ids(&tree);
        let live: HashSet<Dot> = node_type_of.keys().copied().collect();
        let node_attrs = l.node_attrs.project(|d| node_type_of.get(&d).copied());
        let node_styles = filter_live(l.node_styles.project(), &live);
        let styles = l.styles.registered_entries();
        let explicit: HashMap<Dot, _> =
            crate::span::derive_explicit_effect(&els, &tree, &resolver, &l.spans)
                .into_iter()
                .collect();
        let src = crate::span::EffectiveSources {
            block_modifiers: &l.block_modifiers,
            explicit_spans: &explicit,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let direct: HashMap<Dot, BTreeMap<ModifierType, Modifier>> =
            derive_full_effective(&tree, &src).into_iter().collect();
        let pd = project_document(&l).unwrap();
        assert_eq!(pd.effective, direct);
    }

    #[test]
    fn project_document_own_modifiers_present() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.own_modifiers
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&crate::OwnModifier {
                value: Modifier::Bold,
                from_style: false
            })
        );
    }

    use proptest::prelude::*;

    fn arb_para_doc() -> impl Strategy<Value = Vec<(Dot, SeqItem)>> {
        "[a-c]{1,8}".prop_map(|s| {
            let para = Dot::new(1, 1);
            let mut v = vec![(
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            )];
            for (i, ch) in s.chars().enumerate() {
                v.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
            }
            v
        })
    }

    proptest! {
        #[test]
        fn invariants_hold(items in arb_para_doc()) {
            let pd = project_document(&logs_of(&items)).unwrap();

            prop_assert!(validate_block_tree(&pd.tree).is_ok());

            let live = collect_real_ids(&pd.tree);
            let live_leaves: HashSet<Dot> = live.iter()
                .filter(|(_, t)| t.spec().is_leaf())
                .map(|(d, _)| *d)
                .collect();

            let eff_keys: HashSet<Dot> = pd.effective.keys().copied().collect();
            prop_assert_eq!(eff_keys, live_leaves.clone());

            let mut covered: Vec<Dot> = pd.runs.iter().flat_map(|r| r.leaves.clone()).collect();
            covered.sort();
            let mut expect: Vec<Dot> = live_leaves.into_iter().collect();
            expect.sort();
            prop_assert_eq!(covered, expect);

            let all_ids: HashSet<Dot> = live.keys().copied().collect();
            for d in pd.node_attrs.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.node_styles.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.node_markers.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.block_modifiers.keys() { prop_assert!(all_ids.contains(d)); }

            fn collect_block_ids(node: &BlockNode, out: &mut HashSet<Dot>) {
                out.insert(node.id);
                for c in &node.children {
                    if let Child::Block(b) = c {
                        collect_block_ids(b, out);
                    }
                }
            }
            let mut block_ids: HashSet<Dot> = HashSet::new();
            for r in &pd.tree.roots {
                collect_block_ids(r, &mut block_ids);
            }
            let be_keys: HashSet<Dot> = pd.block_effective.keys().copied().collect();
            prop_assert_eq!(be_keys, block_ids);
        }

        #[test]
        fn deterministic_under_shuffle(items in arb_para_doc()) {
            let ev = events(&items);
            let mut a = logs_of(&[]);
            a.seq = build_oplog(&ev);
            let mut rev = ev.clone();
            rev.reverse();
            let mut b = logs_of(&[]);
            b.seq = build_oplog(&rev);
            prop_assert_eq!(project_document(&a).unwrap(), project_document(&b).unwrap());
        }
    }
}

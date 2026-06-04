use editor_crdt::Op;
use editor_model::{Doc, DocOp, NodeAttr, NodeId, TableNodeAttr};
use editor_resource::Resource;
use std::sync::{Arc, Mutex};

use crate::view_state::ViewState;

use super::MeasuredNode;
use super::cache::MeasureCache;
use super::nodes::dispatch;

pub struct Measurer {
    pub(crate) cache: MeasureCache,
    pub(crate) resource: Arc<Mutex<Resource>>,
}

impl std::fmt::Debug for Measurer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Measurer")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl Measurer {
    pub fn new(resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            cache: MeasureCache::new(),
            resource,
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn invalidate_with_doc_ops(
        &mut self,
        old_doc: &Doc,
        new_doc: &Doc,
        ops: &[Op<DocOp>],
    ) -> bool {
        // Many ops in a single batch typically target the same handful of
        // nodes (e.g. one IME-driven text). Deduplicating node ids before the
        // ancestor walk turns 200 redundant cache-invalidate cascades into one
        // per unique node.
        let mut affected: hashbrown::HashSet<NodeId> = hashbrown::HashSet::new();
        let mut affected_in_old: hashbrown::HashSet<NodeId> = hashbrown::HashSet::new();
        for op in ops {
            for id in affected_node_ids_for_doc_op(&op.payload, old_doc) {
                if new_doc.node(id).is_none() {
                    affected_in_old.insert(id);
                }
                affected.insert(id);
            }
        }
        let mut invalidated = false;
        for id in &affected {
            invalidated = self.invalidate_with_ancestors(new_doc, *id) || invalidated;
        }
        for id in &affected_in_old {
            invalidated = self.invalidate_with_ancestors(old_doc, *id) || invalidated;
        }
        for op in ops {
            match &op.payload {
                DocOp::Modifier { node_id, .. } => {
                    let doc_for_subtree = if new_doc.node(*node_id).is_some() {
                        new_doc
                    } else {
                        old_doc
                    };
                    invalidated =
                        self.invalidate_descendants(doc_for_subtree, *node_id) || invalidated;
                }
                DocOp::Style { .. } => {
                    for id in affected_node_ids_for_doc_op(&op.payload, old_doc) {
                        let doc_for_subtree = if new_doc.node(id).is_some() {
                            new_doc
                        } else {
                            old_doc
                        };
                        invalidated =
                            self.invalidate_descendants(doc_for_subtree, id) || invalidated;
                    }
                }
                // proportion은 각 셀의 측정 너비를 결정하므로 셀 자손까지 무효화한다 (border_style 등은 측정 무관).
                DocOp::Attr {
                    node_id,
                    op:
                        NodeAttr::Table {
                            attr: TableNodeAttr::Proportion(_),
                        },
                } => {
                    let doc_for_subtree = if new_doc.node(*node_id).is_some() {
                        new_doc
                    } else {
                        old_doc
                    };
                    invalidated =
                        self.invalidate_descendants(doc_for_subtree, *node_id) || invalidated;
                }
                _ => {}
            }
        }
        invalidated
    }

    pub fn invalidate_with_ancestors(&mut self, doc: &Doc, node_id: NodeId) -> bool {
        let mut invalidated = self.cache.invalidate(node_id);
        if let Some(node_ref) = doc.node(node_id)
            && let Some(parent) = node_ref.parent()
        {
            invalidated = self.invalidate_with_ancestors(doc, parent.id()) || invalidated;
        }
        invalidated
    }

    pub fn invalidate_descendants(&mut self, doc: &Doc, node_id: NodeId) -> bool {
        let Some(node_ref) = doc.node(node_id) else {
            return false;
        };
        let mut invalidated = false;
        for child in node_ref.children() {
            invalidated = self.cache.invalidate(child.id()) || invalidated;
            invalidated = self.invalidate_descendants(doc, child.id()) || invalidated;
        }
        invalidated
    }

    pub fn measure(
        &mut self,
        doc: &Doc,
        node_id: NodeId,
        width: f32,
        view_state: &ViewState,
    ) -> Arc<MeasuredNode> {
        if let Some(cached) = self.cache.get(node_id) {
            return cached.clone();
        }
        let node = doc.node(node_id).unwrap();
        let measured = dispatch::measure_node(self, doc, &node, width, view_state);
        let arc = Arc::new(measured);
        self.cache.insert(node_id, arc.clone());
        arc
    }
}

fn affected_node_ids_for_doc_op(op: &DocOp, old_doc: &Doc) -> Vec<NodeId> {
    use editor_crdt::{LwwRegOp, OrMapOp, RgaOp};
    match op {
        DocOp::Text { node_id, .. }
        | DocOp::Modifier { node_id, .. }
        | DocOp::Attr { node_id, .. }
        | DocOp::NodeStyle { node_id, .. } => {
            vec![*node_id]
        }
        DocOp::Style { style_id, .. } => old_doc
            .nodes_iter()
            .filter_map(|(node_id, _)| {
                let entry = old_doc.get_entry(*node_id)?;
                if entry.style.get().as_deref() == Some(style_id.as_str()) {
                    Some(*node_id)
                } else {
                    None
                }
            })
            .collect(),
        DocOp::Presence { node_id, op } => {
            let mut ids = vec![*node_id];
            if let OrMapOp::Unset { .. } = op
                && let Some(entry) = old_doc.get_entry(*node_id)
                && let Some(parent) = *entry.parent.get()
            {
                ids.push(parent);
            }
            ids
        }
        DocOp::Parent { node_id, op } => {
            let mut ids = vec![*node_id];
            if let Some(entry) = old_doc.get_entry(*node_id)
                && let Some(parent) = *entry.parent.get()
            {
                ids.push(parent);
            }
            if let LwwRegOp::Set { value: Some(p) } = op {
                ids.push(*p);
            }
            ids
        }
        DocOp::Children {
            node_id: parent_id,
            op,
        } => {
            let mut ids = vec![*parent_id];
            match op {
                RgaOp::Insert { value, .. } => ids.push(*value),
                RgaOp::Remove { observed } => {
                    // RgaOp::Remove carries only the observed Dot; the NodeId only
                    // survives in the baseline RGA, so look it up there.
                    if let Some(entry) = old_doc.get_entry(*parent_id)
                        && let Some(child) = entry.children.get(*observed)
                    {
                        ids.push(*child);
                    }
                }
            }
            // Sibling children share parent context (indent/padding from list_item,
            // blockquote, etc.), so a structural change can shift their layout.
            // Measurer cache keys on NodeId only, so invalidate all live siblings
            // to evict potentially stale measurements.
            if let Some(entry) = old_doc.get_entry(*parent_id) {
                ids.extend(entry.children.iter().copied());
            }
            ids
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Measurer {
    pub fn new_test() -> Self {
        use fontique::ScriptExt;

        let mut resource = Resource::new_test();
        let font_data = include_bytes!("../../assets/test-font.ttf");
        let families = resource.font_context.collection.register_fonts(
            fontique::Blob::new(Arc::new(font_data.to_vec())),
            Some(fontique::FontInfoOverride {
                family_name: Some("Noto Sans"),
                weight: Some(fontique::FontWeight::new(400.0)),
                ..Default::default()
            }),
        );
        let family_ids: Vec<_> = families.into_iter().map(|(id, _)| id).collect();
        for &script in fontique::Script::all_samples()
            .iter()
            .map(|(s, _)| s)
            .chain(&[
                fontique::Script::COMMON,
                fontique::Script::INHERITED,
                fontique::Script::UNKNOWN,
            ])
        {
            resource.font_context.collection.set_fallbacks(
                fontique::FallbackKey::new(script, None),
                family_ids.iter().copied(),
            );
        }
        Self::new(Arc::new(Mutex::new(resource)))
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;
    use editor_model::{Doc, NodeId};

    use super::*;
    use crate::measure::{MeasuredContent, MeasuredNode};

    fn dummy() -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 100.0,
            height: 20.0,
            content: MeasuredContent::Spacing(0.0),
        })
    }

    #[test]
    fn cached_atom_index_updates_after_sibling_removal() {
        use crate::measure::MeasuredTree;
        use crate::paginate::{LayoutContent, Paginator};
        use editor_common::EdgeInsets;
        use editor_crdt::{Dot, RgaOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        let (doc1, p1, ..) = doc! {
            root {
                p1: paragraph
                horizontal_rule
                paragraph
            }
        };

        let _ = measurer.measure(&doc1, NodeId::ROOT, 400.0, &vs);

        let root_entry = doc1.get_entry(NodeId::ROOT).unwrap();
        let p1_dot = root_entry
            .children
            .iter_with_dot()
            .find(|(_, id)| **id == p1)
            .map(|(d, _)| d)
            .unwrap();

        let mut plain = doc1.to_plain();
        plain.nodes.remove(&p1);
        if let Some(root_entry) = plain.nodes.get_mut(&NodeId::ROOT) {
            root_entry.children.retain(|&id| id != p1);
        }
        let (doc2, _) = Doc::from_plain(plain);

        let ops = vec![make_op(
            Dot::new(99, 0),
            DocOp::Children {
                node_id: NodeId::ROOT,
                op: RgaOp::Remove { observed: p1_dot },
            },
        )];
        measurer.invalidate_with_doc_ops(&doc1, &doc2, &ops);

        let root = measurer.measure(&doc2, NodeId::ROOT, 400.0, &vs);
        let tree = MeasuredTree {
            root: std::sync::Arc::unwrap_or_clone(root),
        };
        let paginator = Paginator::continuous(440.0, 1024.0, EdgeInsets::all(20.0));
        let layout = paginator.paginate(tree).tree;

        let LayoutContent::Box(root_box) = &layout.root.content else {
            panic!("expected box");
        };
        let atom = root_box
            .children
            .iter()
            .find_map(|c| match &c.content {
                LayoutContent::Atom(a) => Some(a),
                _ => None,
            })
            .expect("should find atom");

        assert_eq!(atom.index, 0, "atom index should reflect current position");
        assert_eq!(atom.parent_id, NodeId::ROOT);
    }

    fn make_op(id: editor_crdt::Dot, payload: DocOp) -> Op<DocOp> {
        Op {
            id,
            parents: Default::default(),
            payload,
        }
    }

    #[test]
    fn doc_ops_text_invalidates_self_and_ancestors() {
        use editor_crdt::{Dot, TextOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();

        let (doc, p, t) = doc! {
            root {
                p: paragraph {
                    t: text("hello")
                }
            }
        };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p, dummy());
        measurer.cache.insert(t, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Text {
                node_id: t,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'x',
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(t).is_none(),
            "text node should be invalidated"
        );
        assert!(
            measurer.cache.get(p).is_none(),
            "paragraph should be invalidated"
        );
        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "root should be invalidated"
        );
    }

    #[test]
    fn doc_ops_attr_invalidates_self() {
        use editor_crdt::Dot;
        use editor_model::{CalloutNodeAttr, CalloutVariant, DocOp, NodeAttr};

        let mut measurer = Measurer::new_test();
        let (doc, c) = doc! { root { c: callout } };

        measurer.cache.insert(c, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: c,
                op: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(c).is_none(),
            "callout should be invalidated"
        );
    }

    #[test]
    fn doc_ops_table_proportion_invalidates_cells_and_triggers_relayout() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, TableNodeAttr};

        let mut measurer = Measurer::new_test();
        let (doc, t, c, p, tx) = doc! {
            root {
                t: table {
                    table_row {
                        c: table_cell {
                            p: paragraph {
                                tx: text("A")
                            }
                        }
                    }
                }
            }
        };

        measurer.cache.insert(t, dummy());
        measurer.cache.insert(c, dummy());
        measurer.cache.insert(p, dummy());
        measurer.cache.insert(tx, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: t,
                op: NodeAttr::Table {
                    attr: TableNodeAttr::Proportion(50),
                },
            },
        )];

        let invalidated = measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        // A non-empty invalidation is what makes tick() report `dirty`, which in
        // turn triggers relayout + RenderInvalidated. set_proportion therefore
        // drives layout/render purely through the doc-op invalidation path.
        assert!(invalidated, "proportion change must report an invalidation");
        assert!(
            measurer.cache.get(t).is_none(),
            "table should be invalidated"
        );
        assert!(
            measurer.cache.get(c).is_none(),
            "cell must be invalidated: proportion changes its measured width"
        );
        assert!(
            measurer.cache.get(p).is_none(),
            "cell's paragraph descendant must be invalidated"
        );
        assert!(
            measurer.cache.get(tx).is_none(),
            "cell's text descendant must be invalidated"
        );
    }

    #[test]
    fn doc_ops_table_border_style_preserves_cell_descendants() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, TableBorderStyle, TableNodeAttr};

        let mut measurer = Measurer::new_test();
        let (doc, t, c, p, tx) = doc! {
            root {
                t: table {
                    table_row {
                        c: table_cell {
                            p: paragraph {
                                tx: text("A")
                            }
                        }
                    }
                }
            }
        };

        measurer.cache.insert(t, dummy());
        measurer.cache.insert(c, dummy());
        measurer.cache.insert(p, dummy());
        measurer.cache.insert(tx, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: t,
                op: NodeAttr::Table {
                    attr: TableNodeAttr::BorderStyle(TableBorderStyle::Dashed),
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(t).is_none(),
            "table itself should still be invalidated"
        );
        assert!(
            measurer.cache.get(c).is_some(),
            "cell must be preserved: border_style does not affect cell measurement"
        );
        assert!(
            measurer.cache.get(p).is_some(),
            "cell's paragraph descendant must be preserved"
        );
        assert!(
            measurer.cache.get(tx).is_some(),
            "cell's text descendant must be preserved"
        );
    }

    #[test]
    fn doc_ops_modifier_invalidates_self_and_preserves_unrelated() {
        use editor_crdt::{Dot, OrMapOp};
        use editor_model::{DocOp, Modifier, ModifierType};

        let mut measurer = Measurer::new_test();
        let (doc, p, sibling) = doc! {
            root {
                p: paragraph
                sibling: paragraph
            }
        };

        measurer.cache.insert(p, dummy());
        measurer.cache.insert(sibling, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Modifier {
                node_id: p,
                op: OrMapOp::Set {
                    key: ModifierType::Bold,
                    value: Modifier::Bold,
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(p).is_none(),
            "paragraph should be invalidated"
        );
        assert!(
            measurer.cache.get(sibling).is_some(),
            "unrelated sibling must be preserved"
        );
    }

    #[test]
    fn doc_ops_modifier_on_root_invalidates_descendant_subtree() {
        use editor_crdt::{Dot, OrMapOp};
        use editor_model::{DocOp, Modifier, ModifierType};

        let mut measurer = Measurer::new_test();
        let (doc, p, t) = doc! {
            root {
                p: paragraph {
                    t: text("hello")
                }
            }
        };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p, dummy());
        measurer.cache.insert(t, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Modifier {
                node_id: NodeId::ROOT,
                op: OrMapOp::Set {
                    key: ModifierType::FontSize,
                    value: Modifier::FontSize { value: 2400 },
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "root must be invalidated"
        );
        assert!(
            measurer.cache.get(p).is_none(),
            "descendant paragraph must be invalidated"
        );
        assert!(
            measurer.cache.get(t).is_none(),
            "descendant text must be invalidated"
        );
    }

    #[test]
    fn doc_ops_style_invalidates_styled_node_and_descendants() {
        use editor_crdt::{Dot, OrSetOp};
        use editor_macros::state;
        use editor_model::{DocOp, Modifier, PlainStyleEntry, StyleOp};
        use editor_transaction::Transaction;

        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![Modifier::FontFamily {
                    value: "Pretendard".into(),
                }]
                .into_iter()
                .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        let (state_with_style, ..) = tr.commit();
        let doc = state_with_style.doc;

        let t1 = doc.node(p1).unwrap().children().next().unwrap().id();

        let mut measurer = Measurer::new_test();
        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p1, dummy());
        measurer.cache.insert(t1, dummy());

        let ops = vec![make_op(
            Dot::new(99, 0),
            DocOp::Style {
                style_id: "h1".into(),
                op: StyleOp::Modifiers(OrSetOp::Add {
                    elem: Modifier::FontFamily {
                        value: "Arial".into(),
                    },
                }),
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(p1).is_none(),
            "styled paragraph should be invalidated"
        );
        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "root (ancestor) should be invalidated"
        );
        assert!(
            measurer.cache.get(t1).is_none(),
            "descendant text node must be invalidated so cascading font_family takes effect"
        );
    }

    #[test]
    fn doc_ops_presence_set_invalidates_self() {
        use editor_crdt::{Dot, OrMapOp};
        use editor_model::{DocOp, NodeType};

        let mut measurer = Measurer::new_test();
        let (doc, p) = doc! { root { p: paragraph } };

        measurer.cache.insert(p, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Presence {
                node_id: p,
                op: OrMapOp::Set {
                    key: p,
                    value: NodeType::Paragraph,
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(p).is_none(),
            "paragraph should be invalidated"
        );
    }

    #[test]
    fn doc_ops_presence_unset_invalidates_old_parent_too() {
        use editor_crdt::{Dot, OrMapOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();
        let (doc, p) = doc! { root { p: paragraph } };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Presence {
                node_id: p,
                op: OrMapOp::Unset { observed: vec![] },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(p).is_none(),
            "paragraph should be invalidated"
        );
        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "old parent (root) should be invalidated"
        );
    }

    #[test]
    fn doc_ops_parent_set_invalidates_old_and_new_parent() {
        use editor_crdt::{Dot, LwwRegOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();

        let (doc, p1, p2, child) = doc! {
            root {
                p1: paragraph {
                    child: paragraph
                }
                p2: paragraph
            }
        };

        measurer.cache.insert(p1, dummy());
        measurer.cache.insert(p2, dummy());
        measurer.cache.insert(child, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Parent {
                node_id: child,
                op: LwwRegOp::Set { value: Some(p2) },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(child).is_none(),
            "child should be invalidated"
        );
        assert!(
            measurer.cache.get(p1).is_none(),
            "old parent should be invalidated"
        );
        assert!(
            measurer.cache.get(p2).is_none(),
            "new parent should be invalidated"
        );
    }

    #[test]
    fn doc_ops_children_insert_invalidates_parent_and_value() {
        use editor_crdt::{Dot, RgaOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();

        let (doc, p1, p2) = doc! {
            root {
                p1: paragraph
                p2: paragraph
            }
        };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p1, dummy());
        measurer.cache.insert(p2, dummy());

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Children {
                node_id: NodeId::ROOT,
                op: RgaOp::Insert {
                    after: None,
                    value: p1,
                },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "parent should be invalidated"
        );
        assert!(
            measurer.cache.get(p1).is_none(),
            "inserted child should be invalidated"
        );
        assert!(
            measurer.cache.get(p2).is_none(),
            "alive sibling must also be invalidated (shared parent context)"
        );
    }

    #[test]
    fn doc_ops_children_remove_resolves_via_baseline_rga() {
        use editor_crdt::{Dot, RgaOp};
        use editor_model::DocOp;

        let mut measurer = Measurer::new_test();

        let (doc, p1, p2) = doc! {
            root {
                p1: paragraph
                p2: paragraph
            }
        };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(p1, dummy());
        measurer.cache.insert(p2, dummy());

        let root_entry = doc.get_entry(NodeId::ROOT).unwrap();
        let p1_dot = root_entry
            .children
            .iter_with_dot()
            .find(|(_, id)| **id == p1)
            .map(|(d, _)| d)
            .unwrap();

        let ops = vec![make_op(
            Dot::new(99, 0),
            DocOp::Children {
                node_id: NodeId::ROOT,
                op: RgaOp::Remove { observed: p1_dot },
            },
        )];

        measurer.invalidate_with_doc_ops(&doc, &doc, &ops);

        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "parent should be invalidated"
        );
        assert!(
            measurer.cache.get(p1).is_none(),
            "removed child resolved via baseline Rga should be invalidated"
        );
        assert!(
            measurer.cache.get(p2).is_none(),
            "sibling shift invalidates alive siblings"
        );
    }
}

use editor_model::{Doc, NodeId};
use editor_resource::Resource;
use editor_transaction::Step;
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

    pub fn invalidate_with_steps(&mut self, old_doc: &Doc, new_doc: &Doc, steps: &[Step]) -> bool {
        let mut invalidated = false;
        for step in steps {
            for id in step.affected_node_ids(old_doc, new_doc) {
                invalidated = self.invalidate_with_ancestors(new_doc, id) || invalidated;
                if new_doc.node(id).is_none() {
                    invalidated = self.invalidate_with_ancestors(old_doc, id) || invalidated;
                }
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
    use editor_model::{Doc, NodeId, Subtree};
    use editor_state::{Position, Selection};

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
    fn invalidate_clears_node_and_ancestors() {
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

        let steps = vec![Step::InsertText {
            node_id: t,
            offset: 5,
            text: " world".into(),
        }];

        measurer.invalidate_with_steps(&doc, &doc, &steps);

        assert!(
            measurer.cache.get(t).is_none(),
            "text should be invalidated"
        );
        assert!(
            measurer.cache.get(p).is_none(),
            "para should be invalidated"
        );
        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "root should be invalidated"
        );
    }

    #[test]
    fn invalidate_preserves_unrelated_nodes() {
        let mut measurer = Measurer::new_test();

        let (doc, p1, t, p2) = doc! {
            root {
                p1: paragraph {
                    t: text("hello")
                }
                p2: paragraph
            }
        };

        measurer.cache.insert(p1, dummy());
        measurer.cache.insert(p2, dummy());
        measurer.cache.insert(t, dummy());

        measurer.invalidate_with_steps(
            &doc,
            &doc,
            &[Step::InsertText {
                node_id: t,
                offset: 0,
                text: "x".into(),
            }],
        );

        assert!(measurer.cache.get(t).is_none());
        assert!(measurer.cache.get(p1).is_none());
        assert!(
            measurer.cache.get(p2).is_some(),
            "para2 should be preserved"
        );
    }

    #[test]
    fn merge_node_invalidates_source_parent() {
        let mut measurer = Measurer::new_test();

        // old_doc: root > [target, wrapper > [source, remaining]]
        let (old_doc, target_id, wrapper_id, source_id, remaining_id) = doc! {
            root {
                target_id: paragraph
                wrapper_id: paragraph {
                    source_id: paragraph
                    remaining_id: paragraph
                }
            }
        };

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(target_id, dummy());
        measurer.cache.insert(wrapper_id, dummy());
        measurer.cache.insert(remaining_id, dummy());
        measurer.cache.insert(source_id, dummy());

        // new_doc: root > [target, wrapper > [remaining]] (source removed)
        let mut plain = old_doc.to_plain();
        plain.nodes.remove(&source_id);
        if let Some(wrapper_entry) = plain.nodes.get_mut(&wrapper_id) {
            wrapper_entry.children.retain(|&id| id != source_id);
        }
        let (new_doc, _) = Doc::from_plain(plain);

        let steps = vec![Step::MergeNode {
            node_id: source_id,
            target_id,
            offset: 0,
        }];

        measurer.invalidate_with_steps(&old_doc, &new_doc, &steps);

        assert!(
            measurer.cache.get(target_id).is_none(),
            "target should be invalidated"
        );
        assert!(
            measurer.cache.get(NodeId::ROOT).is_none(),
            "root should be invalidated"
        );
        assert!(
            measurer.cache.get(wrapper_id).is_none(),
            "wrapper (former parent of merged-away source) should be invalidated"
        );
    }

    #[test]
    fn selection_step_invalidates_nothing() {
        let mut measurer = Measurer::new_test();

        let id = NodeId::new();
        measurer.cache.insert(id, dummy());

        let (doc,) = doc! { root { paragraph } };
        let sel = Selection::collapsed(Position::new(id, 0));
        measurer.invalidate_with_steps(&doc, &doc, &[Step::SetSelection { old: sel, new: sel }]);

        assert!(measurer.cache.get(id).is_some());
    }

    #[test]
    fn cached_atom_index_updates_after_sibling_removal() {
        use crate::measure::MeasuredTree;
        use crate::paginate::{LayoutContent, Paginator};
        use editor_common::EdgeInsets;

        let mut measurer = Measurer::new_test();
        let vs = ViewState::new();

        // doc1: root { p1, hr, p2 } — hr at child index 1
        let (doc1, p1, ..) = doc! {
            root {
                p1: paragraph
                horizontal_rule
                paragraph
            }
        };

        // Measure full doc — hr gets cached
        let _ = measurer.measure(&doc1, NodeId::ROOT, 400.0, &vs);

        // Delete p1; keep hr at new index 0
        let steps = vec![Step::RemoveSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree: Subtree::leaf(
                p1,
                editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode::default()),
            ),
        }];

        // doc2: root { hr, p2 }
        let mut plain = doc1.to_plain();
        plain.nodes.remove(&p1);
        if let Some(root_entry) = plain.nodes.get_mut(&NodeId::ROOT) {
            root_entry.children.retain(|&id| id != p1);
        }
        let (doc2, _) = Doc::from_plain(plain);

        measurer.invalidate_with_steps(&doc1, &doc2, &steps);

        // Re-measure and paginate
        let root = measurer.measure(&doc2, NodeId::ROOT, 400.0, &vs);
        let tree = MeasuredTree {
            root: std::sync::Arc::unwrap_or_clone(root),
        };
        let paginator = Paginator::continuous(440.0, 1024.0, EdgeInsets::all(20.0));
        let (layout, _) = paginator.paginate(tree);

        // Find the atom in the layout tree
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

        // After p1 deletion, hr is child index 0
        assert_eq!(atom.index, 0, "atom index should reflect current position");
        assert_eq!(atom.parent_id, NodeId::ROOT);
    }
}

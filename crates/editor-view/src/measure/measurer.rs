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

    pub fn invalidate_with_steps(&mut self, doc: &Doc, steps: &[Step]) -> bool {
        let mut invalidated = false;
        for step in steps {
            for id in step.affected_node_ids() {
                invalidated = self.invalidate_with_ancestors(doc, id) || invalidated;
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

        let mut resource = Resource::new();
        let font_data = include_bytes!("../../assets/Noto-Phantom.ttf");
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
    use editor_model::{Doc, Node, NodeEntry, NodeId, ParagraphNode, TextAlign, TextNode};
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

        let paragraph_id = NodeId::new();
        let text_id = NodeId::new();

        measurer.cache.insert(NodeId::ROOT, dummy());
        measurer.cache.insert(paragraph_id, dummy());
        measurer.cache.insert(text_id, dummy());

        let doc = Doc::new_test()
            .insert_node(
                paragraph_id,
                NodeEntry::new(Node::Paragraph(ParagraphNode {
                    align: TextAlign::Left,
                }))
                .with_parent(NodeId::ROOT),
            )
            .insert_node(
                text_id,
                NodeEntry::new(Node::Text(TextNode {
                    text: "hello".into(),
                }))
                .with_parent(paragraph_id),
            );

        let steps = vec![Step::InsertText {
            node_id: text_id,
            offset: 5,
            text: " world".into(),
        }];

        measurer.invalidate_with_steps(&doc, &steps);

        assert!(
            measurer.cache.get(text_id).is_none(),
            "text should be invalidated"
        );
        assert!(
            measurer.cache.get(paragraph_id).is_none(),
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

        let paragraph1_id = NodeId::new();
        let paragraph2_id = NodeId::new();
        let text_id = NodeId::new();

        measurer.cache.insert(paragraph1_id, dummy());
        measurer.cache.insert(paragraph2_id, dummy());
        measurer.cache.insert(text_id, dummy());

        let doc = Doc::new_test()
            .insert_node(
                paragraph1_id,
                NodeEntry::new(Node::Paragraph(ParagraphNode {
                    align: TextAlign::Left,
                }))
                .with_parent(NodeId::ROOT),
            )
            .insert_node(
                text_id,
                NodeEntry::new(Node::Text(TextNode {
                    text: "hello".into(),
                }))
                .with_parent(paragraph1_id),
            )
            .insert_node(
                paragraph2_id,
                NodeEntry::new(Node::Paragraph(ParagraphNode {
                    align: TextAlign::Left,
                }))
                .with_parent(NodeId::ROOT),
            );

        measurer.invalidate_with_steps(
            &doc,
            &[Step::InsertText {
                node_id: text_id,
                offset: 0,
                text: "x".into(),
            }],
        );

        assert!(measurer.cache.get(text_id).is_none());
        assert!(measurer.cache.get(paragraph1_id).is_none());
        assert!(
            measurer.cache.get(paragraph2_id).is_some(),
            "para2 should be preserved"
        );
    }

    #[test]
    fn selection_step_invalidates_nothing() {
        let mut measurer = Measurer::new_test();

        let id = NodeId::new();
        measurer.cache.insert(id, dummy());

        let doc = Doc::new_test();
        let sel = Selection::collapsed(Position::new(id, 0));
        measurer.invalidate_with_steps(&doc, &[Step::SetSelection { old: sel, new: sel }]);

        assert!(measurer.cache.get(id).is_some());
    }
}

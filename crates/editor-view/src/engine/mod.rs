mod cache;
pub(crate) mod measure_nodes;
mod paginator;
pub(crate) mod resolve;

pub use cache::LayoutCache;

use editor_common::{Alignment, Size};
use editor_model::{Doc, LayoutMode, Node, NodeId};
use editor_resource::Resource;
use editor_transaction::Step;
use std::sync::{Arc, Mutex};

use crate::measure::*;
use crate::page::Page;
use crate::view_state::ViewState;
use crate::viewport::Viewport;

pub struct LayoutEngine {
    pub(crate) cache: LayoutCache,
    pages: Vec<Page>,
    pub(crate) resource: Arc<Mutex<Resource>>,
}

impl std::fmt::Debug for LayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutEngine")
            .field("cache", &self.cache)
            .field("pages", &self.pages)
            .finish()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl LayoutEngine {
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

impl LayoutEngine {
    pub fn new(resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            cache: LayoutCache::new(),
            pages: vec![],
            resource,
        }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn invalidate_with_steps(&mut self, doc: &Doc, steps: &[Step]) -> bool {
        let mut invalidated = false;

        for step in steps {
            for id in dirty_nodes(step) {
                invalidated = self.invalidate_with_ancestors(doc, id);
            }
        }

        invalidated
    }

    pub fn invalidate_with_ancestors(&mut self, doc: &Doc, node_id: NodeId) -> bool {
        let mut invalidated = self.cache.invalidate(node_id);

        if let Some(node_ref) = doc.node(node_id) {
            if let Some(parent) = node_ref.parent() {
                invalidated = self.invalidate_with_ancestors(doc, parent.id());
            }
        }

        invalidated
    }

    pub fn compute(&mut self, doc: &Doc, viewport: &Viewport, view_state: &ViewState) {
        let (content_width, mut paginator) = match doc.attrs().layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => {
                let cw = page_width - page_margin_left - page_margin_right;
                let p = paginator::Paginator::new_paginated(
                    cw,
                    page_height,
                    page_margin_top,
                    page_margin_bottom,
                    page_margin_left,
                );
                (cw, p)
            }
            LayoutMode::Continuous { max_width } => {
                const CONTINUOUS_MAX_CONTENT_HEIGHT: f32 = 1024.0;
                const CONTINUOUS_MARGIN: f32 = 20.0;
                let cw = viewport.width.min(max_width);
                let p = paginator::Paginator::new_continuous(
                    cw,
                    CONTINUOUS_MAX_CONTENT_HEIGHT,
                    CONTINUOUS_MARGIN,
                    CONTINUOUS_MARGIN,
                    CONTINUOUS_MARGIN,
                );
                (cw, p)
            }
        };

        let root_m = self.measure(doc, NodeId::ROOT, content_width, view_state);

        if let MeasuredContent::Container(ContainerContent { children, .. }) = &root_m.content {
            for child in children {
                paginator.place(child.node_id, &child.measurement);
            }
        }

        self.pages = paginator.finish();
    }

    pub(crate) fn measure(
        &mut self,
        doc: &Doc,
        node_id: NodeId,
        width: f32,
        view_state: &ViewState,
    ) -> Arc<Measurement> {
        if let Some(cached) = self.cache.get(&node_id) {
            return cached.clone();
        }

        let node = doc.node(node_id).unwrap();
        let measurement = self.measure_inner(doc, &node, width, view_state);
        let arc = Arc::new(measurement);
        self.cache.insert(node_id, arc.clone());

        arc
    }

    fn measure_inner(
        &mut self,
        doc: &Doc,
        node: &editor_model::NodeRef<'_>,
        width: f32,
        view_state: &ViewState,
    ) -> Measurement {
        match node.node() {
            Node::Image(_)
            | Node::File(_)
            | Node::Embed(_)
            | Node::Archived(_)
            | Node::HorizontalRule(_) => measure_nodes::measure_atom(node, width, view_state),
            Node::PageBreak(_) => Measurement {
                size: Size { width, height: 0.0 },
                gap_after: 0.0,
                content: MeasuredContent::PageBreak,
                alignment: Alignment::Start,
            },
            Node::ListItem(_) => {
                measure_nodes::measure_list_item(self, doc, node, width, view_state)
            }
            Node::Blockquote(_) => {
                measure_nodes::measure_blockquote(self, doc, node, width, view_state)
            }
            Node::Callout(_) => measure_nodes::measure_callout(self, doc, node, width, view_state),
            Node::Fold(_) => measure_nodes::measure_fold(self, doc, node, width, view_state),
            Node::FoldTitle(_) => {
                measure_nodes::measure_fold_title(self, doc, node, width, view_state)
            }
            Node::FoldContent(_) => {
                measure_nodes::measure_fold_content(self, doc, node, width, view_state)
            }
            Node::Table(_) => measure_nodes::measure_table(self, doc, node, width, view_state),
            Node::TableCell(_) => {
                measure_nodes::measure_table_cell(self, doc, node, width, view_state)
            }
            Node::Paragraph(_) => measure_nodes::measure_paragraph(self, doc, node, width),
            _ => measure_nodes::measure_default_container(self, doc, node, width, view_state),
        }
    }
}

fn dirty_nodes(step: &Step) -> Vec<NodeId> {
    match step {
        Step::InsertText { node_id, .. }
        | Step::RemoveText { node_id, .. }
        | Step::AddModifier { node_id, .. }
        | Step::RemoveModifier { node_id, .. }
        | Step::SetModifiers { node_id, .. }
        | Step::SetNode { node_id, .. } => vec![*node_id],
        Step::InsertSubtree { parent_id, .. } | Step::RemoveSubtree { parent_id, .. } => {
            vec![*parent_id]
        }
        Step::SplitNode { node_id, .. } | Step::MergeNode { node_id, .. } => vec![*node_id],
        Step::MoveNode {
            old_parent,
            new_parent,
            ..
        } => vec![*old_parent, *new_parent],
        Step::SetSelection { .. }
        | Step::SetPendingModifiers { .. }
        | Step::SetComposition { .. }
        | Step::SetDocumentAttrs { .. } => vec![],
    }
}

#[cfg(test)]
mod tests {
    use editor_common::Size;
    use editor_macros::doc;
    use editor_model::{Doc, Node, NodeEntry, NodeId, ParagraphNode, TextAlign, TextNode};
    use editor_state::{Position, Selection};
    use std::sync::Arc;

    use super::*;
    use crate::measure::{MeasuredContent, Measurement};

    fn dummy() -> Arc<Measurement> {
        Arc::new(Measurement {
            size: Size {
                width: 100.0,
                height: 20.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Atom {
                parent_id: NodeId::ROOT,
                index: 0,
            },
        })
    }

    #[test]
    fn invalidate_clears_node_and_ancestors() {
        let mut engine = LayoutEngine::new_test();

        let paragraph_id = NodeId::new();
        let text_id = NodeId::new();

        engine.cache.insert(NodeId::ROOT, dummy());
        engine.cache.insert(paragraph_id, dummy());
        engine.cache.insert(text_id, dummy());

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

        engine.invalidate_with_steps(&doc, &steps);

        assert!(
            engine.cache.get(&text_id).is_none(),
            "text should be invalidated"
        );
        assert!(
            engine.cache.get(&paragraph_id).is_none(),
            "para should be invalidated"
        );
        assert!(
            engine.cache.get(&NodeId::ROOT).is_none(),
            "root should be invalidated"
        );
    }

    #[test]
    fn invalidate_preserves_unrelated_nodes() {
        let mut engine = LayoutEngine::new_test();

        let paragraph1_id = NodeId::new();
        let paragraph2_id = NodeId::new();
        let text_id = NodeId::new();

        engine.cache.insert(paragraph1_id, dummy());
        engine.cache.insert(paragraph2_id, dummy());
        engine.cache.insert(text_id, dummy());

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

        engine.invalidate_with_steps(
            &doc,
            &[Step::InsertText {
                node_id: text_id,
                offset: 0,
                text: "x".into(),
            }],
        );

        assert!(engine.cache.get(&text_id).is_none());
        assert!(engine.cache.get(&paragraph1_id).is_none());
        assert!(
            engine.cache.get(&paragraph2_id).is_some(),
            "para2 should be preserved"
        );
    }

    #[test]
    fn selection_step_invalidates_nothing() {
        let mut engine = LayoutEngine::new_test();

        let id = NodeId::new();
        engine.cache.insert(id, dummy());

        let doc = Doc::new_test();
        let sel = Selection::collapsed(Position::new(id, 0));
        engine.invalidate_with_steps(&doc, &[Step::SetSelection { old: sel, new: sel }]);

        assert!(engine.cache.get(&id).is_some());
    }

    #[test]
    fn compute_with_fold_collapsed() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let viewport = Viewport {
            width: 400.0,
            height: 800.0,
            scale_factor: 1.0,
        };
        let mut vs = ViewState::new();
        vs.fold_states.insert(f1, false);

        let mut engine = LayoutEngine::new_test();
        engine.compute(&doc, &viewport, &vs);

        assert!(!engine.pages().is_empty());
        let page = &engine.pages()[0];
        assert!(!page.fragments.is_empty());
    }

    #[test]
    fn compute_with_fold_expanded() {
        let (doc, f1) = doc! {
            root {
                f1: fold {
                    fold_title { paragraph { text("Title") } }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };

        let viewport = Viewport {
            width: 400.0,
            height: 800.0,
            scale_factor: 1.0,
        };
        let mut vs = ViewState::new();
        vs.fold_states.insert(f1, true);

        let mut engine = LayoutEngine::new_test();
        engine.compute(&doc, &viewport, &vs);

        assert!(!engine.pages().is_empty());
    }

    #[test]
    fn compute_with_table() {
        let (doc,) = doc! {
            root {
                table {
                    table_row {
                        table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            }
        };

        let viewport = Viewport {
            width: 400.0,
            height: 800.0,
            scale_factor: 1.0,
        };
        let vs = ViewState::new();

        let mut engine = LayoutEngine::new_test();
        engine.compute(&doc, &viewport, &vs);

        assert!(!engine.pages().is_empty());
        let page = &engine.pages()[0];
        assert!(!page.fragments.is_empty());
    }

    #[test]
    fn compute_with_paragraph_text() {
        let (doc,) = doc! {
            root { paragraph { text("Hello, world!") } }
        };
        let viewport = Viewport {
            width: 400.0,
            height: 800.0,
            scale_factor: 1.0,
        };
        let vs = ViewState::new();
        let mut engine = LayoutEngine::new_test();
        engine.compute(&doc, &viewport, &vs);
        assert!(!engine.pages().is_empty());
        let page = &engine.pages()[0];
        assert!(!page.fragments.is_empty());
    }

    #[test]
    fn paragraph_produces_text_block() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("Hello") } }
        };
        let mut engine = LayoutEngine::new_test();
        let vs = ViewState::new();
        let m = engine.measure(&doc, p1, 400.0, &vs);
        assert!(matches!(m.content, MeasuredContent::TextBlock { .. }));
        assert!(
            m.size.height > 0.0,
            "paragraph with text should have height"
        );
        assert_eq!(m.size.width, 400.0);
    }

    #[test]
    fn paragraph_multiple_styled_runs() {
        let (doc, p1) = doc! {
            root { p1: paragraph { text("normal") text("bold") [font_size(2400)] } }
        };
        let mut engine = LayoutEngine::new_test();
        let vs = ViewState::new();
        let m = engine.measure(&doc, p1, 400.0, &vs);
        assert!(matches!(m.content, MeasuredContent::TextBlock { .. }));
        assert!(
            m.size.height > 0.0,
            "multi-run paragraph should have height"
        );
    }

    #[test]
    fn empty_paragraph_has_height() {
        let (doc, p1) = doc! {
            root { p1: paragraph }
        };
        let mut engine = LayoutEngine::new_test();
        let vs = ViewState::new();
        let m = engine.measure(&doc, p1, 400.0, &vs);
        assert!(
            m.size.height > 0.0,
            "empty paragraph should have strut-based height"
        );
    }
}

use std::sync::Arc;

use editor_model::NodeView;
use editor_resource::Resource;

use crate::measure::cache::MeasureCache;
use crate::measure::context::MeasureContext;
use crate::measure::nodes::dispatch::measure_node;
use crate::measure::types::MeasuredNode;

pub(crate) struct Measurer {
    cache: MeasureCache,
    pub(crate) seg_cache: crate::measure::text::seg_cache::SegmentCache,
}

impl Measurer {
    pub(crate) fn new() -> Self {
        Self {
            cache: MeasureCache::new(),
            seg_cache: Default::default(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.seg_cache.clear();
    }

    pub(crate) fn measure(
        &mut self,
        node: &NodeView,
        width: f32,
        ctx: &MeasureContext,
        resource: &mut Resource,
    ) -> Arc<MeasuredNode> {
        let id = node.id();
        if let Some(cached) = self.cache.get(id, width) {
            return cached.clone();
        }
        let measured = Arc::new(measure_node(self, node, width, ctx, resource));
        self.cache.insert(id, width, measured.clone());
        measured
    }

    pub(crate) fn invalidate_with_ancestors(&mut self, node: &NodeView) -> bool {
        let mut invalidated = self.cache.invalidate(node.id());
        for anc in node.ancestors() {
            invalidated = self.cache.invalidate(anc.id()) || invalidated;
        }
        invalidated
    }

    pub(crate) fn invalidate_subtree(&mut self, node: &NodeView) -> bool {
        let mut invalidated = self.cache.invalidate(node.id());
        for child in node.child_blocks() {
            invalidated = self.invalidate_subtree(&child) || invalidated;
        }
        invalidated
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;

    use crate::measure::context::MeasureContext;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
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
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    #[test]
    fn second_measure_of_same_block_is_cache_hit() {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('H')),
            (Dot::new(1, 4), SeqItem::Char('i')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('Y')),
            (Dot::new(1, 6), SeqItem::Char('o')),
        ];
        let doc = logs(&items);
        let pd = project_document(&doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();
        let ctx = MeasureContext::default();

        let mut m = Measurer::new();
        let _ = m.measure(&root_node, 400.0, &ctx, &mut res);
        let a = m
            .cache
            .get(p1, 400.0)
            .expect("p1 cached after first measure")
            .clone();

        let _ = m.measure(&root_node, 400.0, &ctx, &mut res);
        let b = m
            .cache
            .get(p1, 400.0)
            .expect("p1 cached after second measure")
            .clone();

        assert!(
            Arc::ptr_eq(&a, &b),
            "second measure of an untouched block must reuse the cached Arc"
        );
    }
}

use super::layout_invalidation::LayoutInvalidationBatch;
use super::view_state::{NodeViewState, ViewStates};
use crate::diagnostics::{FrameDiagnostics, LayoutPassRecorder};
#[cfg(test)]
use crate::layout::LayoutNode;
use crate::layout::{LayoutCache, LayoutContext, Page, Paginator};
use crate::model::{
    CONTINUOUS_PAGE_MARGIN, Decorations, DefaultAttrs, Doc, DocumentSettings, NodeId,
};
use crate::types::{BoxConstraints, Size};
use std::cell::RefCell;
#[cfg(test)]
use std::rc::Rc;

pub(crate) struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
    width: f32,
    scale_factor: f64,
    pages: Vec<Page>,
    layout_cache: RefCell<LayoutCache>,
    view_states: ViewStates,
    diagnostics: FrameDiagnostics,
    layout_debug_enabled: bool,
}

impl LayoutEngine {
    pub(crate) fn new(width: f32, scale_factor: f64, diagnostics: FrameDiagnostics) -> Self {
        Self {
            viewport_width: width,
            viewport_height: 0.0,
            width,
            scale_factor,
            pages: Vec::new(),
            layout_cache: RefCell::new(LayoutCache::new()),
            view_states: ViewStates::default(),
            diagnostics,
            layout_debug_enabled: false,
        }
    }

    pub(crate) fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub(crate) fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    pub(crate) fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    pub(crate) fn width(&self) -> f32 {
        self.width
    }

    pub(crate) fn set_width(&mut self, width: f32) {
        self.width = width;
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub(crate) fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub(crate) fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub(crate) fn set_layout_debug_enabled(&mut self, enabled: bool) {
        self.layout_debug_enabled = enabled;
        if !enabled {
            self.diagnostics.clear_layout_pass();
        }
    }

    #[cfg(test)]
    pub(crate) fn is_layout_cached(&self, node_id: NodeId) -> bool {
        self.layout_cache.borrow().get(node_id).is_some()
    }

    #[cfg(test)]
    pub(crate) fn cached_layout(&self, node_id: NodeId) -> Option<Rc<LayoutNode>> {
        self.layout_cache.borrow().get(node_id)
    }

    pub(crate) fn set_fold_state(&mut self, node_id: NodeId, expanded: bool) {
        self.view_states
            .insert(node_id, NodeViewState::Fold { expanded });
    }

    pub(crate) fn fold_expanded(&self, node_id: NodeId) -> bool {
        self.view_states
            .get(&node_id)
            .map(|state| state.fold_expanded())
            .unwrap_or(false)
    }

    pub(crate) fn set_external_height(&mut self, node_id: NodeId, height: f32) {
        self.view_states
            .insert(node_id, NodeViewState::ExternalHeight { height });
    }

    pub(crate) fn external_height(&self, node_id: NodeId) -> Option<f32> {
        self.view_states
            .get(&node_id)
            .and_then(|state| state.external_height())
    }

    pub(crate) fn apply_invalidation(&mut self, doc: &Doc, batch: &LayoutInvalidationBatch) {
        if batch.is_empty() {
            return;
        }

        let mut cache = self.layout_cache.borrow_mut();

        if batch.is_full() {
            cache.invalidate_all();
            return;
        }

        for node_id in batch.subtree_and_ancestors_ids() {
            if let Some(node) = doc.node(node_id) {
                let descendants: Vec<_> = node.descendants().map(|n| n.node_id()).collect();
                cache.invalidate_with_descendants(node_id, descendants.into_iter());
                let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                for ancestor_id in ancestors {
                    cache.invalidate(ancestor_id);
                }
            } else {
                cache.invalidate(node_id);
            }
        }

        for node_id in batch.node_and_ancestors_ids() {
            if let Some(node) = doc.node(node_id) {
                let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                cache.invalidate_with_ancestors(node_id, ancestors.into_iter());
            } else {
                cache.invalidate(node_id);
            }
        }
    }

    pub(crate) fn recompute(
        &mut self,
        doc: &Doc,
        settings: &DocumentSettings,
        default_attrs: &DefaultAttrs,
        decorations: &Decorations,
    ) {
        let (page_width, page_height, margin_top, margin_bottom, margin_left, margin_right) =
            match settings.layout_mode {
                crate::model::LayoutMode::Paginated {
                    page_width,
                    page_height,
                    page_margin_top,
                    page_margin_bottom,
                    page_margin_left,
                    page_margin_right,
                } => (
                    page_width,
                    page_height,
                    page_margin_top,
                    page_margin_bottom,
                    page_margin_left,
                    page_margin_right,
                ),
                crate::model::LayoutMode::Continuous { max_width } => {
                    let page_margin = CONTINUOUS_PAGE_MARGIN;
                    let page_width = self.width.min(max_width + 2.0 * page_margin);
                    (
                        page_width,
                        f32::INFINITY,
                        page_margin,
                        page_margin,
                        page_margin,
                        page_margin,
                    )
                }
            };

        let constraints = BoxConstraints::loose(Size::new(
            page_width - margin_left - margin_right,
            page_height - margin_top - margin_bottom,
        ));

        let root_ref = doc.node(NodeId::ROOT).expect("root must exist");
        let trace = self
            .layout_debug_enabled
            .then(|| RefCell::new(LayoutPassRecorder::new()));
        let ctx = if let Some(trace_ref) = trace.as_ref() {
            LayoutContext::new_with_trace(
                &root_ref,
                settings,
                default_attrs,
                decorations,
                self.scale_factor,
                &self.view_states,
                &self.layout_cache,
                Some(trace_ref),
            )
        } else {
            LayoutContext::new(
                &root_ref,
                settings,
                default_attrs,
                decorations,
                self.scale_factor,
                &self.view_states,
                &self.layout_cache,
            )
        };

        let root_layout = {
            use crate::tracing::TRACER;
            use opentelemetry::trace::{Tracer, mark_span_as_active};
            let _s = mark_span_as_active(TRACER.start("layout.tree"));
            ctx.layout(&root_ref, constraints)
        };
        self.layout_cache.borrow_mut().clear_prev();

        let paginator = Paginator::new(
            page_width,
            page_height,
            margin_top,
            margin_bottom,
            margin_left,
            settings.layout_mode,
        );
        self.pages = {
            use crate::tracing::TRACER;
            use opentelemetry::trace::{Tracer, mark_span_as_active};
            let _s = mark_span_as_active(TRACER.start("layout.paginate"));
            paginator.paginate_rc(root_layout)
        };
        if let Some(trace) = trace {
            self.diagnostics.commit_layout_pass(trace.into_inner());
        } else {
            self.diagnostics.clear_layout_pass();
        }
    }
}

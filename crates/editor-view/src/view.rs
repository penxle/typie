use editor_common::{EdgeInsets, Movement};
use editor_model::{Doc, LayoutMode, NodeId};
use editor_resource::Resource;
use editor_state::{Position, Selection};
use editor_transaction::Step;
use std::sync::{Arc, Mutex};

use crate::measure::{MeasuredTree, Measurer};
use crate::page::{LayoutPage, PageRect};
use crate::paginate::{LayoutTree, Paginator};
use crate::query;
use crate::view_state::ViewState;
use crate::viewport::Viewport;

#[derive(Debug)]
pub struct View {
    measurer: Measurer,
    layout: Option<LayoutResult>,
    viewport: Viewport,
    view_state: ViewState,
}

#[derive(Debug)]
struct LayoutResult {
    tree: LayoutTree,
    pages: Vec<LayoutPage>,
}

impl View {
    pub fn new(viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            measurer: Measurer::new(resource),
            viewport,
            view_state: ViewState::new(),
            layout: None,
        }
    }

    pub fn reconcile(&mut self, doc: &Doc, steps: &[Step]) -> bool {
        if self.measurer.invalidate_with_steps(doc, steps) {
            self.compute(doc);
            true
        } else {
            false
        }
    }

    pub fn invalidate_nodes(&mut self, doc: &Doc, node_ids: &[NodeId]) -> bool {
        if node_ids.is_empty() {
            return false;
        }
        let mut invalidated = false;
        for &id in node_ids {
            if self.measurer.invalidate_with_ancestors(doc, id) {
                invalidated = true;
            }
        }
        if invalidated {
            self.compute(doc);
        }
        invalidated
    }

    pub fn layout(&mut self, doc: &Doc) {
        self.measurer.clear_cache();
        self.compute(doc);
    }

    fn compute(&mut self, doc: &Doc) {
        let paginator = match doc.attrs().layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => Paginator::paginated(
                page_width,
                page_height,
                EdgeInsets {
                    top: page_margin_top,
                    bottom: page_margin_bottom,
                    left: page_margin_left,
                    right: page_margin_right,
                },
            ),
            LayoutMode::Continuous { max_width } => Paginator::continuous(
                max_width.min(self.viewport.width),
                1024.0,
                EdgeInsets::all(20.0),
            ),
        };

        let content_width = paginator.content_width();

        let root = self
            .measurer
            .measure(doc, NodeId::ROOT, content_width, &self.view_state);
        let measured_tree = MeasuredTree {
            root: Arc::unwrap_or_clone(root),
        };

        let (tree, pages) = paginator.paginate(measured_tree);

        self.layout = Some(LayoutResult { tree, pages });
    }

    pub fn visit_page(&self, page_idx: usize, visitor: &mut impl query::PageVisitor) {
        if let Some(ref result) = self.layout
            && let Some(page) = result.pages.get(page_idx)
        {
            query::visit_page(&result.tree, page, visitor);
        }
    }

    pub fn hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        let page = result.pages.get(page_idx)?;
        query::exact_hit_test(&result.tree, page, x, y)
            .or_else(|| query::closest_hit_test(&result.tree, page, x, y))
    }

    pub fn resolve_movement(
        &self,
        pos: &Position,
        movement: &Movement,
        _doc: &Doc,
        resource: &Resource,
    ) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        query::resolve_movement(&result.tree, pos, movement, &self.viewport, resource)
    }

    pub fn cursor_rect(&self, pos: &Position) -> Option<PageRect> {
        let result = self.layout.as_ref()?;
        query::cursor_rect(&result.tree, &result.pages, pos)
    }

    pub fn pages(&self) -> &[LayoutPage] {
        self.layout.as_ref().map_or(&[], |r| &r.pages)
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    pub fn set_fold_state(&mut self, node_id: NodeId, expanded: bool) {
        self.view_state.fold_states.insert(node_id, expanded);
    }

    pub fn set_external_height(&mut self, node_id: NodeId, height: f32) {
        self.view_state.external_heights.insert(node_id, height);
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl View {
    pub fn new_test() -> Self {
        Self {
            measurer: Measurer::new_test(),
            viewport: Viewport::new(800.0, 600.0, 1.0),
            view_state: ViewState::new(),
            layout: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn layout_produces_pages() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(!view.pages().is_empty());
    }

    #[test]
    fn reconcile_returns_true_on_change() {
        let (doc, t1) = doc! { root { paragraph { t1: text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let steps = vec![Step::InsertText {
            node_id: t1,
            offset: 5,
            text: " world".into(),
        }];
        assert!(view.reconcile(&doc, &steps));
    }

    #[test]
    fn invalidate_nodes_returns_false_for_empty_list() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        assert!(!view.invalidate_nodes(&doc, &[]));
    }
}

use std::sync::{Arc, Mutex};

use editor_common::{Movement, Rect, TextSegmenters};
use editor_model::{Doc, NodeId};
use editor_resource::Resource;
use editor_state::{Position, Selection};
use editor_transaction::Step;

use crate::cursor;
use crate::engine::LayoutEngine;
use crate::view_state::ViewState;
use crate::viewport::Viewport;

#[derive(Debug)]
pub struct View {
    engine: LayoutEngine,
    viewport: Viewport,
    view_state: ViewState,
}

impl View {
    pub fn new(viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            engine: LayoutEngine::new(resource),
            viewport,
            view_state: ViewState::new(),
        }
    }

    pub fn reconcile(&mut self, doc: &Doc, steps: &[Step]) -> bool {
        if self.engine.invalidate_with_steps(doc, steps) {
            self.engine.compute(doc, &self.viewport, &self.view_state);
            true
        } else {
            false
        }
    }

    pub fn invalidate_nodes(&mut self, doc: &Doc, node_ids: &[NodeId]) -> bool {
        if node_ids.is_empty() {
            return false;
        }

        if node_ids
            .iter()
            .any(|&id| self.engine.invalidate_with_ancestors(doc, id))
        {
            self.engine.compute(doc, &self.viewport, &self.view_state);
            true
        } else {
            false
        }
    }

    pub fn layout(&mut self, doc: &Doc) {
        self.engine.cache.clear();
        self.engine.compute(doc, &self.viewport, &self.view_state);
    }

    pub fn hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection> {
        let page = self.engine.pages().get(page_idx)?;
        cursor::hit_test(page, x, y)
    }

    pub fn resolve_movement(
        &self,
        pos: &Position,
        movement: &Movement,
        segmenters: Option<&TextSegmenters>,
    ) -> Option<Selection> {
        cursor::resolve_movement(
            self.engine.pages(),
            pos,
            movement,
            &self.viewport,
            segmenters,
        )
    }

    pub fn cursor_rect(&self, pos: &Position) -> Option<(usize, Rect)> {
        cursor::cursor_rect(self.engine.pages(), pos)
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

    pub fn pages(&self) -> &[crate::Page] {
        self.engine.pages()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl View {
    pub fn new_test() -> Self {
        Self {
            engine: LayoutEngine::new_test(),
            viewport: Viewport::new(800.0, 600.0, 1.0),
            view_state: ViewState::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn invalidate_nodes_returns_false_for_empty_list() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        assert!(!view.invalidate_nodes(&doc, &[]));
    }

    #[test]
    fn invalidate_nodes_returns_true_for_nonempty_list() {
        let (doc, t1) = doc! { root { paragraph { t1: text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(view.invalidate_nodes(&doc, &[t1]));
    }
}

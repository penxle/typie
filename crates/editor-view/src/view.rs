use editor_common::{EdgeInsets, Movement};
use editor_model::{Doc, LayoutMode, Node, NodeId};
use editor_resource::Resource;
use editor_state::{Position, ResolvedPosition, ResolvedSelection, Selection};
use editor_transaction::Step;
use std::sync::{Arc, Mutex};

use crate::measure::text::resolve::resolve_text_style;
use crate::measure::text::strut::compute_strut;
use crate::measure::{MeasuredTree, Measurer};
use crate::page::LayoutPage;
use crate::paginate::{LayoutTree, Paginator};
use crate::query;
use crate::query::{CursorMetrics, SelectionRect};
use crate::view_state::{PendingStyle, ViewState};
use crate::viewport::Viewport;

#[derive(Debug)]
pub struct View {
    measurer: Measurer,
    layout: Option<LayoutResult>,
    fingerprint: Option<LayoutFingerprint>,
    viewport: Viewport,
    view_state: ViewState,
}

#[derive(Debug)]
struct LayoutResult {
    tree: LayoutTree,
    pages: Vec<LayoutPage>,
}

#[derive(Debug, Clone, PartialEq)]
struct LayoutFingerprint {
    layout_mode: LayoutMode,
    effective_viewport_width: f32,
}

impl View {
    pub fn new(viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            measurer: Measurer::new(resource),
            viewport,
            view_state: ViewState::new(),
            layout: None,
            fingerprint: None,
        }
    }

    pub fn reconcile(
        &mut self,
        old_doc: &Doc,
        new_doc: &Doc,
        steps: &[Step],
        new_pending_style: Option<PendingStyle>,
    ) -> bool {
        let nodes_invalidated = self.measurer.invalidate_with_steps(old_doc, new_doc, steps);
        let attrs_changed = steps.iter().any(|s| s.is_doc_attr_step());

        let pending_changed = self.view_state.pending_style != new_pending_style;
        if pending_changed {
            let old_node_id = self.view_state.pending_style.as_ref().map(|ps| ps.node_id);
            let new_node_id = new_pending_style.as_ref().map(|ps| ps.node_id);

            if let Some(id) = old_node_id {
                self.measurer.invalidate_with_ancestors(new_doc, id);
                if new_doc.node(id).is_none() {
                    self.measurer.invalidate_with_ancestors(old_doc, id);
                }
            }
            if let Some(id) = new_node_id
                && old_node_id != Some(id)
            {
                self.measurer.invalidate_with_ancestors(new_doc, id);
            }
        }

        let dirty = nodes_invalidated || attrs_changed || pending_changed;
        // IMPORTANT: assign pending_style before compute — compute reads view_state.pending_style.
        self.view_state.pending_style = new_pending_style;
        if dirty {
            self.compute(new_doc);
            self.view_state.preferred_x = None;
        }
        dirty
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
        self.view_state.pending_style = None;
        self.compute(doc);
        self.view_state.preferred_x = None;
    }

    fn build_paginator(&self, doc: &Doc) -> (Paginator, LayoutFingerprint) {
        let layout_mode = doc.attrs().layout_mode;
        let (paginator, effective_viewport_width) = match layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => (
                Paginator::paginated(
                    page_width,
                    page_height,
                    EdgeInsets {
                        top: page_margin_top,
                        bottom: page_margin_bottom,
                        left: page_margin_left,
                        right: page_margin_right,
                    },
                ),
                // Paginated layout is viewport-independent; 0.0 keeps the fingerprint
                // stable across resizes so self-heal treats them as no-ops.
                0.0,
            ),
            LayoutMode::Continuous { max_width } => {
                let effective = max_width.min(self.viewport.width);
                (
                    Paginator::continuous(effective, 1024.0, EdgeInsets::all(20.0)),
                    effective,
                )
            }
        };
        let fingerprint = LayoutFingerprint {
            layout_mode,
            effective_viewport_width,
        };
        (paginator, fingerprint)
    }

    fn compute(&mut self, doc: &Doc) {
        let (paginator, new_fingerprint) = self.build_paginator(doc);
        if self.fingerprint.as_ref() != Some(&new_fingerprint) {
            self.measurer.clear_cache();
            self.fingerprint = Some(new_fingerprint);
        }

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

    pub fn select_word_at(
        &self,
        pos: &ResolvedPosition<'_>,
        resource: &Resource,
    ) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        let segmenters = &resource.segmenters;
        query::segmentation::select_word_at(&result.tree, pos, segmenters)
    }

    pub fn select_paragraph_at(&self, pos: &Position) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        query::segmentation::select_paragraph_at(&result.tree, pos)
    }

    pub fn resolve_movement(
        &mut self,
        pos: &Position,
        movement: &Movement,
        resource: &Resource,
    ) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        let (selection, new_preferred_x) = query::resolve_movement(
            &result.tree,
            pos,
            movement,
            &self.viewport,
            resource,
            self.view_state.preferred_x,
        );
        self.view_state.preferred_x = new_preferred_x;
        selection
    }

    pub fn cursor_metrics(&self, doc: &Doc, pos: &Position) -> Option<CursorMetrics> {
        let result = self.layout.as_ref()?;
        let metrics_override = self.cursor_metrics_at(doc, pos);
        query::cursor_metrics(&result.tree, &result.pages, pos, metrics_override)
    }

    fn cursor_metrics_at(&self, doc: &Doc, pos: &Position) -> Option<(f32, f32)> {
        let node = doc.node(pos.node_id)?;
        if !matches!(node.node(), Node::Text(_)) {
            return None;
        }
        let style = resolve_text_style(&node);
        let mut resource = self.measurer.resource.lock().unwrap();
        let strut = compute_strut(&mut resource, &style)?;
        Some((strut.ascent, strut.descent))
    }

    pub fn selection_rects(&self, selection: &ResolvedSelection) -> Vec<SelectionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::selection::selection_rects(&result.tree, &result.pages, selection)
    }

    pub fn composition_rects(
        &self,
        from: &Position,
        to: &Position,
    ) -> Vec<query::composition::CompositionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::composition::composition_rects(&result.tree, &result.pages, from, to)
    }

    pub fn pages(&self) -> &[LayoutPage] {
        self.layout.as_ref().map_or(&[], |r| &r.pages)
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn resize(&mut self, viewport: Viewport, doc: &Doc) -> bool {
        let old_fingerprint = self.fingerprint.clone();
        self.viewport = viewport;
        self.compute(doc);
        let changed = self.fingerprint.as_ref() != old_fingerprint.as_ref();
        if changed {
            self.view_state.preferred_x = None;
        }
        changed
    }

    pub fn set_fold_state(&mut self, node_id: NodeId, expanded: bool) {
        self.view_state.fold_states.insert(node_id, expanded);
    }

    pub fn set_external_height(&mut self, node_id: NodeId, height: f32) {
        self.view_state.external_heights.insert(node_id, height);
    }

    pub fn clear_preferred_x(&mut self) {
        self.view_state.preferred_x = None;
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
            fingerprint: None,
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
        assert!(view.reconcile(&doc, &doc, &steps, None));
    }

    #[test]
    fn invalidate_nodes_returns_false_for_empty_list() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        assert!(!view.invalidate_nodes(&doc, &[]));
    }

    #[test]
    fn cursor_rect_matches_strut_ignoring_pending_when_empty() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut view = View::new_test();
        view.layout(&doc);

        let pos = Position::new(p1, 0);
        let default_rect = view.cursor_metrics(&doc, &pos).unwrap();

        // With no pending modifiers, cursor uses stored strut metrics.
        assert!(default_rect.caret.height > 0.0);
    }

    #[test]
    fn cursor_rect_matches_adjacent_text_font_size() {
        // Text node with its own FontSize modifier (24pt) alongside default text.
        // Cursor inside the big-sized text should reflect that text's style, not
        // the paragraph default.
        let (doc, t1, t2) = doc! {
            root {
                paragraph {
                    t1: text("hi")
                    t2: text("HI") [font_size(2400)]
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let r1 = view.cursor_metrics(&doc, &Position::new(t1, 0)).unwrap();
        let r2 = view.cursor_metrics(&doc, &Position::new(t2, 0)).unwrap();

        assert!(
            r2.caret.height > r1.caret.height,
            "cursor inside bigger-sized text should match the text's size \
             (r1.height={}, r2.height={})",
            r1.caret.height,
            r2.caret.height
        );
    }

    #[test]
    fn cursor_metrics_pending_grows_line_on_empty_paragraph() {
        use crate::view_state::PendingStyle;
        use editor_model::Modifier;
        use editor_state::PendingModifier;

        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut view = View::new_test();
        view.layout(&doc);
        let pos = Position::new(p1, 0);
        let baseline = view.cursor_metrics(&doc, &pos).unwrap();

        let pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        view.reconcile(&doc, &doc, &[], pending_style);
        let pending = view.cursor_metrics(&doc, &pos).unwrap();

        assert!(pending.caret.height > baseline.caret.height);
        assert!(pending.line.height > baseline.line.height);
        assert!(pending.line.height >= pending.caret.height);
    }

    #[test]
    fn cursor_metrics_pending_on_non_empty_paragraph_unchanged() {
        use crate::view_state::PendingStyle;
        use editor_model::Modifier;
        use editor_state::PendingModifier;

        let (doc, p1, t1) = doc! { root { p1: paragraph { t1: text("hi") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let pos = Position::new(t1, 0);
        let baseline = view.cursor_metrics(&doc, &pos).unwrap();

        let pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        view.reconcile(&doc, &doc, &[], pending_style);
        let after = view.cursor_metrics(&doc, &pos).unwrap();

        assert!((after.caret.height - baseline.caret.height).abs() < 0.01);
        assert!((after.line.height - baseline.line.height).abs() < 0.01);
    }

    #[test]
    fn page_width_change_triggers_reflow() {
        use editor_model::{DocumentAttrs, LayoutMode};
        use editor_transaction::Step;

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        let initial_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 400.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let doc = doc.with_attrs(initial_attrs.clone());
        view.layout(&doc);
        assert_eq!(view.pages()[0].size.width, 400.0);

        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 600.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let new_doc = doc.with_attrs(new_attrs.clone());
        let steps = vec![Step::SetDocumentAttrs {
            old: initial_attrs,
            new: new_attrs,
        }];
        let changed = view.reconcile(&doc, &new_doc, &steps, None);
        assert!(changed, "reconcile should return true for attrs change");
        assert_eq!(view.pages()[0].size.width, 600.0);
    }

    #[test]
    fn set_attrs_with_same_layout_mode_produces_same_layout() {
        use editor_model::{DocumentAttrs, LayoutMode};
        use editor_transaction::Step;

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 400.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let doc = doc.with_attrs(attrs.clone());
        let mut view = View::new_test();
        view.layout(&doc);

        let steps = vec![Step::SetDocumentAttrs {
            old: attrs.clone(),
            new: attrs,
        }];
        let changed = view.reconcile(&doc, &doc, &steps, None);
        assert!(changed, "attrs_changed branch returns true");
        assert_eq!(view.pages()[0].size.width, 400.0);
    }

    #[test]
    fn paginated_viewport_resize_is_noop() {
        use editor_model::{DocumentAttrs, LayoutMode};

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 400.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let doc = doc.with_attrs(attrs);
        let mut view = View::new_test();
        view.layout(&doc);

        let new_viewport = Viewport::new(1200.0, 800.0, 1.0);
        let changed = view.resize(new_viewport, &doc);
        assert!(
            !changed,
            "paginated mode must not reflow on viewport change"
        );
        assert_eq!(view.pages()[0].size.width, 400.0);
    }

    #[test]
    fn continuous_viewport_shrink_triggers_reflow() {
        use editor_model::{DocumentAttrs, LayoutMode};

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 800.0 },
        };
        let doc = doc.with_attrs(attrs);
        let mut view = View::new_test();
        view.layout(&doc);

        let new_viewport = Viewport::new(500.0, 600.0, 1.0);
        let changed = view.resize(new_viewport, &doc);
        assert!(
            changed,
            "continuous mode must reflow when effective width shrinks"
        );
    }

    #[test]
    fn continuous_viewport_growth_above_max_is_noop() {
        use editor_model::{DocumentAttrs, LayoutMode};

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 400.0 },
        };
        let doc = doc.with_attrs(attrs);
        let mut view = View::new_test();
        view.resize(Viewport::new(800.0, 600.0, 1.0), &doc);
        view.layout(&doc);

        let changed = view.resize(Viewport::new(2000.0, 600.0, 1.0), &doc);
        assert!(!changed, "growth above max_width must not reflow");
    }

    #[test]
    fn mode_switch_paginated_to_continuous_triggers_reflow() {
        use editor_model::{DocumentAttrs, LayoutMode};
        use editor_transaction::Step;

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let old_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 400.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let doc_old = doc.clone().with_attrs(old_attrs.clone());
        let mut view = View::new_test();
        view.layout(&doc_old);
        let old_page_width = view.pages()[0].size.width;

        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 600.0 },
        };
        let doc_new = doc.with_attrs(new_attrs.clone());
        let steps = vec![Step::SetDocumentAttrs {
            old: old_attrs,
            new: new_attrs,
        }];
        let changed = view.reconcile(&doc_old, &doc_new, &steps, None);
        assert!(changed);
        assert_ne!(view.pages()[0].size.width, old_page_width);
    }

    #[test]
    fn mode_switch_continuous_to_paginated_triggers_reflow() {
        use editor_model::{DocumentAttrs, LayoutMode};
        use editor_transaction::Step;

        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let old_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 500.0 },
        };
        let doc_old = doc.clone().with_attrs(old_attrs.clone());
        let mut view = View::new_test();
        view.layout(&doc_old);

        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 700.0,
                page_height: 900.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
        };
        let doc_new = doc.with_attrs(new_attrs.clone());
        let steps = vec![Step::SetDocumentAttrs {
            old: old_attrs,
            new: new_attrs,
        }];
        let changed = view.reconcile(&doc_old, &doc_new, &steps, None);
        assert!(changed);
        assert_eq!(view.pages()[0].size.width, 700.0);
    }

    #[test]
    fn layout_fingerprint_distinguishes_modes() {
        // Guards against a regression where the fingerprint is reduced to a scalar
        // (e.g. content_width). LayoutMode variant must remain part of the fingerprint
        // so mode switches always invalidate the cache, regardless of whether the
        // resulting numeric widths happen to coincide.
        let paginated_fp = LayoutFingerprint {
            layout_mode: LayoutMode::Paginated {
                page_width: 440.0,
                page_height: 600.0,
                page_margin_top: 20.0,
                page_margin_bottom: 20.0,
                page_margin_left: 20.0,
                page_margin_right: 20.0,
            },
            effective_viewport_width: 0.0,
        };
        let continuous_fp = LayoutFingerprint {
            layout_mode: LayoutMode::Continuous { max_width: 400.0 },
            // Match paginated's value so layout_mode is the only discriminator.
            // Realism of this synthetic value vs. what build_paginator would produce is irrelevant —
            // we are unit-testing the type's discrimination contract, not the producer.
            effective_viewport_width: 0.0,
        };
        assert_ne!(paginated_fp, continuous_fp);
    }
}

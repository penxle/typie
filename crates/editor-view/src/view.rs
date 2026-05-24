use editor_common::{EdgeInsets, Movement};
use editor_crdt::Op;
use editor_model::{Doc, DocOp, LayoutMode, Node, NodeId};
use editor_resource::Resource;
use editor_state::{Position, ResolvedSelection, Selection};
use std::sync::{Arc, Mutex};

use crate::ExternalElement;
use crate::measure::text::resolve::resolve_text_style;
use crate::measure::text::strut::compute_strut;
use crate::measure::{MeasuredTree, Measurer};
use crate::page::LayoutPage;
use crate::paginate::{LayoutTree, Paginator};
use crate::query;
use crate::query::{CursorMetrics, PointerStyle, SelectionEndpoints, SelectionRect};
use crate::view_state::{GapPhantom, PendingStyle, ViewState};
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

    pub fn reconcile_with_ops(
        &mut self,
        old_doc: &Doc,
        new_doc: &Doc,
        ops: &[Op<DocOp>],
        new_pending_style: Option<PendingStyle>,
        new_gap_phantom: Option<GapPhantom>,
    ) -> bool {
        let nodes_invalidated = self.measurer.invalidate_with_doc_ops(old_doc, new_doc, ops);
        let attrs_changed = ops.iter().any(
            |op| matches!(&op.payload, DocOp::Attr { node_id, .. } if *node_id == NodeId::ROOT),
        );

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

        let gap_changed = self.view_state.gap_phantom != new_gap_phantom;
        if gap_changed {
            for gp in [self.view_state.gap_phantom, new_gap_phantom]
                .into_iter()
                .flatten()
            {
                self.measurer.invalidate_with_ancestors(new_doc, gp.parent);
            }
        }

        let dirty = nodes_invalidated || attrs_changed || pending_changed || gap_changed;
        // IMPORTANT: assign pending_style before compute — compute reads view_state.pending_style.
        self.view_state.pending_style = new_pending_style;
        self.view_state.gap_phantom = new_gap_phantom;
        if dirty {
            self.compute(new_doc);
            if nodes_invalidated || attrs_changed || pending_changed {
                self.view_state.preferred_x = None;
            }
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
        self.view_state.gap_phantom = None;
        self.compute(doc);
        self.view_state.preferred_x = None;
    }

    fn build_paginator(&self, doc: &Doc) -> (Paginator, LayoutFingerprint) {
        let layout_mode = match &doc.get_entry(NodeId::ROOT).expect("root must exist").node {
            Node::Root(r) => *r.layout_mode.get(),
            _ => unreachable!("root entry must be Node::Root"),
        };
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
                    page_width as f32,
                    page_height as f32,
                    EdgeInsets {
                        top: page_margin_top as f32,
                        bottom: page_margin_bottom as f32,
                        left: page_margin_left as f32,
                        right: page_margin_right as f32,
                    },
                ),
                // Paginated layout is viewport-independent; 0.0 keeps the fingerprint
                // stable across resizes so self-heal treats them as no-ops.
                0.0,
            ),
            LayoutMode::Continuous { max_width } => {
                let effective = (max_width as f32).min(self.viewport.width);
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

    pub fn hit_test_extending(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        let page = result.pages.get(page_idx)?;
        query::exact_hit_test(&result.tree, page, x, y)
            .or_else(|| query::closest_hit_test_extending(&result.tree, page, x, y))
    }

    pub fn interactive_hit_test(
        &self,
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::query::InteractiveHit> {
        let result = self.layout.as_ref()?;
        let page = result.pages.get(page_idx)?;
        crate::query::interactive_hit_test(&result.tree, page, doc, x, y)
    }

    pub fn pointer_style_at(
        &self,
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
        read_only: bool,
    ) -> Option<PointerStyle> {
        let result = self.layout.as_ref()?;
        let page = result.pages.get(page_idx)?;
        Some(crate::query::pointer_style_at(
            &result.tree,
            page,
            doc,
            x,
            y,
            read_only,
        ))
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

    pub fn editable_position_inside(&self, node_id: NodeId, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        query::navigation::editable_position_inside(&result.tree, node_id, at_end)
    }

    pub fn is_at_edge_line_of(&self, node_id: NodeId, head: &Position, at_end: bool) -> bool {
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        query::navigation::is_at_edge_line_of(&result.tree, node_id, head, at_end)
    }

    pub fn ensure_preferred_x_at(&mut self, pos: &Position) {
        if self.view_state.preferred_x.is_some() {
            return;
        }
        let Some(result) = self.layout.as_ref() else {
            return;
        };
        self.view_state.preferred_x = query::navigation::compute_preferred_x_at(&result.tree, pos);
    }

    pub fn position_at_preferred_x_in(&self, node_id: NodeId, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        let x = self.view_state.preferred_x?;
        query::navigation::position_at_preferred_x_in(&result.tree, node_id, at_end, x)
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

    pub fn selection_endpoints(&self, selection: &ResolvedSelection) -> Option<SelectionEndpoints> {
        let result = self.layout.as_ref()?;
        query::selection::selection_endpoints(&result.tree, &result.pages, selection)
    }

    pub fn selection_hit_test(
        &self,
        selection: &ResolvedSelection,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(ref result) = self.layout else {
            return false;
        };
        query::selection::selection_hit_test(&result.tree, &result.pages, selection, page_idx, x, y)
    }

    pub fn node_box_rects(&self, ids: &[NodeId]) -> Vec<SelectionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::search::node_box_rects(&result.tree, &result.pages, ids)
    }

    pub fn nearest_node_box(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
        ids: &[NodeId],
    ) -> Option<NodeId> {
        let result = self.layout.as_ref()?;
        let page = result.pages.get(page_idx)?;
        query::search::nearest_node_box(&result.tree, page, x, y, ids)
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

    pub fn external_elements(
        &self,
        doc: &Doc,
        selection: Option<&Selection>,
    ) -> Vec<ExternalElement> {
        let Some(selection) = selection else {
            return Vec::new();
        };
        let Some(ref result) = self.layout else {
            return Vec::new();
        };
        crate::external::external_elements(&result.tree, &result.pages, doc, selection)
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

    pub fn set_external_height(&mut self, doc: &Doc, node_id: NodeId, height: f32) -> bool {
        if !height.is_finite() || height <= 0.0 || doc.node(node_id).is_none() {
            return false;
        }

        if self.view_state.external_height(node_id) == Some(height) {
            return false;
        }

        self.view_state.external_heights.insert(node_id, height);
        self.measurer.invalidate_with_ancestors(doc, node_id);
        self.compute(doc);
        self.view_state.preferred_x = None;
        true
    }

    pub fn fold_expanded(&self, node_id: NodeId) -> bool {
        self.view_state.fold_expanded(node_id)
    }

    pub fn toggle_fold(&mut self, doc: &Doc, node_id: NodeId) -> bool {
        let Some(node_ref) = doc.node(node_id) else {
            return false;
        };
        if !matches!(node_ref.node(), Node::Fold(_)) {
            return false;
        }
        let expanded = self.view_state.fold_expanded(node_id);
        self.view_state.fold_states.insert(node_id, !expanded);
        // fold-title's measured chevron/border embeds the parent fold's expanded
        // state; the measure cache is node-id-keyed and invalidate_with_ancestors
        // only walks upward, so the fold-title child needs explicit invalidation
        // or it stays stale.
        for child in node_ref.children() {
            if matches!(child.node(), Node::FoldTitle(_)) {
                self.measurer.invalidate_with_ancestors(doc, child.id());
            }
        }
        self.measurer.invalidate_with_ancestors(doc, node_id);
        self.compute(doc);
        self.view_state.preferred_x = None;
        true
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

    pub fn layout_tree_for_test(&self) -> Option<&crate::paginate::LayoutTree> {
        self.layout.as_ref().map(|r| &r.tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::Direction;
    use editor_macros::{doc, state};

    fn make_op(id: editor_crdt::Dot, payload: DocOp) -> Op<DocOp> {
        Op {
            id,
            parents: Default::default(),
            payload,
        }
    }

    #[test]
    fn layout_produces_pages() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(!view.pages().is_empty());
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
    fn cursor_on_small_text_in_mixed_font_line_aligns_to_baseline() {
        let (doc, small, big) = doc! {
            root {
                paragraph {
                    small: text("a")
                    big: text("A") [font_size(4800)]
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let small_caret = view
            .cursor_metrics(&doc, &Position::new(small, 0))
            .unwrap()
            .caret;
        let big_caret = view
            .cursor_metrics(&doc, &Position::new(big, 0))
            .unwrap()
            .caret;

        assert!(
            big_caret.height > small_caret.height,
            "big caret height {} should exceed small caret height {}",
            big_caret.height,
            small_caret.height,
        );
        let small_bottom = small_caret.y + small_caret.height;
        let big_bottom = big_caret.y + big_caret.height;
        assert!(
            (small_bottom - big_bottom).abs() < big_caret.height * 0.25,
            "small caret bottom {small_bottom} should be baseline-aligned with big caret \
             bottom {big_bottom}",
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
        view.reconcile_with_ops(&doc, &doc, &[], pending_style, None);
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
        view.reconcile_with_ops(&doc, &doc, &[], pending_style, None);
        let after = view.cursor_metrics(&doc, &pos).unwrap();

        assert!((after.caret.height - baseline.caret.height).abs() < 0.01);
        assert!((after.line.height - baseline.line.height).abs() < 0.01);
    }

    #[test]
    fn page_width_change_triggers_reflow() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, RootNodeAttr};

        let (doc,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 600,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        assert_eq!(view.pages()[0].size.width, 400.0);

        let (new_doc,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 600,
                    page_height: 600,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: NodeId::ROOT,
                op: NodeAttr::Root {
                    attr: RootNodeAttr::LayoutMode(LayoutMode::Paginated {
                        page_width: 600,
                        page_height: 600,
                        page_margin_top: 20,
                        page_margin_bottom: 20,
                        page_margin_left: 20,
                        page_margin_right: 20,
                    }),
                },
            },
        )];
        let changed = view.reconcile_with_ops(&doc, &new_doc, &ops, None, None);
        assert!(
            changed,
            "reconcile_with_ops should return true for root attr change"
        );
        assert_eq!(view.pages()[0].size.width, 600.0);
    }

    #[test]
    fn set_attrs_with_same_layout_mode_produces_same_layout() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, RootNodeAttr};

        let (doc,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 600,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: NodeId::ROOT,
                op: NodeAttr::Root {
                    attr: RootNodeAttr::LayoutMode(LayoutMode::Paginated {
                        page_width: 400,
                        page_height: 600,
                        page_margin_top: 20,
                        page_margin_bottom: 20,
                        page_margin_left: 20,
                        page_margin_right: 20,
                    }),
                },
            },
        )];
        let changed = view.reconcile_with_ops(&doc, &doc, &ops, None, None);
        assert!(changed, "attrs_changed branch returns true");
        assert_eq!(view.pages()[0].size.width, 400.0);
    }

    #[test]
    fn paginated_viewport_resize_is_noop() {
        let (doc,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 600,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
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
        let (doc,) = doc! {
            root (layout_mode: LayoutMode::Continuous { max_width: 800 }) {
                paragraph { text("hello") }
            }
        };
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
        let (doc,) = doc! {
            root (layout_mode: LayoutMode::Continuous { max_width: 400 }) {
                paragraph { text("hello") }
            }
        };
        let mut view = View::new_test();
        view.resize(Viewport::new(800.0, 600.0, 1.0), &doc);
        view.layout(&doc);

        let changed = view.resize(Viewport::new(2000.0, 600.0, 1.0), &doc);
        assert!(!changed, "growth above max_width must not reflow");
    }

    #[test]
    fn mode_switch_paginated_to_continuous_triggers_reflow() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, RootNodeAttr};

        let (doc_old,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 600,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc_old);
        let old_page_width = view.pages()[0].size.width;

        let (doc_new,) = doc! {
            root (layout_mode: LayoutMode::Continuous { max_width: 600 }) {
                paragraph { text("hello") }
            }
        };
        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: NodeId::ROOT,
                op: NodeAttr::Root {
                    attr: RootNodeAttr::LayoutMode(LayoutMode::Continuous { max_width: 600 }),
                },
            },
        )];
        let changed = view.reconcile_with_ops(&doc_old, &doc_new, &ops, None, None);
        assert!(changed);
        assert_ne!(view.pages()[0].size.width, old_page_width);
    }

    #[test]
    fn mode_switch_continuous_to_paginated_triggers_reflow() {
        use editor_crdt::Dot;
        use editor_model::{DocOp, NodeAttr, RootNodeAttr};

        let (doc_old,) = doc! {
            root (layout_mode: LayoutMode::Continuous { max_width: 500 }) {
                paragraph { text("hello") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc_old);

        let (doc_new,) = doc! {
            root (
                layout_mode: LayoutMode::Paginated {
                    page_width: 700,
                    page_height: 900,
                    page_margin_top: 20,
                    page_margin_bottom: 20,
                    page_margin_left: 20,
                    page_margin_right: 20,
                }
            ) {
                paragraph { text("hello") }
            }
        };
        let ops = vec![make_op(
            Dot::new(1, 0),
            DocOp::Attr {
                node_id: NodeId::ROOT,
                op: NodeAttr::Root {
                    attr: RootNodeAttr::LayoutMode(LayoutMode::Paginated {
                        page_width: 700,
                        page_height: 900,
                        page_margin_top: 20,
                        page_margin_bottom: 20,
                        page_margin_left: 20,
                        page_margin_right: 20,
                    }),
                },
            },
        )];
        let changed = view.reconcile_with_ops(&doc_old, &doc_new, &ops, None, None);
        assert!(changed);
        assert_eq!(view.pages()[0].size.width, 700.0);
    }

    #[test]
    fn reconcile_with_ops_invalidates_view() {
        use editor_crdt::{Dot, TextOp};
        use editor_model::DocOp;

        let mut view = View::new_test();
        let (doc_old, _p, t) = doc! {
            root { p: paragraph { t: text("hi") } }
        };
        view.layout(&doc_old);

        let mut new_plain = doc_old.to_plain();
        if let Some(entry) = new_plain.nodes.get_mut(&t)
            && let editor_model::PlainNode::Text(tn) = &mut entry.node
        {
            tn.text = "hello".into();
        }
        let (doc_new, _) = Doc::from_plain(new_plain);

        let op = Op {
            id: Dot::new(0, 0),
            parents: Default::default(),
            payload: DocOp::Text {
                node_id: t,
                op: TextOp::InsertChar {
                    ch: 'x',
                    after: None,
                },
            },
        };
        let dirty = view.reconcile_with_ops(&doc_old, &doc_new, &[op], None, None);
        assert!(dirty);
    }

    #[test]
    fn layout_fingerprint_distinguishes_modes() {
        // Guards against a regression where the fingerprint is reduced to a scalar
        // (e.g. content_width). LayoutMode variant must remain part of the fingerprint
        // so mode switches always invalidate the cache, regardless of whether the
        // resulting numeric widths happen to coincide.
        let paginated_fp = LayoutFingerprint {
            layout_mode: LayoutMode::Paginated {
                page_width: 440,
                page_height: 600,
                page_margin_top: 20,
                page_margin_bottom: 20,
                page_margin_left: 20,
                page_margin_right: 20,
            },
            effective_viewport_width: 0.0,
        };
        let continuous_fp = LayoutFingerprint {
            layout_mode: LayoutMode::Continuous { max_width: 400 },
            // Match paginated's value so layout_mode is the only discriminator.
            // Realism of this synthetic value vs. what build_paginator would produce is irrelevant —
            // we are unit-testing the type's discrimination contract, not the producer.
            effective_viewport_width: 0.0,
        };
        assert_ne!(paginated_fp, continuous_fp);
    }

    #[test]
    fn view_node_box_rects_and_nearest_for_table() {
        use editor_macros::doc;
        let (d, c00, c11) = doc! {
            root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } }
        };
        let mut view = View::new_test();
        view.layout(&d);

        let rects = view.node_box_rects(&[c00, c11]);
        assert_eq!(rects.len(), 2);
        assert!(
            rects
                .iter()
                .all(|r| r.rect.width > 0.0 && r.rect.height > 0.0)
        );

        let c11_rect = view.node_box_rects(&[c11])[0].rect;
        let cx = c11_rect.x + c11_rect.width / 2.0;
        let cy = c11_rect.y + c11_rect.height / 2.0;
        assert_eq!(view.nearest_node_box(0, cx, cy, &[c00, c11]), Some(c11));

        assert!(view.node_box_rects(&[]).is_empty());
        assert_eq!(view.nearest_node_box(0, cx, cy, &[]), None);
    }

    #[test]
    fn arrow_right_onto_image_selects_image_then_passes() {
        let (state, t1, t2) = state! {
            doc { root {
                paragraph { t1: text("ab") }
                image
                paragraph { t2: text("cd") }
            } }
            selection: (t1, 2)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);
        let root = editor_model::NodeId::ROOT;

        // First →: cursor is at end of "ab" (before the image), so movement lands on the
        // image and produces a node-selection spanning it rather than passing through.
        let sel1 = view
            .resolve_movement(
                &Position::new(t1, 2),
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();
        assert!(
            !sel1.is_collapsed(),
            "first → must select image, got {:?}",
            sel1
        );
        assert_eq!(
            sel1.anchor,
            Position {
                node_id: root,
                offset: 1,
                affinity: editor_state::Affinity::Downstream
            }
        );
        assert_eq!(
            sel1.head,
            Position {
                node_id: root,
                offset: 2,
                affinity: editor_state::Affinity::Upstream
            }
        );

        // Second →: cursor is at the trailing edge of the image node-selection, so movement
        // passes through and lands at the start of the following paragraph's text.
        let sel2 = view
            .resolve_movement(
                &sel1.head,
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();
        assert!(
            sel2.is_collapsed(),
            "second → must pass image, got {:?}",
            sel2
        );
        assert_eq!(sel2.head.node_id, t2);
        assert_eq!(sel2.head.offset, 0);
    }

    #[test]
    fn arrow_left_onto_horizontal_rule_selects_it() {
        let (state, t2) = state! {
            doc { root {
                paragraph { text("ab") }
                horizontal_rule
                paragraph { t2: text("cd") }
            } }
            selection: (t2, 0)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);
        let root = editor_model::NodeId::ROOT;

        let sel = view
            .resolve_movement(
                &Position::new(t2, 0),
                &Movement::Grapheme {
                    direction: Direction::Backward,
                },
                &Resource::new_test(),
            )
            .unwrap();
        assert!(
            !sel.is_collapsed(),
            "← onto hr must node-select, got {:?}",
            sel
        );
        // Backward direction: anchor is at the trailing edge (offset 2, Upstream),
        // head is at the leading edge (offset 1, Downstream).
        assert_eq!(
            sel.anchor,
            Position {
                node_id: root,
                offset: 2,
                affinity: editor_state::Affinity::Upstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node_id: root,
                offset: 1,
                affinity: editor_state::Affinity::Downstream
            }
        );
    }

    #[test]
    fn view_selection_endpoints_single_line_uses_first_and_last_rect_edges() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let only = &rects[0];

        let endpoints = view.selection_endpoints(&resolved).unwrap();
        assert_eq!(endpoints.from.page_idx, only.page_idx);
        assert_eq!(endpoints.from.rect.x, only.rect.x);
        assert_eq!(endpoints.from.rect.y, only.rect.y);
        assert_eq!(endpoints.from.rect.width, 0.0);
        assert_eq!(endpoints.from.rect.height, only.rect.height);

        assert_eq!(endpoints.to.page_idx, only.page_idx);
        assert_eq!(endpoints.to.rect.x, only.rect.x + only.rect.width);
        assert_eq!(endpoints.to.rect.y, only.rect.y);
        assert_eq!(endpoints.to.rect.width, 0.0);
        assert_eq!(endpoints.to.rect.height, only.rect.height);
    }

    #[test]
    fn view_selection_endpoints_anchor_after_head_still_uses_doc_order() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);

        let forward = Selection::new(Position::new(t, 1), Position::new(t, 4))
            .resolve(&doc)
            .unwrap();
        let reverse = Selection::new(Position::new(t, 4), Position::new(t, 1))
            .resolve(&doc)
            .unwrap();

        let a = view.selection_endpoints(&forward).unwrap();
        let b = view.selection_endpoints(&reverse).unwrap();
        assert_eq!(a.from.rect.x, b.from.rect.x);
        assert_eq!(a.to.rect.x, b.to.rect.x);
    }

    #[test]
    fn view_selection_endpoints_multi_line_uses_first_and_last_only() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let first = &rects[0];
        let last = rects.last().unwrap();

        let endpoints = view.selection_endpoints(&resolved).unwrap();
        assert_eq!(endpoints.from.rect.x, first.rect.x);
        assert_eq!(endpoints.from.rect.y, first.rect.y);
        assert_eq!(endpoints.to.rect.x, last.rect.x + last.rect.width);
        assert_eq!(endpoints.to.rect.y, last.rect.y);
    }

    #[test]
    fn view_selection_endpoints_atom_uses_atom_left_and_right_edges() {
        let (doc,) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 1),
            Position::new(NodeId::ROOT, 2),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let atom = &rects[0];
        assert_eq!(atom.meta, crate::query::SelectionRectKind::Atom);

        let endpoints = view.selection_endpoints(&resolved).unwrap();
        assert_eq!(endpoints.from.rect.x, atom.rect.x);
        assert_eq!(endpoints.to.rect.x, atom.rect.x + atom.rect.width);
        assert_eq!(endpoints.from.rect.height, atom.rect.height);
        assert_eq!(endpoints.to.rect.height, atom.rect.height);
    }

    #[test]
    fn view_selection_endpoints_block_uses_block_left_and_right_edges() {
        let (doc,) = doc! {
            root {
                callout(variant: editor_model::CalloutVariant::Danger) {
                    paragraph { text("hi") }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 0),
            Position::new(NodeId::ROOT, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let block = &rects[0];
        assert_eq!(block.meta, crate::query::SelectionRectKind::Block);

        let endpoints = view.selection_endpoints(&resolved).unwrap();
        assert_eq!(endpoints.from.rect.x, block.rect.x);
        assert_eq!(endpoints.to.rect.x, block.rect.x + block.rect.width);
    }

    #[test]
    fn view_selection_endpoints_multi_page_carries_per_page_idx() {
        let (doc,) = doc! {
            root (
                layout_mode: editor_model::LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 120,
                    page_margin_top: 10,
                    page_margin_bottom: 10,
                    page_margin_left: 10,
                    page_margin_right: 10,
                }
            ) {
                fold {
                    fold_title { text("title") }
                    fold_content {
                        paragraph { text("a") }
                        paragraph { text("b") }
                        paragraph { text("c") }
                        paragraph { text("d") }
                        paragraph { text("e") }
                        paragraph { text("f") }
                        paragraph { text("g") }
                        paragraph { text("h") }
                    }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(view.pages().len() >= 2);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 0),
            Position::new(NodeId::ROOT, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let first = &rects[0];
        let last = rects.last().unwrap();
        assert_ne!(first.page_idx, last.page_idx);

        let endpoints = view.selection_endpoints(&resolved).unwrap();
        assert_eq!(endpoints.from.page_idx, first.page_idx);
        assert_eq!(endpoints.to.page_idx, last.page_idx);
    }

    #[test]
    fn view_selection_endpoints_collapsed_returns_none() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let resolved = Selection::collapsed(Position::new(t, 2))
            .resolve(&doc)
            .unwrap();
        assert!(view.selection_endpoints(&resolved).is_none());
    }

    #[test]
    fn view_selection_hit_test_envelope_band() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hi") }
                paragraph { t2: text("a much longer line") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let resolved = Selection::new(Position::new(t1, 0), Position::new(t2, 18))
            .resolve(&doc)
            .unwrap();

        let rects = view.selection_rects(&resolved);
        let first = rects[0].rect;
        let last = rects[1].rect;
        let max_x = last.x + last.width;

        let probe_x = first.x + first.width + 5.0;
        let probe_y = first.y + first.height * 0.5;
        assert!(probe_x < max_x);
        assert!(view.selection_hit_test(&resolved, 0, probe_x, probe_y));
        assert!(!view.selection_hit_test(&resolved, 0, max_x + 10.0, probe_y));
    }
}

#[cfg(test)]
mod interactive_tests {
    use super::*;
    use crate::paginate::{LayoutContent, LayoutNode};
    use crate::query::InteractiveHit;
    use crate::style::DecorationData;
    use editor_macros::doc;

    fn find_box(node: &LayoutNode, target: NodeId) -> Option<&LayoutNode> {
        if let LayoutContent::Box(b) = &node.content {
            if b.node_id == target {
                return Some(node);
            }
            for c in &b.children {
                if let Some(found) = find_box(c, target) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn fold_title_bool(node: &LayoutNode, fold_title: NodeId) -> bool {
        let b = match &find_box(node, fold_title).unwrap().content {
            LayoutContent::Box(b) => b,
            _ => unreachable!(),
        };
        match b.style.decorations.iter().find(|d| d.id == 0).unwrap().data {
            DecorationData::Bool(v) => v,
            _ => panic!("fold-title decoration must be Bool(expanded)"),
        }
    }

    #[test]
    fn toggle_fold_flips_relayouts_and_refreshes_chevron() {
        let (doc, f1, ft1) = doc! {
            root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let tree = view.layout_tree_for_test().unwrap();
        let expanded_h = find_box(&tree.root, f1).unwrap().rect.height;
        assert!(fold_title_bool(&tree.root, ft1), "starts expanded");

        assert!(view.toggle_fold(&doc, f1));
        let tree = view.layout_tree_for_test().unwrap();
        let collapsed_h = find_box(&tree.root, f1).unwrap().rect.height;
        assert!(collapsed_h < expanded_h, "collapsed shorter");
        assert!(
            !fold_title_bool(&tree.root, ft1),
            "chevron Bool must refresh to collapsed (not stale)"
        );

        assert!(view.toggle_fold(&doc, f1));
        let tree = view.layout_tree_for_test().unwrap();
        assert_eq!(find_box(&tree.root, f1).unwrap().rect.height, expanded_h);
        assert!(fold_title_bool(&tree.root, ft1), "chevron back to expanded");
    }

    #[test]
    fn toggle_fold_rejects_non_fold_node() {
        let (doc, ft1) = doc! {
            root { fold { ft1: fold_title { text("T") } fold_content { paragraph } } }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(!view.toggle_fold(&doc, ft1), "non-Fold id rejected");
    }

    #[test]
    fn interactive_hit_test_finds_fold_title() {
        let (doc, f1, ft1) = doc! {
            root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Content") } }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let tb = find_box(&tree.root, ft1).unwrap();
        // View::new_test continuous layout → single page (y_start=0), so the
        // absolute tree coords double as page-local input here.
        let hit = view.interactive_hit_test(&doc, 0, tb.rect.x + 4.0, tb.rect.y + 4.0);
        assert!(
            matches!(hit, Some(InteractiveHit::FoldTitle { id, .. }) if id == f1),
            "got {hit:?}"
        );
    }

    #[test]
    fn gap_phantom_change_triggers_recompute_without_ops() {
        let (doc, ..) = doc! {
            root {
                fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                paragraph {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let changed = view.reconcile_with_ops(
            &doc,
            &doc,
            &[],
            None,
            Some(GapPhantom {
                parent: editor_model::NodeId::ROOT,
                index: 1,
            }),
        );
        assert!(
            changed,
            "gap_phantom change must trigger recompute even with no doc ops"
        );
    }
}

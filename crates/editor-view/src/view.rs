use std::sync::{Arc, Mutex, OnceLock};

use editor_common::{EdgeInsets, Movement};
use editor_crdt::Dot;
use editor_model::{LayoutMode, Node};
use editor_resource::Resource;
use editor_state::{Position, ResolvedSelection, Selection, State};

use crate::measure::context::measure_context;
use crate::measure::nodes::dispatch::measure_node;
use crate::measure::types::MeasuredTree;
use crate::page::LayoutPage;
use crate::page_fragment::{PageFragmentTree, build_page_fragment_tree};
use crate::paginate::paginator::Paginator;
use crate::query::cursor::CursorMetrics;
use crate::query::layout_index::LayoutIndex;
use crate::view_state::{GapPhantom, GroupDecoration, PendingStyle, ViewState};
use crate::viewport::Viewport;

const CONTINUOUS_MARGIN_X: f32 = 20.0;

pub struct View {
    resource: Arc<Mutex<Resource>>,
    layout: Option<LayoutResult>,
    fingerprint: Option<LayoutFingerprint>,
    viewport: Viewport,
    view_state: ViewState,
}

struct LayoutResult {
    pages: Vec<LayoutPage>,
    page_fragments: Vec<OnceLock<PageFragmentTree>>,
    content_width: f32,
    layout_index: LayoutIndex,
}

#[derive(Debug, Clone, PartialEq)]
struct LayoutFingerprint {
    layout_mode: LayoutMode,
    effective_viewport_width: f32,
}

impl View {
    pub fn new(viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            resource,
            viewport,
            view_state: ViewState::new(),
            layout: None,
            fingerprint: None,
        }
    }

    pub fn layout(&mut self, state: &State) {
        self.view_state.pending_style = None;
        self.view_state.gap_phantom = None;
        self.compute(state);
        self.view_state.preferred_x = None;
    }

    pub fn reconcile(
        &mut self,
        state: &State,
        new_pending_style: Option<PendingStyle>,
        new_gap_phantom: Option<GapPhantom>,
    ) -> bool {
        let pending_changed = self.view_state.pending_style != new_pending_style;
        let gap_changed = self.view_state.gap_phantom != new_gap_phantom;
        self.view_state.pending_style = new_pending_style;
        self.view_state.gap_phantom = new_gap_phantom;
        let old_fingerprint = self.fingerprint.clone();
        self.compute(state);
        if pending_changed {
            self.view_state.preferred_x = None;
        }
        // The eg-walker projection re-measures the whole tree every reconcile (the
        // incremental measurer cache is not yet wired), so this can only report the
        // change sources it can see directly. Doc-content changes are signalled to
        // the caller separately via the applied-op set.
        pending_changed || gap_changed || self.fingerprint != old_fingerprint
    }

    pub fn invalidate(&mut self, state: &State) -> bool {
        self.compute(state);
        true
    }

    fn doc_layout_mode(state: &State) -> LayoutMode {
        match state.view().root().map(|r| r.node()) {
            Some(Node::Root(r)) => *r.layout_mode.get(),
            _ => LayoutMode::default(),
        }
    }

    fn build_pipeline(&self, state: &State) -> (Paginator, f32, LayoutFingerprint) {
        let layout_mode = Self::doc_layout_mode(state);
        match layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => {
                let margins = EdgeInsets {
                    top: page_margin_top as f32,
                    bottom: page_margin_bottom as f32,
                    left: page_margin_left as f32,
                    right: page_margin_right as f32,
                };
                let content_width = page_width as f32 - margins.left - margins.right;
                let paginator =
                    Paginator::paginated(page_width as f32, page_height as f32, margins);
                (
                    paginator,
                    content_width,
                    LayoutFingerprint {
                        layout_mode,
                        effective_viewport_width: 0.0,
                    },
                )
            }
            LayoutMode::Continuous { max_width } => {
                let avail_content = (self.viewport.width - 2.0 * CONTINUOUS_MARGIN_X).max(0.0);
                let content_width = (max_width as f32).min(avail_content);
                let page_width = content_width + 2.0 * CONTINUOUS_MARGIN_X;
                let paginator = Paginator::continuous(
                    page_width,
                    100_000.0,
                    EdgeInsets::all(CONTINUOUS_MARGIN_X),
                );
                (
                    paginator,
                    content_width,
                    LayoutFingerprint {
                        layout_mode,
                        effective_viewport_width: content_width,
                    },
                )
            }
        }
    }

    fn compute(&mut self, state: &State) {
        let (paginator, content_width, new_fingerprint) = self.build_pipeline(state);
        self.fingerprint = Some(new_fingerprint);

        let view = state.view();
        let Some(root) = view.root() else {
            self.layout = None;
            return;
        };
        let ctx = measure_context(&self.view_state);
        let measured = {
            let mut resource = self.resource.lock().unwrap();
            measure_node(&root, content_width, &ctx, &mut resource)
        };
        let paginated = paginator.paginate(MeasuredTree { root: measured });
        let pages = paginated.pages;
        let page_fragments = (0..pages.len()).map(|_| OnceLock::new()).collect();
        let layout_index = LayoutIndex::new(paginated.tree, &pages);
        self.layout = Some(LayoutResult {
            pages,
            page_fragments,
            content_width,
            layout_index,
        });
    }

    fn fragment_for_page(&self, page_idx: usize) -> Option<&PageFragmentTree> {
        let result = self.layout.as_ref()?;
        let cell = result.page_fragments.get(page_idx)?;
        let page = result.pages.get(page_idx)?;
        Some(
            cell.get_or_init(|| {
                build_page_fragment_tree(result.layout_index.tree(), page_idx, page)
            }),
        )
    }

    pub fn visit_page(&self, page_idx: usize, visitor: &mut impl crate::query::visit::PageVisitor) {
        if let Some(fragment) = self.fragment_for_page(page_idx) {
            crate::query::visit::visit_page(fragment, visitor);
        }
    }

    pub fn hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection> {
        let layout_index = &self.layout.as_ref()?.layout_index;
        crate::query::hit_test::hit_test(layout_index, page_idx, x, y)
    }

    pub fn hit_test_extending(
        &self,
        state: &State,
        anchor: &Position,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<Selection> {
        let layout_index = &self.layout.as_ref()?.layout_index;
        crate::query::hit_test::hit_test_extending(
            layout_index,
            &state.view(),
            anchor,
            page_idx,
            x,
            y,
        )
    }

    pub fn drop_target_at(
        &self,
        state: &State,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::dnd::DropTarget> {
        let layout_index = &self.layout.as_ref()?.layout_index;
        crate::query::dnd::drop_target_at(layout_index, &state.view(), page_idx, x, y)
    }

    pub fn interactive_hit_test(
        &self,
        state: &State,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::query::interactive::InteractiveHit> {
        let layout_index = &self.layout.as_ref()?.layout_index;
        crate::query::interactive::interactive_hit_test(layout_index, &state.view(), page_idx, x, y)
    }

    pub fn page_link_rects(&self, page_idx: usize) -> Vec<crate::query::link::LinkRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::query::link::page_link_rects(&result.layout_index, page_idx)
    }

    pub fn link_rects(&self) -> Vec<crate::query::link::LinkRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for idx in 0..result.pages.len() {
            out.extend(crate::query::link::page_link_rects(
                &result.layout_index,
                idx,
            ));
        }
        out
    }

    pub fn link_hit_test(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::query::link::LinkRect> {
        let result = self.layout.as_ref()?;
        crate::query::link::link_hit_test(&result.layout_index, page_idx, x, y)
    }

    pub fn pointer_style_at(
        &self,
        state: &State,
        page_idx: usize,
        x: f32,
        y: f32,
        read_only: bool,
    ) -> Option<crate::query::pointer_style::PointerStyle> {
        let result = self.layout.as_ref()?;
        Some(crate::query::pointer_style::pointer_style_at(
            &result.layout_index,
            &state.view(),
            page_idx,
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
        let (selection, new_preferred_x) = crate::query::navigation::resolve_movement(
            &result.layout_index,
            pos,
            movement,
            &self.viewport,
            resource,
            self.view_state.preferred_x,
        );
        self.view_state.preferred_x = new_preferred_x;
        selection
    }

    pub fn editable_position_inside(&self, node: Dot, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        crate::query::navigation::editable_position_inside(&result.layout_index, &node, at_end)
    }

    pub fn is_at_edge_line_of(&self, node: Dot, head: &Position, at_end: bool) -> bool {
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        crate::query::navigation::is_at_edge_line_of(&result.layout_index, &node, head, at_end)
    }

    pub fn ensure_preferred_x_at(&mut self, pos: &Position) {
        if self.view_state.preferred_x.is_some() {
            return;
        }
        if let Some(result) = self.layout.as_ref() {
            self.view_state.preferred_x =
                crate::query::navigation::compute_preferred_x_at(&result.layout_index, pos);
        }
    }

    pub fn position_at_preferred_x_in(&self, node: Dot, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        let x = self.view_state.preferred_x?;
        crate::query::navigation::position_at_preferred_x_in(&result.layout_index, &node, at_end, x)
    }

    pub fn cursor_metrics(&self, _state: &State, pos: &Position) -> Option<CursorMetrics> {
        let result = self.layout.as_ref()?;
        crate::query::cursor::cursor_metrics(&result.layout_index, pos, None)
    }

    pub fn placeholder_metrics(
        &self,
        state: &State,
    ) -> Option<crate::query::placeholder::PlaceholderMetrics> {
        let result = self.layout.as_ref()?;
        crate::query::placeholder::placeholder_metrics(&result.layout_index, &state.view())
    }

    pub fn selection_rects(
        &self,
        selection: &ResolvedSelection,
    ) -> Vec<crate::query::selection::SelectionRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::query::selection::selection_rects(&result.layout_index, selection)
    }

    pub fn selection_text_rects(
        &self,
        selection: &ResolvedSelection,
    ) -> Vec<crate::query::selection::SelectionRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::query::selection::selection_text_rects(&result.layout_index, selection)
    }

    pub fn selection_endpoints(
        &self,
        selection: &ResolvedSelection,
    ) -> Option<crate::query::selection::SelectionEndpoints> {
        let result = self.layout.as_ref()?;
        crate::query::selection::selection_endpoints(&result.layout_index, selection)
    }

    pub fn selection_hit_test(
        &self,
        selection: &ResolvedSelection,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        crate::query::selection::selection_hit_test(&result.layout_index, selection, page_idx, x, y)
    }

    pub fn node_box_rects(&self, ids: &[Dot]) -> Vec<crate::query::selection::SelectionRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::query::selection::block_selection_rects(&result.layout_index, ids)
    }

    pub fn nearest_node_box(&self, page_idx: usize, x: f32, y: f32, ids: &[Dot]) -> Option<Dot> {
        let result = self.layout.as_ref()?;
        let point = result.layout_index.point(page_idx, x, y)?;
        result.layout_index.nearest_box(point, ids)
    }

    pub fn node_box_contains(&self, page_idx: usize, x: f32, y: f32, id: Dot) -> bool {
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        let Some(point) = result.layout_index.point(page_idx, x, y) else {
            return false;
        };
        result.layout_index.box_contains(point, &id)
    }

    pub fn composition_rects(
        &self,
        from: &Position,
        to: &Position,
    ) -> Vec<crate::query::composition::CompositionRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::query::composition::composition_rects(&result.layout_index, from, to)
    }

    pub fn pages(&self) -> &[LayoutPage] {
        self.layout.as_ref().map_or(&[], |r| &r.pages)
    }

    pub fn external_elements(
        &self,
        state: &State,
        selection: Option<&ResolvedSelection>,
    ) -> Vec<crate::external::ExternalElement> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::external::external_elements(&result.layout_index, &state.view(), selection)
    }

    pub fn page_external_elements(
        &self,
        state: &State,
        page_idx: usize,
        selection: Option<&ResolvedSelection>,
    ) -> Vec<crate::external::ExternalElement> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        crate::external::page_external_elements(
            &result.layout_index,
            &state.view(),
            page_idx,
            selection,
        )
    }

    pub fn page_table_overlays(
        &self,
        state: &State,
        page_idx: usize,
        selection: Option<&ResolvedSelection>,
    ) -> Vec<crate::table_overlay::TableOverlay> {
        let Some(content_width) = self.layout.as_ref().map(|r| r.content_width) else {
            return Vec::new();
        };
        let Some(fragment) = self.fragment_for_page(page_idx) else {
            return Vec::new();
        };
        crate::table_overlay::page_table_overlays(fragment, &state.view(), selection, content_width)
    }

    pub fn table_overlays(
        &self,
        state: &State,
        selection: Option<&ResolvedSelection>,
    ) -> Vec<crate::table_overlay::TableOverlay> {
        let page_count = self.layout.as_ref().map(|r| r.pages.len()).unwrap_or(0);
        let mut overlays = Vec::new();
        for page_idx in 0..page_count {
            overlays.extend(self.page_table_overlays(state, page_idx, selection));
        }
        overlays
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn resize(&mut self, viewport: Viewport, state: &State) -> bool {
        let old_fingerprint = self.fingerprint.clone();
        self.viewport = viewport;
        self.compute(state);
        let changed = self.fingerprint.as_ref() != old_fingerprint.as_ref();
        if changed {
            self.view_state.preferred_x = None;
        }
        changed
    }

    pub fn set_fold_state(&mut self, node: Dot, expanded: bool) {
        self.view_state.fold_states.insert(node, expanded);
    }

    pub fn set_external_height(&mut self, state: &State, node: Dot, height: f32) -> bool {
        if !height.is_finite()
            || height <= 0.0
            || (state.view().node(node).is_none() && state.view().leaf(node).is_none())
        {
            return false;
        }
        if self.view_state.external_height(node) == Some(height) {
            return false;
        }
        self.view_state.external_heights.insert(node, height);
        self.compute(state);
        self.view_state.preferred_x = None;
        true
    }

    pub fn fold_expanded(&self, node: Dot) -> bool {
        self.view_state.fold_expanded(node)
    }

    pub fn toggle_fold(&mut self, state: &State, node: Dot) -> bool {
        {
            let view = state.view();
            let Some(node_view) = view.node(node) else {
                return false;
            };
            if !matches!(node_view.node(), Node::Fold(_)) {
                return false;
            }
        }
        let expanded = self.view_state.fold_expanded(node);
        self.view_state.fold_states.insert(node, !expanded);
        self.compute(state);
        self.view_state.preferred_x = None;
        true
    }

    pub fn clear_preferred_x(&mut self) {
        self.view_state.preferred_x = None;
    }

    pub fn set_group_decoration(&mut self, group: String, decoration: GroupDecoration) {
        self.view_state
            .tracked_decoration_groups
            .insert(group, decoration);
    }

    pub fn remove_group_decoration(&mut self, group: &str) -> bool {
        self.view_state
            .tracked_decoration_groups
            .remove(group)
            .is_some()
    }

    pub fn would_set_group_decoration(&self, group: &str, decoration: &GroupDecoration) -> bool {
        self.view_state.group_decoration(group) != Some(decoration)
    }

    pub fn would_remove_group_decoration(&self, group: &str) -> bool {
        self.view_state.group_decoration(group).is_some()
    }

    pub fn preferred_x(&self) -> Option<f32> {
        self.view_state.preferred_x
    }

    pub fn view_state(&self) -> &ViewState {
        &self.view_state
    }

    pub fn would_resize(&self, viewport: Viewport, _state: &State) -> bool {
        self.viewport != viewport
    }

    pub fn would_set_external_height(&self, state: &State, node: Dot, height: f32) -> bool {
        if !height.is_finite()
            || height <= 0.0
            || (state.view().node(node).is_none() && state.view().leaf(node).is_none())
        {
            return false;
        }
        self.view_state.external_height(node) != Some(height)
    }

    pub fn would_toggle_fold(&self, state: &State, id: Dot) -> bool {
        state
            .view()
            .node(id)
            .is_some_and(|n| matches!(n.node(), Node::Fold(_)))
    }

    pub fn would_clear_preferred_x(&self) -> bool {
        self.view_state.preferred_x.is_some()
    }

    pub fn would_ensure_preferred_x_at(&self, pos: &Position) -> bool {
        if self.view_state.preferred_x.is_some() {
            return false;
        }
        self.layout.as_ref().is_some_and(|result| {
            crate::query::navigation::compute_preferred_x_at(&result.layout_index, pos).is_some()
        })
    }

    pub fn would_resolve_movement(
        &self,
        pos: &Position,
        movement: &Movement,
        resource: &Resource,
    ) -> Option<(Option<Selection>, Option<f32>)> {
        let result = self.layout.as_ref()?;
        Some(crate::query::navigation::resolve_movement(
            &result.layout_index,
            pos,
            movement,
            &self.viewport,
            resource,
            self.view_state.preferred_x,
        ))
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl View {
    pub fn new_test() -> Self {
        Self::new(
            Viewport::new(800.0, 600.0, 1.0),
            Arc::new(Mutex::new(Resource::new_test())),
        )
    }
}

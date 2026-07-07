use std::sync::{Arc, Mutex, OnceLock};

use editor_common::{EdgeInsets, Movement};
use editor_crdt::Dot;
use editor_model::{LayoutMode, Node, NodeType, NodeView};
use editor_resource::Resource;
use editor_state::{LayoutDirty, Position, ResolvedSelection, Selection, State};

use crate::measure::Measurer;
use crate::measure::context::measure_context;
use crate::measure::types::MeasuredTree;
use crate::page::LayoutPage;
use crate::page_fragment::{PageFragmentTree, build_page_fragment_tree};
use crate::paginate::paginator::Paginator;
use crate::query::cursor::CursorMetrics;
use crate::query::layout_index::LayoutIndex;
use crate::view_state::{GapPhantom, GroupDecoration, PendingOverlay, ViewState};
use crate::viewport::Viewport;

const CONTINUOUS_MARGIN_X: f32 = 20.0;
const CONTINUOUS_CONTENT_CAP: f32 = 1024.0;

pub struct View {
    resource: Arc<Mutex<Resource>>,
    layout: Option<LayoutResult>,
    fingerprint: Option<LayoutFingerprint>,
    viewport: Viewport,
    view_state: ViewState,
    measurer: Measurer,
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
            measurer: Measurer::new(),
        }
    }

    pub fn layout(&mut self, state: &State) {
        self.view_state.pending_overlay = None;
        self.view_state.gap_phantom = None;
        self.measurer.clear();
        self.compute(state);
        self.view_state.preferred_x = None;
    }

    pub fn reconcile(
        &mut self,
        state: &State,
        dirty: LayoutDirty,
        new_pending_overlay: Option<PendingOverlay>,
        new_gap_phantom: Option<GapPhantom>,
    ) -> bool {
        let pending_changed = self.view_state.pending_overlay != new_pending_overlay;
        let gap_changed = self.view_state.gap_phantom != new_gap_phantom;

        let mut dirty = dirty;
        if pending_changed {
            for id in [
                self.view_state.pending_overlay.as_ref().map(|p| p.node_id),
                new_pending_overlay.as_ref().map(|p| p.node_id),
            ]
            .into_iter()
            .flatten()
            {
                dirty.mark_content(id);
            }
        }
        if gap_changed {
            for gp in [self.view_state.gap_phantom, new_gap_phantom]
                .into_iter()
                .flatten()
            {
                dirty.mark_content(gp.parent);
            }
        }

        self.view_state.pending_overlay = new_pending_overlay;
        self.view_state.gap_phantom = new_gap_phantom;

        let dirty_empty = matches!(
            &dirty,
            LayoutDirty::Incremental { content, structural }
                if content.is_empty() && structural.is_empty()
        );
        if dirty_empty && self.layout.is_some() {
            let (_, _, new_fingerprint) = self.build_pipeline(state);
            if self.fingerprint.as_ref() == Some(&new_fingerprint) {
                return false;
            }
        }

        let old_fingerprint = self.fingerprint.clone();

        let view = state.view();
        let mut content_targets: Option<Vec<editor_crdt::Dot>> = None;
        match &dirty {
            LayoutDirty::Full => self.measurer.clear(),
            LayoutDirty::Incremental {
                content,
                structural,
            } => {
                let mut targets = structural.is_empty().then(Vec::new);
                for id in content.iter().chain(structural.iter()) {
                    let Some(node) = view
                        .node(*id)
                        .or_else(|| view.leaf(*id).and_then(|l| l.parent()))
                    else {
                        continue;
                    };
                    let table = if node.node_type() == NodeType::Table {
                        Some(node)
                    } else {
                        node.ancestors().find(|a| a.node_type() == NodeType::Table)
                    };
                    let target = table.as_ref().unwrap_or(&node);
                    self.measurer.invalidate_subtree(target);
                    self.measurer.invalidate_with_ancestors(target);
                    if let Some(t) = targets.as_mut() {
                        t.push(target.id());
                    }
                }
                content_targets = targets;
            }
        }

        self.compute_inner(state, content_targets.as_deref());

        if pending_changed {
            self.view_state.preferred_x = None;
        }

        pending_changed || gap_changed || self.fingerprint != old_fingerprint
    }

    pub fn invalidate(&mut self, state: &State) -> bool {
        self.compute(state);
        true
    }

    pub fn invalidate_measure_with_ancestors(&mut self, node: &NodeView) -> bool {
        self.measurer
            .invalidate_measure_and_segments_with_ancestors(node)
    }

    pub fn clear_measure_cache(&mut self) {
        self.measurer.clear();
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
                    CONTINUOUS_CONTENT_CAP,
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
        self.compute_inner(state, None);
    }

    fn compute_inner(&mut self, state: &State, content_targets: Option<&[editor_crdt::Dot]>) {
        let old_fingerprint = self.fingerprint.clone();
        let (paginator, content_width, new_fingerprint) = self.build_pipeline(state);
        let fingerprint_unchanged = old_fingerprint.as_ref() == Some(&new_fingerprint);
        if !fingerprint_unchanged {
            self.measurer.clear();
        }
        self.fingerprint = Some(new_fingerprint);

        let view = state.view();
        let Some(root) = view.root() else {
            self.layout = None;
            return;
        };
        let ctx = measure_context(&self.view_state);
        let measured = {
            let mut resource = self.resource.lock().unwrap();
            let root_arc = self
                .measurer
                .measure(&root, content_width, &ctx, &mut resource);
            Arc::unwrap_or_clone(root_arc)
        };
        let paginated = paginator.paginate(MeasuredTree { root: measured });
        let pages = paginated.pages;
        let prev = self.layout.take();

        // Content-only edit whose blocks kept their exact geometry: every
        // index structure (entries, node maps, per-page lists, R-tree) is a
        // pure function of geometry/ids, so reuse it under the new tree
        // instead of rebuilding it O(document). Verification is O(edited
        // blocks): unchanged sibling geometry follows from unchanged block
        // heights (pagination is deterministic in its geometric inputs).
        let reusable = if let (true, Some(prev_layout), Some(targets)) =
            (fingerprint_unchanged, &prev, content_targets)
        {
            prev_layout.pages == pages
                && !targets.is_empty()
                && targets.iter().all(|d| {
                    prev_layout
                        .layout_index
                        .subtree_geometry_matches(&paginated.tree, d)
                })
        } else {
            false
        };
        let layout_index = if reusable {
            prev.expect("checked above")
                .layout_index
                .rebind_tree(paginated.tree)
        } else {
            LayoutIndex::new(paginated.tree, &pages)
        };

        let page_fragments = (0..pages.len()).map(|_| OnceLock::new()).collect();
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
        crate::query::placeholder::placeholder_metrics(
            &result.layout_index,
            &state.view(),
            self.view_state.pending_overlay.as_ref(),
        )
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

    /// Per-page maximum ("backing") size for incremental rendering. The CPU
    /// buffer and canvas are allocated at this fixed size so a content-height
    /// change never resizes (and thus clears) the surface. Continuous pages cap
    /// at the content cap plus vertical margins — but never below an oversized
    /// page that a single unbreakable block forced taller; paginated pages have
    /// a fixed height that already equals their backing size.
    pub fn page_backing_sizes(&self) -> Vec<editor_common::Size> {
        let cap = match self.fingerprint.as_ref().map(|f| &f.layout_mode) {
            Some(LayoutMode::Continuous { .. }) => {
                Some(CONTINUOUS_CONTENT_CAP + 2.0 * CONTINUOUS_MARGIN_X)
            }
            _ => None,
        };
        self.pages()
            .iter()
            .map(|p| {
                editor_common::Size::new(
                    p.size.width,
                    match cap {
                        Some(c) => p.size.height.max(c),
                        None => p.size.height,
                    },
                )
            })
            .collect()
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

    pub fn set_fold_state(&mut self, state: &State, node: Dot, expanded: bool) {
        self.view_state.fold_states.insert(node, expanded);
        self.evict_measure_for(state, node);
    }

    fn evict_measure_for(&mut self, state: &State, node: Dot) {
        let view = state.view();
        let Some(nv) = view
            .node(node)
            .or_else(|| view.leaf(node).and_then(|l| l.parent()))
        else {
            return;
        };
        self.measurer.invalidate_subtree(&nv);
        self.measurer.invalidate_with_ancestors(&nv);
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
        self.evict_measure_for(state, node);
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
        self.evict_measure_for(state, node);
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

#[cfg(test)]
mod invalidation_tests {
    use std::sync::{Arc, Mutex};

    use editor_crdt::{Dot, ListOp};
    use editor_model::{
        CalloutNodeAttr, CalloutVariant, EditOp, Modifier, ModifierAttrOp, NodeAttr, NodeAttrOp,
        NodeType, SeqItem, TableNodeAttr,
    };
    use editor_resource::Resource;
    use editor_state::{LayoutDirty, ProjectedState, State};

    use super::View;
    use crate::measure::context::measure_context;
    use crate::measure::types::MeasuredNode;
    use crate::viewport::Viewport;

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block {
                node_type,
                parents,
                attrs: vec![],
            },
        })
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    fn make_view(width: f32) -> View {
        View::new(
            Viewport::new(width, 800.0, 1.0),
            Arc::new(Mutex::new(Resource::new_test())),
        )
    }

    fn cached_arc(view: &mut View, state: &State, node: Dot) -> Arc<MeasuredNode> {
        let (_, content_width, _) = view.build_pipeline(state);
        let dv = state.view();
        let nv = dv.node(node).expect("block node present");
        let ctx = measure_context(&view.view_state);
        let mut resource = view.resource.lock().unwrap();
        view.measurer
            .measure(&nv, content_width, &ctx, &mut resource)
    }

    #[test]
    fn text_insert_marks_owning_block_content() {
        let mut ps = ProjectedState::empty();
        ps.commit();
        let para = ps
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let _ = ps.take_layout_dirty();

        ps.apply(seq_char(1, 'x')).unwrap();

        match ps.take_layout_dirty() {
            LayoutDirty::Incremental { content, .. } => {
                assert!(content.contains(&para));
                assert!(
                    !content.contains(&Dot::ROOT),
                    "char insert must not propagate to ROOT"
                );
            }
            LayoutDirty::Full => panic!("char insert must not force Full"),
        }
    }

    #[test]
    fn attr_change_marks_node_content_dirty() {
        let mut ps = ProjectedState::empty();
        let root = Dot::ROOT;
        let callout = ps
            .apply(seq_block(1, NodeType::Callout, vec![root]))
            .unwrap()
            .id;
        ps.apply(seq_block(2, NodeType::Paragraph, vec![root, callout]))
            .unwrap();
        ps.commit();
        let _ = ps.take_layout_dirty();

        ps.apply(EditOp::NodeAttr(NodeAttrOp {
            target: callout,
            attr: NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
            },
        }))
        .unwrap();

        match ps.take_layout_dirty() {
            LayoutDirty::Incremental { content, .. } => {
                assert!(content.contains(&callout));
            }
            LayoutDirty::Full => panic!("NodeAttr must not force Full"),
        }
    }

    #[test]
    fn modifier_marks_target_preserves_sibling() {
        {
            let mut ps = ProjectedState::empty();
            let root = Dot::ROOT;
            let p1 = ps
                .apply(seq_block(1, NodeType::Paragraph, vec![root]))
                .unwrap()
                .id;
            let sibling = ps
                .apply(seq_block(2, NodeType::Paragraph, vec![root]))
                .unwrap()
                .id;
            ps.commit();
            let _ = ps.take_layout_dirty();

            ps.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: p1,
                modifier: Modifier::Bold,
            }))
            .unwrap();

            match ps.take_layout_dirty() {
                LayoutDirty::Incremental { content, .. } => {
                    assert!(
                        content.contains(&p1),
                        "modified paragraph must be content-dirty"
                    );
                    assert!(
                        !content.contains(&sibling),
                        "unrelated sibling must not be content-dirty"
                    );
                }
                LayoutDirty::Full => panic!("BlockModifier must not force Full"),
            }
        }

        {
            let mut ps = ProjectedState::empty();
            let root = Dot::ROOT;
            let p1 = ps
                .apply(seq_block(1, NodeType::Paragraph, vec![root]))
                .unwrap()
                .id;
            let sibling = ps
                .apply(seq_block(2, NodeType::Paragraph, vec![root]))
                .unwrap()
                .id;
            ps.commit();
            let _ = ps.take_layout_dirty();

            let state_pre = State::new(ps.clone(), None);
            let mut view = make_view(800.0);
            view.layout(&state_pre);
            let before = cached_arc(&mut view, &state_pre, sibling);

            ps.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: p1,
                modifier: Modifier::Bold,
            }))
            .unwrap();
            let dirty = ps.take_layout_dirty();

            let state_post = State::new(ps, None);
            view.reconcile(&state_post, dirty, None, None);
            let after = cached_arc(&mut view, &state_post, sibling);

            assert!(
                Arc::ptr_eq(&before, &after),
                "sibling cache must survive modifier on the target block"
            );
        }
    }

    #[test]
    fn modifier_on_root_marks_all_descendants_content_dirty() {
        let mut ps = ProjectedState::empty();
        let root = Dot::ROOT;
        let p = ps
            .apply(seq_block(1, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        ps.apply(seq_char(2, 'h')).unwrap();
        ps.commit();
        let _ = ps.take_layout_dirty();

        ps.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
            target: Dot::ROOT,
            modifier: Modifier::FontSize { value: 2400 },
        }))
        .unwrap();

        match ps.take_layout_dirty() {
            LayoutDirty::Incremental { content, .. } => {
                assert!(content.contains(&Dot::ROOT), "ROOT must be content-dirty");
                assert!(
                    content.contains(&p),
                    "descendant paragraph must be content-dirty"
                );
            }
            LayoutDirty::Full => panic!("BlockModifier on ROOT must not force Full"),
        }
    }

    #[test]
    fn table_proportion_marks_table_and_cell_descendants() {
        let mut ps = ProjectedState::empty();
        let root = Dot::ROOT;
        let table = ps
            .apply(seq_block(1, NodeType::Table, vec![root]))
            .unwrap()
            .id;
        let row = ps
            .apply(seq_block(2, NodeType::TableRow, vec![root, table]))
            .unwrap()
            .id;
        let cell = ps
            .apply(seq_block(3, NodeType::TableCell, vec![root, table, row]))
            .unwrap()
            .id;
        let para = ps
            .apply(seq_block(
                4,
                NodeType::Paragraph,
                vec![root, table, row, cell],
            ))
            .unwrap()
            .id;
        ps.apply(seq_char(5, 'A')).unwrap();
        ps.commit();
        let _ = ps.take_layout_dirty();

        ps.apply(EditOp::NodeAttr(NodeAttrOp {
            target: table,
            attr: NodeAttr::Table {
                attr: TableNodeAttr::Proportion(50),
            },
        }))
        .unwrap();

        match ps.take_layout_dirty() {
            LayoutDirty::Incremental { content, .. } => {
                assert!(content.contains(&table), "table must be content-dirty");
                assert!(
                    content.contains(&cell),
                    "cell must be content-dirty: proportion changes its measured width"
                );
                assert!(
                    content.contains(&para),
                    "cell's paragraph descendant must be content-dirty"
                );
            }
            LayoutDirty::Full => panic!("NodeAttr must not force Full"),
        }
    }

    #[test]
    fn inherited_modifier_dirties_empty_descendant_paragraph() {
        {
            let mut ps = ProjectedState::empty();
            ps.commit();
            let empty_para = ps
                .view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .dot()
                .unwrap();
            let _ = ps.take_layout_dirty();

            ps.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: Dot::ROOT,
                modifier: Modifier::FontSize { value: 2400 },
            }))
            .unwrap();

            match ps.take_layout_dirty() {
                LayoutDirty::Incremental { content, .. } => {
                    assert!(
                        content.contains(&empty_para),
                        "empty paragraph must be content-dirty when root modifier changes"
                    );
                }
                LayoutDirty::Full => panic!("BlockModifier on ROOT must not force Full"),
            }
        }

        {
            let mut ps = ProjectedState::empty();
            ps.commit();
            let empty_para = ps
                .view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .dot()
                .unwrap();
            let _ = ps.take_layout_dirty();

            let state_pre = State::new(ps.clone(), None);
            let mut view = make_view(800.0);
            view.layout(&state_pre);
            let before = cached_arc(&mut view, &state_pre, empty_para);

            ps.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: Dot::ROOT,
                modifier: Modifier::FontSize { value: 2400 },
            }))
            .unwrap();
            let dirty = ps.take_layout_dirty();

            let state_post = State::new(ps, None);
            view.reconcile(&state_post, dirty, None, None);
            let after = cached_arc(&mut view, &state_post, empty_para);

            assert!(
                !Arc::ptr_eq(&before, &after),
                "empty paragraph cache must be evicted and re-measured after root font-size change"
            );
        }
    }
}

#[cfg(test)]
mod incremental_tests {
    use std::sync::{Arc, Mutex};

    use editor_crdt::{Dot, ListOp, OpGraph};
    use editor_model::{AtomLeaf, ChildView, EditOp, Node, NodeType, SeqItem};
    use editor_resource::Resource;
    use editor_state::{ProjectedState, State};

    use super::View;
    use crate::measure::context::measure_context;
    use crate::measure::types::MeasuredNode;
    use crate::viewport::Viewport;

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block {
                node_type,
                parents,
                attrs: vec![],
            },
        })
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    fn seq_image(pos: usize, parents: Vec<Dot>) -> EditOp {
        let node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::BlockAtom {
                leaf: AtomLeaf::Image { node },
                parents,
            },
        })
    }

    fn make_view(width: f32) -> View {
        View::new(
            Viewport::new(width, 800.0, 1.0),
            Arc::new(Mutex::new(Resource::new_test())),
        )
    }

    fn cached_arc(view: &mut View, state: &State, node: Dot) -> Arc<MeasuredNode> {
        let (_, content_width, _) = view.build_pipeline(state);
        let dv = state.view();
        let nv = dv.node(node).expect("block node present");
        let ctx = measure_context(&view.view_state);
        let mut resource = view.resource.lock().unwrap();
        view.measurer
            .measure(&nv, content_width, &ctx, &mut resource)
    }

    fn all_block_dots(state: &State) -> Vec<Dot> {
        let view = state.view();
        let mut out = Vec::new();
        if let Some(root) = view.root() {
            for d in root.descendants() {
                if let ChildView::Block(b) = d {
                    out.push(b.id());
                }
            }
        }
        out
    }

    fn page_sig(view: &View) -> Vec<(f32, f32, f32, f32)> {
        view.pages()
            .iter()
            .map(|p| (p.y_start, p.y_end, p.content_y_start, p.content_y_end))
            .collect()
    }

    #[test]
    fn reconcile_one_char_matches_full_layout() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let mut pos = 0;
        for _ in 0..3 {
            g.add_mut(seq_block(pos, NodeType::Paragraph, vec![root]))
                .unwrap();
            pos += 1;
            for ch in "mmmmmmmmmmmm".chars() {
                g.add_mut(seq_char(pos, ch)).unwrap();
                pos += 1;
            }
        }
        g.commit_mut();
        let base = ProjectedState::from_graph(g).unwrap();

        let width = 50.0;
        let pre = State::new(base.clone(), None);
        let mut view = make_view(width);
        view.layout(&pre);

        let mut ed = base;
        let _ = ed.take_layout_dirty();
        ed.apply(seq_char(1, 'X')).unwrap();
        let dirty = ed.take_layout_dirty();
        let post = State::new(ed, None);

        view.reconcile(&post, dirty, None, None);

        let mut fresh = make_view(width);
        fresh.layout(&post);

        let ids = all_block_dots(&post);
        assert!(!ids.is_empty());
        assert_eq!(view.node_box_rects(&ids), fresh.node_box_rects(&ids));
        assert_eq!(page_sig(&view), page_sig(&fresh));
    }

    #[test]
    fn reconcile_content_edit_reuses_index_when_geometry_unchanged() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let mut pos = 0;
        for _ in 0..3 {
            g.add_mut(seq_block(pos, NodeType::Paragraph, vec![root]))
                .unwrap();
            pos += 1;
            for ch in "mmmmmmmmmmmm".chars() {
                g.add_mut(seq_char(pos, ch)).unwrap();
                pos += 1;
            }
        }
        g.commit_mut();
        let base = ProjectedState::from_graph(g).unwrap();

        // Wide enough that one extra char never wraps: block geometry is
        // unchanged and the index-reuse fast path must engage.
        let width = 800.0;
        let pre = State::new(base.clone(), None);
        let mut view = make_view(width);
        view.layout(&pre);

        // Force the lazy R-tree so reuse (which keeps it) is observable.
        let _ = view.hit_test(0, 1.0, 1.0);
        assert!(
            view.layout
                .as_ref()
                .is_some_and(|l| l.layout_index.rtree_built()),
            "hit_test must have built the R-tree"
        );

        let mut ed = base;
        let _ = ed.take_layout_dirty();
        ed.apply(seq_char(1, 'X')).unwrap();
        let dirty = ed.take_layout_dirty();
        assert!(matches!(
            &dirty,
            editor_state::LayoutDirty::Incremental { structural, .. } if structural.is_empty()
        ));
        let post = State::new(ed, None);

        view.reconcile(&post, dirty, None, None);

        // Reuse keeps the already-built R-tree; a rebuild would reset it.
        assert!(
            view.layout
                .as_ref()
                .is_some_and(|l| l.layout_index.rtree_built()),
            "geometry-unchanged content edit must reuse the layout index"
        );

        let mut fresh = make_view(width);
        fresh.layout(&post);
        let ids = all_block_dots(&post);
        assert!(!ids.is_empty());
        assert_eq!(view.node_box_rects(&ids), fresh.node_box_rects(&ids));
        assert_eq!(page_sig(&view), page_sig(&fresh));
        for (x, y) in [(1.0, 1.0), (60.0, 10.0), (120.0, 40.0)] {
            assert_eq!(view.hit_test(0, x, y), fresh.hit_test(0, x, y));
        }
    }

    #[test]
    fn fold_toggle_remeasures_only_the_fold_subtree() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let fold = g
            .add_mut(seq_block(0, NodeType::Fold, vec![root]))
            .unwrap()
            .id;
        g.add_mut(seq_block(1, NodeType::FoldTitle, vec![root, fold]))
            .unwrap();
        g.add_mut(seq_char(2, 'T')).unwrap();
        let fc = g
            .add_mut(seq_block(3, NodeType::FoldContent, vec![root, fold]))
            .unwrap()
            .id;
        g.add_mut(seq_block(4, NodeType::Paragraph, vec![root, fold, fc]))
            .unwrap();
        g.add_mut(seq_char(5, 'C')).unwrap();
        let unrelated = g
            .add_mut(seq_block(6, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        g.commit_mut();
        let state = State::new(ProjectedState::from_graph(g).unwrap(), None);

        let mut view = make_view(800.0);
        view.layout(&state);

        let before_fold = cached_arc(&mut view, &state, fold);
        let before_unrelated = cached_arc(&mut view, &state, unrelated);

        assert!(view.toggle_fold(&state, fold));

        let after_fold = cached_arc(&mut view, &state, fold);
        let after_unrelated = cached_arc(&mut view, &state, unrelated);

        assert!(
            after_fold.height < before_fold.height,
            "collapsing the fold must shrink its measured height: {} -> {}",
            before_fold.height,
            after_fold.height
        );
        assert!(
            Arc::ptr_eq(&before_unrelated, &after_unrelated),
            "sibling paragraph must stay cached (cache hit, no re-measure)"
        );
    }

    #[test]
    fn external_height_resize_remeasures_leaf_owner() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let fold = g
            .add_mut(seq_block(0, NodeType::Fold, vec![root]))
            .unwrap()
            .id;
        g.add_mut(seq_block(1, NodeType::FoldTitle, vec![root, fold]))
            .unwrap();
        g.add_mut(seq_char(2, 'T')).unwrap();
        let fc = g
            .add_mut(seq_block(3, NodeType::FoldContent, vec![root, fold]))
            .unwrap()
            .id;
        let img = g.add_mut(seq_image(4, vec![root, fold, fc])).unwrap().id;
        let unrelated = g
            .add_mut(seq_block(5, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        g.commit_mut();
        let state = State::new(ProjectedState::from_graph(g).unwrap(), None);

        let mut view = make_view(800.0);
        view.layout(&state);

        let before_fold = cached_arc(&mut view, &state, fold);
        let before_unrelated = cached_arc(&mut view, &state, unrelated);

        assert!(view.set_external_height(&state, img, 200.0));

        let after_fold = cached_arc(&mut view, &state, fold);
        let after_unrelated = cached_arc(&mut view, &state, unrelated);

        assert!(
            after_fold.height > before_fold.height,
            "growing the image must grow its owning fold subtree: {} -> {}",
            before_fold.height,
            after_fold.height
        );
        assert!(
            Arc::ptr_eq(&before_unrelated, &after_unrelated),
            "unrelated block must stay cached (cache hit, no re-measure)"
        );
    }
}

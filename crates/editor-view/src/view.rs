use editor_common::{EdgeInsets, Movement, Rect};
use editor_crdt::Op;
use editor_model::{Doc, DocOp, LayoutMode, Node, NodeId};
use editor_resource::Resource;
use editor_state::{
    Affinity, Position, ResolvedPosition, ResolvedSelection, Selection, State,
    resolve_effective_modifiers_at,
};
use std::sync::{Arc, Mutex};

use crate::ExternalElement;
use crate::TableOverlay;
use crate::measure::text::resolve::style_from_effective_modifiers;
use crate::measure::text::strut::compute_strut;
use crate::measure::{MeasuredTree, Measurer};
use crate::page::LayoutPage;
use crate::page_fragment::PageFragmentTree;
use crate::paginate::{
    ChildAttachment, LayoutAtom, LayoutContent, LayoutLine, LayoutNode, Paginator, SpacingKind,
};
use crate::query;
use crate::query::{CursorMetrics, PointerStyle, SelectionEndpoints, SelectionRect};
use crate::view_state::{GapPhantom, GroupDecoration, PendingStyle, ViewState};
use crate::viewport::Viewport;

const CONTINUOUS_MARGIN_X: f32 = 20.0;

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
    pages: Vec<LayoutPage>,
    page_fragments: Vec<PageFragmentTree>,
    content_width: f32,
    layout_index: query::layout_index::LayoutIndex,
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
                let avail_content = (self.viewport.width - 2.0 * CONTINUOUS_MARGIN_X).max(0.0);
                let content_width = (max_width as f32).min(avail_content);
                let page_width = content_width + 2.0 * CONTINUOUS_MARGIN_X;
                (
                    Paginator::continuous(page_width, 1024.0, EdgeInsets::all(CONTINUOUS_MARGIN_X)),
                    content_width,
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

        let paginated = paginator.paginate(measured_tree);

        let pages = paginated.pages;
        let page_fragments = paginated.page_fragments;
        let layout_index = query::layout_index::LayoutIndex::new(paginated.tree, &pages);
        self.layout = Some(LayoutResult {
            pages,
            page_fragments,
            content_width,
            layout_index,
        });
    }

    pub fn visit_page(&self, page_idx: usize, visitor: &mut impl query::PageVisitor) {
        if let Some(ref result) = self.layout
            && let Some(fragment_tree) = result.page_fragments.get(page_idx)
        {
            query::visit_page(fragment_tree, visitor);
        }
    }

    pub fn hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<Selection> {
        let layout_index = &self.layout.as_ref()?.layout_index;
        let point = layout_index.point(page_idx, x, y)?;
        layout_index
            .exact_entry(point, is_text_or_atom_hit_entry)
            .or_else(|| layout_index.closest_entry(point, is_text_or_atom_hit_entry))
            .and_then(|entry| text_or_atom_selection_for_entry(layout_index, entry, point.x))
    }

    pub fn hit_test_extending(
        &self,
        doc: &Doc,
        anchor: Position,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<Selection> {
        let result = self.layout.as_ref()?;
        let point = result.layout_index.point(page_idx, x, y)?;
        let anchor = anchor.resolve(doc)?;

        if let Some(entry) = result
            .layout_index
            .exact_entry(point, is_drag_exact_hit_entry)
        {
            if let Some(selection) =
                query::hard_break::drag_selection_for_entry(&result.layout_index, doc, entry, point)
            {
                return Some(selection);
            }
            if let Some(selection) = query::paragraph_break::drag_selection_for_entry(
                &result.layout_index,
                doc,
                &anchor,
                entry,
                point,
            ) {
                return Some(selection);
            }
            return match entry.content(&result.layout_index) {
                Some(LayoutContent::Line(_) | LayoutContent::Atom(_)) => {
                    text_or_atom_selection_for_entry(&result.layout_index, entry, point.x)
                }
                Some(LayoutContent::Box(b)) if b.style.monolithic => b.attachment.map(select_unit),
                Some(LayoutContent::Spacing(SpacingKind::Gap { .. })) => {
                    drag_boundary_fallback(&result.layout_index, doc, &anchor, point)
                }
                Some(LayoutContent::Box(_) | LayoutContent::Spacing(SpacingKind::Fill)) | None => {
                    None
                }
            };
        }

        drag_boundary_fallback(&result.layout_index, doc, &anchor, point)
    }

    pub fn drop_target_at(
        &self,
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::DropTarget> {
        let result = self.layout.as_ref()?;
        query::drop_target_at(&result.layout_index, doc, page_idx, x, y)
    }

    pub fn interactive_hit_test(
        &self,
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::query::InteractiveHit> {
        let result = self.layout.as_ref()?;
        crate::query::interactive_hit_test(&result.layout_index, doc, page_idx, x, y)
    }

    pub fn page_link_rects(&self, doc: &Doc, page_idx: usize) -> Vec<crate::query::LinkRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        query::page_link_rects(&result.layout_index, page_idx, doc)
    }

    pub fn link_rects(&self, doc: &Doc) -> Vec<crate::query::LinkRect> {
        let Some(result) = self.layout.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for idx in 0..result.pages.len() {
            out.extend(query::page_link_rects(&result.layout_index, idx, doc));
        }
        out
    }

    pub fn link_hit_test(
        &self,
        doc: &Doc,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<crate::query::LinkRect> {
        let result = self.layout.as_ref()?;
        query::link_hit_test(&result.layout_index, page_idx, doc, x, y)
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
        Some(crate::query::pointer_style_at(
            &result.layout_index,
            page_idx,
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

    pub fn editable_position_inside(&self, node_id: NodeId, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        query::navigation::editable_position_inside(&result.layout_index, node_id, at_end)
    }

    pub fn is_at_edge_line_of(&self, node_id: NodeId, head: &Position, at_end: bool) -> bool {
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        query::navigation::is_at_edge_line_of(&result.layout_index, node_id, head, at_end)
    }

    pub fn ensure_preferred_x_at(&mut self, pos: &Position) {
        if self.view_state.preferred_x.is_some() {
            return;
        }
        let Some(result) = self.layout.as_ref() else {
            return;
        };
        self.view_state.preferred_x =
            query::navigation::compute_preferred_x_at(&result.layout_index, pos);
    }

    pub fn position_at_preferred_x_in(&self, node_id: NodeId, at_end: bool) -> Option<Position> {
        let result = self.layout.as_ref()?;
        let x = self.view_state.preferred_x?;
        query::navigation::position_at_preferred_x_in(&result.layout_index, node_id, at_end, x)
    }

    pub fn cursor_metrics(&self, state: &State, pos: &Position) -> Option<CursorMetrics> {
        let result = self.layout.as_ref()?;
        let metrics_override = self.cursor_metrics_at(state, pos);
        query::cursor_metrics(&result.layout_index, pos, metrics_override)
    }

    pub fn placeholder_metrics(&self, doc: &Doc) -> Option<crate::query::PlaceholderMetrics> {
        let result = self.layout.as_ref()?;
        crate::query::placeholder_metrics(&result.layout_index, doc)
    }

    // 입력 경로(`resolve_effective_modifiers_at`)와 동일한 effective set을 써서
    // span 경계의 Expand 규칙·pending_modifiers를 커서 높이에 반영한다.
    fn cursor_metrics_at(&self, state: &State, pos: &Position) -> Option<(f32, f32)> {
        let node = state.doc.node(pos.node_id)?;
        if !matches!(node.node(), Node::Text(_)) {
            return None;
        }
        let modifiers = resolve_effective_modifiers_at(state, pos);
        let style = style_from_effective_modifiers(&modifiers);
        let mut resource = self.measurer.resource.lock().unwrap();
        let strut = compute_strut(&mut resource, &style)?;
        Some((strut.ascent, strut.descent))
    }

    pub fn selection_rects(&self, selection: &ResolvedSelection) -> Vec<SelectionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::selection::selection_rects(&result.layout_index, selection)
    }

    pub fn selection_text_rects(&self, selection: &ResolvedSelection) -> Vec<SelectionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::selection::selection_text_rects(&result.layout_index, selection)
    }

    pub fn selection_endpoints(&self, selection: &ResolvedSelection) -> Option<SelectionEndpoints> {
        let result = self.layout.as_ref()?;
        query::selection::selection_endpoints(&result.layout_index, selection)
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
        query::selection::selection_hit_test(&result.layout_index, selection, page_idx, x, y)
    }

    pub fn node_box_rects(&self, ids: &[NodeId]) -> Vec<SelectionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::selection::block_selection_rects(&result.layout_index, ids)
    }

    pub fn nearest_node_box(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
        ids: &[NodeId],
    ) -> Option<NodeId> {
        let result = self.layout.as_ref()?;
        let point = result.layout_index.point(page_idx, x, y)?;
        result.layout_index.nearest_box(point, ids)
    }

    pub fn node_box_contains(&self, page_idx: usize, x: f32, y: f32, id: NodeId) -> bool {
        let Some(ref result) = self.layout else {
            return false;
        };
        let Some(point) = result.layout_index.point(page_idx, x, y) else {
            return false;
        };
        result.layout_index.box_contains(point, id)
    }

    pub fn node_exact_box_hit_test(&self, page_idx: usize, x: f32, y: f32, id: NodeId) -> bool {
        let Some(ref result) = self.layout else {
            return false;
        };
        let Some(point) = result.layout_index.point(page_idx, x, y) else {
            return false;
        };
        result
            .layout_index
            .exact_entry(point, |_, _| true)
            .and_then(|entry| entry.content(&result.layout_index))
            .is_some_and(|content| matches!(content, LayoutContent::Box(b) if b.node_id == id))
    }

    pub fn composition_rects(
        &self,
        from: &Position,
        to: &Position,
    ) -> Vec<query::composition::CompositionRect> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        query::composition::composition_rects(&result.layout_index, from, to)
    }

    pub fn pages(&self) -> &[LayoutPage] {
        self.layout.as_ref().map_or(&[], |r| &r.pages)
    }

    pub fn external_elements(
        &self,
        doc: &Doc,
        selection: Option<&Selection>,
    ) -> Vec<ExternalElement> {
        let Some(ref result) = self.layout else {
            return Vec::new();
        };
        crate::external::external_elements(&result.layout_index, doc, selection)
    }

    pub fn table_overlays(&self, doc: &Doc, selection: Option<&Selection>) -> Vec<TableOverlay> {
        let Some(ref result) = self.layout else {
            return vec![];
        };
        crate::table_overlay::table_overlays(
            &result.page_fragments,
            doc,
            selection,
            result.content_width,
        )
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
        self.view_state.tracked_decoration_groups.get(group) != Some(decoration)
    }

    pub fn would_remove_group_decoration(&self, group: &str) -> bool {
        self.view_state
            .tracked_decoration_groups
            .contains_key(group)
    }

    pub fn preferred_x(&self) -> Option<f32> {
        self.view_state.preferred_x
    }

    pub fn view_state(&self) -> &ViewState {
        &self.view_state
    }

    pub fn would_resize(&self, viewport: Viewport, _doc: &Doc) -> bool {
        if self.viewport != viewport {
            return true;
        }
        // When viewport is unchanged, fingerprint change depends on doc attributes.
        // Without computing, we conservatively report no change.
        false
    }

    pub fn would_set_external_height(&self, doc: &Doc, node_id: NodeId, height: f32) -> bool {
        if !height.is_finite() || height <= 0.0 || doc.node(node_id).is_none() {
            return false;
        }
        self.view_state.external_height(node_id) != Some(height)
    }

    pub fn would_toggle_fold(&self, doc: &Doc, id: NodeId) -> bool {
        doc.node(id)
            .is_some_and(|n| matches!(n.node(), Node::Fold(_)))
    }

    pub fn would_clear_preferred_x(&self) -> bool {
        self.view_state.preferred_x.is_some()
    }

    pub fn would_ensure_preferred_x_at(&self, pos: &Position) -> bool {
        if self.view_state.preferred_x.is_some() {
            return false;
        }
        let Some(result) = self.layout.as_ref() else {
            return false;
        };
        query::navigation::compute_preferred_x_at(&result.layout_index, pos).is_some()
    }

    /// Dry-run of `resolve_movement` without mutating state.
    ///
    /// Returns `(would_selection, would_preferred_x)` so the caller can compare
    /// against the current selection to detect whether a mutation would actually change anything.
    pub fn would_resolve_movement(
        &self,
        pos: &Position,
        movement: &Movement,
        resource: &Resource,
    ) -> Option<(Option<Selection>, Option<f32>)> {
        let result = self.layout.as_ref()?;
        Some(query::resolve_movement(
            &result.layout_index,
            pos,
            movement,
            &self.viewport,
            resource,
            self.view_state.preferred_x,
        ))
    }
}

fn is_text_or_atom_hit_entry(_entry: &query::layout_index::LayoutEntry, node: &LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_) | LayoutContent::Atom(_)
    )
}

fn is_drag_exact_hit_entry(_entry: &query::layout_index::LayoutEntry, node: &LayoutNode) -> bool {
    match &node.content {
        LayoutContent::Line(_)
        | LayoutContent::Atom(_)
        | LayoutContent::Spacing(SpacingKind::Gap { .. }) => true,
        LayoutContent::Box(b) => b.style.monolithic && b.attachment.is_some(),
        LayoutContent::Spacing(SpacingKind::Fill) => false,
    }
}

fn text_or_atom_selection_for_entry(
    layout_index: &query::layout_index::LayoutIndex,
    entry: &query::layout_index::LayoutEntry,
    x: f32,
) -> Option<Selection> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            Some(Selection::collapsed(position_in_line(line, &entry.rect, x)))
        }
        LayoutContent::Atom(atom) => Some(select_atom(atom)),
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => None,
    }
}

fn drag_boundary_fallback(
    layout_index: &query::layout_index::LayoutIndex,
    doc: &Doc,
    anchor: &ResolvedPosition,
    point: query::layout_index::LayoutPoint,
) -> Option<Selection> {
    let mut inside: Option<DragFallbackCandidate> = None;
    let mut before: Option<DragFallbackCandidate> = None;
    let mut after: Option<DragFallbackCandidate> = None;

    for entry in layout_index.entries_on_page(point.page_idx) {
        let Some(candidate) = drag_fallback_candidate(layout_index, entry, point) else {
            continue;
        };
        let slot = if point.y >= entry.rect.y && point.y < entry.rect.bottom() {
            &mut inside
        } else if entry.rect.bottom() <= point.y {
            &mut before
        } else if entry.rect.y >= point.y {
            &mut after
        } else {
            continue;
        };
        if candidate.is_better_than(slot.as_ref()) {
            *slot = Some(candidate);
        }
    }

    if let Some(candidate) = inside {
        return Some(candidate.selection);
    }

    let prefer_before = after
        .as_ref()
        .and_then(|candidate| candidate.start.resolve(doc))
        .is_none_or(|after_start| anchor < &after_start);
    let candidate = if prefer_before {
        before.or(after)
    } else {
        after.or(before)
    };
    candidate.map(|candidate| candidate.selection)
}

struct DragFallbackCandidate {
    distance: (f32, f32),
    start: Position,
    selection: Selection,
}

impl DragFallbackCandidate {
    fn new(
        entry: &query::layout_index::LayoutEntry,
        point: query::layout_index::LayoutPoint,
        start: Position,
        selection: Selection,
    ) -> Self {
        Self {
            distance: distance_key(&entry.rect, point.x, point.y),
            start,
            selection,
        }
    }

    fn is_better_than(&self, other: Option<&Self>) -> bool {
        other.is_none_or(|best| compare_distance_key(self.distance, best.distance).is_lt())
    }
}

fn drag_fallback_candidate(
    layout_index: &query::layout_index::LayoutIndex,
    entry: &query::layout_index::LayoutEntry,
    point: query::layout_index::LayoutPoint,
) -> Option<DragFallbackCandidate> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            let start = position_in_line(line, &entry.rect, entry.rect.x);
            let end = query::grapheme::last_position_in_line(line);
            let pos = if point.y < entry.rect.y {
                start
            } else if point.y >= entry.rect.bottom() {
                end
            } else {
                position_in_line(line, &entry.rect, point.x)
            };
            Some(DragFallbackCandidate::new(
                entry,
                point,
                start,
                Selection::collapsed(pos),
            ))
        }
        LayoutContent::Atom(atom) => {
            let hit = select_atom(atom);
            Some(DragFallbackCandidate::new(entry, point, hit.anchor, hit))
        }
        LayoutContent::Box(b) if b.style.monolithic && b.attachment.is_some() => {
            if point.y >= entry.rect.y && point.y < entry.rect.bottom() {
                return None;
            }
            let hit = select_unit(b.attachment?);
            Some(DragFallbackCandidate::new(entry, point, hit.anchor, hit))
        }
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => None,
    }
}

fn position_in_line(line: &LayoutLine, rect: &Rect, x: f32) -> Position {
    query::grapheme::position_at_x(line, x - rect.x)
}

fn select_atom(atom: &LayoutAtom) -> Selection {
    Selection::new(
        Position {
            node_id: atom.attachment.parent_id,
            offset: atom.attachment.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: atom.attachment.parent_id,
            offset: atom.attachment.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

fn select_unit(attachment: ChildAttachment) -> Selection {
    Selection::new(
        Position {
            node_id: attachment.parent_id,
            offset: attachment.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: attachment.parent_id,
            offset: attachment.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

fn compare_distance_key(a: (f32, f32), b: (f32, f32)) -> std::cmp::Ordering {
    match a.0.total_cmp(&b.0) {
        std::cmp::Ordering::Equal => a.1.total_cmp(&b.1),
        ordering => ordering,
    }
}

fn distance_key(rect: &Rect, x: f32, y: f32) -> (f32, f32) {
    (
        axis_distance(rect.y, rect.bottom(), y),
        axis_distance(rect.x, rect.right(), x),
    )
}

fn axis_distance(start: f32, end: f32, value: f32) -> f32 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.0
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
        self.layout.as_ref().map(|r| r.layout_index.tree())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::Direction;
    use editor_macros::{doc, state};
    use editor_state::{Affinity, Position};

    fn make_op(id: editor_crdt::Dot, payload: DocOp) -> Op<DocOp> {
        Op {
            id,
            parents: Default::default(),
            payload,
        }
    }

    fn mk_state(doc: Doc) -> State {
        State::new(doc, editor_crdt::OpGraph::new(), None)
    }

    #[test]
    fn layout_produces_pages() {
        let (doc,) = doc! { root { paragraph { text("hello") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        assert!(!view.pages().is_empty());
    }

    #[test]
    fn hit_test_extending_above_top_returns_first_flow_target() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("before") }
                paragraph { t2: text("after") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view
            .hit_test_extending(&doc, Position::new(t2, 2), 0, 20.0, -100.0)
            .unwrap();
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor, Position::new(t1, 0));
        assert_eq!(sel.head, Position::new(t1, 0));
    }

    #[test]
    fn hit_test_extending_below_bottom_returns_last_flow_target() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("before") }
                paragraph { t2: text("after") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view
            .hit_test_extending(&doc, Position::new(t1, 2), 0, 20.0, 99999.0)
            .unwrap();
        let end = Position {
            node_id: t2,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor, end);
        assert_eq!(sel.head, end);
    }

    #[test]
    fn hit_test_extending_selects_monolithic_box_from_leading_padding() {
        let (doc, callout) = doc! {
            root {
                paragraph { text("before") }
                callout: callout {
                    paragraph { text("inside") }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let callout_rect = view.node_box_rects(&[callout])[0].rect;

        let sel = view
            .hit_test_extending(
                &doc,
                Position::new(NodeId::ROOT, 0),
                0,
                callout_rect.x + callout_rect.width / 2.0,
                callout_rect.y + 4.0,
            )
            .unwrap();

        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor.node_id, NodeId::ROOT);
        assert_eq!(sel.anchor.offset, 1);
        assert_eq!(sel.anchor.affinity, Affinity::Downstream);
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(sel.head.offset, 2);
        assert_eq!(sel.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn hit_test_extending_selects_monolithic_box_from_trailing_padding() {
        let (doc, callout) = doc! {
            root {
                callout: callout {
                    paragraph { text("inside") }
                }
                paragraph { text("after") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let callout_rect = view.node_box_rects(&[callout])[0].rect;

        let sel = view
            .hit_test_extending(
                &doc,
                Position::new(NodeId::ROOT, 1),
                0,
                callout_rect.x + callout_rect.width / 2.0,
                callout_rect.y + callout_rect.height - 4.0,
            )
            .unwrap();

        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor.node_id, NodeId::ROOT);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.anchor.affinity, Affinity::Downstream);
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(sel.head.offset, 1);
        assert_eq!(sel.head.affinity, Affinity::Upstream);
    }

    #[test]
    fn hit_test_extending_in_paragraph_block_gap_returns_paragraph_break() {
        let (doc, p1, t1, p2) = doc! {
            root {
                p1: paragraph { t1: text("one") }
                p2: paragraph { text("two") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let p2_rect = view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        let sel = view
            .hit_test_extending(
                &doc,
                Position::new(t1, 1),
                0,
                p1_rect.x + p1_rect.width / 2.0,
                (p1_rect.bottom() + p2_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            sel,
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 3))
                .expect("P -> P has PB")
        );
    }

    #[test]
    fn hit_test_extending_in_paragraph_gap_selects_paragraph_break() {
        let (doc, p1, t1, p2, _t2) = doc! {
            root [block_gap(200)] {
                p1: paragraph { t1: text("one") }
                p2: paragraph { t2: text("two") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let p2_rect = view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(t1, 3),
                0,
                p1_rect.x + p1_rect.width / 2.0,
                (p1_rect.bottom() + p2_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            hit,
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 3))
                .expect("P -> P has PB")
        );
    }

    #[test]
    fn hit_test_extending_up_to_paragraph_gap_excludes_previous_paragraph_break() {
        let (doc, p1, _t1, p2, t2) = doc! {
            root [block_gap(200)] {
                p1: paragraph { t1: text("one") }
                p2: paragraph { t2: text("two") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let p2_rect = view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(t2, 1),
                0,
                p2_rect.x + p2_rect.width / 2.0,
                (p1_rect.bottom() + p2_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            hit,
            Selection::collapsed(Position {
                node_id: t2,
                offset: 0,
                affinity: Affinity::Upstream,
            })
        );
    }

    #[test]
    fn hit_test_extending_in_gap_after_text_paragraph_before_atom_is_not_paragraph_break() {
        let (doc, p1, t1, image) = doc! {
            root [block_gap(200)] {
                p1: paragraph { t1: text("one") }
                image: image
                paragraph { text("tail") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let image_rect = view
            .external_elements(&doc, None)
            .into_iter()
            .find(|element| element.node_id == image)
            .expect("image external element exists")
            .bounds;
        assert!(
            p1_rect.bottom() < image_rect.y,
            "test requires a visible paragraph gap"
        );

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(t1, 1),
                0,
                p1_rect.x + p1_rect.width / 2.0,
                (p1_rect.bottom() + image_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            hit,
            Selection::collapsed(Position {
                node_id: t1,
                offset: 3,
                affinity: Affinity::Upstream,
            })
        );
    }

    #[test]
    fn hit_test_extending_in_gap_after_removable_empty_paragraph_selects_paragraph_break() {
        let (doc, p1, empty, image) = doc! {
            root [block_gap(200)] {
                p1: paragraph { text("one") }
                empty: paragraph {}
                image: image
                paragraph { text("tail") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let empty_rect = view.node_box_rects(&[empty])[0].rect;
        let image_rect = view
            .external_elements(&doc, None)
            .into_iter()
            .find(|element| element.node_id == image)
            .expect("image external element exists")
            .bounds;
        assert!(
            empty_rect.bottom() < image_rect.y,
            "test requires a visible paragraph gap"
        );

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(p1, 0),
                0,
                empty_rect.x + empty_rect.width / 2.0,
                (empty_rect.bottom() + image_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            hit,
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(empty, 0))
                .expect("removable empty paragraph has PB")
        );
    }

    #[test]
    fn hit_test_extending_paragraph_break_right_side_returns_next_paragraph_start() {
        let (doc, _p1, t1, t2) = doc! {
            root {
                p1: paragraph { t1: text("aa") }
                paragraph { t2: text("bb") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let pb_selection =
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 2))
                .expect("P -> P has PB")
                .resolve(&doc)
                .unwrap();
        let pb_rect = view
            .selection_rects(&pb_selection)
            .into_iter()
            .find(|rect| rect.meta == crate::query::SelectionRectKind::ParagraphBreak)
            .expect("paragraph break rect exists");

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(t1, 0),
                pb_rect.page_idx,
                pb_rect.rect.right() + 4.0,
                pb_rect.rect.y + pb_rect.rect.height / 2.0,
            )
            .unwrap();

        assert_eq!(
            hit,
            Selection::new(
                Position {
                    node_id: t1,
                    offset: 2,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: t2,
                    offset: 0,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn hit_test_extending_next_line_page_margin_after_paragraph_break_returns_next_paragraph_start()
    {
        let (doc, _p1, t1, t2) = doc! {
            root {
                p1: paragraph { t1: text("aa") }
                paragraph { t2: text("bb") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = mk_state(doc.clone());
        let next_line = view
            .cursor_metrics(&state, &Position::new(t2, 0))
            .expect("line after paragraph break has cursor metrics");

        let hit = view
            .hit_test_extending(
                &state.doc,
                Position::new(t1, 0),
                next_line.page_idx,
                next_line.line.x - 12.0,
                next_line.line.y + next_line.line.height / 2.0,
            )
            .unwrap();

        assert_eq!(hit, Selection::collapsed(Position::new(t2, 0)));
    }

    #[test]
    fn hit_test_extending_hard_break_rect_returns_hard_break_selection() {
        let (doc, p1, _t1, _t2) = doc! {
            root {
                p1: paragraph {
                    t1: text("a")
                    hard_break
                    t2: text("b")
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let hard_break = Selection::new(
            Position {
                node_id: p1,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: p1,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let hard_break_rect = view
            .selection_rects(
                &hard_break
                    .resolve(&doc)
                    .expect("hard_break selection resolves"),
            )
            .into_iter()
            .find(|rect| rect.meta == crate::query::SelectionRectKind::Text)
            .expect("hard_break rect exists");

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(p1, 0),
                hard_break_rect.page_idx,
                hard_break_rect.rect.x + hard_break_rect.rect.width / 2.0,
                hard_break_rect.rect.y + hard_break_rect.rect.height / 2.0,
            )
            .unwrap();

        assert_eq!(hit, hard_break);
    }

    #[test]
    fn hit_test_extending_hard_break_line_right_side_returns_hard_break_selection() {
        let (doc, p1, _t1, _t2) = doc! {
            root {
                p1: paragraph {
                    t1: text("a")
                    hard_break
                    t2: text("b")
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let hard_break = Selection::new(
            Position {
                node_id: p1,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position {
                node_id: p1,
                offset: 2,
                affinity: Affinity::Upstream,
            },
        );
        let hard_break_rect = view
            .selection_rects(
                &hard_break
                    .resolve(&doc)
                    .expect("hard_break selection resolves"),
            )
            .into_iter()
            .find(|rect| rect.meta == crate::query::SelectionRectKind::Text)
            .expect("hard_break rect exists");

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(p1, 0),
                hard_break_rect.page_idx,
                hard_break_rect.rect.right() + 12.0,
                hard_break_rect.rect.y + hard_break_rect.rect.height / 2.0,
            )
            .unwrap();

        assert_eq!(hit, hard_break);
    }

    #[test]
    fn hit_test_extending_next_line_page_margin_after_hard_break_returns_next_line_start() {
        let (doc, p1, _t1, t2) = doc! {
            root {
                p1: paragraph {
                    t1: text("a")
                    hard_break
                    t2: text("b")
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = mk_state(doc.clone());
        let next_line = view
            .cursor_metrics(
                &state,
                &Position {
                    node_id: p1,
                    offset: 2,
                    affinity: Affinity::Downstream,
                },
            )
            .expect("line after hard_break has cursor metrics");

        let hit = view
            .hit_test_extending(
                &state.doc,
                Position::new(p1, 0),
                next_line.page_idx,
                next_line.line.x - 12.0,
                next_line.line.y + next_line.line.height / 2.0,
            )
            .unwrap();

        assert_eq!(hit, Selection::collapsed(Position::new(t2, 0)));
    }

    #[test]
    fn hit_test_extending_removable_empty_paragraph_break_right_side_returns_block_boundary() {
        let (doc, root, p1) = doc! {
            root: root {
                p1: paragraph {}
                image
                paragraph {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let pb_end = Position {
            node_id: root,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let pb_selection = Selection::new(Position::new(p1, 0), pb_end)
            .resolve(&doc)
            .unwrap();
        let pb_rect = view
            .selection_rects(&pb_selection)
            .into_iter()
            .find(|rect| rect.meta == crate::query::SelectionRectKind::ParagraphBreak)
            .expect("paragraph break rect exists");

        let hit = view
            .hit_test_extending(
                &doc,
                Position::new(p1, 0),
                pb_rect.page_idx,
                pb_rect.rect.right() + 4.0,
                pb_rect.rect.y + pb_rect.rect.height / 2.0,
            )
            .unwrap();

        assert_eq!(hit, Selection::new(Position::new(p1, 0), pb_end));
    }

    #[test]
    fn hit_test_extending_in_monolithic_internal_paragraph_gap_returns_paragraph_break() {
        let (doc, callout, p1, t1, p2) = doc! {
            root {
                callout: callout {
                    p1: paragraph { t1: text("one") }
                    p2: paragraph { text("two") }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let callout_rect = view.node_box_rects(&[callout])[0].rect;
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let p2_rect = view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        let sel = view
            .hit_test_extending(
                &doc,
                Position::new(t1, 1),
                0,
                callout_rect.x + callout_rect.width / 2.0,
                (p1_rect.bottom() + p2_rect.y) / 2.0,
            )
            .unwrap();

        assert_eq!(
            sel,
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 3))
                .expect("P -> P has PB")
        );
    }

    #[test]
    fn hit_test_click_in_page_margin_uses_nearest_text_leaf() {
        let (doc, title) = doc! {
            root {
                fold {
                    fold_title { title: text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view.hit_test(0, 20.0, -100.0).unwrap();
        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, title);
    }

    #[test]
    fn hit_test_click_in_block_gap_uses_nearest_text_boundary() {
        let (doc, p1, t1, p2) = doc! {
            root {
                p1: paragraph { t1: text("one") }
                p2: paragraph { text("two") }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let p1_rect = view.node_box_rects(&[p1])[0].rect;
        let p2_rect = view.node_box_rects(&[p2])[0].rect;
        assert!(
            p1_rect.bottom() < p2_rect.y,
            "test requires a visible paragraph gap"
        );

        let sel = view
            .hit_test(
                0,
                p1_rect.x + p1_rect.width / 2.0,
                (p1_rect.bottom() + p2_rect.y) / 2.0,
            )
            .unwrap();

        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, t1);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn hit_test_click_in_unit_chrome_uses_text_fallback() {
        let (doc, callout, t) = doc! {
            root {
                callout: callout {
                    paragraph { t: text("inside") }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let callout_rect = view.node_box_rects(&[callout])[0].rect;

        let sel = view
            .hit_test(
                0,
                callout_rect.x + callout_rect.width / 2.0,
                callout_rect.y + 4.0,
            )
            .unwrap();

        assert!(sel.is_collapsed());
        assert_eq!(
            sel.head.node_id, t,
            "plain click hit-test must land on text, not select the containing unit"
        );
    }

    #[test]
    fn hit_test_click_on_atom_selects_atom() {
        let (doc, image) = doc! {
            root {
                image: image(id: Some("img".to_string()), proportion: 50)
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);
        let image_rect = view
            .external_elements(&doc, None)
            .into_iter()
            .find(|element| element.node_id == image)
            .expect("image external element")
            .bounds;

        let sel = view
            .hit_test(
                0,
                image_rect.x + image_rect.width / 2.0,
                image_rect.y + image_rect.height / 2.0,
            )
            .unwrap();

        assert!(!sel.is_collapsed());
        assert_eq!(sel.anchor, Position::new(NodeId::ROOT, 0));
        assert_eq!(
            sel.head,
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
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
        let state = mk_state(doc);

        let pos = Position::new(p1, 0);
        let default_rect = view.cursor_metrics(&state, &pos).unwrap();

        // With no pending modifiers, cursor uses stored strut metrics.
        assert!(default_rect.caret.height > 0.0);
    }

    #[test]
    fn cursor_rect_matches_adjacent_text_font_size() {
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
        let state = mk_state(doc);

        let r1 = view.cursor_metrics(&state, &Position::new(t1, 1)).unwrap();
        let r2 = view.cursor_metrics(&state, &Position::new(t2, 1)).unwrap();

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
        let state = mk_state(doc);

        // offset 1 = end of each single-char node so the Expand::After mark
        // on `big` actually applies (Expand::After only kicks in at_end).
        let small_caret = view
            .cursor_metrics(&state, &Position::new(small, 1))
            .unwrap()
            .caret;
        let big_caret = view
            .cursor_metrics(&state, &Position::new(big, 1))
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
        let state = mk_state(doc);
        let pos = Position::new(p1, 0);
        let baseline = view.cursor_metrics(&state, &pos).unwrap();

        let pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        view.reconcile_with_ops(&state.doc, &state.doc, &[], pending_style, None);
        let pending = view.cursor_metrics(&state, &pos).unwrap();

        assert!(pending.caret.height > baseline.caret.height);
        assert!(pending.line.height > baseline.line.height);
        assert!(pending.line.height >= pending.caret.height);
    }

    #[test]
    fn cursor_on_empty_right_aligned_paragraph_rests_at_right_edge() {
        let (doc, p1) = doc! { root { p1: paragraph [alignment(Alignment::Right)] } };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = mk_state(doc);

        let pos = Position::new(p1, 0);
        let m = view.cursor_metrics(&state, &pos).unwrap();

        assert!(
            (m.caret.x - (m.line.x + m.line.width)).abs() < 1.0,
            "right-aligned empty paragraph caret must rest at the right edge \
             (caret.x={}, line.x={}, line.width={})",
            m.caret.x,
            m.line.x,
            m.line.width,
        );
    }

    #[test]
    fn cursor_on_empty_center_aligned_paragraph_rests_at_horizontal_center() {
        let (doc, p1) = doc! { root { p1: paragraph [alignment(Alignment::Center)] } };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = mk_state(doc);

        let pos = Position::new(p1, 0);
        let m = view.cursor_metrics(&state, &pos).unwrap();

        let mid = m.line.x + m.line.width / 2.0;
        assert!(
            (m.caret.x - mid).abs() < 1.0,
            "center-aligned empty paragraph caret must rest at horizontal center \
             (caret.x={}, mid={})",
            m.caret.x,
            mid,
        );
    }

    #[test]
    fn cursor_metrics_pending_on_non_empty_paragraph_unchanged() {
        use crate::view_state::PendingStyle;
        use editor_model::Modifier;
        use editor_state::PendingModifier;

        let (doc, p1, t1) = doc! { root { p1: paragraph { t1: text("hi") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let state = mk_state(doc);
        let pos = Position::new(t1, 0);
        let baseline = view.cursor_metrics(&state, &pos).unwrap();

        let pending_style = Some(PendingStyle {
            node_id: p1,
            modifiers: vec![PendingModifier::Set {
                modifier: Modifier::FontSize { value: 9600 },
            }],
        });
        view.reconcile_with_ops(&state.doc, &state.doc, &[], pending_style, None);
        let after = view.cursor_metrics(&state, &pos).unwrap();

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
    fn view_selection_rects_use_cell_boxes_for_cell_rect() {
        let (state, c00, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
                table_row {
                    table_cell { paragraph { text("z") } }
                    table_cell { paragraph { text("w") } }
                    table_cell { paragraph { text("v") } }
                }
            } } }
            selection: (c00, 0)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);

        let selection = editor_state::cell_rect_selection(&state.doc, c00, c11).unwrap();
        let resolved = selection.resolve(&state.doc).unwrap();
        let ids: Vec<_> = resolved
            .as_cell_rect()
            .unwrap()
            .cells()
            .map(|cell| cell.id())
            .collect();

        assert_eq!(view.selection_rects(&resolved), view.node_box_rects(&ids));
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
        assert_eq!(endpoints.from_position, Position::new(t, 1));
        assert_eq!(endpoints.from.rect.x, only.rect.x);
        assert_eq!(endpoints.from.rect.y, only.rect.y);
        assert_eq!(endpoints.from.rect.width, 0.0);
        assert_eq!(endpoints.from.rect.height, only.rect.height);

        assert_eq!(endpoints.to.page_idx, only.page_idx);
        assert_eq!(endpoints.to_position, Position::new(t, 4));
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
        assert_eq!(b.from_position, Position::new(t, 1));
        assert_eq!(b.to_position, Position::new(t, 4));
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

        let rects: Vec<_> = view
            .selection_rects(&resolved)
            .into_iter()
            .filter(|rect| rect.meta == crate::query::SelectionRectKind::Text)
            .collect();
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
mod tests_would {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn would_clear_preferred_x_false_when_none() {
        let v = View::new_test();
        assert!(!v.would_clear_preferred_x());
    }

    #[test]
    fn would_toggle_fold_false_for_non_fold_node() {
        let (d, p1) = doc! { root { p1: paragraph { text("hi") } } };
        let v = View::new_test();
        assert!(!v.would_toggle_fold(&d, p1));
    }

    #[test]
    fn would_resize_false_for_same_viewport() {
        let (d,) = doc! { root { paragraph { text("hi") } } };
        let mut v = View::new_test();
        let vp = Viewport::new(800.0, 600.0, 1.0);
        v.resize(vp, &d);
        assert!(!v.would_resize(vp, &d));
    }

    #[test]
    fn would_resize_true_for_different_viewport() {
        let (d,) = doc! { root { paragraph { text("hi") } } };
        let mut v = View::new_test();
        let vp = Viewport::new(800.0, 600.0, 1.0);
        v.resize(vp, &d);
        let vp2 = Viewport::new(1024.0, 600.0, 1.0);
        assert!(v.would_resize(vp2, &d));
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

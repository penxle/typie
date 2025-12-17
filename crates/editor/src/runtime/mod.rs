mod cmd;
mod context;
mod dnd;
mod effect;
mod handlers;
mod message;
mod pointer;
mod state;
mod view_state;

pub use cmd::*;
pub use context::*;
pub use dnd::*;
pub use effect::*;
pub use message::*;
pub use pointer::*;
pub use state::*;
pub use view_state::*;

use crate::inspect::inspect_page_element;
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::query::find_drag_image_bounds;
use crate::layout::{Layout, LayoutCache, LayoutContext, Page, Paginator};
use crate::model::*;
use crate::render::{RenderResult, Renderer};

use crate::schema::Expand;
use crate::state::selection_helpers::{build_selection_decorations, compute_selection_aggregates};
use crate::state::{Position, Preedit, Selection, find_child_at_offset, position_in_selection};
use crate::transaction::Transaction;
use crate::types::{Affinity, BoxConstraints, PointerStyle, Rect, Size, WritingSystem};
use anyhow::Result;
use loro::UndoManager;
use rustc_hash::FxHashSet;
use std::cell::RefCell;

#[derive(Default)]
struct PendingUpdates {
    doc: bool,
    selection: bool,
    active_marks: bool,
    cursor: bool,
    scroll_to_cursor: bool,
    settings: bool,
    layout: bool,
    external_elements: bool,
    pointer_style: Option<PointerStyle>,
    fonts: Vec<(String, u16)>,
    writing_systems: Vec<WritingSystem>,
    render: bool,
    drop_indicator: Option<DropIndicator>,
    enabled_actions: bool,
    exited_document_start: bool,
    pointer_mode_changed: bool,
}

#[derive(Clone)]
struct SelectionSnapshot {
    from: Position,
    to: Position,
    block_ids: Vec<NodeId>,
    block_set: FxHashSet<NodeId>,
    stats: SelectionStats,
    uniform_marks: Vec<Mark>,
    mixed_marks: Vec<MarkType>,
}

#[allow(dead_code)]
pub struct Runtime {
    width: f32,
    scale_factor: f64,
    renderer: Renderer,
    pages: Vec<Page>,

    state: State,
    undo_manager: UndoManager,
    layout_cache: RefCell<LayoutCache>,
    view_states: ViewStates,

    loaded_fonts: FxHashSet<(String, u16)>,
    loaded_scripts: FxHashSet<WritingSystem>,

    selection_cache: Option<SelectionSnapshot>,
    pending: PendingUpdates,
    message_queue: Vec<Message>,
    pointer: PointerState,

    undo_selections: Vec<Selection>,
    redo_selections: Vec<Selection>,
}

#[allow(dead_code)]
impl Runtime {
    pub fn new(width: f32, scale_factor: f64, mut state: State) -> Self {
        let mut undo_manager = UndoManager::new(&state.doc.loro_doc());
        undo_manager.set_merge_interval(1000);

        let orphans = state.doc.find_orphan_nodes();
        state.garbage_ids.extend(orphans);

        Self {
            width,
            scale_factor,
            renderer: Renderer::new(scale_factor),
            state,
            pages: Vec::new(),
            undo_manager,
            layout_cache: RefCell::new(LayoutCache::new()),
            view_states: ViewStates::default(),
            loaded_fonts: FxHashSet::default(),
            loaded_scripts: FxHashSet::default(),
            selection_cache: None,
            pending: PendingUpdates {
                doc: true,
                cursor: true,
                scroll_to_cursor: true,
                selection: true,
                active_marks: true,
                settings: true,
                layout: true,
                external_elements: true,
                pointer_style: None,
                fonts: Vec::new(),
                writing_systems: Vec::new(),
                render: true,
                drop_indicator: None,
                enabled_actions: true,
                exited_document_start: false,
                pointer_mode_changed: true,
            },
            message_queue: Vec::new(),
            pointer: PointerState::default(),
            undo_selections: Vec::new(),
            redo_selections: Vec::new(),
        }
    }

    pub fn enqueue_message(&mut self, msg: Message) {
        if let Some(last) = self.message_queue.last_mut() {
            if Self::try_merge_message(last, &msg) {
                return;
            }
        }

        self.message_queue.push(msg);
    }

    fn try_merge_message(last: &mut Message, new: &Message) -> bool {
        match (last, new) {
            (Message::Input { text: last_text }, Message::Input { text: new_text }) => {
                last_text.push_str(new_text);
                true
            }
            (last @ Message::PointerMove { .. }, new @ Message::PointerMove { .. }) => {
                *last = new.clone();
                true
            }
            (
                Message::CompositionUpdate { text: last_text },
                Message::CompositionUpdate { text: new_text },
            ) => {
                *last_text = new_text.clone();
                true
            }
            _ => false,
        }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn doc(&self) -> &Doc {
        &self.state.doc
    }

    pub fn selection(&self) -> &Selection {
        &self.state.selection
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.state.preedit.as_ref()
    }

    fn selection_snapshot(&mut self) -> Option<&SelectionSnapshot> {
        let selection = &self.state.selection;
        let Ok((from, to)) = selection.as_sorted(self.doc()) else {
            self.selection_cache = None;
            return None;
        };

        let needs_rebuild = self
            .selection_cache
            .as_ref()
            .map(|cache| cache.from != from || cache.to != to)
            .unwrap_or(true);

        if needs_rebuild {
            let block_ids =
                crate::state::collect_blocks_in_range(self.doc(), from.clone(), to.clone())
                    .unwrap_or_default();
            let mut block_set = FxHashSet::default();
            block_set.extend(block_ids.iter().copied());

            let (paragraph_count, uniform_align, uniform_line_height, uniform_marks, mixed_marks) =
                compute_selection_aggregates(self.doc(), &block_ids, from.clone(), to.clone());

            let stats = SelectionStats {
                block_count: block_ids.len(),
                paragraph_count,
                uniform_align,
                uniform_line_height,
            };

            self.selection_cache = Some(SelectionSnapshot {
                from: from.clone(),
                to: to.clone(),
                block_ids,
                block_set,
                stats,
                uniform_marks,
                mixed_marks,
            });
        }

        self.selection_cache.as_ref()
    }

    fn selection_snapshot_owned(&mut self) -> Option<SelectionSnapshot> {
        self.selection_snapshot().cloned()
    }
    fn set_width(&mut self, width: f32) {
        if self.width != width {
            self.layout_cache.borrow_mut().invalidate_all();
        }
        self.width = width;
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub fn layout(&mut self) {
        let settings = self.doc().settings();

        let (page_width, page_height, margin_top, margin_bottom, margin_left, margin_right) =
            match settings.layout_mode {
                LayoutMode::Paginated {
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
                LayoutMode::Continuous { max_width } => {
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

        let preedit = self.preedit();

        let decorations = Decorations {
            preedit: preedit.map(|preedit| PreeditDecor {
                node_id: preedit.node_id,
                offset: preedit.offset,
                text: preedit.text.clone(),
                marks: preedit.marks.clone(),
            }),
        };

        let root_ref = self.doc().node(NodeId::ROOT).unwrap();
        let ctx = LayoutContext::new(
            &root_ref,
            &settings,
            &decorations,
            self.scale_factor,
            &self.view_states,
            &self.layout_cache,
        );

        let root_node = root_ref.node().layout(&ctx, constraints);

        let paginator = Paginator::new(
            page_width,
            page_height,
            margin_top,
            margin_bottom,
            margin_left,
            settings.layout_mode,
        );
        self.pages = paginator.paginate(root_node);
    }

    pub fn render_page(&mut self, page_index: usize) -> Option<RenderResult> {
        let snapshot = self.selection_snapshot_owned();

        let doc = &self.state.doc;
        let selection = &self.state.selection;

        let selections = if let Some(snapshot) = snapshot.as_ref() {
            build_selection_decorations(doc, selection, Some(&snapshot.block_ids))
        } else {
            build_selection_decorations(doc, selection, None)
        };

        let page = self.pages.get(page_index)?;

        let layout_mode = doc.settings().layout_mode;
        let page_height = match layout_mode {
            LayoutMode::Paginated { page_height, .. } => page_height.ceil(),
            LayoutMode::Continuous { .. } => page.root.node.size.height.ceil(),
        };

        self.renderer
            .set_size(self.width.ceil(), page_height, self.scale_factor);

        let drop_indicator = self.pending.drop_indicator.as_ref();

        Some(
            self.renderer
                .render(page, page_index, &selections, drop_indicator, doc),
        )
    }

    pub fn render_drag_image(
        &mut self,
        visible_pages: &[usize],
        drag_page_idx: usize,
    ) -> Option<crate::render::DragImageResult> {
        let doc = &self.state.doc;
        let selection = &self.state.selection;

        let bounds = find_drag_image_bounds(doc, selection, &self.pages)?;

        let selections = build_selection_decorations(doc, selection, None);

        self.renderer.render_drag_image(
            &self.pages,
            &bounds,
            &selections,
            doc,
            visible_pages,
            drag_page_idx,
        )
    }

    fn collect_selection_marks(
        &mut self,
        snapshot: Option<SelectionSnapshot>,
    ) -> (Vec<Mark>, Vec<MarkType>) {
        if let Some(marks) = &self.state.pending_marks {
            return (marks.clone(), Vec::new());
        }

        let selection = &self.state.selection;

        if selection.is_collapsed() {
            return (self.get_marks_at_position(&selection.head), Vec::new());
        }

        let snapshot_owned = snapshot.or_else(|| self.selection_snapshot_owned());
        let Some(snapshot) = snapshot_owned else {
            return (Vec::new(), Vec::new());
        };

        (snapshot.uniform_marks, snapshot.mixed_marks)
    }

    fn get_marks_at_position(&self, position: &Position) -> Vec<Mark> {
        let Some(node) = self.doc().node(position.node_id) else {
            return Vec::new();
        };

        let Some((child_id, local_offset)) = find_child_at_offset(&node, position.offset) else {
            return Vec::new();
        };

        let Some(child) = self.doc().node(child_id) else {
            return Vec::new();
        };

        let Node::Text(text_node) = child.node() else {
            return Vec::new();
        };

        let schema = self.doc().schema();
        let segments = text_node.text.get_rich_text_segments();
        let mut current_offset = 0;

        for (segment_text, segment_marks) in segments {
            let segment_len = segment_text.chars().count();
            let segment_start = current_offset;
            let segment_end = current_offset + segment_len;

            let at_middle = local_offset > segment_start && local_offset < segment_end;
            let at_end = local_offset == segment_end && segment_len > 0;
            let at_start = local_offset == segment_start;

            if at_middle {
                return segment_marks;
            }

            if at_end {
                return segment_marks
                    .into_iter()
                    .filter(|m| {
                        matches!(
                            schema.mark_spec(m.as_type()).expand,
                            Expand::After | Expand::Both
                        )
                    })
                    .collect();
            }

            if at_start {
                return segment_marks
                    .into_iter()
                    .filter(|m| {
                        matches!(
                            schema.mark_spec(m.as_type()).expand,
                            Expand::Before | Expand::Both
                        )
                    })
                    .collect();
            }

            current_offset += segment_len;
        }

        Vec::new()
    }

    pub fn get_pointer_style(&self, page_idx: usize, x: f32, y: f32) -> PointerStyle {
        let Some(page) = self.pages.get(page_idx) else {
            return PointerStyle::Default;
        };

        page.get_pointer_style(x, y)
    }

    pub fn tick(&mut self) -> Vec<Cmd> {
        let messages = std::mem::take(&mut self.message_queue);

        for msg in messages {
            self.process_message(msg);
        }

        self.build_commands()
    }

    pub fn flush(&mut self) {
        self.collect_garbage();

        if self.state.pending_loro_commit {
            self.state.doc.loro_doc().commit();
            self.state.pending_loro_commit = false;
        }
    }

    fn collect_garbage(&mut self) {
        if self.state.garbage_ids.is_empty() {
            return;
        }

        let chunk_size = 500;
        let len = self.state.garbage_ids.len();
        let drain_count = chunk_size.min(len);

        let ids_to_delete: Vec<NodeId> = self.state.garbage_ids.drain(0..drain_count).collect();

        for &id in &ids_to_delete {
            let children = self.doc().get_children_ids(id);
            self.state.garbage_ids.extend(children);
        }

        if let Err(e) = self.doc().delete_nodes_batch(&ids_to_delete) {
            eprintln!("Failed to delete garbage nodes: {:?}", e);
        }

        self.state.pending_loro_commit = true;
    }

    fn process_message(&mut self, msg: Message) {
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn build_commands(&mut self) -> Vec<Cmd> {
        let mut cmds = Vec::new();

        if self.pending.doc {
            cmds.push(Cmd::DocChanged);
            self.pending.doc = false;
        }

        if self.pending.render {
            cmds.push(Cmd::RenderRequired);
            self.pending.render = false;
        }

        if self.pending.settings {
            let settings = self.doc().settings();
            cmds.push(Cmd::SettingsChanged {
                paragraph_indent: settings.paragraph_indent,
                block_gap: settings.block_gap,
            });
            self.pending.settings = false;
        }

        if self.pending.layout {
            self.layout();

            let layout_mode = self.doc().settings().layout_mode;
            let page_width = match layout_mode {
                LayoutMode::Paginated { page_width, .. } => page_width.ceil(),
                LayoutMode::Continuous { max_width } => self
                    .width
                    .min(max_width + 2.0 * CONTINUOUS_PAGE_MARGIN)
                    .ceil(),
            };

            let page_heights: Vec<f32> = match layout_mode {
                LayoutMode::Paginated { page_height, .. } => {
                    vec![page_height.ceil(); self.pages.len()]
                }
                LayoutMode::Continuous { .. } => self
                    .pages
                    .iter()
                    .map(|p| p.root.node.size.height.ceil())
                    .collect(),
            };

            cmds.push(Cmd::LayoutChanged {
                page_count: self.pages.len(),
                layout_mode,
                page_width,
                page_heights,
            });
            self.pending.layout = false;
        }

        if self.pending.cursor {
            let selection = self.selection();
            let ctx = NavigationContext::new(&self.state.doc);
            let (page_idx, bounds, show) = Cursor::bounds(&ctx, &self.pages, selection.head)
                .map(|(idx, rect)| (Some(idx), Some(rect), selection.is_collapsed()))
                .unwrap_or((None, None, false));

            cmds.push(Cmd::CursorChanged {
                page_idx,
                bounds,
                show,
                scroll_to_cursor: self.pending.scroll_to_cursor,
            });
            self.pending.cursor = false;
            self.pending.scroll_to_cursor = false;
        }

        if self.pending.selection {
            let snapshot = self.selection_snapshot_owned();
            let stats = snapshot
                .as_ref()
                .map(|s| s.stats.clone())
                .unwrap_or_default();

            cmds.push(Cmd::SelectionChanged {
                stats,
                collapsed: self.selection().is_collapsed(),
            });
            self.pending.selection = false;
        }

        if self.pending.active_marks {
            let snapshot = self.selection_snapshot_owned();
            let (uniform_marks, mixed_marks) = self.collect_selection_marks(snapshot);

            cmds.push(Cmd::ActiveMarksChanged {
                uniform_marks,
                mixed_marks,
            });
            self.pending.active_marks = false;
        }

        if self.pending.external_elements {
            let elements = self.build_external_elements();
            cmds.push(Cmd::ExternalElementChanged { elements });
            self.pending.external_elements = false;
        }

        if let Some(style) = self.pending.pointer_style.take() {
            cmds.push(Cmd::PointerStyleChanged { style });
        }

        let fonts = std::mem::take(&mut self.pending.fonts);
        if !fonts.is_empty() {
            cmds.push(Cmd::FontsRequired { fonts });
        }

        let systems = std::mem::take(&mut self.pending.writing_systems);
        if !systems.is_empty() {
            cmds.push(Cmd::WritingSystemRequired { systems });
        }

        if cmds.iter().any(|c| matches!(c, Cmd::LayoutChanged { .. })) {
            cmds.push(Cmd::RenderRequired);
        }

        if self.pending.enabled_actions {
            let enabled = self.evaluate_enabled_actions();
            cmds.push(Cmd::EnabledActionsChanged { enabled });
            self.pending.enabled_actions = false;
        }

        if self.pending.exited_document_start {
            cmds.push(Cmd::ExitedDocumentStart);
            self.pending.exited_document_start = false;
        }

        if self.pending.pointer_mode_changed {
            cmds.push(Cmd::PointerModeChanged {
                is_idle: self.pointer.mode.is_idle(),
            });
            self.pending.pointer_mode_changed = false;
        }

        cmds
    }

    fn evaluate_enabled_actions(&self) -> Vec<String> {
        let ctx = Context::new(&self.state, &self.undo_manager);
        tracked_actions_with_when()
            .into_iter()
            .filter(|(_, when)| when.evaluate(&ctx))
            .map(|(name, _)| {
                // PascalCase -> camelCase
                let mut chars = name.chars();
                match chars.next() {
                    Some(first) => first.to_lowercase().chain(chars).collect(),
                    None => String::new(),
                }
            })
            .collect()
    }

    fn build_external_elements(&mut self) -> Vec<ExternalElement> {
        let mut elements = Vec::new();
        let selected_nodes = self
            .selection_snapshot_owned()
            .map(|snapshot| snapshot.block_set)
            .unwrap_or_default();

        for (page_idx, p) in self.pages.iter().enumerate() {
            for (pos, ext) in p.external_elements() {
                let data = match &ext.data {
                    crate::layout::elements::ExternalElementData::Image {
                        src,
                        original_width,
                        original_height,
                    } => crate::layout::elements::ExternalElementData::Image {
                        src: src.clone(),
                        original_width: *original_width,
                        original_height: *original_height,
                    },
                };

                elements.push(ExternalElement {
                    page_idx,
                    node_id: ext.id.to_string(),
                    bounds: Rect {
                        x: pos.x,
                        y: pos.y,
                        width: ext.size.width,
                        height: ext.size.height,
                    },
                    data,
                    is_selected: selected_nodes.contains(&ext.id),
                });
            }
        }

        elements
    }

    pub fn inspect_state(&self) -> String {
        crate::inspect::inspect_state(&self.doc(), &self.state.selection)
    }

    pub fn inspect_state_as_macro(&self) -> String {
        crate::inspect::inspect_state_as_macro(&self.doc(), &self.state.selection)
    }

    pub fn inspect_fragment_as_macro(&self, fragment: &Fragment) -> String {
        crate::inspect::inspect_fragment_as_macro(fragment)
    }

    pub fn inspect_page_element(&self, page_idx: usize, x: f32, y: f32) -> Option<String> {
        let Some(page) = self.pages.get(page_idx) else {
            return None;
        };

        inspect_page_element(page, x, y)
    }

    pub fn inspect_selection_as_fragment_macro(&self) -> Option<String> {
        let Ok(fragment) = Fragment::new_from_selection(self.doc(), self.selection()) else {
            return None;
        };
        Some(self.inspect_fragment_as_macro(&fragment))
    }

    pub(crate) fn is_position_in_selection(&self, position: Position) -> bool {
        let selection = self.state.selection;
        !selection.is_collapsed() && position_in_selection(&self.state.doc, position, &selection)
    }

    pub(crate) fn is_block_selectable_hit(&self, hit_selection: &Selection) -> bool {
        if hit_selection.is_collapsed() {
            return false;
        }

        let anchor = hit_selection.anchor;
        let Some(parent) = self.state.doc.node(anchor.node_id) else {
            return false;
        };

        let Some((child_id, _)) = find_child_at_offset(&parent, anchor.offset) else {
            return false;
        };

        self.state
            .doc
            .node(child_id)
            .map(|child| child.spec().selectable)
            .unwrap_or(false)
    }

    pub fn update(&mut self, msg: Message) {
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn process_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::FontUsageChanged { family, weight } => {
                    let font = (family.clone(), weight);
                    if !self.loaded_fonts.contains(&font) {
                        self.loaded_fonts.insert(font.clone());
                        self.pending.fonts.push(font);
                    }
                }
                Effect::WritingSystemsUsageChanged { systems: scripts } => {
                    for s in scripts {
                        if !self.loaded_scripts.contains(&s) {
                            self.loaded_scripts.insert(s);
                            self.pending.writing_systems.push(s);
                        }
                    }
                }
                Effect::DocChanged => {
                    self.selection_cache = None;
                    self.pending.doc = true;
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.scroll_to_cursor = true;
                    self.pending.selection = true;
                    self.pending.active_marks = true;
                    self.pending.external_elements = true;
                    self.pending.enabled_actions = true;
                }
                Effect::NodeChanged { node_id } => {
                    if let Some(node) = self.doc().node(node_id) {
                        let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                        self.layout_cache
                            .borrow_mut()
                            .invalidate_with_ancestors(node_id, ancestors.into_iter());
                    } else {
                        self.layout_cache.borrow_mut().invalidate(node_id);
                    }
                    self.selection_cache = None;
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_marks = true;
                    self.pending.external_elements = true;
                }
                Effect::SelectionChanged => {
                    self.selection_cache = None;
                    self.pending.cursor = true;
                    self.pending.scroll_to_cursor = true;
                    self.pending.selection = true;
                    self.pending.active_marks = true;
                    self.pending.render = true;
                    self.pending.external_elements = true;
                    self.pending.enabled_actions = true;
                }
                Effect::PendingMarksChanged => {
                    self.pending.active_marks = true;
                }
                Effect::ExternalElementChanged => {
                    self.pending.external_elements = true;
                }
                Effect::LayoutChanged => {
                    self.pending.layout = true;
                    self.pending.cursor = true;
                    self.pending.render = true;
                    self.pending.selection = true;
                    self.pending.active_marks = true;
                    self.pending.external_elements = true;
                }
                Effect::PointerStyleChanged { style } => {
                    self.pending.pointer_style = Some(style);
                }
                Effect::StructureChanged => {
                    // normalize_to_schema 에서 사용하기 위한 Effect라 아래 pending flags 설정은 그냥 형식적인 것임
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.external_elements = true;
                }
                Effect::PreeditChanged { node_id } => {
                    if let Some(nid) = node_id {
                        if let Some(node) = self.doc().node(nid) {
                            let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                            self.layout_cache
                                .borrow_mut()
                                .invalidate_with_ancestors(nid, ancestors.into_iter());
                        } else {
                            self.layout_cache.borrow_mut().invalidate(nid);
                        }
                    }

                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                }
                Effect::SettingsChanged => {
                    self.layout_cache.borrow_mut().invalidate_all();
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.settings = true;
                    self.pending.external_elements = true;
                }
                Effect::DropTargetChanged { target } => {
                    self.pending.drop_indicator = target.and_then(|position| {
                        let ctx = NavigationContext::new(&self.state.doc);
                        DropIndicator::from_position(&ctx, &self.pages, position)
                    });
                    self.pending.render = true;
                }
                Effect::ExitedDocumentStart => {
                    self.pending.exited_document_start = true;
                }
            }
        }
    }

    fn transact<F>(&mut self, f: F) -> Vec<Effect>
    where
        F: FnOnce(&mut Transaction) -> Result<bool>,
    {
        self.transact_internal(f, true)
    }

    #[allow(unused)]
    fn transact_immediate<F>(&mut self, f: F) -> Vec<Effect>
    where
        F: FnOnce(&mut Transaction) -> Result<bool>,
    {
        self.transact_internal(f, false)
    }

    fn transact_internal<F>(&mut self, f: F, defer_commit: bool) -> Vec<Effect>
    where
        F: FnOnce(&mut Transaction) -> Result<bool>,
    {
        let previous_selection = self.state.selection;

        let mut tr = Transaction::new(&self.state);

        match f(&mut tr) {
            Ok(true) => {}
            Ok(false) => {
                let _ = tr.rollback();
                return vec![];
            }
            Err(e) => {
                let _ = tr.rollback();
                error!("transaction error: {:?}", e);
                return vec![];
            }
        }

        let result = if defer_commit {
            tr.commit()
        } else {
            tr.commit_immediate()
        };

        match result {
            Ok((new_state, effects)) => {
                if effects.iter().any(|e| matches!(e, Effect::DocChanged)) {
                    self.undo_selections.push(previous_selection);
                    self.redo_selections.clear();
                }
                self.state = new_state;
                effects
            }
            Err(e) => {
                error!("commit failed: {:?}", e);
                vec![]
            }
        }
    }

    #[cfg(test)]
    pub fn is_layout_pending(&self) -> bool {
        self.pending.layout
    }

    #[cfg(test)]
    pub fn is_layout_cached(&self, node_id: NodeId) -> bool {
        self.layout_cache.borrow().get(node_id).is_some()
    }

    fn validate_selection(&self, selection: Selection) -> Selection {
        let anchor = self.validate_position(selection.anchor);
        let head = self.validate_position(selection.head);
        Selection::new(anchor, head)
    }

    fn validate_position(&self, position: Position) -> Position {
        if let Some(node) = self.doc().node(position.node_id) {
            let max_offset = self.calculate_max_offset(&node);
            let adjusted_offset = position.offset.min(max_offset);
            return Position::new(position.node_id, adjusted_offset, position.affinity);
        }

        self.find_nearest_valid_position()
    }

    fn calculate_max_offset(&self, node: &NodeRef) -> usize {
        let mut offset = 0;
        for child in node.children() {
            match child.node() {
                Node::Text(text) => offset += text.text.char_len(),
                _ => offset += 1,
            }
        }
        offset
    }

    fn find_nearest_valid_position(&self) -> Position {
        let root = self.doc().node(NodeId::ROOT).unwrap();

        for child in root.children() {
            if child.is_block() {
                return Position::new(child.node_id(), 0, Affinity::Downstream);
            }
        }

        Position::new(NodeId::ROOT, 0, Affinity::Downstream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Position;
    use crate::state::position_helpers::calculate_offset_before_child;
    use crate::types::Affinity;
    use std::rc::Rc;

    #[test]
    fn selection_across_empty_paragraphs_only_draws_first() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph { }
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let decorations = build_selection_decorations(&state.doc, &state.selection, None);

        assert_eq!(
            decorations.len(),
            1,
            "should only decorate first empty paragraph"
        );
        let decor = &decorations[0];
        assert_eq!(decor.node_id, p1);
        assert_eq!(decor.start_offset, 0);
        assert_eq!(
            decor.end_offset, 1,
            "empty paragraph should render minimal range"
        );
    }

    #[test]
    fn test_pointer_down_moves_cursor() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello World" }
                }
            }
            selection { (p, 11) }
        };

        runtime.layout();

        let target_offset = 5;
        let target_pos = Position::new(p, target_offset, Affinity::default());

        let ctx = NavigationContext::new(&runtime.state.doc);
        let (page_idx, rect) = Cursor::bounds(&ctx, &runtime.pages, target_pos.clone())
            .expect("Failed to get cursor bounds for target position");

        let click_x = rect.x;
        let click_y = rect.y + rect.height / 2.0;

        runtime.update(Message::PointerDown {
            page_idx,
            x: click_x,
            y: click_y,
            click_count: 1,
            modifier: Modifier::default(),
            button: PointerButton::Primary,
        });

        let selection = runtime.state().selection.clone();
        assert_eq!(selection.is_collapsed(), true);
        assert_eq!(selection.head.node_id, p);
        assert_eq!(selection.head.offset, target_offset);
    }

    #[test]
    fn test_double_click_at_line_end_after_hard_break_stays_on_same_line() {
        let mut p = id!();
        let mut second = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Top" }
                    hard_break {}
                    @second text { "Bottom" }
                }
            }
            selection { (p, 0) }
        };

        runtime.layout();

        let state = runtime.state();
        let paragraph = state.doc.node(p).unwrap();
        let second_node = state.doc.node(second).unwrap();
        let Node::Text(second_text) = second_node.node() else {
            panic!("Second node is not text");
        };

        let start_of_second = calculate_offset_before_child(&paragraph, second);
        let end_of_second = start_of_second + second_text.text.char_len();

        let target_pos = Position::new(p, end_of_second, Affinity::default());
        let ctx = NavigationContext::new(&runtime.state.doc);
        let (page_idx, rect) = Cursor::bounds(&ctx, &runtime.pages, target_pos.clone())
            .expect("Failed to get cursor bounds for target position");

        let click_x = rect.x;
        let click_y = rect.y + rect.height / 2.0;

        runtime.update(Message::PointerDown {
            page_idx,
            x: click_x,
            y: click_y,
            click_count: 2,
            modifier: Modifier::default(),
            button: PointerButton::Primary,
        });

        let state = runtime.state();
        let selection = state.selection.clone();
        let (from, to) = selection.as_sorted(&state.doc).unwrap();

        assert_eq!(from.node_id, p);
        assert_eq!(to.node_id, p);
        assert_eq!(from.offset, start_of_second);
        assert_eq!(to.offset, end_of_second);
    }

    #[test]
    fn test_insert_newline_selecting_range() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "asdf" }
                }
            }
            selection { (p, 1) -> (p, 4) }
        };

        runtime.update(Message::InsertNewline);

        let actual = runtime.state();
        let expected = state! {
            doc {
                paragraph {
                    text { "a" }
                }
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn test_delete_backward_joins_paragraph_with_leading_hard_break() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { }
                @p2 paragraph {
                    hard_break { }
                }
            }
            selection { (p2, 0, Affinity::Downstream) }
        };

        runtime.update(Message::DeleteBackward);

        let actual = runtime.state();
        let expected = state! {
            doc {
                @p1 paragraph {
                    hard_break { }
                }
            }
            selection { (p1, 0, Affinity::Downstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn regression_image_selection_after_newline() {
        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                image(src: "http://example.com/image.png".to_string(), width: 100.0, height: 100.0,)
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1) }
        };

        let doc = rt.doc();
        let root = doc.node(NodeId::ROOT).unwrap();
        assert_eq!(root.children().count(), 2);

        rt.layout();

        rt.update(Message::InsertNewline);

        assert!(
            rt.is_layout_pending(),
            "Layout should be pending after InsertNewline"
        );
        rt.layout();

        let root = rt.doc().node(NodeId::ROOT).unwrap();
        assert_eq!(root.children().count(), 3);

        let image_pos = Position::new(NodeId::ROOT, 1, Affinity::default());
        let ctx = NavigationContext::new(rt.doc());
        let (page_idx, rect) =
            Cursor::bounds(&ctx, &rt.pages(), image_pos).expect("Failed to get image bounds");

        let click_x = rect.x + rect.width / 2.0;
        let click_y = rect.y + rect.height / 2.0;

        rt.update(Message::PointerDown {
            page_idx,
            x: click_x,
            y: click_y,
            click_count: 1,
            modifier: Modifier::default(),
            button: PointerButton::Primary,
        });

        let sel = rt.state().selection;
        let expected_sel = Selection::new(
            Position::new(NodeId::ROOT, 1, Affinity::default()),
            Position::new(NodeId::ROOT, 2, Affinity::default()),
        );

        assert_eq!(
            sel, expected_sel,
            "Should select the image after clicking it"
        );
    }

    #[test]
    fn regression_delete_forward_invalidates_list_layout_after_lift() {
        let mut p = id!();
        let mut list = id!();
        let mut second_item = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
                @list ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                    @second_item list_item {
                        paragraph {
                            text { "2" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(list),
            "precondition: list layout should be cached"
        );

        rt.update(Message::DeleteForward);

        assert!(
            !rt.is_layout_cached(list),
            "list layout cache should be invalidated after DeleteForward"
        );

        let expected = state! {
            doc {
                @p paragraph {
                    text { "1" }
                }
                ordered_list {
                    list_item {
                        paragraph {
                            text { "2" }
                        }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn regression_indent_invalidates_list_layout_after_sink() {
        let mut item = id!();
        let mut n1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                    @item list_item {
                        @n1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        rt.layout();

        assert!(
            rt.is_layout_cached(item),
            "precondition: list list layout should be cached"
        );

        rt.update(Message::Indent);

        assert!(
            !rt.is_layout_cached(item),
            "list item layout cache should be invalidated after Indent"
        );

        let expected = state! {
            doc {
                ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                        ordered_list {
                            list_item {
                                @n1 paragraph {}
                            }
                        }
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        assert_state_eq!(rt.state(), expected);
    }

    #[test]
    fn regression_indent_recomputes_nested_list_layout() {
        let mut item = id!();
        let mut n1 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                ordered_list {
                    list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                    @item list_item {
                        @n1 paragraph {}
                    }
                }
                paragraph {}
            }
            selection { (n1, 0) }
        };

        rt.layout();

        rt.update(Message::Indent);

        // 중첩 리스트가 새로 생겼는지 확인하고 레이아웃 캐시가 채워지는지 본다.
        let doc = rt.doc();
        let root = doc.node(NodeId::ROOT).expect("root not found");
        let outer_list = root.first_child().expect("root list not found");
        let first_item = outer_list.first_child().expect("first list item not found");
        let nested_list_id = first_item
            .children()
            .find(|c| matches!(c.node(), Node::OrderedList(_)))
            .map(|c| c.node_id())
            .expect("nested list not created");

        rt.layout();

        assert!(
            rt.is_layout_cached(nested_list_id),
            "nested list should have layout cached after layout recomputation"
        );
    }

    #[test]
    fn regression_list_toggle_alignment_bug() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 0) }
        };

        runtime.layout();

        runtime
            .doc()
            .update_settings(|s| s.paragraph_indent = 1.0)
            .unwrap();
        runtime.layout();

        let p_layout_initial = runtime
            .layout_cache
            .borrow()
            .get(p)
            .expect("Paragraph layout should be cached");
        assert!(runtime.is_layout_cached(p));

        let effects = runtime.transact(|tr| tr.toggle_bullet_list());
        runtime.process_effects(effects);

        let is_cached = runtime.is_layout_cached(p);
        assert!(
            !is_cached,
            "Paragraph layout should be invalidated after toggle"
        );

        runtime.layout();

        let p_layout_after = runtime
            .layout_cache
            .borrow()
            .get(p)
            .expect("Paragraph layout should be cached");

        assert!(
            !Rc::ptr_eq(&p_layout_initial, &p_layout_after),
            "Layout should be recomputed (fix verified)"
        );
    }

    #[test]
    fn get_marks_at_position_filters_expand_none_at_end() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(marks: [ruby("루비")]) { "베이스" }
                    text { "일반" }
                }
            }
            selection { (p, 3) }
        };

        runtime.layout();
        let marks = runtime.get_marks_at_position(&runtime.state().selection.head);

        assert!(
            !marks.iter().any(|m| matches!(m, Mark::Ruby(_))),
            "Ruby mark should not be returned at segment end (Expand::None)"
        );
    }

    #[test]
    fn get_marks_at_position_filters_expand_none_at_start() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(marks: [ruby("루비")]) { "베이스" }
                }
            }
            selection { (p, 0) }
        };

        runtime.layout();
        let marks = runtime.get_marks_at_position(&runtime.state().selection.head);

        assert!(
            !marks.iter().any(|m| matches!(m, Mark::Ruby(_))),
            "Ruby mark should not be returned at segment start (Expand::None)"
        );
    }

    #[test]
    fn get_marks_at_position_returns_expand_after_at_end() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert(FontFamilyMark::default().family, vec![400, 700]);
        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "aa" => [font_weight(700)], "bb" }
                }
            }
            selection { (p, 2) }
        };

        runtime.layout();
        let marks = runtime.get_marks_at_position(&runtime.state().selection.head);

        assert!(
            marks.iter().any(|m| matches!(m, Mark::FontWeight(_))),
            "FontWeight mark should be returned at segment end (Expand::After)"
        );
    }

    #[test]
    fn get_marks_at_position_returns_all_marks_in_middle() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(marks: [ruby("루비")]) { "베이스텍스트" }
                }
            }
            selection { (p, 3) }
        };

        runtime.layout();
        let marks = runtime.get_marks_at_position(&runtime.state().selection.head);

        assert!(
            marks.iter().any(|m| matches!(m, Mark::Ruby(_))),
            "Ruby mark should be returned in segment middle"
        );
    }
}

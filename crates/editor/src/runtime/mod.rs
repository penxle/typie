mod cmd;
mod context;
mod dnd;
mod effect;
mod handlers;
mod message;
mod pointer;
pub mod search;
pub mod slate;
mod state;
mod table;
pub mod text_replacement;
pub mod tracked_items;
mod view_state;

use crate::inspect::{
    inspect_fragment_as_macro, inspect_page_element, inspect_state, inspect_state_as_macro,
};
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::elements::ExternalElementData;
use crate::layout::query::{find_drag_image_bounds, is_selectable_block_hit};
use crate::layout::{LayoutCache, LayoutContext, Page, Paginator};
use crate::model::*;
use crate::render::{DragImageResult, RenderInfo, RenderResult, Renderer};
use crate::state::ancestor_helpers::lowest_common_ancestor_id;
use crate::state::selection_helpers::{
    SelectionAttributes, build_selection_decorations, collect_block_attrs_at,
    collect_selected_block_ids, compute_selection_attrs, compute_structure_selection,
};
use crate::state::{
    Position, Preedit, Selection, find_child_at_offset, find_text_at_offset, position_in_selection,
};
use crate::transaction::Transaction;
use crate::types::{Affinity, BoxConstraints, PointerStyle, Rect, Size};
use anyhow::Result;
pub use cmd::*;
pub use context::*;
pub use dnd::*;
pub use effect::*;
use loro::UndoManager;
pub use message::*;
pub use pointer::*;
use rustc_hash::{FxHashMap, FxHashSet};
use slate::*;
pub use state::*;
use std::cell::RefCell;
pub use text_replacement::RawTextReplacementRule;
pub use tracked_items::{RawTrackedItem, TrackedItem};
pub use view_state::*;

fn get_styles_at_offset(text_node: &loro::LoroText, offset: usize) -> Vec<Style> {
    let rich_value = text_node.get_richtext_value();
    let mut styles = Vec::new();
    if let loro::LoroValue::List(list) = rich_value {
        let mut current_offset = 0;
        for item in list.iter() {
            if let loro::LoroValue::Map(map) = item {
                let text = map
                    .get("insert")
                    .and_then(|v| v.as_string())
                    .cloned()
                    .unwrap_or_default();
                let segment_len = text.chars().count();
                let segment_end = current_offset + segment_len;

                if offset >= current_offset && offset < segment_end {
                    if let Some(attrs_value) = map.get("attributes") {
                        if let loro::LoroValue::Map(attrs) = attrs_value {
                            for (key, value) in attrs.iter() {
                                if let Some(style) = Style::from_key_value(key, value.clone()) {
                                    styles.push(style);
                                }
                            }
                        }
                    }
                    break;
                }

                current_offset = segment_end;
            }
        }
    }
    styles
}

fn reapply_styles(text_node: &loro::LoroText, offset: usize, len: usize, styles: &[Style]) {
    for style in styles {
        let key = style.key();
        let value = style.to_loro_value();
        let _ = text_node.mark(offset..offset + len, key, value);
    }
}

#[derive(Default)]
struct PendingUpdates {
    doc: bool,
    selection: bool,
    active_styles: bool,
    cursor: bool,
    settings: bool,
    layout: bool,
    external_elements: bool,
    pointer_style: Option<PointerStyle>,
    fonts: FxHashMap<(String, u16), Vec<u32>>,
    codepoints: Vec<u32>,
    render: bool,
    drop_indicator: Option<DropIndicator>,
    enabled_actions: bool,
    exited_document_start: bool,
    pointer_mode_changed: bool,
    placeholder: bool,
    link_overlays: bool,
    tracked_items: bool,
    table_overlays: bool,
    html_pasted: Option<(String, Position, Position)>,
}

#[derive(Clone)]
struct SelectionSnapshot {
    from: Position,
    to: Position,
    block_ids: Vec<NodeId>,
    block_set: FxHashSet<NodeId>,
    attrs: SelectionAttributes,
}

pub struct Runtime {
    viewport_width: f32,
    viewport_height: f32,
    width: f32,
    scale_factor: f64,
    renderer: Renderer,
    pages: Vec<Page>,

    state: State,
    undo_manager: UndoManager,
    layout_cache: RefCell<LayoutCache>,
    view_states: ViewStates,

    loaded_font_codepoints: FxHashMap<(String, u16), FxHashSet<u32>>,
    loaded_codepoints: FxHashSet<u32>,

    selection_cache: Option<SelectionSnapshot>,
    pending: PendingUpdates,
    message_queue: Vec<Message>,
    pointer: PointerState,

    pub slate: Slate,
    pub slab: Slab,

    undo_selections: Vec<Selection>,
    redo_selections: Vec<Selection>,

    cached_plain_text: Option<String>,

    auto_surround_enabled: bool,
    tracked_items: Vec<TrackedItem>,
    is_focused: bool,
    last_table_overlays: Vec<TableOverlay>,
    text_replacement_undo: Option<text_replacement::ReplacementUndoState>,
}

impl Runtime {
    pub fn new(width: f32, scale_factor: f64, state: State) -> Self {
        let mut undo_manager = UndoManager::new(&state.doc.loro_doc());
        undo_manager.set_merge_interval(1000);

        Self {
            viewport_width: width,
            viewport_height: 0.0,
            width,
            scale_factor,
            renderer: Renderer::new(scale_factor),
            state,
            pages: Vec::new(),
            undo_manager,
            layout_cache: RefCell::new(LayoutCache::new()),
            view_states: ViewStates::default(),
            loaded_font_codepoints: FxHashMap::default(),
            loaded_codepoints: FxHashSet::default(),
            selection_cache: None,
            pending: PendingUpdates {
                doc: true,
                cursor: true,
                selection: true,
                active_styles: true,
                settings: true,
                layout: true,
                external_elements: true,
                pointer_style: None,
                fonts: FxHashMap::default(),
                codepoints: Vec::new(),
                render: true,
                drop_indicator: None,
                enabled_actions: true,
                exited_document_start: false,
                pointer_mode_changed: true,
                placeholder: true,
                link_overlays: false,
                tracked_items: false,
                table_overlays: true,
                html_pasted: None,
            },
            message_queue: Vec::new(),
            pointer: PointerState::default(),
            slate: Slate::default(),
            slab: Slab::new(),
            undo_selections: Vec::new(),
            redo_selections: Vec::new(),
            cached_plain_text: None,
            auto_surround_enabled: true,
            tracked_items: Vec::new(),
            is_focused: true,
            last_table_overlays: Vec::new(),
            text_replacement_undo: None,
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

    pub fn set_read_only(&mut self, read_only: bool) {
        if self.state.read_only != read_only {
            self.state.read_only = read_only;
            self.pending.enabled_actions = true;
        }
    }

    pub fn is_read_only(&self) -> bool {
        self.state.read_only
    }

    pub fn set_auto_surround_enabled(&mut self, enabled: bool) {
        self.auto_surround_enabled = enabled;
    }

    #[allow(dead_code)]
    pub fn is_auto_surround_enabled(&self) -> bool {
        self.auto_surround_enabled
    }

    pub fn import_updates(&mut self, updates: &[u8]) -> Result<()> {
        let old_frontiers = self.state.doc.frontiers();
        self.state.doc.import_updates(updates)?;
        let new_frontiers = self.state.doc.frontiers();

        if old_frontiers != new_frontiers {
            self.state.frontiers = new_frontiers;
            self.handle_external_doc_change();
        }
        Ok(())
    }

    pub fn import_updates_batch(&mut self, updates_batch: &[Vec<u8>]) -> Result<()> {
        let old_frontiers = self.state.doc.frontiers();
        self.state.doc.import_updates_batch(updates_batch)?;
        let new_frontiers = self.state.doc.frontiers();

        if old_frontiers != new_frontiers {
            self.state.frontiers = new_frontiers;
            self.handle_external_doc_change();
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn export(&self, mode: DocExportMode) -> Result<Vec<u8>> {
        self.state.doc.export(mode)
    }

    pub fn checkout(&mut self, version: &[u8]) -> Result<()> {
        let vv = loro::VersionVector::decode(version)?;
        if !self.state.doc.loro_doc().oplog_vv().includes_vv(&vv) {
            anyhow::bail!("Cannot checkout to unknown version");
        }
        let frontiers = self.state.doc.loro_doc().vv_to_frontiers(&vv);
        self.state.doc.checkout(&frontiers)?;
        self.handle_external_doc_change();
        Ok(())
    }

    pub fn checkout_to_latest(&mut self) -> Result<()> {
        self.state.doc.checkout_to_latest()?;
        self.handle_external_doc_change();
        Ok(())
    }

    pub fn is_detached(&self) -> bool {
        self.state.doc.is_detached()
    }

    pub fn revert_to(&mut self, version: &[u8]) -> Result<()> {
        let vv = loro::VersionVector::decode(version)?;
        if !self.state.doc.loro_doc().oplog_vv().includes_vv(&vv) {
            anyhow::bail!("Cannot revert to unknown version");
        }
        let frontiers = self.state.doc.loro_doc().vv_to_frontiers(&vv);
        self.state.doc.revert_to(&frontiers)?;
        self.handle_external_doc_change();
        Ok(())
    }

    pub fn insert_template_fragment(&mut self, snapshot: Vec<u8>) -> Result<()> {
        let template_doc = std::rc::Rc::new(Doc::from_snapshot(snapshot));
        let template_settings = template_doc.settings();
        let fragment = Fragment::from_doc(&template_doc)?;

        let effects = self.transact(|tr| {
            tr.doc().update_settings(|s| {
                s.block_gap = template_settings.block_gap;
                s.paragraph_indent = template_settings.paragraph_indent;
                s.layout_mode = template_settings.layout_mode;
            })?;

            tr.delete_selection()?;
            tr.paste_fragment(fragment, None)?;

            tr.push_effect(Effect::SettingsChanged);
            Ok(true)
        });
        self.process_effects(effects);
        Ok(())
    }

    fn handle_external_doc_change(&mut self) {
        // TODO: 최적화?
        self.state.doc.clear_children_cache();
        self.layout_cache.borrow_mut().invalidate_all();
        self.selection_cache = None;
        self.cached_plain_text = None;
        self.text_replacement_undo = None;
        self.pending.layout = true;
        self.pending.render = true;
        self.pending.selection = true;
        self.pending.active_styles = true;
        self.pending.cursor = true;
        self.pending.external_elements = true;
        self.pending.settings = true;
        self.pending.enabled_actions = true;
    }

    pub fn replace_text_in_block(
        &mut self,
        block_id: NodeId,
        start_offset: usize,
        end_offset: usize,
        replacement: &str,
    ) -> Result<()> {
        use anyhow::Context;
        let doc_handle = self.state.doc.clone();

        let node = doc_handle
            .node(block_id)
            .context(format!("Failed to find block node: {:?}", block_id))?;

        let (start_child_id, start_internal, _) =
            find_text_at_offset(&doc_handle, &node, start_offset).context(format!(
                "Failed to find text at start offset: {} in block {:?}",
                start_offset, block_id
            ))?;

        let (end_child_id, _, _) =
            find_text_at_offset(&doc_handle, &node, end_offset).context(format!(
                "Failed to find text at end offset: {} in block {:?}",
                end_offset, block_id
            ))?;

        if start_child_id != end_child_id {
            anyhow::bail!(
                "Replacement across different text nodes is not supported: {:?} vs {:?}",
                start_child_id,
                end_child_id
            );
        }

        let (_, _, text_node) =
            find_text_at_offset(&doc_handle, &node, start_offset).context(format!(
                "Failed to re-find text node at start offset: {} (this should not happen)",
                start_offset
            ))?;

        let end_internal_offset = end_offset - start_offset + start_internal;

        let styles = get_styles_at_offset(&text_node, start_internal);

        let _ = text_node.splice(
            start_internal,
            end_internal_offset - start_internal,
            replacement,
        );

        let replacement_len = replacement.chars().count();
        if replacement_len > 0 {
            reapply_styles(&text_node, start_internal, replacement_len, &styles);
        }

        self.handle_external_doc_change();

        Ok(())
    }

    pub fn replace_text_in_blocks(
        &mut self,
        replacements: &[(NodeId, usize, usize, &str)],
    ) -> Result<()> {
        use anyhow::Context;

        let mut indices: Vec<usize> = (0..replacements.len()).collect();
        indices.sort_by(|&a, &b| {
            let ra = &replacements[a];
            let rb = &replacements[b];
            match ra.0.cmp(&rb.0) {
                std::cmp::Ordering::Equal => rb.1.cmp(&ra.1),
                other => other,
            }
        });

        let doc_handle = self.state.doc.clone();

        for i in indices {
            let (block_id, start_offset, end_offset, replacement) = replacements[i];

            let node = doc_handle
                .node(block_id)
                .context(format!("Failed to find block node: {:?}", block_id))?;

            let (start_child_id, start_internal, _) =
                find_text_at_offset(&doc_handle, &node, start_offset).context(format!(
                    "Failed to find text at start offset: {} in block {:?}",
                    start_offset, block_id
                ))?;

            let (end_child_id, _, _) = find_text_at_offset(&doc_handle, &node, end_offset)
                .context(format!(
                    "Failed to find text at end offset: {} in block {:?}",
                    end_offset, block_id
                ))?;

            if start_child_id != end_child_id {
                continue;
            }

            let (_, _, text_node) =
                find_text_at_offset(&doc_handle, &node, start_offset).context(format!(
                    "Failed to re-find text node at start offset: {}",
                    start_offset
                ))?;

            let end_internal_offset = end_offset - start_offset + start_internal;

            let styles = get_styles_at_offset(&text_node, start_internal);

            let _ = text_node.splice(
                start_internal,
                end_internal_offset - start_internal,
                replacement,
            );

            let replacement_len = replacement.chars().count();
            if replacement_len > 0 {
                reapply_styles(&text_node, start_internal, replacement_len, &styles);
            }
        }

        self.handle_external_doc_change();

        Ok(())
    }

    pub fn get_cached_plain_text(&mut self) -> String {
        if let Some(ref cached) = self.cached_plain_text {
            return cached.clone();
        }

        let text = self.state.doc.to_plain_text();
        self.cached_plain_text = Some(text.clone());
        text
    }

    pub fn selection(&self) -> &Selection {
        &self.state.selection
    }

    pub fn preedit(&self) -> Option<&Preedit> {
        self.state.preedit.as_ref()
    }

    fn selection_snapshot(&mut self) -> Option<SelectionSnapshot> {
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
            let cell_selection = compute_structure_selection(self.doc(), selection);
            let block_ids = collect_selected_block_ids(self.doc(), selection, &cell_selection);

            let mut block_set = FxHashSet::default();
            block_set.extend(block_ids.iter().copied());

            let attrs = compute_selection_attrs(self.doc(), &block_ids, from.clone(), to.clone());

            self.selection_cache = Some(SelectionSnapshot {
                from: from.clone(),
                to: to.clone(),
                block_ids,
                block_set,
                attrs,
            });
        }

        self.selection_cache.clone()
    }

    fn selection_common_ancestor_ids(&self, selection: Selection) -> Vec<NodeId> {
        let Some(lca_id) =
            lowest_common_ancestor_id(self.doc(), selection.anchor.node_id, selection.head.node_id)
        else {
            return Vec::new();
        };

        self.doc()
            .node(lca_id)
            .map(|node| {
                node.ancestors()
                    .map(|ancestor| ancestor.node_id())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn selection_type(node_type: NodeType) -> u32 {
        match node_type {
            NodeType::HorizontalRule => SELECTION_TYPE_HORIZONTAL_RULE,
            NodeType::Callout => SELECTION_TYPE_CALLOUT,
            NodeType::Fold => SELECTION_TYPE_FOLD,
            NodeType::BulletList => SELECTION_TYPE_BULLET_LIST,
            NodeType::OrderedList => SELECTION_TYPE_ORDERED_LIST,
            NodeType::Image => SELECTION_TYPE_IMAGE,
            NodeType::File => SELECTION_TYPE_FILE,
            NodeType::Embed => SELECTION_TYPE_EMBED,
            NodeType::Archived => SELECTION_TYPE_ARCHIVED,
            NodeType::Blockquote => SELECTION_TYPE_BLOCKQUOTE,
            _ => SELECTION_TYPE_NONE,
        }
    }

    fn selection_type_for_id(&self, node_id: NodeId) -> u32 {
        self.doc()
            .node(node_id)
            .map(|node| Self::selection_type(node.node_type()))
            .unwrap_or(SELECTION_TYPE_NONE)
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
            }),
            pending_styles: PendingStylesDecor {
                node_id: self.state.selection.head.node_id,
                styles: self.state.pending_styles.clone(),
            },
        };

        let root_ref = self.doc().node(NodeId::ROOT).unwrap();
        let default_styles = self.doc().default_styles();
        let ctx = LayoutContext::new(
            &root_ref,
            &settings,
            &default_styles,
            &decorations,
            self.scale_factor,
            &self.view_states,
            &self.layout_cache,
        );

        let root_layout = ctx.layout(&root_ref, constraints);

        self.layout_cache.borrow_mut().clear_prev();

        let paginator = Paginator::new(
            page_width,
            page_height,
            margin_top,
            margin_bottom,
            margin_left,
            settings.layout_mode,
        );
        self.pages = paginator.paginate_rc(root_layout);
    }

    pub fn render_page(&mut self, page_index: usize) -> Option<RenderResult> {
        let snapshot = self.selection_snapshot();

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
        self.renderer.set_focused(self.is_focused);

        let drop_indicator = self.pending.drop_indicator.as_ref();

        Some(
            self.renderer
                .render(page, page_index, &selections, drop_indicator, doc),
        )
    }

    #[allow(dead_code)]
    pub fn get_render_info(&mut self, page_index: usize) -> Option<RenderInfo> {
        let doc = &self.state.doc;
        let page = self.pages.get(page_index)?;

        let layout_mode = doc.settings().layout_mode;
        let page_height = match layout_mode {
            LayoutMode::Paginated { page_height, .. } => page_height.ceil(),
            LayoutMode::Continuous { .. } => page.root.node.size.height.ceil(),
        };

        self.renderer
            .set_size(self.width.ceil(), page_height, self.scale_factor);

        let width = self.renderer.width();
        let height = self.renderer.height();
        let buffer_size = width as usize * height as usize * 4;

        Some(RenderInfo {
            width,
            height,
            buffer_size,
        })
    }

    #[allow(dead_code)]
    pub fn render_page_to(&mut self, page_index: usize, dst: &mut [u8]) -> bool {
        let snapshot = self.selection_snapshot();

        let doc = &self.state.doc;
        let selection = &self.state.selection;

        let selections = if let Some(snapshot) = snapshot.as_ref() {
            build_selection_decorations(doc, selection, Some(&snapshot.block_ids))
        } else {
            build_selection_decorations(doc, selection, None)
        };

        let Some(page) = self.pages.get(page_index) else {
            return false;
        };

        let layout_mode = doc.settings().layout_mode;
        let page_height = match layout_mode {
            LayoutMode::Paginated { page_height, .. } => page_height.ceil(),
            LayoutMode::Continuous { .. } => page.root.node.size.height.ceil(),
        };

        self.renderer
            .set_size(self.width.ceil(), page_height, self.scale_factor);
        self.renderer.set_focused(self.is_focused);

        let drop_indicator = self.pending.drop_indicator.as_ref();

        self.renderer
            .render_to(page, page_index, &selections, drop_indicator, doc, dst)
    }

    pub fn render_drag_image(
        &mut self,
        visible_pages: &[usize],
        drag_page_idx: usize,
    ) -> Option<DragImageResult> {
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

    fn get_annotations_at_position(&self, position: &Position) -> Vec<Annotation> {
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

        let segments = text_node.text.get_segments();
        let mut current_offset = 0;

        for segment in segments {
            let segment_len = segment.text.chars().count();
            let segment_start = current_offset;
            let segment_end = current_offset + segment_len;

            let in_range = (local_offset > segment_start && local_offset <= segment_end)
                || (local_offset == 0 && segment_start == 0);

            if in_range {
                return segment.annotations.clone();
            }
            current_offset += segment_len;
        }

        Vec::new()
    }

    pub fn get_pointer_style(&self, page_idx: usize, x: f32, y: f32) -> PointerStyle {
        let Some(page) = self.pages.get(page_idx) else {
            return PointerStyle::Default;
        };

        page.get_pointer_style(x, y, self.is_read_only())
    }

    pub fn tick(&mut self) {
        self.slate.dirty = 0;
        self.slab.reset();
        let messages = std::mem::take(&mut self.message_queue);
        for msg in messages {
            self.process_message(msg);
        }
        self.build_output();
        self.slate.slab_len = self.slab.len() as u32;
        self.slate.slab_capacity = self.slab.data.capacity() as u32;
    }

    pub fn flush(&mut self) {
        if self.state.pending_loro_commit {
            self.state.doc.loro_doc().commit();
            self.state.pending_loro_commit = false;
        }
    }

    fn process_message(&mut self, msg: Message) {
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn node_id_to_bytes(id: NodeId) -> [u8; 16] {
        *id.as_uuid().as_bytes()
    }

    fn affinity_to_u32(a: Affinity) -> u32 {
        match a {
            Affinity::Upstream => 0,
            Affinity::Downstream => 1,
        }
    }

    fn pointer_style_to_u32(s: PointerStyle) -> u32 {
        match s {
            PointerStyle::Default => 0,
            PointerStyle::Text => 1,
            PointerStyle::Pointer => 2,
        }
    }

    fn build_output(&mut self) {
        let mut had_layout = false;

        if self.pending.doc {
            self.slate.dirty |= DIRTY_DOC_CHANGED;
            self.pending.doc = false;
        }

        if self.pending.render {
            self.slate.dirty |= DIRTY_RENDER_REQUIRED;
            self.pending.render = false;
        }

        if self.pending.settings {
            let settings = self.doc().settings();
            self.slate.paragraph_indent = settings.paragraph_indent;
            self.slate.block_gap = settings.block_gap;
            self.slate.dirty |= DIRTY_SETTINGS;
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

            let pages_data: Vec<f32> = page_heights.iter().flat_map(|&h| [page_width, h]).collect();
            let (off, cnt) = self.slab.write_f32_slice(&pages_data);
            self.slate.pages_offset = off;
            self.slate.pages_count = cnt / 2;

            let lm_start = self.slab.alloc(0, 4);
            match layout_mode {
                LayoutMode::Paginated {
                    page_width,
                    page_height,
                    page_margin_top,
                    page_margin_bottom,
                    page_margin_left,
                    page_margin_right,
                } => {
                    self.slab.write_u32_slice(&[0]);
                    self.slab.write_f32_slice(&[
                        page_width,
                        page_height,
                        page_margin_top,
                        page_margin_bottom,
                        page_margin_left,
                        page_margin_right,
                    ]);
                }
                LayoutMode::Continuous { max_width } => {
                    self.slab.write_u32_slice(&[1]);
                    self.slab.write_f32_slice(&[max_width]);
                }
            }
            self.slate.layout_mode_offset = lm_start;

            self.slate.dirty |= DIRTY_LAYOUT;
            self.pending.layout = false;
            had_layout = true;

            if self.state.read_only {
                self.pending.link_overlays = true;
            }

            if !self.tracked_items.is_empty() {
                self.pending.tracked_items = true;
            }

            self.pending.table_overlays = true;
        }

        if self.pending.cursor {
            let selection = self.state.selection;
            let ctx = NavigationContext::new(&self.state.doc);
            let (page_idx, bounds, visible) = Cursor::bounds(&ctx, &self.pages, selection.head)
                .map(|(idx, rect)| (Some(idx), Some(rect), selection.is_collapsed()))
                .unwrap_or((None, None, false));

            let preceding_char_widths = if selection.is_collapsed() {
                Cursor::preceding_char_widths(&ctx, &self.pages, selection.head, 64)
            } else {
                None
            };

            self.slate.cursor_page_idx = page_idx.map(|i| i as i32).unwrap_or(-1);
            if let Some(b) = bounds {
                self.slate.cursor_x = b.x;
                self.slate.cursor_y = b.y;
                self.slate.cursor_width = b.width;
                self.slate.cursor_height = b.height;
            } else {
                self.slate.cursor_x = 0.0;
                self.slate.cursor_y = 0.0;
                self.slate.cursor_width = 0.0;
                self.slate.cursor_height = 0.0;
            }
            self.slate.cursor_visible = visible as u32;

            if let Some(widths) = preceding_char_widths {
                let (off, cnt) = self.slab.write_f32_slice(&widths);
                self.slate.preceding_char_widths_offset = off;
                self.slate.preceding_char_widths_count = cnt;
            } else {
                self.slate.preceding_char_widths_offset = 0;
                self.slate.preceding_char_widths_count = 0;
            }

            self.slate.dirty |= DIRTY_CURSOR;
            self.pending.cursor = false;
        }

        if self.pending.selection {
            let selection = self.state.selection;
            let collapsed = selection.is_collapsed();

            let cmp = if collapsed {
                0
            } else {
                match selection.as_sorted(&self.state.doc) {
                    Ok((from, _to)) => {
                        if from == selection.anchor {
                            1
                        } else {
                            -1
                        }
                    }
                    Err(_) => 0,
                }
            };

            let ctx = NavigationContext::new(&self.state.doc);
            let anchor_handle =
                Cursor::selection_handle_bounds(&ctx, &self.pages, selection.anchor)
                    .map(|(page_idx, bounds)| SelectionHandleBounds { page_idx, bounds });
            let head_handle = Cursor::selection_handle_bounds(&ctx, &self.pages, selection.head)
                .map(|(page_idx, bounds)| SelectionHandleBounds { page_idx, bounds });
            let selected_block_ids = if collapsed {
                Vec::new()
            } else {
                self.selection_snapshot()
                    .map(|snapshot| snapshot.block_ids)
                    .unwrap_or_default()
            };
            let selected_block_types: Vec<u32> = selected_block_ids
                .iter()
                .map(|node_id| self.selection_type_for_id(*node_id))
                .collect();
            let common_ancestor_ids = self.selection_common_ancestor_ids(selection);
            let common_ancestor_types: Vec<u32> = common_ancestor_ids
                .iter()
                .map(|node_id| self.selection_type_for_id(*node_id))
                .collect();
            let (selected_block_ids_offset, selected_block_ids_count) =
                self.slab.write_node_id_slice(&selected_block_ids);
            let (selected_block_types_offset, selected_block_types_count) =
                self.slab.write_u32_slice(&selected_block_types);
            let (common_ancestor_ids_offset, common_ancestor_ids_count) =
                self.slab.write_node_id_slice(&common_ancestor_ids);
            let (common_ancestor_types_offset, common_ancestor_types_count) =
                self.slab.write_u32_slice(&common_ancestor_types);

            self.slate.selection_cmp = cmp;
            self.slate.selection_block_ids_offset = selected_block_ids_offset;
            self.slate.selection_block_ids_count = selected_block_ids_count;
            self.slate.selection_block_types_offset = selected_block_types_offset;
            self.slate.selection_block_types_count = selected_block_types_count;
            self.slate.selection_common_ancestor_ids_offset = common_ancestor_ids_offset;
            self.slate.selection_common_ancestor_ids_count = common_ancestor_ids_count;
            self.slate.selection_common_ancestor_types_offset = common_ancestor_types_offset;
            self.slate.selection_common_ancestor_types_count = common_ancestor_types_count;
            self.slate.selection_anchor_node_id = Self::node_id_to_bytes(selection.anchor.node_id);
            self.slate.selection_anchor_offset = selection.anchor.offset as u32;
            self.slate.selection_anchor_affinity = Self::affinity_to_u32(selection.anchor.affinity);
            self.slate.selection_head_node_id = Self::node_id_to_bytes(selection.head.node_id);
            self.slate.selection_head_offset = selection.head.offset as u32;
            self.slate.selection_head_affinity = Self::affinity_to_u32(selection.head.affinity);

            if let Some(h) = anchor_handle {
                self.slate.selection_anchor_page_idx = h.page_idx as i32;
                self.slate.selection_anchor_x = h.bounds.x;
                self.slate.selection_anchor_y = h.bounds.y;
                self.slate.selection_anchor_width = h.bounds.width;
                self.slate.selection_anchor_height = h.bounds.height;
            } else {
                self.slate.selection_anchor_page_idx = -1;
                self.slate.selection_anchor_x = 0.0;
                self.slate.selection_anchor_y = 0.0;
                self.slate.selection_anchor_width = 0.0;
                self.slate.selection_anchor_height = 0.0;
            }

            if let Some(h) = head_handle {
                self.slate.selection_head_page_idx = h.page_idx as i32;
                self.slate.selection_head_x = h.bounds.x;
                self.slate.selection_head_y = h.bounds.y;
                self.slate.selection_head_width = h.bounds.width;
                self.slate.selection_head_height = h.bounds.height;
            } else {
                self.slate.selection_head_page_idx = -1;
                self.slate.selection_head_x = 0.0;
                self.slate.selection_head_y = 0.0;
                self.slate.selection_head_width = 0.0;
                self.slate.selection_head_height = 0.0;
            }

            self.slate.dirty |= DIRTY_SELECTION;
            self.pending.selection = false;
        }

        if self.pending.active_styles {
            let selection = self.state.selection;

            if selection.is_collapsed() {
                let block_attrs = collect_block_attrs_at(self.doc(), selection.head.node_id);
                let annotations = self.get_annotations_at_position(&selection.head);

                let mut style_values: FxHashMap<StyleType, Vec<Style>> = FxHashMap::default();
                for style in &self.state.pending_styles {
                    let st = style.as_type();
                    style_values.entry(st).or_default().push(style.clone());
                }

                let mut annotation_values: FxHashMap<AnnotationType, Vec<Annotation>> =
                    FxHashMap::default();
                for annotation in &annotations {
                    let at = annotation.as_type();
                    let values = annotation_values.entry(at).or_default();
                    if !values.iter().any(|v| v == annotation) {
                        values.push(annotation.clone());
                    }
                }

                let attrs = SelectionAttributes {
                    block_attrs,
                    style_values,
                    annotation_values,
                    absent_styles: FxHashSet::default(),
                    absent_annotations: FxHashSet::default(),
                };

                let (offset, count) = self.slab.write_attrs(&attrs);
                self.slate.attrs_offset = offset;
                self.slate.attrs_count = count;
            } else {
                let snapshot = self.selection_snapshot();
                if let Some(snapshot) = snapshot {
                    let (offset, count) = self.slab.write_attrs(&snapshot.attrs);
                    self.slate.attrs_offset = offset;
                    self.slate.attrs_count = count;
                } else {
                    self.slate.attrs_offset = 0;
                    self.slate.attrs_count = 0;
                }
            }

            self.slate.dirty |= DIRTY_ATTRS;
            self.pending.active_styles = false;
        }

        if self.pending.external_elements {
            let elements = self.build_external_elements();
            let start = self.slab.alloc(0, 4);
            for el in &elements {
                self.slab.write_u32_slice(&[el.page_idx as u32]);
                self.slab.write_str(&el.node_id);
                self.slab.write_f32_slice(&[
                    el.bounds.x,
                    el.bounds.y,
                    el.bounds.width,
                    el.bounds.height,
                ]);
                self.slab.write_u32_slice(&[el.is_selected as u32]);
                match &el.data {
                    ExternalElementData::Image {
                        id,
                        proportion,
                        upload_id,
                    } => {
                        self.slab.write_u32_slice(&[0]);
                        self.slab.write_str(id.as_deref().unwrap_or(""));
                        self.slab.write_str(upload_id.as_deref().unwrap_or(""));
                        self.slab.write_f32_slice(&[*proportion]);
                    }
                    ExternalElementData::File { id, upload_id } => {
                        self.slab.write_u32_slice(&[1]);
                        self.slab.write_str(id.as_deref().unwrap_or(""));
                        self.slab.write_str(upload_id.as_deref().unwrap_or(""));
                    }
                    ExternalElementData::Embed { id } => {
                        self.slab.write_u32_slice(&[2]);
                        self.slab.write_str(id.as_deref().unwrap_or(""));
                    }
                    ExternalElementData::Archived { id } => {
                        self.slab.write_u32_slice(&[3]);
                        self.slab.write_str(id.as_deref().unwrap_or(""));
                    }
                }
            }
            self.slate.external_elements_offset = start;
            self.slate.external_elements_count = elements.len() as u32;
            self.slate.dirty |= DIRTY_EXTERNAL_ELEMENTS;
            self.pending.external_elements = false;
        }

        if let Some(style) = self.pending.pointer_style.take() {
            self.slate.pointer_style = Self::pointer_style_to_u32(style);
            self.slate.dirty |= DIRTY_POINTER;
        }

        let fonts = std::mem::take(&mut self.pending.fonts);
        if !fonts.is_empty() {
            let start = self.slab.alloc(0, 4);
            let mut count = 0u32;
            for ((family, weight), codepoints) in &fonts {
                if !codepoints.is_empty() {
                    self.slab.write_str(family);
                    self.slab.write_u32_slice(&[*weight as u32]);
                    self.slab.write_u32_slice(&[codepoints.len() as u32]);
                    self.slab.write_u32_slice(codepoints);
                    count += 1;
                }
            }
            if count > 0 {
                self.slate.font_requests_offset = start;
                self.slate.font_requests_count = count;
                self.slate.dirty |= DIRTY_FONT_REQUIRED;
            }
        }

        let codepoints = std::mem::take(&mut self.pending.codepoints);
        if !codepoints.is_empty() {
            let (off, cnt) = self.slab.write_u32_slice(&codepoints);
            self.slate.fallback_codepoints_offset = off;
            self.slate.fallback_codepoints_count = cnt;
            self.slate.dirty |= DIRTY_FALLBACK_FONT_REQUIRED;
        }

        if had_layout {
            self.slate.dirty |= DIRTY_RENDER_REQUIRED;
        }

        if self.pending.enabled_actions {
            let enabled = self.evaluate_enabled_actions();
            let start = self.slab.alloc(0, 4);
            for action in &enabled {
                self.slab.write_str(action);
            }
            self.slate.enabled_actions_offset = start;
            self.slate.enabled_actions_count = enabled.len() as u32;
            self.slate.dirty |= DIRTY_ENABLED_ACTIONS;
            self.pending.enabled_actions = false;
        }

        if self.pending.exited_document_start {
            self.slate.dirty |= DIRTY_EXITED_DOCUMENT_START;
            self.pending.exited_document_start = false;
        }

        if self.pending.pointer_mode_changed {
            self.slate.pointer_state = self.pointer.mode.as_u32();
            self.slate.dirty |= DIRTY_POINTER;
            self.pending.pointer_mode_changed = false;
        }

        if self.pending.placeholder {
            let visible = self.doc().is_empty()
                && self.preedit().is_none()
                && self.selection().is_collapsed();
            let bounds = if visible {
                self.get_first_paragraph_bounds()
            } else {
                None
            };
            self.slate.placeholder_visible = visible as u32;
            if let Some(b) = bounds {
                self.slate.placeholder_x = b.x;
                self.slate.placeholder_y = b.y;
                self.slate.placeholder_width = b.width;
                self.slate.placeholder_height = b.height;
            } else {
                self.slate.placeholder_x = 0.0;
                self.slate.placeholder_y = 0.0;
                self.slate.placeholder_width = 0.0;
                self.slate.placeholder_height = 0.0;
            }
            self.slate.dirty |= DIRTY_PLACEHOLDER;
            self.pending.placeholder = false;
        }

        if self.pending.link_overlays {
            let overlays = self.build_link_overlays();
            let start = self.slab.alloc(0, 4);
            for o in &overlays {
                self.slab.write_u32_slice(&[o.page_idx as u32]);
                self.slab.write_str(&o.href);
                self.slab.write_u32_slice(&[o.bounds.len() as u32]);
                self.slab.write_text_bounds(&o.bounds);
            }
            self.slate.link_overlays_offset = start;
            self.slate.link_overlays_count = overlays.len() as u32;
            self.slate.dirty |= DIRTY_LINK_OVERLAYS;
            self.pending.link_overlays = false;
        }

        if self.pending.tracked_items {
            let overlays = tracked_items::build_tracked_item_overlays(
                &self.pages,
                &self.tracked_items,
                &self.state.doc,
            );
            let start = self.slab.alloc(0, 4);
            for o in &overlays {
                self.slab.write_u32_slice(&[o.page_idx as u32]);
                self.slab.write_u32_slice(&[o.group]);
                self.slab.write_str(&o.id);
                self.slab.write_bytes(o.node_id.as_uuid().as_bytes());
                self.slab
                    .write_u32_slice(&[o.start_offset as u32, o.end_offset as u32]);
                self.slab.write_u32_slice(&[o.bounds.len() as u32]);
                self.slab.write_text_bounds(&o.bounds);
            }
            self.slate.tracked_items_offset = start;
            self.slate.tracked_items_count = overlays.len() as u32;
            self.slate.dirty |= DIRTY_TRACKED_ITEMS;
            self.pending.tracked_items = false;
        }

        if self.pending.table_overlays {
            let overlays = self.build_table_overlays();
            if overlays != self.last_table_overlays {
                self.last_table_overlays = overlays.clone();
                let start = self.slab.alloc(0, 4);
                for o in &overlays {
                    self.slab.write_u32_slice(&[o.page_idx as u32]);
                    self.slab.write_str(&o.table_id);
                    self.slab.write_f32_slice(&[
                        o.bounds.x,
                        o.bounds.y,
                        o.bounds.width,
                        o.bounds.height,
                    ]);
                    self.slab.write_str(&o.border_style);
                    self.slab.write_str(&o.align);
                    self.slab.write_u32_slice(&[
                        o.start_row_index as u32,
                        o.total_rows as u32,
                        o.is_focused as u32,
                    ]);
                    self.slab.write_u32_slice(&[o.col_widths.len() as u32]);
                    self.slab.write_f32_slice(&o.col_widths);
                    self.slab.write_u32_slice(&[o.col_positions.len() as u32]);
                    self.slab.write_f32_slice(&o.col_positions);
                    self.slab.write_u32_slice(&[o.row_heights.len() as u32]);
                    self.slab.write_f32_slice(&o.row_heights);
                    self.slab.write_u32_slice(&[o.row_positions.len() as u32]);
                    self.slab.write_f32_slice(&o.row_positions);
                }
                self.slate.table_overlays_offset = start;
                self.slate.table_overlays_count = overlays.len() as u32;
                self.slate.dirty |= DIRTY_TABLE_OVERLAYS;
            }
            self.pending.table_overlays = false;
        }

        if let Some((text, from, to)) = self.pending.html_pasted.take() {
            let start = self.slab.alloc(0, 4);
            let str_off = self.slab.write_str(&text);
            let from_node = Self::node_id_to_bytes(from.node_id);
            self.slab.write_bytes(&from_node);
            self.slab
                .write_u32_slice(&[from.offset as u32, Self::affinity_to_u32(from.affinity)]);
            let to_node = Self::node_id_to_bytes(to.node_id);
            self.slab.write_bytes(&to_node);
            self.slab
                .write_u32_slice(&[to.offset as u32, Self::affinity_to_u32(to.affinity)]);
            self.slate.html_pasted_offset = start;
            self.slate.html_pasted_len = text.len() as u32;
            let _ = str_off;
            self.slate.dirty |= DIRTY_HTML_PASTED;
        }
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
            .selection_snapshot()
            .map(|snapshot| snapshot.block_set)
            .unwrap_or_default();

        for (page_idx, p) in self.pages.iter().enumerate() {
            for (pos, ext) in p.external_elements() {
                elements.push(ExternalElement {
                    page_idx,
                    node_id: ext.id.to_string(),
                    bounds: Rect {
                        x: pos.x,
                        y: pos.y,
                        width: ext.size.width,
                        height: ext.size.height,
                    },
                    data: ext.data.clone(),
                    is_selected: selected_nodes.contains(&ext.id),
                });
            }
        }

        elements
    }

    fn build_link_overlays(&self) -> Vec<cmd::LinkOverlay> {
        let link_ranges = self.doc().get_link_ranges();
        let mut overlays = Vec::new();

        for (page_idx, page) in self.pages.iter().enumerate() {
            for (href, bounds) in page.get_link_overlays(&link_ranges) {
                overlays.push(cmd::LinkOverlay {
                    page_idx,
                    href,
                    bounds,
                });
            }
        }

        overlays
    }

    fn get_first_paragraph_bounds(&self) -> Option<Rect> {
        let page = self.pages.first()?;
        let (pos, _element) = page.first_element()?;

        let settings = self.doc().settings();
        let paragraph_indent = settings.paragraph_indent * 16.0;
        let (page_width, margin_left, margin_right) = match settings.layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_margin_left,
                page_margin_right,
                ..
            } => (page_width, page_margin_left, page_margin_right),
            LayoutMode::Continuous { max_width } => {
                let page_margin = CONTINUOUS_PAGE_MARGIN;
                let page_width = self.width.min(max_width + 2.0 * page_margin);
                (page_width, page_margin, page_margin)
            }
        };

        Some(Rect {
            x: margin_left + paragraph_indent,
            y: pos.y,
            width: page_width - margin_left - margin_right - paragraph_indent,
            height: 0.0, // 안 씀
        })
    }

    pub fn inspect_state(&self) -> String {
        inspect_state(&self.doc(), &self.state.selection)
    }

    pub fn inspect_state_as_macro(&self) -> String {
        inspect_state_as_macro(&self.doc(), &self.state.selection)
    }

    pub fn inspect_fragment_as_macro(&self, fragment: &Fragment) -> String {
        inspect_fragment_as_macro(fragment)
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
        if selection.is_collapsed() {
            return false;
        }
        position_in_selection(&self.state.doc, position, &selection)
    }

    pub(crate) fn is_block_selectable_hit(&self, hit_selection: &Selection) -> bool {
        is_selectable_block_hit(&self.state.doc, hit_selection)
    }

    pub fn update(&mut self, msg: Message) {
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn process_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::FontDetected {
                    family,
                    weight,
                    codepoints,
                } => {
                    let loaded = self
                        .loaded_font_codepoints
                        .entry((family.clone(), weight))
                        .or_default();
                    let pending = self.pending.fonts.entry((family, weight)).or_default();
                    for cp in codepoints {
                        if loaded.insert(cp) {
                            pending.push(cp);
                        }
                    }
                }
                Effect::CodepointDetected { codepoints } => {
                    for cp in codepoints {
                        if !self.loaded_codepoints.contains(&cp) {
                            self.loaded_codepoints.insert(cp);
                            self.pending.codepoints.push(cp);
                        }
                    }
                }
                Effect::DocChanged => {
                    self.selection_cache = None;
                    self.cached_plain_text = None;
                    self.text_replacement_undo = None;
                    self.pending.doc = true;
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.external_elements = true;
                    self.pending.enabled_actions = true;
                    self.pending.placeholder = true;
                    self.pending.table_overlays = true;
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
                    self.pending.active_styles = true;
                    self.pending.external_elements = true;
                }
                Effect::SubtreeChanged { node_id } => {
                    if let Some(node) = self.doc().node(node_id) {
                        let descendants: Vec<_> = node.descendants().map(|n| n.node_id()).collect();
                        self.layout_cache
                            .borrow_mut()
                            .invalidate_with_descendants(node_id, descendants.into_iter());
                        let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                        for ancestor_id in ancestors {
                            self.layout_cache.borrow_mut().invalidate(ancestor_id);
                        }
                    } else {
                        self.layout_cache.borrow_mut().invalidate(node_id);
                    }
                    self.selection_cache = None;
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.external_elements = true;
                    self.pending.table_overlays = true;
                }
                Effect::SelectionChanged => {
                    self.selection_cache = None;
                    self.text_replacement_undo = None;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.render = true;
                    self.pending.external_elements = true;
                    self.pending.enabled_actions = true;
                    self.pending.placeholder = true;
                    self.pending.table_overlays = true;
                }
                Effect::PendingStylesChanged => {
                    self.pending.active_styles = true;
                    let nid = self.state.selection.head.node_id;
                    if let Some(node) = self.doc().node(nid) {
                        let ancestors: Vec<_> = node.ancestors().map(|n| n.node_id()).collect();
                        self.layout_cache
                            .borrow_mut()
                            .invalidate_with_ancestors(nid, ancestors.into_iter());
                    } else {
                        self.layout_cache.borrow_mut().invalidate(nid);
                    }
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                }
                Effect::ExternalElementChanged => {
                    self.pending.external_elements = true;
                }
                Effect::LayoutChanged => {
                    self.pending.layout = true;
                    self.pending.cursor = true;
                    self.pending.render = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.external_elements = true;
                    self.pending.placeholder = true;
                    self.pending.table_overlays = true;
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
                    self.pending.placeholder = true;
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
                Effect::TextReplacementApplied { undo_state } => {
                    self.text_replacement_undo = Some(undo_state);
                }
                Effect::HtmlPasted { text, from, to } => {
                    self.pending.html_pasted = Some((text, from, to));
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

    #[allow(dead_code)]
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
                let doc_changed = effects.iter().any(|e| matches!(e, Effect::DocChanged));
                let _selection_changed = effects
                    .iter()
                    .any(|e| matches!(e, Effect::SelectionChanged));

                if doc_changed {
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
    use crate::state::position_helpers::calculate_offset_before_child;
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
        assert_eq!(decor.node_id(), p1);
        assert_eq!(decor.start_offset(), 0);
        assert_eq!(
            decor.end_offset(),
            1,
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
                image(id: Some("test-image-id".to_string()),)
                paragraph {}
            }
            selection { (NodeId::ROOT, 0) -> (NodeId::ROOT, 1, Affinity::Upstream) }
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
            Position::new(NodeId::ROOT, 2, Affinity::Upstream),
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
    fn selection_common_ancestors_are_root_when_endpoints_span_outside_callout() {
        let mut outside = id!();
        let mut callout = id!();
        let mut inside = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @outside paragraph { text { "outside" } }
                @callout callout {
                    @inside paragraph { text { "inside" } }
                }
                paragraph {}
            }
            selection { (outside, 1) -> (inside, 3) }
        };

        runtime.tick();

        let ancestor_ids = runtime.selection_common_ancestor_ids(runtime.state.selection);
        assert_eq!(
            ancestor_ids,
            vec![NodeId::ROOT],
            "only ROOT should be common ancestor when endpoints are in different branches"
        );
    }

    #[test]
    fn selection_common_ancestors_include_callout_when_both_endpoints_inside() {
        let mut callout = id!();
        let mut p1 = id!();
        let mut p2 = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @callout callout {
                    @p1 paragraph { text { "a" } }
                    @p2 paragraph { text { "b" } }
                }
                paragraph {}
            }
            selection { (p1, 0) -> (p2, 1) }
        };

        runtime.tick();

        let ancestor_ids = runtime.selection_common_ancestor_ids(runtime.state.selection);
        assert_eq!(
            ancestor_ids,
            vec![callout, NodeId::ROOT],
            "callout and ROOT should be shared ancestor chain"
        );
    }

    #[test]
    fn cycle_callout_variant_requires_common_callout_ancestor() {
        let mut outside = id!();
        let mut callout = id!();
        let mut inside = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @outside paragraph { text { "outside" } }
                @callout callout {
                    @inside paragraph { text { "inside" } }
                }
                paragraph {}
            }
            selection { (outside, 1) -> (inside, 3) }
        };

        runtime.update(Message::CycleCalloutVariant);

        let callout_node = runtime.doc().node(callout).unwrap();
        let Node::Callout(callout_data) = callout_node.node() else {
            panic!("node should be callout");
        };
        assert_eq!(callout_data.variant, CalloutVariant::Info);
    }

    #[test]
    fn cycle_callout_variant_changes_when_selection_is_in_callout() {
        let mut callout = id!();
        let mut inside = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @callout callout {
                    @inside paragraph { text { "inside" } }
                }
                paragraph {}
            }
            selection { (inside, 0) -> (inside, 6) }
        };

        runtime.update(Message::CycleCalloutVariant);

        let callout_node = runtime.doc().node(callout).unwrap();
        let Node::Callout(callout_data) = callout_node.node() else {
            panic!("node should be callout");
        };
        assert_eq!(callout_data.variant, CalloutVariant::Success);
    }
}

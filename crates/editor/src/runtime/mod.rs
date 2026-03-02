mod cmd;
mod context;
mod dnd;
mod effect;
mod handlers;
mod history_state;
mod layout_engine;
mod layout_invalidation;
mod message;
mod pointer;
pub mod search;
pub mod slate;
mod state;
mod table;
pub(crate) mod text_replacement;
pub mod tracked_items;
mod view_state;

use crate::diagnostics::FrameDiagnostics;
use crate::inspect::{
    inspect_fragment_as_macro, inspect_page_element, inspect_state, inspect_state_as_macro,
};
use crate::layout::Page;
use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::query::{find_drag_image_bounds, find_node_bounds, is_selectable_block_hit};
use crate::model::*;
#[cfg(feature = "native")]
use crate::render::RenderInfo;
use crate::render::{DragImageResult, RenderResult, Renderer};
use crate::runtime::history_state::RuntimeHistory;
use crate::runtime::text_replacement::ReplacementUndoState;
use crate::state::ancestor_helpers::lowest_common_ancestor_id;
use crate::state::selection_helpers::{
    SelectionAttributes, build_selection_decorations, collect_block_attrs_at,
    collect_selected_block_ids, compute_selection_attrs, compute_structure_selection,
};
use crate::state::{
    Position, Preedit, Selection, find_child_at_offset, find_text_at_offset, position_in_selection,
};
use crate::transaction::{
    Transaction, compute_styles_at_cursor, paragraph_range_at, sentence_range_at, word_range_at,
};
use crate::types::{Affinity, PointerStyle, Rect};
use anyhow::Result;
pub use cmd::*;
pub use context::*;
pub use dnd::*;
pub use effect::*;
use layout_engine::LayoutEngine;
use layout_invalidation::{LayoutInvalidationBatch, LayoutInvalidationOp};
use loro::{Frontiers, UndoManager};
pub use message::*;
pub use pointer::*;
use rustc_hash::{FxHashMap, FxHashSet};
use slate::{Slab, Slate};
pub use state::*;
pub use tracked_items::{RawTrackedItem, TrackedItem};
pub use view_state::*;

pub(in crate::runtime) const ORIGIN_REMOTE_PREFIX: &str = "remote/";
pub(in crate::runtime) const ORIGIN_REMOTE_SYNC: &str = "remote/sync";

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
    render: bool,
    drop_indicator: Option<DropIndicator>,
    enabled_actions: bool,
    exited_document_start: bool,
    pointer_mode_changed: bool,
    placeholder: bool,
    link_overlays: bool,
    tracked_items: bool,
    table_overlays: bool,
    interactive_overlays: bool,
    default_attrs: bool,
    repaste: bool,
    remarks: bool,
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
    layout_engine: LayoutEngine,
    renderer: Renderer,

    state: State,
    undo_manager: UndoManager,

    loaded_font_codepoints: FxHashMap<(String, u16), FxHashSet<u32>>,
    missing_font_nodes: FxHashMap<(String, u16), (FxHashSet<NodeId>, FxHashSet<u32>)>,

    selection_cache: Option<SelectionSnapshot>,
    pending: PendingUpdates,
    message_queue: Vec<Message>,
    pointer: PointerState,

    pub slate: Slate,
    pub slab: Slab,

    history: RuntimeHistory,

    cached_plain_text: Option<String>,

    tracked_items: Vec<TrackedItem>,
    is_focused: bool,
    last_table_overlays: Vec<TableOverlay>,
    text_replacement_undo: Option<ReplacementUndoState>,

    repaste_text: Option<(Selection, String, Vec<Style>, Option<ParagraphNode>)>,
}

impl Runtime {
    pub fn new(width: f32, scale_factor: f64, state: State) -> Self {
        let mut undo_manager = UndoManager::new(state.doc.loro_doc());
        let history = RuntimeHistory::new();
        history.configure_undo_manager(&mut undo_manager);
        let diagnostics = FrameDiagnostics::new();

        Self {
            layout_engine: LayoutEngine::new(width, scale_factor, diagnostics.clone()),
            renderer: Renderer::new(scale_factor, diagnostics),
            state,
            undo_manager,
            loaded_font_codepoints: FxHashMap::default(),
            missing_font_nodes: FxHashMap::default(),
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
                render: true,
                drop_indicator: None,
                enabled_actions: true,
                exited_document_start: false,
                pointer_mode_changed: true,
                placeholder: true,
                link_overlays: false,
                tracked_items: false,
                table_overlays: true,
                interactive_overlays: true,
                default_attrs: true,
                repaste: false,
                remarks: true,
            },
            message_queue: Vec::new(),
            pointer: PointerState::default(),
            slate: Slate::default(),
            slab: Slab::new(),
            history,
            cached_plain_text: None,
            tracked_items: Vec::new(),
            is_focused: true,
            last_table_overlays: Vec::new(),
            text_replacement_undo: None,
            repaste_text: None,
        }
    }

    pub fn enqueue_message(&mut self, msg: Message) {
        if let Some(last) = self.message_queue.last_mut()
            && Self::try_merge_message(last, &msg)
        {
            return;
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
        self.layout_engine.pages()
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
            self.pending.interactive_overlays = true;
        }
    }

    pub fn is_read_only(&self) -> bool {
        self.state.read_only
    }

    pub fn set_render_debug(&mut self, enabled: bool) {
        self.renderer.set_render_debug(enabled);
        self.pending.render = true;
    }

    pub fn set_layout_debug(&mut self, enabled: bool) {
        self.layout_engine.set_layout_debug_enabled(enabled);
        self.renderer.set_layout_debug(enabled);
        if enabled {
            self.pending.layout = true;
        }
        self.pending.render = true;
    }

    pub fn set_all_folds_expanded(&mut self, expanded: bool) {
        let fold_ids = self
            .doc()
            .node(NodeId::ROOT)
            .map(|root| {
                root.descendants()
                    .filter_map(|node| match node.node() {
                        Some(Node::Fold(_)) => Some(node.node_id()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if fold_ids.is_empty() {
            return;
        }

        let mut effects = Vec::new();
        for fold_id in fold_ids {
            if self.layout_engine.fold_expanded(fold_id) == expanded {
                continue;
            }
            effects.extend(self.toggle_view_state(fold_id));
        }

        if !effects.is_empty() {
            self.process_effects(effects);
        }
    }

    pub fn import_updates(&mut self, updates: &[u8]) -> Result<()> {
        self.flush();
        let old_frontiers = self.state.doc.frontiers();
        let old_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        self.state.doc.import_updates(updates, ORIGIN_REMOTE_SYNC)?;
        let new_frontiers = self.state.doc.frontiers();

        if old_frontiers != new_frontiers {
            self.state.frontiers = new_frontiers;
            self.handle_external_doc_change(Some(old_state_frontiers));
        }
        Ok(())
    }

    pub fn import_updates_batch(&mut self, updates_batch: &[Vec<u8>]) -> Result<()> {
        self.flush();
        let old_frontiers = self.state.doc.frontiers();
        let old_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        self.state
            .doc
            .import_updates_batch(updates_batch, ORIGIN_REMOTE_SYNC)?;
        let new_frontiers = self.state.doc.frontiers();

        if old_frontiers != new_frontiers {
            self.state.frontiers = new_frontiers;
            self.handle_external_doc_change(Some(old_state_frontiers));
        }
        Ok(())
    }

    pub fn export(&mut self, mode: DocExportMode) -> Result<Vec<u8>> {
        self.flush();
        self.state.doc.export(mode)
    }

    pub fn checkout(&mut self, version: &[u8]) -> Result<()> {
        self.flush();
        let vv = loro::VersionVector::decode(version)?;
        if !self.state.doc.loro_doc().oplog_vv().includes_vv(&vv) {
            anyhow::bail!("Cannot checkout to unknown version");
        }
        let old_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        let frontiers = self.state.doc.loro_doc().vv_to_frontiers(&vv);
        self.state.doc.checkout(&frontiers)?;
        self.handle_external_doc_change(Some(old_state_frontiers));
        Ok(())
    }

    pub fn checkout_to_latest(&mut self) -> Result<()> {
        self.flush();
        let old_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        self.state.doc.checkout_to_latest()?;
        self.handle_external_doc_change(Some(old_state_frontiers));
        Ok(())
    }

    pub fn is_detached(&self) -> bool {
        self.state.doc.is_detached()
    }

    pub fn revert_to(&mut self, version: &[u8]) -> Result<()> {
        self.flush();
        let vv = loro::VersionVector::decode(version)?;
        if !self.state.doc.loro_doc().oplog_vv().includes_vv(&vv) {
            anyhow::bail!("Cannot revert to unknown version");
        }
        let old_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        let frontiers = self.state.doc.loro_doc().vv_to_frontiers(&vv);
        self.state.doc.revert_to(&frontiers)?;
        self.handle_external_doc_change(Some(old_state_frontiers));
        Ok(())
    }

    pub fn insert_template_fragment(&mut self, snapshot: Vec<u8>) -> Result<()> {
        let template_doc = std::rc::Rc::new(Doc::from_snapshot(snapshot)?);
        let template_settings = template_doc.settings();
        let template_default_attrs = template_doc.default_attrs();
        let fragment = Fragment::from_doc(&template_doc)?;

        let effects = self.transact(|tr| {
            let _ = tr.set_document_settings(template_settings)?;
            let _ = tr.set_default_attrs(template_default_attrs)?;

            let children: Vec<NodeId> = tr.doc().get_children_ids(NodeId::ROOT).to_vec();
            for child_id in children {
                tr.delete_node_recursive(child_id)?;
            }
            tr.set_selection(Selection::collapsed(Position::new(
                NodeId::ROOT,
                0,
                Affinity::Downstream,
            )));
            tr.paste_fragment(fragment, None)?;
            Ok(true)
        });
        self.process_effects(effects);
        Ok(())
    }

    fn handle_external_doc_change(&mut self, old_state_frontiers: Option<Frontiers>) {
        fn push_font_detected_effects(
            effects: &mut Vec<Effect>,
            fonts: Vec<(String, u16, FxHashSet<u32>)>,
        ) {
            effects.extend(fonts.into_iter().map(|(family, weight, codepoints)| {
                Effect::FontDetected {
                    family,
                    weight,
                    codepoints: codepoints.into_iter().collect(),
                }
            }));
        }

        fn push_full_fallback_effects(effects: &mut Vec<Effect>) {
            effects.push(Effect::FullLayoutInvalidation);
            effects.push(Effect::SettingsChanged);
        }

        self.state.doc.clear_children_cache();
        let mut effects = vec![Effect::DocChanged];

        let Some(old_state_frontiers) = old_state_frontiers.as_ref() else {
            push_full_fallback_effects(&mut effects);
            let fonts = self.collect_doc_fonts_from_nodes([NodeId::ROOT]);
            push_font_detected_effects(&mut effects, fonts);
            self.process_effects(effects);
            return;
        };

        let new_state_frontiers = self.state.doc.loro_doc().state_frontiers();
        let Some((node_mutations, settings_changed)) =
            self.analyze_external_doc_diff(old_state_frontiers, &new_state_frontiers)
        else {
            push_full_fallback_effects(&mut effects);
            let fonts = self.collect_doc_fonts_from_nodes([NodeId::ROOT]);
            push_font_detected_effects(&mut effects, fonts);
            self.process_effects(effects);
            return;
        };

        if settings_changed {
            effects.push(Effect::SettingsChanged);
        }

        effects.extend(
            node_mutations
                .iter()
                .map(|(node_id, kind)| Effect::NodeMutated {
                    node_id: *node_id,
                    kind: *kind,
                }),
        );

        let fonts = if node_mutations.is_empty() {
            if !settings_changed {
                push_full_fallback_effects(&mut effects);
                self.collect_doc_fonts_from_nodes([NodeId::ROOT])
            } else {
                Vec::new()
            }
        } else {
            self.collect_doc_fonts_from_nodes(node_mutations.keys().copied())
        };

        push_font_detected_effects(&mut effects, fonts);

        self.process_effects(effects);
    }

    fn resolve_invalidation(node_id: NodeId, kind: MutationKind) -> LayoutInvalidationOp {
        match kind {
            MutationKind::Text | MutationKind::Attr | MutationKind::ViewState => {
                LayoutInvalidationOp::NodeAndAncestors { node_id }
            }
            MutationKind::Structure | MutationKind::UnknownRemote => {
                LayoutInvalidationOp::SubtreeAndAncestors { node_id }
            }
        }
    }

    fn merge_node_mutation_kind(prev: MutationKind, next: MutationKind) -> MutationKind {
        const fn priority(kind: MutationKind) -> u8 {
            match kind {
                MutationKind::ViewState => 0,
                MutationKind::Attr => 1,
                MutationKind::Text => 2,
                MutationKind::Structure => 3,
                MutationKind::UnknownRemote => 4,
            }
        }

        if priority(next) > priority(prev) {
            next
        } else {
            prev
        }
    }

    fn classify_external_node_mutation(
        node_field_path: &[String],
        container_diff: &loro::event::Diff<'_>,
    ) -> MutationKind {
        if matches!(container_diff, loro::event::Diff::Unknown) {
            return MutationKind::UnknownRemote;
        }

        if node_field_path
            .iter()
            .any(|key| key == "children" || key == "parent")
        {
            return MutationKind::Structure;
        }

        match container_diff {
            loro::event::Diff::Text(_) => MutationKind::Text,
            loro::event::Diff::Map(_) => MutationKind::Attr,
            loro::event::Diff::List(_) => MutationKind::UnknownRemote,
            loro::event::Diff::Tree(_) => MutationKind::UnknownRemote,
            loro::event::Diff::Unknown => MutationKind::UnknownRemote,
        }
    }

    fn merge_external_node_mutation(
        node_mutations: &mut FxHashMap<NodeId, MutationKind>,
        node_id: NodeId,
        next: MutationKind,
    ) {
        let merged = node_mutations
            .get(&node_id)
            .map_or(next, |prev| Self::merge_node_mutation_kind(*prev, next));
        node_mutations.insert(node_id, merged);
    }

    fn analyze_external_doc_diff(
        &self,
        old_state_frontiers: &Frontiers,
        new_state_frontiers: &Frontiers,
    ) -> Option<(FxHashMap<NodeId, MutationKind>, bool)> {
        const NODES_KEY: &str = "nodes";
        const SETTINGS_KEY: &str = "settings";
        const CASCADE_ATTRS_KEY: &str = "cascade_attrs";

        let diff = self
            .state
            .doc
            .loro_doc()
            .diff(old_state_frontiers, new_state_frontiers)
            .ok()?;

        let mut node_mutations: FxHashMap<NodeId, MutationKind> = FxHashMap::default();
        let mut settings_changed = false;

        for (container_id, container_diff) in diff.iter() {
            let path = self
                .state
                .doc
                .loro_doc()
                .get_path_to_container(container_id)?;

            let key_path: Vec<String> = path
                .iter()
                .filter_map(|(_, index)| match index {
                    loro::Index::Key(key) => Some(key.to_string()),
                    _ => None,
                })
                .collect();

            let Some(root_key) = key_path.first() else {
                return None;
            };
            match root_key.as_str() {
                SETTINGS_KEY => {
                    settings_changed = true;
                }
                NODES_KEY => {
                    if key_path.len() >= 2 {
                        let node_key = &key_path[1];
                        let node_id = NodeId::from_string(&node_key)?;
                        if node_id == NodeId::ROOT
                            && key_path.iter().skip(2).any(|key| key == CASCADE_ATTRS_KEY)
                        {
                            settings_changed = true;
                        }
                        let kind =
                            Self::classify_external_node_mutation(&key_path[2..], container_diff);
                        Self::merge_external_node_mutation(&mut node_mutations, node_id, kind);
                    } else {
                        let loro::event::Diff::Map(map_diff) = container_diff else {
                            return None;
                        };
                        for node_key in map_diff.updated.keys() {
                            let node_id = NodeId::from_string(node_key.as_ref())?;
                            Self::merge_external_node_mutation(
                                &mut node_mutations,
                                node_id,
                                MutationKind::Structure,
                            );
                        }
                    }
                }
                _ => return None,
            }
        }

        Some((node_mutations, settings_changed))
    }

    pub fn replace_text_in_block(
        &mut self,
        block_id: NodeId,
        start_offset: usize,
        end_offset: usize,
        replacement: &str,
    ) -> Result<()> {
        use anyhow::Context;
        let previous_selection_snapshot = self.capture_history_selection(self.state.selection);
        let pending_txn_len_before = self.state.doc.loro_doc().get_pending_txn_len();
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

        let styles = Text::from_loro_text(text_node.clone()).styles_at_offset(start_internal);

        let _ = text_node.splice(
            start_internal,
            end_internal_offset - start_internal,
            replacement,
        );

        let replacement_len = replacement.chars().count();
        if replacement_len > 0 {
            reapply_styles(&text_node, start_internal, replacement_len, &styles);
        }

        let pending_txn_len_after = self.state.doc.loro_doc().get_pending_txn_len();
        let doc_changed = pending_txn_len_after > pending_txn_len_before;

        if doc_changed {
            self.capture_history_for_pending_doc_change(previous_selection_snapshot);
        }

        self.state.pending_loro_commit = self.state.pending_loro_commit || doc_changed;
        self.handle_external_doc_change(None);

        Ok(())
    }

    pub fn replace_text_in_blocks(
        &mut self,
        replacements: &[(NodeId, usize, usize, &str)],
    ) -> Result<()> {
        use anyhow::Context;
        let previous_selection_snapshot = self.capture_history_selection(self.state.selection);
        let pending_txn_len_before = self.state.doc.loro_doc().get_pending_txn_len();

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

            let styles = Text::from_loro_text(text_node.clone()).styles_at_offset(start_internal);

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

        let pending_txn_len_after = self.state.doc.loro_doc().get_pending_txn_len();
        let doc_changed = pending_txn_len_after > pending_txn_len_before;

        if doc_changed {
            self.capture_history_for_pending_doc_change(previous_selection_snapshot);
        }

        self.state.pending_loro_commit = self.state.pending_loro_commit || doc_changed;
        self.handle_external_doc_change(None);

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
        if selection.is_collapsed() {
            self.selection_cache = None;
            return None;
        }

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

            let attrs = compute_selection_attrs(self.doc(), &block_ids, from, to);

            self.selection_cache = Some(SelectionSnapshot {
                from,
                to,
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

    fn selection_type_for_id(&self, node_id: NodeId) -> u32 {
        self.doc()
            .node(node_id)
            .and_then(|node| node.node_type().map(|nt| slate::selection_type(nt)))
            .unwrap_or(slate::SELECTION_TYPE_NONE)
    }

    fn compute_selection_expandable(&self, selection: Selection) -> u32 {
        const EXPAND_WORD: u32 = 1;
        const EXPAND_SENTENCE: u32 = 2;
        const EXPAND_PARAGRAPH: u32 = 4;
        const EXPAND_ALL: u32 = 8;

        let Ok((from, to)) = selection.as_sorted(&self.state.doc) else {
            return EXPAND_WORD | EXPAND_SENTENCE | EXPAND_PARAGRAPH | EXPAND_ALL;
        };

        let mut flags = 0u32;

        if selection.is_collapsed() {
            let head = selection.head;

            if let Some((ws, we)) = word_range_at(&self.state.doc, head) {
                if ws != we {
                    flags |= EXPAND_WORD;
                }
            }

            if let Some((ss, se)) = sentence_range_at(&self.state.doc, head) {
                if ss != se {
                    flags |= EXPAND_SENTENCE;
                }
            }

            if let Some((_, ps, pe)) = paragraph_range_at(&self.state.doc, head) {
                if pe > ps {
                    flags |= EXPAND_PARAGRAPH;
                }
            }
        } else if from.node_id == to.node_id {
            let min_off = from.offset;
            let max_off = to.offset;
            let to_inner = Position::new(
                to.node_id,
                to.offset.saturating_sub(1),
                Affinity::Downstream,
            );

            // Word: only expandable if both endpoints are in the same word
            {
                let from_word = word_range_at(&self.state.doc, from);
                let to_word = if to.offset > 0 {
                    word_range_at(&self.state.doc, to_inner)
                } else {
                    from_word
                };
                if let (Some((ws1, we1)), Some((ws2, we2))) = (from_word, to_word) {
                    if ws1 == ws2 && we1 == we2 && (ws1 < min_off || we1 > max_off) {
                        flags |= EXPAND_WORD;
                    }
                }
            }

            // Sentence: only expandable if both endpoints are in the same sentence
            {
                let from_sent = sentence_range_at(&self.state.doc, from);
                let to_sent = if to.offset > 0 {
                    sentence_range_at(&self.state.doc, to_inner)
                } else {
                    from_sent
                };
                if let (Some((ss1, se1)), Some((ss2, se2))) = (from_sent, to_sent) {
                    if ss1 == ss2 && se1 == se2 && (ss1 < min_off || se1 > max_off) {
                        flags |= EXPAND_SENTENCE;
                    }
                }
            }

            // Paragraph: check if current selection is smaller than paragraph
            if let Some((para_id, ps, pe)) = paragraph_range_at(&self.state.doc, from) {
                if para_id != from.node_id || ps < min_off || pe > max_off {
                    flags |= EXPAND_PARAGRAPH;
                }
            }
        } else {
            // Cross-block selection: word/sentence/paragraph at head would shrink
            // But paragraph might expand if it covers more than current selection
            // For simplicity: only "all" can expand for cross-block selections
        }

        // All: check against document boundaries
        let ctx = NavigationContext::new(&self.state.doc);
        if let (Some(start_sel), Some(end_sel)) = (
            Cursor::move_to_document_start(&ctx, self.pages()),
            Cursor::move_to_document_end(&ctx, self.pages()),
        ) {
            if let (Ok((ds, _)), Ok((_, de))) = (
                start_sel.as_sorted(&self.state.doc),
                end_sel.as_sorted(&self.state.doc),
            ) {
                if from != ds || to != de {
                    flags |= EXPAND_ALL;
                }
            }
        }

        flags
    }

    pub fn layout(&mut self) {
        let doc = self.state.doc.as_ref();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();

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

        self.layout_engine
            .recompute(doc, &settings, &default_attrs, &decorations);
        self.renderer
            .prune_page_cache(self.layout_engine.page_count());
    }

    fn page_height_for_rendering(layout_mode: LayoutMode, page: &Page) -> f32 {
        match layout_mode {
            LayoutMode::Paginated { page_height, .. } => page_height.ceil(),
            LayoutMode::Continuous { .. } => page.root.node.size.height.ceil(),
        }
    }

    pub fn render_page(&mut self, page_index: usize) -> Option<RenderResult> {
        let snapshot = self.selection_snapshot();

        let doc = self.state.doc.as_ref();
        let selection = self.state.selection;

        let selections = if let Some(snapshot) = snapshot.as_ref() {
            build_selection_decorations(doc, &selection, Some(&snapshot.block_ids))
        } else {
            build_selection_decorations(doc, &selection, None)
        };

        let layout_mode = doc.settings().layout_mode;
        let pages = self.layout_engine.pages();
        let page = pages.get(page_index)?;
        let next_page = pages.get(page_index + 1);
        let page_height = Self::page_height_for_rendering(layout_mode, page);
        let page_width = self.layout_engine.width().ceil();
        let scale_factor = self.layout_engine.scale_factor();

        let drop_indicator = self.pending.drop_indicator.as_ref();
        let renderer = &mut self.renderer;
        renderer.set_size(page_width, page_height, scale_factor);
        renderer.set_focused(self.is_focused);

        Some(renderer.render(
            page,
            page_index,
            next_page,
            &selections,
            drop_indicator,
            doc,
        ))
    }

    pub fn export_page_vector(&mut self, page_index: usize) -> Option<Vec<u8>> {
        let doc = self.state.doc.as_ref();
        let layout_mode = doc.settings().layout_mode;
        let pages = self.layout_engine.pages();
        let page = pages.get(page_index)?;
        let next_page = pages.get(page_index + 1);
        let page_height = Self::page_height_for_rendering(layout_mode, page);
        let page_width = self.layout_engine.width().ceil();

        let vector_page =
            self.renderer
                .export_page_vector(page, next_page, doc, page_width, page_height);

        Some(crate::render::encode_vector_page(&vector_page))
    }

    #[cfg(feature = "native")]
    pub fn get_render_info(&mut self, page_index: usize) -> Option<RenderInfo> {
        let layout_mode = self.doc().settings().layout_mode;
        let page = self.layout_engine.pages().get(page_index)?;
        let page_height = Self::page_height_for_rendering(layout_mode, page);
        let page_width = self.layout_engine.width().ceil();
        let scale_factor = self.layout_engine.scale_factor();
        self.renderer
            .set_size(page_width, page_height, scale_factor);

        let width = self.renderer.width();
        let height = self.renderer.height();
        let buffer_size = width as usize * height as usize * 4;

        Some(RenderInfo {
            width,
            height,
            buffer_size,
        })
    }

    #[cfg(feature = "native")]
    pub fn render_page_to(&mut self, page_index: usize, dst: &mut [u8]) -> bool {
        let snapshot = self.selection_snapshot();

        let doc = self.state.doc.as_ref();
        let selection = self.state.selection;

        let selections = if let Some(snapshot) = snapshot.as_ref() {
            build_selection_decorations(doc, &selection, Some(&snapshot.block_ids))
        } else {
            build_selection_decorations(doc, &selection, None)
        };

        let layout_mode = doc.settings().layout_mode;
        let pages = self.layout_engine.pages();
        let Some(page) = pages.get(page_index) else {
            return false;
        };
        let next_page = pages.get(page_index + 1);
        let page_height = Self::page_height_for_rendering(layout_mode, page);
        let page_width = self.layout_engine.width().ceil();
        let scale_factor = self.layout_engine.scale_factor();
        let drop_indicator = self.pending.drop_indicator.as_ref();
        let renderer = &mut self.renderer;

        renderer.set_size(page_width, page_height, scale_factor);
        renderer.set_focused(self.is_focused);
        renderer.render_to(
            page,
            page_index,
            next_page,
            &selections,
            drop_indicator,
            doc,
            dst,
        )
    }

    pub fn render_drag_image(
        &mut self,
        visible_pages: &[usize],
        drag_page_idx: usize,
    ) -> Option<DragImageResult> {
        let doc = self.state.doc.as_ref();
        let selection = self.state.selection;
        let pages = self.layout_engine.pages();

        let bounds = find_drag_image_bounds(doc, &selection, pages)?;

        let selections = build_selection_decorations(doc, &selection, None);

        self.renderer.render_drag_image(
            pages,
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

        let Some(Node::Text(text_node)) = child.node() else {
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
                if !segment.annotations.is_empty() && local_offset == segment_end {
                    return Vec::new();
                }
                return segment.annotations.clone();
            }
            current_offset += segment_len;
        }

        Vec::new()
    }

    pub fn get_pointer_style(&self, page_idx: usize, x: f32, y: f32) -> PointerStyle {
        let Some(page) = self.pages().get(page_idx) else {
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
            let history_flush = self.begin_history_flush();
            self.state.doc.loro_doc().commit();
            self.state.pending_loro_commit = false;
            self.finish_history_flush(history_flush);
        }
    }

    fn process_message(&mut self, msg: Message) {
        self.ensure_valid_selection_state();
        if Self::requires_flush_before_handle(&msg) {
            self.flush();
        }
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn requires_flush_before_handle(msg: &Message) -> bool {
        matches!(msg, Message::Undo | Message::Redo)
    }

    fn ensure_valid_selection_state(&mut self) {
        let validated = self.validate_selection(self.state.selection);
        if validated != self.state.selection {
            self.state.selection = validated;
            self.state.pending_styles =
                compute_styles_at_cursor(&self.state.doc, &self.state.selection.head);
            self.pending.selection = true;
            self.pending.cursor = true;
            self.pending.active_styles = true;
        }
    }

    fn build_output(&mut self) {
        let mut had_layout = false;

        if self.pending.doc {
            self.slate.mark_doc_changed();
            self.pending.doc = false;
        }

        if self.pending.render {
            self.slab
                .write_drop_indicator(&mut self.slate, self.pending.drop_indicator.as_ref());
            self.slate.mark_render_required();
            self.pending.render = false;
        }

        if self.pending.settings {
            let settings = self.doc().settings();
            self.slab.write_settings(
                &mut self.slate,
                settings.paragraph_indent,
                settings.block_gap,
                settings.layout_mode,
            );
            self.pending.settings = false;
        }

        if self.pending.default_attrs {
            let attrs = self.doc().default_attrs();
            self.slab.write_default_attrs(&mut self.slate, &attrs);
            self.pending.default_attrs = false;
        }

        if self.pending.layout {
            self.layout();

            let layout_mode = self.doc().settings().layout_mode;
            let scale_factor = self.layout_engine.scale_factor();

            // Snap logical size to a value whose physical pixel count (round(logical * scale))
            // exactly equals CSS size * DPR, preventing sub-pixel stretching on fractional DPR
            // displays when the canvas is rendered with image-rendering: pixelated.
            let snap = |logical: f32| -> f32 {
                let physical = (logical as f64 * scale_factor).round();
                (physical / scale_factor) as f32
            };

            let page_width = match layout_mode {
                LayoutMode::Paginated { page_width, .. } => snap(page_width.ceil()),
                LayoutMode::Continuous { max_width } => snap(
                    self.layout_engine
                        .width()
                        .min(max_width + 2.0 * CONTINUOUS_PAGE_MARGIN)
                        .ceil(),
                ),
            };

            let page_heights: Vec<f32> = match layout_mode {
                LayoutMode::Paginated { page_height, .. } => {
                    vec![snap(page_height.ceil()); self.pages().len()]
                }
                LayoutMode::Continuous { .. } => self
                    .pages()
                    .iter()
                    .map(|p| snap(p.root.node.size.height.ceil()))
                    .collect(),
            };

            let pages_data: Vec<f32> = page_heights.iter().flat_map(|&h| [page_width, h]).collect();
            self.slab.write_pages(&mut self.slate, &pages_data);
            self.pending.layout = false;
            had_layout = true;

            if self.state.read_only {
                self.pending.link_overlays = true;
            }

            if !self.tracked_items.is_empty() {
                self.pending.tracked_items = true;
            }

            self.pending.table_overlays = true;
            self.pending.interactive_overlays = true;
            self.pending.remarks = true;
        }

        if self.pending.cursor {
            let selection = self.state.selection;
            let ctx = NavigationContext::new(&self.state.doc);
            let (page_idx, bounds, visible) = Cursor::bounds(&ctx, self.pages(), selection.head)
                .map(|(idx, rect)| (Some(idx), Some(rect), selection.is_collapsed()))
                .unwrap_or((None, None, false));

            let preceding_char_widths = if selection.is_collapsed() {
                Cursor::preceding_char_widths(&ctx, self.pages(), selection.head, 64)
            } else {
                None
            };

            self.slab.write_cursor(
                &mut self.slate,
                page_idx,
                bounds,
                visible,
                preceding_char_widths.as_deref(),
            );
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
                Cursor::selection_handle_bounds(&ctx, self.pages(), selection.anchor)
                    .map(|(page_idx, bounds)| SelectionHandleBounds { page_idx, bounds });
            let head_handle = Cursor::selection_handle_bounds(&ctx, self.pages(), selection.head)
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
            self.slab.write_selection(
                &mut self.slate,
                selection,
                cmp,
                &selected_block_ids,
                &selected_block_types,
                &common_ancestor_ids,
                &common_ancestor_types,
                anchor_handle.as_ref(),
                head_handle.as_ref(),
            );
            self.slate.selection_expandable = self.compute_selection_expandable(selection);

            let anchor_node = self.doc().node(selection.anchor.node_id);
            if let Some(anchor_node) = anchor_node {
                let block_node = if anchor_node.is_inline() {
                    anchor_node.parent()
                } else {
                    match anchor_node.child(selection.anchor.offset) {
                        Some(c) if c.is_block() => Some(c),
                        _ => Some(anchor_node),
                    }
                };
                if let Some(block) = block_node {
                    let block_id = block.node_id();
                    self.slate.current_block_node_id = *block_id.as_uuid().as_bytes();
                    if let Some(nb) = find_node_bounds(self.doc(), self.pages(), block_id) {
                        self.slate.current_block_page_idx = nb.page_idx as i32;
                        self.slate.current_block_x = nb.x;
                        self.slate.current_block_y = nb.y;
                        self.slate.current_block_width = nb.width;
                        self.slate.current_block_height = nb.height;
                    } else {
                        self.slate.current_block_page_idx = -1;
                    }
                }
            }

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
                    has_text_segments: false,
                };

                self.slab.write_attrs(&mut self.slate, Some(&attrs));
            } else {
                let snapshot = self.selection_snapshot();
                if let Some(mut snapshot) = snapshot {
                    if !snapshot.attrs.has_text_segments {
                        for style in &self.state.pending_styles {
                            let st = style.as_type();
                            snapshot
                                .attrs
                                .style_values
                                .entry(st)
                                .or_default()
                                .push(style.clone());
                        }
                    }
                    self.slab
                        .write_attrs(&mut self.slate, Some(&snapshot.attrs));
                } else {
                    self.slab.write_attrs(&mut self.slate, None);
                }
            }
            self.pending.active_styles = false;
        }

        if self.pending.external_elements {
            let elements = self.build_external_elements();
            self.slab
                .write_external_elements(&mut self.slate, &elements);
            self.pending.external_elements = false;
        }

        if let Some(style) = self.pending.pointer_style.take() {
            self.slab.write_pointer_style(&mut self.slate, style);
        }

        let fonts = std::mem::take(&mut self.pending.fonts);
        if !fonts.is_empty() {
            self.slab.write_font_requests(&mut self.slate, &fonts);
        }

        if had_layout {
            self.slate.mark_render_required();
        }

        if self.pending.enabled_actions {
            let enabled = self.evaluate_enabled_actions();
            self.slab.write_enabled_actions(&mut self.slate, &enabled);
            self.pending.enabled_actions = false;
        }

        if self.pending.exited_document_start {
            self.slate.mark_exited_document_start();
            self.pending.exited_document_start = false;
        }

        if self.pending.pointer_mode_changed {
            self.slate.write_pointer_state(self.pointer.mode.as_u32());
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
            self.slab
                .write_placeholder(&mut self.slate, visible, bounds);
            self.pending.placeholder = false;
        }

        if self.pending.link_overlays {
            let overlays = self.build_link_overlays();
            self.slab.write_link_overlays(&mut self.slate, &overlays);
            self.pending.link_overlays = false;
        }

        if self.pending.tracked_items {
            let overlays = tracked_items::build_tracked_item_overlays(
                self.pages(),
                &self.tracked_items,
                &self.state.doc,
            );
            self.slab.write_tracked_items(&mut self.slate, &overlays);
            self.pending.tracked_items = false;
        }

        if self.pending.table_overlays {
            let overlays = self.build_table_overlays();
            if overlays != self.last_table_overlays {
                self.last_table_overlays = overlays.clone();
                self.slab.write_table_overlays(&mut self.slate, &overlays);
            }
            self.pending.table_overlays = false;
        }

        if self.pending.interactive_overlays {
            let overlays = self.build_interactive_overlays();
            self.slab
                .write_interactive_overlays(&mut self.slate, &overlays);
            self.pending.interactive_overlays = false;
        }

        if self.pending.repaste {
            self.slate
                .write_repaste_enabled(self.repaste_text.is_some());
            self.pending.repaste = false;
        }

        if self.pending.remarks {
            let overlays = self.build_remark_overlays();
            self.slab.write_remarks(&mut self.slate, &overlays);
            self.pending.remarks = false;
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

        for (page_idx, p) in self.pages().iter().enumerate() {
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

    fn build_interactive_overlays(&self) -> Vec<InteractiveOverlay> {
        let mut result = Vec::new();
        let read_only = self.is_read_only();
        for (page_idx, page) in self.pages().iter().enumerate() {
            page.collect_interactive_overlays(page_idx, read_only, &mut result);
        }
        result
    }

    fn build_remark_overlays(&self) -> Vec<cmd::RemarkOverlay> {
        let mut overlays = Vec::new();
        for (node_id, remark) in self.doc().all_remarks() {
            if let Some(nb) = find_node_bounds(self.doc(), self.pages(), node_id) {
                overlays.push(cmd::RemarkOverlay {
                    page_idx: nb.page_idx,
                    node_id,
                    remark_id: remark.id,
                    user_id: remark.user_id,
                    text: remark.text,
                    created_at: remark.created_at,
                    bounds: Rect {
                        x: nb.x,
                        y: nb.y,
                        width: nb.width,
                        height: nb.height,
                    },
                });
            }
        }
        overlays
    }

    fn build_link_overlays(&self) -> Vec<cmd::LinkOverlay> {
        let link_ranges = self.doc().get_link_ranges();
        let mut overlays = Vec::new();

        for (page_idx, page) in self.pages().iter().enumerate() {
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
        let page = self.pages().first()?;
        let (pos, _element) = page.first_element()?;

        let settings = self.doc().settings();
        let paragraph_indent = settings.paragraph_indent as f32 / 100.0 * 16.0;
        let (page_width, margin_left, margin_right) = match settings.layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_margin_left,
                page_margin_right,
                ..
            } => (page_width, page_margin_left, page_margin_right),
            LayoutMode::Continuous { max_width } => {
                let page_margin = CONTINUOUS_PAGE_MARGIN;
                let page_width = self
                    .layout_engine
                    .width()
                    .min(max_width + 2.0 * page_margin);
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
        inspect_state(self.doc(), &self.state.selection)
    }

    pub fn inspect_state_as_macro(&self) -> String {
        inspect_state_as_macro(self.doc(), &self.state.selection)
    }

    pub fn inspect_fragment_as_macro(&self, fragment: &Fragment) -> String {
        inspect_fragment_as_macro(fragment)
    }

    pub fn inspect_page_element(&self, page_idx: usize, x: f32, y: f32) -> Option<String> {
        let page = self.pages().get(page_idx)?;

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
        self.ensure_valid_selection_state();
        if Self::requires_flush_before_handle(&msg) {
            self.flush();
        }
        let effects = msg.handle(self);
        self.process_effects(effects);
    }

    fn process_effects(&mut self, mut effects: Vec<Effect>) {
        effects.sort_by_key(|e| e.priority());
        let mut invalidation = LayoutInvalidationBatch::new();
        let mut font_affected_nodes: FxHashSet<NodeId> = FxHashSet::default();

        for effect in effects {
            match effect {
                Effect::FontDetected {
                    family,
                    weight,
                    codepoints,
                } => {
                    let key = (family.clone(), weight);
                    let loaded = self.loaded_font_codepoints.entry(key.clone()).or_default();
                    let pending = self.pending.fonts.entry(key.clone()).or_default();
                    let mut newly_detected = Vec::new();
                    for cp in codepoints {
                        if loaded.insert(cp) {
                            pending.push(cp);
                            newly_detected.push(cp);
                        }
                    }

                    if !newly_detected.is_empty() {
                        let (nodes_for_font, cps_for_font) =
                            self.missing_font_nodes.entry(key).or_default();
                        if font_affected_nodes.is_empty() {
                            // fallback
                            nodes_for_font.insert(NodeId::ROOT);
                        } else {
                            nodes_for_font.extend(font_affected_nodes.iter().copied());
                        }
                        cps_for_font.extend(newly_detected.iter().copied());
                    }
                }
                Effect::DocChanged => {
                    self.selection_cache = None;
                    self.cached_plain_text = None;
                    self.text_replacement_undo = None;
                    self.repaste_text = None;
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
                    self.pending.repaste = true;
                    self.pending.remarks = true;
                }
                Effect::NodeMutated { node_id, kind } => {
                    self.selection_cache = None;
                    if matches!(
                        kind,
                        MutationKind::Text
                            | MutationKind::Attr
                            | MutationKind::Structure
                            | MutationKind::UnknownRemote
                    ) {
                        font_affected_nodes.insert(node_id);
                    }
                    invalidation.push(Self::resolve_invalidation(node_id, kind));
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.external_elements = true;
                    if matches!(kind, MutationKind::Structure | MutationKind::UnknownRemote) {
                        self.pending.table_overlays = true;
                        self.pending.remarks = true;
                    }
                }
                Effect::SelectionChanged => {
                    self.selection_cache = None;
                    self.text_replacement_undo = None;
                    self.repaste_text = None;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.active_styles = true;
                    self.pending.render = true;
                    self.pending.external_elements = true;
                    self.pending.enabled_actions = true;
                    self.pending.placeholder = true;
                    self.pending.table_overlays = true;
                    self.pending.repaste = true;
                }
                Effect::PendingStylesChanged => {
                    self.pending.active_styles = true;
                    let nid = self.state.selection.head.node_id;
                    font_affected_nodes.insert(nid);
                    invalidation.push(LayoutInvalidationOp::NodeAndAncestors { node_id: nid });
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
                    self.pending.remarks = true;
                }
                Effect::FullLayoutInvalidation => {
                    invalidation.push(LayoutInvalidationOp::Full);
                }
                Effect::PointerStyleChanged { style } => {
                    self.pending.pointer_style = Some(style);
                }
                Effect::PreeditChanged { node_id } => {
                    if let Some(nid) = node_id {
                        font_affected_nodes.insert(nid);
                        invalidation.push(LayoutInvalidationOp::NodeAndAncestors { node_id: nid });
                    }

                    self.text_replacement_undo = None;
                    self.repaste_text = None;

                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.selection = true;
                    self.pending.placeholder = true;
                    self.pending.repaste = true;
                }
                Effect::SettingsChanged => {
                    let layout_mode = self.doc().settings().layout_mode;
                    self.sync_layout_width(layout_mode);
                    invalidation.push(LayoutInvalidationOp::Full);
                    self.pending.layout = true;
                    self.pending.render = true;
                    self.pending.cursor = true;
                    self.pending.settings = true;
                    self.pending.external_elements = true;
                    self.pending.default_attrs = true;
                }
                Effect::DropTargetChanged { target } => {
                    self.pending.drop_indicator = target.and_then(|position| {
                        let ctx = NavigationContext::new(&self.state.doc);
                        DropIndicator::from_position(&ctx, self.pages(), position)
                    });
                    self.pending.render = true;
                }
                Effect::ExitedDocumentStart => {
                    self.pending.exited_document_start = true;
                }
                Effect::TextReplacementApplied { undo_state } => {
                    self.text_replacement_undo = Some(undo_state);
                }
                Effect::HtmlPasted {
                    selection,
                    text,
                    styles,
                    paragraph_attrs,
                } => {
                    let structure_selection = compute_structure_selection(self.doc(), &selection);
                    font_affected_nodes.extend(collect_selected_block_ids(
                        self.doc(),
                        &selection,
                        &structure_selection,
                    ));
                    self.repaste_text = Some((selection, text, styles, paragraph_attrs));
                    self.pending.repaste = true;
                }
            }
        }

        if !invalidation.is_empty() {
            self.layout_engine
                .apply_invalidation(self.state.doc.as_ref(), &invalidation);
        }
    }

    fn transact<F>(&mut self, f: F) -> Vec<Effect>
    where
        F: FnOnce(&mut Transaction) -> Result<bool>,
    {
        self.transact_internal(f, true)
    }

    fn transact_internal<F>(&mut self, f: F, defer_commit: bool) -> Vec<Effect>
    where
        F: FnOnce(&mut Transaction) -> Result<bool>,
    {
        let previous_selection = self.state.selection;
        let previous_selection_snapshot = self.capture_history_selection(previous_selection);

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
                let doc_changed = effects
                    .iter()
                    .any(|e| matches!(e, Effect::DocChanged | Effect::NodeMutated { .. }))
                    || new_state.frontiers != self.state.frontiers;
                let selection_changed = effects
                    .iter()
                    .any(|e| matches!(e, Effect::SelectionChanged));

                if doc_changed {
                    self.capture_history_for_pending_doc_change(previous_selection_snapshot);
                } else if selection_changed {
                    if self.state.pending_loro_commit {
                        self.flush();
                    }
                    self.schedule_split_next_history_group();
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
        self.layout_engine.is_layout_cached(node_id)
    }

    #[cfg(test)]
    pub fn cached_layout(&self, node_id: NodeId) -> Option<std::rc::Rc<crate::layout::LayoutNode>> {
        self.layout_engine.cached_layout(node_id)
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

        self.state.doc.clear_children_cache();
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
                Some(Node::Text(text)) => offset += text.text.char_len(),
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
    use crate::runtime::slate::{
        DIRTY_DEFAULT_ATTRS, DIRTY_DOC_CHANGED, DIRTY_FONT_REQUIRED, DIRTY_SETTINGS,
    };
    use crate::state::position_helpers::calculate_offset_before_child;
    use std::rc::Rc;

    fn has_dirty_flag(runtime: &Runtime, flag: u64) -> bool {
        runtime.slate.dirty & flag != 0
    }

    #[test]
    fn test_tracked_items_shift_after_replace_text_in_block() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) }
        };

        runtime.set_tracked_items(
            0,
            vec![RawTrackedItem {
                id: "item-1".to_string(),
                node_id: p,
                start_offset: 6,
                end_offset: 11,
            }],
        );

        let before = runtime.tracked_items[0]
            .resolve_range(runtime.doc())
            .expect("tracked item should resolve before replacement");
        assert_eq!(before.1, 6);
        assert_eq!(before.2, 11);

        runtime
            .replace_text_in_block(p, 6, 6, "big ")
            .expect("replace_text_in_block should succeed");

        let after = runtime.tracked_items[0]
            .resolve_range(runtime.doc())
            .expect("tracked item should resolve after replacement");
        assert_eq!(after.1, 10);
        assert_eq!(after.2, 15);
    }

    #[test]
    fn test_tracked_items_shift_after_replace_text_in_blocks() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 0) }
        };

        runtime.set_tracked_items(
            0,
            vec![RawTrackedItem {
                id: "item-1".to_string(),
                node_id: p,
                start_offset: 6,
                end_offset: 11,
            }],
        );

        let replacements = vec![(p, 6, 6, "big ")];
        runtime
            .replace_text_in_blocks(&replacements)
            .expect("replace_text_in_blocks should succeed");

        let after = runtime.tracked_items[0]
            .resolve_range(runtime.doc())
            .expect("tracked item should resolve after replacement");
        assert_eq!(after.1, 10);
        assert_eq!(after.2, 15);
    }

    #[test]
    fn test_undo_restores_selection_after_replace_text_in_block() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 2) }
        };

        runtime
            .replace_text_in_block(p, 0, 5, "hi")
            .expect("replace_text_in_block should succeed");
        runtime.flush();

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 11,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 11,
            head_affinity: Affinity::Downstream,
        });

        runtime.update(Message::Undo);

        assert_eq!(runtime.doc().to_plain_text(), "hello world");
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 2, Affinity::Downstream)),
            "undo should restore selection captured before replacement"
        );
    }

    #[test]
    fn test_undo_restores_selection_after_replace_text_in_blocks() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello world" }
                }
            }
            selection { (p, 3) }
        };

        let replacements = vec![(p, 0, 5, "hi"), (p, 6, 11, "planet")];
        runtime
            .replace_text_in_blocks(&replacements)
            .expect("replace_text_in_blocks should succeed");
        runtime.flush();

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 9,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 9,
            head_affinity: Affinity::Downstream,
        });

        runtime.update(Message::Undo);

        assert_eq!(runtime.doc().to_plain_text(), "hello world");
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 3, Affinity::Downstream)),
            "undo should restore selection captured before bulk replacement"
        );
    }

    #[test]
    fn test_undo_respects_selection_boundary_between_inputs() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 0,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 0,
            head_affinity: Affinity::Downstream,
        });

        runtime.update(Message::Input {
            text: "y".to_string(),
        });

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 3,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 3,
            head_affinity: Affinity::Downstream,
        });

        runtime.update(Message::Input {
            text: "z".to_string(),
        });

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.doc().to_plain_text(),
            "yAx",
            "first undo should revert only the third input"
        );
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 3, Affinity::Downstream)),
            "first undo should restore selection before third input"
        );

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.doc().to_plain_text(),
            "Ax",
            "second undo should revert only the second input"
        );
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            "second undo should restore selection before second input"
        );

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.doc().to_plain_text(),
            "A",
            "third undo should revert the first input"
        );
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "third undo should restore selection before first input"
        );
    }

    #[test]
    fn test_undo_preserves_upstream_affinity_before_input() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });

        runtime.update(Message::Undo);

        assert_eq!(
            runtime.doc().to_plain_text(),
            "AB",
            "undo should revert inserted text"
        );
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Upstream)),
            "undo should preserve pre-input affinity as well as position"
        );
    }

    #[test]
    fn test_undo_restores_latest_selection_after_undo_stack_rollover() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        let mut expected_text_before_last_input = String::new();
        let mut expected_selection_before_last_input = runtime.state().selection;

        for _round in 0..120 {
            expected_text_before_last_input = runtime.doc().to_plain_text();
            expected_selection_before_last_input = runtime.state().selection;

            runtime.update(Message::Input {
                text: "x".to_string(),
            });

            runtime.update(Message::SetSelection {
                anchor_node_id: p.to_string(),
                anchor_offset: 0,
                anchor_affinity: Affinity::Downstream,
                head_node_id: p.to_string(),
                head_offset: 0,
                head_affinity: Affinity::Downstream,
            });
        }

        assert_eq!(
            runtime.undo_manager.undo_count(),
            100,
            "precondition: undo stack should hit Loro's default max size"
        );

        runtime.update(Message::Undo);

        assert_eq!(
            runtime.doc().to_plain_text(),
            expected_text_before_last_input
        );
        assert_eq!(
            runtime.state().selection,
            expected_selection_before_last_input,
            "latest undo entry should restore the selection captured before the latest input"
        );
    }

    #[test]
    fn test_undo_roundtrip_matches_checkpoint_selection_under_mixed_input_and_moves() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        #[derive(Clone)]
        struct Checkpoint {
            text: String,
            selection: Selection,
        }

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        fn pick<'a>(seed: &mut u64, items: &'a [&'a str]) -> &'a str {
            let idx = (next_u32(seed) as usize) % items.len();
            items[idx]
        }

        fn random_affinity(seed: &mut u64) -> Affinity {
            if next_u32(seed) & 1 == 0 {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            }
        }

        let mut seed = 0xCEA5EED_u64;
        let mut checkpoints: Vec<Checkpoint> = Vec::new();
        let rounds = 80;

        for _ in 0..rounds {
            let before_text = runtime.doc().to_plain_text();
            let before_selection = runtime.state().selection;

            if next_u32(&mut seed) & 1 == 0 {
                let token = pick(&mut seed, &["a", "b", " ", "xy"]);
                runtime.update(Message::Input {
                    text: token.to_string(),
                });
            } else {
                let sequence = pick(&mut seed, &["ㅎ,하,한", "ㄱ,가,각", "ㅂ,바,밥"]);
                let mut parts = sequence.split(',');
                let start = parts.next().unwrap_or_default().to_string();
                let update = parts.next().unwrap_or_default().to_string();
                runtime.update(Message::CompositionStart { text: start });
                runtime.update(Message::CompositionUpdate { text: update });
                runtime.update(Message::CommitPreedit);
            }

            let after_text = runtime.doc().to_plain_text();
            if after_text != before_text {
                checkpoints.push(Checkpoint {
                    text: before_text,
                    selection: before_selection,
                });
            }

            let max_offset = runtime
                .doc()
                .node(p)
                .map(|node| runtime.calculate_max_offset(&node))
                .unwrap_or(0);
            let offset = (next_u32(&mut seed) as usize) % (max_offset + 1);
            let affinity = random_affinity(&mut seed);
            runtime.update(Message::SetSelection {
                anchor_node_id: p.to_string(),
                anchor_offset: offset,
                anchor_affinity: affinity,
                head_node_id: p.to_string(),
                head_offset: offset,
                head_affinity: affinity,
            });
        }

        let undo_steps = checkpoints.len().min(runtime.undo_manager.undo_count());
        let mut last_undo_text: Option<String> = Some(runtime.doc().to_plain_text());
        for undo_index in 0..undo_steps {
            runtime.update(Message::Undo);
            let actual_text = runtime.doc().to_plain_text();

            let noop_undo = last_undo_text
                .as_ref()
                .map_or(false, |previous| previous == &actual_text);
            last_undo_text = Some(actual_text.clone());
            if noop_undo {
                continue;
            }

            let mut matched_index = None;
            for idx in (0..checkpoints.len()).rev() {
                if checkpoints[idx].text == actual_text {
                    matched_index = Some(idx);
                    break;
                }
            }

            let Some(matched_index) = matched_index else {
                panic!(
                    "undo#{}, no checkpoint matched content: {}",
                    undo_index + 1,
                    actual_text
                );
            };

            let expected_selection = checkpoints[matched_index].selection;
            assert_eq!(
                runtime.state().selection,
                expected_selection,
                "undo#{} should restore selection for matched checkpoint idx={}",
                undo_index + 1,
                matched_index
            );

            checkpoints.truncate(matched_index);
        }
    }

    #[test]
    fn test_undo_roundtrip_matches_checkpoint_selection_across_dynamic_textblocks() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        #[derive(Clone)]
        struct Checkpoint {
            text: String,
            selection: Selection,
        }

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        fn pick<'a>(seed: &mut u64, items: &'a [&'a str]) -> &'a str {
            let idx = (next_u32(seed) as usize) % items.len();
            items[idx]
        }

        fn random_affinity(seed: &mut u64) -> Affinity {
            if next_u32(seed) & 1 == 0 {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            }
        }

        fn collect_textblocks(runtime: &Runtime) -> Vec<NodeId> {
            let mut ids = Vec::new();
            let mut stack = vec![NodeId::ROOT];

            while let Some(node_id) = stack.pop() {
                let Some(node) = runtime.doc().node(node_id) else {
                    continue;
                };

                if node
                    .spec()
                    .map_or(false, |spec| spec.is_textblock(runtime.doc().schema()))
                {
                    ids.push(node_id);
                }

                for child in node.children() {
                    stack.push(child.node_id());
                }
            }

            ids
        }

        let mut seed = 0xD1A0_2026_u64;
        let mut checkpoints: Vec<Checkpoint> = Vec::new();
        let rounds = 120;

        for _ in 0..rounds {
            let before_text = runtime.doc().to_plain_text();
            let before_selection = runtime.state().selection;

            match next_u32(&mut seed) % 3 {
                0 => {
                    let token = pick(&mut seed, &["a", "b", " ", "xy", "zz"]);
                    runtime.update(Message::Input {
                        text: token.to_string(),
                    });
                }
                1 => {
                    let sequence =
                        pick(&mut seed, &["ㅎ,하,한", "ㄱ,가,각", "ㅂ,바,밥", "ㄴ,나,난"]);
                    let mut parts = sequence.split(',');
                    let start = parts.next().unwrap_or_default().to_string();
                    let update = parts.next().unwrap_or_default().to_string();
                    runtime.update(Message::CompositionStart { text: start });
                    runtime.update(Message::CompositionUpdate { text: update });
                    runtime.update(Message::CommitPreedit);
                }
                _ => {
                    runtime.update(Message::InsertNewline);
                }
            }

            let after_text = runtime.doc().to_plain_text();
            if after_text != before_text {
                checkpoints.push(Checkpoint {
                    text: before_text,
                    selection: before_selection,
                });
            }

            let textblocks = collect_textblocks(&runtime);
            assert!(
                !textblocks.is_empty(),
                "precondition: there should be at least one textblock"
            );

            let block_id = textblocks[(next_u32(&mut seed) as usize) % textblocks.len()];
            let max_offset = runtime
                .doc()
                .node(block_id)
                .map(|node| runtime.calculate_max_offset(&node))
                .unwrap_or(0);
            let offset = (next_u32(&mut seed) as usize) % (max_offset + 1);
            let affinity = random_affinity(&mut seed);

            runtime.update(Message::SetSelection {
                anchor_node_id: block_id.to_string(),
                anchor_offset: offset,
                anchor_affinity: affinity,
                head_node_id: block_id.to_string(),
                head_offset: offset,
                head_affinity: affinity,
            });
        }

        let undo_steps = checkpoints.len().min(runtime.undo_manager.undo_count());
        let mut last_undo_text: Option<String> = None;
        for undo_index in 0..undo_steps {
            runtime.update(Message::Undo);
            let actual_text = runtime.doc().to_plain_text();

            let noop_undo = last_undo_text
                .as_ref()
                .map_or(false, |previous| previous == &actual_text);
            last_undo_text = Some(actual_text.clone());
            if noop_undo {
                continue;
            }

            let mut matched_index = None;
            for idx in (0..checkpoints.len()).rev() {
                if checkpoints[idx].text == actual_text {
                    matched_index = Some(idx);
                    break;
                }
            }

            let Some(matched_index) = matched_index else {
                panic!(
                    "undo#{}, no checkpoint matched content: {}",
                    undo_index + 1,
                    actual_text
                );
            };

            let expected_selection = checkpoints[matched_index].selection;
            assert_eq!(
                runtime.state().selection,
                expected_selection,
                "undo#{} should restore selection for matched checkpoint idx={}",
                undo_index + 1,
                matched_index
            );

            checkpoints.truncate(matched_index + 1);
        }
    }

    #[test]
    fn test_top_undo_marker_stability_for_merged_inputs() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "" }
                }
            }
            selection { (p, 0) }
        };

        runtime.update(Message::Input {
            text: "a".to_string(),
        });
        runtime.flush();
        let marker_after_first = runtime.top_undo_marker();

        runtime.update(Message::Input {
            text: "b".to_string(),
        });
        runtime.flush();
        let marker_after_second = runtime.top_undo_marker();

        assert_eq!(
            runtime.undo_manager.undo_count(),
            1,
            "consecutive inputs should merge into a single undo entry"
        );

        assert_eq!(
            marker_after_first, marker_after_second,
            "merged undo entry should keep the same top marker"
        );
    }

    #[test]
    fn test_merged_inputs_restore_selection_from_group_start() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.update(Message::Input {
            text: "y".to_string(),
        });
        runtime.update(Message::Input {
            text: "z".to_string(),
        });

        runtime.update(Message::Undo);

        assert_eq!(
            runtime.doc().to_plain_text(),
            "A",
            "merged inputs should undo as one content step"
        );
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "merged inputs should restore selection from the first input boundary"
        );
    }

    #[test]
    fn test_set_block_gap_keeps_history_selection_stack_aligned() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::SetBlockGap { gap: 101 });
        runtime.flush();

        assert_eq!(
            runtime.undo_manager.undo_count(),
            1,
            "changing block gap should create exactly one undo step"
        );
        assert_eq!(
            runtime.history_undo_marker_len(),
            1,
            "selection snapshot must be captured for setting changes"
        );

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "undo should restore the pre-change selection"
        );
    }

    #[test]
    fn test_set_layout_mode_keeps_history_selection_stack_aligned() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::SetLayoutMode {
            mode: LayoutMode::Continuous { max_width: 480.0 },
        });
        runtime.flush();

        assert_eq!(
            runtime.undo_manager.undo_count(),
            1,
            "changing layout mode should create exactly one undo step"
        );
        assert_eq!(
            runtime.history_undo_marker_len(),
            1,
            "selection snapshot must be captured for layout-mode changes"
        );

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "undo should restore the pre-change selection"
        );
    }

    #[test]
    fn test_undo_redo_layout_mode_change_marks_settings_dirty() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        let initial_layout_mode = runtime.doc().settings().layout_mode;
        let changed_layout_mode = LayoutMode::Continuous { max_width: 480.0 };

        runtime.update(Message::SetLayoutMode {
            mode: changed_layout_mode,
        });
        runtime.flush();
        runtime.tick();

        assert_eq!(runtime.doc().settings().layout_mode, changed_layout_mode);

        runtime.update(Message::Undo);
        runtime.tick();

        assert_eq!(
            runtime.doc().settings().layout_mode,
            initial_layout_mode,
            "undo should restore layout mode in document state"
        );
        assert!(
            has_dirty_flag(&runtime, DIRTY_SETTINGS),
            "undoing layout-mode change should emit settings payload for frontend"
        );

        runtime.update(Message::Redo);
        runtime.tick();

        assert_eq!(
            runtime.doc().settings().layout_mode,
            changed_layout_mode,
            "redo should re-apply layout mode in document state"
        );
        assert!(
            has_dirty_flag(&runtime, DIRTY_SETTINGS),
            "redoing layout-mode change should emit settings payload for frontend"
        );
    }

    #[test]
    fn test_selection_only_transaction_does_not_create_undo_entry() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 2,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 2,
            head_affinity: Affinity::Downstream,
        });
        runtime.flush();

        assert_eq!(
            runtime.undo_manager.undo_count(),
            0,
            "selection-only transaction must not create undo entries"
        );
        assert_eq!(
            runtime.history_undo_marker_len(),
            0,
            "selection snapshot stack must stay empty when no content changed"
        );
    }

    #[test]
    fn test_input_queues_pending_history_selection_before_flush() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });

        assert!(
            runtime.history_has_pending_selection(),
            "doc-changing input must queue an undo selection snapshot before flush"
        );
        assert!(
            runtime.history_has_pending_group_start_selection(),
            "doc-changing input must remember group-start selection before flush"
        );
    }

    #[test]
    fn test_replace_text_in_block_queues_pending_history_selection_before_flush() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 2) }
        };

        runtime
            .replace_text_in_block(p, 0, 1, "H")
            .expect("replace_text_in_block should succeed");

        assert!(
            runtime.history_has_pending_selection(),
            "block replacement must queue an undo selection snapshot before flush"
        );
        assert!(
            runtime.history_has_pending_group_start_selection(),
            "block replacement must remember group-start selection before flush"
        );
        assert!(
            runtime.state.pending_loro_commit,
            "block replacement should leave a pending loro commit"
        );
    }

    #[test]
    fn test_export_flushes_pending_commit_without_history_desync() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        assert!(
            runtime.state.pending_loro_commit,
            "precondition: input should leave a pending local commit before export"
        );

        let _ = runtime
            .export(DocExportMode::Snapshot)
            .expect("runtime export should succeed");

        assert_eq!(
            runtime.undo_manager.undo_count() as usize,
            runtime.history_undo_marker_len(),
            "export must not introduce undo entries without matching selection snapshots"
        );
        assert_eq!(
            runtime.undo_manager.undo_count(),
            1,
            "export should preserve one local undo item from the pending edit"
        );

        runtime.update(Message::Undo);
        assert_eq!(runtime.doc().to_plain_text(), "A");
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "undo after export should restore pre-input selection"
        );
    }

    #[test]
    fn test_runtime_starts_with_empty_undo_history_for_snapshot_doc() {
        let mut p = id!();
        let mut source = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        source.update(Message::Input {
            text: "x".to_string(),
        });
        source.flush();
        assert!(
            source.undo_manager.undo_count() > 0,
            "precondition: source runtime should have local undo history"
        );

        let snapshot = source
            .doc()
            .export(DocExportMode::Snapshot)
            .expect("source should export snapshot");
        let restored = Rc::new(Doc::from_snapshot(snapshot).expect("snapshot should decode"));
        let target_state = State::new(
            restored,
            Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
        );
        let target = Runtime::new(800.0, 1.0, target_state);

        assert_eq!(
            target.undo_manager.undo_count(),
            0,
            "new runtime should not inherit undo history that lacks selection snapshots"
        );
        assert_eq!(
            target.history_undo_marker_len(),
            0,
            "new runtime should start with empty undo selection stack"
        );
    }

    #[test]
    fn test_flush_uses_group_start_selection_when_pending_snapshot_is_missing() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.history_drop_pending_selection_for_test();
        runtime.flush();

        assert_eq!(runtime.undo_manager.undo_count(), 1);
        assert_eq!(
            runtime.history_undo_marker_len(),
            1,
            "group-start fallback should still capture one undo selection snapshot"
        );
        let marker = runtime
            .history_first_undo_marker()
            .expect("one undo marker should exist");
        let stored_selection = runtime
            .history_undo_selection_for_marker(marker)
            .expect("one undo selection snapshot should exist");
        assert_eq!(
            stored_selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "fallback should use selection captured at the start of the edit group"
        );

        runtime.update(Message::Undo);
        assert_eq!(
            runtime.state().selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "undo should restore the fallback-captured group-start selection"
        );
    }

    #[test]
    fn test_group_start_fallback_survives_click_transaction_before_flush() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.history_drop_pending_selection_for_test();

        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 0,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 0,
            head_affinity: Affinity::Downstream,
        });

        assert_eq!(runtime.undo_manager.undo_count(), 1);
        assert_eq!(
            runtime.history_undo_marker_len(),
            1,
            "fallback should keep undo snapshot even if a selection-only tx runs before flush"
        );
        let marker = runtime
            .history_first_undo_marker()
            .expect("one undo marker should exist");
        let stored_selection = runtime
            .history_undo_selection_for_marker(marker)
            .expect("one undo selection snapshot should exist");
        assert_eq!(
            stored_selection,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            "fallback must keep the original pre-input selection, not the later click position"
        );
    }

    #[test]
    fn test_undo_keeps_content_and_selection_in_lockstep_across_inputs_and_moves() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        let checkpoints = vec![
            (
                "yAx".to_string(),
                Selection::collapsed(Position::new(p, 2, Affinity::Downstream)),
            ),
            (
                "Ax".to_string(),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
            (
                "A".to_string(),
                Selection::collapsed(Position::new(p, 1, Affinity::Downstream)),
            ),
        ];

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 0,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 0,
            head_affinity: Affinity::Downstream,
        });
        runtime.update(Message::Input {
            text: "y".to_string(),
        });
        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 2,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 2,
            head_affinity: Affinity::Downstream,
        });
        runtime.update(Message::Input {
            text: "z".to_string(),
        });
        runtime.flush();

        assert_eq!(runtime.doc().to_plain_text(), "yAzx");

        for (expected_content, expected_selection) in checkpoints {
            runtime.update(Message::Undo);
            assert_eq!(
                runtime.doc().to_plain_text(),
                expected_content,
                "undo should restore content in step with selection checkpoints",
            );
            assert_eq!(
                runtime.state().selection,
                expected_selection,
                "selection must match the exact pre-input checkpoint for each undo step",
            );
        }
    }

    #[test]
    fn test_undo_falls_back_to_validated_current_selection_when_snapshot_missing() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.flush();

        runtime.history_clear_undo_snapshots_for_test();

        runtime.update(Message::Undo);

        assert_eq!(runtime.doc().to_plain_text(), "A");
        assert!(
            runtime
                .doc()
                .node(runtime.state().selection.head.node_id)
                .is_some(),
            "selection node_id should always be valid after undo"
        );
    }

    #[test]
    fn test_reconcile_does_not_backfill_missing_history_selection_snapshots() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::Input {
            text: "x".to_string(),
        });
        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 0,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 0,
            head_affinity: Affinity::Downstream,
        });
        runtime.update(Message::Input {
            text: "y".to_string(),
        });
        runtime.update(Message::SetSelection {
            anchor_node_id: p.to_string(),
            anchor_offset: 2,
            anchor_affinity: Affinity::Downstream,
            head_node_id: p.to_string(),
            head_offset: 2,
            head_affinity: Affinity::Downstream,
        });

        assert!(
            runtime.undo_manager.undo_count() >= 2,
            "precondition: test requires multiple undo steps"
        );

        let undo_count_before = runtime.undo_manager.undo_count() as usize;
        runtime.history_clear_undo_snapshots_for_test();
        runtime.sync_history_selection_state();
        assert_eq!(
            runtime.history_undo_marker_len(),
            0,
            "reconcile must not synthesize undo snapshots from current selection"
        );
        assert_eq!(
            runtime.undo_manager.undo_count() as usize,
            undo_count_before,
            "reconcile should not mutate undo manager state"
        );
        assert_eq!(
            runtime.history_redo_marker_len(),
            runtime.undo_manager.redo_count() as usize,
            "redo snapshots should still match redo stack size after reconcile"
        );

        runtime.update(Message::Undo);
        assert!(
            runtime
                .doc()
                .node(runtime.state().selection.head.node_id)
                .is_some(),
            "selection node_id should stay valid while undoing with missing snapshots"
        );
        assert!(
            runtime.history_undo_marker_len() <= runtime.undo_manager.undo_count() as usize,
            "undo snapshots should never exceed undo stack size"
        );
        assert_eq!(
            runtime.history_redo_marker_len(),
            runtime.undo_manager.redo_count() as usize,
            "redo snapshots should stay aligned with redo count"
        );
    }

    #[test]
    fn test_resolve_history_selection_falls_back_to_current_selection_when_node_id_is_invalid() {
        let mut p = id!();
        let runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        let snapshot = history_state::HistorySelectionSnapshot {
            selection: Selection::collapsed(Position::new(
                NodeId::from_string("00000000-0000-0000-0000-0000000000ff").unwrap(),
                1,
                Affinity::Downstream,
            )),
        };

        let restored = runtime.resolve_history_selection(&snapshot);
        assert_eq!(
            restored,
            Selection::collapsed(Position::new(p, 1, Affinity::Downstream))
        );
    }

    #[test]
    fn test_resolve_history_selection_clamps_offset_when_node_exists() {
        let mut p = id!();
        let runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        let snapshot = history_state::HistorySelectionSnapshot {
            selection: Selection::collapsed(Position::new(p, 999, Affinity::Upstream)),
        };

        let restored = runtime.resolve_history_selection(&snapshot);
        assert_eq!(
            restored,
            Selection::collapsed(Position::new(p, 1, Affinity::Upstream)),
            "existing node selection must still be offset-validated during history restore",
        );
    }

    #[test]
    fn test_update_repairs_invalid_selection_before_handling_message() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "A" }
                }
            }
            selection { (p, 1) }
        };

        runtime.state.selection = Selection::collapsed(Position::new(
            NodeId::from_string("00000000-0000-0000-0000-0000000000ff").unwrap(),
            0,
            Affinity::Downstream,
        ));

        runtime.update(Message::SelectAll);

        assert!(
            runtime
                .doc()
                .node(runtime.state().selection.head.node_id)
                .is_some(),
            "invalid selection should be repaired before message handling"
        );
    }

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
    fn html_pasted_collects_font_affected_nodes_from_anchor_to_head_range() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "A" } }
                @p2 paragraph { text { "B" } }
            }
            selection { (p1, 0) }
        };

        let selection = Selection::new(
            Position::new(p1, 0, Affinity::Downstream),
            Position::new(p2, 1, Affinity::Upstream),
        );

        runtime.process_effects(vec![
            Effect::FontDetected {
                family: "PasteFont".to_string(),
                weight: 400,
                codepoints: vec!['A' as u32],
            },
            Effect::HtmlPasted {
                selection,
                text: "A\nB".to_string(),
                styles: Vec::new(),
                paragraph_attrs: None,
            },
        ]);

        let (nodes, _) = runtime
            .missing_font_nodes
            .get(&(String::from("PasteFont"), 400))
            .expect("pasted range should be tracked for missing font");

        assert!(nodes.contains(&p1));
        assert!(nodes.contains(&p2));
        assert!(
            !nodes.contains(&NodeId::ROOT),
            "pasted range should avoid root fallback"
        );
    }

    #[test]
    fn import_updates_text_change_marks_font_dirty_without_settings_refresh() {
        let mut p1 = id!();
        let mut p2 = id!();
        let base_state = state! {
            doc {
                @p1 paragraph { text { "x" } }
                @p2 paragraph { text { "y" } }
            }
            selection { (p1, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source
            .replace_text_in_block(p1, 0, 1, "A")
            .expect("source should replace text");
        source.flush();
        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        target.loaded_font_codepoints.clear();
        target.missing_font_nodes.clear();

        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        assert_eq!(target.doc().to_plain_text(), "A\ny");
        assert!(
            has_dirty_flag(&target, DIRTY_DOC_CHANGED),
            "text-only external diff should mark doc changed"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_FONT_REQUIRED),
            "text-only external diff should request fonts"
        );
        assert!(
            !has_dirty_flag(&target, DIRTY_SETTINGS),
            "text-only external diff should not refresh settings payload"
        );
        assert!(
            !has_dirty_flag(&target, DIRTY_DEFAULT_ATTRS),
            "text-only external diff should not refresh default attrs payload"
        );

        let has_detected_a = target
            .loaded_font_codepoints
            .values()
            .any(|codepoints| codepoints.contains(&('A' as u32)));
        assert!(
            has_detected_a,
            "external doc change should detect codepoints from imported text"
        );

        let has_detected_unchanged_y = target
            .loaded_font_codepoints
            .values()
            .any(|codepoints| codepoints.contains(&('y' as u32)));
        assert!(
            !has_detected_unchanged_y,
            "external doc change should avoid requesting unchanged text node fonts"
        );

        let tracks_root = target
            .missing_font_nodes
            .values()
            .any(|(nodes, _)| nodes.contains(&NodeId::ROOT));
        assert!(
            !tracks_root,
            "external doc change font tracking should avoid root fallback when diff is text-only"
        );

        let has_non_root_tracking = target
            .missing_font_nodes
            .values()
            .any(|(nodes, _)| !nodes.is_empty());
        assert!(
            has_non_root_tracking,
            "external doc change font tracking should keep non-root nodes from diff"
        );
    }

    #[test]
    fn import_updates_does_not_create_undo_entries() {
        let mut p = id!();
        let base_state = state! {
            doc {
                @p paragraph { text { "A" } }
            }
            selection { (p, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );
        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source.update(Message::Input {
            text: "x".to_string(),
        });
        source.flush();
        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        assert_eq!(target.undo_manager.undo_count(), 0);
        assert_eq!(target.history_undo_marker_len(), 0);

        target
            .import_updates(&updates)
            .expect("target should import updates");

        assert_eq!(
            target.undo_manager.undo_count(),
            0,
            "external import should not create local undo entries"
        );
        assert_eq!(
            target.history_undo_marker_len(),
            0,
            "external import should not create local undo selection snapshots"
        );
    }

    #[test]
    fn import_updates_batch_does_not_create_undo_entries() {
        let mut p = id!();
        let base_state = state! {
            doc {
                @p paragraph { text { "A" } }
            }
            selection { (p, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );
        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );

        let version_1 = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source.update(Message::Input {
            text: "x".to_string(),
        });
        source.flush();
        let updates_1 = source
            .export(DocExportMode::UpdatesFrom { version: version_1 })
            .expect("source should export updates_1");

        let version_2 = source
            .export(DocExportMode::Version)
            .expect("source should export version_2");
        source.update(Message::Input {
            text: "y".to_string(),
        });
        source.flush();
        let updates_2 = source
            .export(DocExportMode::UpdatesFrom { version: version_2 })
            .expect("source should export updates_2");

        target
            .import_updates_batch(&[updates_1, updates_2])
            .expect("target should import updates batch");

        assert_eq!(
            target.undo_manager.undo_count(),
            0,
            "external import batch should not create local undo entries"
        );
        assert_eq!(
            target.history_undo_marker_len(),
            0,
            "external import batch should not create local undo selection snapshots"
        );
    }

    #[test]
    fn import_updates_settings_change_marks_settings_without_font_request() {
        let mut p1 = id!();
        let base_state = state! {
            doc {
                @p1 paragraph { text { "x" } }
            }
            selection { (p1, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source.update(Message::SetLayoutMode {
            mode: LayoutMode::Continuous { max_width: 480.0 },
        });
        source.flush();
        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        target.loaded_font_codepoints.clear();
        target.missing_font_nodes.clear();

        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        assert!(
            has_dirty_flag(&target, DIRTY_DOC_CHANGED),
            "settings-only external diff should mark doc changed"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_SETTINGS),
            "settings-only external diff should refresh settings payload"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_DEFAULT_ATTRS),
            "settings effect should also refresh default attrs payload"
        );
        assert!(
            !has_dirty_flag(&target, DIRTY_FONT_REQUIRED),
            "settings-only external diff should not request fonts"
        );
        assert_eq!(
            target.doc().settings().layout_mode,
            LayoutMode::Continuous { max_width: 480.0 }
        );
        assert!(
            target.missing_font_nodes.is_empty(),
            "settings-only external diff should not track font nodes"
        );
    }

    #[test]
    fn import_updates_unknown_root_path_falls_back_to_safe_full_scan() {
        let mut p1 = id!();
        let mut p2 = id!();
        let base_state = state! {
            doc {
                @p1 paragraph { text { "x" } }
                @p2 paragraph { text { "y" } }
            }
            selection { (p1, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");

        let meta = source.doc().loro_doc().get_map("meta");
        meta.insert("v", 1).expect("source should update meta map");
        source.doc().loro_doc().commit();

        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        target.loaded_font_codepoints.clear();
        target.missing_font_nodes.clear();

        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        assert!(
            has_dirty_flag(&target, DIRTY_DOC_CHANGED),
            "unknown external diff path should still mark doc changed"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_SETTINGS),
            "unknown external diff path should fall back to settings update"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_DEFAULT_ATTRS),
            "unknown external diff path should refresh default attrs as fallback"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_FONT_REQUIRED),
            "unknown external diff path should request fonts via full fallback"
        );

        let has_x = target
            .loaded_font_codepoints
            .values()
            .any(|codepoints| codepoints.contains(&('x' as u32)));
        let has_y = target
            .loaded_font_codepoints
            .values()
            .any(|codepoints| codepoints.contains(&('y' as u32)));
        assert!(
            has_x && has_y,
            "fallback font scan should include whole doc"
        );

        let tracks_root = target
            .missing_font_nodes
            .values()
            .any(|(nodes, _)| nodes.contains(&NodeId::ROOT));
        assert!(
            tracks_root,
            "fallback font tracking should use root invalidation"
        );
    }

    #[test]
    fn import_updates_mixed_text_and_settings_change_marks_both_paths() {
        let mut p1 = id!();
        let base_state = state! {
            doc {
                @p1 paragraph { text { "x" } }
            }
            selection { (p1, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p1, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source
            .replace_text_in_block(p1, 0, 1, "A")
            .expect("source should replace text");
        source.update(Message::SetLayoutMode {
            mode: LayoutMode::Continuous { max_width: 480.0 },
        });
        source.flush();

        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        target.loaded_font_codepoints.clear();
        target.missing_font_nodes.clear();

        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        assert_eq!(target.doc().to_plain_text(), "A");
        assert_eq!(
            target.doc().settings().layout_mode,
            LayoutMode::Continuous { max_width: 480.0 }
        );
        assert!(
            has_dirty_flag(&target, DIRTY_DOC_CHANGED),
            "mixed external diff should mark doc changed"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_SETTINGS),
            "mixed external diff should refresh settings payload"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_DEFAULT_ATTRS),
            "mixed external diff should refresh default attrs payload"
        );
        assert!(
            has_dirty_flag(&target, DIRTY_FONT_REQUIRED),
            "mixed external diff should request fonts for changed text"
        );

        let tracks_root = target
            .missing_font_nodes
            .values()
            .any(|(nodes, _)| nodes.contains(&NodeId::ROOT));
        assert!(
            !tracks_root,
            "mixed diff with explicit changed nodes should avoid root fallback"
        );
        let has_non_root_tracking = target
            .missing_font_nodes
            .values()
            .any(|(nodes, _)| !nodes.is_empty());
        assert!(
            has_non_root_tracking,
            "mixed diff should keep non-root font invalidation targets"
        );
    }

    #[test]
    fn import_updates_add_table_column_recomputes_table_external_element_width() {
        let mut p = id!();
        let mut t = id!();
        let mut image_1 = id!();
        let mut image_2 = id!();
        let base_state = state! {
            doc {
                @p paragraph { text { "x" } }
                @t table {
                    table_row {
                        table_cell {
                            @image_1 image (id: Some("img-1".to_string()),)
                        }
                    }
                    table_row {
                        table_cell {
                            @image_2 image (id: Some("img-2".to_string()),)
                        }
                    }
                }
            }
            selection { (p, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );
        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();

        let external_width = |runtime: &Runtime, node_id: NodeId| {
            runtime
                .pages()
                .iter()
                .flat_map(|page| page.external_elements())
                .find_map(|(_, ext)| (ext.id == node_id).then_some(ext.size.width))
                .expect("external element should be present in page layout")
        };

        let before_width = external_width(&target, image_2);

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source.update(Message::AddTableColumn {
            table_id: t.to_string(),
            col: 0,
            before: false,
        });
        source.flush();
        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");

        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        let after_width = external_width(&target, image_2);
        assert!(
            after_width < before_width,
            "remote table column insertion should shrink existing table external width: before={before_width}, after={after_width}"
        );
    }

    #[test]
    fn import_updates_table_attr_change_does_not_invalidate_table_descendant_cache() {
        let mut p = id!();
        let mut table = id!();
        let mut row = id!();
        let mut cell = id!();
        let base_state = state! {
            doc {
                @p paragraph { text { "x" } }
                @table table {
                    @row table_row {
                        @cell table_cell {
                            paragraph { text { "cell" } }
                        }
                    }
                }
            }
            selection { (p, 0) }
        };
        let snapshot = base_state
            .doc
            .export(DocExportMode::Snapshot)
            .expect("base should export snapshot");

        let mut source = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(
                    Doc::from_snapshot(snapshot.clone()).expect("test: snapshot should decode"),
                ),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );
        let mut target = Runtime::new(
            800.0,
            1.0,
            State::new(
                Rc::new(Doc::from_snapshot(snapshot).expect("test: snapshot should decode")),
                Selection::collapsed(Position::new(p, 0, Affinity::Downstream)),
            ),
        );

        source.tick();
        target.tick();
        assert!(
            target.is_layout_cached(cell),
            "precondition: table cell layout should be cached"
        );

        let version = source
            .export(DocExportMode::Version)
            .expect("source should export version");
        source.update(Message::SetTableAlign {
            table_id: table.to_string(),
            align: TableAlign::Center,
        });
        source.flush();

        let updates = source
            .export(DocExportMode::UpdatesFrom { version })
            .expect("source should export incremental updates");
        target
            .import_updates(&updates)
            .expect("target should import updates");
        target.tick();

        assert!(
            target.is_layout_cached(cell),
            "remote table attr change should keep descendant layout cache (node-level invalidation only)"
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
        let (page_idx, rect) = Cursor::bounds(&ctx, runtime.pages(), target_pos.clone())
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
        let Some(Node::Text(second_text)) = second_node.node() else {
            panic!("Second node is not text");
        };

        let start_of_second = calculate_offset_before_child(&paragraph, second);
        let end_of_second = start_of_second + second_text.text.char_len();

        let target_pos = Position::new(p, end_of_second, Affinity::default());
        let ctx = NavigationContext::new(&runtime.state.doc);
        let (page_idx, rect) = Cursor::bounds(&ctx, runtime.pages(), target_pos.clone())
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
            Cursor::bounds(&ctx, rt.pages(), image_pos).expect("Failed to get image bounds");

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
            .find(|c| matches!(c.node(), Some(Node::OrderedList(_))))
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

        runtime.update(Message::SetParagraphIndent { indent: 100 });
        runtime.flush();
        runtime.layout();

        let p_layout_initial = runtime
            .cached_layout(p)
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
            .cached_layout(p)
            .expect("Paragraph layout should be cached");

        assert!(
            !Rc::ptr_eq(&p_layout_initial, &p_layout_after),
            "Layout should be recomputed (fix verified)"
        );
    }

    #[test]
    fn layout_changed_effect_does_not_invalidate_layout_cache() {
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
        assert!(
            runtime.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        runtime.process_effects(vec![Effect::LayoutChanged]);

        assert!(
            runtime.is_layout_cached(p),
            "LayoutChanged should not invalidate layout cache by itself"
        );
    }

    #[test]
    fn settings_changed_effect_invalidates_layout_cache() {
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
        assert!(
            runtime.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        runtime.process_effects(vec![Effect::SettingsChanged]);

        assert!(
            !runtime.is_layout_cached(p),
            "SettingsChanged should invalidate layout cache"
        );
    }

    #[test]
    fn full_layout_invalidation_effect_invalidates_layout_cache() {
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
        assert!(
            runtime.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        runtime.process_effects(vec![Effect::FullLayoutInvalidation]);

        assert!(
            !runtime.is_layout_cached(p),
            "FullLayoutInvalidation should invalidate layout cache"
        );
    }

    #[test]
    fn structure_mutation_on_root_invalidates_layout_cache() {
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
        assert!(
            runtime.is_layout_cached(p),
            "precondition: paragraph layout should be cached"
        );

        runtime.process_effects(vec![Effect::NodeMutated {
            node_id: NodeId::ROOT,
            kind: MutationKind::Structure,
        }]);

        assert!(
            !runtime.is_layout_cached(p),
            "Structure mutation on ROOT should invalidate full layout cache"
        );
    }

    #[test]
    fn mixed_effects_merge_to_subtree_invalidation() {
        let mut list = id!();
        let mut item = id!();
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @list ordered_list {
                    @item list_item {
                        paragraph {
                            text { "1" }
                        }
                    }
                }
                @p paragraph {
                    text { "tail" }
                }
            }
            selection { (p, 0) }
        };

        runtime.layout();
        assert!(
            runtime.is_layout_cached(item),
            "precondition: list item layout should be cached"
        );

        runtime.process_effects(vec![
            Effect::NodeMutated {
                node_id: list,
                kind: MutationKind::Attr,
            },
            Effect::NodeMutated {
                node_id: list,
                kind: MutationKind::Structure,
            },
        ]);

        assert!(
            !runtime.is_layout_cached(item),
            "subtree invalidation should win over node invalidation for the same node"
        );
    }

    #[test]
    fn table_attr_mutation_does_not_over_escalate_to_subtree_cache_invalidation() {
        let mut table = id!();
        let mut row = id!();
        let mut cell = id!();
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @table table {
                    @row table_row {
                        @cell table_cell {
                            paragraph {
                                text { "cell" }
                            }
                        }
                    }
                }
                @p paragraph {
                    text { "tail" }
                }
            }
            selection { (p, 0) }
        };

        runtime.layout();
        assert!(
            runtime.is_layout_cached(cell),
            "precondition: table cell layout should be cached"
        );

        runtime.process_effects(vec![Effect::NodeMutated {
            node_id: table,
            kind: MutationKind::Attr,
        }]);

        assert!(
            runtime.is_layout_cached(cell),
            "Attr mutation should not invalidate table descendants as subtree"
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
        let Some(Node::Callout(callout_data)) = callout_node.node() else {
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
        let Some(Node::Callout(callout_data)) = callout_node.node() else {
            panic!("node should be callout");
        };
        assert_eq!(callout_data.variant, CalloutVariant::Success);
    }

    mod get_annotations_at_position {
        use super::*;

        fn annotations_at(runtime: &Runtime, node_id: NodeId, offset: usize) -> Vec<Annotation> {
            let position = Position::new(node_id, offset, Affinity::default());
            runtime.get_annotations_at_position(&position)
        }

        #[test]
        fn no_annotations_on_plain_text() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "hello" }
                    }
                }
                selection { (p, 0) }
            };

            for offset in 0..=5 {
                assert_eq!(annotations_at(&runtime, p, offset), vec![]);
            }
        }

        #[test]
        fn cursor_inside_link_returns_annotation() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "aa" @[link("https://x.com")], "bb" }
                    }
                }
                selection { (p, 0) }
            };

            let link = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://x.com".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![link.clone()]);
        }

        #[test]
        fn cursor_after_link_end_returns_empty() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "aa" @[link("https://x.com")], "bb" }
                    }
                }
                selection { (p, 0) }
            };

            assert_eq!(annotations_at(&runtime, p, 2), vec![]);
            assert_eq!(annotations_at(&runtime, p, 3), vec![]);
            assert_eq!(annotations_at(&runtime, p, 4), vec![]);
        }

        #[test]
        fn cursor_inside_ruby_returns_annotation() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "abc" @[ruby("かな")], "def" }
                    }
                }
                selection { (p, 0) }
            };

            let ruby = Annotation::Ruby(crate::model::RubyAnnotation {
                text: "かな".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![ruby.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![ruby.clone()]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![ruby.clone()]);
        }

        #[test]
        fn cursor_after_ruby_end_returns_empty() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "abc" @[ruby("かな")], "def" }
                    }
                }
                selection { (p, 0) }
            };

            assert_eq!(annotations_at(&runtime, p, 3), vec![]);
        }

        #[test]
        fn plain_before_link_after_plain() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "aa", "bb" @[link("https://x.com")], "cc" }
                    }
                }
                selection { (p, 0) }
            };

            let link = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://x.com".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![]);
            assert_eq!(annotations_at(&runtime, p, 3), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 4), vec![]);
            assert_eq!(annotations_at(&runtime, p, 5), vec![]);
            assert_eq!(annotations_at(&runtime, p, 6), vec![]);
        }

        #[test]
        fn adjacent_different_annotations() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "aa" @[link("https://a.com")], "bb" @[link("https://b.com")], "cc" }
                    }
                }
                selection { (p, 0) }
            };

            let link_a = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://a.com".into(),
            });
            let link_b = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://b.com".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![link_a.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![link_a.clone()]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![]);
            assert_eq!(annotations_at(&runtime, p, 3), vec![link_b.clone()]);
            assert_eq!(annotations_at(&runtime, p, 4), vec![]);
        }

        #[test]
        fn annotation_at_text_start() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "ab" @[link("https://x.com")] }
                    }
                }
                selection { (p, 0) }
            };

            let link = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://x.com".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![]);
        }

        #[test]
        fn annotation_covers_entire_text() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "abc" @[link("https://x.com")] }
                    }
                }
                selection { (p, 0) }
            };

            let link = Annotation::Link(crate::model::LinkAnnotation {
                href: "https://x.com".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![link.clone()]);
            assert_eq!(annotations_at(&runtime, p, 3), vec![]);
        }

        #[test]
        fn annotation_with_multibyte_characters() {
            let mut p = id!();
            let runtime = runtime! {
                doc {
                    @p paragraph {
                        text { "가나" @[ruby("かな")], "다" }
                    }
                }
                selection { (p, 0) }
            };

            let ruby = Annotation::Ruby(crate::model::RubyAnnotation {
                text: "かな".into(),
            });

            assert_eq!(annotations_at(&runtime, p, 0), vec![ruby.clone()]);
            assert_eq!(annotations_at(&runtime, p, 1), vec![ruby.clone()]);
            assert_eq!(annotations_at(&runtime, p, 2), vec![]);
            assert_eq!(annotations_at(&runtime, p, 3), vec![]);
        }
    }

    #[test]
    fn insert_template_fragment_does_not_leave_empty_paragraph() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        rt.layout();

        let template_doc = Rc::new(Doc::new());
        let root = template_doc.node(NodeId::ROOT).unwrap();
        let tp_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();
        let tp = template_doc.node(tp_id).unwrap();
        tp.as_mut()
            .insert_child(
                0,
                Node::Text(TextNode {
                    text: Text::from("template text"),
                }),
            )
            .unwrap();
        template_doc.loro_doc().commit();

        let snapshot = template_doc.export(DocExportMode::Snapshot).unwrap();
        rt.insert_template_fragment(snapshot).unwrap();

        let root_children = rt.doc().get_children_ids(NodeId::ROOT);
        assert_eq!(
            root_children.len(),
            1,
            "should have exactly one paragraph after template insertion, got {}",
            root_children.len()
        );

        assert_eq!(rt.doc().to_plain_text(), "template text");
    }

    #[test]
    fn insert_template_fragment_clears_cascade_attrs_from_previous_paragraph() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        rt.layout();

        rt.transact(|tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "CustomFont".to_string(),
                })]),
            )?;
            Ok(true)
        });

        let template_doc = Rc::new(Doc::new());
        let root = template_doc.node(NodeId::ROOT).unwrap();
        let tp_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();
        let tp = template_doc.node(tp_id).unwrap();
        tp.as_mut()
            .insert_child(
                0,
                Node::Text(TextNode {
                    text: Text::from("template"),
                }),
            )
            .unwrap();
        template_doc.loro_doc().commit();

        let snapshot = template_doc.export(DocExportMode::Snapshot).unwrap();
        rt.insert_template_fragment(snapshot).unwrap();

        let root_children = rt.doc().get_children_ids(NodeId::ROOT);
        for &child_id in root_children.iter() {
            let node = rt.doc().node(child_id).unwrap();
            if let Some(cascade) = node.cascade_attrs() {
                let has_custom_font = cascade.iter().any(
                    |a| matches!(a, Attr::Style(Style::FontFamily(f)) if f.family == "CustomFont"),
                );
                assert!(
                    !has_custom_font,
                    "previous paragraph's cascade_attrs should not remain after template insertion"
                );
            }
        }
    }

    #[test]
    fn insert_template_fragment_updates_layout_width_from_template_settings() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        rt.update(Message::SetLayoutMode {
            mode: LayoutMode::Paginated {
                page_width: 320.0,
                page_height: 600.0,
                page_margin_top: 40.0,
                page_margin_bottom: 40.0,
                page_margin_left: 40.0,
                page_margin_right: 40.0,
            },
        });
        assert_eq!(rt.layout_engine.width(), 320.0);

        let template_doc = Rc::new(Doc::new());
        let root = template_doc.node(NodeId::ROOT).unwrap();
        let tp_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();
        let tp = template_doc.node(tp_id).unwrap();
        tp.as_mut()
            .insert_child(
                0,
                Node::Text(TextNode {
                    text: Text::from("template"),
                }),
            )
            .unwrap();
        let _ = template_doc.update_settings(|s| {
            s.layout_mode = LayoutMode::Paginated {
                page_width: 900.0,
                page_height: 1200.0,
                page_margin_top: 80.0,
                page_margin_bottom: 80.0,
                page_margin_left: 80.0,
                page_margin_right: 80.0,
            };
        });
        template_doc.loro_doc().commit();

        let snapshot = template_doc.export(DocExportMode::Snapshot).unwrap();
        rt.insert_template_fragment(snapshot).unwrap();

        assert_eq!(rt.layout_engine.width(), 900.0);
    }
}

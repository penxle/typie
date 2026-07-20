use editor_crdt::{Dot, Op};
use editor_model::{ChildView, DocView, EditOp, Modifier, ModifierType, NodeView};
use editor_resource::{FontRegistry, Resolution};
use editor_transaction::Effect;
use hashbrown::{HashMap, HashSet};
use std::collections::BTreeMap;

use crate::editor::{Editor, ManifestRequestClass};
use crate::error::EditorError;
use crate::event::{EditorEvent, FontData};
use crate::state_field::StateField;

pub(crate) type FontRequests = HashMap<(String, u16), HashMap<Dot, HashSet<u32>>>;

fn font_from_effective(eff: &BTreeMap<ModifierType, Modifier>) -> (String, u16) {
    let family = match eff.get(&ModifierType::FontFamily) {
        Some(Modifier::FontFamily { value }) => value.clone(),
        _ => String::new(),
    };
    let weight = match eff.get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => *value,
        _ => 400,
    };
    (family, weight)
}

fn collect_for_block(block: &NodeView, font_registry: &FontRegistry, output: &mut FontRequests) {
    let block_id = block.id();
    let mut has_char = false;
    // One font-key resolution per uniform run group; per leaf only the char
    // codepoint is read. The per-leaf effective lookup (plus a family String
    // clone and set allocation per char) made this scan a per-tick O(N · alloc)
    // tax on styling edits.
    let mut leaves = block
        .children()
        .filter_map(|c| match c {
            ChildView::Leaf(l) => Some(l),
            ChildView::Block(_) => None,
        })
        .peekable();
    let grouped = block.run_groups().map(|(_, n)| n).sum::<usize>() == block.leaf_child_count();
    if grouped {
        for (eff, len) in block.run_groups() {
            let mut chars: Vec<u32> = Vec::new();
            for _ in 0..len {
                let Some(leaf) = leaves.next() else {
                    break;
                };
                if let Some(ch) = leaf.as_char() {
                    chars.push(ch as u32);
                }
            }
            if chars.is_empty() {
                continue;
            }
            has_char = true;
            let cps = output
                .entry(font_from_effective(eff))
                .or_default()
                .entry(block_id)
                .or_default();
            cps.extend(chars);
            if let Some(Modifier::Ruby { text }) = eff.get(&ModifierType::Ruby) {
                cps.extend(text.chars().map(|c| c as u32));
            }
        }
    } else {
        for (slot, child) in block.children().enumerate() {
            let ChildView::Leaf(leaf) = child else {
                continue;
            };
            let Some(ch) = leaf.as_char() else {
                continue;
            };
            let Some(st) = block.leaf_state_at(slot) else {
                continue;
            };
            has_char = true;
            let eff = st.eff;
            let (family, weight) = font_from_effective(eff);
            let cps = output
                .entry((family, weight))
                .or_default()
                .entry(block_id)
                .or_default();
            cps.insert(ch as u32);
            if let Some(Modifier::Ruby { text }) = eff.get(&ModifierType::Ruby) {
                cps.extend(text.chars().map(|c| c as u32));
            }
        }
    }
    if !has_char && block.spec().is_textblock() {
        let (family, weight) = font_from_effective(block.effective());
        if let Some(family_id) = font_registry.intern_id(&family)
            && matches!(
                font_registry.resolve(family_id, weight, ' ' as u32),
                Resolution::Pending { .. } | Resolution::AwaitingManifest { .. }
            )
        {
            output
                .entry((family, weight))
                .or_default()
                .entry(block_id)
                .or_default()
                .insert(' ' as u32);
        }
    }
}

pub(crate) fn collect_block_recursive(
    block: &NodeView,
    font_registry: &FontRegistry,
    output: &mut FontRequests,
) {
    collect_for_block(block, font_registry, output);
    for child in block.child_blocks() {
        collect_block_recursive(&child, font_registry, output);
    }
}

pub(crate) fn collect_subtree_block_dots(block: &NodeView, output: &mut Vec<Dot>) {
    output.push(block.id());
    for child in block.child_blocks() {
        collect_subtree_block_dots(&child, output);
    }
}

pub(crate) fn collect_font_requests(view: &DocView, font_registry: &FontRegistry) -> FontRequests {
    let mut result = FontRequests::new();
    if let Some(root) = view.root() {
        collect_block_recursive(&root, font_registry, &mut result);
    }
    result
}

// eg-walker ops reference seq positions/dots rather than block ids, so deriving
// the affected blocks incrementally isn't a cheap lookup. The View already does a
// full re-measure per edit, so a full rescan here is consistent.
pub(crate) fn derive_font_updates_from_ops(
    view: &DocView,
    font_registry: &FontRegistry,
    _ops: &[Op<EditOp>],
) -> FontRequests {
    collect_font_requests(view, font_registry)
}

pub(crate) fn reresolve_fonts(editor: &mut Editor) -> Result<(), EditorError> {
    editor.requested_manifests.clear();
    editor.pending_font_index = None;
    editor.font_activity = true;
    editor.prefetch_backlog.clear();
    {
        let resource = editor.resource.lock().unwrap();
        let view = editor.state.view();
        editor.pending_fonts = collect_font_requests(&view, &resource.font_registry);
    }

    let requests: Vec<_> = editor
        .pending_fonts
        .iter()
        .map(|((family, weight), nodes)| {
            let all_cps: Vec<u32> = nodes
                .values()
                .flatten()
                .copied()
                .collect::<HashSet<u32>>()
                .into_iter()
                .collect();
            (family.clone(), *weight, all_cps)
        })
        .collect();

    editor.transact(|tr| {
        for (family, weight, codepoints) in requests {
            tr.push_effect(Effect::LoadFont {
                family,
                weight,
                codepoints,
            });
        }
        Ok(())
    })
}

fn invalidate_font_affected(editor: &mut Editor, affected_nodes: &[Dot]) {
    if affected_nodes.is_empty() {
        return;
    }
    let mut any = false;
    {
        let view = editor.state.view();
        for node_id in affected_nodes {
            if let Some(nv) = view.node(*node_id) {
                any |= editor.view.invalidate_measure_with_ancestors(&nv);
            }
        }
    }
    if any {
        editor.view.invalidate(&editor.state);
        // Real font metrics can introduce soft-wrap (page heights grow) and shift
        // line ascent/descent (caret coordinates change), so the host must re-query
        // both — otherwise the canvas stays sized for the pre-load layout.
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![
                StateField::Cursor,
                StateField::PageSizes,
                StateField::ExternalElements,
                StateField::TableOverlays,
                StateField::Placeholder,
            ],
        });
        editor.invalidate_render();
    }
}

pub(crate) fn resolve_pending_fonts(editor: &mut Editor) {
    if editor.pending_fonts.is_empty() {
        return;
    }
    let requests: Vec<(String, u16, Vec<u32>)> = editor
        .pending_fonts
        .iter()
        .map(|((family, weight), nodes)| {
            let cps: Vec<u32> = nodes
                .values()
                .flatten()
                .copied()
                .collect::<HashSet<u32>>()
                .into_iter()
                .collect();
            (family.clone(), *weight, cps)
        })
        .collect();
    for (family, weight, cps) in requests {
        editor.resolve_fonts(&family, weight, &cps);
    }
}

#[derive(Debug)]
pub(crate) enum FontLoadKind {
    Base,
    Chunk(u16),
}

#[derive(Debug)]
pub(crate) struct FontLoadNotice {
    pub(crate) family: String,
    pub(crate) weight: u16,
    pub(crate) kind: FontLoadKind,
}

/// Reverse index over `pending_fonts`, keyed by the awaited resolution target.
/// A chunk/base arrival then touches only the codepoints waiting on that exact
/// (family, weight[, chunk]) instead of re-resolving every pending codepoint.
/// Resolution routing depends only on configured families and manifests, so the
/// index stays valid until `pending_fonts` is mutated or a manifest/config
/// change reroutes resolution — both drop it for a rebuild.
#[derive(Default)]
pub(crate) struct PendingFontIndex {
    keys: Vec<(String, u16)>,
    by_target: HashMap<(u16, u16), HashMap<u16, Vec<(usize, Dot, u32)>>>,
}

impl PendingFontIndex {
    fn file(&mut self, target: &editor_resource::Target, entry: (usize, Dot, u32)) {
        self.by_target
            .entry((target.family_id, target.weight))
            .or_default()
            .entry(target.chunk_id)
            .or_default()
            .push(entry);
    }
}

fn remove_pending_cps(editor: &mut Editor, key: &(String, u16), removals: &[(Dot, u32)]) {
    let Some(nodes) = editor.pending_fonts.get_mut(key) else {
        return;
    };
    for (node, cp) in removals {
        if let Some(cps) = nodes.get_mut(node) {
            cps.remove(cp);
            if cps.is_empty() {
                nodes.remove(node);
            }
        }
    }
    if nodes.is_empty() {
        editor.pending_fonts.remove(key);
    }
}

fn build_pending_font_index(editor: &mut Editor, affected: &mut Vec<Dot>) -> PendingFontIndex {
    let mut index = PendingFontIndex::default();
    let mut removals: Vec<(usize, Vec<(Dot, u32)>)> = Vec::new();
    {
        let resource = editor.resource.lock().unwrap();
        for ((family, weight), nodes) in editor.pending_fonts.iter() {
            let Some(family_id) = resource.font_registry.intern_id(family) else {
                continue;
            };
            let key_idx = index.keys.len();
            index.keys.push((family.clone(), *weight));
            let mut ready: Vec<(Dot, u32)> = Vec::new();
            for (node_id, cps) in nodes {
                for &cp in cps {
                    match resource.font_registry.resolve(family_id, *weight, cp) {
                        Resolution::Ready(_) => {
                            ready.push((*node_id, cp));
                            affected.push(*node_id);
                        }
                        Resolution::Pending { target, .. } => {
                            index.file(&target, (key_idx, *node_id, cp));
                        }
                        Resolution::AwaitingManifest { .. } | Resolution::Missing => {}
                    }
                }
            }
            if !ready.is_empty() {
                removals.push((key_idx, ready));
            }
        }
    }
    for (key_idx, ready) in removals {
        let key = index.keys[key_idx].clone();
        remove_pending_cps(editor, &key, &ready);
    }
    index
}

/// Applies every font base/chunk arrival queued during this tick in one pass:
/// index lookups scoped to the arrived data, one invalidation for all affected
/// nodes. Replaces the per-event full rescan of `pending_fonts`, which made a
/// chunked font load O(files × pending codepoints).
pub(crate) fn flush_font_loads(editor: &mut Editor) {
    if editor.pending_font_loads.is_empty() {
        return;
    }
    let notices = std::mem::take(&mut editor.pending_font_loads);
    editor.font_activity = true;
    if editor.pending_fonts.is_empty() {
        editor.pending_font_index = None;
        return;
    }

    let mut affected: Vec<Dot> = Vec::new();
    if editor.pending_font_index.is_none() {
        let index = build_pending_font_index(editor, &mut affected);
        editor.pending_font_index = Some(index);
    }
    let mut index = editor.pending_font_index.take().unwrap_or_default();

    for notice in notices {
        let candidates: Vec<(usize, Dot, u32)> = {
            let resource = editor.resource.lock().unwrap();
            let Some(family_id) = resource.font_registry.intern_id(&notice.family) else {
                continue;
            };
            let target_key = (family_id, notice.weight);
            match notice.kind {
                FontLoadKind::Chunk(chunk_id) => index
                    .by_target
                    .get_mut(&target_key)
                    .and_then(|chunks| chunks.remove(&chunk_id))
                    .unwrap_or_default(),
                FontLoadKind::Base => index
                    .by_target
                    .remove(&target_key)
                    .map(|chunks| chunks.into_values().flatten().collect())
                    .unwrap_or_default(),
            }
        };
        if candidates.is_empty() {
            continue;
        }

        let base_event = matches!(notice.kind, FontLoadKind::Base);
        let mut removals: HashMap<usize, Vec<(Dot, u32)>> = HashMap::new();
        {
            let resource = editor.resource.lock().unwrap();
            for (key_idx, node_id, cp) in candidates {
                let (req_family, req_weight) = &index.keys[key_idx];
                let Some(req_family_id) = resource.font_registry.intern_id(req_family) else {
                    continue;
                };
                match resource
                    .font_registry
                    .resolve(req_family_id, *req_weight, cp)
                {
                    Resolution::Ready(_) => {
                        removals.entry(key_idx).or_default().push((node_id, cp));
                        affected.push(node_id);
                    }
                    Resolution::Pending { target, needs_base } => {
                        if base_event && !needs_base {
                            affected.push(node_id);
                        }
                        index.file(&target, (key_idx, node_id, cp));
                    }
                    Resolution::AwaitingManifest { .. } | Resolution::Missing => {}
                }
            }
        }
        for (key_idx, ready) in removals {
            let key = index.keys[key_idx].clone();
            remove_pending_cps(editor, &key, &ready);
        }
    }
    editor.pending_font_index = Some(index);

    affected.sort_unstable();
    affected.dedup();
    invalidate_font_affected(editor, &affected);
}

#[derive(Default)]
pub(crate) struct PrefetchBacklog {
    pub(crate) chunk_targets: HashSet<(u16, u16)>,
    pub(crate) manifest_targets: HashSet<(u16, u16)>,
}

impl PrefetchBacklog {
    pub(crate) fn is_empty(&self) -> bool {
        self.chunk_targets.is_empty() && self.manifest_targets.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.chunk_targets.clear();
        self.manifest_targets.clear();
    }
}

fn fonts_quiescent(editor: &Editor) -> bool {
    if editor
        .requested_manifests
        .values()
        .any(|class| *class == ManifestRequestClass::Required)
    {
        return false;
    }
    if let Some(index) = &editor.pending_font_index {
        return index
            .by_target
            .values()
            .flat_map(|chunks| chunks.values())
            .all(|refs| refs.is_empty());
    }
    let resource = editor.resource.lock().unwrap();
    for ((family, weight), nodes) in &editor.pending_fonts {
        let Some(family_id) = resource.font_registry.intern_id(family) else {
            continue;
        };
        for cps in nodes.values() {
            for &cp in cps {
                if matches!(
                    resource.font_registry.resolve(family_id, *weight, cp),
                    Resolution::Pending { .. }
                ) {
                    return false;
                }
            }
        }
    }
    true
}

/// Deferred prefetch: while any required font data is still in flight the
/// backlog only accumulates, so the network stays dedicated to first paint.
pub(crate) fn emit_prefetch_if_quiescent(editor: &mut Editor) {
    if !std::mem::take(&mut editor.font_activity) {
        return;
    }
    if editor.prefetch_backlog.is_empty() || !fonts_quiescent(editor) {
        return;
    }
    let backlog = std::mem::take(&mut editor.prefetch_backlog);
    let mut chunk_targets: Vec<(u16, u16)> = backlog.chunk_targets.into_iter().collect();
    chunk_targets.sort_unstable();
    let mut manifest_targets: Vec<(u16, u16)> = backlog.manifest_targets.into_iter().collect();
    manifest_targets.sort_unstable();

    let mut events: Vec<EditorEvent> = Vec::new();
    {
        let resource = editor.resource.lock().unwrap();
        for (family_id, weight) in chunk_targets {
            let Some(manifest) = resource.font_registry.manifest(family_id, weight) else {
                continue;
            };
            let mut prefetch: Vec<FontData> = Vec::new();
            if !resource.font_registry.is_base_loaded(family_id, weight) {
                prefetch.push(FontData::Base);
            }
            prefetch.extend(
                manifest
                    .all_chunk_ids()
                    .filter(|&cid| {
                        !resource
                            .font_registry
                            .is_chunk_loaded(family_id, weight, cid)
                    })
                    .map(|id| FontData::Chunk { id }),
            );
            if prefetch.is_empty() {
                continue;
            }
            let Some(family) = resource.font_registry.family_name_opt(family_id) else {
                continue;
            };
            events.push(EditorEvent::FontDataMissing {
                family: family.to_string(),
                weight,
                required: Vec::new(),
                prefetch,
            });
        }
        for (family_id, weight) in manifest_targets {
            if resource.font_registry.has_manifest(family_id, weight) {
                continue;
            }
            if editor
                .requested_manifests
                .contains_key(&(family_id, weight))
            {
                continue;
            }
            let Some(family) = resource.font_registry.family_name_opt(family_id) else {
                continue;
            };
            events.push(EditorEvent::FontDataMissing {
                family: family.to_string(),
                weight,
                required: Vec::new(),
                prefetch: vec![FontData::Manifest],
            });
            editor
                .requested_manifests
                .insert((family_id, weight), ManifestRequestClass::Prefetch);
        }
    }
    for event in events {
        editor.push_event(event);
    }
}

#[cfg(test)]
mod base_style_tests {
    use super::*;
    use editor_macros::state;
    use editor_resource::FontRegistry;

    #[test]
    fn collect_requests_base_style_font_without_panic() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] { p1: paragraph { text("Hi") } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        let key = ("Pretendard".to_string(), 400u16);
        assert!(
            result.contains_key(&key),
            "base style font must be requested; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p1));
    }

    #[test]
    fn collect_uses_effective_weight_override() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] { p1: paragraph { text("Hi") [font_weight(700)] } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        // The per-run weight override wins over the inherited base-style weight.
        let key = ("Pretendard".to_string(), 700u16);
        assert!(
            result.contains_key(&key),
            "effective weight override must be requested; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p1));
    }

    #[test]
    fn derive_font_updates_rescans_doc() {
        let (state, _p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] { p1: paragraph { text("Hi") } }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        // derive is a full rescan in the eg-walker model (ops are not consulted), so
        // an empty op slice still yields the document's current font requests.
        let requests = derive_font_updates_from_ops(&view, &FontRegistry::new(), &[]);
        assert!(
            !requests.is_empty(),
            "a doc with a base-style font must derive font requests"
        );
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;

    #[test]
    fn collect_from_single_text_node() {
        let (state, p1) = state! {
            doc {
                root [font_family("Arial".to_string()), font_weight(400)] {
                    p1: paragraph { text("AB") }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        let key = ("Arial".to_string(), 400u16);
        assert!(result.contains_key(&key));

        let nodes = &result[&key];
        assert!(nodes.contains_key(&p1));

        let cps = &nodes[&p1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
    }

    #[test]
    fn collect_inherits_font_from_ancestor() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("A")
                        text("B") [font_weight(700)]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        assert!(result.contains_key(&("Pretendard".to_string(), 400)));
        assert!(result.contains_key(&("Pretendard".to_string(), 700)));
        // Both text runs share the same containing paragraph.
        assert!(result[&("Pretendard".to_string(), 400)].contains_key(&p1));
        assert!(result[&("Pretendard".to_string(), 700)].contains_key(&p1));
    }

    #[test]
    fn collect_groups_codepoints_per_node() {
        let (state, p1, p2) = state! {
            doc {
                root [font_family("Arial".to_string()), font_weight(400)] {
                    p1: paragraph { text("AB") }
                    p2: paragraph { text("CD") }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());
        let nodes = &result[&("Arial".to_string(), 400)];

        assert_eq!(nodes.len(), 2);
        assert!(nodes[&p1].contains(&('A' as u32)));
        assert!(nodes[&p2].contains(&('C' as u32)));
    }

    #[test]
    fn collect_includes_ruby_text_codepoints() {
        let (state, p1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    p1: paragraph {
                        text("AB") [ruby(text: "한자".to_string())]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        let key = ("Pretendard".to_string(), 400u16);
        let cps = &result[&key][&p1];
        assert!(cps.contains(&('A' as u32)));
        assert!(cps.contains(&('B' as u32)));
        assert!(cps.contains(&('한' as u32)));
        assert!(cps.contains(&('자' as u32)));
    }

    #[test]
    fn collect_requests_fold_title_weight_override() {
        let (state, ft1) = state! {
            doc {
                root [font_family("Pretendard".to_string()), font_weight(400)] {
                    fold {
                        ft1: fold_title { text("1234") }
                        fold_content { paragraph { text("c") } }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 0)
        };

        let view = state.view();
        let result = collect_font_requests(&view, &FontRegistry::new());

        // FoldTitle imposes weight 500 on its text; the render path requests that
        // weight, so font collection must request it too (else the glyphs never load).
        let key = ("Pretendard".to_string(), 500u16);
        assert!(
            result.contains_key(&key),
            "missing (Pretendard, 500); keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&ft1));
    }

    #[test]
    fn derive_font_updates_rescans_target_font_scope() {
        let (state, _p1, p2) = state! {
            doc {
                root [font_family("Source".to_string()), font_weight(400)] {
                    p1: paragraph { text("나") }
                    p2: paragraph {
                        text("가") [font_family("Target".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 0)
        };

        let view = state.view();
        // Full rescan picks up the Target-fonted block's codepoints regardless of which op triggered it.
        let result = derive_font_updates_from_ops(&view, &FontRegistry::new(), &[]);

        let key = ("Target".to_string(), 700u16);
        assert!(
            result.contains_key(&key),
            "target scope should request target font; keys = {:?}",
            result.keys().collect::<Vec<_>>()
        );
        assert!(result[&key].contains_key(&p2));
        assert!(result[&key][&p2].contains(&('가' as u32)));
    }
}

use crate::model::tree::node_ref::CASCADE_ATTRS_KEY;
use crate::model::tree::{BlockTextIterator, DocInner, NodeRef, TextSegmentIterator};
use crate::model::*;
use crate::schema::Schema;
use anyhow::{Context, Result};
use loro::{ExpandType, ExportMode, Frontiers, LoroDoc, LoroMap, StyleConfig};
use rustc_hash::FxHashSet;
use serde::Deserialize;
use std::rc::Rc;

const SETTINGS_KEY: &str = "settings";

#[derive(Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(from_wasm_abi))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DocExportMode {
    Snapshot,
    Version,
    AllUpdates,
    UpdatesFrom {
        #[serde(with = "serde_bytes")]
        #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
        version: Vec<u8>,
    },
}

#[derive(Debug)]
pub struct Doc {
    inner: DocInner,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMapping {
    #[serde(serialize_with = "serialize_node_id")]
    pub node_id: NodeId,
    pub text_start: usize,
    pub text_end: usize,
    pub block_offset: usize,
}

fn serialize_node_id<S>(node_id: &NodeId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&node_id.to_string())
}

impl Doc {
    pub fn new() -> Self {
        let schema = Rc::new(Schema::default());

        let loro = LoroDoc::new();
        loro.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::None,
        }));

        let map = loro.get_map(SETTINGS_KEY);
        let mut settings = DocumentSettings::new();
        settings
            .encode(&map)
            .expect("Doc::new: failed to encode default settings");

        let nodes = loro.get_map("nodes");

        let map = nodes
            .insert_container(&NodeId::ROOT.to_string(), LoroMap::new())
            .expect("Doc::new: failed to insert root node container");
        let mut root = Node::Root(RootNode::default());
        root.encode(&map)
            .expect("Doc::new: failed to encode root node");

        {
            let defaults = DefaultAttrs::default();
            let cascade_map = map
                .insert_container(CASCADE_ATTRS_KEY, LoroMap::new())
                .expect("Doc::new: failed to insert cascade attrs container");
            for attr in defaults.to_attrs() {
                cascade_map
                    .insert(attr.key(), attr.to_loro_value())
                    .expect("Doc::new: failed to insert default cascade attr");
            }
        }

        let inner = DocInner::new(loro, schema);

        Self { inner }
    }

    pub fn from_snapshot(snapshot: Vec<u8>) -> anyhow::Result<Self> {
        let schema = Rc::new(Schema::default());

        let loro = LoroDoc::from_snapshot(&snapshot).context("Failed to decode snapshot")?;
        loro.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::None,
        }));

        let inner = DocInner::new(loro, schema);

        Ok(Self { inner })
    }

    pub fn loro_doc(&self) -> &LoroDoc {
        &self.inner.loro
    }

    pub fn frontiers(&self) -> Frontiers {
        self.inner.loro.oplog_frontiers()
    }

    pub fn export(&self, mode: DocExportMode) -> Result<Vec<u8>> {
        match mode {
            DocExportMode::Snapshot => self
                .inner
                .loro
                .export(ExportMode::snapshot())
                .context("Failed to export snapshot"),
            DocExportMode::Version => Ok(self.inner.loro.oplog_vv().encode()),
            DocExportMode::AllUpdates => self
                .inner
                .loro
                .export(ExportMode::all_updates())
                .context("Failed to export all updates"),
            DocExportMode::UpdatesFrom { version } => {
                let vv = loro::VersionVector::decode(&version)?;
                self.inner
                    .loro
                    .export(ExportMode::updates(&vv))
                    .context("Failed to export updates from version")
            }
        }
    }

    pub fn import_updates(&self, updates: &[u8]) -> Result<()> {
        self.inner
            .loro
            .import(updates)
            .context("Failed to import updates")?;
        Ok(())
    }

    pub fn import_updates_batch(&self, updates_batch: &[Vec<u8>]) -> Result<()> {
        self.inner
            .loro
            .import_batch(updates_batch)
            .context("Failed to import updates batch")?;
        Ok(())
    }

    pub fn revert_to(&self, frontiers: &Frontiers) -> Result<()> {
        self.inner.loro.revert_to(frontiers)?;
        Ok(())
    }

    pub fn checkout(&self, frontiers: &Frontiers) -> Result<()> {
        self.inner.loro.checkout(frontiers)?;
        Ok(())
    }

    pub fn checkout_to_latest(&self) -> Result<()> {
        self.inner.loro.checkout_to_latest();
        Ok(())
    }

    pub fn is_detached(&self) -> bool {
        self.inner.loro.is_detached()
    }

    pub fn schema(&self) -> &Schema {
        &self.inner.schema
    }

    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        NodeRef::new(&self.inner, id)
    }

    pub fn all_remarks(&self) -> Vec<(NodeId, Remark)> {
        let mut result = Vec::new();
        let mut stack = vec![NodeId::ROOT];
        while let Some(id) = stack.pop() {
            let Some(node_ref) = self.node(id) else {
                continue;
            };
            // remarks() is already sorted by created_at within a node
            for remark in node_ref.remarks() {
                result.push((id, remark));
            }
            let children = self.get_children_ids(id);
            for &child_id in children.iter().rev() {
                stack.push(child_id);
            }
        }
        result
    }

    pub fn to_plain_text(&self) -> String {
        let mut text = String::new();

        for (_, block_text) in self.iter_blocks() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&block_text);
        }
        text
    }

    pub fn to_text_with_mappings(&self) -> (String, Vec<TextMapping>) {
        let mut full_text = String::new();
        let mut mappings = Vec::new();
        let mut char_offset = 0usize;

        for (block_id, _) in self.iter_blocks() {
            let mut block_offset = 0;

            for &child_id in self.get_children_ids(block_id).iter() {
                match self.get_node_type(child_id) {
                    Some(NodeType::Text) => {
                        if let Some(segments) = self.get_text_segments(child_id) {
                            for seg in segments {
                                let text_start = char_offset;
                                let char_len = seg.text.chars().count();
                                full_text.push_str(&seg.text);
                                char_offset += char_len;

                                mappings.push(TextMapping {
                                    node_id: block_id,
                                    text_start,
                                    text_end: char_offset,
                                    block_offset,
                                });

                                block_offset += char_len;
                            }
                        }
                    }
                    Some(NodeType::HardBreak) => {
                        let text_start = char_offset;
                        full_text.push('\n');
                        char_offset += 1;

                        mappings.push(TextMapping {
                            node_id: block_id,
                            text_start,
                            text_end: char_offset,
                            block_offset,
                        });

                        block_offset += 1;
                    }
                    _ => {}
                }
            }

            if !full_text.is_empty() && !full_text.ends_with('\n') {
                full_text.push('\n');
                let text_start = char_offset;
                char_offset += 1;

                mappings.push(TextMapping {
                    node_id: block_id,
                    text_start,
                    text_end: char_offset,
                    block_offset,
                });
            }
        }

        (full_text, mappings)
    }

    pub fn settings(&self) -> DocumentSettings {
        let map = self.inner.loro.get_map(SETTINGS_KEY);
        DocumentSettings::decode(&map).unwrap_or_default()
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut DocumentSettings)) -> Result<()> {
        let map = self.inner.loro.get_map(SETTINGS_KEY);
        let mut settings = DocumentSettings::decode(&map)?;
        f(&mut settings);
        settings.encode(&map)?;
        self.inner.loro.commit();
        Ok(())
    }

    pub fn update_default_attrs(&self, attrs: DefaultAttrs) -> Result<()> {
        if let Some(root) = self.node(NodeId::ROOT) {
            root.as_mut().set_cascade_attrs(&attrs.to_attrs())?;
        }
        self.inner.loro.commit();
        Ok(())
    }

    pub fn default_attrs(&self) -> DefaultAttrs {
        self.node(NodeId::ROOT)
            .and_then(|root| root.cascade_attrs())
            .map(|attrs| DefaultAttrs::from_attrs(&attrs))
            .unwrap_or_default()
    }

    /// Exhaustive document validation. Traverses every reachable node and checks
    /// content schema, styles, annotations, parent-child consistency, and
    /// grandparent constraints. Intended for scripts/WASM tooling only — too
    /// expensive for hot paths.
    pub fn validate_exhaustive(&self) -> Result<()> {
        // 1. Root node must exist
        let root = self
            .node(NodeId::ROOT)
            .context("Root node does not exist")?;
        anyhow::ensure!(
            root.node_type() == Some(NodeType::Root),
            "Root node has wrong type: {:?}",
            root.node_type()
        );

        // 2. Collect all reachable nodes via DFS from root
        let mut visited = FxHashSet::default();
        let mut stack: Vec<(NodeId, Vec<NodeType>)> = vec![(NodeId::ROOT, Vec::new())];
        let mut parent_map: Vec<(NodeId, Option<NodeId>)> = Vec::new();

        while let Some((id, inherited_forbidden)) = stack.pop() {
            if !visited.insert(id) {
                anyhow::bail!("Cycle detected: node {} visited twice", id);
            }

            let node_ref = self
                .node(id)
                .with_context(|| format!("Node {} is referenced but does not exist", id))?;

            let node_type = node_ref
                .node_type()
                .with_context(|| format!("Node {} could not be decoded", id))?;

            // Check forbidden_descendants violation
            anyhow::ensure!(
                !inherited_forbidden.contains(&node_type),
                "Node {} ({:?}) is a forbidden descendant of an ancestor",
                id,
                node_type
            );

            let expected_parent = if id == NodeId::ROOT {
                None
            } else {
                Some(node_ref.parent_id())
            };
            parent_map.push((id, expected_parent.flatten()));

            // Build forbidden list for children
            let spec = self.inner.schema.node_spec(node_type);
            let mut child_forbidden = inherited_forbidden;
            if let Some(forbidden) = spec.forbidden_descendants {
                for &ft in forbidden {
                    if !child_forbidden.contains(&ft) {
                        child_forbidden.push(ft);
                    }
                }
            }

            let children_ids = self.get_children_ids(id);
            // Push in reverse so we visit in order
            for &child_id in children_ids.iter().rev() {
                stack.push((child_id, child_forbidden.clone()));
            }
        }

        // 3. Validate parent-child consistency
        for &(id, expected_parent) in &parent_map {
            if id == NodeId::ROOT {
                continue;
            }

            let parent_id =
                expected_parent.with_context(|| format!("Non-root node {} has no parent", id))?;

            anyhow::ensure!(
                visited.contains(&parent_id),
                "Node {} has parent {} which is not reachable",
                id,
                parent_id
            );

            let parent_children = self.get_children_ids(parent_id);
            anyhow::ensure!(
                parent_children.contains(&id),
                "Node {} claims parent {} but is not in parent's children list",
                id,
                parent_id
            );
        }

        // 4. Validate each node: content schema, styles, annotations, grandparent constraint
        for &id in &visited {
            self.validate_node(id)
                .with_context(|| format!("Validation failed at node {}", id))?;

            // grandparent_must_be check
            let node_ref = self.node(id).unwrap();
            let Some(nt) = node_ref.node_type() else {
                continue;
            };
            let spec = self.inner.schema.node_spec(nt);
            if let Some(required_grandparent) = spec.grandparent_must_be {
                let parent_id = node_ref.parent_id();
                let grandparent_type = parent_id
                    .and_then(|pid| self.node(pid))
                    .and_then(|p| p.parent_id())
                    .and_then(|gid| self.node(gid))
                    .and_then(|g| g.node_type());

                anyhow::ensure!(
                    grandparent_type == Some(required_grandparent),
                    "Node {} ({:?}) requires grandparent {:?}, but got {:?}",
                    id,
                    node_ref.node_type(),
                    required_grandparent,
                    grandparent_type
                );
            }
        }

        Ok(())
    }

    pub fn validate_node(&self, node_id: NodeId) -> Result<()> {
        let node_ref = self.node(node_id).context("Node not found")?;
        let node_type = node_ref.node_type().context("Node decode failed")?;
        let spec = self.inner.schema.node_spec(node_type);

        let child_types: Vec<NodeType> = node_ref
            .children()
            .filter_map(|child| child.node().map(|n| n.as_type()))
            .collect();

        spec.content.validate(&child_types).with_context(|| {
            format!(
                "Content validation failed for '{:?}' at node {}",
                node_type, node_id
            )
        })?;

        if let Some(Node::Text(text_node)) = node_ref.node() {
            let allowed_styles = self.allowed_styles_for(node_id);
            let allowed_annotations = self.allowed_annotations_for(node_id);

            let segments = text_node.text.get_segments();
            for seg in segments {
                for style in &seg.styles {
                    let style_type = style.as_type();
                    if !allowed_styles.contains(&style_type) {
                        anyhow::bail!("Style '{:?}' not allowed at node {}", style_type, node_id,);
                    }
                }
                for ann in &seg.annotations {
                    let ann_type = ann.as_type();
                    if !allowed_annotations.contains(&ann_type) {
                        anyhow::bail!(
                            "Annotation '{:?}' not allowed at node {}",
                            ann_type,
                            node_id,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn allowed_styles_for(&self, node_id: NodeId) -> FxHashSet<StyleType> {
        self.collect_allowed(node_id, |spec| spec.styles, true)
    }

    pub fn allowed_annotations_for(&self, node_id: NodeId) -> FxHashSet<AnnotationType> {
        self.collect_allowed(node_id, |spec| spec.annotations, true)
    }

    pub fn allowed_styles_for_content(&self, node_id: NodeId) -> FxHashSet<StyleType> {
        self.collect_allowed(node_id, |spec| spec.styles, false)
    }

    fn collect_allowed<T: Eq + std::hash::Hash + Copy + 'static>(
        &self,
        node_id: NodeId,
        get_field: impl Fn(&crate::schema::NodeSpec) -> Option<&'static [T]>,
        skip_self: bool,
    ) -> FxHashSet<T> {
        let mut allowed = FxHashSet::default();
        let Some(node) = self.node(node_id) else {
            return allowed;
        };

        let skip_count = if skip_self { 1 } else { 0 };
        for ancestor in node.ancestors().skip(skip_count) {
            let Some(node_type) = ancestor.node_type() else {
                continue;
            };
            let spec = self.inner.schema.node_spec(node_type);
            match get_field(spec) {
                Some(items) if !items.is_empty() => {
                    for &item in items {
                        allowed.insert(item);
                    }
                }
                Some(_) => {}
                None => break,
            }
        }

        allowed
    }

    /// node_id 위치에서 target_type이 조상의 forbidden_descendants에 의해 금지되는지 확인
    pub fn is_type_forbidden_at(&self, node_id: NodeId, target_type: NodeType) -> bool {
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node_type) = self.get_node_type(id) {
                let spec = self.inner.schema.node_spec(node_type);
                if let Some(forbidden) = spec.forbidden_descendants {
                    if forbidden.contains(&target_type) {
                        return true;
                    }
                }
            }
            current = self.get_parent_id(id);
        }
        false
    }

    pub fn is_ancestor(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut current = self.get_parent_id(node);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.get_parent_id(parent);
        }
        false
    }

    pub fn get_parent_id(&self, node_id: NodeId) -> Option<NodeId> {
        let map = self.inner.get_node_map(node_id)?;
        map.get("parent")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())
            .and_then(|v| NodeId::from_string(&v))
    }

    pub fn get_node_type(&self, node_id: NodeId) -> Option<NodeType> {
        let map = self.inner.get_node_map(node_id)?;
        let type_str = map
            .get("type")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())?;

        serde_json::from_value(serde_json::Value::String(type_str.to_string())).ok()
    }

    pub fn get_children_ids(&self, node_id: NodeId) -> Rc<Vec<NodeId>> {
        self.inner.get_children_ids_cached(node_id)
    }

    pub fn invalidate_children_cache_for(&self, node_id: NodeId) {
        self.inner.invalidate_children_cache_for(node_id);
    }

    pub fn clear_children_cache(&self) {
        self.inner.clear_children_cache();
    }

    pub fn get_children_list(&self, node_id: NodeId) -> Option<loro::LoroList> {
        self.inner.get_children_list(node_id)
    }

    pub fn mark_unreachable_subtree(&self, node_id: NodeId) {
        self.inner.mark_unreachable_subtree(node_id);
    }

    pub fn is_empty(&self) -> bool {
        let root_children = self.get_children_ids(NodeId::ROOT);
        if root_children.len() != 1 {
            return false;
        }

        let first_child_id = root_children[0];

        if self.get_node_type(first_child_id) != Some(NodeType::Paragraph) {
            return false;
        }

        self.get_children_ids(first_child_id).is_empty()
    }

    pub fn get_block_text(&self, block_id: NodeId) -> String {
        let mut result = String::new();

        for &child_id in self.get_children_ids(block_id).iter() {
            match self.get_node_type(child_id) {
                Some(NodeType::Text) => {
                    if let Some(segments) = self.get_text_segments(child_id) {
                        for seg in segments {
                            result.push_str(&seg.text);
                        }
                    }
                }
                Some(NodeType::HardBreak) => {
                    result.push('\n');
                }
                _ => {}
            }
        }

        result
    }

    pub(crate) fn get_text_segments(&self, node_id: NodeId) -> Option<Vec<TextSegment>> {
        let node_map = self.inner.get_node_map(node_id)?;
        let text = Text::decode_field(&node_map, "text").ok()?;
        Some(text.get_segments())
    }

    pub fn get_link_ranges(&self) -> Vec<LinkRange> {
        let mut ranges: Vec<LinkRange> = Vec::new();

        for (block_id, offset, seg) in self.iter_segments() {
            let segment_len = seg.text.chars().count();

            for ann in &seg.annotations {
                if let Annotation::Link(link) = ann {
                    if let Some(last) = ranges.last_mut() {
                        if last.block_id == block_id
                            && last.href == link.href
                            && last.end_offset == offset
                        {
                            last.end_offset = offset + segment_len;
                            continue;
                        }
                    }

                    ranges.push(LinkRange {
                        block_id,
                        start_offset: offset,
                        end_offset: offset + segment_len,
                        href: link.href.clone(),
                    });
                }
            }
        }

        ranges
    }

    pub fn iter_blocks(&self) -> BlockTextIterator<'_> {
        BlockTextIterator::new(self)
    }

    pub fn iter_segments(&self) -> TextSegmentIterator<'_> {
        TextSegmentIterator::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct LinkRange {
    pub block_id: NodeId,
    pub start_offset: usize,
    pub end_offset: usize,
    pub href: String,
}

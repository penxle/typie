use crate::model::tree::{BlockTextIterator, DocInner, NodeRef, TextSegmentIterator};
use crate::model::*;
use crate::schema::{Expand, Schema};
use anyhow::{Context, Result};
use loro::{ExpandType, ExportMode, Frontiers, LoroDoc, LoroMap, StyleConfig, StyleConfigMap};
use rustc_hash::FxHashSet;
use std::rc::Rc;

const SETTINGS_KEY: &str = "settings";

#[derive(Debug)]
pub struct Doc {
    inner: DocInner,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellcheckTextMapping {
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
    #[allow(dead_code)]
    pub fn new() -> Self {
        let schema = Rc::new(Schema::default());

        let loro = LoroDoc::new();
        loro.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::After,
        }));

        let mut styles = StyleConfigMap::new();
        for (mark_type, mark_spec) in schema.marks() {
            let expand = match mark_spec.expand {
                Expand::Before => ExpandType::Before,
                Expand::After => ExpandType::After,
                Expand::Both => ExpandType::Both,
                Expand::None => ExpandType::None,
            };
            styles.insert(mark_type.key().into(), StyleConfig { expand });
        }
        loro.config_text_style(styles);

        let nodes = loro.get_map("nodes");

        let map = nodes
            .insert_container(&NodeId::ROOT.to_string(), LoroMap::new())
            .unwrap();
        let mut root = Node::Root(RootNode::default());
        root.encode(&map).unwrap();

        let map = loro.get_map(SETTINGS_KEY);
        let mut settings = DocumentSettings::new();
        settings.encode(&map).unwrap();

        let inner = DocInner::new(loro, schema);

        Self { inner }
    }

    pub fn from_snapshot(snapshot: Vec<u8>) -> Self {
        let schema = Rc::new(Schema::default());

        let loro = LoroDoc::from_snapshot(&snapshot).unwrap();
        loro.config_default_text_style(Some(StyleConfig {
            expand: ExpandType::After,
        }));

        let mut styles = StyleConfigMap::new();
        for (mark_type, mark_spec) in schema.marks() {
            let expand = match mark_spec.expand {
                Expand::Before => ExpandType::Before,
                Expand::After => ExpandType::After,
                Expand::Both => ExpandType::Both,
                Expand::None => ExpandType::None,
            };
            styles.insert(mark_type.key().into(), StyleConfig { expand });
        }
        loro.config_text_style(styles);

        let inner = DocInner::new(loro, schema);

        Self { inner }
    }

    pub fn fork(&self) -> Self {
        Self {
            inner: self.inner.fork(),
        }
    }

    pub fn loro_doc(&self) -> &LoroDoc {
        &self.inner.loro
    }

    pub fn frontiers(&self) -> Frontiers {
        self.inner.loro.oplog_frontiers()
    }

    pub fn snapshot(&self) -> Result<Vec<u8>> {
        self.inner
            .loro
            .export(ExportMode::snapshot())
            .context("Failed to export document snapshot")
    }

    pub fn export_all_updates(&self) -> Result<Vec<u8>> {
        self.inner
            .loro
            .export(ExportMode::all_updates())
            .context("Failed to export all updates")
    }

    pub fn export_updates_from(&self, version: &loro::VersionVector) -> Result<Vec<u8>> {
        self.inner
            .loro
            .export(ExportMode::updates(version))
            .context("Failed to export updates from version")
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

    pub fn to_spellcheck_text(&self) -> (String, Vec<SpellcheckTextMapping>) {
        let mut full_text = String::new();
        let mut mappings = Vec::new();
        let mut char_offset = 0usize;

        for (block_id, _) in self.iter_blocks() {
            let mut block_offset = 0;

            for &child_id in self.get_children_ids(block_id).iter() {
                match self.get_node_type(child_id) {
                    Some(NodeType::Text) => {
                        if let Some(segments) = self.get_text_segments(child_id) {
                            for (segment_text, _) in segments {
                                let text_start = char_offset;
                                let char_len = segment_text.chars().count();
                                full_text.push_str(&segment_text);
                                char_offset += char_len;

                                mappings.push(SpellcheckTextMapping {
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

                        mappings.push(SpellcheckTextMapping {
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

                mappings.push(SpellcheckTextMapping {
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
        DocumentSettings::decode(&map).unwrap()
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut DocumentSettings)) -> Result<()> {
        let map = self.inner.loro.get_map(SETTINGS_KEY);
        let mut settings = DocumentSettings::decode(&map)?;
        f(&mut settings);
        settings.encode(&map)?;
        self.inner.loro.commit();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        self.validate_schema()?;
        Ok(())
    }

    pub fn validate_node(&self, node_id: NodeId) -> Result<()> {
        let node_ref = self.node(node_id).context("Node not found")?;
        let node_type = node_ref.node_type();
        let spec = self.inner.schema.node_spec(node_type);

        let child_types: Vec<NodeType> = node_ref
            .children()
            .map(|child| child.node().as_type())
            .collect();

        spec.content.validate(&child_types).with_context(|| {
            format!(
                "Content validation failed for '{:?}' at node {}",
                node_type, node_id
            )
        })?;

        if let Node::Text(text_node) = node_ref.node() {
            if let Some(parent) = node_ref.parent() {
                let parent_spec = self.inner.schema.node_spec(parent.node().as_type());
                let allowed_marks = parent_spec.marks.unwrap_or(&[]);

                let segments = text_node.text.get_rich_text_segments();
                for (_, marks) in segments {
                    for mark in marks {
                        let mark_type = mark.as_type();
                        if !allowed_marks.contains(&mark_type) {
                            anyhow::bail!(
                                "Mark '{:?}' not allowed in node {} (parent type: {:?})",
                                mark_type,
                                node_id,
                                parent.node().as_type()
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn validate_schema(&self) -> Result<()> {
        self.validate_schema_subtree(NodeId::ROOT, &FxHashSet::default())
    }

    #[allow(dead_code)]
    fn validate_schema_subtree(
        &self,
        node_id: NodeId,
        allowed_marks: &FxHashSet<MarkType>,
    ) -> Result<()> {
        let node_ref = self
            .node(node_id)
            .with_context(|| format!("Node not found during schema validation: {:?}", node_id))?;

        let node_type = node_ref.node_type();
        let spec = self.inner.schema.node_spec(node_type);

        let child_types: Vec<NodeType> = node_ref
            .children()
            .map(|child| child.node().as_type())
            .collect();

        spec.content.validate(&child_types).with_context(|| {
            format!(
                "Content validation failed for '{:?}' at node {}",
                node_type, node_id
            )
        })?;

        if let Node::Text(text_node) = node_ref.node() {
            let segments = text_node.text.get_rich_text_segments();
            for (_, marks) in segments {
                for mark in marks {
                    let mark_type = mark.as_type();
                    if !allowed_marks.contains(&mark_type) {
                        anyhow::bail!(
                            "Mark '{:?}' is not allowed at Text node {}. Allowed marks: {:?}",
                            mark_type,
                            node_id,
                            allowed_marks
                        );
                    }
                }
            }
        }

        let next_allowed_marks = match spec.marks {
            None => FxHashSet::default(),
            Some(marks) => {
                if marks.is_empty() {
                    allowed_marks.clone()
                } else {
                    let mut new_marks = allowed_marks.clone();
                    for &mark in marks {
                        new_marks.insert(mark);
                    }
                    new_marks
                }
            }
        };

        for child in node_ref.children() {
            self.validate_schema_subtree(child.node_id(), &next_allowed_marks)?;
        }

        Ok(())
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

    pub fn delete_nodes_batch(&self, node_ids: &[NodeId]) -> anyhow::Result<()> {
        let nodes = self.inner.loro.get_map("nodes");
        for node_id in node_ids {
            nodes.delete(&node_id.to_string())?;
        }
        Ok(())
    }

    pub fn find_orphan_nodes(&self) -> Vec<NodeId> {
        let nodes = self.inner.loro.get_map("nodes");
        let all_keys: Vec<String> = nodes.keys().map(|s| s.to_string()).collect();

        let mut reachable: FxHashSet<String> = FxHashSet::default();
        let mut stack = vec![NodeId::ROOT];
        while let Some(id) = stack.pop() {
            let key = id.to_string();
            if reachable.insert(key) {
                let children = self.get_children_ids(id);
                stack.extend(children.iter().copied());
            }
        }

        all_keys
            .into_iter()
            .filter(|key| !reachable.contains(key))
            .filter_map(|key| NodeId::from_string(&key))
            .collect()
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
                        for (segment_text, _) in segments {
                            result.push_str(&segment_text);
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

    pub(crate) fn get_text_segments(&self, node_id: NodeId) -> Option<Vec<(String, Vec<Mark>)>> {
        let node_map = self.inner.get_node_map(node_id)?;
        let text = Text::decode_field(&node_map, "text").ok()?;
        Some(text.get_rich_text_segments())
    }

    pub fn get_link_ranges(&self) -> Vec<LinkRange> {
        let mut ranges: Vec<LinkRange> = Vec::new();

        for (block_id, offset, text, marks) in self.iter_segments() {
            let segment_len = text.chars().count();

            for mark in &marks {
                if let Mark::Link(link) = mark {
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

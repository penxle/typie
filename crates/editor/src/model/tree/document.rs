use crate::model::tree::{DocInner, NodeRef};
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

    pub fn schema(&self) -> &Schema {
        &self.inner.schema
    }

    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        NodeRef::new(&self.inner, id)
    }

    pub fn to_plain_text(&self) -> String {
        use crate::state::BlockTraverser;

        let mut result = String::new();
        let Ok(mut traverser) = BlockTraverser::new(self, NodeId::ROOT) else {
            return result;
        };

        let mut is_first_block = true;

        while let Some(block_id) = traverser.next() {
            let Some(block) = self.node(block_id) else {
                continue;
            };

            if !is_first_block {
                result.push('\n');
            }
            is_first_block = false;

            for child in block.children() {
                match child.node() {
                    Node::Text(text_node) => {
                        result.push_str(&text_node.text.as_str());
                    }
                    Node::HardBreak(_) => {
                        result.push('\n');
                    }
                    _ => {}
                }
            }
        }

        result
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

    pub fn get_children_ids(&self, node_id: NodeId) -> Vec<NodeId> {
        if let Some(children) = self.inner.get_children_list(node_id) {
            let mut ids = Vec::with_capacity(children.len());
            for i in 0..children.len() {
                if let Some(child) = children.get(i) {
                    if let Ok(value) = child.into_value() {
                        if let Ok(s) = value.into_string() {
                            if let Some(id) = NodeId::from_string(&s) {
                                ids.push(id);
                            }
                        }
                    }
                }
            }
            ids
        } else {
            Vec::new()
        }
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
                stack.extend(children);
            }
        }

        all_keys
            .into_iter()
            .filter(|key| !reachable.contains(key))
            .filter_map(|key| NodeId::from_string(&key))
            .collect()
    }
}

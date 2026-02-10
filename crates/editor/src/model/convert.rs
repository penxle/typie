use crate::model::{Codec, Node, NodeId};
use crate::schema::{Expand, Schema};
use anyhow::{Context, Result};
use loro::{
    ExpandType, ExportMode, LoroDoc, LoroList, LoroMap, LoroValue, StyleConfig, StyleConfigMap,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct DocumentJson {
    pub settings: serde_json::Value,
    pub nodes: HashMap<String, NodeEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeEntry {
    #[serde(flatten)]
    pub node: Node,
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

fn configure_text_styles(loro: &LoroDoc) {
    let schema = Schema::default();

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
}

fn loro_value_to_json(value: &LoroValue) -> serde_json::Value {
    match value {
        LoroValue::Null => serde_json::Value::Null,
        LoroValue::Bool(b) => serde_json::Value::Bool(*b),
        LoroValue::I64(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        LoroValue::Double(d) => serde_json::Number::from_f64(*d)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        LoroValue::String(s) => serde_json::Value::String(s.to_string()),
        LoroValue::List(list) => {
            serde_json::Value::Array(list.iter().map(|v| loro_value_to_json(v)).collect())
        }
        LoroValue::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.to_string(), loro_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        LoroValue::Binary(b) => serde_json::Value::Array(
            b.iter()
                .map(|byte| serde_json::Value::Number(serde_json::Number::from(*byte)))
                .collect(),
        ),
        LoroValue::Container(_) => serde_json::Value::Null,
    }
}

fn apply_json_to_loro_map(map: &LoroMap, value: &serde_json::Value) -> Result<()> {
    let obj = value.as_object().context("settings must be an object")?;
    for (key, val) in obj {
        match val {
            serde_json::Value::Null => {}
            serde_json::Value::Bool(b) => {
                map.insert(key, *b)?;
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    map.insert(key, i)?;
                } else if let Some(f) = n.as_f64() {
                    map.insert(key, f)?;
                }
            }
            serde_json::Value::String(s) => {
                map.insert(key, s.as_str())?;
            }
            serde_json::Value::Object(_) => {
                let sub_map = map.insert_container(key, LoroMap::new())?;
                apply_json_to_loro_map(&sub_map, val)?;
            }
            serde_json::Value::Array(arr) => {
                let list = map.insert_container(key, LoroList::new())?;
                for item in arr {
                    apply_json_value_to_list(&list, item)?;
                }
            }
        }
    }
    Ok(())
}

fn apply_json_value_to_list(list: &LoroList, value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Null => {
            list.push(LoroValue::Null)?;
        }
        serde_json::Value::Bool(b) => {
            list.push(*b)?;
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                list.push(i)?;
            } else if let Some(f) = n.as_f64() {
                list.push(f)?;
            }
        }
        serde_json::Value::String(s) => {
            list.push(s.as_str())?;
        }
        serde_json::Value::Object(_) => {
            let sub_map = list.push_container(LoroMap::new())?;
            apply_json_to_loro_map(&sub_map, value)?;
        }
        serde_json::Value::Array(arr) => {
            let sub_list = list.push_container(LoroList::new())?;
            for item in arr {
                apply_json_value_to_list(&sub_list, item)?;
            }
        }
    }
    Ok(())
}

pub fn snapshot_to_json(snapshot: &[u8]) -> Result<DocumentJson> {
    let loro = LoroDoc::from_snapshot(snapshot).context("failed to parse snapshot")?;
    configure_text_styles(&loro);

    let settings_map = loro.get_map("settings");
    let settings = loro_value_to_json(&settings_map.get_deep_value());

    let nodes_map = loro.get_map("nodes");
    let mut nodes = HashMap::new();

    let mut reachable: HashMap<String, Vec<String>> = HashMap::new();
    let root_id = NodeId::ROOT.to_string();

    let mut stack = vec![root_id.clone()];
    while let Some(id_str) = stack.pop() {
        if reachable.contains_key(&id_str) {
            continue;
        }

        let Some(node_map) = nodes_map
            .get(&id_str)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
        else {
            continue;
        };

        let children_ids: Vec<String> = node_map
            .get("children")
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_list().ok())
            .map(|list| {
                let mut ids = Vec::new();
                if let LoroValue::List(values) = list.get_value() {
                    for v in values.iter() {
                        if let LoroValue::String(s) = v {
                            ids.push(s.to_string());
                        }
                    }
                }
                ids
            })
            .unwrap_or_default();

        for child_id in &children_ids {
            stack.push(child_id.clone());
        }

        reachable.insert(id_str, children_ids);
    }

    for (id_str, children_ids) in &reachable {
        let node_map = nodes_map
            .get(id_str)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .context("node map not found")?;

        let node =
            Node::decode(&node_map).with_context(|| format!("failed to decode node {}", id_str))?;

        let parent = node_map
            .get("parent")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string());

        nodes.insert(
            id_str.clone(),
            NodeEntry {
                node,
                children: children_ids.clone(),
                parent,
            },
        );
    }

    Ok(DocumentJson { settings, nodes })
}

pub fn json_to_snapshot(doc_json: &DocumentJson) -> Result<Vec<u8>> {
    let loro = LoroDoc::new();
    configure_text_styles(&loro);

    let nodes_map = loro.get_map("nodes");

    for (id_str, entry) in &doc_json.nodes {
        let node_map = nodes_map
            .insert_container(id_str, LoroMap::new())
            .context("failed to create node map")?;

        let mut node = entry.node.clone();
        node.encode(&node_map)?;

        if let Some(parent) = &entry.parent {
            node_map.insert("parent", parent.as_str())?;
        }

        let children_list = node_map.insert_container("children", LoroList::new())?;
        for child_id in &entry.children {
            children_list.push(child_id.as_str())?;
        }
    }

    let settings_map = loro.get_map("settings");
    apply_json_to_loro_map(&settings_map, &doc_json.settings)?;

    loro.commit();

    loro.export(ExportMode::snapshot())
        .context("failed to export snapshot")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tree::Doc;

    #[test]
    fn test_roundtrip_empty_doc() {
        let doc = Doc::new();
        let snapshot = doc.export(crate::model::DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        assert!(json.nodes.contains_key(&NodeId::ROOT.to_string()));

        let new_snapshot = json_to_snapshot(&json).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot);

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }

    #[test]
    fn test_roundtrip_doc_with_content() {
        use crate::model::{Node, ParagraphNode, TextNode};

        let doc = Doc::new();
        let root = doc.node(NodeId::ROOT).unwrap();
        let para_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        let para = doc.node(para_id).unwrap();
        para.as_mut()
            .insert_child(
                0,
                Node::Text(TextNode {
                    text: crate::model::Text::from("hello world"),
                }),
            )
            .unwrap();

        let snapshot = doc.export(crate::model::DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        let new_snapshot = json_to_snapshot(&json).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot);

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }

    #[test]
    fn test_roundtrip_doc_with_marks() {
        use crate::model::{Mark, Node, ParagraphNode, Text, TextNode, marks::ItalicMark};

        let doc = Doc::new();
        let root = doc.node(NodeId::ROOT).unwrap();
        let para_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        let text = Text::from_segments(&[
            ("normal ".to_string(), vec![]),
            (
                "bold".to_string(),
                vec![Mark::FontWeight(crate::model::marks::FontWeightMark {
                    weight: 700,
                })],
            ),
            (" ".to_string(), vec![]),
            ("italic".to_string(), vec![Mark::Italic(ItalicMark)]),
        ]);

        let para = doc.node(para_id).unwrap();
        para.as_mut()
            .insert_child(0, Node::Text(TextNode { text }))
            .unwrap();

        let snapshot = doc.export(crate::model::DocExportMode::Snapshot).unwrap();
        let json = snapshot_to_json(&snapshot).unwrap();

        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: DocumentJson = serde_json::from_str(&serialized).unwrap();

        let new_snapshot = json_to_snapshot(&deserialized).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot);

        let orig_root = doc.node(NodeId::ROOT).unwrap();
        let orig_para = orig_root.child(0).unwrap();
        let orig_text = orig_para.child(0).unwrap();
        let original_segments = match orig_text.node() {
            Node::Text(t) => t.text.get_rich_text_segments(),
            _ => panic!("expected text node"),
        };

        let new_root = new_doc.node(NodeId::ROOT).unwrap();
        let new_para = new_root.child(0).unwrap();
        let new_text = new_para.child(0).unwrap();
        let new_segments = match new_text.node() {
            Node::Text(t) => t.text.get_rich_text_segments(),
            _ => panic!("expected text node"),
        };

        assert_eq!(
            original_segments.len(),
            new_segments.len(),
            "segment count mismatch"
        );
        for (i, ((orig_text, orig_marks), (new_text, new_marks))) in original_segments
            .iter()
            .zip(new_segments.iter())
            .enumerate()
        {
            assert_eq!(orig_text, new_text, "text mismatch at segment {}", i);
            assert_eq!(
                orig_marks.len(),
                new_marks.len(),
                "marks count mismatch at segment {}",
                i
            );
            for (orig_mark, new_mark) in orig_marks.iter().zip(new_marks.iter()) {
                assert_eq!(orig_mark, new_mark, "mark mismatch at segment {}", i);
            }
        }
    }

    #[test]
    fn test_json_serialization() {
        let doc = Doc::new();
        let snapshot = doc.export(crate::model::DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: DocumentJson = serde_json::from_str(&serialized).unwrap();

        let new_snapshot = json_to_snapshot(&deserialized).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot);

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }
}

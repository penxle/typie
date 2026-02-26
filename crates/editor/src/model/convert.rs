use crate::model::tree::{CASCADE_ATTRS_KEY, REMARKS_KEY};
use crate::model::{Codec, Node, NodeId};
use anyhow::{Context, Result};
use loro::{ExpandType, ExportMode, LoroDoc, LoroList, LoroMap, LoroValue, StyleConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct DocumentJson {
    pub settings: serde_json::Map<String, serde_json::Value>,
    pub nodes: HashMap<String, NodeEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeEntry {
    #[serde(flatten)]
    pub node: Node,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cascade_attrs: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remarks: Option<serde_json::Map<String, serde_json::Value>>,
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
    loro.config_default_text_style(Some(StyleConfig {
        expand: ExpandType::None,
    }));

    let settings_map = loro.get_map("settings");
    let settings_json = loro_value_to_json(&settings_map.get_deep_value());
    let settings = match settings_json {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };

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

        let node = Node::decode(&node_map).with_context(|| {
            let deep_value = node_map.get_deep_value();
            format!("failed to decode node {}: {:?}", id_str, deep_value)
        })?;

        let parent = node_map
            .get("parent")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string());

        let cascade_attrs = node_map
            .get(CASCADE_ATTRS_KEY)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .map(|m| {
                let json = loro_value_to_json(&m.get_deep_value());
                match json {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                }
            })
            .filter(|m| !m.is_empty());

        let remarks = node_map
            .get(REMARKS_KEY)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .map(|m| {
                let json = loro_value_to_json(&m.get_deep_value());
                match json {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                }
            })
            .filter(|m| !m.is_empty());

        nodes.insert(
            id_str.clone(),
            NodeEntry {
                node,
                children: children_ids.clone(),
                parent,
                cascade_attrs,
                remarks,
            },
        );
    }

    Ok(DocumentJson { settings, nodes })
}

pub fn json_to_snapshot(doc_json: &DocumentJson) -> Result<Vec<u8>> {
    let loro = LoroDoc::new();
    loro.config_default_text_style(Some(StyleConfig {
        expand: ExpandType::None,
    }));

    let settings_map = loro.get_map("settings");
    apply_json_to_loro_map(
        &settings_map,
        &serde_json::Value::Object(doc_json.settings.clone()),
    )?;

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

        if let Some(cascade) = &entry.cascade_attrs {
            let cascade_map = node_map.insert_container(CASCADE_ATTRS_KEY, LoroMap::new())?;
            apply_json_to_loro_map(&cascade_map, &serde_json::Value::Object(cascade.clone()))?;
        }

        if let Some(remarks) = &entry.remarks {
            let remarks_map = node_map.insert_container(REMARKS_KEY, LoroMap::new())?;
            for (remark_id, remark_value) in remarks {
                if let serde_json::Value::Object(fields) = remark_value {
                    let entry_map = remarks_map.insert_container(remark_id, LoroMap::new())?;
                    apply_json_to_loro_map(&entry_map, &serde_json::Value::Object(fields.clone()))?;
                }
            }
        }

        let children_list = node_map.insert_container("children", LoroList::new())?;
        for child_id in &entry.children {
            children_list.push(child_id.as_str())?;
        }
    }

    loro.commit();

    loro.export(ExportMode::snapshot())
        .context("failed to export snapshot")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Doc, DocExportMode, FontWeightStyle, ItalicStyle, Node, ParagraphNode, Remark, Style, Text,
        TextNode, TextSegment,
    };

    #[test]
    fn test_roundtrip_empty_doc() {
        let doc = Doc::new();
        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        assert!(json.nodes.contains_key(&NodeId::ROOT.to_string()));

        let new_snapshot = json_to_snapshot(&json).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }

    #[test]
    fn test_roundtrip_doc_with_content() {
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
                    text: Text::from("hello world"),
                }),
            )
            .unwrap();

        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        let new_snapshot = json_to_snapshot(&json).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }

    #[test]
    fn test_roundtrip_doc_with_styles() {
        let doc = Doc::new();
        let root = doc.node(NodeId::ROOT).unwrap();
        let para_id = root
            .as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        let text = Text::from_segments(&[
            TextSegment {
                text: "normal ".to_string(),
                styles: vec![],
                annotations: vec![],
            },
            TextSegment {
                text: "bold".to_string(),
                styles: vec![Style::FontWeight(FontWeightStyle { weight: 700 })],
                annotations: vec![],
            },
            TextSegment {
                text: " ".to_string(),
                styles: vec![],
                annotations: vec![],
            },
            TextSegment {
                text: "italic".to_string(),
                styles: vec![Style::Italic(ItalicStyle {})],
                annotations: vec![],
            },
        ]);

        let para = doc.node(para_id).unwrap();
        para.as_mut()
            .insert_child(0, Node::Text(TextNode { text }))
            .unwrap();

        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();
        let json = snapshot_to_json(&snapshot).unwrap();

        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: DocumentJson = serde_json::from_str(&serialized).unwrap();

        let new_snapshot = json_to_snapshot(&deserialized).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        let orig_root = doc.node(NodeId::ROOT).unwrap();
        let orig_para = orig_root.child(0).unwrap();
        let orig_text = orig_para.child(0).unwrap();
        let original_segments = match orig_text.node().unwrap() {
            Node::Text(t) => t.text.get_segments(),
            _ => panic!("expected text node"),
        };

        let new_root = new_doc.node(NodeId::ROOT).unwrap();
        let new_para = new_root.child(0).unwrap();
        let new_text = new_para.child(0).unwrap();
        let new_segments = match new_text.node().unwrap() {
            Node::Text(t) => t.text.get_segments(),
            _ => panic!("expected text node"),
        };

        assert_eq!(
            original_segments.len(),
            new_segments.len(),
            "segment count mismatch"
        );
        for (i, (orig_seg, new_seg)) in original_segments
            .iter()
            .zip(new_segments.iter())
            .enumerate()
        {
            assert_eq!(
                orig_seg.text, new_seg.text,
                "text mismatch at segment {}",
                i
            );
            assert_eq!(
                orig_seg.styles.len(),
                new_seg.styles.len(),
                "styles count mismatch at segment {}",
                i
            );
            for (orig_style, new_style) in orig_seg.styles.iter().zip(new_seg.styles.iter()) {
                assert_eq!(orig_style, new_style, "style mismatch at segment {}", i);
            }
        }
    }

    #[test]
    fn test_json_serialization() {
        let doc = Doc::new();
        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();

        let json = snapshot_to_json(&snapshot).unwrap();
        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: DocumentJson = serde_json::from_str(&serialized).unwrap();

        let new_snapshot = json_to_snapshot(&deserialized).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        assert_eq!(doc.to_plain_text(), new_doc.to_plain_text());
    }

    #[test]
    fn test_roundtrip_doc_with_remarks() {
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
                    text: Text::from("hello"),
                }),
            )
            .unwrap();

        let remark = Remark {
            id: NodeId::new(),
            user_id: "user1".to_string(),
            text: "a comment".to_string(),
            created_at: 1700000000000,
        };
        para.as_mut().add_remark(&remark).unwrap();

        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();
        let json = snapshot_to_json(&snapshot).unwrap();

        let para_entry = json.nodes.get(&para_id.to_string()).unwrap();
        assert!(para_entry.remarks.is_some());
        let remarks_map = para_entry.remarks.as_ref().unwrap();
        assert_eq!(remarks_map.len(), 1);

        let serialized = serde_json::to_string(&json).unwrap();
        let deserialized: DocumentJson = serde_json::from_str(&serialized).unwrap();
        let new_snapshot = json_to_snapshot(&deserialized).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        let new_para = new_doc.node(para_id).unwrap();
        let remarks = new_para.remarks();
        assert_eq!(remarks.len(), 1);
        assert_eq!(remarks[0].id, remark.id);
        assert_eq!(remarks[0].user_id, "user1");
        assert_eq!(remarks[0].text, "a comment");
        assert_eq!(remarks[0].created_at, 1700000000000);
    }

    #[test]
    fn test_validate_doc_with_trailing_paragraph() {
        let doc = Doc::new();
        let root = doc.node(NodeId::ROOT).unwrap();
        root.as_mut()
            .insert_child(0, Node::Paragraph(ParagraphNode::default()))
            .unwrap();

        doc.validate_exhaustive().unwrap();
    }

    #[test]
    fn test_validate_doc_with_content() {
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
                    text: Text::from("hello world"),
                }),
            )
            .unwrap();

        doc.validate_exhaustive().unwrap();
    }

    #[test]
    fn test_validate_roundtripped_doc() {
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
                    text: Text::from("test"),
                }),
            )
            .unwrap();

        let snapshot = doc.export(DocExportMode::Snapshot).unwrap();
        let json = snapshot_to_json(&snapshot).unwrap();
        let new_snapshot = json_to_snapshot(&json).unwrap();
        let new_doc = Doc::from_snapshot(new_snapshot).expect("test: snapshot should decode");

        new_doc.validate_exhaustive().unwrap();
    }
}

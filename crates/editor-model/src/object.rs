use std::collections::{HashMap, HashSet};

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::doc::Doc;
use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::Node;

#[derive(Debug, thiserror::Error)]
pub enum ReconstructError {
    #[error("missing object: hash={hash}, referenced by {referenced_by:?}")]
    MissingObject {
        hash: String,
        referenced_by: Option<NodeId>,
    },
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChildRef {
    pub node_id: NodeId,
    pub hash: String,
}

/// `ObjectContent::hash()` of this struct (via canonical JSON) is its Object hash —
/// any change to `Serialize` shape changes the hash and breaks CAS dedup.
#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectContent {
    pub node_id: NodeId,
    pub node: Node,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifiers: Vec<Modifier>,
    pub children: Vec<ChildRef>,
}

impl ObjectContent {
    pub fn hash(&self) -> String {
        let bytes = crate::canonical::canonical_serialize(self);
        format!("{:032x}", xxhash_rust::xxh3::xxh3_128(&bytes))
    }
}

/// `CommitContent::hash()` of this struct (via canonical JSON) is its commit hash —
/// any change to `Serialize` shape changes the hash and breaks CAS dedup.
#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommitContent {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub second_parent_hash: Option<String>,
    pub object_hash: String,
}

impl CommitContent {
    pub fn hash(&self) -> String {
        let bytes = crate::canonical::canonical_serialize(self);
        format!("{:032x}", xxhash_rust::xxh3::xxh3_128(&bytes))
    }
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommitObject {
    pub hash: String,
    pub content: ObjectContent,
}

impl Doc {
    /// Returned objects are in post-order: children appear before their parents.
    pub fn derive_objects_for_path(
        &self,
        affected_node_ids: &[NodeId],
    ) -> (String, Vec<CommitObject>) {
        if affected_node_ids.is_empty() {
            return (self.compute_subtree_hash(NodeId::ROOT), vec![]);
        }

        let mut path: HashSet<NodeId> = HashSet::new();
        for &id in affected_node_ids {
            let mut cur = Some(id);
            while let Some(n) = cur {
                if !path.insert(n) {
                    break;
                }
                cur = self.get_entry(n).and_then(|e| e.parent);
            }
        }

        let mut all_objects: Vec<CommitObject> = Vec::new();
        let root_hash = self.derive_post_order(NodeId::ROOT, &path, &mut all_objects);
        (root_hash, all_objects)
    }

    fn derive_post_order(
        &self,
        node_id: NodeId,
        path: &HashSet<NodeId>,
        all_objects: &mut Vec<CommitObject>,
    ) -> String {
        let entry = self.get_entry(node_id).expect("node must exist");
        let mut children_hashes: Vec<ChildRef> = Vec::new();
        for &child_id in entry.children.iter() {
            let child_hash = if path.contains(&child_id) {
                self.derive_post_order(child_id, path, all_objects)
            } else {
                self.compute_subtree_hash(child_id)
            };
            children_hashes.push(ChildRef {
                node_id: child_id,
                hash: child_hash,
            });
        }

        let content = ObjectContent {
            node_id,
            node: entry.node.clone(),
            parent: entry.parent,
            modifiers: entry.modifiers.clone(),
            children: children_hashes,
        };
        let hash = content.hash();
        all_objects.push(CommitObject {
            hash: hash.clone(),
            content,
        });
        hash
    }

    fn compute_subtree_hash(&self, node_id: NodeId) -> String {
        let entry = self.get_entry(node_id).expect("node must exist");
        let children_hashes: Vec<ChildRef> = entry
            .children
            .iter()
            .map(|&cid| ChildRef {
                node_id: cid,
                hash: self.compute_subtree_hash(cid),
            })
            .collect();
        let content = ObjectContent {
            node_id,
            node: entry.node.clone(),
            parent: entry.parent,
            modifiers: entry.modifiers.clone(),
            children: children_hashes,
        };
        content.hash()
    }

    pub fn derive_all_objects(&self) -> (String, Vec<CommitObject>) {
        let all_ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        self.derive_objects_for_path(&all_ids)
    }

    pub fn reconstruct_from_objects(
        root_hash: &str,
        objects: &[(String, ObjectContent)],
    ) -> Result<Doc, ReconstructError> {
        let by_hash: HashMap<&str, &ObjectContent> =
            objects.iter().map(|(h, c)| (h.as_str(), c)).collect();

        let root_content =
            by_hash
                .get(root_hash)
                .ok_or_else(|| ReconstructError::MissingObject {
                    hash: root_hash.into(),
                    referenced_by: None,
                })?;

        let mut nodes = imbl::HashMap::new();
        Self::collect_node(root_content, &by_hash, &mut nodes)?;

        Ok(Doc { nodes })
    }

    fn collect_node(
        content: &ObjectContent,
        by_hash: &HashMap<&str, &ObjectContent>,
        nodes: &mut imbl::HashMap<NodeId, NodeEntry>,
    ) -> Result<(), ReconstructError> {
        if nodes.contains_key(&content.node_id) {
            return Ok(());
        }

        let children_ids: imbl::Vector<NodeId> =
            content.children.iter().map(|c| c.node_id).collect();

        nodes.insert(
            content.node_id,
            NodeEntry {
                node: content.node.clone(),
                parent: content.parent,
                children: children_ids,
                modifiers: content.modifiers.clone(),
            },
        );

        for child in &content.children {
            let child_content = by_hash.get(child.hash.as_str()).ok_or_else(|| {
                ReconstructError::MissingObject {
                    hash: child.hash.clone(),
                    referenced_by: Some(content.node_id),
                }
            })?;
            Self::collect_node(child_content, by_hash, nodes)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn derive_objects_for_root_only() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                }
            }
        };
        let affected: Vec<NodeId> = vec![NodeId::ROOT];
        let (root_hash, objects) = doc.derive_objects_for_path(&affected);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].content.node_id, NodeId::ROOT);
        assert_eq!(root_hash, objects[0].hash);
    }

    #[test]
    fn derive_objects_for_text_change() {
        let (doc, p, t) = doc! {
            root {
                p: paragraph {
                    t: text("hello")
                }
            }
        };
        let affected = vec![t];
        let (root_hash, objects) = doc.derive_objects_for_path(&affected);
        let ids: Vec<NodeId> = objects.iter().map(|o| o.content.node_id).collect();
        assert!(ids.contains(&t));
        assert!(ids.contains(&p));
        assert!(ids.contains(&NodeId::ROOT));
        assert_eq!(objects.len(), 3);
        let root_obj = objects
            .iter()
            .find(|o| o.content.node_id == NodeId::ROOT)
            .unwrap();
        assert_eq!(root_hash, root_obj.hash);
    }

    #[test]
    fn derive_returns_no_duplicates() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph {
                    t1: text("a")
                    t2: text("b")
                }
            }
        };
        let affected = vec![t1, t2];
        let (_root_hash, objects) = doc.derive_objects_for_path(&affected);
        assert_eq!(objects.len(), 4);
        let mut ids: Vec<NodeId> = objects.iter().map(|o| o.content.node_id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn derive_with_empty_affected_returns_no_objects() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                }
            }
        };
        let (root_hash, objects) = doc.derive_objects_for_path(&[]);
        assert!(objects.is_empty());
        let (full_hash, _) = doc.derive_objects_for_path(&[NodeId::ROOT]);
        assert_eq!(root_hash, full_hash);
    }

    #[test]
    fn derive_all_visits_every_node() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                    text("world")
                }
            }
        };
        let (root_hash, objects) = doc.derive_all_objects();
        assert_eq!(objects.len(), doc.nodes.len());
        assert!(objects.iter().any(|o| o.hash == root_hash));
    }

    #[test]
    fn reconstruct_roundtrip() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                }
            }
        };
        let (root_hash, derived) = doc.derive_all_objects();
        let pairs: Vec<(String, ObjectContent)> =
            derived.into_iter().map(|d| (d.hash, d.content)).collect();
        let restored = Doc::reconstruct_from_objects(&root_hash, &pairs).unwrap();
        assert_eq!(restored.nodes.len(), doc.nodes.len());
        let restored_root = restored.get_entry(NodeId::ROOT).unwrap();
        let original_root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(restored_root.node, original_root.node);
    }

    #[test]
    fn reconstruct_missing_object_returns_error() {
        let result = Doc::reconstruct_from_objects("nonexistent_hash", &[]);
        assert!(matches!(
            result,
            Err(ReconstructError::MissingObject {
                ref hash,
                referenced_by: None,
            }) if hash == "nonexistent_hash"
        ));
    }

    #[test]
    fn reconstruct_missing_descendant_returns_error_with_parent_context() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                }
            }
        };
        let all_ids: Vec<NodeId> = doc.nodes.keys().copied().collect();
        let (root_hash, derived) = doc.derive_objects_for_path(&all_ids);
        let root_only: Vec<(String, ObjectContent)> = derived
            .into_iter()
            .filter(|d| d.content.node_id == NodeId::ROOT)
            .map(|d| (d.hash, d.content))
            .collect();
        let result = Doc::reconstruct_from_objects(&root_hash, &root_only);
        assert!(matches!(
            result,
            Err(ReconstructError::MissingObject {
                referenced_by: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn commit_hash_is_32_char_hex() {
        let c = CommitContent {
            parent_hash: None,
            second_parent_hash: None,
            object_hash: "deadbeef".to_string(),
        };
        let h = c.hash();
        assert_eq!(h.len(), 32);
        assert!(
            h.chars()
                .all(|ch| ch.is_ascii_digit() || ch.is_ascii_lowercase())
        );
    }

    #[test]
    fn root_commit_differs_from_child_with_same_object() {
        let root = CommitContent {
            parent_hash: None,
            second_parent_hash: None,
            object_hash: "obj1".to_string(),
        };
        let child = CommitContent {
            parent_hash: Some("p1".to_string()),
            second_parent_hash: None,
            object_hash: "obj1".to_string(),
        };
        assert_ne!(root.hash(), child.hash());
    }

    #[test]
    fn merge_commit_differs_from_linear_commit() {
        let linear = CommitContent {
            parent_hash: Some("p1".to_string()),
            second_parent_hash: None,
            object_hash: "obj1".to_string(),
        };
        let merge = CommitContent {
            parent_hash: Some("p1".to_string()),
            second_parent_hash: Some("p2".to_string()),
            object_hash: "obj1".to_string(),
        };
        assert_ne!(linear.hash(), merge.hash());
    }

    #[test]
    fn parent_order_matters() {
        let a_b = CommitContent {
            parent_hash: Some("a".to_string()),
            second_parent_hash: Some("b".to_string()),
            object_hash: "obj".to_string(),
        };
        let b_a = CommitContent {
            parent_hash: Some("b".to_string()),
            second_parent_hash: Some("a".to_string()),
            object_hash: "obj".to_string(),
        };
        assert_ne!(a_b.hash(), b_a.hash());
    }

    #[test]
    fn same_input_produces_same_hash() {
        let c1 = CommitContent {
            parent_hash: Some("p".to_string()),
            second_parent_hash: None,
            object_hash: "obj".to_string(),
        };
        let c2 = CommitContent {
            parent_hash: Some("p".to_string()),
            second_parent_hash: None,
            object_hash: "obj".to_string(),
        };
        assert_eq!(c1.hash(), c2.hash());
    }
}

#[cfg(test)]
mod proptest_laws {
    use super::*;
    use crate::nodes::{ParagraphNode, RootNode, TextNode};
    use proptest::prelude::*;

    fn arb_modifier() -> impl Strategy<Value = Modifier> {
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            Just(Modifier::Underline),
            Just(Modifier::Strikethrough),
        ]
    }

    fn arb_simple_doc() -> impl Strategy<Value = Doc> {
        let para_strat = (
            prop::collection::vec(arb_modifier(), 0..=2),
            prop::collection::vec(
                (prop::collection::vec(arb_modifier(), 0..=2), "[a-z]{1,5}"),
                1..=3,
            ),
        );
        prop::collection::vec(para_strat, 1..=3).prop_map(|paragraphs| {
            let mut nodes = imbl::HashMap::new();
            let mut root_children = imbl::Vector::new();

            for (para_mods, text_entries) in paragraphs {
                let para_id = NodeId::new();
                let mut para_children = imbl::Vector::new();
                for (text_mods, text) in text_entries {
                    let text_id = NodeId::new();
                    nodes.insert(
                        text_id,
                        NodeEntry::new(Node::Text(TextNode { text }))
                            .with_parent(para_id)
                            .with_modifiers(text_mods),
                    );
                    para_children.push_back(text_id);
                }
                nodes.insert(
                    para_id,
                    NodeEntry::new(Node::Paragraph(ParagraphNode::default()))
                        .with_parent(NodeId::ROOT)
                        .with_children(para_children)
                        .with_modifiers(para_mods),
                );
                root_children.push_back(para_id);
            }

            nodes.insert(
                NodeId::ROOT,
                NodeEntry::new(Node::Root(RootNode::default())).with_children(root_children),
            );

            Doc { nodes }
        })
    }

    proptest! {
        #[test]
        fn hash_determinism(doc in arb_simple_doc()) {
            let (h1, _) = doc.derive_all_objects();
            let (h2, _) = doc.derive_all_objects();
            prop_assert_eq!(h1.len(), 32);
            prop_assert_eq!(h1, h2);
        }

        #[test]
        fn walk_consistency(doc in arb_simple_doc()) {
            let (_, objects) = doc.derive_all_objects();
            prop_assert_eq!(objects.len(), doc.nodes.len());
            for node_id in doc.nodes.keys() {
                prop_assert!(objects.iter().any(|o| o.content.node_id == *node_id));
            }
        }

        #[test]
        fn reconstruct_roundtrip(doc in arb_simple_doc()) {
            let (root_hash, derived) = doc.derive_all_objects();
            let pairs: Vec<(String, ObjectContent)> =
                derived.into_iter().map(|d| (d.hash, d.content)).collect();
            let restored = Doc::reconstruct_from_objects(&root_hash, &pairs).unwrap();
            prop_assert_eq!(restored.nodes.len(), doc.nodes.len());
            for (id, original_entry) in doc.nodes.iter() {
                let restored_entry = restored.get_entry(*id).expect("missing node");
                prop_assert_eq!(&restored_entry.node, &original_entry.node);
                prop_assert_eq!(&restored_entry.parent, &original_entry.parent);
                prop_assert_eq!(&restored_entry.children, &original_entry.children);
                prop_assert_eq!(&restored_entry.modifiers, &original_entry.modifiers);
            }
        }

        #[test]
        fn commit_hash_determinism(
            parent in proptest::option::of("[a-f0-9]{32}"),
            second_parent in proptest::option::of("[a-f0-9]{32}"),
            object in "[a-f0-9]{32}",
        ) {
            let c = CommitContent {
                parent_hash: parent,
                second_parent_hash: second_parent,
                object_hash: object,
            };
            let h1 = c.hash();
            let h2 = c.hash();
            prop_assert_eq!(h1.len(), 32);
            prop_assert_eq!(h1, h2);
        }
    }
}

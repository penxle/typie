use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::node_ref::NodeRef;
use crate::nodes::{Node, ParagraphNode, RootNode};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub nodes: imbl::HashMap<NodeId, NodeEntry>,
}

impl Default for Doc {
    fn default() -> Self {
        Self {
            nodes: imbl::hashmap! {
                NodeId::ROOT => NodeEntry {
                    node: Node::Root(RootNode::default()),
                    parent: None,
                    children: imbl::Vector::new(),
                    modifiers: vec![],
                }
            },
        }
    }
}

impl Doc {
    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        if self.nodes.contains_key(&id) {
            Some(NodeRef::new(self, id))
        } else {
            None
        }
    }

    pub fn root(&self) -> NodeRef<'_> {
        NodeRef::new(self, NodeId::ROOT)
    }

    pub fn get_entry(&self, id: NodeId) -> Option<&NodeEntry> {
        self.nodes.get(&id)
    }

    pub fn with_node(&self, id: NodeId, entry: NodeEntry) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.update(id, entry);
        new
    }

    pub fn with_node_updated(&self, id: NodeId, f: impl FnOnce(NodeEntry) -> NodeEntry) -> Doc {
        let mut new = self.clone();
        if let Some(entry) = new.nodes.get(&id).cloned() {
            new.nodes = new.nodes.update(id, f(entry));
        }
        new
    }

    pub fn insert_node(&self, id: NodeId, entry: NodeEntry) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.update(id, entry);
        new
    }

    pub fn remove_node(&self, id: NodeId) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.without(&id);
        new
    }

    pub fn with_preset(root: RootNode, modifiers: Vec<Modifier>) -> Doc {
        let paragraph_id = NodeId::new();
        Self {
            nodes: imbl::hashmap! {
                NodeId::ROOT => NodeEntry {
                    node: Node::Root(root),
                    parent: None,
                    children: imbl::vector![paragraph_id],
                    modifiers,
                },
                paragraph_id => NodeEntry {
                    node: Node::Paragraph(ParagraphNode::default()),
                    parent: Some(NodeId::ROOT),
                    children: imbl::Vector::new(),
                    modifiers: vec![],
                },
            },
        }
    }

    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        self.extract_text_recursive(NodeId::ROOT, &mut out);
        out
    }

    fn extract_text_recursive(&self, node_id: NodeId, out: &mut String) {
        let Some(entry) = self.get_entry(node_id) else {
            return;
        };
        match &entry.node {
            Node::Text(t) => out.push_str(&t.text),
            Node::HardBreak(_)
            | Node::PageBreak(_)
            | Node::Image(_)
            | Node::File(_)
            | Node::Embed(_)
            | Node::Archived(_) => {}
            _ => {
                for &child in entry.children.iter() {
                    self.extract_text_recursive(child, out);
                }
                out.push('\n');
            }
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Doc {
    pub fn new_test() -> Self {
        use crate::default_modifiers;

        Self {
            nodes: imbl::hashmap! {
                NodeId::ROOT => NodeEntry {
                    node: Node::Root(RootNode::default()),
                    parent: None,
                    children: imbl::Vector::new(),
                    modifiers: default_modifiers(),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::*;

    fn make_doc() -> Doc {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("Hello")
                }
            }
        };
        doc
    }

    #[test]
    fn node_returns_some_for_existing() {
        let doc = make_doc();
        assert!(doc.node(NodeId::ROOT).is_some());
    }

    #[test]
    fn node_returns_none_for_missing() {
        let doc = make_doc();
        assert!(doc.node(NodeId::new()).is_none());
    }

    #[test]
    fn root_returns_root_node() {
        let doc = make_doc();
        let root = doc.root();
        assert!(matches!(root.node(), &Node::Root(_)));
    }

    #[test]
    fn clone_is_o1() {
        let doc = make_doc();
        let doc2 = doc.clone();
        assert!(doc.node(NodeId::ROOT).is_some());
        assert!(doc2.node(NodeId::ROOT).is_some());
    }

    #[test]
    fn with_node_returns_new_doc() {
        let doc = make_doc();
        let new_id = NodeId::new();
        let doc2 = doc.insert_node(new_id, NodeEntry::new(Node::HardBreak(HardBreakNode {})));
        assert!(doc.node(new_id).is_none());
        assert!(doc2.node(new_id).is_some());
    }

    #[test]
    fn with_node_updated() {
        let doc = make_doc();
        let root = doc.root();
        let p1 = root.entry().children[0];

        let doc2 = doc.with_node_updated(p1, |mut entry| {
            entry.modifiers.push(Modifier::Bold);
            entry
        });

        let updated = doc2.node(p1).unwrap();
        assert!(updated.modifiers().contains(&Modifier::Bold));

        let original = doc.node(p1).unwrap();
        assert!(!original.modifiers().contains(&Modifier::Bold));
    }

    #[test]
    fn remove_node() {
        let doc = make_doc();
        let new_id = NodeId::new();
        let doc2 = doc.insert_node(new_id, NodeEntry::new(Node::HardBreak(HardBreakNode {})));
        let doc3 = doc2.remove_node(new_id);
        assert!(doc3.node(new_id).is_none());
    }

    #[test]
    fn extract_text_concatenates_text_nodes() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                    text(" world")
                }
            }
        };
        let text = doc.extract_text();
        assert!(text.contains("hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn extract_text_exact_output() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello world")
                }
            }
        };
        let text = doc.extract_text();
        assert_eq!(text, "hello world\n\n");
    }

    #[test]
    fn extract_text_hard_break_does_not_add_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("first")
                    hard_break
                    text("second")
                }
            }
        };
        let text = doc.extract_text();
        assert_eq!(text, "firstsecond\n\n");
    }

    #[test]
    fn extract_text_preserves_block_separation() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("first")
                }
                paragraph {
                    text("second")
                }
            }
        };
        let text = doc.extract_text();
        assert!(text.contains("first"));
        assert!(text.contains("second"));
        let pos1 = text.find("first").unwrap();
        let pos2 = text.find("second").unwrap();
        assert!(pos2 > pos1);
        let between = &text[pos1 + 5..pos2];
        assert!(
            between.contains('\n'),
            "expected newline between blocks: {:?}",
            between
        );
    }

    #[test]
    fn root_default_has_continuous_layout_and_default_modifiers() {
        let doc = make_doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        match &root.node {
            Node::Root(r) => assert!(matches!(r.layout_mode, LayoutMode::Continuous { .. })),
            _ => panic!("expected Root"),
        }
        assert!(
            root.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { value } if value == "Pretendard"))
        );
    }

    #[test]
    fn with_preset_builds_root_with_paragraph() {
        let doc = Doc::with_preset(RootNode::default(), vec![]);
        let root = doc.get_entry(NodeId::ROOT).expect("root must exist");
        assert!(matches!(root.node, Node::Root(_)));
        assert!(root.parent.is_none());
        assert_eq!(root.children.len(), 1, "root must have exactly one child");

        let paragraph_id = root.children[0];
        let paragraph = doc.get_entry(paragraph_id).expect("paragraph must exist");
        assert!(matches!(paragraph.node, Node::Paragraph(_)));
        assert_eq!(paragraph.parent, Some(NodeId::ROOT));
        assert!(paragraph.children.is_empty());
        assert!(paragraph.modifiers.is_empty());
        assert_eq!(doc.nodes.len(), 2, "doc must have exactly root + paragraph");
    }

    #[test]
    fn with_preset_applies_modifiers_to_root() {
        let mods = vec![
            Modifier::FontFamily {
                value: "MyFont".into(),
            },
            Modifier::FontSize { value: 1800 },
            Modifier::LineHeight { value: 200 },
        ];
        let doc = Doc::with_preset(RootNode::default(), mods.clone());
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        assert_eq!(root.modifiers, mods);
    }

    #[test]
    fn with_preset_applies_layout_mode() {
        let root_node = RootNode {
            layout_mode: LayoutMode::Paginated {
                page_width: 794.0,
                page_height: 1123.0,
                page_margin_top: 50.0,
                page_margin_bottom: 50.0,
                page_margin_left: 50.0,
                page_margin_right: 50.0,
            },
        };
        let doc = Doc::with_preset(root_node.clone(), vec![]);
        let root_entry = doc.get_entry(NodeId::ROOT).unwrap();
        match &root_entry.node {
            Node::Root(r) => assert_eq!(r, &root_node),
            _ => panic!("expected Root"),
        }
    }

    #[test]
    fn with_preset_round_trip_via_objects() {
        let mods = vec![Modifier::FontSize { value: 1500 }];
        let original = Doc::with_preset(RootNode::default(), mods);
        let (root_hash, derived) = original.derive_all_objects();
        let pairs: Vec<(String, ObjectContent)> =
            derived.into_iter().map(|d| (d.hash, d.content)).collect();
        let reconstructed = Doc::reconstruct_from_objects(&root_hash, &pairs).unwrap();
        assert_doc_eq!(original, reconstructed);
    }
}

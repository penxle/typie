use std::collections::{BTreeMap, BTreeSet};

use editor_crdt::{Dot, LwwRegOp, OpGraph, OrMapOp, OrSetOp, RgaOp, TextOp, ToPlain};
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::{
    Doc, DocOp, Modifier, ModifierType, NodeId, PlainNode, PlainTextNode, StyleOp, apply_doc_op,
};

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlainDoc {
    pub nodes: BTreeMap<NodeId, PlainNodeEntry>,
    #[serde(default)]
    pub styles: BTreeMap<String, PlainStyleEntry>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PlainStyleEntry {
    pub name: String,
    pub modifiers: BTreeSet<Modifier>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlainNodeEntry {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub modifiers: BTreeMap<ModifierType, Modifier>,
    #[serde(default)]
    pub style: Option<String>,
    pub node: PlainNode,
}

impl Doc {
    pub fn to_plain(&self) -> PlainDoc {
        let nodes: BTreeMap<NodeId, PlainNodeEntry> = self
            .nodes_iter()
            .map(|(id, _kind)| {
                let entry = self.get_entry(*id).expect("nodes_iter consistency");
                let plain_entry = PlainNodeEntry {
                    parent: entry.parent.to_plain(),
                    children: entry.children.to_plain(),
                    modifiers: entry.modifiers.to_plain(),
                    style: entry.style.to_plain(),
                    node: entry.node.to_plain(),
                };
                (*id, plain_entry)
            })
            .collect();

        let styles: BTreeMap<String, PlainStyleEntry> = self
            .styles_iter()
            .map(|(id, _)| {
                let entry = self.style_entry(id).cloned().unwrap_or_default();
                (
                    id.clone(),
                    PlainStyleEntry {
                        name: entry.name.to_plain(),
                        modifiers: entry.modifiers.to_plain(),
                    },
                )
            })
            .collect();

        PlainDoc { nodes, styles }
    }

    pub fn from_plain(plain: PlainDoc) -> (Self, OpGraph<DocOp>) {
        let roots: Vec<NodeId> = plain
            .nodes
            .iter()
            .filter(|(_, entry)| matches!(entry.node, PlainNode::Root(_)))
            .map(|(id, _)| *id)
            .collect();
        assert!(
            roots.len() == 1,
            "PlainDoc must contain exactly one Root node"
        );
        let root_id = roots[0];

        let mut graph = OpGraph::new();
        let mut doc = Doc::empty();

        emit_node(&mut graph, &mut doc, &plain, root_id, None, None);

        let mut queue = vec![root_id];
        while let Some(id) = queue.first().copied() {
            queue.remove(0);
            let children: Vec<NodeId> = plain.nodes[&id].children.clone();
            for (i, child_id) in children.into_iter().enumerate() {
                emit_node(&mut graph, &mut doc, &plain, child_id, Some(id), Some(i));
                queue.push(child_id);
            }
        }

        emit_style_entries(&mut graph, &mut doc, &plain);

        let graph = graph.commit();
        (doc, graph)
    }
}

fn apply_and_record(graph: &mut OpGraph<DocOp>, doc: &mut Doc, payload: DocOp) -> Dot {
    let (g, op) = graph.add(payload).expect("local create");
    *graph = g;
    let new_doc = apply_doc_op(std::mem::take(doc), &op).expect("local apply");
    *doc = new_doc;
    op.id
}

fn emit_node(
    graph: &mut OpGraph<DocOp>,
    doc: &mut Doc,
    plain: &PlainDoc,
    id: NodeId,
    parent: Option<NodeId>,
    index: Option<usize>,
) {
    let entry = &plain.nodes[&id];

    apply_and_record(
        graph,
        doc,
        DocOp::Presence {
            node_id: id,
            op: OrMapOp::Set {
                key: id,
                value: entry.node.as_type(),
            },
        },
    );

    if let Some(parent_id) = parent {
        apply_and_record(
            graph,
            doc,
            DocOp::Parent {
                node_id: id,
                op: LwwRegOp::Set {
                    value: Some(parent_id),
                },
            },
        );

        let anchor = doc
            .get_entry(parent_id)
            .expect("parent entry must exist before child insert")
            .children
            .dot_at(index.expect("index required when parent is Some"))
            .expect("dot_at error");

        apply_and_record(
            graph,
            doc,
            DocOp::Children {
                node_id: parent_id,
                op: RgaOp::Insert {
                    after: anchor,
                    value: id,
                },
            },
        );
    }

    for attr in entry.node.to_attrs() {
        apply_and_record(
            graph,
            doc,
            DocOp::Attr {
                node_id: id,
                op: attr,
            },
        );
    }

    if let PlainNode::Text(PlainTextNode { text }) = &entry.node {
        let mut prev: Option<Dot> = None;
        for ch in text.chars() {
            let dot = apply_and_record(
                graph,
                doc,
                DocOp::Text {
                    node_id: id,
                    op: TextOp::InsertChar { after: prev, ch },
                },
            );
            prev = Some(dot);
        }
    }

    for (k, v) in entry.modifiers.iter() {
        apply_and_record(
            graph,
            doc,
            DocOp::Modifier {
                node_id: id,
                op: OrMapOp::Set {
                    key: *k,
                    value: v.clone(),
                },
            },
        );
    }

    if entry.style.is_some() {
        apply_and_record(
            graph,
            doc,
            DocOp::NodeStyle {
                node_id: id,
                op: LwwRegOp::Set {
                    value: entry.style.clone(),
                },
            },
        );
    }
}

fn emit_style_entries(graph: &mut OpGraph<DocOp>, doc: &mut Doc, plain: &PlainDoc) {
    for (style_id, entry) in plain.styles.iter() {
        apply_and_record(
            graph,
            doc,
            DocOp::Style {
                style_id: style_id.clone(),
                op: StyleOp::Presence(OrMapOp::Set {
                    key: style_id.clone(),
                    value: (),
                }),
            },
        );
        apply_and_record(
            graph,
            doc,
            DocOp::Style {
                style_id: style_id.clone(),
                op: StyleOp::Name(LwwRegOp::Set {
                    value: entry.name.clone(),
                }),
            },
        );
        for m in entry.modifiers.iter() {
            apply_and_record(
                graph,
                doc,
                DocOp::Style {
                    style_id: style_id.clone(),
                    op: StyleOp::Modifiers(OrSetOp::Add { elem: m.clone() }),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlockquoteVariant, CalloutVariant, HorizontalRuleVariant, LayoutMode, PlainArchivedNode,
        PlainBlockquoteNode, PlainCalloutNode, PlainEmbedNode, PlainFileNode,
        PlainHorizontalRuleNode, PlainImageNode, PlainNode, PlainNodeEntry, PlainParagraphNode,
        PlainRootNode, PlainTabNode, PlainTableCellNode, PlainTableNode, PlainTableRowNode,
        TableBorderStyle,
    };

    #[test]
    fn empty_doc_to_plain_is_empty() {
        let doc = Doc::empty();
        let plain = doc.to_plain();
        assert!(plain.nodes.is_empty());
    }

    #[test]
    #[should_panic]
    fn empty_plain_panics() {
        let _ = Doc::from_plain(PlainDoc::default());
    }

    fn three_level_plain() -> PlainDoc {
        let root_id = NodeId::new();
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: vec![para_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![text_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text_id,
            PlainNodeEntry {
                parent: Some(para_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Text(PlainTextNode {
                    text: "hi".to_string(),
                }),
            },
        );

        PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        }
    }

    #[test]
    fn three_level_bootstrap_trace() {
        let plain = three_level_plain();
        let plain_clone = plain.clone();
        let (doc, graph) = Doc::from_plain(plain);

        let presence_count = graph
            .iter_all()
            .filter(|op| matches!(op.payload, DocOp::Presence { .. }))
            .count();
        assert_eq!(presence_count, 3);

        let parent_count = graph
            .iter_all()
            .filter(|op| matches!(op.payload, DocOp::Parent { .. }))
            .count();
        assert_eq!(parent_count, 2);

        let children_count = graph
            .iter_all()
            .filter(|op| matches!(op.payload, DocOp::Children { .. }))
            .count();
        assert_eq!(children_count, 2);

        assert!(doc.root().is_some());
        let round_tripped = doc.to_plain();
        assert_eq!(round_tripped, plain_clone);
    }

    fn all_attr_plain() -> PlainDoc {
        let root_id = NodeId::ROOT;

        let blockquote_id = NodeId::new();
        let callout_id = NodeId::new();
        let hr_id = NodeId::new();
        let table_id = NodeId::new();
        let table_row_id = NodeId::new();
        let table_cell_id = NodeId::new();
        let archived_id = NodeId::new();
        let embed_id = NodeId::new();
        let file_id = NodeId::new();
        let image_id = NodeId::new();

        let root_children = vec![
            blockquote_id,
            callout_id,
            hr_id,
            table_id,
            archived_id,
            embed_id,
            file_id,
            image_id,
        ];

        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: root_children,
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Root(PlainRootNode {
                    layout_mode: LayoutMode::Continuous { max_width: 1234 },
                }),
            },
        );
        nodes.insert(
            blockquote_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Blockquote(PlainBlockquoteNode {
                    variant: BlockquoteVariant::LeftQuote,
                }),
            },
        );
        nodes.insert(
            callout_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Callout(PlainCalloutNode {
                    variant: CalloutVariant::Warning,
                }),
            },
        );
        nodes.insert(
            hr_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::HorizontalRule(PlainHorizontalRuleNode {
                    variant: HorizontalRuleVariant::DashedLine,
                }),
            },
        );
        nodes.insert(
            table_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![table_row_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Table(PlainTableNode {
                    border_style: TableBorderStyle::Dashed,
                    proportion: 75,
                }),
            },
        );
        nodes.insert(
            table_row_id,
            PlainNodeEntry {
                parent: Some(table_id),
                children: vec![table_cell_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::TableRow(PlainTableRowNode {}),
            },
        );
        nodes.insert(
            table_cell_id,
            PlainNodeEntry {
                parent: Some(table_row_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::TableCell(PlainTableCellNode {
                    col_width: Some(120),
                    background_color: None,
                }),
            },
        );
        nodes.insert(
            archived_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Archived(PlainArchivedNode {
                    id: Some("arc-001".to_string()),
                }),
            },
        );
        nodes.insert(
            embed_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Embed(PlainEmbedNode {
                    id: Some("emb-001".to_string()),
                }),
            },
        );
        nodes.insert(
            file_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::File(PlainFileNode {
                    id: Some("file-001".to_string()),
                }),
            },
        );
        nodes.insert(
            image_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Image(PlainImageNode {
                    id: Some("img-001".to_string()),
                    proportion: 50,
                }),
            },
        );

        PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        }
    }

    #[test]
    fn per_attr_round_trip() {
        let plain = all_attr_plain();
        let (doc, _) = Doc::from_plain(plain.clone());
        let result = doc.to_plain();
        assert_eq!(result, plain);
    }

    #[test]
    fn modifier_round_trip() {
        let root_id = NodeId::new();
        let mut modifiers = BTreeMap::new();
        modifiers.insert(ModifierType::Bold, Modifier::Bold);
        modifiers.insert(ModifierType::Italic, Modifier::Italic);

        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: vec![],
                modifiers,
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );

        let plain = PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        };
        let (doc, _) = Doc::from_plain(plain.clone());
        let result = doc.to_plain();
        assert_eq!(result, plain);
    }

    #[test]
    fn tab_with_font_size_survives_plain_roundtrip() {
        let root_id = NodeId::ROOT;
        let para_id = NodeId::new();
        let tab_id = NodeId::new();

        let mut tab_modifiers = BTreeMap::new();
        tab_modifiers.insert(ModifierType::FontSize, Modifier::FontSize { value: 2400 });

        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: vec![para_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para_id,
            PlainNodeEntry {
                parent: Some(root_id),
                children: vec![tab_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            tab_id,
            PlainNodeEntry {
                parent: Some(para_id),
                children: vec![],
                modifiers: tab_modifiers,
                style: None,
                node: PlainNode::Tab(PlainTabNode {}),
            },
        );

        let plain = PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        };
        let (doc, _) = Doc::from_plain(plain.clone());
        let result = doc.to_plain();
        assert_eq!(result, plain);

        let has_tab_with_size = result.nodes.values().any(|e| {
            matches!(e.node, PlainNode::Tab(_)) && e.modifiers.contains_key(&ModifierType::FontSize)
        });
        assert!(
            has_tab_with_size,
            "Tab's font_size must survive plain roundtrip"
        );
    }

    #[test]
    fn from_plain_returns_committed_graph() {
        let root_id = NodeId::new();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root_id,
            PlainNodeEntry {
                parent: None,
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        let plain = PlainDoc {
            nodes,
            styles: BTreeMap::new(),
        };
        let (_doc, graph) = Doc::from_plain(plain);
        assert!(
            graph.pending().is_empty(),
            "Doc::from_plain must commit pending before return"
        );
    }
}

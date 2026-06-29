use editor_crdt::{Dot, ListOp, LwwRegOp, OpGraph, OrMapOp, OrSetOp};
use editor_model::{
    Anchor, AtomLeaf, Bias, EditOp, ModifierAttrOp, NodeAttrOp, NodeLwwOp, NodeType, PlainDoc,
    PlainNode, PlainNodeEntry, PlainTextNode, SeqClass, SeqItem, SpanOp, StyleOp, StyleRegOp,
    classify,
};

use crate::fragment_builder::{GraphSink, emit_text_run};
use crate::projected_state::{ProjectedState, SpineError};
use crate::{Selection, doc_start_selection};

#[derive(Debug)]
pub enum BuildError {
    MissingRoot,
    DanglingChild,
    UnsupportedNode,
    Spine(SpineError),
}

impl From<SpineError> for BuildError {
    fn from(e: SpineError) -> Self {
        BuildError::Spine(e)
    }
}

fn emit_node(
    entry: &PlainNodeEntry,
    parents: &[Dot],
    graph: &mut OpGraph<EditOp>,
    seq_pos: &mut usize,
) -> Result<(), BuildError> {
    let node_type = entry.node.as_type();

    match classify(node_type) {
        SeqClass::Block => {
            let dot = if node_type == NodeType::Root {
                Dot::ROOT
            } else {
                let dot = graph
                    .add_mut(EditOp::Seq(ListOp::Ins {
                        pos: *seq_pos,
                        item: SeqItem::Block {
                            node_type,
                            parents: parents.to_vec(),
                        },
                    }))
                    .expect("local seq block insert never conflicts")
                    .id;
                *seq_pos += 1;
                dot
            };

            for modifier in entry.modifiers.values() {
                graph
                    .add_mut(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                        target: dot,
                        modifier: modifier.clone(),
                    }))
                    .expect("local block modifier never conflicts");
            }
            if let Some(style_id) = &entry.style {
                graph
                    .add_mut(EditOp::NodeStyle(NodeLwwOp {
                        target: dot,
                        op: LwwRegOp::Set {
                            value: Some(style_id.clone()),
                        },
                    }))
                    .expect("local node style never conflicts");
            }
            if let Some(marker) = &entry.marker {
                graph
                    .add_mut(EditOp::NodeMarker(NodeLwwOp {
                        target: dot,
                        op: LwwRegOp::Set {
                            value: Some(marker.clone()),
                        },
                    }))
                    .expect("local node marker never conflicts");
            }
            for attr in entry.node.to_attrs() {
                graph
                    .add_mut(EditOp::NodeAttr(NodeAttrOp { target: dot, attr }))
                    .expect("local node attr never conflicts");
            }

            let mut child_parents = parents.to_vec();
            child_parents.push(dot);
            for child in &entry.children {
                emit_node(child, &child_parents, graph, seq_pos)?;
            }
            Ok(())
        }
        SeqClass::Text => {
            if let PlainNode::Text(PlainTextNode { text }) = &entry.node {
                let mut sink = GraphSink::new(graph);
                emit_text_run(
                    &mut sink,
                    seq_pos,
                    text,
                    &entry.modifiers,
                    entry.style.as_deref(),
                )
                .expect("local text run never conflicts");
            }
            Ok(())
        }
        SeqClass::Atom => {
            let leaf = AtomLeaf::from_plain_node(&entry.node).ok_or(BuildError::UnsupportedNode)?;
            let item = if leaf.is_block_level() {
                SeqItem::BlockAtom {
                    leaf,
                    parents: parents.to_vec(),
                }
            } else {
                SeqItem::Atom(leaf)
            };
            let dot = graph
                .add_mut(EditOp::Seq(ListOp::Ins {
                    pos: *seq_pos,
                    item,
                }))
                .expect("local seq atom insert never conflicts")
                .id;
            *seq_pos += 1;
            for modifier in entry.modifiers.values() {
                graph
                    .add_mut(EditOp::Span(SpanOp::AddSpan {
                        start: Anchor {
                            id: dot,
                            bias: Bias::Before,
                        },
                        end: Anchor {
                            id: dot,
                            bias: Bias::After,
                        },
                        modifier: modifier.clone(),
                    }))
                    .expect("local atom span never conflicts");
            }
            if let Some(style_id) = &entry.style {
                graph
                    .add_mut(EditOp::NodeStyle(NodeLwwOp {
                        target: dot,
                        op: LwwRegOp::Set {
                            value: Some(style_id.clone()),
                        },
                    }))
                    .expect("local atom style never conflicts");
            }
            Ok(())
        }
    }
}

fn build_graph_from_plain(template: &PlainDoc) -> Result<OpGraph<EditOp>, BuildError> {
    if !matches!(template.root.node, PlainNode::Root(_)) {
        return Err(BuildError::MissingRoot);
    }

    let mut graph = OpGraph::<EditOp>::with_actor(1);
    let mut seq_pos: usize = 0;

    emit_node(&template.root, &[], &mut graph, &mut seq_pos)?;

    for (style_id, entry) in template.styles.iter() {
        graph
            .add_mut(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Presence(OrMapOp::Set {
                    key: style_id.clone(),
                    value: (),
                }),
            }))
            .expect("local style presence never conflicts");
        graph
            .add_mut(EditOp::Style(StyleRegOp {
                style_id: style_id.clone(),
                op: StyleOp::Name(LwwRegOp::Set {
                    value: entry.name.clone(),
                }),
            }))
            .expect("local style name never conflicts");
        for modifier in entry.modifiers.iter() {
            graph
                .add_mut(EditOp::Style(StyleRegOp {
                    style_id: style_id.clone(),
                    op: StyleOp::Modifiers(OrSetOp::Add {
                        elem: modifier.clone(),
                    }),
                }))
                .expect("local style modifier never conflicts");
        }
    }

    Ok(graph)
}

pub(crate) fn load_from_plain(
    template: &PlainDoc,
) -> Result<(ProjectedState, Option<Selection>), BuildError> {
    let mut graph = build_graph_from_plain(template)?;
    graph.commit_mut();
    let state = ProjectedState::from_graph(graph)?;
    let sel = {
        let view = state.view();
        doc_start_selection(&view)
    };
    Ok((state, sel))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use editor_model::{
        Alignment, BlockquoteVariant, Marker, Modifier, ModifierType, NodeType,
        PlainBlockquoteNode, PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode,
        PlainRootNode, PlainStyleEntry, PlainTextNode,
    };

    use crate::projected_state::ProjectedState;

    use super::{build_graph_from_plain, load_from_plain};

    fn block_entry(children: Vec<PlainNodeEntry>, node: PlainNode) -> PlainNodeEntry {
        PlainNodeEntry {
            node,
            modifiers: BTreeMap::new(),
            style: None,
            marker: None,
            children,
        }
    }

    fn default_doc() -> PlainDoc {
        let text = block_entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "hi".to_string(),
            }),
        );
        let para = block_entry(vec![text], PlainNode::Paragraph(PlainParagraphNode {}));
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));

        PlainDoc {
            root,
            styles: BTreeMap::new(),
        }
    }

    #[test]
    fn build_graph_from_plain_for_default_doc() {
        let template = default_doc();
        let graph = build_graph_from_plain(&template).expect("builds graph");
        let state = ProjectedState::from_graph(graph).expect("projects");
        let view = state.view();
        let root = view.root().expect("root present");

        let children: Vec<_> = root.child_blocks().collect();
        assert_eq!(children.len(), 1, "exactly one paragraph child");
        assert_eq!(children[0].node_type(), NodeType::Paragraph);
        assert_eq!(children[0].inline_text(), "hi");
    }

    #[test]
    fn load_nested_blocks() {
        let text = block_entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "ab".to_string(),
            }),
        );
        let para = block_entry(vec![text], PlainNode::Paragraph(PlainParagraphNode {}));
        let bq = block_entry(
            vec![para],
            PlainNode::Blockquote(PlainBlockquoteNode {
                variant: BlockquoteVariant::LeftQuote,
            }),
        );
        let root = block_entry(vec![bq], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        let view = state.view();
        let root = view.root().expect("root present");
        let bq = root.child_blocks().next().expect("blockquote child");
        assert_eq!(bq.node_type(), NodeType::Blockquote);
        let para = bq.child_blocks().next().expect("paragraph child");
        assert_eq!(para.node_type(), NodeType::Paragraph);
        assert_eq!(para.inline_text(), "ab");
    }

    #[test]
    fn load_block_overlays() {
        let mut modifiers = BTreeMap::new();
        modifiers.insert(
            ModifierType::Alignment,
            Modifier::Alignment {
                value: Alignment::Center,
            },
        );
        let marker = Marker {
            modifiers: vec![Modifier::Bold],
            style: None,
        };

        let para = PlainNodeEntry {
            node: PlainNode::Paragraph(PlainParagraphNode {}),
            modifiers,
            style: Some("s1".to_string()),
            marker: Some(marker.clone()),
            children: vec![],
        };
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        let para_dot = para.dot().expect("paragraph is a real node");

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para_dot)
                .get(&ModifierType::Alignment),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            })
        );
        assert_eq!(
            state.node_styles().value_of(para_dot),
            Some("s1".to_string())
        );
        assert_eq!(state.node_markers().value_of(para_dot), Some(marker));
    }

    #[test]
    fn load_style_registry_draws_on_leaf() {
        let mut style_mods = BTreeSet::new();
        style_mods.insert(Modifier::FontSize { value: 1800 });
        let mut styles = BTreeMap::new();
        styles.insert(
            "s1".to_string(),
            PlainStyleEntry {
                name: "Heading".to_string(),
                modifiers: style_mods,
            },
        );

        let text = block_entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "x".to_string(),
            }),
        );
        let para = PlainNodeEntry {
            node: PlainNode::Paragraph(PlainParagraphNode {}),
            modifiers: BTreeMap::new(),
            style: Some("s1".to_string()),
            marker: None,
            children: vec![text],
        };
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc { root, styles };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        assert!(state.styles().registered("s1"), "style registered");

        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        let inline = para.inline();
        assert_eq!(inline.len(), 1, "one inline char");
        assert_eq!(
            inline[0].effective.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1800 }),
            "leaf effective draws the style modifier"
        );
    }

    #[test]
    fn load_text_modifiers_draw_on_leaf() {
        let mut text_mods = BTreeMap::new();
        text_mods.insert(ModifierType::Bold, Modifier::Bold);

        let text = PlainNodeEntry {
            node: PlainNode::Text(PlainTextNode {
                text: "ab".to_string(),
            }),
            modifiers: text_mods,
            style: None,
            marker: None,
            children: vec![],
        };
        let para = block_entry(vec![text], PlainNode::Paragraph(PlainParagraphNode {}));
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        let inline = para.inline();
        assert_eq!(inline.len(), 2, "two inline chars");
        for item in &inline {
            assert_eq!(
                item.effective.get(&ModifierType::Bold),
                Some(&Modifier::Bold),
                "each loaded char draws the text-node bold modifier"
            );
        }
    }

    #[test]
    fn load_text_style_draws_on_leaf() {
        let mut style_mods = BTreeSet::new();
        style_mods.insert(Modifier::Bold);
        let mut styles = BTreeMap::new();
        styles.insert(
            "s1".to_string(),
            PlainStyleEntry {
                name: "Strong".to_string(),
                modifiers: style_mods,
            },
        );

        let text = PlainNodeEntry {
            node: PlainNode::Text(PlainTextNode {
                text: "x".to_string(),
            }),
            modifiers: BTreeMap::new(),
            style: Some("s1".to_string()),
            marker: None,
            children: vec![],
        };
        let para = block_entry(vec![text], PlainNode::Paragraph(PlainParagraphNode {}));
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc { root, styles };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        assert!(state.styles().registered("s1"), "style registered");
        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        let inline = para.inline();
        assert_eq!(inline.len(), 1, "one inline char");
        assert_eq!(
            inline[0].effective.get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "leaf draws the text-node style's modifier"
        );
    }

    #[test]
    fn load_plain_text_still_char_only() {
        let text = block_entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "abc".to_string(),
            }),
        );
        let para = block_entry(vec![text], PlainNode::Paragraph(PlainParagraphNode {}));
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };

        let (state, _sel) = load_from_plain(&template).expect("loads");
        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        assert_eq!(para.inline_text(), "abc");
        let inline = para.inline();
        assert_eq!(inline.len(), 3, "three inline chars");
        for item in &inline {
            assert!(
                item.effective.get(&ModifierType::Bold).is_none(),
                "plain loaded text gains no spurious styling"
            );
        }
    }

    #[test]
    fn load_returns_initial_caret() {
        let template = default_doc();
        let (state, sel) = load_from_plain(&template).expect("loads");
        let sel = sel.expect("doc-start caret present");
        assert_eq!(sel.anchor, sel.head, "caret is collapsed");

        let view = state.view();
        let node = view
            .node(sel.anchor.node)
            .expect("caret node is live in the view");
        assert!(
            sel.anchor.offset <= node.children().count(),
            "caret offset within node bounds"
        );
    }

    #[test]
    fn load_empty_paragraph_caret_at_zero() {
        let para = block_entry(vec![], PlainNode::Paragraph(PlainParagraphNode {}));
        let root = block_entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };

        let (state, sel) = load_from_plain(&template).expect("loads");
        let sel = sel.expect("caret present");

        let view = state.view();
        let para = view
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .expect("paragraph");
        let para_dot = para.dot().expect("paragraph is a real node");
        assert_eq!(sel.anchor.node, para_dot);
        assert_eq!(sel.anchor.offset, 0);
    }

    #[test]
    fn load_block_image_and_inline_tab() {
        use editor_model::{AtomLeaf, ChildView, PlainImageNode, PlainTabNode};

        let img = block_entry(
            vec![],
            PlainNode::Image(PlainImageNode {
                id: Some("img-1".to_string()),
                proportion: 50,
            }),
        );
        let tab = block_entry(vec![], PlainNode::Tab(PlainTabNode {}));
        let para = block_entry(
            vec![tab],
            PlainNode::Paragraph(PlainParagraphNode::default()),
        );
        let root = block_entry(vec![img, para], PlainNode::Root(PlainRootNode::default()));

        let template = PlainDoc {
            root,
            styles: BTreeMap::new(),
        };
        let graph = build_graph_from_plain(&template).expect("builds graph with atoms");
        let state = ProjectedState::from_graph(graph).expect("projects");
        let view = state.view();
        let root = view.root().expect("root present");

        match root.child_at(0).expect("first child") {
            ChildView::Leaf(l) => match l.as_atom() {
                Some(AtomLeaf::Image { node }) => {
                    assert_eq!(node.id.get(), &Some("img-1".to_string()));
                    assert_eq!(
                        *node.proportion.get(),
                        50,
                        "image proportion preserved on load"
                    );
                }
                other => panic!("expected block image atom, got {other:?}"),
            },
            ChildView::Block(_) => panic!("expected the image as a block atom leaf"),
        }

        let para = root.child_blocks().next().expect("paragraph present");
        let has_tab = para
            .children()
            .any(|c| matches!(c, ChildView::Leaf(l) if l.as_atom() == Some(&AtomLeaf::Tab)));
        assert!(has_tab, "inline tab loaded into the paragraph");
    }
}

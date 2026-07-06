use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Marker, Modifier, ModifierType, NodeView, OwnModifier, PlainDoc, PlainNode,
    PlainNodeEntry, PlainTextNode, ProjectedDoc,
};

pub fn to_plain(projected: &ProjectedDoc) -> PlainDoc {
    let view = DocView::new(projected);
    let root = match view.root() {
        Some(root) => emit_block(projected, &root),
        None => PlainDoc::default().root,
    };

    PlainDoc { root }
}

fn emit_block(projected: &ProjectedDoc, nv: &NodeView) -> PlainNodeEntry {
    let mut children: Vec<PlainNodeEntry> = Vec::new();
    let mut run = PendingRun::default();

    for (slot, child) in nv.children().enumerate() {
        match child {
            ChildView::Block(b) => {
                run.flush(&mut children);
                children.push(emit_block(projected, &b));
            }
            ChildView::Leaf(l) => {
                let own = nv.leaf_state_at(slot).map(|s| s.own);
                if let Some(ch) = l.as_char() {
                    let modifiers = own.map(span_modifiers).unwrap_or_default();
                    run.push(ch, modifiers, &mut children);
                } else if let Some(node) = l.node() {
                    run.flush(&mut children);
                    children.push(emit_atom(node.to_plain(), own));
                }
            }
        }
    }
    run.flush(&mut children);

    let dot = nv.dot();
    PlainNodeEntry {
        node: nv.node().to_plain(),
        modifiers: dot
            .map(|d| block_modifiers(projected, d))
            .unwrap_or_default(),
        marker: dot.and_then(|d| node_marker(projected, d)),
        children,
    }
}

fn emit_atom(node: PlainNode, own: Option<&BTreeMap<ModifierType, OwnModifier>>) -> PlainNodeEntry {
    PlainNodeEntry {
        node,
        modifiers: own.map(span_modifiers).unwrap_or_default(),
        marker: None,
        children: Vec::new(),
    }
}

#[derive(Default)]
struct PendingRun {
    active: bool,
    text: String,
    modifiers: BTreeMap<ModifierType, Modifier>,
}

impl PendingRun {
    fn push(
        &mut self,
        ch: char,
        modifiers: BTreeMap<ModifierType, Modifier>,
        children: &mut Vec<PlainNodeEntry>,
    ) {
        if self.active && self.modifiers != modifiers {
            self.flush(children);
        }
        if !self.active {
            self.active = true;
            self.modifiers = modifiers;
            self.text.clear();
        }
        self.text.push(ch);
    }

    fn flush(&mut self, children: &mut Vec<PlainNodeEntry>) {
        if !self.active {
            return;
        }
        children.push(PlainNodeEntry {
            node: PlainNode::Text(PlainTextNode {
                text: std::mem::take(&mut self.text),
            }),
            modifiers: std::mem::take(&mut self.modifiers),
            marker: None,
            children: Vec::new(),
        });
        self.active = false;
    }
}

fn span_modifiers(own: &BTreeMap<ModifierType, OwnModifier>) -> BTreeMap<ModifierType, Modifier> {
    own.iter().map(|(ty, o)| (*ty, o.value.clone())).collect()
}

fn block_modifiers(projected: &ProjectedDoc, dot: Dot) -> BTreeMap<ModifierType, Modifier> {
    projected
        .block_modifiers
        .get(&dot)
        .cloned()
        .unwrap_or_default()
}

fn node_marker(projected: &ProjectedDoc, dot: Dot) -> Option<Marker> {
    projected.node_markers.get(&dot).cloned().flatten()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use editor_model::{
        AtomLeaf, Modifier, ModifierType, PlainBlockquoteNode, PlainDoc, PlainNode, PlainNodeEntry,
        PlainParagraphNode, PlainRootNode, PlainTextNode,
    };

    use crate::state::State;

    fn entry(children: Vec<PlainNodeEntry>, node: PlainNode) -> PlainNodeEntry {
        PlainNodeEntry {
            node,
            modifiers: BTreeMap::new(),
            marker: None,
            children,
        }
    }

    fn round_trip(plain: &PlainDoc) {
        let s1 = State::from_plain(plain).expect("load template");
        let plain2 = s1.to_plain();
        let s2 = State::from_plain(&plain2).expect("load round-trip");
        crate::test_utils::assert_state_eq_impl(&s1, &s2);
    }

    #[test]
    fn round_trip_nested_blocks_span_and_block_modifier() {
        let mut text_entry = entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "Hi".to_string(),
            }),
        );
        text_entry
            .modifiers
            .insert(ModifierType::Bold, Modifier::Bold);

        let mut para_entry = entry(
            vec![text_entry],
            PlainNode::Paragraph(PlainParagraphNode {}),
        );
        para_entry
            .modifiers
            .insert(ModifierType::FontSize, Modifier::FontSize { value: 1600 });

        let bq_text = entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "Yo".to_string(),
            }),
        );
        let bq_para = entry(vec![bq_text], PlainNode::Paragraph(PlainParagraphNode {}));
        let bq = entry(
            vec![bq_para],
            PlainNode::Blockquote(PlainBlockquoteNode::default()),
        );

        let root_entry = entry(
            vec![para_entry, bq],
            PlainNode::Root(PlainRootNode::default()),
        );

        round_trip(&PlainDoc { root: root_entry });
    }

    #[test]
    fn round_trip_mixed_runs_and_atom() {
        let plain_text = entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "ab".to_string(),
            }),
        );
        let mut bold_text = entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "cd".to_string(),
            }),
        );
        bold_text
            .modifiers
            .insert(ModifierType::Bold, Modifier::Bold);
        let hr = entry(vec![], AtomLeaf::HardBreak.into_node().to_plain());

        let para = entry(
            vec![plain_text, bold_text, hr],
            PlainNode::Paragraph(PlainParagraphNode {}),
        );
        let root = entry(vec![para], PlainNode::Root(PlainRootNode::default()));

        round_trip(&PlainDoc { root });
    }
}

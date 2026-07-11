use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Modifier, ModifierType, NodeView, OwnModifier, PlainDoc, PlainNode,
    PlainNodeEntry, PlainTextNode, ProjectedDoc,
};

pub fn to_plain(projected: &ProjectedDoc) -> PlainDoc {
    to_plain_impl(projected, false)
}

/// Like [`to_plain`] but omits synthetic scaffold blocks (e.g. the normalizer's
/// trailing paragraph), reproducing authored content only.
#[cfg(any(test, feature = "test-utils"))]
pub(crate) fn to_plain_authored(projected: &ProjectedDoc) -> PlainDoc {
    to_plain_impl(projected, true)
}

fn to_plain_impl(projected: &ProjectedDoc, authored_only: bool) -> PlainDoc {
    let view = DocView::new(projected);
    let root = match view.root() {
        Some(root) => emit_block(projected, &root, authored_only),
        None => PlainDoc::default().root,
    };

    PlainDoc { root }
}

fn emit_block(projected: &ProjectedDoc, nv: &NodeView, authored_only: bool) -> PlainNodeEntry {
    let mut children: Vec<PlainNodeEntry> = Vec::new();
    let mut run = PendingRun::default();

    for (slot, child) in nv.children().enumerate() {
        match child {
            ChildView::Block(b) => {
                if authored_only && b.id().is_synthetic() {
                    continue;
                }
                run.flush(&mut children);
                children.push(emit_block(projected, &b, authored_only));
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
        carry: dot.map(|d| carry_of(projected, d)).unwrap_or_default(),
        children,
    }
}

fn emit_atom(node: PlainNode, own: Option<&BTreeMap<ModifierType, OwnModifier>>) -> PlainNodeEntry {
    PlainNodeEntry {
        node,
        modifiers: own.map(span_modifiers).unwrap_or_default(),
        carry: Vec::new(),
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
            carry: Vec::new(),
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

fn carry_of(projected: &ProjectedDoc, dot: Dot) -> Vec<Modifier> {
    projected.carry_modifiers(dot).into_values().collect()
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
            carry: Vec::new(),
            children,
        }
    }

    fn round_trip(plain: &PlainDoc) {
        let s1 = State::from_plain(plain).expect("load template");
        let plain2 = s1.to_plain();
        let s2 = State::from_plain(&plain2).expect("load round-trip");
        crate::test_utils::assert_state_eq_impl(&s1, &s2);
    }

    fn para_with_carry(carry: Vec<Modifier>) -> PlainDoc {
        let mut para = entry(vec![], PlainNode::Paragraph(PlainParagraphNode {}));
        para.carry = carry;
        let root = entry(vec![para], PlainNode::Root(PlainRootNode::default()));
        PlainDoc { root }
    }

    fn loaded_paragraph_carry(plain: &PlainDoc) -> Vec<Modifier> {
        let state = State::from_plain(plain).expect("load template");
        let out = state.to_plain();
        out.root.children[0].carry.clone()
    }

    #[test]
    fn authored_to_plain_skips_trailing_scaffold() {
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
        let root = entry(vec![bq], PlainNode::Root(PlainRootNode::default()));
        let plain = PlainDoc { root };
        let s1 = State::from_plain(&plain).expect("load template");
        assert_ne!(
            s1.to_plain(),
            plain,
            "plain to_plain includes the normalizer's trailing scaffold paragraph"
        );
        assert_eq!(
            crate::to_plain::to_plain_authored(s1.projected.projected()),
            plain,
            "authored to_plain reproduces exactly the authored content"
        );
    }

    #[test]
    fn carry_survives_to_plain_and_load() {
        let carry = loaded_paragraph_carry(&para_with_carry(vec![Modifier::Bold]));
        assert_eq!(carry, vec![Modifier::Bold]);
    }

    #[test]
    fn non_carry_kind_in_plain_carry_is_dropped() {
        let carry = loaded_paragraph_carry(&para_with_carry(vec![Modifier::Link {
            href: "https://e.com".to_string(),
        }]));
        assert!(
            carry.is_empty(),
            "a non-carry kind supplied via plain carry never survives the round trip"
        );
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

    /// plain doc -> build -> graph.changesets -> encode_changesets(from_local_ops) ->
    /// decode(.into_graph_input()) -> from_changesets -> 투영 -> to_plain == 원본 plain.
    /// 기존 `round_trip`(plain -> State -> to_plain -> State)과 달리, 코덱의 wire 경계
    /// (encode_changesets/decode_changesets)를 실제로 관통시켜 의미 보존을 증명한다.
    #[test]
    fn plain_doc_survives_codec_round_trip() {
        let mut text_entry = entry(
            vec![],
            PlainNode::Text(PlainTextNode {
                text: "Hi".to_string(),
            }),
        );
        text_entry
            .modifiers
            .insert(ModifierType::Bold, Modifier::Bold);
        let hr = entry(vec![], AtomLeaf::HardBreak.into_node().to_plain());

        let mut para_entry = entry(
            vec![text_entry, hr],
            PlainNode::Paragraph(PlainParagraphNode {}),
        );
        para_entry
            .modifiers
            .insert(ModifierType::FontSize, Modifier::FontSize { value: 1600 });
        para_entry.carry = vec![Modifier::Bold];

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
        let plain = PlainDoc { root: root_entry };

        let s1 = State::from_plain(&plain).expect("load template");
        let css = s1.graph().changesets_as_vec();
        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(css),
        )
        .unwrap();
        let decoded = editor_codec::decode_changesets(&bytes)
            .unwrap()
            .into_graph_input();
        let s2 = State::from_changesets(decoded, None).expect("load round-trip");

        assert_eq!(s1.to_plain(), s2.to_plain());
        assert_eq!(
            s1.projected.projected(),
            s2.projected.projected(),
            "코덱 왕복 전후 ProjectedDoc 동등(atom 없는 픽스처 — 장부 재시딩 이슈 없음)"
        );
    }
}

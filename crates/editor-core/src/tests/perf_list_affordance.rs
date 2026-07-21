//! Run: cargo test -p editor-core --release perf_list_affordance -- --ignored --nocapture

use std::collections::BTreeMap;
use std::time::Instant;

use editor_model::{
    ChildView, PlainBulletListNode, PlainDoc, PlainListItemNode, PlainNode, PlainNodeEntry,
    PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, State};

use crate::block_state::resolve_block_state;

fn plain_text(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn plain_item(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::ListItem(PlainListItemNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![PlainNodeEntry {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![plain_text(text)],
        }],
    }
}

fn build_list_state(items: usize) -> (State, editor_crdt::Dot) {
    let list = PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: (0..items)
            .map(|i| plain_item(format!("item {i}")))
            .collect(),
    };
    let plain = PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![list],
        },
    };
    let (state, handles) = build_state_from_plain(plain);
    // path [0, 0, 0] = list -> first item -> its paragraph
    let first_para = *handles
        .get(&vec![0, 0, 0])
        .expect("first paragraph handle must exist");
    (state, first_para)
}

#[test]
#[ignore]
fn perf_list_affordance_resolve() {
    let resource = editor_resource::Resource::new_test();
    let (mut state, first_para) = build_list_state(200);
    state.selection = Some(Selection::collapsed(Position::new(first_para, 0)));

    for round in 0..3 {
        let started = Instant::now();
        let bs = resolve_block_state(&state, &resource).unwrap();
        let elapsed = started.elapsed();
        println!(
            "round {round}: resolve_block_state(list 200 items) = {:?} (indent={}, outdent={})",
            elapsed, bs.list.indent, bs.list.outdent
        );
    }
}

fn long_paragraph_text(target_len: usize) -> String {
    const SENTENCE: &str = "The quick brown fox jumps over the lazy dog. Second sentence follows here. Third sentence continues the flow. ";
    let mut text = String::with_capacity(target_len + SENTENCE.len());
    while text.len() < target_len {
        text.push_str(SENTENCE);
    }
    text.truncate(target_len);
    text
}

fn build_long_paragraph_state(chars: usize) -> State {
    let plain = PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![PlainNodeEntry {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: vec![plain_text(long_paragraph_text(chars))],
            }],
        },
    };
    let (state, _handles) = build_state_from_plain(plain);
    state
}

fn place_caret_at_paragraph_offset(state: &mut State, offset: usize) {
    let para_id = {
        let view = state.view();
        // path [0] = root -> single paragraph
        match view.root().unwrap().child_at(0).unwrap() {
            ChildView::Block(b) => b.id(),
            ChildView::Leaf(_) => panic!("expected paragraph block"),
        }
    };
    state.selection = Some(Selection::collapsed(Position::new(para_id, offset)));
}

#[test]
#[ignore]
fn perf_expansion_affordance_resolve() {
    let resource = editor_resource::Resource::new_test();
    let mut state = build_long_paragraph_state(4000);
    place_caret_at_paragraph_offset(&mut state, 2000);
    for round in 0..3 {
        let started = Instant::now();
        let bs = resolve_block_state(&state, &resource).unwrap();
        println!(
            "round {round}: resolve_block_state(4k-char paragraph) = {:?} (word={}, sentence={})",
            started.elapsed(),
            bs.expansion.word,
            bs.expansion.sentence
        );
    }
}

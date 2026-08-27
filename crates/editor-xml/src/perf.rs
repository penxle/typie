use std::collections::BTreeMap;
use std::time::Instant;

use editor_model::{
    PlainBlockquoteNode, PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode,
    PlainTextNode,
};
use editor_state::State;

use crate::reader::from_xml;
use crate::test_support::live_heads;
use crate::tree::{InlineEntry, InlineLeaf, XmlChild, XmlNode, XmlTree};
use crate::writer::to_xml;
use crate::{ChangeCounts, edit};

const BLOCKS: usize = 1000;
const CHARS_PER_BLOCK: usize = 100;
const QUOTE_AT: usize = 500;

fn paragraph(nth: usize) -> PlainNodeEntry {
    let text: String = (0..CHARS_PER_BLOCK)
        .map(|i| char::from(b'a' + ((nth + i) % 26) as u8))
        .collect();
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![PlainNodeEntry {
            node: PlainNode::Text(PlainTextNode { text }),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: Vec::new(),
        }],
    }
}

/// A thousand root children and a hundred thousand characters, one of the
/// children a blockquote so that an unwrap has something to unwrap.
fn big_doc() -> PlainDoc {
    let mut children: Vec<PlainNodeEntry> = (0..BLOCKS).map(paragraph).collect();
    let quoted = vec![
        children[QUOTE_AT].clone(),
        children[QUOTE_AT + 1].clone(),
        children[QUOTE_AT + 2].clone(),
    ];
    children.splice(
        QUOTE_AT..QUOTE_AT + 3,
        [PlainNodeEntry {
            node: PlainNode::Blockquote(PlainBlockquoteNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: quoted,
        }],
    );
    children.push(paragraph(BLOCKS));
    PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children,
        },
    }
}

fn block_at(tree: &mut XmlTree, index: usize) -> &mut XmlNode {
    match &mut tree.root.children[index] {
        XmlChild::Block(block) => block,
        XmlChild::Inline(_) => panic!("root child {index} is not a block"),
    }
}

fn retyped(tree: &XmlTree, index: usize, text: &str) -> XmlTree {
    let mut out = tree.clone();
    let block = block_at(&mut out, index);
    let pos = block.pos;
    block.children = text
        .chars()
        .map(|ch| {
            XmlChild::Inline(InlineEntry {
                pos,
                leaf: InlineLeaf::Char(ch),
                own: BTreeMap::new(),
            })
        })
        .collect();
    out
}

fn reordered(tree: &XmlTree, from: usize, len: usize) -> XmlTree {
    let mut out = tree.clone();
    out.root.children[from..from + len].reverse();
    out
}

fn unwrapped(tree: &XmlTree, index: usize) -> XmlTree {
    let mut out = tree.clone();
    let lifted: Vec<XmlChild> = block_at(&mut out, index)
        .block_children()
        .cloned()
        .map(XmlChild::Block)
        .collect();
    out.root.children.splice(index..index + 1, lifted);
    out
}

fn timed_edit(label: &str, state: &State, target: &XmlTree) -> ChangeCounts {
    let started = Instant::now();
    let outcome = edit(state.clone(), target).expect("the edit applies");
    println!("{label}: {:?} {:?}", started.elapsed(), outcome.changed);
    outcome.changed
}

#[test]
#[ignore = "perf smoke; run with --release -- --ignored perf"]
fn perf_smoke_of_a_hundred_thousand_character_document() {
    let doc = big_doc();
    let state = State::from_plain(&doc).expect("the document loads");
    let heads = live_heads(&state);

    let started = Instant::now();
    let xml = to_xml(&state, &heads).expect("the document serializes");
    println!(
        "to_xml: {:?} ({} bytes, {} root children)",
        started.elapsed(),
        xml.len(),
        state.to_plain().root.children.len()
    );

    let started = Instant::now();
    let tree = from_xml(&xml).expect("the file parses");
    println!("from_xml: {:?}", started.elapsed());

    let text: String = (0..CHARS_PER_BLOCK).map(|_| 'z').collect();
    timed_edit(
        "edit (one paragraph retyped)",
        &state,
        &retyped(&tree, 0, &text),
    );
    timed_edit(
        "edit (twenty blocks reordered)",
        &state,
        &reordered(&tree, 100, 20),
    );
    timed_edit(
        "edit (one blockquote unwrapped)",
        &state,
        &unwrapped(&tree, QUOTE_AT),
    );
}

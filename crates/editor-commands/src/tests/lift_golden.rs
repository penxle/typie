use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    Modifier, ModifierType, PlainBlockquoteNode, PlainBulletListNode, PlainDoc, PlainListItemNode,
    PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Affinity, Position, Selection, State};
use hashbrown::HashMap;

use crate::commands::lift_list_item;
use crate::test_utils::selection_shape;

fn text(t: &str) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode {
            text: t.to_string(),
        }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn bold_text(t: &str) -> PlainNodeEntry {
    let mut e = text(t);
    e.modifiers.insert(ModifierType::Bold, Modifier::Bold);
    e
}

fn para(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn list_item(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::ListItem(PlainListItemNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn bullet_list(items: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: items,
    }
}

fn blockquote(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Blockquote(PlainBlockquoteNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn root(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn dot_at(handles: &HashMap<Vec<usize>, Dot>, path: &[usize]) -> Dot {
    handles[&path.to_vec()]
}

fn run_lift(doc_root: PlainNodeEntry, anchor: (&[usize], usize), head: (&[usize], usize)) -> State {
    let (mut state, handles) = build_state_from_plain(PlainDoc { root: doc_root });
    let anchor_pos = Position::new(dot_at(&handles, anchor.0), anchor.1);
    let head_pos = Position::new(dot_at(&handles, head.0), head.1);
    state.selection = Some(Selection::new(anchor_pos, head_pos));

    let mut tr = editor_transaction::Transaction::new(&state);
    assert!(
        lift_list_item(&mut tr).unwrap(),
        "lift_list_item must succeed"
    );
    let (after, ..) = tr.commit();
    after
}

fn plain_60_doc() -> PlainNodeEntry {
    let items: Vec<PlainNodeEntry> = (0..60)
        .map(|i| list_item(vec![para(vec![text(&i.to_string())])]))
        .collect();
    root(vec![bullet_list(items), para(vec![])])
}

fn consecutive_reverse_doc() -> PlainNodeEntry {
    root(vec![
        bullet_list(vec![
            list_item(vec![para(vec![text("A")])]),
            list_item(vec![para(vec![text("B")])]),
            list_item(vec![para(vec![text("C")])]),
            list_item(vec![para(vec![text("D")])]),
            list_item(vec![para(vec![text("E")])]),
            list_item(vec![para(vec![text("F")])]),
        ]),
        para(vec![]),
    ])
}

fn nested_doc() -> PlainNodeEntry {
    root(vec![
        bullet_list(vec![list_item(vec![
            para(vec![text("outer")]),
            bullet_list(vec![
                list_item(vec![
                    para(vec![text("B")]),
                    bullet_list(vec![list_item(vec![para(vec![text("b_sub")])])]),
                ]),
                list_item(vec![para(vec![text("C")])]),
                list_item(vec![para(vec![text("D")])]),
            ]),
        ])]),
        para(vec![]),
    ])
}

fn multi_list_doc() -> PlainNodeEntry {
    root(vec![
        bullet_list(vec![list_item(vec![para(vec![text("A")])])]),
        bullet_list(vec![list_item(vec![para(vec![text("B")])])]),
        para(vec![]),
    ])
}

fn styled_doc() -> PlainNodeEntry {
    root(vec![
        bullet_list(vec![
            list_item(vec![para(vec![bold_text("A"), text(" B")])]),
            list_item(vec![para(vec![text("C")])]),
        ]),
        para(vec![]),
    ])
}

fn single_item_blockquote_doc() -> PlainNodeEntry {
    root(vec![
        blockquote(vec![bullet_list(vec![list_item(vec![para(vec![text(
            "Z",
        )])])])]),
        para(vec![]),
    ])
}

const PLAIN_60_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "0",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "1",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "2",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "3",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "4",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "5",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "6",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "7",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "8",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "9",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "10",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "11",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "12",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "13",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "14",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "15",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "16",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "17",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "18",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "19",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "20",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "21",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "22",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "23",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "24",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "25",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "26",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "27",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "28",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "29",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "30",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "31",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "32",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "33",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "34",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "35",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "36",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "37",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "38",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "39",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "40",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "41",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "42",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "43",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "44",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "45",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "46",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "47",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "48",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "49",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "50",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "51",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "52",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "53",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "54",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "55",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "56",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "57",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "58",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "59",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

const CONSECUTIVE_REVERSE_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: BulletList(
                    PlainBulletListNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: ListItem(
                            PlainListItemNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Paragraph(
                                    PlainParagraphNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: Text(
                                            PlainTextNode {
                                                text: "A",
                                            },
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "B",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "C",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "D",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "E",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: BulletList(
                    PlainBulletListNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: ListItem(
                            PlainListItemNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Paragraph(
                                    PlainParagraphNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: Text(
                                            PlainTextNode {
                                                text: "F",
                                            },
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

const NESTED_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: BulletList(
                    PlainBulletListNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: ListItem(
                            PlainListItemNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Paragraph(
                                    PlainParagraphNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: Text(
                                            PlainTextNode {
                                                text: "outer",
                                            },
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [],
                                    },
                                ],
                            },
                        ],
                    },
                    PlainNodeEntry {
                        node: ListItem(
                            PlainListItemNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Paragraph(
                                    PlainParagraphNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: Text(
                                            PlainTextNode {
                                                text: "B",
                                            },
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [],
                                    },
                                ],
                            },
                            PlainNodeEntry {
                                node: BulletList(
                                    PlainBulletListNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: ListItem(
                                            PlainListItemNode,
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [
                                            PlainNodeEntry {
                                                node: Paragraph(
                                                    PlainParagraphNode,
                                                ),
                                                modifiers: {},
                                                carry: [],
                                                children: [
                                                    PlainNodeEntry {
                                                        node: Text(
                                                            PlainTextNode {
                                                                text: "b_sub",
                                                            },
                                                        ),
                                                        modifiers: {},
                                                        carry: [],
                                                        children: [],
                                                    },
                                                ],
                                            },
                                        ],
                                    },
                                    PlainNodeEntry {
                                        node: ListItem(
                                            PlainListItemNode,
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [
                                            PlainNodeEntry {
                                                node: Paragraph(
                                                    PlainParagraphNode,
                                                ),
                                                modifiers: {},
                                                carry: [],
                                                children: [
                                                    PlainNodeEntry {
                                                        node: Text(
                                                            PlainTextNode {
                                                                text: "C",
                                                            },
                                                        ),
                                                        modifiers: {},
                                                        carry: [],
                                                        children: [],
                                                    },
                                                ],
                                            },
                                        ],
                                    },
                                    PlainNodeEntry {
                                        node: ListItem(
                                            PlainListItemNode,
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [
                                            PlainNodeEntry {
                                                node: Paragraph(
                                                    PlainParagraphNode,
                                                ),
                                                modifiers: {},
                                                carry: [],
                                                children: [
                                                    PlainNodeEntry {
                                                        node: Text(
                                                            PlainTextNode {
                                                                text: "D",
                                                            },
                                                        ),
                                                        modifiers: {},
                                                        carry: [],
                                                        children: [],
                                                    },
                                                ],
                                            },
                                        ],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

const MULTI_LIST_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "A",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "B",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

const STYLED_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: "A",
                            },
                        ),
                        modifiers: {
                            Bold: Bold,
                        },
                        carry: [],
                        children: [],
                    },
                    PlainNodeEntry {
                        node: Text(
                            PlainTextNode {
                                text: " B",
                            },
                        ),
                        modifiers: {},
                        carry: [],
                        children: [],
                    },
                ],
            },
            PlainNodeEntry {
                node: BulletList(
                    PlainBulletListNode,
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: ListItem(
                            PlainListItemNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Paragraph(
                                    PlainParagraphNode,
                                ),
                                modifiers: {},
                                carry: [],
                                children: [
                                    PlainNodeEntry {
                                        node: Text(
                                            PlainTextNode {
                                                text: "C",
                                            },
                                        ),
                                        modifiers: {},
                                        carry: [],
                                        children: [],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

const SINGLE_ITEM_BLOCKQUOTE_GOLDEN: &str = r#"PlainDoc {
    root: PlainNodeEntry {
        node: Root(
            PlainRootNode {
                layout_mode: Continuous {
                    max_width: 600,
                },
            },
        ),
        modifiers: {},
        carry: [],
        children: [
            PlainNodeEntry {
                node: Blockquote(
                    PlainBlockquoteNode {
                        variant: LeftLine,
                    },
                ),
                modifiers: {},
                carry: [],
                children: [
                    PlainNodeEntry {
                        node: Paragraph(
                            PlainParagraphNode,
                        ),
                        modifiers: {},
                        carry: [],
                        children: [
                            PlainNodeEntry {
                                node: Text(
                                    PlainTextNode {
                                        text: "Z",
                                    },
                                ),
                                modifiers: {},
                                carry: [],
                                children: [],
                            },
                        ],
                    },
                ],
            },
            PlainNodeEntry {
                node: Paragraph(
                    PlainParagraphNode,
                ),
                modifiers: {},
                carry: [],
                children: [],
            },
        ],
    },
}"#;

#[test]
fn golden_plain_60() {
    let after = run_lift(plain_60_doc(), (&[0, 0, 0], 0), (&[0, 59, 0], 2));
    assert_eq!(format!("{:#?}", after.to_plain()), PLAIN_60_GOLDEN);
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![0, 0], Affinity::Downstream),
            (vec![59, 2], Affinity::Downstream)
        ))
    );
}

#[test]
fn golden_consecutive_reverse() {
    let after = run_lift(consecutive_reverse_doc(), (&[0, 1, 0], 0), (&[0, 4, 0], 1));
    assert_eq!(
        format!("{:#?}", after.to_plain()),
        CONSECUTIVE_REVERSE_GOLDEN
    );
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![1, 0], Affinity::Downstream),
            (vec![4, 1], Affinity::Downstream)
        ))
    );
}

#[test]
fn golden_nested() {
    let after = run_lift(nested_doc(), (&[0, 0, 1, 0, 0], 0), (&[0, 0, 1, 0, 0], 0));
    assert_eq!(format!("{:#?}", after.to_plain()), NESTED_GOLDEN);
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![0, 1, 0, 0], Affinity::Downstream),
            (vec![0, 1, 0, 0], Affinity::Downstream)
        ))
    );
}

#[test]
fn golden_multi_list() {
    let after = run_lift(multi_list_doc(), (&[0, 0, 0], 0), (&[1, 0, 0], 1));
    assert_eq!(format!("{:#?}", after.to_plain()), MULTI_LIST_GOLDEN);
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![0, 0], Affinity::Downstream),
            (vec![1, 1], Affinity::Downstream)
        ))
    );
}

#[test]
fn golden_styled() {
    let after = run_lift(styled_doc(), (&[0, 0, 0], 0), (&[0, 0, 0], 0));
    assert_eq!(format!("{:#?}", after.to_plain()), STYLED_GOLDEN);
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![0, 0], Affinity::Downstream),
            (vec![0, 0], Affinity::Downstream)
        ))
    );
}

#[test]
fn golden_single_item_blockquote() {
    let after = run_lift(
        single_item_blockquote_doc(),
        (&[0, 0, 0, 0], 0),
        (&[0, 0, 0, 0], 0),
    );
    assert_eq!(
        format!("{:#?}", after.to_plain()),
        SINGLE_ITEM_BLOCKQUOTE_GOLDEN
    );
    assert_eq!(
        selection_shape(&after),
        Some((
            (vec![0, 0, 0], Affinity::Downstream),
            (vec![0, 0, 0], Affinity::Downstream)
        ))
    );
}

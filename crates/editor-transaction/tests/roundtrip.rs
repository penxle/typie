use editor_crdt::Dot;
use editor_macros::state;
use editor_model::{
    CalloutVariant, ChildView, Modifier, ModifierType, Node, PlainCalloutNode, PlainNode,
};
use editor_state::{Position, Selection};
use editor_transaction::Step;

fn block_text(state: &editor_state::State, elem: &Dot) -> String {
    state.view().node(*elem).unwrap().inline_text()
}

fn char_dot(state: &editor_state::State, block: &Dot, offset: usize) -> Dot {
    match state.view().node(*block).unwrap().child_at(offset).unwrap() {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(_) => panic!("expected a char leaf at offset {offset}"),
    }
}

fn root_block_count(state: &editor_state::State) -> usize {
    state.view().root().unwrap().child_blocks().count()
}

#[test]
fn paragraph_split_merge_scenario() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello ") [bold]
                    text("beautiful world")
                }
            }
        }
        selection: (p1, 0)
    };

    assert_eq!(block_text(&state, &p1), "Hello beautiful world");

    // Split the paragraph after "Hello " (offset 6).
    let split = Step::SplitNode {
        block: p1,
        offset: 6,
    };
    let state2 = split.apply(&state).unwrap().state;
    assert_eq!(root_block_count(&state2), 2);
    let view = state2.view();
    let blocks: Vec<_> = view.root().unwrap().child_blocks().collect();
    assert_eq!(blocks[0].inline_text(), "Hello ");
    assert_eq!(blocks[1].inline_text(), "beautiful world");

    // Inverse (MergeNode survivor=p1, offset=6) restores the single paragraph.
    let state3 = split.inverse().apply(&state2).unwrap().state;
    assert_eq!(root_block_count(&state3), 1);
    assert_eq!(block_text(&state3, &p1), "Hello beautiful world");
}

#[test]
fn insert_text_then_bold_scenario() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello World")
                }
            }
        }
        selection: (p1, 5)
    };

    // Insert " amazing" at end of the block.
    let insert = Step::InsertText {
        block: p1,
        offset: 11,
        text: " amazing".to_string(),
    };
    let state2 = insert.apply(&state).unwrap().state;
    assert_eq!(block_text(&state2, &p1), "Hello World amazing");

    // Bold the block.
    let bold = Step::AddModifier {
        block: p1,
        modifier: Modifier::Bold,
    };
    let state3 = bold.apply(&state2).unwrap().state;
    assert_eq!(
        state3
            .view()
            .node(p1)
            .unwrap()
            .block_modifier(ModifierType::Bold),
        Some(&Modifier::Bold)
    );

    // Undo both.
    let state4 = bold.inverse().apply(&state3).unwrap().state;
    let state5 = insert.inverse().apply(&state4).unwrap().state;

    assert_eq!(block_text(&state5, &p1), "Hello World");
    assert_eq!(
        state5
            .view()
            .node(p1)
            .unwrap()
            .block_modifier(ModifierType::Bold),
        None
    );
}

#[test]
fn set_node_and_selection_combined() {
    let (state, c1, p1) = state! {
        doc {
            root {
                c1: callout {
                    p1: paragraph {
                        text("Hello World")
                    }
                }
            }
        }
        selection: (p1, 0)
    };

    let set_node = Step::SetNode {
        block: c1,
        old_node: PlainNode::Callout(PlainCalloutNode::default()),
        new_node: PlainNode::Callout(PlainCalloutNode {
            variant: CalloutVariant::Warning,
        }),
    };
    let state2 = set_node.apply(&state).unwrap().state;

    let new_sel = Selection::collapsed(Position::new(p1, 5));
    let set_sel = Step::SetSelection {
        old: state2.selection,
        new: Some(new_sel),
    };
    let state3 = set_sel.apply(&state2).unwrap().state;
    assert_eq!(state3.selection, Some(new_sel));

    let state4 = set_sel.inverse().apply(&state3).unwrap().state;
    let state5 = set_node.inverse().apply(&state4).unwrap().state;

    assert_eq!(state5.selection, state.selection);
    if let Node::Callout(n) = state5.view().node(c1).unwrap().node() {
        assert_eq!(*n.variant.get(), CalloutVariant::Info);
    } else {
        panic!("expected Callout node");
    }
}

#[test]
fn subtree_capture_emit_preserves_inline_runs() {
    use editor_model::{EditOp, SpanOp};

    let (state, p1, _p2) = state! {
        doc {
            root {
                p1: paragraph {
                    text("AB")
                    text("CD") [bold]
                }
                p2: paragraph { text("x") }
            }
        }
        selection: (p1, 0)
    };

    let root = state.view().root().unwrap().id();
    let subtree = editor_transaction::capture_subtree(&state.projected, p1).unwrap();
    let remove = Step::RemoveSubtree {
        parent: root,
        index: 0,
        subtree,
    };
    let removed = remove.apply(&state).unwrap().state;
    assert_eq!(
        root_block_count(&removed),
        1,
        "the captured paragraph is removed"
    );

    let out = remove.inverse().apply(&removed).unwrap();
    let restored = out.state;
    let view = restored.view();
    let para = view.root().unwrap().child_blocks().next().unwrap();
    assert_eq!(para.inline_text(), "ABCD");
    let own_at = |slot: usize| para.leaf_own_modifiers_at(slot);
    assert!(
        own_at(0).is_empty(),
        "'A' stays plain across the round-trip"
    );
    assert!(
        own_at(1).is_empty(),
        "'B' stays plain across the round-trip"
    );
    assert!(
        own_at(2).contains(&Modifier::Bold),
        "'C' keeps its bold run across the round-trip"
    );
    assert!(
        own_at(3).contains(&Modifier::Bold),
        "'D' keeps its bold run across the round-trip"
    );

    for op in &out.ops {
        if let EditOp::Span(SpanOp::AddSpan { modifier, .. }) = &op.payload {
            assert!(
                modifier.as_type().is_text_applicable(),
                "emit recorded a span carrying a non-text modifier: {modifier:?}"
            );
        }
    }
}

#[test]
fn inline_span_modifier_bolds_range_and_inverts() {
    let (state, p1) = state! {
        doc {
            root {
                p1: paragraph {
                    text("Hello World")
                }
            }
        }
        selection: (p1, 0)
    };

    // Bold "World" (offsets 6..=10).
    let first = char_dot(&state, &p1, 6);
    let last = char_dot(&state, &p1, 10);
    let bold = Step::AddSpanModifier {
        first,
        last,
        modifier: Modifier::Bold,
    };
    let state2 = bold.apply(&state).unwrap().state;

    let view = state2.view();
    let bold_at = |off: usize| -> bool {
        let d = char_dot(&state2, &p1, off);
        view.leaf_state_by_dot_slow(d)
            .unwrap()
            .eff
            .get(&ModifierType::Bold)
            == Some(&Modifier::Bold)
    };
    assert!(!bold_at(0), "'H' is outside the span");
    assert!(bold_at(6), "'W' is inside the span");
    assert!(bold_at(10), "'d' is the last char of the span");

    // Inverse removes the span.
    let state3 = bold.inverse().apply(&state2).unwrap().state;
    let view3 = state3.view();
    let d6 = char_dot(&state3, &p1, 6);
    assert_eq!(
        view3
            .leaf_state_by_dot_slow(d6)
            .unwrap()
            .eff
            .get(&ModifierType::Bold),
        None,
        "inverse of AddSpanModifier removes the span"
    );
}

use editor_macros::state;
use editor_model::*;
use editor_state::*;
use editor_transaction::test_utils::DocTestExt;
use editor_transaction::*;

#[test]
fn paragraph_split_scenario() {
    let (state, p1, _t1, t2, _t3) = state! {
        doc {
            root {
                p1: paragraph {
                    t1: text("Hello ") [bold]
                    t2: text("beautiful ") [italic]
                    t3: text("world")
                }
            }
        }
        selection: (t2, 4)
    };

    let t_new = NodeId::new();
    let p_new = NodeId::new();

    // Step 1: Split "beautiful " at offset 4 -> "beau" | "tiful "
    let split_text = Step::SplitNode {
        node_id: t2,
        offset: 4,
        new_node_id: t_new,
    };
    let state2 = split_text.apply(&state).unwrap().state;

    // Verify text split
    assert_eq!(state2.text(t2).text, "beau");
    assert_eq!(state2.text(t_new).text, "tiful ");

    // Step 2: Split paragraph — t_new and t3 move to new paragraph
    let split_para = Step::SplitNode {
        node_id: p1,
        offset: 2,
        new_node_id: p_new,
    };
    let state3 = split_para.apply(&state2).unwrap().state;

    // Verify structure: root has 2 paragraphs now
    assert_eq!(state3.node(NodeId::ROOT).children().len(), 2);

    // Undo: reverse order
    let state4 = split_para.inverse().apply(&state3).unwrap().state;
    let state5 = split_text.inverse().apply(&state4).unwrap().state;

    // Verify fully restored
    assert_eq!(state5.node(NodeId::ROOT).children().len(), 1);
    assert!(!state5.has_node(t_new));
    assert!(!state5.has_node(p_new));
    assert_eq!(state5.node(p1).children().len(), 3);
}

#[test]
fn insert_text_then_bold_scenario() {
    let (state, t1) = state! {
        doc {
            root {
                paragraph {
                    t1: text("Hello World")
                }
            }
        }
        selection: (t1, 5)
    };

    // Insert " amazing" at end of t1
    let insert = Step::InsertText {
        node_id: t1,
        offset: 11,
        text: " amazing".to_string(),
    };
    let state2 = insert.apply(&state).unwrap().state;
    assert_eq!(state2.text(t1).text, "Hello World amazing");

    // Bold the whole node
    let bold = Step::AddModifier {
        node_id: t1,
        modifier: Modifier::Bold,
    };
    let state3 = bold.apply(&state2).unwrap().state;
    assert_eq!(
        state3.doc.get_entry(t1).unwrap().modifiers,
        vec![Modifier::Bold]
    );

    // Undo both
    let state4 = bold.inverse().apply(&state3).unwrap().state;
    let state5 = insert.inverse().apply(&state4).unwrap().state;

    assert_eq!(state5.text(t1).text, "Hello World");
    assert!(state5.doc.get_entry(t1).unwrap().modifiers.is_empty());
}

#[test]
fn set_node_and_selection_combined() {
    let (state, p1, t1) = state! {
        doc {
            root {
                p1: paragraph {
                    t1: text("Hello World")
                }
            }
        }
        selection: (t1, 0)
    };

    // Change paragraph alignment
    let set_node = Step::SetNode {
        node_id: p1,
        old_node: Node::Paragraph(ParagraphNode::default()),
        new_node: Node::Paragraph(ParagraphNode {
            align: TextAlign::Center,
        }),
    };
    let state2 = set_node.apply(&state).unwrap().state;

    // Move selection
    let set_sel = Step::SetSelection {
        old: state2.selection.clone(),
        new: Selection::collapsed(Position::new(t1, 5)),
    };
    let state3 = set_sel.apply(&state2).unwrap().state;

    // Undo both
    let state4 = set_sel.inverse().apply(&state3).unwrap().state;
    let state5 = set_node.inverse().apply(&state4).unwrap().state;

    assert_eq!(state5.selection, state.selection);
    assert_eq!(
        *state5.node(p1).node(),
        Node::Paragraph(ParagraphNode::default())
    );
}

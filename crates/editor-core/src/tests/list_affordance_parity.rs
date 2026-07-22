use editor_commands as commands;
use editor_macros::state;
use editor_model::NodeType;
use editor_state::{Position, Selection, State, flat_size};

use crate::editor::Editor;
use crate::message::*;

fn fixtures() -> Vec<State> {
    let mut out = Vec::new();
    {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("plain one") }
                bullet_list {
                    list_item { paragraph { text("alpha") } }
                    list_item {
                        paragraph { text("beta") }
                        bullet_list { list_item { paragraph { text("nested") } } }
                    }
                }
                paragraph { text("plain two") }
            } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { p1: paragraph { text("one") } }
                    list_item { paragraph { text("two") } }
                }
                bullet_list { list_item { paragraph { text("three") } } }
            } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { text("quoted") } }
                bullet_list { list_item { p1: paragraph { text("item") } } }
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("inside") } }
                }
            } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("only first") } } }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("Hello world. Second sentence here. Third one!") }
                paragraph { text("다른 문단의 텍스트") }
            } }
            selection: (p1, 3)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        out.push(s);
    }
    {
        let (s, ..) = state! {
            doc { r: root { image image } }
            selection: (r, 0, <)
        };
        out.push(s);
    }
    out
}

fn assert_parity(editor: &mut Editor) {
    let Some(selection) = editor.state().selection else {
        return;
    };
    let cases = [
        (
            Message::List {
                op: ListOp::ToggleKind {
                    kind: ListKind::Bullet,
                },
            },
            {
                let view = editor.state().view();
                commands::judge_toggle_list_kind(&view, &selection, NodeType::BulletList).changes()
            },
        ),
        (
            Message::List {
                op: ListOp::ToggleKind {
                    kind: ListKind::Ordered,
                },
            },
            {
                let view = editor.state().view();
                commands::judge_toggle_list_kind(&view, &selection, NodeType::OrderedList).changes()
            },
        ),
        (Message::List { op: ListOp::Indent }, {
            let view = editor.state().view();
            commands::judge_indent_list(&view, &selection).changes()
        }),
        (
            Message::List {
                op: ListOp::Outdent,
            },
            {
                let view = editor.state().view();
                commands::judge_outdent_list(&view, &selection).changes()
            },
        ),
    ];
    for (msg, judged) in cases {
        let executed = {
            let mut scratch = Editor::new_test(editor.state().clone());
            crate::test_utils::apply_and_report_change(&mut scratch, msg.clone())
        };
        assert_eq!(
            judged, executed,
            "judgment must match execution for {msg:?}"
        );
    }
}

fn assert_expand_parity(editor: &mut Editor) {
    let selection = editor.state().selection;
    for msg_unit in [
        SelectionExpansionUnit::Word,
        SelectionExpansionUnit::Sentence,
        SelectionExpansionUnit::Paragraph,
        SelectionExpansionUnit::All,
    ] {
        let judged = {
            let resource = editor.resource().lock().unwrap();
            let view = editor.state().view();
            match msg_unit {
                SelectionExpansionUnit::Word => {
                    commands::judge_expand_word(&view, selection, &resource).changes()
                }
                SelectionExpansionUnit::Sentence => {
                    commands::judge_expand_sentence(&view, selection, &resource).changes()
                }
                SelectionExpansionUnit::Paragraph => {
                    commands::judge_expand_paragraph(&view, selection).changes()
                }
                SelectionExpansionUnit::All => {
                    commands::judge_expand_all(&view, selection).changes()
                }
            }
        };
        let expand_msg = Message::Selection {
            op: SelectionOp::Expand { unit: msg_unit },
        };
        let executed = {
            let mut scratch = Editor::new_test(editor.state().clone());
            crate::test_utils::apply_and_report_change(&mut scratch, expand_msg.clone())
        };
        assert_eq!(
            judged, executed,
            "expand judgment must match execution for {msg_unit:?}"
        );
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
    #[test]
    fn judgment_matches_execution(fixture_idx in 0usize..7, start in 0usize..64, end in 0usize..64) {
        let state = fixtures().swap_remove(fixture_idx);
        let size = flat_size(&state.view());
        let start = start.min(size);
        let end = end.min(size);
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection { op: SelectionOp::SetFlat { start, end } });
        assert_parity(&mut editor);
        assert_expand_parity(&mut editor);
    }
}

#[test]
fn judgment_matches_execution_for_gap_cursor() {
    let (state, ..) = state! {
        doc { r: root { image paragraph { text("b") } } }
        selection: (r, 0, <)
    };
    let mut editor = Editor::new_test(state);
    assert_parity(&mut editor);
    assert_expand_parity(&mut editor);
}

#[test]
fn judgment_matches_execution_for_unit_selection() {
    let (state, ..) = state! {
        doc { r: root {
            img: image
            paragraph { text("ab") }
        } }
        selection: (r, 0, >) -> (r, 1, <)
    };
    let mut editor = Editor::new_test(state);
    assert_parity(&mut editor);
    assert_expand_parity(&mut editor);
}

#[test]
fn judgment_matches_execution_for_synthetic_selection() {
    let (mut state, ..) = state! {
        doc { root { fold paragraph {} } }
        selection: none
    };
    let (title, body) = {
        let view = state.view();
        let fold = view
            .root()
            .unwrap()
            .child_blocks()
            .find(|block| block.node_type() == NodeType::Fold)
            .unwrap();
        let title = fold
            .child_blocks()
            .find(|block| block.node_type() == NodeType::FoldTitle)
            .unwrap();
        let content = fold
            .child_blocks()
            .find(|block| block.node_type() == NodeType::FoldContent)
            .unwrap();
        let body = content.child_blocks().next().unwrap();
        (title.id(), body.id())
    };
    state.selection = Some(Selection::new(
        Position::new(body, 0),
        Position::new(title, 0),
    ));
    let mut editor = Editor::new_test(state);
    assert_parity(&mut editor);
    assert_expand_parity(&mut editor);
}

#[test]
fn judgment_matches_execution_for_synthetic_endpoint_absorb_only() {
    let state = fixtures().swap_remove(1);
    let size = flat_size(&state.view());
    let start = 28usize.min(size);
    let end = 14usize.min(size);
    let mut editor = Editor::new_test(state);
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start, end },
    });
    assert_parity(&mut editor);
    assert_expand_parity(&mut editor);
}

#[test]
fn judgment_matches_execution_for_root_endpoint_absorb_only() {
    let state = fixtures().swap_remove(0);
    let size = flat_size(&state.view());
    let start = 53usize.min(size);
    let end = 0usize.min(size);
    let mut editor = Editor::new_test(state);
    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start, end },
    });
    assert_parity(&mut editor);
    assert_expand_parity(&mut editor);
}

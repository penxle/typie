use editor_crdt::{Changeset, Dot, ListOp};
use editor_macros::state;
use editor_model::{
    BlockquoteVariant, EditOp, Fragment, Modifier, ModifierType, PlainBlockquoteNode,
    PlainHorizontalRuleNode, PlainNode, PlainParagraphNode, PlainTextNode, SeqItem,
};
use editor_state::State;
use hashbrown::HashSet;
use proptest::prelude::*;

use crate::editor::Editor;
use crate::message::*;

#[derive(Debug, Clone)]
enum EditAction {
    InsertChar { at: usize, ch: char },
    DeleteBackward { at: usize },
    DeleteRange { at: usize, len: usize },
    ParagraphBreak { at: usize },
    InsertHrBlock { at: usize },
    InsertMessageBlockquote { at: usize },
    SetInlineModifier { at: usize, len: usize, which: u8 },
    SetBlockModifier { at: usize, value: u32 },
    SetRootModifier { value: u32 },
    SetTableColumnWidths { w0: f32, w1: f32 },
    Undo,
    Redo,
    RemoteMerge { at: usize, ch: char },
}

fn ascii(n: u32) -> char {
    char::from(b'a' + (n % 26) as u8)
}

fn edit_action() -> impl Strategy<Value = EditAction> {
    prop_oneof![
        (any::<u16>(), 0u32..26).prop_map(|(at, n)| EditAction::InsertChar {
            at: at as usize,
            ch: ascii(n),
        }),
        any::<u16>().prop_map(|at| EditAction::DeleteBackward { at: at as usize }),
        (any::<u16>(), 0usize..8).prop_map(|(at, len)| EditAction::DeleteRange {
            at: at as usize,
            len,
        }),
        any::<u16>().prop_map(|at| EditAction::ParagraphBreak { at: at as usize }),
        any::<u16>().prop_map(|at| EditAction::InsertHrBlock { at: at as usize }),
        any::<u16>().prop_map(|at| EditAction::InsertMessageBlockquote { at: at as usize }),
        (any::<u16>(), 0usize..8, 0u8..3).prop_map(|(at, len, which)| {
            EditAction::SetInlineModifier {
                at: at as usize,
                len,
                which,
            }
        }),
        (any::<u16>(), 80u32..320).prop_map(|(at, value)| EditAction::SetBlockModifier {
            at: at as usize,
            value,
        }),
        (800u32..4000).prop_map(|value| EditAction::SetRootModifier { value }),
        (20.0f32..400.0, 20.0f32..400.0)
            .prop_map(|(w0, w1)| EditAction::SetTableColumnWidths { w0, w1 }),
        Just(EditAction::Undo),
        Just(EditAction::Redo),
        (any::<u16>(), 0u32..26).prop_map(|(at, n)| EditAction::RemoteMerge {
            at: at as usize,
            ch: ascii(n)
        }),
    ]
}

fn edit_op_sequence() -> impl Strategy<Value = Vec<EditAction>> {
    prop::collection::vec(edit_action(), 1..16)
}

fn build_test_editor() -> (Editor, Dot) {
    let (state, _p0, tbl) = state! {
        doc { root {
            p0: paragraph { text("alpha") }
            blockquote { paragraph { text("quote") } }
            tbl: table {
                table_row {
                    table_cell { paragraph { text("a") } }
                    table_cell { paragraph { text("b") } }
                }
                table_row {
                    table_cell { paragraph { text("c") } }
                    table_cell { paragraph { text("d") } }
                }
            }
            paragraph {}
            paragraph { text("beta") }
        } }
        selection: (p0, 0)
    };
    (Editor::new_test(state), tbl)
}

fn flat_n(editor: &Editor) -> usize {
    let view = editor.state().view();
    editor_state::flat_size(&view)
}

fn sel_flat(start: usize, end: usize) -> Message {
    Message::Selection {
        op: SelectionOp::SetFlat { start, end },
    }
}

fn hr_fragment() -> Fragment {
    Fragment::leaf(PlainNode::HorizontalRule(PlainHorizontalRuleNode::default()))
}

fn message_blockquote_fragment() -> Fragment {
    Fragment {
        node: PlainNode::Blockquote(PlainBlockquoteNode {
            variant: BlockquoteVariant::MessageSent,
        }),
        modifiers: vec![],
        children: vec![Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: "msg".into(),
            }))],
        }],
    }
}

fn remote_insert_changeset(state: &State, ch: char) -> Option<Changeset<EditOp>> {
    let mut pa = state.projected.as_ref().clone();
    let baseline: HashSet<Dot> = pa.graph().current_heads().copied().collect();
    pa.apply_batch(vec![EditOp::Seq(ListOp::Ins {
        pos: 1,
        item: SeqItem::Char(ch),
    })])
    .ok()?;
    pa.commit();
    let mut css = pa.graph().local_changesets_since(&baseline).ok()?;
    if css.is_empty() {
        None
    } else {
        Some(css.remove(0))
    }
}

fn apply_op(editor: &mut Editor, op: &EditAction, table: Dot) {
    let n = flat_n(editor);
    match op {
        EditAction::InsertChar { at, ch } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Insertion {
                op: InsertionOp::Text {
                    text: ch.to_string(),
                },
            });
        }
        EditAction::DeleteBackward { at } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Deletion {
                op: DeletionOp::Surrounding {
                    before: 1,
                    after: 0,
                },
            });
        }
        EditAction::DeleteRange { at, len } => {
            let s = at % (n + 1);
            let e = (s + len).min(n);
            editor.enqueue(sel_flat(s, e));
            editor.enqueue(Message::Deletion {
                op: DeletionOp::Selection,
            });
        }
        EditAction::ParagraphBreak { at } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Insertion {
                op: InsertionOp::Break {
                    kind: Break::Paragraph,
                },
            });
        }
        EditAction::InsertHrBlock { at } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Insertion {
                op: InsertionOp::Fragment {
                    fragment: hr_fragment(),
                },
            });
        }
        EditAction::InsertMessageBlockquote { at } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Insertion {
                op: InsertionOp::Fragment {
                    fragment: message_blockquote_fragment(),
                },
            });
        }
        EditAction::SetInlineModifier { at, len, which } => {
            let s = at % (n + 1);
            let e = (s + len).min(n);
            editor.enqueue(sel_flat(s, e));
            match which % 3 {
                0 => editor.enqueue(Message::Modifier {
                    op: ModifierOp::Toggle {
                        modifier_type: ModifierType::Bold,
                    },
                }),
                1 => editor.enqueue(Message::Modifier {
                    op: ModifierOp::Toggle {
                        modifier_type: ModifierType::Italic,
                    },
                }),
                _ => editor.enqueue(Message::Modifier {
                    op: ModifierOp::Set {
                        modifier: Modifier::FontSize { value: 2000 },
                    },
                }),
            }
        }
        EditAction::SetBlockModifier { at, value } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            editor.enqueue(Message::Modifier {
                op: ModifierOp::Set {
                    modifier: Modifier::LineHeight { value: *value },
                },
            });
        }
        EditAction::SetRootModifier { value } => {
            editor.enqueue(Message::Modifier {
                op: ModifierOp::SetOnNode {
                    id: Dot::ROOT,
                    modifier: Modifier::FontSize { value: *value },
                },
            });
        }
        EditAction::SetTableColumnWidths { w0, w1 } => {
            editor.enqueue(Message::Node {
                op: NodeOp::Table {
                    id: table,
                    op: TableOp::SetColumnWidths {
                        widths: vec![*w0, *w1],
                    },
                },
            });
        }
        EditAction::Undo => editor.enqueue(Message::History {
            op: HistoryOp::Undo,
        }),
        EditAction::Redo => editor.enqueue(Message::History {
            op: HistoryOp::Redo,
        }),
        EditAction::RemoteMerge { at, ch } => {
            let p = at % (n + 1);
            editor.enqueue(sel_flat(p, p));
            if let Some(changeset) = remote_insert_changeset(editor.state(), *ch) {
                editor.enqueue(Message::Remote { changeset });
            }
            editor.enqueue(Message::Insertion {
                op: InsertionOp::Text {
                    text: ch.to_string(),
                },
            });
        }
    }
    let _ = editor.tick();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn incremental_matches_full_after_random_edits(ops in edit_op_sequence()) {
        let (mut editor, tbl) = build_test_editor();
        for op in &ops {
            apply_op(&mut editor, op, tbl);
        }
    }
}

use editor_commands::{self as commands, CommandError, CommandResult};
use editor_schema::{DocFlatExt, FLAT_CLOSE, FLAT_OPEN, ResolvedPositionFlatExt};
use editor_state::{Composition, ResolvedPosition, Selection};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::FlatImeOp;

fn is_token(c: char) -> bool {
    c == FLAT_OPEN || c == FLAT_CLOSE
}

fn utf16_units_to_chars(iter: impl Iterator<Item = char>, utf16_units: usize) -> usize {
    let mut remaining = utf16_units;
    let mut count = 0;
    for c in iter {
        if remaining == 0 {
            break;
        }
        remaining = remaining.saturating_sub(c.len_utf16());
        count += 1;
    }
    count
}

#[derive(Debug, Clone)]
struct FlatImeState {
    text: Vec<char>,
    sel_start: usize,
    sel_end: usize,
    comp: Option<(usize, usize)>,
}

impl FlatImeState {
    fn from_editor(editor: &Editor) -> Option<Self> {
        let state = editor.state();
        let doc = &state.doc;
        let flat_size = doc.flat_size();
        let text: Vec<char> = doc.flat_text(0..flat_size).chars().collect();

        let anchor = state.selection.anchor.resolve(doc)?.to_flat();
        let head = state.selection.head.resolve(doc)?.to_flat();

        let comp = state.composition.map(|c| (c.start, c.end));

        Some(FlatImeState {
            text,
            sel_start: anchor.min(head),
            sel_end: anchor.max(head),
            comp,
        })
    }

    fn apply(&mut self, op: &FlatImeOp) {
        match op {
            FlatImeOp::SetSelection { start, end } => {
                self.sel_start = (*start).min(self.text.len());
                self.sel_end = (*end).min(self.text.len());
            }
            FlatImeOp::ReplaceSelection { text } => {
                let chars: Vec<char> = text.chars().collect();
                let start = self.sel_start.min(self.text.len());
                let end = self.sel_end.min(self.text.len());
                self.text.splice(start..end, chars.iter().copied());
                let new_pos = start + chars.len();
                self.sel_start = new_pos;
                self.sel_end = new_pos;
                self.comp = None;
            }
            FlatImeOp::Compose { text } => {
                let chars: Vec<char> = text.chars().collect();
                let (start, end) = self.comp.unwrap_or((self.sel_start, self.sel_end));
                let start = start.min(self.text.len());
                let end = end.min(self.text.len());
                self.text.splice(start..end, chars.iter().copied());
                let new_end = start + chars.len();
                self.sel_start = new_end;
                self.sel_end = new_end;
                self.comp = Some((start, new_end));
            }
            FlatImeOp::DeleteSurrounding { before, after } => {
                let cursor = self.sel_start.min(self.text.len());
                let del_start = cursor.saturating_sub(*before);
                let del_end = (cursor + after).min(self.text.len());
                if del_end > cursor {
                    self.text.splice(cursor..del_end, std::iter::empty());
                }
                if del_start < cursor {
                    self.text.splice(del_start..cursor, std::iter::empty());
                }
                self.sel_start = del_start;
                self.sel_end = del_start;
            }
            FlatImeOp::DeleteSurroundingUtf16 { before, after } => {
                let cursor = self.sel_start.min(self.text.len());
                let before_chars =
                    utf16_units_to_chars(self.text[..cursor].iter().rev().copied(), *before);
                let after_chars = utf16_units_to_chars(self.text[cursor..].iter().copied(), *after);
                let del_start = cursor - before_chars;
                let del_end = cursor + after_chars;
                if del_end > cursor {
                    self.text.splice(cursor..del_end, std::iter::empty());
                }
                if del_start < cursor {
                    self.text.splice(del_start..cursor, std::iter::empty());
                }
                self.sel_start = del_start;
                self.sel_end = del_start;
            }
            FlatImeOp::SetComposition { start, end } => {
                self.comp = Some((*start, *end));
            }
            FlatImeOp::ClearComposition => {
                self.comp = None;
            }
            FlatImeOp::MoveCursor { delta } => {
                let pos = if *delta >= 0 {
                    self.sel_end.saturating_add(*delta as usize)
                } else {
                    self.sel_start.saturating_sub(delta.unsigned_abs() as usize)
                }
                .min(self.text.len());
                self.sel_start = pos;
                self.sel_end = pos;
            }
        }
    }

    fn reduce(mut self, ops: &[FlatImeOp]) -> Self {
        for op in ops {
            self.apply(op);
        }
        self
    }
}

fn common_prefix_len(a: &[char], b: &[char]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

fn common_suffix_len(a: &[char], b: &[char]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

fn count_opens(chars: &[char]) -> usize {
    chars.iter().filter(|&&c| c == FLAT_OPEN).count()
}

fn count_closes(chars: &[char]) -> usize {
    chars.iter().filter(|&&c| c == FLAT_CLOSE).count()
}

fn handle_text_delta(
    tr: &mut Transaction,
    del_start: usize,
    del_end: usize,
    text: &str,
) -> CommandResult {
    let doc = tr.doc();
    let start_pos = ResolvedPosition::from_flat(&doc, del_start)
        .ok_or(CommandError::Corrupted("flat start unresolvable".into()))?;
    let end_pos = ResolvedPosition::from_flat(&doc, del_end)
        .ok_or(CommandError::Corrupted("flat end unresolvable".into()))?;

    commands::chain!(
        tr,
        commands::set_selection(Selection::new((&start_pos).into(), (&end_pos).into())),
        commands::when!(del_start != del_end, commands::delete_selection()),
        commands::when!(!text.is_empty(), commands::insert_text(text)),
    )
}

fn structural_backward(tr: &mut Transaction) -> CommandResult {
    commands::first!(
        tr,
        commands::join_paragraph_backward(),
        commands::sink_paragraph_backward(),
        commands::lift_first_paragraph(),
    )
}

fn structural_forward(tr: &mut Transaction) -> CommandResult {
    commands::first!(
        tr,
        commands::join_paragraph_forward(),
        commands::lift_last_paragraph(),
        commands::lift_paragraph_forward(),
    )
}

struct FlatDelta {
    start_tokens: usize,
    text_start: usize,
    text_end: usize,
    end_tokens: usize,
    ins_text: String,
}

fn analyze_delta(
    chars: &[char],
    del_start: usize,
    del_end: usize,
    ins: &[char],
    cursor: usize,
) -> FlatDelta {
    let del = &chars[del_start..del_end];

    let first_text = del.iter().position(|c| !is_token(*c));
    let last_text = del.iter().rposition(|c| !is_token(*c));

    let (text_start, text_end, left_tokens, right_tokens) = match (first_text, last_text) {
        (Some(first), Some(last)) => {
            let left = &del[..first];
            let right = &del[last + 1..];
            (del_start + first, del_start + last + 1, left, right)
        }
        _ => {
            let cursor_offset = cursor.clamp(del_start, del_end) - del_start;
            let left = &del[..cursor_offset];
            let right = &del[cursor_offset..];
            (
                del_start + cursor_offset,
                del_start + cursor_offset,
                left,
                right,
            )
        }
    };

    let backward_count = count_opens(left_tokens).max(count_closes(left_tokens));
    let forward_count = count_opens(right_tokens).max(count_closes(right_tokens));

    FlatDelta {
        start_tokens: backward_count,
        text_start,
        text_end,
        end_tokens: forward_count,
        ins_text: ins.iter().collect(),
    }
}

pub fn handle_flat_ime(editor: &mut Editor, ops: Vec<FlatImeOp>) -> Result<(), EditorError> {
    let initial = match FlatImeState::from_editor(editor) {
        Some(s) => s,
        None => return Ok(()),
    };

    let result = initial.clone().reduce(&ops);

    let prefix = common_prefix_len(&initial.text, &result.text);
    let suffix = common_suffix_len(&initial.text[prefix..], &result.text[prefix..]);

    let del = &initial.text[prefix..initial.text.len() - suffix];
    let ins = &result.text[prefix..result.text.len() - suffix];

    let del_opens = count_opens(del);
    let del_closes = count_closes(del);
    let ins_opens = count_opens(ins);
    let ins_closes = count_closes(ins);

    let tokens_increased = ins_opens > del_opens || ins_closes > del_closes;
    if tokens_increased {
        return Ok(());
    }

    if ins.iter().any(|c| is_token(*c)) {
        return Ok(());
    }

    let del_end = initial.text.len() - suffix;
    let delta = analyze_delta(&initial.text, prefix, del_end, ins, initial.sel_start);

    if delta.start_tokens == 0 && delta.end_tokens == 0 {
        if !del.is_empty() || !ins.is_empty() {
            editor.transact(|tr| {
                handle_text_delta(tr, prefix, del_end, &delta.ins_text)?;
                Ok(())
            })?;
        }

        if let Some((start, end)) = result.comp {
            editor.transact(|tr| {
                tr.set_composition(Some(Composition { start, end }))?;
                Ok(())
            })?;
        } else if editor.state().composition.is_some() {
            editor.transact(|tr| {
                tr.set_composition(None)?;
                Ok(())
            })?;
        }
    } else {
        editor.transact(|tr| {
            if delta.end_tokens > 0 {
                let doc = tr.doc();
                if let Some(pos) = ResolvedPosition::from_flat(&doc, delta.text_end) {
                    commands::set_selection(tr, Selection::collapsed((&pos).into()))?;
                }
                for _ in 0..delta.end_tokens {
                    structural_forward(tr)?;
                }
            }

            if delta.text_start != delta.text_end || !delta.ins_text.is_empty() {
                handle_text_delta(tr, delta.text_start, delta.text_end, &delta.ins_text)?;
            }

            if delta.start_tokens > 0 {
                let doc = tr.doc();
                if let Some(pos) = ResolvedPosition::from_flat(&doc, delta.text_start) {
                    commands::set_selection(tr, Selection::collapsed((&pos).into()))?;
                }
                for _ in 0..delta.start_tokens {
                    structural_backward(tr)?;
                }
            }

            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(text: &str, sel: usize) -> FlatImeState {
        FlatImeState {
            text: text.chars().collect(),
            sel_start: sel,
            sel_end: sel,
            comp: None,
        }
    }

    fn state_sel(text: &str, sel_start: usize, sel_end: usize) -> FlatImeState {
        FlatImeState {
            text: text.chars().collect(),
            sel_start,
            sel_end,
            comp: None,
        }
    }

    fn text(s: &FlatImeState) -> String {
        s.text.iter().collect()
    }

    #[test]
    fn replace_selection_inserts_text() {
        let s = state("hello", 5);
        let s = s.reduce(&[FlatImeOp::ReplaceSelection { text: "!".into() }]);
        assert_eq!(text(&s), "hello!");
        assert_eq!(s.sel_start, 6);
    }

    #[test]
    fn replace_selection_replaces_range() {
        let s = state_sel("hello", 1, 4);
        let s = s.reduce(&[FlatImeOp::ReplaceSelection { text: "X".into() }]);
        assert_eq!(text(&s), "hXo");
        assert_eq!(s.sel_start, 2);
    }

    #[test]
    fn delete_surrounding_backward() {
        let s = state("hello", 3);
        let s = s.reduce(&[FlatImeOp::DeleteSurrounding {
            before: 2,
            after: 0,
        }]);
        assert_eq!(text(&s), "hlo");
        assert_eq!(s.sel_start, 1);
    }

    #[test]
    fn compose_without_existing_composition() {
        let s = state("hello", 3);
        let s = s.reduce(&[FlatImeOp::Compose { text: "X".into() }]);
        assert_eq!(text(&s), "helXlo");
        assert_eq!(s.comp, Some((3, 4)));
    }

    #[test]
    fn compose_replaces_existing_composition() {
        let mut s = state("hello", 3);
        s.comp = Some((1, 4));
        let s = s.reduce(&[FlatImeOp::Compose { text: "XY".into() }]);
        assert_eq!(text(&s), "hXYo");
        assert_eq!(s.comp, Some((1, 3)));
    }

    #[test]
    fn korean_ime_recomposition_batch() {
        let o = FLAT_OPEN;
        let c = FLAT_CLOSE;
        let initial = format!("{o}!ㅇ{c}");
        let s = FlatImeState {
            text: initial.chars().collect(),
            sel_start: 3,
            sel_end: 3,
            comp: None,
        };
        let s = s.reduce(&[
            FlatImeOp::SetSelection { start: 0, end: 3 },
            FlatImeOp::ReplaceSelection { text: "".into() },
            FlatImeOp::ReplaceSelection {
                text: format!("{o}").into(),
            },
            FlatImeOp::ReplaceSelection { text: "아".into() },
        ]);
        assert_eq!(text(&s), format!("{o}아{c}"));
    }

    #[test]
    fn empty_paragraph_backspace_batch() {
        let o = FLAT_OPEN;
        let c = FLAT_CLOSE;
        let initial = format!("{o}{c}");
        let s = FlatImeState {
            text: initial.chars().collect(),
            sel_start: 1,
            sel_end: 1,
            comp: None,
        };
        let s = s.reduce(&[
            FlatImeOp::SetSelection { start: 0, end: 1 },
            FlatImeOp::ReplaceSelection { text: "".into() },
        ]);
        assert_eq!(text(&s), format!("{c}"));
    }

    use editor_macros::state;
    use editor_resource::Resource;
    use editor_state::assert_state_eq;
    use std::sync::{Arc, Mutex};

    use crate::editor::Editor;
    use crate::message::Message;

    fn editor_with_resource(s: editor_state::State) -> Editor {
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        Editor::new_test_with_resource(s, resource)
    }

    #[test]
    fn flat_ime_text_replacement() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![FlatImeOp::ReplaceSelection { text: "!".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello!") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_korean_recomposition_preserves_structure() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("!ㅇ") } } }
            selection: (t1, 2)
        };
        let mut editor = editor_with_resource(s);
        let o = "\u{2028}";
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: o.into() },
                FlatImeOp::ReplaceSelection {
                    text: "!아".into()
                },
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("!아") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_no_change_is_noop() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![FlatImeOp::SetSelection { start: 4, end: 4 }],
        });
        use editor_schema::DocFlatExt;
        assert_eq!(editor.state().doc.flat_text(1..6), "hello");
    }

    #[test]
    fn flat_ime_pua_reinsert_filtered() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 2)
        };
        let mut editor = editor_with_resource(s);
        let o = "\u{2028}";
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: o.into() },
                FlatImeOp::ReplaceSelection { text: "ab".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_delete_surrounding() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 2,
                after: 0,
            }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hlo") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_compose_sets_composition() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helXlo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 4, end: 5 })
        );
    }

    #[test]
    fn flat_ime_bulk_backward_delete_at_boundary_does_structural() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("paragraph1") } paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 1,
                after: 0,
            }],
        });
        let state = editor.state();
        use editor_schema::DocFlatExt;
        let flat = state.doc.flat_text(0..state.doc.flat_size());
        assert!(
            !flat.contains("\u{2028}\u{2029}\u{2029}"),
            "empty paragraph should have been removed"
        );
    }

    #[test]
    fn flat_ime_join_paragraph_backward_cursor_at_end() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("A") } p2: paragraph {} } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 3, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_empty_paragraph_backspace_removes_paragraph() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } p2: paragraph {} } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        use editor_schema::DocFlatExt;
        let original_size = editor.state().doc.flat_size();
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 8 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let new_size = editor.state().doc.flat_size();
        assert!(
            new_size < original_size,
            "empty paragraph should be removed: new_size={new_size} original={original_size}"
        );
    }

    #[test]
    fn flat_ime_input_context_has_tokens() {
        let (s, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("") } } paragraph {} } }
            selection: (t1, 0)
        };
        let editor = Editor::new_test(s);
        let ctx = editor.input_context(100, 100).unwrap();
        assert!(
            !ctx.text.is_empty(),
            "empty blockquote paragraph should have tokens in buffer"
        );
        assert!(
            ctx.text.contains(FLAT_OPEN),
            "buffer should contain Open tokens"
        );
    }

    #[test]
    fn flat_ime_bulk_delete_single_open_token() {
        // flat: Obq=0 Op=1 h=2 e=3 l=4 l=5 o=6 Cp=7 Cbq=8 Op2=9 w=10 o=11 r=12 l=13 d=14 Cp2=15
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 9, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_close_open_pair() {
        // flat: Obq=0 Op=1 h=2 e=3 l=4 l=5 o=6 Cp=7 Cbq=8 Op2=9 w=10 o=11 r=12 l=13 d=14 Cp2=15
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 8, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_two_boundaries() {
        // flat: Obq=0 Op=1 h=2 e=3 l=4 l=5 o=6 Cp=7 Cbq=8 Op2=9 w=10 o=11 r=12 l=13 d=14 Cp2=15
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("helloworld") } }
                paragraph {}
            } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_text_across_structure() {
        // flat: Obq=0 Op=1 h=2 e=3 l=4 l=5 o=6 Cp=7 Cbq=8 Op2=9 w=10 o=11 r=12 l=13 d=14 Cp2=15
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::FlatIme {
            ops: vec![
                FlatImeOp::SetSelection { start: 3, end: 13 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hld") } }
                paragraph {}
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}

use std::collections::BTreeMap;

use editor_commands::CommandError;
use editor_model::DocView;
use editor_state::{ResolvedPosition, ResolvedPositionFlatExt, StablePosition, StableResolveCtx};
use editor_transaction::Transaction;

use super::{FlatImeState, FlatImeTextChange};
use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::FlatImeOp;

// Positions outside an insertion belong to existing document content. Positions
// inside it belong to the text supplied by the IME, whose newlines may create
// more than one structural flat unit. Bind those positions as the handler edits
// the document, instead of predicting the number of structural tokens.
pub(super) enum ImeTextPosition {
    Existing(StablePosition),
    Inserted(usize),
}

impl ImeTextPosition {
    pub(super) fn capture(
        view: &DocView,
        change: &FlatImeTextChange,
        offset: usize,
    ) -> Option<Self> {
        let inserted_end = change.replace_start + change.insert.len();
        if (change.replace_start != change.replace_end || !change.insert.is_empty())
            && (change.replace_start..=inserted_end).contains(&offset)
        {
            return Some(Self::Inserted(offset - change.replace_start));
        }
        let original = if offset < change.replace_start {
            offset
        } else {
            change.replace_end.checked_add(offset - inserted_end)?
        };
        let position = ResolvedPosition::from_flat(view, original)?;
        Some(Self::Existing(StablePosition::capture(
            &(&position).into(),
            view,
        )))
    }

    pub(super) fn resolve(
        &self,
        tr: &Transaction,
        inserted: &BTreeMap<usize, Option<StablePosition>>,
    ) -> Option<usize> {
        let position = match self {
            Self::Existing(position) => position,
            Self::Inserted(offset) => inserted.get(offset)?.as_ref()?,
        };
        let view = tr.view();
        let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
        position
            .resolve(&ctx)?
            .resolve(&view)
            .map(|pos| pos.to_flat())
    }

    pub(super) fn bind(
        self,
        inserted: &BTreeMap<usize, Option<StablePosition>>,
    ) -> Option<StablePosition> {
        match self {
            Self::Existing(position) => Some(position),
            Self::Inserted(offset) => inserted.get(&offset).cloned().flatten(),
        }
    }
}

// Rebase the rest of a native batch after a commit has executed structural
// edits or automatic text replacement. Both buffers advance with each command:
// later positions refer to the text after earlier commands, not to a snapshot
// frozen at the preceding commit boundary.
pub(super) fn remap_ime_tail(
    editor: &Editor,
    mut input: FlatImeState,
    bindings: Vec<Option<StablePosition>>,
    ops: &mut [FlatImeOp],
) -> Result<(), EditorError> {
    let view = editor.state().view();
    let ctx = StableResolveCtx::from_live(&view, editor.state().projected.seq_checkout());
    let mut offsets: Vec<_> = bindings
        .iter()
        .map(|binding| {
            binding
                .as_ref()?
                .resolve(&ctx)?
                .resolve(&view)
                .map(|pos| pos.to_flat())
        })
        .collect();
    if offsets
        .iter()
        .enumerate()
        .all(|(index, offset)| *offset == Some(input.text.base + index))
    {
        return Ok(());
    }
    let mut reach = ops.to_vec();
    reach.push(FlatImeOp::SetSelection {
        start: offsets.iter().flatten().copied().min().unwrap_or(0),
        end: offsets.iter().flatten().copied().max().unwrap_or(0),
    });
    let Some(mut output) = FlatImeState::from_editor(editor, &reach) else {
        return Ok(());
    };

    for op in ops {
        let input_base = input.text.base;
        let map = |offset: usize| {
            offset
                .checked_sub(input_base)
                .and_then(|index| offsets.get(index))
                .copied()
                .flatten()
                .ok_or_else(|| {
                    EditorError::from(CommandError::InvalidArgument(
                        "IME position no longer maps to the edited document".into(),
                    ))
                })
        };
        let mapped = match op {
            FlatImeOp::SetSelection { start, end } => FlatImeOp::SetSelection {
                start: map(*start)?,
                end: map(*end)?,
            },
            FlatImeOp::SetComposition { start, end } => FlatImeOp::SetComposition {
                start: map(*start)?,
                end: map(*end)?,
            },
            FlatImeOp::MoveCursor { .. } => {
                let mut moved = input.clone();
                moved.apply(op);
                let position = map(moved.sel_start)?;
                FlatImeOp::SetSelection {
                    start: position,
                    end: position,
                }
            }
            FlatImeOp::DeleteSurrounding { .. } | FlatImeOp::DeleteSurroundingUtf16 { .. } => {
                let mut deleted = input.clone();
                let change = deleted.apply(op);
                let (before, after) = input.surrounding_delete_bases();
                let (output_before, output_after) = output.surrounding_delete_bases();
                let (start, end) = change.map_or((before, after), |change| {
                    (change.replace_start, change.replace_end)
                });
                FlatImeOp::DeleteSurrounding {
                    before: if start < before {
                        output_before.saturating_sub(map(start)?)
                    } else {
                        0
                    },
                    after: if end > after {
                        map(end)?.saturating_sub(output_after)
                    } else {
                        0
                    },
                }
            }
            _ => op.clone(),
        };
        let input_change = input.apply(op);
        let output_change = output.apply(&mapped);
        if let Some(input_change) = input_change {
            // Several input boundaries can name one document boundary (CRLF,
            // or text collapsed by automatic replacement). Deleting between
            // them still shortens the input buffer even if the document edit
            // is empty, so its entries must be removed from this map.
            let output_change = match output_change {
                Some(change) => change,
                None => FlatImeTextChange::collapsed_at(map(input_change.replace_start)?),
            };
            let output_end = output_change.replace_start + output_change.insert.len();
            for offset in offsets.iter_mut().flatten() {
                *offset = if *offset >= output_change.replace_end {
                    output_end + (*offset - output_change.replace_end)
                } else {
                    (*offset).min(output_change.replace_start)
                };
            }
            offsets.splice(
                input_change.replace_start - input.text.base
                    ..=input_change.replace_end - input.text.base,
                (output_change.replace_start..=output_end).map(Some),
            );
        }
        *op = mapped;
    }
    Ok(())
}

use editor_commands as commands;
use editor_common::HistoryTag;
use editor_model::{ChildView, NodeType};
use editor_resource::Resource;
use editor_state::undo::{UndoEntry, UndoHistory};
use editor_state::{Position, Selection, State, replacement_paint};
use icu_properties::props::GeneralCategoryGroup;

use super::paragraph_break::apply_paragraph_break;
use crate::{Editor, EditorError};

pub(super) fn trailing_input_separator<'a>(text: &'a str, resource: &Resource) -> Option<&'a str> {
    let start = resource
        .segmenters()
        .grapheme
        .as_borrowed()
        .segment_str(text)
        .filter(|&offset| offset < text.len())
        .last()?;
    let separator = &text[start..];
    let first = separator.chars().next()?;
    let category = resource.general_category().as_borrowed().get(first);
    (first.is_whitespace()
        || GeneralCategoryGroup::Punctuation.contains(category)
        || GeneralCategoryGroup::Symbol.contains(category))
    .then_some(separator)
}

pub(super) enum InputSeparator {
    Text(String),
    Tab,
    ParagraphBreak,
    LineBreak,
}

/// Backspace eligibility has a shorter lifetime than undo history, but can
/// survive the separator that finishes the input. Any other edit expires it.
pub(crate) struct AutoReplacement {
    entry: UndoEntry,
    caret: Position,
    separator: Option<(InputSeparator, UndoEntry)>,
}

impl AutoReplacement {
    pub(crate) fn from_history(state: &State, history: &UndoHistory) -> Option<Self> {
        if !matches!(history.last_tag(), Some(HistoryTag::AutoReplacement)) {
            return None;
        }
        let selection = state.selection.filter(Selection::is_collapsed)?;
        Some(Self {
            entry: history.last_entry()?.clone(),
            caret: selection.head,
            separator: None,
        })
    }

    pub(super) fn ends_with_paragraph_break(&self) -> bool {
        matches!(self.separator, Some((InputSeparator::ParagraphBreak, _)))
    }
}

pub(super) fn take_auto_replacement_for_text(
    editor: &mut Editor,
    text: &str,
) -> Option<(AutoReplacement, InputSeparator)> {
    editor.auto_replacement.as_ref()?;
    {
        let resource = editor.resource.lock().unwrap();
        // One input separator, including a multi-codepoint symbol or CRLF.
        // Another separator is a new input and must expire the previous record.
        if trailing_input_separator(text, &resource)? != text {
            return None;
        }
    }
    let separator = match text {
        "\n" | "\r" | "\r\n" => InputSeparator::ParagraphBreak,
        _ => InputSeparator::Text(text.into()),
    };
    take_auto_replacement_for_separator(editor, separator)
}

pub(super) fn take_auto_replacement_for_separator(
    editor: &mut Editor,
    separator: InputSeparator,
) -> Option<(AutoReplacement, InputSeparator)> {
    let replacement = editor.auto_replacement.as_ref()?;
    if replacement.separator.is_some()
        || editor.state.composition.is_some()
        || editor.state.selection != Some(Selection::collapsed(replacement.caret))
    {
        return None;
    }
    Some((editor.auto_replacement.take()?, separator))
}

pub(super) fn finish_auto_replacement_separator(
    editor: &mut Editor,
    replacement: Option<(AutoReplacement, InputSeparator)>,
) {
    let Some((mut replacement, separator)) = replacement else {
        return;
    };
    if editor.auto_replacement.is_some() || editor.state.composition.is_some() {
        return;
    }
    let Some(selection) = editor.state.selection.filter(Selection::is_collapsed) else {
        return;
    };
    let view = editor.state.view();
    let inserted = match &separator {
        InputSeparator::Text(text) => {
            selection.head.node == replacement.caret.node
                && selection.head.offset == replacement.caret.offset + text.chars().count()
        }
        InputSeparator::ParagraphBreak => {
            // Leaving a list or quote moves the original paragraph;
            // splitting it preserves the paragraph before the break.
            selection.head.node != replacement.caret.node
                && selection.head.offset == 0
                && view
                    .node(replacement.caret.node)
                    .is_some_and(|before| replacement.caret.offset == before.child_count())
        }
        InputSeparator::Tab | InputSeparator::LineBreak => {
            let node_type = if matches!(separator, InputSeparator::Tab) {
                NodeType::Tab
            } else {
                NodeType::HardBreak
            };
            selection.head.node == replacement.caret.node
                && selection.head.offset == replacement.caret.offset + 1
                && view.node(replacement.caret.node).is_some_and(|before| {
                    matches!(before.child_at(replacement.caret.offset), Some(ChildView::Leaf(leaf)) if leaf.node_type() == node_type)
                })
        }
    };
    if !inserted {
        return;
    }
    let Some(entry) = editor.undo_history.last_entry() else {
        return;
    };
    // Enter/Tab can navigate or change list structure without inserting
    // a separator. Never treat the replacement's own entry as its suffix.
    if entry.ops.is_empty()
        || entry.ops.last().map(|r| r.op.id) == replacement.entry.ops.last().map(|r| r.op.id)
    {
        return;
    }
    replacement.separator = Some((separator, entry.clone()));
    replacement.caret = selection.head;
    editor.auto_replacement = Some(replacement);
}

pub(super) fn try_undo_auto_replacement(editor: &mut Editor) -> Result<bool, EditorError> {
    let Some(replacement) = editor.auto_replacement.take() else {
        return Ok(false);
    };
    if editor.state.composition.is_some()
        || editor.state.selection != Some(Selection::collapsed(replacement.caret))
    {
        return Ok(false);
    }
    let Some((separator, separator_entry)) = replacement.separator else {
        // Preserve the existing immediate-backspace undo/redo contract.
        return Ok(editor.try_undo());
    };
    editor.transact(|tr| {
        let paint_len = match &separator {
            InputSeparator::Text(text) => Some(text.chars().count()),
            InputSeparator::Tab => Some(1),
            _ => None,
        };
        let paint = paint_len.and_then(|len| {
            replacement_paint(
                &tr.state().projected,
                Position::new(
                    replacement.caret.node,
                    replacement.caret.offset.saturating_sub(len),
                ),
                replacement.caret,
            )
        });
        // Restoring older CRDT ops beneath a live suffix can place the
        // restored text after that suffix. Remove the separator first,
        // restore the replacement, then append the preserved separator.
        tr.apply_undo_entry_inverse(&separator_entry)?;
        tr.apply_undo_entry_inverse(&replacement.entry)?;
        match &separator {
            InputSeparator::Text(text) if text == " " => {}
            InputSeparator::Text(text) => {
                commands::replace_selection_with_text(tr, text, paint)?;
            }
            InputSeparator::Tab => {
                commands::insert_tab(tr, paint)?;
            }
            InputSeparator::ParagraphBreak => {
                apply_paragraph_break(tr)?;
            }
            InputSeparator::LineBreak => {
                commands::insert_hard_break(tr)?;
            }
        }
        Ok(())
    })?;
    Ok(true)
}

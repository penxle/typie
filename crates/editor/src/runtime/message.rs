use super::effect::Effect;
use super::{ContextKey, Runtime, When};
use crate::model::{HorizontalRuleVariant, LayoutMode, Mark, TextAlign};
use crate::types::Theme;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    WordLeft,
    WordRight,
    DocumentStart,
    DocumentEnd,
}

macro_rules! define_messages {
    (
        $(
            $name:ident $( { $($field:ident : $type:ty),* $(,)? } )?
            => when $when:expr
            => handle($rt:ident) $block:block
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
        #[serde(tag = "type", rename_all = "camelCase")]
        pub enum Message {
            $(
                #[serde(rename_all = "camelCase")]
                $name $( { $($field: $type),* } )?,
            )*
        }

        impl Message {
            pub fn when(&self) -> When {
                match self {
                    $(
                        Self::$name { .. } => $when,
                    )*
                }
            }

            pub fn handle(self, runtime: &mut Runtime) -> Vec<Effect> {
                match self {
                    $(
                        Self::$name $( { $($field),* } )? => {
                            let $rt = runtime;
                            $block
                        },
                    )*
                }
            }

            pub fn all_actions_with_when() -> Vec<(&'static str, When)> {
                vec![
                    $(
                        (stringify!($name), $when),
                    )*
                ]
            }
        }
    };
}

const TRACKED_ACTIONS: &[&str] = &[
    "Undo",
    "Redo",
    "ToggleBold",
    "ToggleItalic",
    "ToggleStrikethrough",
    "ToggleUnderline",
    "ToggleTextColor",
    "ToggleBackgroundColor",
    "ToggleRuby",
    "SetTextAlign",
    "SetLineHeight",
    "SetLetterSpacing",
    "SetFontFamily",
    "SetFontSize",
    "SetFontWeight",
    "ClearFormatting",
];

/// Toolbar에서 추적할 action들의 목록과 when 조건
pub fn tracked_actions_with_when() -> Vec<(&'static str, When)> {
    Message::all_actions_with_when()
        .into_iter()
        .filter(|(name, _)| TRACKED_ACTIONS.contains(name))
        .collect()
}

define_messages! {
    Initialize { theme: Theme }
    => when When::True
    => handle(rt) { rt.handle_initialize(theme) },

    Input { text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_input(&text) },

    Paste {
        fragment: Option<String>,
        html: Option<String>,
        text: String,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_paste(fragment, html, text) },

    CompositionStart { text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_composition_update(&text) },

    CompositionUpdate { text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_composition_update(&text) },

    CompositionEnd
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::InComposition))
    => handle(rt) { rt.handle_composition_end() },

    PointerDown {
        page_idx: usize,
        x: f32,
        y: f32,
        click_count: u32,
        shift_key: bool,
        is_primary: bool,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_down(page_idx, x, y, click_count, shift_key, is_primary) },

    PointerMove {
        page_idx: usize,
        x: f32,
        y: f32,
        is_pressed: bool,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_move(page_idx, x, y, is_pressed) },

    PointerUp {
        page_idx: usize,
        x: f32,
        y: f32,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_up(page_idx, x, y) },

    DragStart { page_idx: usize, x: f32, y: f32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drag_start(page_idx, x, y) },

    DragOver {
        page_idx: usize,
        x: f32,
        y: f32,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drag_over(page_idx, x, y) },

    DragEnter
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drag_enter() },

    DragLeave
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drag_leave() },

    Drop {
        page_idx: usize,
        x: f32,
        y: f32,
        text: Option<String>,
        html: Option<String>,
        fragment: Option<String>,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drop(page_idx, x, y, text, html, fragment) },

    DragEnd
    => when When::True
    => handle(rt) { rt.handle_drag_end() },

    Navigate {
        direction: Direction,
        extend: bool,
    }
    => when When::True
    => handle(rt) { rt.handle_navigate(direction, extend) },

    SelectAll
    => when When::True
    => handle(rt) { rt.handle_select_all() },

    DeleteSelection
    => when When::key(ContextKey::RangeSelection)
        .and(When::key(ContextKey::CanEdit))
    => handle(rt) { rt.transact(|tr| tr.delete_selection()) },

    DeleteBackward
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_backward() },

    DeleteForward
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_forward() },

    DeleteWordBackward
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_word_backward() },

    DeleteWordForward
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_word_forward() },

    DeleteToLineStart
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_to_line_start() },

    InsertNewline
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_newline() },

    InsertHardBreak
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_hard_break() },

    InsertPageBreak
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_page_break() },

    ToggleBold
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_bold() },

    ToggleItalic
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_italic() },

    ToggleStrikethrough
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_strikethrough() },

    ToggleUnderline
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_underline() },

    ToggleRuby { text: String }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection))
    => handle(rt) { rt.handle_toggle_ruby(text) },

    ToggleBlockquote
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_blockquote() },

    ToggleCallout
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_callout() },

    ToggleBulletList
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_bullet_list() },

    ToggleOrderedList
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_ordered_list() },

    Undo
    => when When::key(ContextKey::CanUndo)
        .and(When::key(ContextKey::CanEdit))
    => handle(rt) { rt.handle_undo() },

    Redo
    => when When::key(ContextKey::CanRedo)
        .and(When::key(ContextKey::CanEdit))
    => handle(rt) { rt.handle_redo() },

    SetFontFamily { family: String }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_font_family(family) },

    SetFontSize { size: f32 }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_font_size(size) },

    SetFontWeight { weight: u16 }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_font_weight(weight) },

    SetLineHeight { height: f32 }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_line_height(height) },

    SetLetterSpacing { spacing: f32 }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_letter_spacing(spacing) },

    SetTextAlign { align: TextAlign }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_text_align(align) },

    SetBlockGap { gap: f32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_block_gap(gap) },

    SetParagraphIndent { indent: f32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_paragraph_indent(indent) },

    ToggleTextColor { key: String }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_text_color(key) },

    ToggleBackgroundColor { key: Option<String> }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_background_color(key) },

    ClearFormatting
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_clear_formatting() },

    Indent
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_indent() },

    Outdent
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_outdent() },

    ExtendMarkRange { mark: Mark }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_extend_mark_range(mark) },

    InsertImage {
        src: String,
        width: f32,
        height: f32,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_image(src, width, height) },

    InsertHorizontalRule { variant: HorizontalRuleVariant }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_horizontal_rule(variant) },

    SetLayoutMode { mode: LayoutMode }
    => when When::True
    => handle(rt) { rt.handle_set_layout_mode(mode) },

    Resize { width: f32, scale_factor: f64 }
    => when When::True
    => handle(rt) { rt.handle_resize(width, scale_factor) },

    SetTheme { theme: Theme }
    => when When::True
    => handle(rt) { rt.handle_set_theme(theme) },

    FontsLoaded
    => when When::True
    => handle(rt) { rt.handle_fonts_loaded() },

    Escape
    => when When::True
    => handle(rt) { rt.handle_escape() },

    ToggleFoldExpansion { node_id: String }
    => when When::True
    => handle(rt) { rt.handle_toggle_fold_expansion(node_id) },

    InsertFold
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_fold() },
}

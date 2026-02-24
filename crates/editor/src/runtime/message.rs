use crate::model::{
    Annotation, AnnotationType, BlockquoteVariant, DefaultAttrs, HorizontalRuleVariant, LayoutMode,
    Style, TableAlign, TextAlign,
};
use crate::runtime::effect::Effect;
use crate::runtime::{Context, ContextKey, Runtime, When};
use crate::state::Selection;
use crate::types::{Affinity, Theme};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub enum PointerButton {
    Primary,
    Auxiliary,
    Secondary,
}

impl PointerButton {
    pub fn is_primary(&self) -> bool {
        matches!(self, Self::Primary)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Modifier {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
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
    PageUp,
    PageDown,
    SentenceUp,
    SentenceDown,
}

macro_rules! define_messages {
    (
        $(
            $name:ident $( { $($field:ident : $type:ty),* $(,)? } )?
            => when $when:expr
            => handle($rt:ident) $block:block
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
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
                let ctx = Context::new(&runtime.state, &runtime.undo_manager);
                if !self.when().evaluate(&ctx) {
                    return Vec::new();
                }

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
    "ToggleStyle",
    "SetTextAlign",
    "SetLineHeight",
    "ClearFormatting",
    "AddAnnotation",
    "RemoveAnnotation",
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

    ReplaceBackward { length: usize, text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_replace_backward(length, &text) },

    PasteHtml {
        html: String,
        text: String,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_paste_html(html, text) },

    PasteText {
        text: String,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_paste_text(text) },

    RepasteAsText
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_repaste_as_text() },

    CompositionStart { text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_composition_update(&text) },

    CompositionUpdate { text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_composition_update(&text) },

    CompositionEnd
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_composition_end() },

    CommitPreedit
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_commit_preedit() },

    PointerDown {
        page_idx: usize,
        x: f32,
        y: f32,
        click_count: u32,
        button: PointerButton,
        modifier: Modifier,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_down(page_idx, x, y, click_count, button, modifier) },

    PointerMove {
        page_idx: usize,
        x: f32,
        y: f32,
        buttons: u16,
        modifier: Modifier,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_move(page_idx, x, y, buttons, modifier) },

    PointerUp {
        page_idx: usize,
        x: f32,
        y: f32,
        button: PointerButton,
        modifier: Modifier,
    }
    => when When::True
    => handle(rt) { rt.handle_pointer_up(page_idx, x, y, button, modifier) },

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
        modifier: Modifier,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drop(page_idx, x, y, text, html, modifier) },

    DropImages {
        page_idx: usize,
        x: f32,
        y: f32,
        upload_ids: Vec<String>,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drop_images(page_idx, x, y, upload_ids) },

    DropFiles {
        page_idx: usize,
        x: f32,
        y: f32,
        upload_ids: Vec<String>,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_drop_files(page_idx, x, y, upload_ids) },

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

    SelectWord
    => when When::True
    => handle(rt) { rt.handle_select_word() },

    SelectSentence
    => when When::True
    => handle(rt) { rt.handle_select_sentence() },

    SelectParagraph
    => when When::True
    => handle(rt) { rt.handle_select_paragraph() },

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

    DeleteSentenceBackward
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_sentence_backward() },

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

    ToggleStyle { style: Style }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_toggle_style(style) },

    AddAnnotation { annotation: Annotation }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection))
    => handle(rt) { rt.handle_add_annotation(annotation) },

    UpdateAnnotation { annotation: Annotation }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_update_annotation(annotation) },

    RemoveAnnotation { annotation_type: AnnotationType }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_remove_annotation(annotation_type) },

    ToggleBlockquote { variant: BlockquoteVariant }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_blockquote(variant) },

    SetBlockquote { variant: BlockquoteVariant }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_blockquote(variant) },

    ToggleCallout
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_toggle_callout() },

    CycleCalloutVariant
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_cycle_callout_variant_in_selection() },

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


    SetLineHeight { height: u32 }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_line_height(height) },


    SetTextAlign { align: TextAlign }
    => when When::key(ContextKey::CanEdit)
        .and(When::key(ContextKey::HasParagraphTextInSelection).or(When::key(ContextKey::InParagraph)))
    => handle(rt) { rt.handle_set_text_align(align) },

    SetBlockGap { gap: u32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_block_gap(gap) },

    SetParagraphIndent { indent: u32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_paragraph_indent(indent) },

    SetDefaultAttrs { attrs: DefaultAttrs }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_default_attrs(attrs) },

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

    InsertImage { upload_id: Option<String> }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_image(upload_id) },

    InsertFile { upload_id: Option<String> }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_file(upload_id) },

    InsertEmbed
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_embed() },

    InsertHorizontalRule { variant: HorizontalRuleVariant }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_horizontal_rule(variant) },

    SetHorizontalRule { variant: HorizontalRuleVariant }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_horizontal_rule(variant) },

    SetLayoutMode { mode: LayoutMode }
    => when When::True
    => handle(rt) { rt.handle_set_layout_mode(mode) },

    Resize { width: f32, height: f32, scale_factor: f64 }
    => when When::True
    => handle(rt) { rt.handle_resize(width, height, scale_factor) },

    SetTheme { theme: Theme }
    => when When::True
    => handle(rt) { rt.handle_set_theme(theme) },

    FontsLoaded {
        family: String,
        weight: u16,
    }
    => when When::True
    => handle(rt) { rt.handle_fonts_loaded(family, weight) },

    Escape
    => when When::True
    => handle(rt) { rt.handle_escape() },

    InsertFold
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_fold() },

    UnwrapFold
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_unwrap_fold() },

    InsertTable { rows: u32, cols: u32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_insert_table(rows, cols) },

    SetColumnWidths { table_id: String, col_widths: Vec<f32> }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_column_widths(table_id, col_widths) },

    AddTableRow { table_id: String, row: usize, before: bool }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_add_table_row(table_id, row, before) },

    AddTableColumn { table_id: String, col: usize, before: bool }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_add_table_column(table_id, col, before) },

    DeleteTableRow { table_id: String, row: usize }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_table_row(table_id, row) },

    DeleteTableColumn { table_id: String, col: usize }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_table_column(table_id, col) },

    SetTableBorderStyle { table_id: String, style: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_table_border_style(table_id, style) },

    SetTableAlign { table_id: String, align: TableAlign }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_table_align(table_id, align) },

    SetTableProportion { table_id: String, proportion: f32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_table_proportion(table_id, proportion) },

    SetTableWidth {
        table_id: String,
        width: f32,
        content_width: f32,
    }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_table_width(table_id, width, content_width) },

    SelectTable { table_id: String }
    => when When::True
    => handle(rt) { rt.handle_select_table(table_id) },

    SelectTableRow { table_id: String, row: usize }
    => when When::True
    => handle(rt) { rt.handle_select_table_row(table_id, row) },

    SelectTableColumn { table_id: String, col: usize }
    => when When::True
    => handle(rt) { rt.handle_select_table_column(table_id, col) },

    MoveTableRow { table_id: String, from_row: usize, to_row: usize }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_move_table_row(table_id, from_row, to_row) },

    MoveTableColumn { table_id: String, from_col: usize, to_col: usize }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_move_table_column(table_id, from_col, to_col) },

    DeleteNode { node_id: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_delete_node(node_id) },

    SetImageProportion { node_id: String, proportion: f32 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_image_proportion(node_id, proportion) },

    SetImageId { node_id: String, image_id: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_image_id(node_id, image_id) },

    SetFileId { node_id: String, file_id: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_file_id(node_id, file_id) },

    SetEmbedId { node_id: String, embed_id: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_set_embed_id(node_id, embed_id) },

    SetExternalElementHeight { node_id: String, height: f32 }
    => when When::True
    => handle(rt) { rt.handle_set_external_element_height(node_id, height) },

    SetFocused { focused: bool }
    => when When::True
    => handle(rt) { rt.handle_set_focused(focused) },

    SetSelection {
        anchor_node_id: String,
        anchor_offset: usize,
        anchor_affinity: Affinity,
        head_node_id: String,
        head_offset: usize,
        head_affinity: Affinity,
    }
    => when When::True
    => handle(rt) { rt.handle_set_selection(anchor_node_id, anchor_offset, anchor_affinity, head_node_id, head_offset, head_affinity) },

    CollapseSelection { to_anchor: bool }
    => when When::True
    => handle(rt) { rt.handle_collapse_selection(to_anchor) },

    ExtendSelectionTo {
        anchor_page_idx: usize,
        anchor_x: f32,
        anchor_y: f32,
        head_page_idx: usize,
        head_x: f32,
        head_y: f32,
        double_tap_initial_range: Option<Selection>,
    }
    => when When::True
    => handle(rt) { rt.handle_extend_selection_to(anchor_page_idx, anchor_x, anchor_y, head_page_idx, head_x, head_y, double_tap_initial_range) },

    AddRemark { node_id: String, user_id: String, text: String, created_at: i64 }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_add_remark(node_id, user_id, text, created_at) },

    UpdateRemark { node_id: String, remark_id: String, text: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_update_remark(node_id, remark_id, text) },

    RemoveRemark { node_id: String, remark_id: String }
    => when When::key(ContextKey::CanEdit)
    => handle(rt) { rt.handle_remove_remark(node_id, remark_id) },
}

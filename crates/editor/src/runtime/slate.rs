use crate::layout::elements::ExternalElementData;
use crate::model::{
    Annotation, AnnotationType, Attr, DefaultAttrs, LayoutMode, NodeId, NodeType, Style, StyleType,
    TextAlign,
};
use crate::runtime::tracked_items::TrackedItemOverlay;
use crate::runtime::{
    DropIndicator, ExternalElement, LinkOverlay, SelectionHandleBounds, TableOverlay,
};
use crate::state::Selection;
use crate::state::selection_helpers::{BlockAttr, SelectionAttributes};
use crate::types::{Affinity, PointerStyle, Rect, TextBound};
use rustc_hash::FxHashMap;

pub const DIRTY_SETTINGS: u64 = 1 << 0;
pub const DIRTY_PAGES: u64 = 1 << 1;
pub const DIRTY_CURSOR: u64 = 1 << 2;
pub const DIRTY_SELECTION: u64 = 1 << 3;
pub const DIRTY_ATTRS: u64 = 1 << 4;
pub const DIRTY_POINTER: u64 = 1 << 5;
pub const DIRTY_DEFAULT_ATTRS: u64 = 1 << 6;
pub const DIRTY_PLACEHOLDER: u64 = 1 << 7;
pub const DIRTY_EXTERNAL_ELEMENTS: u64 = 1 << 8;
pub const DIRTY_ENABLED_ACTIONS: u64 = 1 << 9;
pub const DIRTY_LINK_OVERLAYS: u64 = 1 << 10;
pub const DIRTY_TRACKED_ITEMS: u64 = 1 << 11;
pub const DIRTY_TABLE_OVERLAYS: u64 = 1 << 14;
pub const DIRTY_DOC_CHANGED: u64 = 1 << 15;
pub const DIRTY_RENDER_REQUIRED: u64 = 1 << 16;
pub const DIRTY_FONT_REQUIRED: u64 = 1 << 17;
pub const DIRTY_EXITED_DOCUMENT_START: u64 = 1 << 19;
pub const DIRTY_REPASTE: u64 = 1 << 20;
pub const DIRTY_REMARKS: u64 = 1 << 21;

pub const ATTR_TAG_BACKGROUND_COLOR: u32 = 0;
pub const ATTR_TAG_BOLD: u32 = 7;
pub const ATTR_TAG_TEXT_COLOR: u32 = 1;
pub const ATTR_TAG_FONT_SIZE: u32 = 2;
pub const ATTR_TAG_FONT_FAMILY: u32 = 3;
pub const ATTR_TAG_FONT_WEIGHT: u32 = 4;
pub const ATTR_TAG_ITALIC: u32 = 5;
pub const ATTR_TAG_LETTER_SPACING: u32 = 6;
pub const ATTR_TAG_STRIKETHROUGH: u32 = 9;
pub const ATTR_TAG_UNDERLINE: u32 = 10;
pub const ATTR_TAG_TEXT_ALIGN: u32 = 20;
pub const ATTR_TAG_LINE_HEIGHT: u32 = 21;
pub const ATTR_TAG_LINK: u32 = 30;
pub const ATTR_TAG_RUBY: u32 = 31;

pub const VK_UNIT: u32 = 0;
pub const VK_F32: u32 = 1;
pub const VK_U32: u32 = 2;
pub const VK_STRING: u32 = 3;
pub const VK_COMPOSITE: u32 = 4;
pub const VK_I32: u32 = 5;

pub const ALIGN_LEFT: u32 = 0;
pub const ALIGN_CENTER: u32 = 1;
pub const ALIGN_RIGHT: u32 = 2;
pub const ALIGN_JUSTIFY: u32 = 3;

pub const SELECTION_TYPE_NONE: u32 = 0;
pub const SELECTION_TYPE_HORIZONTAL_RULE: u32 = 1;
pub const SELECTION_TYPE_CALLOUT: u32 = 2;
pub const SELECTION_TYPE_FOLD: u32 = 3;
pub const SELECTION_TYPE_BULLET_LIST: u32 = 4;
pub const SELECTION_TYPE_ORDERED_LIST: u32 = 5;
pub const SELECTION_TYPE_IMAGE: u32 = 6;
pub const SELECTION_TYPE_FILE: u32 = 7;
pub const SELECTION_TYPE_EMBED: u32 = 8;
pub const SELECTION_TYPE_ARCHIVED: u32 = 9;
pub const SELECTION_TYPE_BLOCKQUOTE: u32 = 10;
pub const SELECTION_TYPE_TABLE: u32 = 11;

pub const AFFINITY_UPSTREAM: u32 = 0;
pub const AFFINITY_DOWNSTREAM: u32 = 1;

pub const POINTER_STYLE_DEFAULT: u32 = 0;
pub const POINTER_STYLE_TEXT: u32 = 1;
pub const POINTER_STYLE_POINTER: u32 = 2;

pub fn node_id_to_bytes(id: NodeId) -> [u8; 16] {
    *id.as_uuid().as_bytes()
}

pub fn selection_type(node_type: NodeType) -> u32 {
    match node_type {
        NodeType::HorizontalRule => SELECTION_TYPE_HORIZONTAL_RULE,
        NodeType::Callout => SELECTION_TYPE_CALLOUT,
        NodeType::Fold => SELECTION_TYPE_FOLD,
        NodeType::BulletList => SELECTION_TYPE_BULLET_LIST,
        NodeType::OrderedList => SELECTION_TYPE_ORDERED_LIST,
        NodeType::Image => SELECTION_TYPE_IMAGE,
        NodeType::File => SELECTION_TYPE_FILE,
        NodeType::Embed => SELECTION_TYPE_EMBED,
        NodeType::Archived => SELECTION_TYPE_ARCHIVED,
        NodeType::Blockquote => SELECTION_TYPE_BLOCKQUOTE,
        NodeType::Table => SELECTION_TYPE_TABLE,
        _ => SELECTION_TYPE_NONE,
    }
}

macro_rules! define_slate {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(pub $field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(pub $field: $ty),*
        }

        pub fn get_slate_offsets() -> Vec<(&'static str, usize)> {
            let mut offsets = vec![
                $((stringify!($field), std::mem::offset_of!($name, $field))),*
            ];
            offsets.sort_by_key(|&(_, off)| off);
            offsets
        }
    };
}

define_slate! {
    #[repr(C)]
    pub struct Slate {
        pub dirty: u64,

        pub slab_len: u32,
        pub slab_capacity: u32,

        pub layout_mode_offset: u32,

        pub paragraph_indent: u32,
        pub block_gap: u32,

        pub pages_offset: u32,
        pub pages_count: u32,

        pub cursor_page_idx: i32,
        pub cursor_x: f32,
        pub cursor_y: f32,
        pub cursor_width: f32,
        pub cursor_height: f32,
        pub cursor_visible: u32,

        pub preceding_char_widths_offset: u32,
        pub preceding_char_widths_count: u32,

        pub selection_cmp: i32,
        pub selection_block_ids_offset: u32,
        pub selection_block_ids_count: u32,
        pub selection_block_types_offset: u32,
        pub selection_block_types_count: u32,
        pub selection_common_ancestor_ids_offset: u32,
        pub selection_common_ancestor_ids_count: u32,
        pub selection_common_ancestor_types_offset: u32,
        pub selection_common_ancestor_types_count: u32,

        pub selection_anchor_node_id: [u8; 16],
        pub selection_anchor_offset: u32,
        pub selection_anchor_affinity: u32,
        pub selection_anchor_page_idx: i32,
        pub selection_anchor_x: f32,
        pub selection_anchor_y: f32,
        pub selection_anchor_width: f32,
        pub selection_anchor_height: f32,

        pub selection_head_node_id: [u8; 16],
        pub selection_head_offset: u32,
        pub selection_head_affinity: u32,
        pub selection_head_page_idx: i32,
        pub selection_head_x: f32,
        pub selection_head_y: f32,
        pub selection_head_width: f32,
        pub selection_head_height: f32,

        pub selection_expandable: u32,

        pub pointer_style: u32,
        pub pointer_state: u32,

        pub placeholder_x: f32,
        pub placeholder_y: f32,
        pub placeholder_width: f32,
        pub placeholder_height: f32,
        pub placeholder_visible: u32,

        pub attrs_offset: u32,
        pub attrs_count: u32,

        pub enabled_actions_offset: u32,
        pub enabled_actions_count: u32,

        pub external_elements_offset: u32,
        pub external_elements_count: u32,

        pub link_overlays_offset: u32,
        pub link_overlays_count: u32,
        pub tracked_items_offset: u32,
        pub tracked_items_count: u32,
        pub table_overlays_offset: u32,
        pub table_overlays_count: u32,

        pub font_requests_offset: u32,
        pub font_requests_count: u32,
        pub html_pasted_offset: u32,
        pub html_pasted_len: u32,

        pub default_attrs_offset: u32,
        pub default_attrs_count: u32,

        pub repaste_enabled: u32,

        pub remarks_offset: u32,
        pub remarks_count: u32,

        pub current_block_node_id: [u8; 16],
        pub current_block_page_idx: i32,
        pub current_block_x: f32,
        pub current_block_y: f32,
        pub current_block_width: f32,
        pub current_block_height: f32,

        pub drop_indicator_page_idx: i32,
        pub drop_indicator_x: f32,
        pub drop_indicator_y: f32,
        pub drop_indicator_width: f32,
        pub drop_indicator_height: f32,
    }
}

impl Default for Slate {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Slate {
    pub fn mark_doc_changed(&mut self) {
        self.dirty |= DIRTY_DOC_CHANGED;
    }

    pub fn mark_render_required(&mut self) {
        self.dirty |= DIRTY_RENDER_REQUIRED;
    }

    pub fn mark_exited_document_start(&mut self) {
        self.dirty |= DIRTY_EXITED_DOCUMENT_START;
    }

    pub fn write_pointer_state(&mut self, state: u32) {
        self.pointer_state = state;
        self.dirty |= DIRTY_POINTER;
    }

    pub fn write_repaste_enabled(&mut self, enabled: bool) {
        self.repaste_enabled = if enabled { 1 } else { 0 };
        self.dirty |= DIRTY_REPASTE
    }
}

pub struct Slab {
    pub data: Vec<u8>,
    len: usize,
}

impl Slab {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(4096),
            len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn alloc(&mut self, size: usize, align: usize) -> u32 {
        let misalign = self.len % align;
        if misalign != 0 {
            self.len += align - misalign;
        }
        let offset = self.len;
        self.len += size;
        if self.data.len() < self.len {
            self.data.resize(self.len, 0);
        }
        offset as u32
    }

    pub fn write_f32_slice(&mut self, values: &[f32]) -> (u32, u32) {
        if values.is_empty() {
            return (0, 0);
        }
        let byte_len = values.len() * 4;
        let offset = self.alloc(byte_len, 4);
        let dst = &mut self.data[offset as usize..offset as usize + byte_len];
        for (i, &v) in values.iter().enumerate() {
            dst[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        (offset, values.len() as u32)
    }

    pub fn write_u32_slice(&mut self, values: &[u32]) -> (u32, u32) {
        if values.is_empty() {
            return (0, 0);
        }
        let byte_len = values.len() * 4;
        let offset = self.alloc(byte_len, 4);
        let dst = &mut self.data[offset as usize..offset as usize + byte_len];
        for (i, &v) in values.iter().enumerate() {
            dst[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        (offset, values.len() as u32)
    }

    pub fn write_i32_slice(&mut self, values: &[i32]) -> (u32, u32) {
        if values.is_empty() {
            return (0, 0);
        }
        let byte_len = values.len() * 4;
        let offset = self.alloc(byte_len, 4);
        let dst = &mut self.data[offset as usize..offset as usize + byte_len];
        for (i, &v) in values.iter().enumerate() {
            dst[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        (offset, values.len() as u32)
    }

    pub fn write_node_id_slice(&mut self, ids: &[crate::model::NodeId]) -> (u32, u32) {
        if ids.is_empty() {
            return (0, 0);
        }

        let byte_len = ids.len() * 16;
        let offset = self.alloc(byte_len, 4);
        let dst = &mut self.data[offset as usize..offset as usize + byte_len];
        for (idx, node_id) in ids.iter().enumerate() {
            let start = idx * 16;
            dst[start..start + 16].copy_from_slice(node_id.as_uuid().as_bytes());
        }
        (offset, ids.len() as u32)
    }

    pub fn write_str(&mut self, s: &str) -> u32 {
        self.write_bytes(s.as_bytes())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> u32 {
        let total = 4 + bytes.len();
        let offset = self.alloc(total, 4);
        let len_bytes = (bytes.len() as u32).to_le_bytes();
        self.data[offset as usize..offset as usize + 4].copy_from_slice(&len_bytes);
        self.data[offset as usize + 4..offset as usize + 4 + bytes.len()].copy_from_slice(bytes);
        offset
    }

    fn encode_attrs(&mut self, attrs: &SelectionAttributes) -> (u32, u32) {
        let start = self.alloc(0, 4);
        let mut entry_count = 0u32;

        for collected in &attrs.block_attrs {
            let count = collected.values.len() as u32 + if collected.has_absent { 1 } else { 0 };
            if count == 0 {
                continue;
            }
            let Some(first) = collected.values.first() else {
                continue;
            };
            match first {
                BlockAttr::TextAlign(_) => {
                    self.write_u32_slice(&[ATTR_TAG_TEXT_ALIGN, VK_U32, count]);
                    for val in &collected.values {
                        if let BlockAttr::TextAlign(align) = val {
                            self.write_u32_slice(&[match align {
                                TextAlign::Left => ALIGN_LEFT,
                                TextAlign::Center => ALIGN_CENTER,
                                TextAlign::Right => ALIGN_RIGHT,
                                TextAlign::Justify => ALIGN_JUSTIFY,
                            }]);
                        }
                    }
                    if collected.has_absent {
                        self.write_null_sentinel(VK_U32);
                    }
                }
                BlockAttr::LineHeight(_) => {
                    self.write_u32_slice(&[ATTR_TAG_LINE_HEIGHT, VK_U32, count]);
                    for val in &collected.values {
                        if let BlockAttr::LineHeight(lh) = val {
                            self.write_u32_slice(&[*lh]);
                        }
                    }
                    if collected.has_absent {
                        self.write_null_sentinel(VK_U32);
                    }
                }
            }
            entry_count += 1;
        }

        for (&st, values) in &attrs.style_values {
            let type_tag = style_type_to_tag(&st);
            let value_kind = style_type_to_value_kind(&st);
            let is_absent = attrs.absent_styles.contains(&st);
            let count = values.len() as u32 + if is_absent { 1 } else { 0 };
            if count == 0 {
                continue;
            }

            self.write_u32_slice(&[type_tag, value_kind, count]);
            for style in values {
                self.write_style_value(style);
            }
            if is_absent {
                self.write_null_sentinel(value_kind);
            }
            entry_count += 1;
        }

        for (&at, values) in &attrs.annotation_values {
            let type_tag = annotation_type_to_tag(&at);
            let is_absent = attrs.absent_annotations.contains(&at);
            let count = values.len() as u32 + if is_absent { 1 } else { 0 };
            if count == 0 {
                continue;
            }

            self.write_u32_slice(&[type_tag, VK_COMPOSITE, count]);
            for annotation in values {
                self.write_annotation_instance(annotation);
            }
            if is_absent {
                self.write_u32_slice(&[0xFFFFFFFF]);
            }
            entry_count += 1;
        }

        (start, entry_count)
    }

    pub fn write_text_bounds(&mut self, bounds: &[TextBound]) {
        for b in bounds {
            self.write_f32_slice(&[b.x, b.y, b.width, b.height, b.ascent]);
        }
    }

    fn write_style_value(&mut self, style: &Style) {
        match style {
            Style::BackgroundColor(s) => {
                self.write_str(&s.color);
            }
            Style::Bold(_) => {
                self.write_u32_slice(&[1]);
            }
            Style::TextColor(s) => {
                self.write_str(&s.color);
            }
            Style::FontSize(s) => {
                self.write_u32_slice(&[s.size]);
            }
            Style::FontFamily(s) => {
                self.write_str(&s.family);
            }
            Style::FontWeight(s) => {
                self.write_u32_slice(&[s.weight as u32]);
            }
            Style::Italic(_) => {
                self.write_u32_slice(&[1]);
            }
            Style::LetterSpacing(s) => {
                self.write_i32_slice(&[s.spacing]);
            }
            Style::Strikethrough(_) => {
                self.write_u32_slice(&[1]);
            }
            Style::Underline(_) => {
                self.write_u32_slice(&[1]);
            }
        }
    }

    fn write_null_sentinel(&mut self, value_kind: u32) {
        match value_kind {
            VK_UNIT | VK_U32 | VK_STRING | VK_COMPOSITE => {
                self.write_u32_slice(&[0xFFFFFFFF]);
            }
            VK_F32 => {
                self.write_f32_slice(&[f32::NAN]);
            }
            VK_I32 => {
                self.write_i32_slice(&[i32::MIN]);
            }
            _ => {}
        }
    }

    fn write_annotation_instance(&mut self, annotation: &Annotation) {
        match annotation {
            Annotation::Link(link) => {
                self.write_u32_slice(&[1]);
                self.write_u32_slice(&[VK_STRING]);
                self.write_str(&link.href);
            }
            Annotation::Ruby(ruby) => {
                self.write_u32_slice(&[1]);
                self.write_u32_slice(&[VK_STRING]);
                self.write_str(&ruby.text);
            }
        }
    }

    pub fn write_pages(&mut self, slate: &mut Slate, pages_data: &[f32]) {
        let (off, cnt) = self.write_f32_slice(pages_data);
        slate.pages_offset = off;
        slate.pages_count = cnt / 2;
        slate.dirty |= DIRTY_PAGES;
    }

    pub fn write_settings(
        &mut self,
        slate: &mut Slate,
        paragraph_indent: u32,
        block_gap: u32,
        layout_mode: LayoutMode,
    ) {
        slate.paragraph_indent = paragraph_indent;
        slate.block_gap = block_gap;

        let lm_start = self.alloc(0, 4);
        match layout_mode {
            LayoutMode::Paginated {
                page_width,
                page_height,
                page_margin_top,
                page_margin_bottom,
                page_margin_left,
                page_margin_right,
            } => {
                self.write_u32_slice(&[0]);
                self.write_f32_slice(&[
                    page_width,
                    page_height,
                    page_margin_top,
                    page_margin_bottom,
                    page_margin_left,
                    page_margin_right,
                ]);
            }
            LayoutMode::Continuous { max_width } => {
                self.write_u32_slice(&[1]);
                self.write_f32_slice(&[max_width]);
            }
        }
        slate.layout_mode_offset = lm_start;
        slate.dirty |= DIRTY_SETTINGS;
    }

    pub fn write_cursor(
        &mut self,
        slate: &mut Slate,
        page_idx: Option<usize>,
        bounds: Option<Rect>,
        visible: bool,
        preceding_char_widths: Option<&[f32]>,
    ) {
        slate.cursor_page_idx = page_idx.map(|i| i as i32).unwrap_or(-1);
        if let Some(b) = bounds {
            slate.cursor_x = b.x;
            slate.cursor_y = b.y;
            slate.cursor_width = b.width;
            slate.cursor_height = b.height;
        } else {
            slate.cursor_x = 0.0;
            slate.cursor_y = 0.0;
            slate.cursor_width = 0.0;
            slate.cursor_height = 0.0;
        }
        slate.cursor_visible = visible as u32;

        if let Some(widths) = preceding_char_widths {
            let (off, cnt) = self.write_f32_slice(widths);
            slate.preceding_char_widths_offset = off;
            slate.preceding_char_widths_count = cnt;
        } else {
            slate.preceding_char_widths_offset = 0;
            slate.preceding_char_widths_count = 0;
        }
        slate.dirty |= DIRTY_CURSOR;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_selection(
        &mut self,
        slate: &mut Slate,
        selection: Selection,
        cmp: i32,
        block_ids: &[NodeId],
        block_types: &[u32],
        ancestor_ids: &[NodeId],
        ancestor_types: &[u32],
        anchor_handle: Option<&SelectionHandleBounds>,
        head_handle: Option<&SelectionHandleBounds>,
    ) {
        let (selected_block_ids_offset, selected_block_ids_count) =
            self.write_node_id_slice(block_ids);
        let (selected_block_types_offset, selected_block_types_count) =
            self.write_u32_slice(block_types);
        let (common_ancestor_ids_offset, common_ancestor_ids_count) =
            self.write_node_id_slice(ancestor_ids);
        let (common_ancestor_types_offset, common_ancestor_types_count) =
            self.write_u32_slice(ancestor_types);

        slate.selection_cmp = cmp;
        slate.selection_block_ids_offset = selected_block_ids_offset;
        slate.selection_block_ids_count = selected_block_ids_count;
        slate.selection_block_types_offset = selected_block_types_offset;
        slate.selection_block_types_count = selected_block_types_count;
        slate.selection_common_ancestor_ids_offset = common_ancestor_ids_offset;
        slate.selection_common_ancestor_ids_count = common_ancestor_ids_count;
        slate.selection_common_ancestor_types_offset = common_ancestor_types_offset;
        slate.selection_common_ancestor_types_count = common_ancestor_types_count;
        slate.selection_anchor_node_id = node_id_to_bytes(selection.anchor.node_id);
        slate.selection_anchor_offset = selection.anchor.offset as u32;
        slate.selection_anchor_affinity = match selection.anchor.affinity {
            Affinity::Upstream => AFFINITY_UPSTREAM,
            Affinity::Downstream => AFFINITY_DOWNSTREAM,
        };
        slate.selection_head_node_id = node_id_to_bytes(selection.head.node_id);
        slate.selection_head_offset = selection.head.offset as u32;
        slate.selection_head_affinity = match selection.head.affinity {
            Affinity::Upstream => AFFINITY_UPSTREAM,
            Affinity::Downstream => AFFINITY_DOWNSTREAM,
        };

        if let Some(h) = anchor_handle {
            slate.selection_anchor_page_idx = h.page_idx as i32;
            slate.selection_anchor_x = h.bounds.x;
            slate.selection_anchor_y = h.bounds.y;
            slate.selection_anchor_width = h.bounds.width;
            slate.selection_anchor_height = h.bounds.height;
        } else {
            slate.selection_anchor_page_idx = -1;
            slate.selection_anchor_x = 0.0;
            slate.selection_anchor_y = 0.0;
            slate.selection_anchor_width = 0.0;
            slate.selection_anchor_height = 0.0;
        }

        if let Some(h) = head_handle {
            slate.selection_head_page_idx = h.page_idx as i32;
            slate.selection_head_x = h.bounds.x;
            slate.selection_head_y = h.bounds.y;
            slate.selection_head_width = h.bounds.width;
            slate.selection_head_height = h.bounds.height;
        } else {
            slate.selection_head_page_idx = -1;
            slate.selection_head_x = 0.0;
            slate.selection_head_y = 0.0;
            slate.selection_head_width = 0.0;
            slate.selection_head_height = 0.0;
        }
        slate.dirty |= DIRTY_SELECTION;
    }

    pub fn write_attrs(&mut self, slate: &mut Slate, attrs: Option<&SelectionAttributes>) {
        if let Some(attrs) = attrs {
            let (offset, count) = self.encode_attrs(attrs);
            slate.attrs_offset = offset;
            slate.attrs_count = count;
        } else {
            slate.attrs_offset = 0;
            slate.attrs_count = 0;
        }
        slate.dirty |= DIRTY_ATTRS;
    }

    pub fn write_external_elements(&mut self, slate: &mut Slate, elements: &[ExternalElement]) {
        let start = self.alloc(0, 4);
        for el in elements {
            self.write_u32_slice(&[el.page_idx as u32]);
            self.write_str(&el.node_id);
            self.write_f32_slice(&[el.bounds.x, el.bounds.y, el.bounds.width, el.bounds.height]);
            self.write_u32_slice(&[el.is_selected as u32]);
            match &el.data {
                ExternalElementData::Image {
                    id,
                    proportion,
                    upload_id,
                } => {
                    self.write_u32_slice(&[0]);
                    self.write_str(id.as_deref().unwrap_or(""));
                    self.write_str(upload_id.as_deref().unwrap_or(""));
                    self.write_f32_slice(&[*proportion]);
                }
                ExternalElementData::File { id, upload_id } => {
                    self.write_u32_slice(&[1]);
                    self.write_str(id.as_deref().unwrap_or(""));
                    self.write_str(upload_id.as_deref().unwrap_or(""));
                }
                ExternalElementData::Embed { id } => {
                    self.write_u32_slice(&[2]);
                    self.write_str(id.as_deref().unwrap_or(""));
                }
                ExternalElementData::Archived { id } => {
                    self.write_u32_slice(&[3]);
                    self.write_str(id.as_deref().unwrap_or(""));
                }
            }
        }
        slate.external_elements_offset = start;
        slate.external_elements_count = elements.len() as u32;
        slate.dirty |= DIRTY_EXTERNAL_ELEMENTS;
    }

    pub fn write_pointer_style(&mut self, slate: &mut Slate, style: PointerStyle) {
        slate.pointer_style = match style {
            PointerStyle::Default => POINTER_STYLE_DEFAULT,
            PointerStyle::Text => POINTER_STYLE_TEXT,
            PointerStyle::Pointer => POINTER_STYLE_POINTER,
        };
        slate.dirty |= DIRTY_POINTER;
    }

    pub fn write_font_requests(
        &mut self,
        slate: &mut Slate,
        fonts: &FxHashMap<(String, u16), Vec<u32>>,
    ) {
        let start = self.alloc(0, 4);
        let mut count = 0u32;
        for ((family, weight), codepoints) in fonts {
            if !codepoints.is_empty() {
                self.write_str(family);
                self.write_u32_slice(&[*weight as u32]);
                self.write_u32_slice(&[codepoints.len() as u32]);
                self.write_u32_slice(codepoints);
                count += 1;
            }
        }
        if count > 0 {
            slate.font_requests_offset = start;
            slate.font_requests_count = count;
            slate.dirty |= DIRTY_FONT_REQUIRED;
        }
    }

    pub fn write_enabled_actions(&mut self, slate: &mut Slate, actions: &[String]) {
        let start = self.alloc(0, 4);
        for action in actions {
            self.write_str(action);
        }
        slate.enabled_actions_offset = start;
        slate.enabled_actions_count = actions.len() as u32;
        slate.dirty |= DIRTY_ENABLED_ACTIONS;
    }

    pub fn write_placeholder(&mut self, slate: &mut Slate, visible: bool, bounds: Option<Rect>) {
        slate.placeholder_visible = visible as u32;
        if let Some(b) = bounds {
            slate.placeholder_x = b.x;
            slate.placeholder_y = b.y;
            slate.placeholder_width = b.width;
            slate.placeholder_height = b.height;
        } else {
            slate.placeholder_x = 0.0;
            slate.placeholder_y = 0.0;
            slate.placeholder_width = 0.0;
            slate.placeholder_height = 0.0;
        }
        slate.dirty |= DIRTY_PLACEHOLDER;
    }

    pub fn write_drop_indicator(&mut self, slate: &mut Slate, indicator: Option<&DropIndicator>) {
        if let Some(indicator) = indicator {
            match indicator {
                DropIndicator::Inline {
                    page_idx,
                    x,
                    y,
                    height,
                } => {
                    slate.drop_indicator_page_idx = *page_idx as i32;
                    slate.drop_indicator_x = *x;
                    slate.drop_indicator_y = *y;
                    slate.drop_indicator_width = 1.0;
                    slate.drop_indicator_height = *height;
                }
                DropIndicator::Block {
                    page_idx,
                    x,
                    y,
                    width,
                } => {
                    slate.drop_indicator_page_idx = *page_idx as i32;
                    slate.drop_indicator_x = *x;
                    slate.drop_indicator_y = *y;
                    slate.drop_indicator_width = *width;
                    slate.drop_indicator_height = 1.0;
                }
            }
            return;
        }

        slate.drop_indicator_page_idx = -1;
        slate.drop_indicator_x = 0.0;
        slate.drop_indicator_y = 0.0;
        slate.drop_indicator_width = 0.0;
        slate.drop_indicator_height = 0.0;
    }

    pub fn write_link_overlays(&mut self, slate: &mut Slate, overlays: &[LinkOverlay]) {
        let start = self.alloc(0, 4);
        for o in overlays {
            self.write_u32_slice(&[o.page_idx as u32]);
            self.write_str(&o.href);
            self.write_u32_slice(&[o.bounds.len() as u32]);
            self.write_text_bounds(&o.bounds);
        }
        slate.link_overlays_offset = start;
        slate.link_overlays_count = overlays.len() as u32;
        slate.dirty |= DIRTY_LINK_OVERLAYS;
    }

    pub fn write_tracked_items(&mut self, slate: &mut Slate, overlays: &[TrackedItemOverlay]) {
        let start = self.alloc(0, 4);
        for o in overlays {
            self.write_u32_slice(&[o.page_idx as u32]);
            self.write_u32_slice(&[o.group]);
            self.write_str(&o.id);
            self.write_bytes(o.node_id.as_uuid().as_bytes());
            self.write_u32_slice(&[o.start_offset as u32, o.end_offset as u32]);
            self.write_u32_slice(&[o.bounds.len() as u32]);
            self.write_text_bounds(&o.bounds);
        }
        slate.tracked_items_offset = start;
        slate.tracked_items_count = overlays.len() as u32;
        slate.dirty |= DIRTY_TRACKED_ITEMS;
    }

    pub fn write_table_overlays(&mut self, slate: &mut Slate, overlays: &[TableOverlay]) {
        let start = self.alloc(0, 4);
        for o in overlays {
            self.write_u32_slice(&[o.page_idx as u32]);
            self.write_str(&o.table_id);
            self.write_f32_slice(&[o.bounds.x, o.bounds.y, o.bounds.width, o.bounds.height]);
            self.write_str(&o.border_style);
            self.write_str(&o.align);
            self.write_f32_slice(&[o.proportion]);
            self.write_u32_slice(&[
                o.start_row_index as u32,
                o.total_rows as u32,
                o.is_focused as u32,
                o.show_cell_selector as u32,
            ]);
            self.write_f32_slice(&[o.content_width, o.min_proportion_width]);
            self.write_f32_slice(&[o.max_proportion_width]);
            self.write_u32_slice(&[o.col_widths_as_px.len() as u32]);
            self.write_f32_slice(&o.col_widths_as_px);
            self.write_u32_slice(&[o.col_widths.len() as u32]);
            self.write_f32_slice(&o.col_widths);
            self.write_u32_slice(&[o.col_positions.len() as u32]);
            self.write_f32_slice(&o.col_positions);
            self.write_u32_slice(&[o.row_heights.len() as u32]);
            self.write_f32_slice(&o.row_heights);
            self.write_u32_slice(&[o.row_positions.len() as u32]);
            self.write_f32_slice(&o.row_positions);
        }
        slate.table_overlays_offset = start;
        slate.table_overlays_count = overlays.len() as u32;
        slate.dirty |= DIRTY_TABLE_OVERLAYS;
    }

    pub fn write_remarks(&mut self, slate: &mut Slate, overlays: &[super::RemarkOverlay]) {
        let start = self.alloc(0, 4);
        for o in overlays {
            self.write_bytes(o.node_id.as_uuid().as_bytes());
            self.write_bytes(o.remark_id.as_uuid().as_bytes());
            self.write_str(&o.user_id);
            self.write_str(&o.text);
            self.write_u32_slice(&[
                (o.created_at as u64 >> 32) as u32,
                (o.created_at as u64 & 0xFFFFFFFF) as u32,
            ]);
            self.write_u32_slice(&[o.page_idx as u32]);
            self.write_f32_slice(&[o.bounds.x, o.bounds.y, o.bounds.width, o.bounds.height]);
        }
        slate.remarks_offset = start;
        slate.remarks_count = overlays.len() as u32;
        slate.dirty |= DIRTY_REMARKS;
    }

    pub fn write_default_attrs(&mut self, slate: &mut Slate, attrs: &DefaultAttrs) {
        let start = self.alloc(0, 4);
        let mut count = 0u32;
        for attr in attrs.attrs() {
            match attr {
                Attr::Style(style) => {
                    self.write_u32_slice(&[
                        style_type_to_tag(&style.as_type()),
                        style_type_to_value_kind(&style.as_type()),
                        1,
                    ]);
                    self.write_style_value(style);
                    count += 1;
                }
                Attr::Paragraph(p) => {
                    self.write_u32_slice(&[ATTR_TAG_LINE_HEIGHT, VK_U32, 1]);
                    self.write_u32_slice(&[p.line_height]);
                    count += 1;
                }
            }
        }
        slate.default_attrs_offset = start;
        slate.default_attrs_count = count;
        slate.dirty |= DIRTY_DEFAULT_ATTRS;
    }
}

fn style_type_to_tag(st: &StyleType) -> u32 {
    match st {
        StyleType::BackgroundColor => ATTR_TAG_BACKGROUND_COLOR,
        StyleType::Bold => ATTR_TAG_BOLD,
        StyleType::TextColor => ATTR_TAG_TEXT_COLOR,
        StyleType::FontSize => ATTR_TAG_FONT_SIZE,
        StyleType::FontFamily => ATTR_TAG_FONT_FAMILY,
        StyleType::FontWeight => ATTR_TAG_FONT_WEIGHT,
        StyleType::Italic => ATTR_TAG_ITALIC,
        StyleType::LetterSpacing => ATTR_TAG_LETTER_SPACING,
        StyleType::Strikethrough => ATTR_TAG_STRIKETHROUGH,
        StyleType::Underline => ATTR_TAG_UNDERLINE,
    }
}

fn style_type_to_value_kind(st: &StyleType) -> u32 {
    match st {
        StyleType::BackgroundColor => VK_STRING,
        StyleType::Bold => VK_UNIT,
        StyleType::TextColor => VK_STRING,
        StyleType::FontSize => VK_U32,
        StyleType::FontFamily => VK_STRING,
        StyleType::FontWeight => VK_U32,
        StyleType::Italic => VK_UNIT,
        StyleType::LetterSpacing => VK_I32,
        StyleType::Strikethrough => VK_UNIT,
        StyleType::Underline => VK_UNIT,
    }
}

fn annotation_type_to_tag(at: &AnnotationType) -> u32 {
    match at {
        AnnotationType::Link => ATTR_TAG_LINK,
        AnnotationType::Ruby => ATTR_TAG_RUBY,
    }
}

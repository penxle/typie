use crate::model::{Annotation, AnnotationType, Style, StyleType, TextAlign};
use crate::state::selection_helpers::{BlockAttr, SelectionAttributes};

pub const DIRTY_SETTINGS: u64 = 1 << 0;
pub const DIRTY_LAYOUT: u64 = 1 << 1;
pub const DIRTY_CURSOR: u64 = 1 << 2;
pub const DIRTY_SELECTION: u64 = 1 << 3;
pub const DIRTY_ATTRS: u64 = 1 << 4;
pub const DIRTY_POINTER: u64 = 1 << 5;
pub const DIRTY_PLACEHOLDER: u64 = 1 << 7;
pub const DIRTY_EXTERNAL_ELEMENTS: u64 = 1 << 8;
pub const DIRTY_ENABLED_ACTIONS: u64 = 1 << 9;
pub const DIRTY_LINK_OVERLAYS: u64 = 1 << 10;
pub const DIRTY_TRACKED_ITEMS: u64 = 1 << 11;
pub const DIRTY_TABLE_OVERLAYS: u64 = 1 << 14;
pub const DIRTY_DOC_CHANGED: u64 = 1 << 15;
pub const DIRTY_RENDER_REQUIRED: u64 = 1 << 16;
pub const DIRTY_FONT_REQUIRED: u64 = 1 << 17;
pub const DIRTY_FALLBACK_FONT_REQUIRED: u64 = 1 << 18;
pub const DIRTY_EXITED_DOCUMENT_START: u64 = 1 << 19;
pub const DIRTY_HTML_PASTED: u64 = 1 << 20;

pub const ATTR_TAG_BACKGROUND_COLOR: u32 = 0;
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

pub const ALIGN_LEFT: u32 = 0;
pub const ALIGN_CENTER: u32 = 1;
pub const ALIGN_RIGHT: u32 = 2;
pub const ALIGN_JUSTIFY: u32 = 3;

#[repr(C)]
pub struct Slate {
    pub dirty: u64,

    pub slab_len: u32,
    pub slab_capacity: u32,

    pub layout_mode_offset: u32,

    pub paragraph_indent: f32,
    pub block_gap: f32,

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
    pub fallback_codepoints_offset: u32,
    pub fallback_codepoints_count: u32,
    pub html_pasted_offset: u32,
    pub html_pasted_len: u32,
}

impl Default for Slate {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
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

    pub fn write_attrs(&mut self, attrs: &SelectionAttributes) -> (u32, u32) {
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
                    self.write_u32_slice(&[ATTR_TAG_LINE_HEIGHT, VK_F32, count]);
                    for val in &collected.values {
                        if let BlockAttr::LineHeight(lh) = val {
                            self.write_f32_slice(&[*lh]);
                        }
                    }
                    if collected.has_absent {
                        self.write_null_sentinel(VK_F32);
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

    pub fn write_text_bounds(&mut self, bounds: &[crate::types::TextBound]) {
        for b in bounds {
            self.write_f32_slice(&[b.x, b.y, b.width, b.height, b.ascent]);
        }
    }

    fn write_style_value(&mut self, style: &Style) {
        match style {
            Style::BackgroundColor(s) => {
                self.write_str(&s.color);
            }
            Style::TextColor(s) => {
                self.write_str(&s.color);
            }
            Style::FontSize(s) => {
                self.write_f32_slice(&[s.size]);
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
                self.write_f32_slice(&[s.spacing]);
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
}

fn style_type_to_tag(st: &StyleType) -> u32 {
    match st {
        StyleType::BackgroundColor => ATTR_TAG_BACKGROUND_COLOR,
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
        StyleType::TextColor => VK_STRING,
        StyleType::FontSize => VK_F32,
        StyleType::FontFamily => VK_STRING,
        StyleType::FontWeight => VK_U32,
        StyleType::Italic => VK_UNIT,
        StyleType::LetterSpacing => VK_F32,
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

pub fn get_slate_offsets() -> Vec<(&'static str, usize)> {
    let mut offsets = vec![
        ("dirty", std::mem::offset_of!(Slate, dirty)),
        ("slab_len", std::mem::offset_of!(Slate, slab_len)),
        ("slab_capacity", std::mem::offset_of!(Slate, slab_capacity)),
        (
            "paragraph_indent",
            std::mem::offset_of!(Slate, paragraph_indent),
        ),
        ("block_gap", std::mem::offset_of!(Slate, block_gap)),
        ("pages_offset", std::mem::offset_of!(Slate, pages_offset)),
        ("pages_count", std::mem::offset_of!(Slate, pages_count)),
        (
            "layout_mode_offset",
            std::mem::offset_of!(Slate, layout_mode_offset),
        ),
        (
            "cursor_page_idx",
            std::mem::offset_of!(Slate, cursor_page_idx),
        ),
        ("cursor_x", std::mem::offset_of!(Slate, cursor_x)),
        ("cursor_y", std::mem::offset_of!(Slate, cursor_y)),
        ("cursor_width", std::mem::offset_of!(Slate, cursor_width)),
        ("cursor_height", std::mem::offset_of!(Slate, cursor_height)),
        (
            "cursor_visible",
            std::mem::offset_of!(Slate, cursor_visible),
        ),
        (
            "preceding_char_widths_offset",
            std::mem::offset_of!(Slate, preceding_char_widths_offset),
        ),
        (
            "preceding_char_widths_count",
            std::mem::offset_of!(Slate, preceding_char_widths_count),
        ),
        ("selection_cmp", std::mem::offset_of!(Slate, selection_cmp)),
        (
            "selection_anchor_node_id",
            std::mem::offset_of!(Slate, selection_anchor_node_id),
        ),
        (
            "selection_anchor_offset",
            std::mem::offset_of!(Slate, selection_anchor_offset),
        ),
        (
            "selection_anchor_affinity",
            std::mem::offset_of!(Slate, selection_anchor_affinity),
        ),
        (
            "selection_head_node_id",
            std::mem::offset_of!(Slate, selection_head_node_id),
        ),
        (
            "selection_head_offset",
            std::mem::offset_of!(Slate, selection_head_offset),
        ),
        (
            "selection_head_affinity",
            std::mem::offset_of!(Slate, selection_head_affinity),
        ),
        (
            "selection_anchor_page_idx",
            std::mem::offset_of!(Slate, selection_anchor_page_idx),
        ),
        (
            "selection_anchor_x",
            std::mem::offset_of!(Slate, selection_anchor_x),
        ),
        (
            "selection_anchor_y",
            std::mem::offset_of!(Slate, selection_anchor_y),
        ),
        (
            "selection_anchor_width",
            std::mem::offset_of!(Slate, selection_anchor_width),
        ),
        (
            "selection_anchor_height",
            std::mem::offset_of!(Slate, selection_anchor_height),
        ),
        (
            "selection_head_page_idx",
            std::mem::offset_of!(Slate, selection_head_page_idx),
        ),
        (
            "selection_head_x",
            std::mem::offset_of!(Slate, selection_head_x),
        ),
        (
            "selection_head_y",
            std::mem::offset_of!(Slate, selection_head_y),
        ),
        (
            "selection_head_width",
            std::mem::offset_of!(Slate, selection_head_width),
        ),
        (
            "selection_head_height",
            std::mem::offset_of!(Slate, selection_head_height),
        ),
        ("attrs_offset", std::mem::offset_of!(Slate, attrs_offset)),
        ("attrs_count", std::mem::offset_of!(Slate, attrs_count)),
        ("pointer_style", std::mem::offset_of!(Slate, pointer_style)),
        ("pointer_state", std::mem::offset_of!(Slate, pointer_state)),
        (
            "placeholder_visible",
            std::mem::offset_of!(Slate, placeholder_visible),
        ),
        ("placeholder_x", std::mem::offset_of!(Slate, placeholder_x)),
        ("placeholder_y", std::mem::offset_of!(Slate, placeholder_y)),
        (
            "placeholder_width",
            std::mem::offset_of!(Slate, placeholder_width),
        ),
        (
            "placeholder_height",
            std::mem::offset_of!(Slate, placeholder_height),
        ),
        (
            "enabled_actions_offset",
            std::mem::offset_of!(Slate, enabled_actions_offset),
        ),
        (
            "enabled_actions_count",
            std::mem::offset_of!(Slate, enabled_actions_count),
        ),
        (
            "external_elements_offset",
            std::mem::offset_of!(Slate, external_elements_offset),
        ),
        (
            "external_elements_count",
            std::mem::offset_of!(Slate, external_elements_count),
        ),
        (
            "link_overlays_offset",
            std::mem::offset_of!(Slate, link_overlays_offset),
        ),
        (
            "link_overlays_count",
            std::mem::offset_of!(Slate, link_overlays_count),
        ),
        (
            "tracked_items_offset",
            std::mem::offset_of!(Slate, tracked_items_offset),
        ),
        (
            "tracked_items_count",
            std::mem::offset_of!(Slate, tracked_items_count),
        ),
        (
            "table_overlays_offset",
            std::mem::offset_of!(Slate, table_overlays_offset),
        ),
        (
            "table_overlays_count",
            std::mem::offset_of!(Slate, table_overlays_count),
        ),
        (
            "font_requests_offset",
            std::mem::offset_of!(Slate, font_requests_offset),
        ),
        (
            "font_requests_count",
            std::mem::offset_of!(Slate, font_requests_count),
        ),
        (
            "fallback_codepoints_offset",
            std::mem::offset_of!(Slate, fallback_codepoints_offset),
        ),
        (
            "fallback_codepoints_count",
            std::mem::offset_of!(Slate, fallback_codepoints_count),
        ),
        (
            "html_pasted_offset",
            std::mem::offset_of!(Slate, html_pasted_offset),
        ),
        (
            "html_pasted_len",
            std::mem::offset_of!(Slate, html_pasted_len),
        ),
    ];
    offsets.sort_by_key(|&(_, off)| off);
    offsets
}

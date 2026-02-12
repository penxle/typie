pub const DIRTY_SETTINGS: u64 = 1 << 0;
pub const DIRTY_LAYOUT: u64 = 1 << 1;
pub const DIRTY_CURSOR: u64 = 1 << 2;
pub const DIRTY_SELECTION: u64 = 1 << 3;
pub const DIRTY_FORMATTING: u64 = 1 << 4;
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

    pub formatting_uniform_align: i32,
    pub formatting_uniform_line_height: f32,
    pub formatting_uniform_styles_offset: u32,
    pub formatting_uniform_styles_count: u32,
    pub formatting_mixed_styles_bitfield: u32,

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
        (
            "formatting_uniform_align",
            std::mem::offset_of!(Slate, formatting_uniform_align),
        ),
        (
            "formatting_uniform_line_height",
            std::mem::offset_of!(Slate, formatting_uniform_line_height),
        ),
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
            "formatting_uniform_styles_offset",
            std::mem::offset_of!(Slate, formatting_uniform_styles_offset),
        ),
        (
            "formatting_uniform_styles_count",
            std::mem::offset_of!(Slate, formatting_uniform_styles_count),
        ),
        (
            "formatting_mixed_styles_bitfield",
            std::mem::offset_of!(Slate, formatting_mixed_styles_bitfield),
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

// Vendored from swash 0.2.6 (MIT/Apache-2.0)
// COLR/CPAL color outline support.

use super::super::internal::{Bytes, RawTag, raw_tag};

const COLR: RawTag = raw_tag(b"COLR");
const CPAL: RawTag = raw_tag(b"CPAL");

#[derive(Copy, Clone, Default)]
pub struct ColorProxy {
    pub colr: u32,
    pub cpal: u32,
}

impl ColorProxy {
    /// Constructs a ColorProxy by scanning the font table directory for COLR/CPAL offsets.
    pub fn from_font_data(data: &[u8], table_dir_offset: u32) -> Self {
        let offset_of = |tag: RawTag| -> u32 {
            let b = Bytes::new(data);
            let base = table_dir_offset as usize;
            let len = b.read_u16(base + 4).unwrap_or(0) as usize;
            let record_base = base + 12;
            let mut l = 0;
            let mut h = len;
            while l < h {
                let i = (l + h) / 2;
                let recbase = i * 16 + record_base;
                let table_tag = b.read_u32(recbase).unwrap_or(0);
                use core::cmp::Ordering::*;
                match tag.cmp(&table_tag) {
                    Less => h = i,
                    Greater => l = i + 1,
                    Equal => {
                        return b.read_u32(recbase + 8).unwrap_or(0);
                    }
                }
            }
            0
        };
        Self {
            colr: offset_of(COLR),
            cpal: offset_of(CPAL),
        }
    }

    pub fn layers<'a>(&self, data: &'a [u8], glyph_id: u16) -> Option<Layers<'a>> {
        let b = Bytes::with_offset(data, self.colr as usize)?;
        let count = b.read::<u16>(2)? as usize;
        let base_offset = b.read::<u32>(4)? as usize;
        let mut l = 0;
        let mut h = count;
        while l < h {
            use core::cmp::Ordering::*;
            let i = l + (h - l) / 2;
            let rec = base_offset + i * 6;
            let id = b.read::<u16>(rec)?;
            match glyph_id.cmp(&id) {
                Less => h = i,
                Greater => l = i + 1,
                Equal => {
                    let first = b.read::<u16>(rec + 2)? as usize;
                    let offset = b.read::<u32>(8)? as usize + first * 4;
                    let len = b.read::<u16>(rec + 4)?;
                    return Some(Layers {
                        data: b,
                        offset,
                        len,
                    });
                }
            }
        }
        None
    }

    pub fn palette<'a>(&self, data: &'a [u8], index: u16) -> Option<ColorPalette<'a>> {
        if self.cpal != 0 {
            ColorPalettes::from_data_and_offset(data, self.cpal).nth(index as usize)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone)]
pub struct Layers<'a> {
    data: Bytes<'a>,
    offset: usize,
    len: u16,
}

impl<'a> Layers<'a> {
    pub fn len(&self) -> u16 {
        self.len
    }

    pub fn get(&self, index: u16) -> Option<ColorLayer> {
        let b = &self.data;
        let base = self.offset + index as usize * 4;
        let glyph_id = b.read::<u16>(base)?;
        let color_index = b.read::<u16>(base + 2)?;
        Some(ColorLayer {
            glyph_id,
            color_index: if color_index != 0xFFFF {
                Some(color_index)
            } else {
                None
            },
        })
    }
}

#[derive(Copy, Clone)]
pub struct ColorLayer {
    pub glyph_id: u16,
    pub color_index: Option<u16>,
}

// Simplified CPAL palette support (inlined from swash palette.rs)

#[derive(Copy, Clone)]
struct ColorPalettes<'a> {
    data: Bytes<'a>,
    len: usize,
    pos: usize,
}

impl<'a> ColorPalettes<'a> {
    fn from_data_and_offset(data: &'a [u8], offset: u32) -> Self {
        let d = data.get(offset as usize..).unwrap_or(&[]);
        let b = Bytes::new(d);
        let len = b.read_or_default::<u16>(4) as usize;
        Self {
            data: b,
            len,
            pos: 0,
        }
    }

    fn get(&self, index: usize) -> Option<ColorPalette<'a>> {
        if index >= self.len {
            return None;
        }
        let b = &self.data;
        let _num_entries = b.read::<u16>(2)?;
        let offset = b.read::<u32>(8)? as usize;
        let first = b.read::<u16>(12 + index * 2)? as usize;
        let offset = offset + first * 4;
        Some(ColorPalette {
            data: *b,
            num_entries: _num_entries,
            offset,
        })
    }
}

impl<'a> Iterator for ColorPalettes<'a> {
    type Item = ColorPalette<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.len {
            return None;
        }
        let result = self.get(self.pos);
        self.pos += 1;
        result
    }
}

/// Collection of colors from a CPAL palette.
#[derive(Copy, Clone)]
pub struct ColorPalette<'a> {
    data: Bytes<'a>,
    num_entries: u16,
    offset: usize,
}

impl<'a> ColorPalette<'a> {
    /// Returns the color for the specified entry in RGBA order.
    pub fn get(&self, index: u16) -> [u8; 4] {
        if index >= self.num_entries {
            return [0; 4];
        }
        let offset = self.offset + index as usize * 4;
        let d = &self.data;
        let b = d.read_or_default::<u8>(offset);
        let g = d.read_or_default::<u8>(offset + 1);
        let r = d.read_or_default::<u8>(offset + 2);
        let a = d.read_or_default::<u8>(offset + 3);
        [r, g, b, a]
    }
}

use hashbrown::HashMap;

use crate::glyph::RasterizedGlyph;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub font_id: u16,
    pub glyph_id: u32,
    pub size_q4: u32,
    pub has_skew: bool,
    pub embolden: bool,
}

impl GlyphCacheKey {
    pub fn new(
        font_id: u16,
        glyph_id: u32,
        font_size: f32,
        has_skew: bool,
        embolden: bool,
    ) -> Self {
        Self {
            font_id,
            glyph_id,
            size_q4: (font_size * 4.0).round() as u32,
            has_skew,
            embolden,
        }
    }
}

struct CachedGlyph {
    result: Option<RasterizedGlyph>,
    font_version: u64,
}

pub struct GlyphCache {
    map: HashMap<GlyphCacheKey, CachedGlyph>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Look up a cached glyph result; `None` entries are only valid when `font_version` matches
    /// because a new font chunk may have populated the glyph data since the miss was recorded.
    pub fn get(
        &self,
        key: &GlyphCacheKey,
        current_font_version: u64,
    ) -> Option<&Option<RasterizedGlyph>> {
        let cached = self.map.get(key)?;
        if cached.result.is_some() || cached.font_version == current_font_version {
            Some(&cached.result)
        } else {
            None
        }
    }

    pub fn insert(
        &mut self,
        key: GlyphCacheKey,
        result: Option<RasterizedGlyph>,
        font_version: u64,
    ) {
        self.map.insert(
            key,
            CachedGlyph {
                result,
                font_version,
            },
        );
    }
}

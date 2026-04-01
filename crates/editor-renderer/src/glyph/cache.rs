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

    /// Look up a cached glyph result.
    ///
    /// For `Some` results, the cache entry is always valid (chunk writes
    /// never overwrite previously populated glyph data).
    ///
    /// For `None` results, the entry is only valid if the stored version
    /// matches `current_font_version`. A version mismatch means a new
    /// chunk may have loaded the glyph data — treat as cache miss.
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

use hashbrown::HashMap;

use crate::glyph::RasterizedGlyph;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub family_id: u16,
    pub weight: u16,
    pub glyph_id: u32,
    pub size_q4: u32,
    pub has_skew: bool,
    pub embolden: bool,
    pub subpixel_x: u8,
}

impl GlyphCacheKey {
    pub fn new(
        family_id: u16,
        weight: u16,
        glyph_id: u32,
        font_size: f32,
        has_skew: bool,
        embolden: bool,
        subpixel_x: u8,
    ) -> Self {
        debug_assert!(subpixel_x < 4);
        Self {
            family_id,
            weight,
            glyph_id,
            size_q4: (font_size * 4.0).round() as u32,
            has_skew,
            embolden,
            subpixel_x,
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

    /// `None` 엔트리는 font_version 이 일치할 때만 유효하다. 새 폰트 청크가 들어오면
    /// miss 가 기록된 이후 glyph 데이터가 채워졌을 수 있으므로 재시도한다.
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

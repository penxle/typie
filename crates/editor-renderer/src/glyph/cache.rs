use hashbrown::HashMap;

use crate::glyph::{GlyphKey, RasterizedGlyph, SvgPathGlyph};
use crate::types::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

struct CachedSvgPathGlyph {
    result: Option<SvgPathGlyph>,
    font_version: u64,
}

pub struct GlyphCache {
    map: HashMap<GlyphCacheKey, CachedGlyph>,
}

pub struct SvgPathGlyphCache {
    map: HashMap<GlyphCacheKey, CachedSvgPathGlyph>,
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

impl SvgPathGlyphCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(
        &self,
        key: &GlyphCacheKey,
        current_font_version: u64,
    ) -> Option<&Option<SvgPathGlyph>> {
        let cached = self.map.get(key)?;
        if cached.result.is_some() || cached.font_version == current_font_version {
            Some(&cached.result)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: GlyphCacheKey, result: Option<SvgPathGlyph>, font_version: u64) {
        self.map.insert(
            key,
            CachedSvgPathGlyph {
                result,
                font_version,
            },
        );
    }
}

const BAKED_GLYPH_CACHE_CAP: usize = 4096;

pub struct BakedGlyphCache {
    current: HashMap<GlyphKey, Image>,
    previous: HashMap<GlyphKey, Image>,
    cap: usize,
}

impl BakedGlyphCache {
    pub fn new() -> Self {
        Self::with_cap(BAKED_GLYPH_CACHE_CAP)
    }

    fn with_cap(cap: usize) -> Self {
        Self {
            current: HashMap::new(),
            previous: HashMap::new(),
            cap,
        }
    }

    pub fn get_or_bake(&mut self, key: GlyphKey, bake: impl FnOnce() -> Image) -> Image {
        if let Some(img) = self.current.get(&key) {
            return img.clone();
        }
        let img = match self.previous.remove(&key) {
            Some(img) => img,
            None => bake(),
        };
        if self.current.len() >= self.cap {
            self.previous = std::mem::take(&mut self.current);
        }
        self.current.insert(key, img.clone());
        img
    }
}

impl Default for BakedGlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{BakedGlyphCache, GlyphCacheKey, SvgPathGlyphCache};
    use crate::glyph::{GlyphKey, SvgPathGlyph};
    use crate::types::{Color, Image, Path, PathElement};

    #[test]
    fn svg_path_cache_preserves_svg_path_glyph() {
        // SVG path glyph cache 가 SVG path 기반 글리프 표현을 그대로 저장하고
        // 다시 꺼낼 수 있는지 확인한다.
        let mut cache = SvgPathGlyphCache::new();
        let key = GlyphCacheKey::new(1, 400, 42, 16.0, false, false, 0);
        let glyph = SvgPathGlyph {
            path: Path {
                elements: vec![
                    PathElement::MoveTo { x: 0.0, y: 0.0 },
                    PathElement::LineTo { x: 1.0, y: 0.0 },
                    PathElement::Close,
                ],
            },
            placement_left: 3,
            placement_top: 4,
        };

        cache.insert(key, Some(glyph), 7);

        let cached = cache
            .get(&key, 7)
            .expect("cache entry must exist")
            .as_ref()
            .expect("cache entry must contain glyph");

        assert_eq!(cached.placement_left, 3);
        assert_eq!(cached.placement_top, 4);
        assert_eq!(cached.path.elements.len(), 3);
    }

    fn baked_glyph_key(id: u32) -> GlyphKey {
        GlyphKey {
            cache_key: GlyphCacheKey::new(1, 400, id, 16.0, false, false, 0),
            color: Color::rgb(0, 0, 0),
            font_generation: 0,
        }
    }

    fn baked_image(byte: u8) -> Image {
        Image {
            data: vec![byte; 4].into(),
            width: 1,
            height: 1,
            glyph: None,
        }
    }

    #[test]
    fn baked_glyph_cache_survives_rotation_via_previous_generation() {
        let mut cache = BakedGlyphCache::with_cap(2);
        let mut bakes = 0;

        let key1 = baked_glyph_key(1);
        let key2 = baked_glyph_key(2);
        let key3 = baked_glyph_key(3);

        cache.get_or_bake(key1, || {
            bakes += 1;
            baked_image(1)
        });
        cache.get_or_bake(key2, || {
            bakes += 1;
            baked_image(2)
        });
        cache.get_or_bake(key3, || {
            bakes += 1;
            baked_image(3)
        });
        assert_eq!(bakes, 3, "3 distinct keys must each bake once");

        cache.get_or_bake(key1, || {
            bakes += 1;
            baked_image(1)
        });
        assert_eq!(
            bakes, 3,
            "key #1 must survive rotation in the previous generation (no re-bake)"
        );

        cache.get_or_bake(key1, || {
            bakes += 1;
            baked_image(1)
        });
        assert_eq!(
            bakes, 3,
            "repeated current-generation hits must not re-bake"
        );

        assert!(cache.current.len() + cache.previous.len() <= 2 * cache.cap);
    }

    #[test]
    fn baked_glyph_cache_second_render_adds_zero_bakes() {
        let mut cache = BakedGlyphCache::new();
        let mut bakes = 0;
        let key = baked_glyph_key(42);

        cache.get_or_bake(key, || {
            bakes += 1;
            baked_image(9)
        });
        assert_eq!(bakes, 1);

        cache.get_or_bake(key, || {
            bakes += 1;
            baked_image(9)
        });
        assert_eq!(bakes, 1, "second lookup of the same key must hit the cache");
    }
}

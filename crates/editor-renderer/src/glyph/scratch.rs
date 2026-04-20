use zeno::Scratch;

/// 마스크·비트맵 생성 과정에서 재사용하는 워크 버퍼.
pub struct GlyphScratch {
    pub zeno: Scratch,
    pub bitmap_0: Vec<u8>,
    pub bitmap_1: Vec<u8>,
}

impl GlyphScratch {
    pub fn new() -> Self {
        Self {
            zeno: Scratch::new(),
            bitmap_0: Vec::new(),
            bitmap_1: Vec::new(),
        }
    }
}

impl Default for GlyphScratch {
    fn default() -> Self {
        Self::new()
    }
}

use crate::global::GLOBALS;
use crate::render::glyph::outline::outline_to_bezpath;
use crate::render::glyph::scale::image::Image;
use crate::render::glyph::scale::{Render, Source, StrikeWith};
use crate::render::glyph::{EMBOLDEN_RATIO, Glyph, calculate_font_hash};
use kurbo::{Affine, BezPath};
use parley::FontData;
use peniko::{Brush, Fill};
use rustc_hash::FxHashMap;
use skrifa::{FontRef, GlyphId};
use zeno::Vector;

// ── GlyphCache ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphCacheKey {
    font_hash: u64,
    glyph_id: u32,
    size_q4: u32,
    has_skew: bool,
    embolden: bool,
    subpixel_x: u8,
    subpixel_y: u8,
}

/// 래스터라이즈 결과를 프레임 간 캐싱한다.
/// thread-local GLOBALS에 보관되어 ScaleContext와 동일한 수명을 가진다.
pub struct GlyphCache {
    map: FxHashMap<GlyphCacheKey, GlyphResult>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }
}

// ── RasterizedGlyph ──────────────────────────────────────────────────

/// 래스터라이즈된 글리프 하나를 나타낸다.
/// - `Path`: 아웃라인에서 변환된 BezPath (Outline 소스)
/// - `Bitmap`: detect_source로 디코딩된 이미지 (ColorOutline/Bitmap/ColorBitmap 소스)
pub enum RasterizedGlyph<'a> {
    Path {
        path: BezPath,
        brush: &'a Brush,
        fill: Fill,
        transform: Affine,
    },
    Bitmap {
        image: Image,
        x: f32,
        y: f32,
    },
}

// ── rasterize_glyphs ─────────────────────────────────────────────────

/// 글리프 목록을 래스터라이즈하여 emit 콜백으로 전달한다.
///
/// ScaleContext와 GlyphCache는 GLOBALS thread-local에서 가져온다.
/// emit 콜백 내에서 GLOBALS를 접근하지 않으므로 RefCell 충돌 없음.
pub fn rasterize_glyphs<'a>(
    font: &FontData,
    font_size: f32,
    brush: &'a Brush,
    transform: Affine,
    glyph_transform: Option<Affine>,
    embolden: bool,
    glyphs: &[Glyph],
    mut emit: impl FnMut(RasterizedGlyph<'a>),
) {
    let font_data = font.data.as_ref();
    let font_hash = calculate_font_hash(font_data);
    let size_q4 = (font_size * 4.0).round() as u32;
    let quantized_size = size_q4 as f32 / 4.0;
    let has_skew = glyph_transform.is_some();

    let font_ref = match FontRef::new(font_data) {
        Ok(f) => f,
        Err(_) => return,
    };

    let coeffs = transform.as_coeffs();
    let tx = coeffs[4] as f32;
    let ty = coeffs[5] as f32;
    let sx = coeffs[0] as f32;
    let sy = coeffs[3] as f32;

    let embolden_amount = if embolden {
        quantized_size * EMBOLDEN_RATIO
    } else {
        0.0
    };

    let render_sources = if has_skew || embolden {
        [
            Source::ColorOutline(0),
            Source::Outline,
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Bitmap(StrikeWith::BestFit),
        ]
    } else {
        [
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
            Source::Bitmap(StrikeWith::BestFit),
        ]
    };

    let skew_transform = glyph_transform.map(|a| {
        let c = a.as_coeffs();
        zeno::Transform {
            xx: c[0] as f32,
            yx: c[2] as f32,
            xy: c[1] as f32,
            yy: c[3] as f32,
            x: 0.0,
            y: 0.0,
        }
    });

    for glyph in glyphs {
        if glyph.id == 0 {
            continue;
        }

        let glyph_x = tx + glyph.x * sx;
        let glyph_y = ty + glyph.y * sy;
        let snapped_x = glyph_x.floor();

        let glyph_id = GlyphId::new(glyph.id);
        let id = [font_hash, 0];

        let cache_key = GlyphCacheKey {
            font_hash,
            glyph_id: glyph.id,
            size_q4,
            has_skew,
            embolden,
            subpixel_x: 0,
            subpixel_y: 0,
        };

        // 캐시 히트 확인 — GLOBALS borrow 스코프를 emit 콜백 전에 닫는다.
        let result = GLOBALS.with(|globals| {
            let globals = globals.borrow();

            // 캐시 조회
            if let Some(cached) = globals.glyph_cache.borrow().map.get(&cache_key) {
                return Some(cached.with_position(snapped_x, glyph_y.floor()));
            }

            // 캐시 미스 — 래스터라이즈
            let mut scale_context = globals.scale_context.borrow_mut();

            let mut image = Image::new();
            let mut scaler = scale_context
                .builder(font_ref.clone(), id)
                .size(quantized_size)
                .hint(true)
                .build();

            let detected = Render::new(&render_sources)
                .offset(Vector::new(0.0, 0.0))
                .embolden(embolden_amount)
                .transform(skew_transform)
                .detect_source(&mut scaler, glyph_id, &mut image);

            if !detected {
                return None;
            }

            let result = match image.source {
                Source::Outline => {
                    let mut scaler2 = scale_context
                        .builder(font_ref.clone(), id)
                        .size(quantized_size)
                        .hint(true)
                        .build();
                    if let Some(outline) =
                        scaler2.scale_outline(glyph_id, skew_transform, embolden_amount)
                    {
                        let bezpath = outline_to_bezpath(&outline, 0.0, 0.0);
                        GlyphResult::Path(bezpath)
                    } else {
                        return None;
                    }
                }
                Source::ColorOutline(_) | Source::Bitmap(_) | Source::ColorBitmap(_) => {
                    GlyphResult::Bitmap(image)
                }
            };

            // 캐시에 저장
            globals
                .glyph_cache
                .borrow_mut()
                .map
                .insert(cache_key, result.clone());

            Some(result.with_position(snapped_x, glyph_y.floor()))
        });

        if let Some(result) = result {
            match result {
                PositionedGlyphResult::Path(path, x, y) => {
                    emit(RasterizedGlyph::Path {
                        path,
                        brush,
                        fill: Fill::NonZero,
                        transform: Affine::translate((x as f64, y as f64)),
                    });
                }
                PositionedGlyphResult::Bitmap(image, x, y) => {
                    emit(RasterizedGlyph::Bitmap { image, x, y });
                }
            }
        }
    }
}

// ── 중간 결과 타입 ───────────────────────────────────────────────────

/// 캐시에 저장되는 위치 무관(position-independent) 결과.
/// Path의 좌표는 (0,0) 기준으로 기록되고, emit 시 transform으로 오프셋을 적용한다.
/// Bitmap의 placement 데이터는 그대로 캐싱하고, 위치만 emit 시 계산한다.
#[derive(Clone)]
enum GlyphResult {
    Path(BezPath),
    Bitmap(Image),
}

impl GlyphResult {
    fn with_position(&self, x: f32, y: f32) -> PositionedGlyphResult {
        match self {
            GlyphResult::Path(path) => PositionedGlyphResult::Path(path.clone(), x, y),
            GlyphResult::Bitmap(image) => PositionedGlyphResult::Bitmap(image.clone(), x, y),
        }
    }
}

/// emit 콜백에 전달하기 위한 위치가 포함된 결과.
enum PositionedGlyphResult {
    Path(BezPath, f32, f32),
    Bitmap(Image, f32, f32),
}

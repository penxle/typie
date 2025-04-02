use js_sys::Uint8Array;
use swash::{
  CacheKey, FontRef, Setting,
  scale::{Render, ScaleContext, Source, StrikeWith},
};
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct GlyphBitmap {
  width: u32,
  height: u32,
  top: i32,
  left: i32,
  buffer: Vec<u8>,
}

#[wasm_bindgen]
impl GlyphBitmap {
  #[wasm_bindgen(getter)]
  pub fn width(&self) -> u32 {
    self.width as u32
  }

  #[wasm_bindgen(getter)]
  pub fn height(&self) -> u32 {
    self.height as u32
  }

  #[wasm_bindgen(getter)]
  pub fn top(&self) -> i32 {
    self.top
  }

  #[wasm_bindgen(getter)]
  pub fn left(&self) -> i32 {
    self.left
  }

  #[wasm_bindgen(getter)]
  pub fn buffer(&self) -> Uint8Array {
    let buffer = Uint8Array::new_with_length(self.buffer.len() as u32);
    buffer.copy_from(&self.buffer);
    buffer
  }
}

#[wasm_bindgen]
pub struct GlyphMetrics {
  advance_width: f32,
  advance_height: f32,
  lsb: f32,
  tsb: f32,
  vertical_origin: f32,
}

#[wasm_bindgen]
impl GlyphMetrics {
  #[wasm_bindgen(getter)]
  pub fn advance_width(&self) -> f32 {
    self.advance_width
  }

  #[wasm_bindgen(getter)]
  pub fn advance_height(&self) -> f32 {
    self.advance_height
  }

  #[wasm_bindgen(getter)]
  pub fn lsb(&self) -> f32 {
    self.lsb
  }

  #[wasm_bindgen(getter)]
  pub fn tsb(&self) -> f32 {
    self.tsb
  }

  #[wasm_bindgen(getter)]
  pub fn vertical_origin(&self) -> f32 {
    self.vertical_origin
  }
}

#[wasm_bindgen]
pub struct Glyphr {
  font: Option<Font>,
}

pub struct Font {
  data: Vec<u8>,
  offset: u32,
  key: CacheKey,
}

impl Font {
  pub fn from_bytes(data: &[u8]) -> Option<Self> {
    let font = FontRef::from_index(data, 0)?;

    Some(Self {
      data: data.to_vec(),
      offset: font.offset,
      key: font.key,
    })
  }

  pub fn as_ref(&self) -> FontRef {
    FontRef {
      data: &self.data,
      offset: self.offset,
      key: self.key,
    }
  }
}

#[wasm_bindgen]
impl Glyphr {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    console_error_panic_hook::set_once();

    Self { font: None }
  }

  #[wasm_bindgen]
  pub fn load_font(&mut self, font_data: &[u8]) -> Result<(), JsError> {
    let font = Font::from_bytes(font_data).ok_or(JsError::new("Invalid font data"))?;
    self.font = Some(font);

    Ok(())
  }

  #[wasm_bindgen]
  pub fn get_metrics(&self, char_code: u32) -> Result<GlyphMetrics, JsError> {
    let font = self
      .font
      .as_ref()
      .ok_or(JsError::new("Font not loaded"))?
      .as_ref();

    let normalized_coords: Vec<_> = font
      .variations()
      .normalized_coords([Setting::default()])
      .collect();
    let metrics = font.glyph_metrics(&normalized_coords).scale(32.0);

    let glyph_id = font.charmap().map(char_code);

    let advance_width = metrics.advance_width(glyph_id);
    let advance_height = metrics.advance_height(glyph_id);
    let lsb = metrics.lsb(glyph_id);
    let tsb = metrics.tsb(glyph_id);
    let vertical_origin = metrics.vertical_origin(glyph_id);

    Ok(GlyphMetrics {
      advance_width,
      advance_height,
      lsb,
      tsb,
      vertical_origin,
    })
  }

  #[wasm_bindgen]
  pub fn render_glyph(&self, char_code: u32) -> Result<GlyphBitmap, JsError> {
    let font = self
      .font
      .as_ref()
      .ok_or(JsError::new("Font not loaded"))?
      .as_ref();

    let glyph_id = font.charmap().map(char_code);

    if glyph_id == 0 {
      return Err(JsError::new("Invalid glyph id"));
    }

    let mut context = ScaleContext::new();
    let mut scaler = context.builder(font).size(32.0).build();

    let sources = [
      Source::ColorBitmap(StrikeWith::BestFit),
      Source::ColorOutline(0),
      Source::Outline,
    ];

    let image = Render::new(&sources).render(&mut scaler, glyph_id);

    if let Some(image) = image {
      Ok(GlyphBitmap {
        width: image.placement.width,
        height: image.placement.height,
        top: image.placement.top,
        left: image.placement.left,
        buffer: image.data,
      })
    } else {
      Ok(GlyphBitmap {
        width: 0,
        height: 0,
        top: 0,
        left: 0,
        buffer: vec![],
      })
    }
  }
}

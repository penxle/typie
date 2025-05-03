use napi::bindgen_prelude::*;
use napi_derive::napi;
use ttf_parser::{Face, Style, name_id};

#[cxx::bridge]
mod ffi {
  unsafe extern "C++" {
    include!("wrapper.h");

    fn compress_woff2(data: &[u8]) -> Vec<u8>;
  }
}

#[napi(object)]
pub struct Font {
  pub weight: u16,

  #[napi(ts_type = "'normal' | 'italic' | 'oblique'")]
  pub style: &'static str,

  #[napi(js_name = "familyName")]
  pub family_name: Option<String>,

  #[napi(js_name = "fullName")]
  pub full_name: Option<String>,

  #[napi(js_name = "postScriptName")]
  pub post_script_name: Option<String>,
}

#[napi(js_name = "getFontMetadata")]
pub fn get_font_metadata(data: Uint8Array) -> Result<Font> {
  let face = Face::parse(data.as_ref(), 0).map_err(|err| Error::from_reason(err.to_string()))?;

  let weight = face.weight().to_number();
  let style = match face.style() {
    Style::Normal => "normal",
    Style::Italic => "italic",
    Style::Oblique => "oblique",
  };

  let family_name = face
    .names()
    .into_iter()
    .find(|name| name.name_id == name_id::TYPOGRAPHIC_FAMILY)
    .and_then(|name| name.to_string())
    .or_else(|| {
      face
        .names()
        .into_iter()
        .find(|name| name.name_id == name_id::FAMILY)
        .and_then(|name| name.to_string())
    });

  let full_name = face
    .names()
    .into_iter()
    .find(|name| name.name_id == name_id::FULL_NAME)
    .and_then(|name| name.to_string());

  let post_script_name = face
    .names()
    .into_iter()
    .find(|name| name.name_id == name_id::POST_SCRIPT_NAME)
    .and_then(|name| name.to_string());

  Ok(Font {
    weight,
    style,
    family_name,
    full_name,
    post_script_name,
  })
}

#[napi(js_name = "toWoff2")]
pub fn to_woff2(data: Uint8Array) -> Result<Uint8Array> {
  let input = data.as_ref();
  let output = ffi::compress_woff2(input);

  if output.is_empty() {
    Err(Error::from_reason("Failed to convert font to WOFF2 format"))
  } else {
    Ok(output.into())
  }
}

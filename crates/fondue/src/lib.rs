use napi::bindgen_prelude::*;
use napi_derive::napi;
use ttf_parser::{Face, Language, Style, name_id};

#[cxx::bridge]
mod ffi {
  unsafe extern "C++" {
    include!("wrapper.h");

    fn compress_woff2(data: &[u8]) -> Vec<u8>;
  }
}

#[napi(object)]
pub struct FontMetadata {
  pub weight: u16,
  #[napi(ts_type = "'normal' | 'italic' | 'oblique'")]
  pub style: String,
  pub family_name: Option<String>,
  pub full_name: Option<String>,
  pub post_script_name: Option<String>,
}

fn get_name(face: &Face, name_id: u16) -> Option<String> {
  let languages = [
    Some(Language::Korean_Korea),
    Some(Language::English_UnitedStates),
    None,
  ];

  for &language in &languages {
    let result = face
      .names()
      .into_iter()
      .find(|name| {
        if let Some(language) = language {
          name.language() == language && name.name_id == name_id
        } else {
          name.name_id == name_id
        }
      })
      .and_then(|name| {
        name
          .to_string()
          .or_else(|| String::from_utf8(name.name.to_vec()).ok())
      });

    if result.is_some() {
      return result;
    }
  }

  None
}

#[napi]
pub fn get_font_metadata(data: Uint8Array) -> Result<FontMetadata> {
  let face = Face::parse(data.as_ref(), 0).map_err(|err| Error::from_reason(err.to_string()))?;

  let weight = face.weight().to_number();
  let style = match face.style() {
    Style::Normal => "normal",
    Style::Italic => "italic",
    Style::Oblique => "oblique",
  }
  .to_string();

  let family_name =
    get_name(&face, name_id::TYPOGRAPHIC_FAMILY).or_else(|| get_name(&face, name_id::FAMILY));
  let full_name = get_name(&face, name_id::FULL_NAME);
  let post_script_name = get_name(&face, name_id::POST_SCRIPT_NAME);

  Ok(FontMetadata {
    weight,
    style,
    family_name,
    full_name,
    post_script_name,
  })
}

#[napi]
pub fn to_woff2(data: Uint8Array) -> Result<Uint8Array> {
  let input = data.as_ref();
  let output = ffi::compress_woff2(input);

  if output.is_empty() {
    Err(Error::from_reason("Failed to convert font to WOFF2 format"))
  } else {
    Ok(output.into())
  }
}

use editor_macros::ffi;
use serde::Serialize;
use skrifa::FontRef;
use skrifa::raw::TableProvider;

use crate::ServerError;

#[ffi]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontName {
    pub name_id: u16,
    pub platform_id: u16,
    pub language_id: u16,
    pub value: String,
}

#[ffi]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontMetadata {
    pub weight: u16,
    pub style: String,
    pub names: Vec<FontName>,
}

pub fn get_font_metadata(data: &[u8]) -> Result<FontMetadata, ServerError> {
    let font = FontRef::new(data).map_err(|e| ServerError::InvalidFont(e.to_string()))?;

    let os2 = font
        .os2()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let weight = os2.us_weight_class();
    let fs_selection = os2.fs_selection();

    let style = if fs_selection.contains(skrifa::raw::tables::os2::SelectionFlags::ITALIC) {
        "italic"
    } else if fs_selection.contains(skrifa::raw::tables::os2::SelectionFlags::OBLIQUE) {
        "oblique"
    } else {
        "normal"
    };

    let name_table = font
        .name()
        .map_err(|e| ServerError::InvalidFont(e.to_string()))?;
    let string_data = name_table.string_data();

    let names = name_table
        .name_record()
        .iter()
        .filter_map(|record| {
            let name_id = record.name_id().to_u16();
            if name_id > 255 {
                return None;
            }
            record.string(string_data).ok().and_then(|name_string| {
                let s = name_string.to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(FontName {
                        name_id,
                        platform_id: record.platform_id(),
                        language_id: record.language_id(),
                        value: s,
                    })
                }
            })
        })
        .collect();

    Ok(FontMetadata {
        weight,
        style: style.to_owned(),
        names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_test_font() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../editor-view/assets/Noto-Phantom.ttf"
        ))
        .expect("test font not found")
    }

    #[test]
    fn metadata_weight_is_nonzero() {
        let data = load_test_font();
        let meta = get_font_metadata(&data).unwrap();
        assert!(meta.weight > 0);
    }

    #[test]
    fn metadata_style_is_valid() {
        let data = load_test_font();
        let meta = get_font_metadata(&data).unwrap();
        assert!(
            meta.style == "normal" || meta.style == "italic" || meta.style == "oblique",
            "unexpected style: {}",
            meta.style
        );
    }

    #[test]
    fn metadata_has_names() {
        let data = load_test_font();
        let meta = get_font_metadata(&data).unwrap();
        assert!(!meta.names.is_empty());
    }

    #[test]
    fn metadata_invalid_data() {
        let result = get_font_metadata(&[0, 1, 2, 3]);
        assert!(result.is_err());
    }
}

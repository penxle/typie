use serde::Serialize;
use skrifa::FontRef;
use skrifa::raw::TableProvider;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
pub struct FontName {
    pub name_id: u16,
    pub platform_id: u16,
    pub language_id: u16,
    pub value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
pub struct FontMetadata {
    pub weight: u16,
    pub style: &'static str,
    pub names: Vec<FontName>,
}

pub fn get_font_metadata(data: &[u8]) -> Result<FontMetadata, String> {
    let font = FontRef::new(data).map_err(|e| e.to_string())?;

    let os2 = font.os2().map_err(|e| e.to_string())?;
    let weight = os2.us_weight_class();
    let fs_selection = os2.fs_selection();

    let style = if fs_selection.contains(skrifa::raw::tables::os2::SelectionFlags::ITALIC) {
        "italic"
    } else if fs_selection.contains(skrifa::raw::tables::os2::SelectionFlags::OBLIQUE) {
        "oblique"
    } else {
        "normal"
    };

    let name_table = font.name().map_err(|e| e.to_string())?;
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
        style,
        names,
    })
}

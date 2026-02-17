use serde::Serialize;
use skrifa::FontRef;
use skrifa::raw::TableProvider;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[cfg_attr(feature = "wasm", tsify(into_wasm_abi))]
pub struct FontMetadata {
    pub weight: u16,
    pub style: &'static str,
    pub family_name: Option<String>,
    pub display_name: Option<String>,
    pub full_name: Option<String>,
    pub post_script_name: String,
    pub subfamily_display_name: Option<String>,
}

const NAME_TYPOGRAPHIC_FAMILY: u16 = 16;
const NAME_TYPOGRAPHIC_SUBFAMILY: u16 = 17;
const NAME_FAMILY: u16 = 1;
const NAME_SUBFAMILY: u16 = 2;
const NAME_FULL: u16 = 4;
const NAME_POSTSCRIPT: u16 = 6;

const PLATFORM_WINDOWS: u16 = 3;

const LANG_EN_US: u16 = 0x0409;
const LANG_KO_KR: u16 = 0x0412;

fn find_name(
    name_table: &skrifa::raw::tables::name::Name,
    name_id: u16,
    language_id: Option<u16>,
) -> Option<String> {
    let string_data = name_table.string_data();
    for record in name_table.name_record() {
        if record.name_id() != skrifa::raw::types::NameId::new(name_id) {
            continue;
        }

        if let Some(lang) = language_id {
            if record.platform_id() != PLATFORM_WINDOWS || record.language_id() != lang {
                continue;
            }
        }

        if let Ok(name_string) = record.string(string_data) {
            let s = name_string.to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }

    None
}

fn get_name_en(name_table: &skrifa::raw::tables::name::Name, name_id: u16) -> Option<String> {
    find_name(name_table, name_id, Some(LANG_EN_US))
        .or_else(|| find_name(name_table, name_id, None))
}

fn get_name_ko(name_table: &skrifa::raw::tables::name::Name, name_id: u16) -> Option<String> {
    find_name(name_table, name_id, Some(LANG_KO_KR)).or_else(|| get_name_en(name_table, name_id))
}

pub(crate) fn get_font_metadata(data: &[u8]) -> Result<FontMetadata, String> {
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

    let family_name = get_name_en(&name_table, NAME_TYPOGRAPHIC_FAMILY)
        .or_else(|| get_name_en(&name_table, NAME_FAMILY));
    let display_name = get_name_ko(&name_table, NAME_TYPOGRAPHIC_FAMILY)
        .or_else(|| get_name_ko(&name_table, NAME_FAMILY));
    let full_name = get_name_en(&name_table, NAME_FULL);
    let post_script_name =
        get_name_en(&name_table, NAME_POSTSCRIPT).ok_or("missing PostScript name")?;
    let subfamily_display_name = get_name_ko(&name_table, NAME_TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| get_name_ko(&name_table, NAME_SUBFAMILY));

    Ok(FontMetadata {
        weight,
        style,
        family_name,
        display_name,
        full_name,
        post_script_name,
        subfamily_display_name,
    })
}

use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;
use icu_provider::buf::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;
use std::sync::OnceLock;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

static ICU_DATA_PROVIDER: OnceLock<BlobDataProvider> = OnceLock::new();
static GENERAL_CATEGORY_DATA: OnceLock<CodePointMapData<GeneralCategory>> = OnceLock::new();

fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ruzstd::decoding::FrameDecoder::new();
    let mut output = Vec::with_capacity(data.len() * 3);
    decoder
        .decode_all_to_vec(data, &mut output)
        .map_err(|e| format!("Failed to decompress zstd ICU data: {:?}", e))?;
    Ok(output)
}

#[cfg(all(feature = "wasm", not(feature = "native"), not(feature = "uniffi")))]
pub fn load_icu_data(data: &[u8]) -> Result<(), JsValue> {
    if ICU_DATA_PROVIDER.get().is_some() {
        return Ok(());
    }

    let decompressed = decompress_zstd(data).map_err(|e| JsValue::from_str(&e))?;

    let provider =
        BlobDataProvider::try_new_from_static_blob(Box::leak(decompressed.into_boxed_slice()))
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize ICU data: {:?}", e)))?;

    let _ = ICU_DATA_PROVIDER.set(provider);

    Ok(())
}

#[cfg(any(feature = "native", feature = "uniffi"))]
pub fn load_icu_data(data: &[u8]) -> Result<(), String> {
    if ICU_DATA_PROVIDER.get().is_some() {
        return Ok(());
    }

    let decompressed = decompress_zstd(data)?;

    let provider =
        BlobDataProvider::try_new_from_static_blob(Box::leak(decompressed.into_boxed_slice()))
            .map_err(|e| format!("Failed to initialize ICU data: {:?}", e))?;

    let _ = ICU_DATA_PROVIDER.set(provider);

    Ok(())
}

pub fn get_icu_provider() -> Option<&'static BlobDataProvider> {
    ICU_DATA_PROVIDER.get()
}

pub fn get_general_category_map() -> Option<&'static CodePointMapData<GeneralCategory>> {
    let provider = get_icu_provider()?;
    Some(GENERAL_CATEGORY_DATA.get_or_init(|| {
        let deserializing_provider = provider.as_deserializing();
        CodePointMapData::<GeneralCategory>::try_new_unstable(&deserializing_provider)
            .expect("Failed to load GeneralCategory data from valid provider")
    }))
}

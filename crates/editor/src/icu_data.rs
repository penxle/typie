use icu_properties::CodePointMapData;
use icu_properties::props::GeneralCategory;
use icu_provider::buf::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;

use std::sync::OnceLock;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

static ICU_DATA_PROVIDER: OnceLock<BlobDataProvider> = OnceLock::new();
static GENERAL_CATEGORY_DATA: OnceLock<CodePointMapData<GeneralCategory>> = OnceLock::new();

#[cfg(feature = "wasm")]
pub fn load_icu_data(data: &[u8]) -> Result<(), JsValue> {
    let provider =
        BlobDataProvider::try_new_from_static_blob(Box::leak(data.to_vec().into_boxed_slice()))
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize ICU data: {:?}", e)))?;

    let _ = ICU_DATA_PROVIDER.set(provider);

    Ok(())
}

#[cfg(feature = "native")]
pub fn load_icu_data(data: &[u8]) -> Result<(), String> {
    let provider =
        BlobDataProvider::try_new_from_static_blob(Box::leak(data.to_vec().into_boxed_slice()))
            .map_err(|e| format!("Failed to initialize ICU data: {:?}", e))?;

    let _ = ICU_DATA_PROVIDER.set(provider);

    Ok(())
}

pub fn get_icu_provider() -> &'static BlobDataProvider {
    ICU_DATA_PROVIDER
        .get()
        .expect("ICU data not initialized. Call load_icu_data first.")
}

pub fn get_general_category_map() -> &'static CodePointMapData<GeneralCategory> {
    GENERAL_CATEGORY_DATA.get_or_init(|| {
        let provider = get_icu_provider();
        let deserializing_provider = provider.as_deserializing();
        CodePointMapData::<GeneralCategory>::try_new_unstable(&deserializing_provider)
            .expect("Failed to load GeneralCategory data")
    })
}

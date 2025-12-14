use icu_provider_blob::BlobDataProvider;

use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

static ICU_DATA_PROVIDER: OnceLock<BlobDataProvider> = OnceLock::new();

pub fn load_icu_data(data: &[u8]) -> Result<(), JsValue> {
    let provider =
        BlobDataProvider::try_new_from_static_blob(Box::leak(data.to_vec().into_boxed_slice()))
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize ICU data: {:?}", e)))?;

    if let Err(_) = ICU_DATA_PROVIDER.set(provider) {
        log!("ICU data already initialized");
    }

    Ok(())
}

pub fn get_icu_provider() -> &'static BlobDataProvider {
    ICU_DATA_PROVIDER
        .get()
        .expect("ICU data not initialized. Call load_icu_data first.")
}

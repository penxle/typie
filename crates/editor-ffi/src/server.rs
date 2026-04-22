use editor_macros::ffi_export;
use wasm_bindgen::prelude::*;

use crate::host::EditorHost;
use crate::prelude::*;

#[ffi_export(wasm)]
impl EditorHost {
    pub fn get_font_metadata(
        &self,
        data: Vec<u8>,
    ) -> EditorResult<Complex<editor_server::font::FontMetadata>> {
        editor_server::font::get_font_metadata(&data)?
            .into_ffi()
            .map_err(Into::into)
    }

    pub fn get_font_codepoints(&self, ttf_data: Vec<u8>) -> EditorResult<JsValue> {
        let cps = editor_server::font::get_font_codepoints(&ttf_data)?;
        serde_wasm_bindgen::to_value(&cps)
            .map_err(|e| FfiError::Serialization(e.to_string()).into())
    }

    pub fn build_font(
        &self,
        ttf_data: Vec<u8>,
        chunk_codepoints: JsValue,
    ) -> EditorResult<Complex<editor_server::font::BuiltFont>> {
        let groups: Vec<Vec<u32>> = serde_wasm_bindgen::from_value(chunk_codepoints)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        editor_server::font::build_font(&ttf_data, &groups)?
            .into_ffi()
            .map_err(Into::into)
    }
}

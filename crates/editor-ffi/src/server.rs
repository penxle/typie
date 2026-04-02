use wasm_bindgen::prelude::*;

use crate::host::EditorHost;
use crate::prelude::*;

#[wasm_bindgen]
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

    pub fn encode_font(
        &self,
        ttf_data: Vec<u8>,
        chunk_codepoints: JsValue,
    ) -> EditorResult<Complex<editor_server::font::EncodedFont>> {
        let chunk_codepoints: Vec<Vec<u32>> = serde_wasm_bindgen::from_value(chunk_codepoints)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        editor_server::font::encode_font(&ttf_data, &chunk_codepoints)?
            .into_ffi()
            .map_err(Into::into)
    }

    pub fn build_font_manifest(&self, chunk_codepoints: JsValue) -> EditorResult<Vec<u8>> {
        let chunk_codepoints: Vec<Vec<u32>> = serde_wasm_bindgen::from_value(chunk_codepoints)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        Ok(editor_server::font::build_font_manifest(&chunk_codepoints)?)
    }

    pub fn build_fallback_font_manifests(&self, entries: JsValue) -> EditorResult<Vec<u8>> {
        #[derive(serde::Deserialize)]
        struct FallbackFontInput {
            family_name: String,
            fonts: Vec<FallbackFontWeightInput>,
        }
        #[derive(serde::Deserialize)]
        struct FallbackFontWeightInput {
            weight: u16,
            manifest: Vec<u8>,
        }

        let inputs: Vec<FallbackFontInput> = serde_wasm_bindgen::from_value(entries)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let entries = inputs
            .into_iter()
            .map(|e| {
                (
                    e.family_name,
                    e.fonts
                        .into_iter()
                        .map(|f| (f.weight, f.manifest))
                        .collect(),
                )
            })
            .collect();
        Ok(editor_server::font::build_fallback_font_manifests(entries)?)
    }
}

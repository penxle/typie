use editor_macros::{ffi, ffi_export};
use serde::{Deserialize, Serialize};

use crate::host::EditorHost;
use crate::prelude::*;

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub merged: editor_model::Doc,
    pub conflicts: Vec<editor_server::sync::ConflictRecord>,
}

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeriveAllObjectsResult {
    pub root_hash: String,
    pub objects: Vec<editor_model::DerivedObject>,
}

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkCodepoints {
    pub chunks: Vec<Vec<u32>>,
}

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectEntry {
    pub hash: String,
    pub content: editor_model::ObjectContent,
}

#[ffi_export(wasm)]
impl EditorHost {
    pub fn merge_docs(
        &self,
        base: Complex<editor_model::Doc>,
        ours: Complex<editor_model::Doc>,
        theirs: Complex<editor_model::Doc>,
    ) -> EditorResult<Complex<MergeResult>> {
        let base = base.from_ffi()?;
        let ours = ours.from_ffi()?;
        let theirs = theirs.from_ffi()?;

        let segmenters =
            self.with_resource(|resource| Ok(std::sync::Arc::clone(&resource.segmenters)))?;
        let (merged, conflicts) =
            editor_server::sync::merge(&segmenters.grapheme, &base, &ours, &theirs);

        Ok(MergeResult { merged, conflicts }.into_ffi()?)
    }

    pub fn get_font_metadata(
        &self,
        data: Vec<u8>,
    ) -> EditorResult<Complex<editor_server::font::FontMetadata>> {
        editor_server::font::get_font_metadata(&data)?
            .into_ffi()
            .map_err(Into::into)
    }

    pub fn get_font_codepoints(&self, ttf_data: Vec<u8>) -> EditorResult<Vec<u32>> {
        Ok(editor_server::font::get_font_codepoints(&ttf_data)?)
    }

    pub fn build_font(
        &self,
        ttf_data: Vec<u8>,
        chunk_codepoints: Complex<ChunkCodepoints>,
    ) -> EditorResult<Complex<editor_server::font::BuiltFont>> {
        let chunk_codepoints = chunk_codepoints.from_ffi()?;
        editor_server::font::build_font(&ttf_data, &chunk_codepoints.chunks)?
            .into_ffi()
            .map_err(Into::into)
    }

    pub fn extract_text(&self, doc: Complex<editor_model::Doc>) -> EditorResult<String> {
        let doc = doc.from_ffi()?;
        Ok(doc.extract_text())
    }

    pub fn derive_all_objects(
        &self,
        doc: Complex<editor_model::Doc>,
    ) -> EditorResult<Complex<DeriveAllObjectsResult>> {
        let doc = doc.from_ffi()?;
        let (root_hash, objects) = doc.derive_all_objects();
        Ok(DeriveAllObjectsResult { root_hash, objects }.into_ffi()?)
    }

    pub fn reconstruct_doc_from_objects(
        &self,
        root_hash: String,
        objects: Vec<Complex<ObjectEntry>>,
    ) -> EditorResult<Complex<editor_model::Doc>> {
        let objects: Vec<ObjectEntry> = objects.from_ffi()?;
        let pairs: Vec<(String, editor_model::ObjectContent)> =
            objects.into_iter().map(|o| (o.hash, o.content)).collect();
        let doc = editor_model::Doc::reconstruct_from_objects(&root_hash, &pairs)?;
        Ok(doc.into_ffi()?)
    }

    pub fn hash_object_content(
        &self,
        content: Complex<editor_model::ObjectContent>,
    ) -> EditorResult<String> {
        let content = content.from_ffi()?;
        Ok(content.hash())
    }
}

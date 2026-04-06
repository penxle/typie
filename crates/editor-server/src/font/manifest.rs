use editor_resource::{FallbackFont, FallbackFontEntry, FontManifest};
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::ServerError;

type FallbackFontEntries = Vec<(String, Vec<(u16, Vec<u8>)>)>;

fn compress_bitcode(data: &[u8]) -> Vec<u8> {
    compress_to_vec(data, CompressionLevel::Fastest)
}

fn decompress_manifest(data: &[u8]) -> Result<Vec<u8>, ServerError> {
    use std::io::Read;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(data)
        .map_err(|e| ServerError::EncodingFailed(format!("{e:?}")))?;
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| ServerError::EncodingFailed(format!("{e:?}")))?;
    Ok(output)
}

pub fn build_font_manifest(chunk_codepoints: &[Vec<u32>]) -> Result<Vec<u8>, ServerError> {
    let manifest = FontManifest::from_chunk_codepoints(chunk_codepoints);
    Ok(compress_bitcode(&bitcode::encode(&manifest)))
}

pub fn build_fallback_font_manifests(entries: FallbackFontEntries) -> Result<Vec<u8>, ServerError> {
    let fallback_entries: Vec<FallbackFontEntry> = entries
        .into_iter()
        .map(|(family_name, fonts)| {
            let fonts = fonts
                .into_iter()
                .map(|(weight, compressed_manifest)| {
                    let raw = decompress_manifest(&compressed_manifest)?;
                    let manifest: FontManifest = bitcode::decode(&raw)
                        .map_err(|e| ServerError::EncodingFailed(e.to_string()))?;
                    Ok(FallbackFont { weight, manifest })
                })
                .collect::<Result<Vec<_>, ServerError>>()?;
            Ok(FallbackFontEntry { family_name, fonts })
        })
        .collect::<Result<Vec<_>, ServerError>>()?;

    Ok(compress_bitcode(&bitcode::encode(&fallback_entries)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_compressed<T: for<'a> bitcode::Decode<'a>>(data: &[u8]) -> T {
        let decompressed = decompress_manifest(data).unwrap();
        bitcode::decode(&decompressed).unwrap()
    }

    #[test]
    fn roundtrip_single_chunk() {
        let bytes = build_font_manifest(&[vec![0x0041, 0x0042]]).unwrap();
        let manifest: FontManifest = decode_compressed(&bytes);
        assert_eq!(manifest.chunk_count, 1);
        assert_eq!(manifest.chunk_index(0x0041), Some(0));
        assert_eq!(manifest.chunk_index(0x0042), Some(0));
    }

    #[test]
    fn roundtrip_multiple_chunks_with_supplementary() {
        let bytes =
            build_font_manifest(&[vec![0x0041], vec![0xAC00, 0x1F600], vec![0x4E00]]).unwrap();
        let manifest: FontManifest = decode_compressed(&bytes);
        assert_eq!(manifest.chunk_count, 3);
        assert_eq!(manifest.chunk_index(0x0041), Some(0));
        assert_eq!(manifest.chunk_index(0xAC00), Some(1));
        assert_eq!(manifest.chunk_index(0x1F600), Some(1));
        assert_eq!(manifest.chunk_index(0x4E00), Some(2));
    }

    #[test]
    fn roundtrip_empty() {
        let bytes = build_font_manifest(&[]).unwrap();
        let manifest: FontManifest = decode_compressed(&bytes);
        assert_eq!(manifest.chunk_count, 0);
    }

    #[test]
    fn fallback_roundtrip() {
        let m1 = build_font_manifest(&[vec![0x0041]]).unwrap();
        let m2 = build_font_manifest(&[vec![0xAC00], vec![0x1F600]]).unwrap();

        let bytes = build_fallback_font_manifests(vec![
            ("FamilyA".into(), vec![(400, m1)]),
            ("FamilyB".into(), vec![(400, m2.clone()), (700, m2)]),
        ])
        .unwrap();

        let entries: Vec<FallbackFontEntry> = decode_compressed(&bytes);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].family_name, "FamilyA");
        assert_eq!(entries[0].fonts.len(), 1);
        assert_eq!(entries[0].fonts[0].weight, 400);
        assert_eq!(entries[0].fonts[0].manifest.chunk_index(0x0041), Some(0));
        assert_eq!(entries[1].family_name, "FamilyB");
        assert_eq!(entries[1].fonts.len(), 2);
        assert_eq!(entries[1].fonts[0].manifest.chunk_index(0xAC00), Some(0));
        assert_eq!(entries[1].fonts[1].weight, 700);
    }
}

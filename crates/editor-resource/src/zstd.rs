use std::io::Read;

use crate::error::ResourceError;

pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, ResourceError> {
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(data)
        .map_err(|e| ResourceError::Decompression(format!("{e:?}")))?;

    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| ResourceError::Decompression(format!("{e:?}")))?;

    Ok(output)
}

pub fn compress_zstd(data: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstd_roundtrip() {
        let original = b"hello world";
        let compressed = compress_zstd(original.as_slice());
        let back = decompress_zstd(&compressed).unwrap();
        assert_eq!(back.as_slice(), original.as_slice());
    }

    #[test]
    fn compress_zstd_roundtrip() {
        let original = b"hello world";
        let compressed = compress_zstd(original);
        let back = decompress_zstd(&compressed).unwrap();
        assert_eq!(back.as_slice(), original.as_slice());
    }
}

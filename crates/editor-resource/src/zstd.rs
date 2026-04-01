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

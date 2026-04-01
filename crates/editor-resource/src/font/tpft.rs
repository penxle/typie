use crate::error::ResourceError;
use crate::zstd::decompress_zstd;

pub(super) const TPFT_MAGIC: &[u8; 4] = b"TPFT";
pub(super) const TPFT_VERSION: u16 = 1;
pub(super) const TPFT_HEADER_SIZE: usize = 6;

pub fn decode_tpft(data: &[u8]) -> Result<Vec<u8>, ResourceError> {
    if data.len() < TPFT_HEADER_SIZE {
        return Err(ResourceError::InvalidFont("TPFT data too short".into()));
    }

    if &data[0..4] != TPFT_MAGIC {
        return Err(ResourceError::InvalidFont("invalid TPFT magic".into()));
    }

    let version = u16::from_be_bytes(data[4..6].try_into().unwrap());
    if version != TPFT_VERSION {
        return Err(ResourceError::InvalidFont(format!(
            "unsupported TPFT version {version}"
        )));
    }

    decompress_zstd(&data[TPFT_HEADER_SIZE..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tpft(payload: &[u8]) -> Vec<u8> {
        let compressed = zstd_compress(payload);
        let mut buf = Vec::with_capacity(TPFT_HEADER_SIZE + compressed.len());
        buf.extend_from_slice(TPFT_MAGIC);
        buf.extend_from_slice(&TPFT_VERSION.to_be_bytes());
        buf.extend_from_slice(&compressed);
        buf
    }

    fn zstd_compress(data: &[u8]) -> Vec<u8> {
        ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest)
    }

    #[test]
    fn decode_valid_tpft() {
        let payload = b"hello world";
        let encoded = make_tpft(payload);
        let decoded = decode_tpft(&encoded).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_too_short() {
        assert!(decode_tpft(&[0; 4]).is_err());
    }

    #[test]
    fn decode_bad_magic() {
        let mut data = make_tpft(b"x");
        data[0] = b'X';
        assert!(decode_tpft(&data).is_err());
    }

    #[test]
    fn decode_bad_version() {
        let mut data = make_tpft(b"x");
        data[4..6].copy_from_slice(&99u16.to_be_bytes());
        assert!(decode_tpft(&data).is_err());
    }
}

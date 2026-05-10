use std::io::Read;

use crate::wire::{WireError, WireResult};

pub const MAGIC: u8 = 0xCD;
pub const VERSION: u8 = 0x01;
pub const FLAG_BODY_COMPRESSED: u8 = 0b0000_0001;
pub const FLAGS_REQUIRED_MASK: u8 = 0b0000_1111;
pub const FLAGS_KNOWN_REQUIRED: u8 = FLAG_BODY_COMPRESSED;

pub const COMPRESSION_THRESHOLD_BYTES: usize = 256;

/// Wraps `body` with envelope; applies zstd if body is large enough and zstd actually shrinks.
pub fn wrap(body: Vec<u8>) -> Vec<u8> {
    let (flags, payload) = if body.len() >= COMPRESSION_THRESHOLD_BYTES {
        let compressed = ruzstd::encoding::compress_to_vec(
            body.as_slice(),
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        if compressed.len() < body.len() {
            (FLAG_BODY_COMPRESSED, compressed)
        } else {
            (0, body)
        }
    } else {
        (0, body)
    };

    let mut out = Vec::with_capacity(3 + payload.len());
    out.push(MAGIC);
    out.push(VERSION);
    out.push(flags);
    out.extend_from_slice(&payload);
    out
}

/// Strips envelope; returns raw body bytes (decompressed if flag set).
pub fn unwrap(bytes: &[u8]) -> WireResult<Vec<u8>> {
    if bytes.len() < 3 {
        return Err(WireError::Truncated {
            expected: 3,
            actual: bytes.len(),
        });
    }
    if bytes[0] != MAGIC {
        return Err(WireError::BadMagic { got: bytes[0] });
    }
    if bytes[1] != VERSION {
        return Err(WireError::UnsupportedVersion { got: bytes[1] });
    }
    let flags = bytes[2];
    let required_bits = flags & FLAGS_REQUIRED_MASK;
    if required_bits & !FLAGS_KNOWN_REQUIRED != 0 {
        return Err(WireError::RequiredFlagSet { flags });
    }
    let payload = &bytes[3..];
    if flags & FLAG_BODY_COMPRESSED != 0 {
        let mut decoder = ruzstd::decoding::StreamingDecoder::new(payload)
            .map_err(|e| WireError::Zstd(format!("{e:?}")))?;
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| WireError::Zstd(format!("{e:?}")))?;
        Ok(out)
    } else {
        Ok(payload.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unwrap_round_trip_small() {
        let body = b"hello, world".to_vec();
        let wrapped = wrap(body.clone());
        assert_eq!(wrapped[0], MAGIC);
        assert_eq!(wrapped[1], VERSION);
        assert_eq!(wrapped[2], 0);
        let unwrapped = unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, body);
    }

    #[test]
    fn wrap_unwrap_round_trip_large() {
        let body = vec![b'x'; 10_000];
        let wrapped = wrap(body.clone());
        assert!(
            wrapped.len() < body.len() / 2,
            "should be compressed: wrapped={}",
            wrapped.len()
        );
        assert_eq!(wrapped[2] & FLAG_BODY_COMPRESSED, FLAG_BODY_COMPRESSED);
        let unwrapped = unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, body);
    }

    #[test]
    fn unwrap_bad_magic_errors() {
        let bytes = vec![0xFF, 0x01, 0];
        let err = unwrap(&bytes).unwrap_err();
        assert!(matches!(err, WireError::BadMagic { got: 0xFF }));
    }

    #[test]
    fn unwrap_unsupported_version_errors() {
        let bytes = vec![MAGIC, 0xFE, 0];
        let err = unwrap(&bytes).unwrap_err();
        assert!(matches!(err, WireError::UnsupportedVersion { got: 0xFE }));
    }

    #[test]
    fn unwrap_required_flag_bit_errors() {
        let bytes = vec![MAGIC, VERSION, 0b0000_0010];
        let err = unwrap(&bytes).unwrap_err();
        assert!(matches!(err, WireError::RequiredFlagSet { .. }));
    }

    #[test]
    fn unwrap_ignorable_flag_bit_passes() {
        let body = b"x".to_vec();
        let mut wrapped = wrap(body.clone());
        wrapped[2] |= 0b0001_0000;
        let unwrapped = unwrap(&wrapped).unwrap();
        assert_eq!(unwrapped, body);
    }

    #[test]
    fn unwrap_too_short_errors() {
        let err = unwrap(&[]).unwrap_err();
        assert!(matches!(err, WireError::Truncated { .. }));
        let err = unwrap(&[MAGIC, VERSION]).unwrap_err();
        assert!(matches!(err, WireError::Truncated { .. }));
    }
}

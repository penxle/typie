use super::{TPFT_HEADER_SIZE, TPFT_MAGIC, TPFT_VERSION};
use std::io::Read;

pub(crate) fn decode_tpft(data: &[u8]) -> Vec<u8> {
    assert_eq!(&data[0..4], TPFT_MAGIC, "invalid TPFT magic");
    let version = u16::from_be_bytes(data[4..6].try_into().unwrap());
    assert_eq!(version, TPFT_VERSION, "unsupported TPFT version {version}");
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(&data[TPFT_HEADER_SIZE..])
        .expect("zstd init failed");
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).expect("zstd decode failed");
    buf
}

use super::{TPFT_HEADER_SIZE, TPFT_MAGIC, TPFT_VERSION};
use anyhow::Context;
use std::io::Read;

pub(crate) fn decode_tpft(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(data.len() >= TPFT_HEADER_SIZE, "TPFT data too short");
    anyhow::ensure!(&data[0..4] == TPFT_MAGIC, "invalid TPFT magic");
    let version = u16::from_be_bytes(data[4..6].try_into().unwrap());
    anyhow::ensure!(
        version == TPFT_VERSION,
        "unsupported TPFT version {version}"
    );
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(&data[TPFT_HEADER_SIZE..])
        .map_err(|e| anyhow::anyhow!("zstd init failed: {e}"))?;
    let mut buf = Vec::new();
    decoder
        .read_to_end(&mut buf)
        .context("zstd decode failed")?;
    Ok(buf)
}

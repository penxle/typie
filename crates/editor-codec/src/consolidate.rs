use crate::bundle::decode_bundle_from_envelope;
use crate::convert::{ReencodableChangesets, changesets_from_ctx_and_bundles, encode_changesets};
use crate::envelope::unwrap_one;
use crate::error::{CodecError, CodecResult};

pub struct Consolidation {
    pub payload: Vec<u8>,
    pub consumed: usize,
    pub consumed_bytes: usize,
}

pub fn consolidate_stream(bytes: &[u8]) -> CodecResult<Option<Consolidation>> {
    let mut input = bytes;
    let mut parts: Vec<ReencodableChangesets> = Vec::new();
    let mut consumed = 0usize;
    let mut consumed_bytes = 0usize;

    while !input.is_empty() {
        let before = input;
        // Decode via the envelope `unwrap_one` already parsed — avoids re-parsing
        // the same header a second time through a bytes-based decode entry point.
        // Once `unwrap_one` succeeds, none of the body-level decoding below can
        // raise `Fenced` (that's an envelope-header-only error class), so only
        // `unwrap_one` itself needs to treat it as a stream boundary.
        let envelope = match unwrap_one(&mut input) {
            Ok(e) => e,
            Err(CodecError::Fenced(_)) => break,
            Err(e) => return Err(e),
        };
        let taken = before.len() - input.len();
        let (ctx, bundles) = decode_bundle_from_envelope(&envelope)?;
        let decoded = changesets_from_ctx_and_bundles(ctx, bundles)?;
        match decoded.into_reencodable() {
            Ok(r) => parts.push(r),
            Err(_) => break,
        }
        consumed += 1;
        consumed_bytes += taken;
    }

    if consumed < 2 {
        return Ok(None);
    }

    let payload = encode_changesets(ReencodableChangesets::concat(parts))?;
    Ok(Some(Consolidation {
        payload,
        consumed,
        consumed_bytes,
    }))
}
